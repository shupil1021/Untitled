// 콘솔 창 없이 뜨게(GUI 앱으로) — 이게 없으면 실행할 때마다 검은 콘솔 창이
// 게임 창과 같이 떠서 연출용치고 지저분해 보인다.
#![windows_subsystem = "windows"]

// PalaceOS Director — 연출/영상 제작 전용 게임 화면 창.
//
// 실제 게임(crackhead.exe, src/main.rs)과는 별개의 실행 파일이다 — 게임 코드는
// 전혀 안 건드리고, src/lib.rs 가 공개하는 같은 씬/렌더러 모듈을 그대로 가져다
// 써서 특정 씬을 바로 띄우거나(Boot/Lobby/Desktop/Erase/Shutdown) 로비 화면의
// 정전기/글리치를 실행 중에 껐다 켤 수 있게 해준다 — 트레일러/스크린샷처럼
// 원하는 장면만 깔끔하게 촬영하고 싶을 때 쓴다.
//
// 조작은 이 창 안이 아니라 director_panel.exe(옆에 따로 뜨는 작은 컨트롤
// 창)에서 한다 — 그래야 이 창(=실제로 녹화할 화면)엔 컨트롤 UI가 전혀 안
// 찍힌다. 두 창은 직접 함수를 부를 수 없는 별개 프로세스라, director_ipc 가
// 정의하는 작은 공유 JSON 파일로 통신한다 — panel 에서 버튼을 누르면 그 파일에
// 써두고, 여기서 매 프레임 다시 읽어서 반영한다. 시작할 때 panel 을 자동으로
// 같이 띄운다(실패해도 조용히 넘어간다 — 수동으로 따로 켜도 된다).

use std::cell::RefCell;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use image::ImageEncoder;
use miniquad::*;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED};

use crackhead::crt::{viewport_4x3, warp};
use crackhead::foundation::{self, Settings, FPS_OPTS, RES_OPTS};
use crackhead::gfx::{Assets, Renderer, VIRTUAL_H as VH, VIRTUAL_W as VW};
use crackhead::scenes::{BlueScreenScene, BootScene, DesktopScene, EraseScene, Frame, Input, LobbyScene, Scene, SceneManager, ShutdownScene};
use crackhead::ui;
use crackhead::director_ipc;

// 로비 화면 자체의 정전기/글리치(scenes/lobby.rs)는 실제 게임과 똑같이
// 항상 자동으로 나오는, panel 로는 설정할 수 없는 그 화면 고유의 연출로
// 둔다(LobbyScene::new() — 평소 게임이 쓰는 것과 완전히 같은 생성자).
// Glitch/Noise 토글은 그것과는 별개로, 지금 보고 있는 씬이 뭐든 상관없이
// director 자신이 화면 위에 매 프레임 직접 덧그리는 오버레이로 처리한다
// (draw_overlay_effects, 아래 Rng/오버레이 필드 참고) — 로비에서는 그 둘이
// 같이(자동 연출 + 토글 오버레이) 보일 수 있다는 뜻이고, 다른 씬에서는
// 오버레이만 보인다.
fn make_scene(name: &str) -> Box<dyn Scene> {
    match name {
        "Boot" => Box::new(BootScene::new()),
        "Desktop" => Box::new(DesktopScene::new()),
        "Erase" => Box::new(EraseScene::new()),
        "Shutdown" => Box::new(ShutdownScene::new()),
        "BlueScreen" => Box::new(BlueScreenScene::new()),
        _ => Box::new(LobbyScene::new()),
    }
}

// 아주 단순한 xorshift64 의사난수 — scenes/lobby.rs, scenes/boot.rs 와 같은
// 이유로 공유 모듈로 안 뽑고 여기서도 작게 따로 둔다.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (self.next_u32() % 1_000_000) as f32 / 1_000_000.0 * (max - min)
    }
}

// director 전용 CRT — src/crt.rs::Crt 를 복사해서 글리치 줄무늬 셰이더를
// 얹은 것이다. 처음엔 crt.rs 자체(=실제 게임과 공유하는 코드)에 이 기능을
// 넣으려다가, 셰이더 컴파일이 실패하면서(정확한 GLSL 원인은 못 밝혔다)
// director 뿐 아니라 실제 게임(crackhead.exe)까지 실행하자마자 꺼지는
// 사고로 이어졌다 — 그래서 crt.rs 는 이 세션에서 글리치 기능이 생기기
// 전 상태로 완전히 되돌리고, 이 실험적인 셰이더는 여기 director.rs 안에만
// 따로 둔다. 이러면 이 셰이더가 또 실패해도(가능성은 낮다고 보지만
// 마지막에도 그렇게 생각했다가 틀렸다) director.exe 하나만 안 뜨고,
// 실제 게임은 이 파일이 존재하는지조차 모른다(별도 실행 파일이라 아예
// 컴파일도 안 된다).
#[repr(C)]
#[derive(Clone, Copy)]
struct DirectorQuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

// 글리치 셰이더 컴파일이 실패했을 때 대신 쓰는, 유니폼 4개짜리 기본 CRT
// 셰이더용 레이아웃 — src/crt.rs::CrtUniform 과 완전히 같은 모양이다.
#[repr(C)]
struct SafeCrtUniform {
    tex_size: [f32; 2],
    time: f32,
    ca_amount: f32,
    intensity: f32,
}

#[repr(C)]
struct DirectorCrtUniform {
    tex_size: [f32; 2],
    time: f32,
    ca_amount: f32,
    intensity: f32,
    glitch_on: f32,
    glitch_seed: f32,
    glitch_freq: f32,
    glitch_fill: f32,
    glitch_shift: f32,
    glitch_chroma: f32,
    glitch_vshift: f32,
}

// present()/present_to_record() 인자 묶음 — clippy::too_many_arguments 를
// 피하려고 시간/색수차/강도/글리치를 하나로 묶었을 뿐, 의미상 특별한 건 없다.
#[derive(Clone, Copy)]
struct PresentParams {
    time: f32,
    ca_amount: f32,
    intensity: f32,
    glitch: Option<GlitchParams>,
}

#[derive(Default, Clone, Copy)]
struct GlitchParams {
    seed: f32,
    freq: f32,
    fill: f32,
    shift: f32,
    vshift: f32,
    chroma: f32,
}

// 글리치 셰이더가 이번에도 컴파일에 실패하면(GPU/드라이버마다 결과가 다를
// 수 있다), director.exe 가 아예 안 뜨는 대신 실제 원인 메시지를 exe 옆에
// 남기고 글리치 없는 기본 CRT 셰이더로 조용히 전환한다 — "안 뜬다"보다
// "글리치만 빠진 채로 뜬다"가 훨씬 낫고, 이 로그 파일 덕분에 다음엔 추측이
// 아니라 실제 컴파일러 메시지를 보고 고칠 수 있다.
fn log_shader_error(err: &ShaderError) {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let _ = std::fs::write(dir.join("director_shader_error.log"), format!("{err}"));
    }
}

// 녹화용 오프스크린 타깃 해상도 — 4:3, 화면 창 크기와 무관하게 항상 이
// 크기로 PNG 를 저장한다(창을 리사이즈해도 녹화 결과물 크기가 안 바뀜).
const RECORD_W: u32 = 1280;
const RECORD_H: u32 = 960;

struct DirectorCrt {
    pass: RenderPass,
    color_tex: TextureId,
    pipeline: Pipeline,
    bindings: Bindings,
    size: (f32, f32),
    // 글리치 셰이더 컴파일에 성공했는지 — 실패했으면 아래 present() 가 유니폼
    // 개수가 다른 "기본 CRT 셰이더용" 값을 대신 보낸다(그렇지 않으면 셰이더가
    // 기대하는 유니폼 레이아웃과 안 맞아서 또 다른 문제가 생길 수 있다).
    glitch_supported: bool,
    // 녹화 전용 렌더 타깃 — 화면에 실제로 보이는 필러박스 창과는 별개로, 항상
    // 고정된 RECORD_W x RECORD_H 크기로 같은 CRT 결과를 한 번 더 그려서
    // texture_read_pixels 로 픽셀을 뽑아 PNG 로 저장한다.
    record_pass: RenderPass,
    record_tex: TextureId,
}

impl DirectorCrt {
    fn new(ctx: &mut dyn RenderingBackend, vw: u32, vh: u32) -> DirectorCrt {
        let color_tex = ctx.new_render_texture(TextureParams {
            width: vw,
            height: vh,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        let pass = ctx.new_render_pass(color_tex, None);
        let verts: [DirectorQuadVertex; 4] = [
            DirectorQuadVertex { pos: [-1.0, 1.0], uv: [0.0, 1.0] },
            DirectorQuadVertex { pos: [1.0, 1.0], uv: [1.0, 1.0] },
            DirectorQuadVertex { pos: [1.0, -1.0], uv: [1.0, 0.0] },
            DirectorQuadVertex { pos: [-1.0, -1.0], uv: [0.0, 0.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let vbuf = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&verts));
        let ibuf = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));

        let glitch_result = ctx.new_shader(
            ShaderSource::Glsl { vertex: DIRECTOR_CRT_VS, fragment: DIRECTOR_CRT_FS },
            ShaderMeta {
                images: vec!["tex".to_string()],
                uniforms: UniformBlockLayout {
                    uniforms: vec![
                        UniformDesc::new("tex_size", UniformType::Float2),
                        UniformDesc::new("time", UniformType::Float1),
                        UniformDesc::new("ca_amount", UniformType::Float1),
                        UniformDesc::new("intensity", UniformType::Float1),
                        UniformDesc::new("glitch_on", UniformType::Float1),
                        UniformDesc::new("glitch_seed", UniformType::Float1),
                        UniformDesc::new("glitch_freq", UniformType::Float1),
                        UniformDesc::new("glitch_fill", UniformType::Float1),
                        UniformDesc::new("glitch_shift", UniformType::Float1),
                        UniformDesc::new("glitch_chroma", UniformType::Float1),
                        UniformDesc::new("glitch_vshift", UniformType::Float1),
                    ],
                },
            },
        );
        let (shader, glitch_supported) = match glitch_result {
            Ok(s) => (s, true),
            Err(e) => {
                log_shader_error(&e);
                let safe = ctx
                    .new_shader(
                        ShaderSource::Glsl { vertex: DIRECTOR_CRT_VS, fragment: DIRECTOR_CRT_FS_SAFE },
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
                    .expect("director 기본 CRT 셰이더(글리치 없음)도 컴파일 실패");
                (safe, false)
            }
        };

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("in_pos", VertexFormat::Float2), VertexAttribute::new("in_uv", VertexFormat::Float2)],
            shader,
            PipelineParams::default(),
        );

        let bindings = Bindings { vertex_buffers: vec![vbuf], index_buffer: ibuf, images: vec![color_tex] };

        let record_tex = ctx.new_render_texture(TextureParams {
            width: RECORD_W,
            height: RECORD_H,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        let record_pass = ctx.new_render_pass(record_tex, None);

        DirectorCrt { pass, color_tex, pipeline, bindings, size: (vw as f32, vh as f32), glitch_supported, record_pass, record_tex }
    }

    fn set_resolution(&mut self, ctx: &mut dyn RenderingBackend, w: u32, h: u32) {
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

    fn begin(&self, ctx: &mut dyn RenderingBackend) {
        ctx.begin_pass(Some(self.pass), PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        ctx.apply_viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_scissor_rect(0, 0, self.size.0 as i32, self.size.1 as i32);
    }

    fn present(&mut self, ctx: &mut dyn RenderingBackend, p: PresentParams) {
        let (sw, sh) = window::screen_size();
        let (ox, oy, vw, vh) = viewport_4x3(sw, sh);
        self.draw_pass(ctx, None, (ox as i32, oy as i32, vw as i32, vh as i32), p);
    }

    // present() 와 완전히 같은 CRT 결과를, 화면 대신 고정 크기(RECORD_W x
    // RECORD_H)의 record_tex 에 한 번 더 그린다 — 창 크기/필러박스와 무관하게
    // 항상 같은 해상도로 녹화하기 위함. capture_record_pixels() 로 이어서
    // 픽셀을 읽어 PNG 로 저장한다(director.rs::DirectorStage 쪽에서 처리).
    fn present_to_record(&mut self, ctx: &mut dyn RenderingBackend, p: PresentParams) {
        self.draw_pass(ctx, Some(self.record_pass), (0, 0, RECORD_W as i32, RECORD_H as i32), p);
    }

    fn capture_record_pixels(&mut self, ctx: &mut dyn RenderingBackend) -> Vec<u8> {
        let mut buf = vec![0u8; (RECORD_W * RECORD_H * 4) as usize];
        ctx.texture_read_pixels(self.record_tex, &mut buf);
        buf
    }

    fn draw_pass(&mut self, ctx: &mut dyn RenderingBackend, pass: Option<RenderPass>, viewport: (i32, i32, i32, i32), p: PresentParams) {
        let PresentParams { time, ca_amount, intensity, glitch } = p;
        let (ox, oy, vw, vh) = viewport;
        match pass {
            Some(p) => ctx.begin_pass(Some(p), PassAction::clear_color(0.0, 0.0, 0.0, 1.0)),
            None => ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0)),
        }
        ctx.apply_pipeline(&self.pipeline);
        self.bindings.images[0] = self.color_tex;
        ctx.apply_bindings(&self.bindings);
        if self.glitch_supported {
            let (glitch_on, g) = match glitch {
                Some(g) => (1.0, g),
                None => (0.0, GlitchParams::default()),
            };
            let u = DirectorCrtUniform {
                tex_size: [self.size.0, self.size.1],
                time,
                ca_amount: ca_amount.clamp(0.0, 1.0),
                intensity: intensity.clamp(0.0, 1.0),
                glitch_on,
                glitch_seed: g.seed,
                glitch_freq: g.freq,
                glitch_fill: g.fill.clamp(0.0, 1.0),
                glitch_shift: g.shift,
                glitch_chroma: g.chroma,
                glitch_vshift: g.vshift,
            };
            ctx.apply_uniforms(UniformsSource::table(&u));
        } else {
            let u = SafeCrtUniform {
                tex_size: [self.size.0, self.size.1],
                time,
                ca_amount: ca_amount.clamp(0.0, 1.0),
                intensity: intensity.clamp(0.0, 1.0),
            };
            ctx.apply_uniforms(UniformsSource::table(&u));
        }
        ctx.apply_viewport(ox, oy, vw, vh);
        ctx.apply_scissor_rect(ox, oy, vw, vh);
        ctx.draw(0, 6, 1);
        ctx.end_render_pass();
    }
}

const DIRECTOR_CRT_VS: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_uv;
varying lowp vec2 uv;
void main() {
    gl_Position = vec4(in_pos, 0.0, 1.0);
    uv = in_uv;
}
"#;

const DIRECTOR_CRT_FS: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
uniform sampler2D tex;
uniform vec2 tex_size;
uniform float time;
uniform float ca_amount;
uniform float intensity;
uniform float glitch_on;
uniform float glitch_seed;
uniform float glitch_freq;
uniform float glitch_fill;
uniform float glitch_shift;
uniform float glitch_chroma;
uniform float glitch_vshift;

float ghash(float n) {
    return fract(sin(n) * 43758.5453123);
}

void main() {
    vec2 uvc = (uv - 0.5) * 1.02 + 0.5;
    vec2 cc = uvc - 0.5;
    float dist = dot(cc, cc);
    vec2 warped = uvc + cc * dist * 0.11;

    if (warped.x < 0.0 || warped.x > 1.0 || warped.y < 0.0 || warped.y > 1.0) {
        gl_FragColor = vec4(0.02, 0.02, 0.02, 1.0);
        return;
    }

    vec2 sampleUv = warped;
    float chromaBoost = 1.0;
    if (glitch_on > 0.5) {
        float row = floor(warped.y * glitch_freq);
        float roll = ghash(row * 12.9898 + glitch_seed);
        float hit = step(roll, glitch_fill);
        float dirSign = mix(-1.0, 1.0, step(0.5, ghash(row * 78.233 + glitch_seed)));
        float mag = 0.3 + 0.7 * ghash(row * 37.719 + glitch_seed);
        float shiftAmt = hit * dirSign * glitch_shift * mag;
        sampleUv.x = fract(sampleUv.x + shiftAmt);
        float vDirSign = mix(-1.0, 1.0, step(0.5, ghash(row * 91.345 + glitch_seed)));
        float vShiftAmt = hit * vDirSign * glitch_vshift * mag;
        sampleUv.y = clamp(sampleUv.y + vShiftAmt, 0.0, 1.0);
        chromaBoost = 1.0 + hit * glitch_chroma * mag;
    }

    vec2 ca_dir = cc / (length(cc) + 0.0001);
    float ca = dist * 0.02 * ca_amount + (chromaBoost - 1.0) * 0.015;
    vec2 rOff = ca_dir * ca;
    vec2 bOff = ca_dir * ca;
    float rC = texture2D(tex, sampleUv + rOff).r;
    float gC = texture2D(tex, sampleUv).g;
    float bC = texture2D(tex, sampleUv - bOff).b;
    vec3 col = vec3(rC, gC, bC);

    float scan = 0.5 + 0.5 * sin(warped.y * tex_size.y * 3.14159 - time * 2.0);
    col *= mix(1.0, 0.68 + 0.32 * scan, intensity);

    float mask = 0.8 + 0.2 * sin(warped.x * tex_size.x * 3.14159);
    col *= mix(1.0, mask, intensity);

    float vig = clamp(1.0 - dist * 0.8, 0.0, 1.0);
    col *= mix(1.0, vig, intensity);

    col *= mix(1.0, 1.22, intensity);

    gl_FragColor = vec4(col, 1.0);
}
"#;

// DIRECTOR_CRT_FS(글리치 포함) 컴파일이 실패했을 때 대신 쓰는, src/crt.rs
// 의 원래 CRT_FS 와 완전히 같은(글리치 없는) 셰이더 — 이미 여러 번 실제로
// 잘 작동한 걸로 확인된 코드라 이쪽은 컴파일 실패 위험이 거의 없다.
const DIRECTOR_CRT_FS_SAFE: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
uniform sampler2D tex;
uniform vec2 tex_size;
uniform float time;
uniform float ca_amount;
uniform float intensity;

void main() {
    vec2 uvc = (uv - 0.5) * 1.02 + 0.5;
    vec2 cc = uvc - 0.5;
    float dist = dot(cc, cc);
    vec2 warped = uvc + cc * dist * 0.11;

    if (warped.x < 0.0 || warped.x > 1.0 || warped.y < 0.0 || warped.y > 1.0) {
        gl_FragColor = vec4(0.02, 0.02, 0.02, 1.0);
        return;
    }

    vec2 ca_dir = cc / (length(cc) + 0.0001);
    float ca = dist * 0.02 * ca_amount;
    vec2 rOff = ca_dir * ca;
    vec2 bOff = ca_dir * ca;
    float rC = texture2D(tex, warped + rOff).r;
    float gC = texture2D(tex, warped).g;
    float bC = texture2D(tex, warped - bOff).b;
    vec3 col = vec3(rC, gC, bC);

    float scan = 0.5 + 0.5 * sin(warped.y * tex_size.y * 3.14159 - time * 2.0);
    col *= mix(1.0, 0.68 + 0.32 * scan, intensity);

    float mask = 0.8 + 0.2 * sin(warped.x * tex_size.x * 3.14159);
    col *= mix(1.0, mask, intensity);

    float vig = clamp(1.0 - dist * 0.8, 0.0, 1.0);
    col *= mix(1.0, vig, intensity);

    col *= mix(1.0, 1.22, intensity);

    gl_FragColor = vec4(col, 1.0);
}
"#;

const NOISE_COUNT: usize = 220;
const GLITCH_BURST: f32 = 0.14;

// glitch_frequency(0.0=뜸하게~1.0=잦게)를 실제 "다음 글리치까지 대기(초)"
// 범위로 바꾼다 — 0% 는 한참 기다렸다 한 번씩, 100% 는 거의 쉴 새 없이
// 터진다. 0.5(기본값)가 이전(주기 조절이 생기기 전 고정값이던 1.4~3.6초)
// 과 비슷한 "중간" 정도가 되도록 양 끝을 잡았다.
fn glitch_gap_range(freq: f32) -> (f32, f32) {
    let f = freq.clamp(0.0, 1.0);
    let gap_min = 4.0 + (0.25 - 4.0) * f;
    let gap_max = 8.0 + (0.9 - 8.0) * f;
    (gap_min, gap_max)
}

// director_panel 을 같은 폴더에서 찾아 자동으로 같이 띄운다 — 못 찾거나
// 실행에 실패해도(예: 아직 따로 안 빌드해둔 경우) 조용히 무시한다. 이 창
// 자체는 panel 없이도 정상 동작하니(그냥 director_state.json 이 기본값에
// 머물 뿐) 실패가 치명적이지 않다. cargo 가 만드는 기본 이름
// (director_panel.exe)과, production/ 폴더에 배포용으로 다시 이름 붙인
// 파일(PalaceOS-Director-Panel.exe) 둘 다 찾아본다 — 후자만 있고 전자가
// 없으면 자동 실행이 조용히 안 되던 문제가 있었다.
fn spawn_panel() {
    let Ok(exe) = std::env::current_exe() else { return };
    let Some(dir) = exe.parent() else { return };
    let candidates: [&str; 2] =
        if cfg!(windows) { ["director_panel.exe", "PalaceOS-Director-Panel.exe"] } else { ["director_panel", "director_panel"] };
    for name in candidates {
        let panel = dir.join(name);
        if panel.exists() {
            let _ = std::process::Command::new(panel).spawn();
            return;
        }
    }
}

// exe 옆 recordings/ 폴더 안에 "rec_YYYYMMDD_HHMMSS" 이름으로 새 폴더를 만들고
// 그 경로를 반환한다 — panel 에서 Record 버튼을 누를 때마다(꺼졌다가 다시
// 켜질 때마다) 매번 새 폴더에 저장해서 이전 촬영분을 덮어쓰지 않는다.
fn start_recording() -> PathBuf {
    let base = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("recordings")))
        .unwrap_or_else(|| PathBuf::from("recordings"));
    let now = date::now();
    let secs = now as i64;
    let millis = ((now.fract()) * 1000.0) as i64;
    let dir = base.join(format!("rec_{secs}_{millis:03}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// PNG 인코딩(+디스크 쓰기)을 draw() 가 도는 메인 스레드에서 그대로 하면 한
// 프레임 저장에 걸리는 시간만큼 게임 자체가 멎어버린다 — 1280x960 짜리
// 이미지를, 그것도 글리치/스캔라인처럼 잘 안 눌리는 고주파 화면을 매번
// 압축하다 보면 그 정지 시간이 누적돼서 "10초를 녹화했는데 실제로는
// 몇 프레임밖에 못 찍고 그마저도 초반 1초 안팎에 몰려 있었다"는 문제가
// 생겼다. 그래서 메인 스레드는 (GPU 읽기까지만 하고) 원본 픽셀을 이
// 전용 스레드로 넘기기만 하고 바로 다음 프레임으로 진행하고, 실제
// PNG 인코딩+저장은 여기서 순서대로(들어온 순서 그대로) 처리한다.
struct FrameWriter {
    tx: std::sync::mpsc::Sender<(u32, Vec<u8>)>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl FrameWriter {
    fn start(dir: PathBuf) -> FrameWriter {
        let (tx, rx) = std::sync::mpsc::channel::<(u32, Vec<u8>)>();
        let handle = std::thread::spawn(move || {
            for (idx, pixels) in rx {
                let path = dir.join(format!("frame_{idx:06}.jpg"));
                // 프레임 하나가 무슨 이유로든(디스크 꽉 참, 인코딩 실패, 예상 못한
                // panic 등) 실패해도 이 스레드 자체가 죽으면 안 된다 — 스레드가
                // 죽으면 그 뒤로 들어오는 모든 프레임이 채널 너머로 조용히
                // 버려져서(send 실패를 메인 쪽에서 무시하게 해뒀다), "녹화가
                // 특정 시점 이후로 뚝 끊긴다"는 증상으로 이어진다(예: 씬 전환
                // 직후 한 프레임에서만 뭔가 어긋나도 그 뒤 전부가 사라짐). 그래서
                // 프레임 하나의 실패를 이 루프 자체를 끝내지 않고 로그만 남기고
                // 다음 프레임으로 넘어가도록 catch_unwind 로 감싼다.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| save_frame_jpg(&path, RECORD_W, RECORD_H, &pixels)));
                let err: Option<String> = match result {
                    Ok(Ok(())) => None,
                    Ok(Err(e)) => Some(e),
                    Err(panic) => Some(
                        panic
                            .downcast_ref::<&str>()
                            .map(|s| s.to_string())
                            .or_else(|| panic.downcast_ref::<String>().cloned())
                            .unwrap_or_else(|| "알 수 없는 panic".to_string()),
                    ),
                };
                if let Some(msg) = err {
                    let log = dir.join("frame_writer_error.log");
                    let line = format!("frame {idx}: {msg}\n");
                    let _ = std::fs::OpenOptions::new().create(true).append(true).open(&log).and_then(|mut f| f.write_all(line.as_bytes()));
                }
            }
        });
        FrameWriter { tx, handle: Some(handle) }
    }

    fn send(&self, idx: u32, pixels: Vec<u8>) {
        let _ = self.tx.send((idx, pixels));
    }

    // 채널의 송신 쪽을 먼저 명시적으로 닫아야(drop) 스레드 쪽 for 루프가
    // 끝난다 — 그래야 대기 중이던 프레임들까지 전부 디스크에 다 쓰고 나서
    // join 이 끝난다(그 다음에야 mux_avi 를 돌려도 안전하다).
    fn finish(self) {
        let FrameWriter { tx, mut handle } = self;
        drop(tx);
        if let Some(h) = handle.take() {
            let _ = h.join();
        }
    }
}

// WASAPI 루프백 캡처로 시스템 출력음(스피커로 나가는 소리)을 녹화 내내
// 별도 스레드에서 그대로 받아 exe 옆 recordings/.../audio.wav 로 흘려
// 쓴다. director 화면 자체에서 나는 소리를 녹음하는 게 목적이라 마이크가
// 아니라 "렌더(출력) 장치를 루프백 모드로 여는" 방식을 쓴다 — 실제 게임
// 오디오 재생(video.rs)과 거의 같은 WASAPI 초기화 패턴이지만, 여긴 재생이
// 아니라 캡처라 IAudioCaptureClient 를 쓴다는 점만 다르다.
struct AudioCapture {
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AudioCapture {
    fn start(wav_path: PathBuf) -> AudioCapture {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = std::thread::spawn(move || {
            if let Err(e) = audio_capture_loop(&wav_path, &stop2) {
                // 실패해도 화면 녹화(PNG) 는 그대로 계속돼야 하니 조용히 로그만 남긴다.
                let log = wav_path.with_file_name("audio_capture_error.log");
                let _ = std::fs::write(log, format!("{e}"));
            }
        });
        AudioCapture { stop, handle: Some(handle) }
    }

    fn stop_and_join(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn write_wav_header<W: Write>(w: &mut W, channels: u16, sample_rate: u32, bits: u16, is_float: bool, data_len: u32) -> std::io::Result<()> {
    // WASAPI 믹스 포맷은 WAVE_FORMAT_EXTENSIBLE 로 오는 경우가 대부분인데,
    // 굳이 그 확장 청크까지 그대로 옮기지 않고 표준 16바이트 fmt 청크(PCM=1,
    // IEEE float=3)로 "납작하게" 펴서 쓴다 — 플레이어/ffmpeg 호환성이 더 좋다.
    let format_tag: u16 = if is_float { 3 } else { 1 };
    let block_align = channels * (bits / 8);
    let byte_rate = sample_rate * block_align as u32;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVE")?;
    w.write_all(b"fmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&format_tag.to_le_bytes())?;
    w.write_all(&channels.to_le_bytes())?;
    w.write_all(&sample_rate.to_le_bytes())?;
    w.write_all(&byte_rate.to_le_bytes())?;
    w.write_all(&block_align.to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())?;
    Ok(())
}

fn audio_capture_loop(wav_path: &std::path::Path, stop: &AtomicBool) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        // COM 은 스레드마다 각자 초기화해야 한다(video.rs 의 재생 스레드와 동일한 이유).
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;

        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        // 마이크가 아니라 "지금 스피커로 나가고 있는 소리"를 그대로 받는 것이 목적이라,
        // 렌더(출력) 장치를 잡되 Initialize 에 LOOPBACK 플래그를 준다.
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let wfx_ptr = client.GetMixFormat()?;
        let wfx = *wfx_ptr;
        let channels = wfx.nChannels;
        let sample_rate = wfx.nSamplesPerSec;
        let bits = wfx.wBitsPerSample;
        let block_align = wfx.nBlockAlign as u32;
        // 오늘날 WASAPI 믹스 포맷은 사실상 항상 32비트 IEEE float 다 — SubFormat
        // GUID 를 따로 비교하지 않고 비트뎁스만으로 판별해도 충분히 안전하다.
        let is_float = bits == 32;

        client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 5_000_000, 0, &wfx, None)?;
        let capture: IAudioCaptureClient = client.GetService()?;

        let file = std::fs::File::create(wav_path)?;
        let mut w = std::io::BufWriter::new(file);
        write_wav_header(&mut w, channels, sample_rate, bits, is_float, 0)?;
        let mut data_len: u64 = 0;

        client.Start()?;
        while !stop.load(Ordering::Relaxed) {
            let packet_size = capture.GetNextPacketSize()?;
            if packet_size == 0 {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut num_frames = 0u32;
            let mut flags = 0u32;
            capture.GetBuffer(&mut data_ptr, &mut num_frames, &mut flags, None, None)?;
            let bytes = num_frames as usize * block_align as usize;
            if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data_ptr.is_null() {
                // 무음 구간도 프레임 수만큼 0 으로 채워 써야 이후 이어질 소리와
                // 타이밍이 안 어긋난다.
                w.write_all(&vec![0u8; bytes])?;
            } else {
                w.write_all(std::slice::from_raw_parts(data_ptr, bytes))?;
            }
            data_len += bytes as u64;
            capture.ReleaseBuffer(num_frames)?;
        }
        client.Stop()?;

        // 처음에 0으로 써둔 data 길이를 실제 캡처한 만큼으로 되돌아가 고쳐 쓴다.
        w.seek(SeekFrom::Start(0))?;
        write_wav_header(&mut w, channels, sample_rate, bits, is_float, data_len.min(u32::MAX as u64) as u32)?;
        w.flush()?;

        CoUninitialize();
    }
    Ok(())
}

// 세로로 뒤집힌(OpenGL 텍스처 원점이 아래쪽이라 texture_read_pixels 결과의
// 첫 행이 이미지의 맨 아래 줄) RGBA8 픽셀을 받아 똑바로 세우면서 동시에
// 알파 채널을 버려(JPEG 은 알파가 없음) RGB 로 만든 뒤 JPEG 로 저장한다.
//
// 처음엔 무손실 PNG 로 저장했는데(프레임마다 1.6~4MB), 10초짜리 녹화도
// 금방 수 GB 로 불어나서 — output.avi 컨테이너 자체의 4GB 한계에
// 자꾸 부딪히고(파트를 여러 개로 쪼개야 했다), 디스크 쓰기 자체도
// 느려서 캡처 가능한 초당 프레임 수까지 깎아먹었다. JPEG(품질 85) 로
// 바꾸면 화질은 약간 손해 보지만 파일이 보통 1/10 이하로 줄어서, 왠만한
// 길이의 녹화는 4GB 한계에 아예 안 걸리고, 저장 자체도 더 빨라진다.
fn save_frame_jpg(path: &std::path::Path, w: u32, h: u32, pixels: &[u8]) -> Result<(), String> {
    let row_bytes = (w * 4) as usize;
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    for y in 0..h as usize {
        let dst_y = h as usize - 1 - y;
        let src_row = &pixels[y * row_bytes..(y + 1) * row_bytes];
        let dst_row = &mut rgb[dst_y * w as usize * 3..(dst_y + 1) * w as usize * 3];
        for x in 0..w as usize {
            let s = &src_row[x * 4..x * 4 + 4];
            let o = x * 3;
            dst_row[o] = s[0];
            dst_row[o + 1] = s[1];
            dst_row[o + 2] = s[2];
        }
    }
    let file = std::fs::File::create(path).map_err(|e| format!("파일 생성 실패: {e}"))?;
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 85);
    encoder.write_image(&rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| format!("JPEG 인코딩 실패: {e}"))
}

// wav 파일을 읽어 (채널 수, 샘플레이트, 비트뎁스, PCM 데이터) 를 돌려준다.
// write_wav_header() 가 쓰는 포맷을 그대로 다시 읽는 용도라 44바이트 고정
// 오프셋으로 짧게 끝낼 수도 있었지만, 혹시 나중에 헤더 형식이 바뀌어도 안
// 깨지도록 그냥 청크를 하나씩 걸어가며 "fmt "/"data" 를 찾는다.
fn read_wav(path: &std::path::Path) -> Option<(u16, u32, u16, Vec<u8>)> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12usize;
    let (mut channels, mut sample_rate, mut bits) = (0u16, 0u32, 0u16);
    let mut data = Vec::new();
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let body_start = pos + 8;
        let body_end = (body_start + size).min(bytes.len());
        if id == b"fmt " && body_end >= body_start + 16 {
            channels = u16::from_le_bytes(bytes[body_start + 2..body_start + 4].try_into().ok()?);
            sample_rate = u32::from_le_bytes(bytes[body_start + 4..body_start + 8].try_into().ok()?);
            bits = u16::from_le_bytes(bytes[body_start + 14..body_start + 16].try_into().ok()?);
        } else if id == b"data" {
            data = bytes[body_start..body_end].to_vec();
        }
        pos = body_end + (size % 2); // RIFF 청크는 짝수 정렬 — 홀수 길이면 패딩 1바이트
    }
    Some((channels, sample_rate, bits, data))
}

// RGBA(위→아래로 저장된, save_frame_png 의 결과와 같은 순서) 픽셀을 Windows
// DIB 이 기대하는 BGRA32 순서로 바꾼다. 처음엔 24bpp(BGR24, 행마다 4바이트
// AVI1.0(RIFF) 은 청크 크기·idx1 오프셋이 전부 32비트 필드라, 파일 하나가
// 4GB 를 넘으면 그 이후 프레임의 idx1 오프셋이 조용히 넘쳐서(overflow, 특히
// release 빌드는 오버플로 검사가 꺼져 있어 panic 도 안 남고 그냥 잘못된 값이
// 된다) 완전히 엉뚱한 위치를 가리키게 된다 — 플레이어가 그 지점부터 못
// 읽거나 재생을 멈춰버린다("영상이 중간에 끝난다"는 제보의 진짜 원인이었다,
// 실제로 확인해보니 문제였던 파일이 7.2GB 였다). JPEG 로 바꿔서 파일이
// 훨씬 작아진 지금도(비압축 시절보다) 아주 긴 녹화라면 여전히 넘을 수
// 있으니, 안전하게 한 파일이 이 크기를 넘지 않도록 여러 개의
// output_001.avi, output_002.avi ... 로 쪼갠다(실제 한계인 4GB 보다 꽤
// 낮게 잡아 오래된/부실한 플레이어의 호환성 여유도 둔다).
const MAX_AVI_VIDEO_BYTES: u64 = 1_932_735_283; // ~1.8 GiB, 영상 데이터 기준(오디오·헤더는 별도)

const AVIH_LEN: u32 = 56;
const STRH_LEN: u32 = 64;
const STRF_V_LEN: u32 = 40;
const STRF_A_LEN: u32 = 18;
const AVIIF_KEYFRAME: u32 = 0x10;

// write_avi_part() 에 넘기는 오디오 포맷 값 묶음 — clippy::too_many_arguments
// 를 피하려고 묶었을 뿐 의미상 특별한 그룹은 아니다.
struct AviAudioFormat {
    channels: u16,
    sample_rate: u32,
    bits: u16,
    is_float: bool,
    block_align: u32,
    avg_bytes_per_sec: u32,
}

// 녹화가 끝나면(Record 버튼을 다시 꺼서 self.recording 이 false 가 되는 순간)
// 이미 저장해둔 frame_NNNNNN.jpg 들과 audio.wav 를 하나로 묶어 exe 없이도
// 바로 재생할 수 있는 output*.avi 를 만든다. mp4/H.264 처럼 Media
// Foundation 의 인코더 MFT 를 새로 붙이는 대신, 이미 저장해둔 JPEG 프레임을
// 그대로(재인코딩 없이) 담는 옛날식 MJPEG-AVI(Video for Windows) 컨테이너를
// 직접 조립했다 — 포맷이 오래됐지만 문서가 안정적이고 코덱 자체가 워낙
// 흔해서(웹캠 캡처용으로 널리 쓰임) 어떤 플레이어에서도 별도 코덱 설치 없이
// 바로 재생된다.
//
// JPEG 는 프레임마다 압축률이 달라 크기가 들쭉날쭉하다 — 그래서 (wav 저장
// 때처럼 나중에 되돌아가 고쳐 쓰는 대신) 먼저 각 파일 크기를 전부
// 읽어둔(디코드 없이 메타데이터만) 뒤, 그 실제 크기로 헤더를 정확히
// 계산해서 한 번에 순서대로 쓴다.
fn mux_avi(dir: &std::path::Path, fps: u32) {
    let mut frame_paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jpg"))
        .collect();
    frame_paths.sort();
    if frame_paths.is_empty() {
        return;
    }
    let sizes: Vec<u32> = frame_paths.iter().map(|p| std::fs::metadata(p).map(|m| m.len() as u32).unwrap_or(0)).collect();

    let (channels, sample_rate, bits, audio_data) = read_wav(&dir.join("audio.wav")).unwrap_or((2, 48000, 32, Vec::new()));
    let block_align = (channels as u32 * (bits as u32 / 8)).max(1);
    let fmt = AviAudioFormat { channels, sample_rate, bits, is_float: bits == 32, block_align, avg_bytes_per_sec: sample_rate * block_align };

    // 누적 크기가 한계를 넘기 직전까지 프레임을 묶어 파트를 나눈다(크기가
    // 프레임마다 달라서 고정 개수로 딱 나눌 수 없다).
    let mut parts: Vec<(usize, usize)> = Vec::new();
    let (mut start, mut acc) = (0usize, 0u64);
    for (i, &sz) in sizes.iter().enumerate() {
        let chunk_bytes = 8u64 + sz as u64;
        if acc + chunk_bytes > MAX_AVI_VIDEO_BYTES && i > start {
            parts.push((start, i));
            start = i;
            acc = 0;
        }
        acc += chunk_bytes;
    }
    parts.push((start, sizes.len()));
    let num_parts = parts.len();

    for (part_idx, &(s, e)) in parts.iter().enumerate() {
        // 이 파트가 담당하는 시간 구간(초)에 해당하는 오디오 바이트만 잘라
        // 쓴다 — block_align 배수로 정렬해야 샘플 경계가 안 어긋난다.
        let (t0, t1) = (s as f64 / fps.max(1) as f64, e as f64 / fps.max(1) as f64);
        let align = |bytes: f64| -> usize {
            let b = (bytes as usize).min(audio_data.len());
            b - (b % fmt.block_align as usize)
        };
        let (a_start, a_end) = (align(t0 * fmt.avg_bytes_per_sec as f64), align(t1 * fmt.avg_bytes_per_sec as f64));
        let part_audio = &audio_data[a_start..a_end.max(a_start)];

        let name = if num_parts == 1 { "output.avi".to_string() } else { format!("output_{:03}.avi", part_idx + 1) };
        let _ = write_avi_part(&dir.join(name), &frame_paths[s..e], &sizes[s..e], part_audio, fps, &fmt);
    }
}

fn write_avi_part(
    path: &std::path::Path, frame_paths: &[PathBuf], frame_sizes: &[u32], audio_data: &[u8], fps: u32, fmt: &AviAudioFormat,
) -> std::io::Result<()> {
    let frame_count = frame_paths.len() as u32;
    // 압축 코덱이라 프레임 크기가 매번 다르다 — 헤더의 "제안 버퍼 크기" 류
    // 필드에는 그중 가장 큰 값을 힌트로 넣어둔다(정확한 값을 안 지켜도 되는
    // 참고용 필드라 최대치면 충분).
    let max_frame_len = frame_sizes.iter().copied().max().unwrap_or(0);

    let strl_v_len = 4 + (8 + STRH_LEN) + (8 + STRF_V_LEN);
    let strl_a_len = 4 + (8 + STRH_LEN) + (8 + STRF_A_LEN);
    let hdrl_len = 4 + (8 + AVIH_LEN) + (8 + strl_v_len) + (8 + strl_a_len);

    let has_audio = !audio_data.is_empty();
    let audio_pad = audio_data.len() as u32 % 2;
    let video_bytes: u32 = frame_sizes.iter().map(|&s| 8 + s + (s % 2)).sum();
    let movi_len = 4 + video_bytes + if has_audio { 8 + audio_data.len() as u32 + audio_pad } else { 0 };
    let idx_entries = frame_count + if has_audio { 1 } else { 0 };
    let idx_len = idx_entries * 16;
    let riff_len = 4 + (8 + hdrl_len) + (8 + movi_len) + (8 + idx_len);

    {
        let file = std::fs::File::create(path)?;
        let mut w = std::io::BufWriter::new(file);

        w.write_all(b"RIFF")?;
        w.write_all(&riff_len.to_le_bytes())?;
        w.write_all(b"AVI ")?;

        w.write_all(b"LIST")?;
        w.write_all(&hdrl_len.to_le_bytes())?;
        w.write_all(b"hdrl")?;

        w.write_all(b"avih")?;
        w.write_all(&AVIH_LEN.to_le_bytes())?;
        w.write_all(&(1_000_000 / fps.max(1)).to_le_bytes())?; // dwMicroSecPerFrame
        w.write_all(&0u32.to_le_bytes())?; // dwMaxBytesPerSec
        w.write_all(&0u32.to_le_bytes())?; // dwPaddingGranularity
        w.write_all(&0x10u32.to_le_bytes())?; // dwFlags = AVIF_HASINDEX
        w.write_all(&frame_count.to_le_bytes())?; // dwTotalFrames
        w.write_all(&0u32.to_le_bytes())?; // dwInitialFrames
        w.write_all(&2u32.to_le_bytes())?; // dwStreams
        w.write_all(&max_frame_len.to_le_bytes())?; // dwSuggestedBufferSize
        w.write_all(&RECORD_W.to_le_bytes())?; // dwWidth
        w.write_all(&RECORD_H.to_le_bytes())?; // dwHeight
        w.write_all(&[0u8; 16])?; // dwReserved[4]

        // MJPEG(모션 JPEG) fourcc — biCompression 뿐 아니라 strh 의 fccHandler
        // 에도 같은 값을 넣어야 일부 플레이어(fccHandler 를 우선 보는 쪽)에서도
        // 코덱을 제대로 알아본다.
        let mjpg = u32::from_le_bytes(*b"MJPG");

        // 비디오 스트림
        w.write_all(b"LIST")?;
        w.write_all(&strl_v_len.to_le_bytes())?;
        w.write_all(b"strl")?;
        w.write_all(b"strh")?;
        w.write_all(&STRH_LEN.to_le_bytes())?;
        w.write_all(b"vids")?;
        w.write_all(&mjpg.to_le_bytes())?; // fccHandler
        w.write_all(&0u32.to_le_bytes())?; // dwFlags
        w.write_all(&0u16.to_le_bytes())?; // wPriority
        w.write_all(&0u16.to_le_bytes())?; // wLanguage
        w.write_all(&0u32.to_le_bytes())?; // dwInitialFrames
        w.write_all(&1u32.to_le_bytes())?; // dwScale
        w.write_all(&fps.max(1).to_le_bytes())?; // dwRate (dwRate/dwScale = fps)
        w.write_all(&0u32.to_le_bytes())?; // dwStart
        w.write_all(&frame_count.to_le_bytes())?; // dwLength
        w.write_all(&max_frame_len.to_le_bytes())?; // dwSuggestedBufferSize
        w.write_all(&0xFFFF_FFFFu32.to_le_bytes())?; // dwQuality = -1(기본)
        w.write_all(&0u32.to_le_bytes())?; // dwSampleSize=0 → 인덱스가 크기를 따로 알려줌
        w.write_all(&0i32.to_le_bytes())?; // rcFrame.left
        w.write_all(&0i32.to_le_bytes())?; // rcFrame.top
        w.write_all(&(RECORD_W as i32).to_le_bytes())?; // rcFrame.right
        w.write_all(&(RECORD_H as i32).to_le_bytes())?; // rcFrame.bottom

        w.write_all(b"strf")?;
        w.write_all(&STRF_V_LEN.to_le_bytes())?;
        w.write_all(&40u32.to_le_bytes())?; // biSize
        w.write_all(&(RECORD_W as i32).to_le_bytes())?; // biWidth
        w.write_all(&(-(RECORD_H as i32)).to_le_bytes())?; // biHeight<0 → 위에서 아래로 저장(top-down)
        w.write_all(&1u16.to_le_bytes())?; // biPlanes
        w.write_all(&24u16.to_le_bytes())?; // biBitCount — MJPEG 는 실제 비트뎁스가 JPEG 스트림 안에 있어 관례적으로 24
        w.write_all(&mjpg.to_le_bytes())?; // biCompression = MJPG
        w.write_all(&max_frame_len.to_le_bytes())?; // biSizeImage
        w.write_all(&0i32.to_le_bytes())?; // biXPelsPerMeter
        w.write_all(&0i32.to_le_bytes())?; // biYPelsPerMeter
        w.write_all(&0u32.to_le_bytes())?; // biClrUsed
        w.write_all(&0u32.to_le_bytes())?; // biClrImportant

        // 오디오 스트림 — audio.wav 가 없거나 비어있어도 스트림 자체는 선언해두고
        // (dwLength=0) movi/idx1 쪽에서만 실제 데이터 청크를 건너뛴다.
        w.write_all(b"LIST")?;
        w.write_all(&strl_a_len.to_le_bytes())?;
        w.write_all(b"strl")?;
        w.write_all(b"strh")?;
        w.write_all(&STRH_LEN.to_le_bytes())?;
        w.write_all(b"auds")?;
        w.write_all(&0u32.to_le_bytes())?; // fccHandler
        w.write_all(&0u32.to_le_bytes())?; // dwFlags
        w.write_all(&0u16.to_le_bytes())?; // wPriority
        w.write_all(&0u16.to_le_bytes())?; // wLanguage
        w.write_all(&0u32.to_le_bytes())?; // dwInitialFrames
        w.write_all(&fmt.block_align.to_le_bytes())?; // dwScale
        w.write_all(&fmt.avg_bytes_per_sec.to_le_bytes())?; // dwRate (dwRate/dwScale = 샘플레이트)
        w.write_all(&0u32.to_le_bytes())?; // dwStart
        w.write_all(&(audio_data.len() as u32 / fmt.block_align).to_le_bytes())?; // dwLength(샘플 수)
        w.write_all(&fmt.block_align.to_le_bytes())?; // dwSuggestedBufferSize
        w.write_all(&0xFFFF_FFFFu32.to_le_bytes())?; // dwQuality
        w.write_all(&fmt.block_align.to_le_bytes())?; // dwSampleSize
        w.write_all(&[0u8; 16])?; // rcFrame(오디오는 안 씀, 0)

        w.write_all(b"strf")?;
        w.write_all(&STRF_A_LEN.to_le_bytes())?;
        w.write_all(&(if fmt.is_float { 3u16 } else { 1u16 }).to_le_bytes())?; // wFormatTag
        w.write_all(&fmt.channels.to_le_bytes())?;
        w.write_all(&fmt.sample_rate.to_le_bytes())?;
        w.write_all(&fmt.avg_bytes_per_sec.to_le_bytes())?;
        w.write_all(&(fmt.block_align as u16).to_le_bytes())?;
        w.write_all(&fmt.bits.to_le_bytes())?;
        w.write_all(&0u16.to_le_bytes())?; // cbSize

        // movi — 프레임/오디오 실제 데이터. 각 청크의 (movi 데이터 시작 기준)
        // 상대 오프셋을 idx1 에 그대로 옮겨 적어야 해서 여기서 같이 기록해둔다.
        w.write_all(b"LIST")?;
        w.write_all(&movi_len.to_le_bytes())?;
        w.write_all(b"movi")?;

        let mut idx: Vec<([u8; 4], u32, u32, u32)> = Vec::with_capacity(idx_entries as usize);
        let mut rel_offset = 0u32;
        for (path, &size) in frame_paths.iter().zip(frame_sizes) {
            // 이미 저장된 JPEG 바이트를 그대로 옮겨 쓴다 — 다시 디코드/재인코딩할
            // 필요가 없다. 파일을 못 읽는 경우(있을 수 없는 일이지만)엔 프레임
            // 개수/오프셋 계산이 어긋나면 안 되니 크기가 같은 빈 데이터로 채운다.
            let data = std::fs::read(path).unwrap_or_else(|_| vec![0u8; size as usize]);
            w.write_all(b"00dc")?;
            w.write_all(&(data.len() as u32).to_le_bytes())?;
            w.write_all(&data)?;
            if data.len() % 2 == 1 {
                w.write_all(&[0u8])?;
            }
            idx.push((*b"00dc", AVIIF_KEYFRAME, rel_offset, data.len() as u32));
            rel_offset += 8 + data.len() as u32 + (data.len() as u32 % 2);
        }
        if has_audio {
            w.write_all(b"01wb")?;
            w.write_all(&(audio_data.len() as u32).to_le_bytes())?;
            w.write_all(audio_data)?;
            if audio_pad == 1 {
                w.write_all(&[0u8])?;
            }
            idx.push((*b"01wb", 0, rel_offset, audio_data.len() as u32));
        }

        w.write_all(b"idx1")?;
        w.write_all(&idx_len.to_le_bytes())?;
        for (fourcc, flags, offset, size) in &idx {
            w.write_all(fourcc)?;
            w.write_all(&flags.to_le_bytes())?;
            w.write_all(&offset.to_le_bytes())?;
            w.write_all(&size.to_le_bytes())?;
        }
        w.flush()
    }
}

struct DirectorStage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    crt: DirectorCrt,
    assets: Assets,
    scenes: SceneManager,
    input: Input,
    settings: Rc<RefCell<Settings>>,
    crt_res: usize,
    start_time: f64,
    last_time: f64,
    glitch_enabled: bool,
    noise_enabled: bool,
    glitch_intensity: f32, // 0.0~1.0
    noise_intensity: f32,  // 0.0~1.0
    glitch_frequency: f32, // 0.0~1.0 — glitch_gap_range() 참고
    overlay_rng: Rng,
    overlay_glitch_timer: f32,
    overlay_glitch_active: f32,
    // 지금 진행 중인 버스트 한 번의 줄무늬 패턴 씨앗 — 버스트가 시작될 때
    // 딱 한 번만 새로 뽑고, 그 버스트가 끝날 때까지(GLITCH_BURST 초) 그대로
    // 유지한다(같은 seed 로 셰이더에 넘기는 한 매 프레임 같은 줄무늬 패턴이
    // 나온다). 예전엔 매 프레임 다시 뽑아서 0.14초 동안 패턴이 계속
    // 어른거렸는데, 그러면 "화면이 찢어졌다"보다 "지지직거린다"에 더
    // 가까워 보였다 — 한 버스트 동안 고정해두면 훨씬 또렷한 "한 번 쫙
    // 찢어진" 스냅샷처럼 보인다.
    overlay_glitch_seed: f32,
    // 이번 버스트의 밴드 굵기(=줄 수, 적을수록 뭉탱이짐) — seed 와 똑같이
    // 버스트 시작할 때 한 번만 랜덤으로 뽑아서 그 버스트 내내 고정한다.
    // 매번 강도만으로 정해지던 걸 랜덤 범위로 바꿔서, 같은 강도로 계속
    // 터져도 매번 다른 굵기로 찢어지게 했다.
    overlay_glitch_freq: f32,
    // 이번 프레임에 실제로 화면에 반영할 글리치 파라미터 — draw_overlay_effects()
    // 에서 채우고, crt.present() 를 부를 때 넘긴다(실제 화면-내용을 줄무늬로
    // 미는 건 DirectorCrt 의 셰이더 쪽에서 한다).
    pending_glitch: Option<GlitchParams>,
    // panel 의 Record 버튼과 매 프레임 동기화되는 녹화 상태 — true 인 동안
    // draw() 가 매 프레임 crt.present_to_record() 로 한 번 더 그려서 PNG 로
    // record_dir 에 저장한다(apply_director_state 에서 false→true 전환을
    // 감지해 새 타임스탬프 폴더를 만든다).
    recording: bool,
    record_dir: Option<PathBuf>,
    record_frame: u32,
    // 녹화가 시작된 실제 시각(date::now()) — 끝날 때 (record_frame / 경과시간)
    // 으로 실측 fps 를 구하는 데 쓴다(apply_director_state 참고).
    record_start_time: f64,
    audio_capture: Option<AudioCapture>,
    frame_writer: Option<FrameWriter>,
}

impl DirectorStage {
    fn new() -> DirectorStage {
        // director 가 녹화 도중 죽거나 강제 종료되면 director_state.json 에
        // recording:true 가 그대로 남는다 — 그 상태로 다음번에 새로 켜지면
        // apply_director_state() 가 "방금 Record 버튼을 눌렀다"로 착각해서
        // 사용자가 아무것도 안 눌렀는데 매번 녹화가 저절로 다시 시작돼버린다.
        // 시작할 때 무조건 한 번 꺼서 저장해둬 이 되살아남을 막는다.
        let mut boot_state = director_ipc::load();
        if boot_state.recording {
            boot_state.recording = false;
            director_ipc::save(&boot_state);
        }

        spawn_panel();
        window::show_mouse(false);
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let settings = foundation::load_settings().or_else(|| foundation::load().map(|s| s.settings)).unwrap_or_default();
        let res_idx = settings.res_idx.min(RES_OPTS.len() - 1);
        let (_, w, h) = RES_OPTS[res_idx];
        let crt = DirectorCrt::new(ctx.as_mut(), w, h);
        let assets = Assets::load(ctx.as_mut());
        let scenes = SceneManager::new(make_scene("Lobby"));
        let now = date::now();
        let mut overlay_rng = Rng::new((now * 1e6) as u64);
        let (gap_min, gap_max) = glitch_gap_range(0.5);
        let overlay_glitch_timer = overlay_rng.range_f32(gap_min, gap_max);
        DirectorStage {
            ctx,
            renderer,
            crt,
            assets,
            scenes,
            input: Input::default(),
            settings: Rc::new(RefCell::new(settings)),
            crt_res: res_idx,
            start_time: now,
            last_time: now,
            glitch_enabled: false,
            noise_enabled: false,
            glitch_intensity: 1.0,
            noise_intensity: 1.0,
            glitch_frequency: 0.5,
            overlay_rng,
            overlay_glitch_timer,
            overlay_glitch_active: 0.0,
            overlay_glitch_seed: 0.0,
            overlay_glitch_freq: 15.0,
            pending_glitch: None,
            recording: false,
            record_dir: None,
            record_frame: 0,
            record_start_time: 0.0,
            audio_capture: None,
            frame_writer: None,
        }
    }

    fn to_virtual(&self, x: f32, y: f32) -> (f32, f32) {
        let (sw, sh) = window::screen_size();
        let (ox, oy, vw, vh) = viewport_4x3(sw, sh);
        let uv = ((x - ox) / vw, (y - oy) / vh);
        let (wx, wy) = warp(uv.0, uv.1);
        (wx * VW as f32, wy * VH as f32)
    }

    // panel 이 공유 파일에 써둔 상태를 읽어 반영한다 — glitch/noise 는 매 프레임
    // 그대로 옮겨 적고(끄고 켜는 게 항상 즉시 반영), jump_to 는 있으면 딱 한
    // 번만 그 씬으로 전환한 뒤 파일에서 지운다(안 지우면 그 씬으로 계속 다시
    // 전환되느라 다른 씬으로 못 넘어간다).
    fn apply_director_state(&mut self) {
        let state = director_ipc::load();
        self.glitch_enabled = state.glitch;
        self.noise_enabled = state.noise;
        self.glitch_intensity = state.glitch_intensity.clamp(0.0, 1.0);
        self.noise_intensity = state.noise_intensity.clamp(0.0, 1.0);
        self.glitch_frequency = state.glitch_frequency.clamp(0.0, 1.0);
        if state.recording && !self.recording {
            let dir = start_recording();
            self.audio_capture = Some(AudioCapture::start(dir.join("audio.wav")));
            self.frame_writer = Some(FrameWriter::start(dir.clone()));
            self.record_dir = Some(dir);
            self.record_frame = 0;
            self.record_start_time = date::now();
        } else if !state.recording && self.recording {
            if let Some(cap) = self.audio_capture.take() {
                cap.stop_and_join();
            }
            // AVI 에 적을 fps 는 설정의 목표 fps(예: 60) 가 아니라, 실제로 이
            // 녹화 동안 프레임이 저장된 속도로 계산한다 — 목표 fps 를 그대로
            // 쓰면 실제 캡처 속도와 안 맞을 때(느려도, 빨라도) 재생 길이가
            // 실제로 녹화한 시간과 달라진다. 실측 fps 를 써야 프레임 수와
            // 무관하게 항상 실제로 녹화한 시간 그대로 재생된다.
            let elapsed = (date::now() - self.record_start_time).max(0.001);
            let measured_fps = ((self.record_frame as f64 / elapsed).round() as u32).clamp(1, 240);
            // wav 가 다 저장된 뒤에야 이어붙일 수 있으니, 위에서 오디오 스레드를
            // 먼저 join 해 파일을 완전히 닫은 다음 mux 를 시작한다. FrameWriter
            // 가 밀린 프레임을 마저 디스크에 쓰는 것(fw.finish())까지 포함해서
            // 전부 이 스레드 안에서 순서대로 처리한다 — 메인 스레드(draw() 루프)
            // 는 밀린 프레임이 몇 개가 됐든 기다리지 않고 바로 다음 프레임으로
            // 진행한다.
            if let (Some(dir), Some(fw)) = (self.record_dir.clone(), self.frame_writer.take()) {
                // panel 이 "합치는 중" 표시를 띄울 수 있게, 시작/끝을 별도
                // 상태 파일에 남긴다(director_ipc::DirectorStatus 참고).
                director_ipc::save_status(&director_ipc::DirectorStatus { muxing: true });
                std::thread::spawn(move || {
                    fw.finish();
                    mux_avi(&dir, measured_fps);
                    director_ipc::save_status(&director_ipc::DirectorStatus { muxing: false });
                });
            }
        }
        self.recording = state.recording;
        if let Some(name) = state.jump_to.clone() {
            self.scenes.set(make_scene(&name));
            director_ipc::clear_jump(state);
        }
    }

    // 지금 보고 있는 씬이 뭐든 상관없이 화면 위에 정전기 알갱이(noise)를
    // 덧그리고, 글리치(glitch) 파라미터를 계산해서 self.pending_glitch 에
    // 채워둔다 — 실제로 화면을 줄무늬로 쪼개서 옆으로 미는 건 여기가 아니라
    // CRT 셰이더(crt.rs::CRT_FS)가 present_with_glitch() 호출 시점에 한다
    // (진짜 화면-내용이 찢겨서 밀려나 보이려면 이미 그려진 화면 텍셀을 다시
    // 샘플링해야 하는데, 그건 셰이더 단계에서만 가능하다). 노이즈는 화면
    // 내용과 무관한 알갱이라 지금처럼 그냥 위에 덧그리는 것으로 충분하다.
    //
    // intensity(0.0~1.0, panel 에서 슬라이더로 조절)는 STRENGTH_MAX 배까지
    // 늘어나는 배율로 바꿔서 쓴다 — 0% 는 정말 거의 안 보일 만큼 약하게,
    // 100% 는 원래(강도 조절이 생기기 전) 세기의 STRENGTH_MAX 배로 훨씬
    // 세게 나오도록. 타이밍/빈도는 안 건드린다 — 그것까지 강도에 따라
    // 바꾸면 "약하게" 켰을 때 오히려 더 뜸해 보여서 직관과 어긋난다.
    fn draw_overlay_effects(&mut self, dt: f32) {
        const STRENGTH_MAX: f32 = 3.0; // 100% 일 때 예전 세기의 몇 배까지 갈지
        let noise_s = self.noise_intensity * STRENGTH_MAX;
        let glitch_s = self.glitch_intensity * STRENGTH_MAX;

        if self.noise_enabled && noise_s > 0.0 {
            let count = (NOISE_COUNT as f32 * noise_s).round() as usize;
            for _ in 0..count {
                let x = self.overlay_rng.range_f32(0.0, 640.0);
                let y = self.overlay_rng.range_f32(0.0, 480.0);
                let s = self.overlay_rng.range_f32(1.0, 2.0);
                let v = (self.overlay_rng.range_f32(0.05, 0.35) * noise_s).min(1.0);
                self.renderer.rect(x, y, s, s, [v, v, v, 1.0]);
            }
        }

        // frequency 슬라이더가 100% 면 "상시 작동" — 버스트/대기 사이 텀 없이
        // 한 버스트가 끝나자마자 바로 다음 버스트를 시작한다.
        let always_on = self.glitch_frequency >= 0.999;
        if !self.glitch_enabled {
            self.overlay_glitch_active = 0.0;
        } else if self.overlay_glitch_active > 0.0 {
            self.overlay_glitch_active -= dt;
            if self.overlay_glitch_active <= 0.0 {
                if always_on {
                    self.overlay_glitch_active = GLITCH_BURST;
                    self.overlay_glitch_seed = self.overlay_rng.range_f32(0.0, 1000.0);
                    self.overlay_glitch_freq = self.overlay_rng.range_f32(6.0, 8.0 + glitch_s * 14.0);
                } else {
                    let (gap_min, gap_max) = glitch_gap_range(self.glitch_frequency);
                    self.overlay_glitch_timer = self.overlay_rng.range_f32(gap_min, gap_max);
                }
            }
        } else {
            self.overlay_glitch_timer -= dt;
            if always_on || self.overlay_glitch_timer <= 0.0 {
                self.overlay_glitch_active = GLITCH_BURST;
                self.overlay_glitch_seed = self.overlay_rng.range_f32(0.0, 1000.0);
                self.overlay_glitch_freq = self.overlay_rng.range_f32(6.0, 8.0 + glitch_s * 14.0);
            }
        }

        // 화면-내용을 실제로 옆으로 미는 건 DirectorCrt 의 전용 셰이더가 한다
        // (이 파일 위쪽 DIRECTOR_CRT_FS 참고) — 여기서는 이번 프레임에 넘길
        // 파라미터만 계산해둔다.
        if self.overlay_glitch_active > 0.0 && glitch_s > 0.0 {
            let gi = glitch_s;
            self.pending_glitch = Some(GlitchParams {
                seed: self.overlay_glitch_seed,
                // 뭉탱이 크기(밴드 굵기)도 버스트마다 랜덤 — overlay_glitch_freq
                // 는 burst 가 새로 시작될 때 위에서 한 번만 뽑아둔 값이다.
                freq: self.overlay_glitch_freq,
                fill: (0.15 + gi * 0.16).min(0.85),
                shift: (0.02 + gi * 0.05).min(0.22),
                // 화면을 살짝 위아래로도 미는 세로 변위 — 가로 찢김보다는 약하게.
                vshift: (0.01 + gi * 0.025).min(0.09),
                chroma: gi * 0.7,
            });
        } else {
            self.pending_glitch = None;
        }
    }
}

// 녹화 도중 창을 그냥 닫아버려도(Record 버튼으로 정식으로 끄지 않아도) wav
// 헤더가 0바이트짜리로 남지 않도록, 스테이지가 사라질 때 캡처 스레드를 마저
// 정리한다.
impl Drop for DirectorStage {
    fn drop(&mut self) {
        if let Some(cap) = self.audio_capture.take() {
            cap.stop_and_join();
        }
        if let Some(fw) = self.frame_writer.take() {
            fw.finish();
        }
    }
}

impl EventHandler for DirectorStage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;
        let elapsed = (now - self.start_time) as f32;

        let res_idx = self.settings.borrow().res_idx.min(RES_OPTS.len() - 1);
        if res_idx != self.crt_res {
            let (_, w, h) = RES_OPTS[res_idx];
            self.crt.set_resolution(self.ctx.as_mut(), w, h);
            self.crt_res = res_idx;
        }

        self.apply_director_state();

        self.renderer.begin(VW as f32, VH as f32);
        let (quit, cursor, show_cursor) = {
            let mut frame = Frame {
                ctx: self.ctx.as_mut(),
                r: &mut self.renderer,
                assets: &self.assets,
                input: &self.input,
                settings: self.settings.clone(),
                dt,
                time: elapsed,
                cursor: ui::CursorKind::Arrow,
                show_cursor: true,
            };
            let quit = self.scenes.update(&mut frame);
            (quit, frame.cursor, frame.show_cursor)
        };

        // 씬을 다 그린 뒤 그 위에 덧그려서, 씬 종류와 무관하게 항상 같은
        // 자리(맨 위)에 나온다.
        self.draw_overlay_effects(dt);

        if show_cursor {
            let cursor_scale = self.settings.borrow().cursor_scale;
            ui::draw_cursor(&mut self.renderer, self.assets.cursor, cursor, self.input.mouse.0, self.input.mouse.1, cursor_scale);
        }

        self.crt.begin(self.ctx.as_mut());
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();

        let (ca_amount, crt_intensity) = {
            let s = self.settings.borrow();
            (s.chromatic_aberration, s.crt_intensity)
        };
        let present_params = PresentParams { time: elapsed, ca_amount, intensity: crt_intensity, glitch: self.pending_glitch };
        self.crt.present(self.ctx.as_mut(), present_params);

        if self.recording && let Some(fw) = &self.frame_writer {
            self.crt.present_to_record(self.ctx.as_mut(), present_params);
            let pixels = self.crt.capture_record_pixels(self.ctx.as_mut());
            fw.send(self.record_frame, pixels);
            self.record_frame += 1;
        }

        self.ctx.commit_frame();

        self.input.end_frame();

        if quit {
            // 씬 자체가 "종료"를 요청해도(예: Shutdown 연출 끝) director 는 그냥
            // 계속 켜둔다 — 실제 프로세스를 끝내면 다음 장면을 이어서 못 본다.
            // 대신 Lobby 로 되돌려서 다시 조작할 수 있게 한다.
            self.scenes.set(make_scene("Lobby"));
        }

        let fps_idx = self.settings.borrow().fps_idx.min(FPS_OPTS.len().saturating_sub(1));
        let target_fps = FPS_OPTS.get(fps_idx).map_or(60, |(_, f)| *f);
        if target_fps > 0 {
            let target_dt = 1.0 / target_fps as f64;
            let frame_elapsed = date::now() - now;
            if frame_elapsed < target_dt {
                std::thread::sleep(std::time::Duration::from_secs_f64(target_dt - frame_elapsed));
            }
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.input.mouse = self.to_virtual(x, y);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.input.mouse = self.to_virtual(x, y);
            self.input.mouse_down = true;
            self.input.mouse_clicked = true;
        } else if button == MouseButton::Right {
            self.input.mouse = self.to_virtual(x, y);
            self.input.right_clicked = true;
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            self.input.mouse_down = false;
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.input.wheel += y;
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.input.on_key_up(keycode);
    }

    fn char_event(&mut self, character: char, _mods: KeyMods, _repeat: bool) {
        self.input.on_char(character);
    }
}

fn main() {
    // 기본 800x600 보다 조금 더 크게 — 4:3 비율(필러박스 계산과 맞춤)은 유지.
    let conf = conf::Conf {
        window_title: "PalaceOS Director".to_owned(),
        window_width: 960,
        window_height: 720,
        fullscreen: false,
        high_dpi: true,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(DirectorStage::new()));
}
