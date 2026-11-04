//! Mesh3D — GPU 렌더러(깊이 테스트, 텍스처별 묶음 그리기, 윤곽선 후처리)와 정점 캐시.

use std::collections::HashMap;

use miniquad::*;

use super::*;

// ================= GPU 렌더러 =================

pub struct Mesh3D {
    pass: RenderPass,
    color_tex: TextureId,
    depth_tex: TextureId,
    pipeline: Pipeline,
    white_tex: TextureId, // 텍스처 없는(단색) 상자용 1x1 흰 텍스처
    size: (f32, f32),
    // 프레임마다 새로 만들지 않고 계속 쓰는 것들 — 그리기 직전에 정점 데이터만 갈아 끼운다.
    index_buf: BufferId,                    // 0, 1, 2, ... 순서 인덱스(정점은 삼각형마다 따로 있다) — 한 번만 만든다
    vertex_bufs: Vec<(BufferId, usize)>,    // 텍스처 묶음마다 하나씩(버퍼, 담을 수 있는 정점 수)
    groups: Vec<(TextureId, Vec<Vertex>)>,  // 텍스처별로 모은 정점 — 매 프레임 비우고 다시 채운다(할당은 재사용)
    mesh_cache: HashMap<MeshKey, CachedMesh>, // 가만히 있는 메시의 월드 변환/셰이딩 결과 — 매 프레임 다시 계산하지 않는다
    tick: u64,                              // render_ex 호출 횟수(캐시의 "최근에 썼나" 기준)
    // 카툰 윤곽선 후처리 — 장면(color_tex/depth_tex)을 읽어 윤곽선을 그린 결과를 out_tex 에 쓴다.
    out_tex: TextureId,
    out_pass: RenderPass,
    edge_pipeline: Pipeline,
    quad_vbuf: BufferId,
    quad_ibuf: BufferId,
}

// 메시 정점 캐시 — 같은 메시가 같은 자리/색으로 다시 나오면(방의 가구, 정글 지형 등 대부분) 삼각형 수만큼의 행렬
// 곱과 셰이딩 계산을 건너뛰고 지난번 결과를 그대로 쓴다. 움직이는 메시는 매번 키가 달라서 캐시에 쌓이기만 하므로
// 전체 크기를 MESH_CACHE_MAX_VERTS 로 묶고, 넘치면 가장 오래 안 쓴 것부터 버린다.
const MESH_CACHE_MAX_VERTS: usize = 600_000;

#[derive(Hash, PartialEq, Eq, Clone)]
pub(super) struct MeshKey {
    mesh: usize, // Rc<MeshData> 주소 — CachedMesh 가 그 Rc 를 붙들고 있어서 주소가 다른 메시에 재사용되지 않는다
    model: [u32; 16],
    tint: [u32; 4],
}

struct CachedMesh {
    verts: Vec<Vertex>,
    last_used: u64,
    _keep: std::rc::Rc<MeshData>,
}

pub(super) fn mesh_key(m: &MeshObject) -> MeshKey {
    let model = m.model_matrix();
    let mut bits = [0u32; 16];
    for (i, row) in model.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            bits[i * 4 + j] = v.to_bits();
        }
    }
    MeshKey { mesh: std::rc::Rc::as_ptr(&m.mesh) as usize, model: bits, tint: m.color.map(f32::to_bits) }
}

// 한 번의 드로우콜에 담을 수 있는 최대 정점 수(인덱스가 u16 이라서) — 삼각형 단위로 맞춘다.
pub(super) const MAX_DRAW_VERTS: usize = 65535 / 3 * 3;

#[repr(C)]
struct Mesh3DUniform {
    view_proj: Mat4,
}

const MESH3D_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec4 in_color;
uniform mat4 view_proj;
varying highp vec2 uv;
varying lowp vec4 color;
void main() {
    gl_Position = view_proj * vec4(in_pos, 1.0);
    uv = in_uv;
    color = in_color;
}
"#;

const MESH3D_FS: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
varying lowp vec4 color;
uniform sampler2D tex;
void main() {
    vec4 texColor = texture2D(tex, uv);
    // 텍스처의 투명한 부분(예: 아이콘 PNG 의 알파=0 배경)을 그냥 곱해서 그리면
    // 블렌딩이 없는 이 3D 파이프라인 특성상 알파가 무시되고 그 자리에 박혀있는
    // RGB 값(보통 검정)이 그대로 불투명하게 칠해진다 — 반투명 합성 대신 완전히
    // 잘라내는(cutout) 방식으로 처리한다. 텍스처 없는 단색 상자는 1x1 흰
    // 텍스처(알파=1)를 쓰므로 이 문턱값에 걸리지 않는다.
    if (texColor.a < 0.5) {
        discard;
    }
    vec3 c = texColor.rgb * color.rgb;
    // 셀 셰이딩 — 밝기를 5단계로 끊는다(색조는 그대로 두고 밝기만 단계화).
    float l = max(max(c.r, c.g), max(c.b, 0.001));
    float band = floor(l * 5.0 + 0.5) / 5.0;
    c *= max(band, 0.12) / l;
    gl_FragColor = vec4(c, texColor.a * color.a);
}
"#;

// 카툰 렌더링 — 1) 조각 셰이더에서 밝기를 몇 단계로 끊고(셀 셰이딩), 2) 후처리에서 깊이/색 차이로 검은 윤곽선을 긋는다.
const EDGE_VS: &str = r#"#version 100
attribute vec2 in_pos;
varying highp vec2 uv;
void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    uv = in_pos * 0.5 + 0.5;
}
"#;

const EDGE_FS: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
uniform sampler2D color_tex;
uniform sampler2D depth_tex;
uniform vec2 texel;
const float NEAR = 0.05;
const float FAR = 200.0;
float lin(vec2 p) {
    float d = texture2D(depth_tex, p).r * 2.0 - 1.0;
    return 2.0 * NEAR * FAR / (FAR + NEAR - d * (FAR - NEAR));
}
float lum(vec2 p) {
    vec3 c = texture2D(color_tex, p).rgb;
    return dot(c, vec3(0.3, 0.59, 0.11));
}
void main() {
    vec4 base = texture2D(color_tex, uv);
    vec2 dx = vec2(texel.x, 0.0);
    vec2 dy = vec2(0.0, texel.y);
    float zc = lin(uv);
    // 깊이의 2차 차분 — 평면(바닥/벽)은 0 이라 비스듬히 보여도 선이 안 생기고, 물체 가장자리에서만 크게 튄다.
    float lap = abs(lin(uv + dx) + lin(uv - dx) - 2.0 * zc) + abs(lin(uv + dy) + lin(uv - dy) - 2.0 * zc);
    float depthEdge = smoothstep(0.03, 0.09, lap / max(zc, 0.2));
    // 같은 깊이에서 색이 확 바뀌는 곳(모서리의 면 밝기 차이, 재질 경계)
    float lc = lum(uv);
    float cdiff = abs(lum(uv + dx) - lc) + abs(lum(uv - dx) - lc) + abs(lum(uv + dy) - lc) + abs(lum(uv - dy) - lc);
    float colorEdge = smoothstep(0.35, 0.6, cdiff);
    float edge = max(depthEdge, colorEdge * 0.8);
    // 하늘(깊이가 가장 먼 곳)에는 선을 안 긋는다.
    if (zc > FAR * 0.9) {
        edge = 0.0;
    }
    gl_FragColor = vec4(mix(base.rgb, vec3(0.03, 0.02, 0.04), edge), 1.0);
}
"#;

impl Mesh3D {
    fn make_targets(ctx: &mut dyn RenderingBackend, w: u32, h: u32) -> (TextureId, TextureId, RenderPass, TextureId, RenderPass) {
        let color_tex = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let depth_tex = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::Depth,
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let pass = ctx.new_render_pass(color_tex, Some(depth_tex));
        let out_tex = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let out_pass = ctx.new_render_pass(out_tex, None);
        (color_tex, depth_tex, pass, out_tex, out_pass)
    }

    pub fn new(ctx: &mut dyn RenderingBackend, vw: u32, vh: u32) -> Mesh3D {
        let (color_tex, depth_tex, pass, out_tex, out_pass) = Self::make_targets(ctx, vw, vh);

        let white_tex = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: MESH3D_VS, fragment: MESH3D_FS },
                ShaderMeta { images: vec!["tex".to_string()], uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("view_proj", UniformType::Mat4)] } },
            )
            .expect("mesh3d 셰이더 컴파일 실패");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float3),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
                VertexAttribute::new("in_color", VertexFormat::Float4),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                cull_face: CullFace::Back,
                ..Default::default()
            },
        );

        let indices: Vec<u16> = (0..MAX_DRAW_VERTS as u16).collect();
        let index_buf = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(indices.as_slice()));

        let edge_shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: EDGE_VS, fragment: EDGE_FS },
                ShaderMeta {
                    images: vec!["color_tex".to_string(), "depth_tex".to_string()],
                    uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("texel", UniformType::Float2)] },
                },
            )
            .expect("윤곽선 셰이더 컴파일 실패");
        let edge_pipeline = ctx.new_pipeline(&[BufferLayout::default()], &[VertexAttribute::new("in_pos", VertexFormat::Float2)], edge_shader, PipelineParams::default());
        let quad: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
        let quad_vbuf = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&quad));
        let quad_idx: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let quad_ibuf = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&quad_idx));

        Mesh3D {
            pass,
            color_tex,
            depth_tex,
            pipeline,
            white_tex,
            size: (vw as f32, vh as f32),
            index_buf,
            vertex_bufs: Vec::new(),
            groups: Vec::new(),
            mesh_cache: HashMap::new(),
            tick: 0,
            out_tex,
            out_pass,
            edge_pipeline,
            quad_vbuf,
            quad_ibuf,
        }
    }

    pub fn set_resolution(&mut self, ctx: &mut dyn RenderingBackend, w: u32, h: u32) {
        ctx.delete_render_pass(self.pass);
        ctx.delete_render_pass(self.out_pass);
        ctx.delete_texture(self.color_tex);
        ctx.delete_texture(self.depth_tex);
        ctx.delete_texture(self.out_tex);
        (self.color_tex, self.depth_tex, self.pass, self.out_tex, self.out_pass) = Self::make_targets(ctx, w, h);
        self.size = (w as f32, h as f32);
    }

    // 윤곽선까지 그려진 최종 결과.
    pub fn color_texture(&self) -> TextureId {
        self.out_tex
    }

    pub fn aspect(&self) -> f32 {
        self.size.0 / self.size.1
    }

    // camera/boxes 로 장면을 오프스크린 타깃에 그린다. 텍스처별로 묶어 드로우콜을
    // 나눈다(텍스처 없는 상자는 1x1 흰 텍스처 + 정점색으로 처리해서 파이프라인이
    // 하나로 충분하다). 상자 수가 적은(수십~수백) 스타일화된 장면이 목표라 매 프레임
    // 버텍스 버퍼를 새로 만들어도 무리 없다. MeshObject 없이 Box3D 만 그리는
    // 호출부(apps/doors_game)가 빈 메시 목록을 매번 넘기지 않아도 되게 render_ex
    // 의 얇은 래퍼로 둔다.
    pub fn render(&mut self, ctx: &mut dyn RenderingBackend, clear: [f32; 4], camera: &Camera, boxes: &[Box3D], fov_y: f32) {
        self.render_ex(ctx, clear, camera, boxes, &[], fov_y);
    }

    // render() 와 같지만 MeshObject(블렌더 등에서 OBJ 로 가져온 임의 메시)도
    // 같이 그린다 — Box3D 와 정확히 같은 정점 포맷(Vertex)과 텍스처별 그룹핑을
    // 그대로 재사용한다(셰이더/파이프라인도 공유). 주의: 텍스처 그룹 하나당
    // 정점 인덱스를 u16 로 순차 매긴다(기존 Box3D 방식 그대로) — 텍스처 하나를
    // 공유하는 메시들의 정점 합이 65536 을 넘으면 넘친다. 이 프로젝트 규모의
    // 스타일화된 소품이면 충분하지만, 아주 고폴리인 메시를 통째로 넣을 땐
    // 주의(필요해지면 u32 인덱스로 바꿔야 한다).
    // 캐시가 한도를 넘으면 가장 오래 안 쓴 것부터 버린다(이번 호출에서 쓴 것은 남긴다).
    fn trim_mesh_cache(&mut self) {
        let mut total: usize = self.mesh_cache.values().map(|c| c.verts.len()).sum();
        if total <= MESH_CACHE_MAX_VERTS {
            return;
        }
        let mut by_age: Vec<(u64, usize, MeshKey)> = self.mesh_cache.iter().map(|(k, c)| (c.last_used, c.verts.len(), k.clone())).collect();
        by_age.sort_by_key(|e| e.0);
        for (used, len, key) in by_age {
            if total <= MESH_CACHE_MAX_VERTS || used == self.tick {
                break;
            }
            self.mesh_cache.remove(&key);
            total -= len;
        }
    }

    pub fn render_ex(&mut self, ctx: &mut dyn RenderingBackend, clear: [f32; 4], camera: &Camera, boxes: &[Box3D], meshes: &[MeshObject], fov_y: f32) {
        let proj = mat_perspective(fov_y, self.aspect(), 0.05, 200.0);
        let view = camera.view_matrix();
        let view_proj = mat_mul(&proj, &view);

        // 텍스처(TextureId 없으면 흰 텍스처)별로 정점을 모은다 — 지난 프레임의 Vec 들을 비워서 다시 쓴다.
        for (_, verts) in &mut self.groups {
            verts.clear();
        }
        let white = self.white_tex;
        let group_of = |groups: &mut Vec<(TextureId, Vec<Vertex>)>, tex: TextureId| -> usize {
            match groups.iter().position(|(t, _)| *t == tex) {
                Some(i) => i,
                None => {
                    groups.push((tex, Vec::new()));
                    groups.len() - 1
                }
            }
        };
        for b in boxes {
            let gi = group_of(&mut self.groups, b.texture.map(|t| t.texture).unwrap_or(white));
            let entry = &mut self.groups[gi].1;
            for (a, bb, c) in b.to_triangles() {
                entry.push(a);
                entry.push(bb);
                entry.push(c);
            }
        }
        self.tick += 1;
        for m in meshes {
            let gi = group_of(&mut self.groups, m.texture.unwrap_or(white));
            let cached = self.mesh_cache.entry(mesh_key(m)).or_insert_with(|| CachedMesh {
                verts: m.to_triangles().into_iter().flat_map(|(a, b, c)| [a, b, c]).collect(),
                last_used: 0,
                _keep: std::rc::Rc::clone(&m.mesh),
            });
            cached.last_used = self.tick;
            self.groups[gi].1.extend_from_slice(&cached.verts);
        }
        self.trim_mesh_cache();

        ctx.begin_pass(Some(self.pass), PassAction::clear_color(clear[0], clear[1], clear[2], clear[3]));
        ctx.apply_viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_scissor_rect(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_pipeline(&self.pipeline);

        // 인덱스가 u16 이라 드로우콜 하나에 MAX_DRAW_VERTS 개까지만 담긴다 — 넘치는 묶음은 여러 조각으로
        // 나눠서 조각마다 자기 정점 버퍼로 그린다(예전엔 넘치는 삼각형을 조용히 버려서 모델 일부가 사라졌다).
        let mut slot = 0;
        for gi in 0..self.groups.len() {
            let tex = self.groups[gi].0;
            for chunk in self.groups[gi].1.chunks(MAX_DRAW_VERTS) {
                // 이 조각의 정점 버퍼 — 모자라면 두 배로 다시 만든다.
                while self.vertex_bufs.len() <= slot {
                    self.vertex_bufs.push((ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vertex>(4096)), 4096));
                }
                if self.vertex_bufs[slot].1 < chunk.len() {
                    ctx.delete_buffer(self.vertex_bufs[slot].0);
                    let cap = chunk.len().next_power_of_two().min(MAX_DRAW_VERTS);
                    self.vertex_bufs[slot] = (ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vertex>(cap)), cap);
                }
                let vbuf = self.vertex_bufs[slot].0;
                slot += 1;
                ctx.buffer_update(vbuf, BufferSource::slice(chunk));
                let bindings = Bindings { vertex_buffers: vec![vbuf], index_buffer: self.index_buf, images: vec![tex] };
                ctx.apply_bindings(&bindings);
                let u = Mesh3DUniform { view_proj: mat_transpose(&view_proj) };
                ctx.apply_uniforms(UniformsSource::table(&u));
                ctx.draw(0, chunk.len() as i32, 1);
            }
        }
        ctx.end_render_pass();

        // 윤곽선 후처리 — 장면 색/깊이를 읽어 out_tex 에 검은 선을 얹어 쓴다.
        ctx.begin_pass(Some(self.out_pass), PassAction::Nothing);
        ctx.apply_viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_pipeline(&self.edge_pipeline);
        ctx.apply_bindings(&Bindings { vertex_buffers: vec![self.quad_vbuf], index_buffer: self.quad_ibuf, images: vec![self.color_tex, self.depth_tex] });
        let texel: [f32; 2] = [1.0 / self.size.0, 1.0 / self.size.1];
        ctx.apply_uniforms(UniformsSource::table(&texel));
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}
