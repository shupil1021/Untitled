//! 오프스크린 렌더 타깃 + CRT 셰이더(곡률/스캔라인/비네팅) + 4:3 필러박스.

use miniquad::*;

#[repr(C)]
#[derive(Clone, Copy)]
struct QuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
struct CrtUniform {
    tex_size: [f32; 2],
    time: f32,
    ca_amount: f32, // 색수차 강도 0..1 (0=끔)
    intensity: f32,
}

pub struct Crt {
    pass: RenderPass,
    color_tex: TextureId,
    pipeline: Pipeline,
    bindings: Bindings,
    size: (f32, f32),
}

impl Crt {
    pub fn new(ctx: &mut dyn RenderingBackend, vw: u32, vh: u32) -> Crt {
        let color_tex = ctx.new_render_texture(TextureParams {
            width: vw,
            height: vh,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        let pass = ctx.new_render_pass(color_tex, None);

        // 화면 전체를 덮는 사각형. uv 의 v 를 뒤집어 오프스크린 내용을 똑바로 세운다.
        let verts: [QuadVertex; 4] = [
            QuadVertex { pos: [-1.0,  1.0], uv: [0.0, 1.0] },
            QuadVertex { pos: [ 1.0,  1.0], uv: [1.0, 1.0] },
            QuadVertex { pos: [ 1.0, -1.0], uv: [1.0, 0.0] },
            QuadVertex { pos: [-1.0, -1.0], uv: [0.0, 0.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let vbuf = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&verts),
        );
        let ibuf = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: CRT_VS, fragment: CRT_FS },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("tex_size", UniformType::Float2),
                            UniformDesc::new("time", UniformType::Float1),
                            UniformDesc::new("ca_amount", UniformType::Float1),
                            UniformDesc::new("intensity", UniformType::Float1),
                        ],
                    },
                },
            )
            .expect("CRT 셰이더 컴파일 실패");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
            PipelineParams::default(),
        );

        let bindings = Bindings {
            vertex_buffers: vec![vbuf],
            index_buffer: ibuf,
            images: vec![color_tex],
        };

        Crt { pass, color_tex, pipeline, bindings, size: (vw as f32, vh as f32) }
    }

    // 오프스크린 렌더 타깃 해상도 변경. (설정의 해상도 값과 연결)
    pub fn set_resolution(&mut self, ctx: &mut dyn RenderingBackend, w: u32, h: u32) {
        ctx.delete_render_pass(self.pass);
        ctx.delete_texture(self.color_tex);
        self.color_tex = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        self.pass = ctx.new_render_pass(self.color_tex, None);
        self.size = (w as f32, h as f32);
    }

    // 오프스크린 렌더 타깃에 그리기 시작. (이후 renderer.flush → ctx.end_render_pass)
    pub fn begin(&self, ctx: &mut dyn RenderingBackend) {
        ctx.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        // 뷰포트는 FBO 바인딩과 무관한 별도 GL 상태라 자동으로 안 바뀐다. 명시적으로
        // 안 맞춰주면 직전 present() 에서 설정한 필러박스 뷰포트(실제 창 좌표 기준)가
        // 그대로 남아있어, 그래픽카드/드라이버에 따라 오프스크린 타깃의 일부에만
        // 그려지고 나머지는 이전 프레임 내용이 남거나 비어 보이는 문제가 생긴다.
        ctx.apply_viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_scissor_rect(0, 0, self.size.0 as i32, self.size.1 as i32);
    }

    // 오프스크린 결과를 CRT 효과로 실제 화면에 합성. (4:3 필러박스)
    pub fn present(&mut self, ctx: &mut dyn RenderingBackend, time: f32, ca_amount: f32, intensity: f32) {
        let (sw, sh) = window::screen_size();
        let (ox, oy, vw, vh) = viewport_4x3(sw, sh);

        ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        ctx.apply_pipeline(&self.pipeline);
        self.bindings.images[0] = self.color_tex;
        ctx.apply_bindings(&self.bindings);
        let u = CrtUniform {
            tex_size: [self.size.0, self.size.1],
            time,
            ca_amount: ca_amount.clamp(0.0, 1.0),
            intensity: intensity.clamp(0.0, 1.0),
        };
        ctx.apply_uniforms(UniformsSource::table(&u));
        ctx.apply_viewport(ox as i32, oy as i32, vw as i32, vh as i32);
        // begin() 에서 오프스크린용으로 좁힌 scissor 가 남아있지 않도록 필러박스 영역
        // 전체로 명시적으로 되돌린다 (역시 드라이버에 따라 자동으로 안 풀릴 수 있음).
        ctx.apply_scissor_rect(ox as i32, oy as i32, vw as i32, vh as i32);
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}

// CRT 셰이더와 동일한 배럴 왜곡. 입력/출력 모두 0..1 정규화 좌표.
// ⚠ 아래 프래그먼트 셰이더의 값(오버스캔 1.02, 곡률 0.11)과 반드시 일치시킬 것.
pub const OVERSCAN: f32 = 1.02;
pub const CURVE_K: f32 = 0.11;

pub fn warp(u: f32, v: f32) -> (f32, f32) {
    let uc = (u - 0.5) * OVERSCAN + 0.5;
    let vc = (v - 0.5) * OVERSCAN + 0.5;
    let cx = uc - 0.5;
    let cy = vc - 0.5;
    let dist = cx * cx + cy * cy;
    (uc + cx * dist * CURVE_K, vc + cy * dist * CURVE_K)
}

// 화면 안에 4:3 비율로 최대 크기의 영역을 중앙 배치. (좌우/상하 검은 여백)
pub fn viewport_4x3(sw: f32, sh: f32) -> (f32, f32, f32, f32) {
    let target = 4.0 / 3.0;
    let (mut vw, mut vh) = (sw, sh);
    if sw / sh > target {
        vw = sh * target;
    } else {
        vh = sw / target;
    }
    ((sw - vw) / 2.0, (sh - vh) / 2.0, vw, vh)
}

const CRT_VS: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_uv;
varying highp vec2 uv;
void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    uv = in_uv;
}
"#;

// 스캔라인/마스크는 sin(warped.y * tex_size.y * 3.14159 - ...) 처럼 화면 해상도
// (수백 단위)를 그대로 곱한 큰 값을 삼각함수에 넣는다. mediump 는 GLSL ES 스펙상
// 상대 정밀도만 보장(대략 2^-10)해서, 이렇게 큰 인자에서는 GPU/드라이버에 따라
// sin() 결과가 완전히 어긋나 스캔라인 대신 불규칙한 격자/무아레 잡음이 낀다 —
// "일부 컴퓨터에서만" 이상해 보이고 intensity(CRT 강도)를 낮추면 (그 잡음 항에
// 곱해지는 mix() 비율이 줄어들어) 괜찮아 보이는 게 딱 이 증상이다. 데스크톱
// GPU 에서는 highp 가 사실상 항상 완전한 32비트 float 라 안전하게 정밀도를
// 올릴 수 있다 — varying uv 도 lowp 로 보간되면 이미 그 시점부터 오차가 실려서
// highp 로 같이 올려야 한다(정점/프래그먼트 양쪽의 varying precision 은 반드시
// 일치해야 하는 GLSL ES 규칙이라 위 정점 셰이더도 같이 고쳤다).
const CRT_FS: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
uniform sampler2D tex;
uniform vec2 tex_size;
uniform float time;
uniform float ca_amount;
uniform float intensity;

void main() {
    // 오버스캔: 아주 살짝 축소해 얇은 검은 베젤을 남긴다.
    vec2 uvc = (uv - 0.5) * 1.02 + 0.5;
    // 배럴(볼록) 왜곡: 중심에서 멀수록 바깥으로 밀어 곡면 모니터처럼.
    // 값이 크면 가장자리 UI(작업표시줄 등)가 잘리므로 약하게.
    vec2 cc = uvc - 0.5;
    float dist = dot(cc, cc);
    vec2 warped = uvc + cc * dist * 0.11;

    // 화면 밖은 검게(모니터 베젤 안쪽 여백).
    if (warped.x < 0.0 || warped.x > 1.0 || warped.y < 0.0 || warped.y > 1.0) {
        gl_FragColor = vec4(0.02, 0.02, 0.02, 1.0);
        return;
    }

    // 색수차: 중심에서 멀어질수록 R/G/B 를 살짝 다른 지점에서 샘플링해 가장자리가
    // 번져 보이게 한다 — 브라운관 특유의 느낌을 가장 직관적으로 살려주는 효과.
    // ca_amount 가 0 이면 세 채널 다 같은 지점을 샘플링해서 자연스럽게 "끔" 이 된다.
    vec2 ca_dir = cc / (length(cc) + 0.0001);
    float ca = dist * 0.02 * ca_amount;
    vec2 rOff = ca_dir * ca;
    vec2 bOff = ca_dir * ca;
    float rC = texture2D(tex, warped + rOff).r;
    float gC = texture2D(tex, warped).g;
    float bC = texture2D(tex, warped - bOff).b;
    vec3 col = vec3(rC, gC, bC);

    // 스캔라인: 세로로 또렷하게 어두운 줄무늬 (도트가 살아있는 느낌 강화).
    // intensity 로 "효과 없음(1.0)" 과 "기본 세기" 사이를 섞는다 — 설정에서 강도를
    // 낮추면 스캔라인/마스크/비네팅이 다 같이 옅어진다.
    float scan = 0.5 + 0.5 * sin(warped.y * tex_size.y * 3.14159 - time * 2.0);
    col *= mix(1.0, 0.68 + 0.32 * scan, intensity);

    // 새도우 마스크 비슷한 가로 RGB 미세 변조 — 이전보다 대비를 키웠다.
    float mask = 0.8 + 0.2 * sin(warped.x * tex_size.x * 3.14159);
    col *= mix(1.0, mask, intensity);

    // 비네팅: 가장자리 어둡게.
    float vig = clamp(1.0 - dist * 0.8, 0.0, 1.0);
    col *= mix(1.0, vig, intensity);

    // 전체 밝기 보정 — 스캔라인/마스크를 더 진하게 만든 만큼 살짝 더 올렸다.
    // intensity 가 낮을수록 애초에 어둡게 만드는 게 적으니 보정도 그만큼 줄인다.
    col *= mix(1.0, 1.22, intensity);

    gl_FragColor = vec4(col, 1.0);
}
"#;
