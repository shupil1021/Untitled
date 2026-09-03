// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs 와 같은 이유.
#![windows_subsystem = "windows"]

// PalaceOS Director Panel — director.exe 옆에 따로 뜨는 작은 조작 창.
//
// 게임 화면(director.exe)과는 완전히 다른 창(다른 프로세스)이다 — 그래야
// director.exe 쪽 화면(=실제로 녹화할 화면)엔 이 조작 UI 가 전혀 안 찍힌다.
// CRT 곡면 효과도 안 쓰는 평범한 창이라 버튼을 누르기도 더 편하다.
//
// 여기서 하는 일은 전부 "director_state.json 에 원하는 상태를 써두는 것"뿐이다
// — 실제로 씬을 바꾸거나 글리치/정전기를 켜고 끄는 건 director.exe 가 매 프레임
// 그 파일을 다시 읽어서 한다(director_ipc 모듈 참고). 이 창을 닫아도 director.exe
// 는 마지막으로 저장된 상태 그대로 계속 돌아간다.

use miniquad::*;

use crackhead::director_ipc::{self, DirectorState};
use crackhead::gfx::Renderer;
use crackhead::scenes::Input;
use crackhead::ui;

const SCENE_BUTTONS: [&str; 6] = ["Boot", "Lobby", "Desktop", "Erase", "Shutdown", "BlueScreen"];
const WIN_W: f32 = 200.0;
// 버튼/줄이 다 들어가고도 아래쪽에 여유가 남게 넉넉히 잡는다 — 예전에 딱
// 맞춰서 240 남짓으로 뒀을 때, high_dpi 창에서 실제 클릭 가능 영역이
// 렌더러가 그리는 캔버스보다 살짝 작게 잡히는 경우가 있어서(특히 아래쪽
// 요소일수록 영향이 컸다) 아래쪽 버튼이 안 눌리는 것처럼 보인 적이 있다.
const WIN_H: f32 = 530.0;

struct PanelStage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    input: Input,
    state: DirectorState,
}

impl PanelStage {
    fn new() -> PanelStage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        PanelStage { ctx, renderer, input: Input::default(), state: director_ipc::load() }
    }
}

// "라벨 NN%" 글자 한 줄 + 그 아래 드래그로 끄는 슬라이더 막대. 트랙 위 아무
// 데나 누른 채로 좌우로 끌면 그 x 위치 비율로 값이 바로 따라온다(클릭
// 한 번만으로도 그 지점 값으로 즉시 이동 — 꼭 손잡이를 정확히 잡을 필요
// 없다). 값이 바뀌었으면 true. 자유 함수로 둔 이유: 뒤에서 self.renderer
// 를 또 mutably 빌리는 호출들과 섞여야 하는데, 여기서 self 를 통째로
// 빌리면 그 사이에 borrow 충돌이 난다 — 그래서 필요한 값만 값으로 복사해
// 받는다(scenes::Input 은 안 쓰고 순수 함수로 처리한다).
fn draw_slider_row(r: &mut Renderer, sw: f32, y: f32, mouse: (f32, f32), mouse_down: bool, label: &str, value: &mut f32) -> bool {
    const TRACK_H: f32 = 12.0;
    const KNOB_W: f32 = 6.0;
    let track_x = 8.0;
    let track_w = sw - 16.0;
    let track_y = y + 14.0;

    let line = format!("{} {}%", label, (*value * 100.0).round() as i32);
    r.text(track_x, y, &line, 0.75, [0.15, 0.15, 0.15, 1.0]);

    r.rect(track_x, track_y, track_w, TRACK_H, [0.55, 0.55, 0.58, 1.0]);
    ui::border(r, track_x, track_y, track_w, TRACK_H, [0.25, 0.25, 0.25, 1.0]);
    let v = value.clamp(0.0, 1.0);
    let fill_w = (track_w * v).max(2.0);
    r.rect(track_x, track_y, fill_w, TRACK_H, [0.2, 0.45, 0.85, 1.0]);
    let knob_x = (track_x + fill_w - KNOB_W / 2.0).clamp(track_x, track_x + track_w - KNOB_W);
    r.rect(knob_x, track_y - 2.0, KNOB_W, TRACK_H + 4.0, [0.05, 0.05, 0.05, 1.0]);

    const HIT_PAD: f32 = 8.0; // 트랙 위아래로 이만큼은 벗어나도 여전히 드래그로 친다
    if mouse_down && mouse.1 >= track_y - HIT_PAD && mouse.1 <= track_y + TRACK_H + HIT_PAD {
        let frac = ((mouse.0 - track_x) / track_w).clamp(0.0, 1.0);
        if (frac - v).abs() > 0.001 {
            *value = frac;
            return true;
        }
    }
    false
}

impl EventHandler for PanelStage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let (sw, sh) = window::screen_size();
        self.renderer.begin(sw, sh);
        self.renderer.rect(0.0, 0.0, sw, sh, [0.75, 0.75, 0.78, 1.0]);
        self.renderer.text(6.0, 4.0, "PalaceOS Director", 0.85, [0.0, 0.0, 0.0, 1.0]);
        self.renderer.text(6.0, 20.0, "scene:", 0.8, [0.25, 0.25, 0.25, 1.0]);

        let win = crackhead::apps::WinInput {
            mouse: self.input.mouse,
            mouse_down: self.input.mouse_down,
            mouse_clicked: self.input.mouse_clicked,
            focused: true,
            wheel: self.input.wheel,
            dt: 0.0,
            time: 0.0,
            input: &self.input,
        };
        let mut dirty = false;
        let mut by = 38.0;
        for name in SCENE_BUTTONS {
            if ui::button(&mut self.renderer, 8.0, by, sw - 16.0, 24.0, name, &win) {
                self.state.jump_to = Some(name.to_string());
                dirty = true;
            }
            by += 28.0;
        }

        by += 6.0;
        let glitch_label = if self.state.glitch { "Glitch: ON" } else { "Glitch: OFF" };
        if ui::button(&mut self.renderer, 8.0, by, sw - 16.0, 24.0, glitch_label, &win) {
            self.state.glitch = !self.state.glitch;
            dirty = true;
        }
        by += 30.0;
        if draw_slider_row(&mut self.renderer, sw, by, self.input.mouse, self.input.mouse_down, "Intensity", &mut self.state.glitch_intensity) {
            dirty = true;
        }
        by += 34.0;
        if draw_slider_row(&mut self.renderer, sw, by, self.input.mouse, self.input.mouse_down, "Frequency", &mut self.state.glitch_frequency) {
            dirty = true;
        }

        by += 40.0;
        let noise_label = if self.state.noise { "Noise: ON" } else { "Noise: OFF" };
        if ui::button(&mut self.renderer, 8.0, by, sw - 16.0, 24.0, noise_label, &win) {
            self.state.noise = !self.state.noise;
            dirty = true;
        }
        by += 30.0;
        if draw_slider_row(&mut self.renderer, sw, by, self.input.mouse, self.input.mouse_down, "Intensity", &mut self.state.noise_intensity) {
            dirty = true;
        }

        by += 40.0;
        // ■/● 같은 기호는 게임 폰트 아틀라스(ASCII+한글+가나+한자만 담음)에
        // 없어서 tofu(마름모+물음표)로 깨져 보인다 — 이 패널도 같은 렌더러를
        // 쓰므로 ASCII 로만 된 라벨을 쓴다.
        let record_label = if self.state.recording { "[STOP] Stop Recording" } else { "[REC] Record" };
        if ui::button(&mut self.renderer, 8.0, by, sw - 16.0, 24.0, record_label, &win) {
            self.state.recording = !self.state.recording;
            dirty = true;
        }

        // Record 를 끄면 director 가 그 즉시 화면에 반영하지만, PNG+wav 를
        // output.avi 로 합치는 작업(mux_avi)은 별도 스레드에서 이어서 좀 더
        // 걸린다 — director_status.json 을 매 프레임 읽어서, 그동안 창 맨
        // 아래에 "합치는 중" 표시를 띄워 아직 안 끝났다는 걸 알려준다.
        if director_ipc::load_status().muxing {
            let dots = ".".repeat(1 + (date::now() * 2.0) as usize % 3);
            let bar_y = sh - 22.0;
            self.renderer.rect(0.0, bar_y, sw, 22.0, [0.15, 0.35, 0.65, 1.0]);
            self.renderer.text(8.0, bar_y + 5.0, &format!("Encoding video{dots}"), 0.75, [1.0, 1.0, 1.0, 1.0]);
        }

        if dirty {
            director_ipc::save(&self.state);
            // jump_to 는 "한 번만 실행할 명령"이라 여기서 바로 지운다 — 안 지우면
            // 다음에 Glitch/Noise 버튼만 눌러도(직접 관련 없는 저장인데) 이
            // state 를 통째로 다시 저장하면서 예전 씬 전환 요청까지 같이 따라가
            // director.exe 가 매번 그 씬으로 또 튕기는 문제가 있었다 — "글리치를
            // 껐다 켜면 그 대신 전에 눌렀던 씬 버튼이 또 실행된다"는 제보가 이거였다.
            self.state.jump_to = None;
        }

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.input.mouse = (x, y);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.input.mouse = (x, y);
            self.input.mouse_down = true;
            self.input.mouse_clicked = true;
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            self.input.mouse_down = false;
        }
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "PalaceOS Director Panel".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        // high_dpi 를 끈다 — 이 창은 CRT 가상 해상도 변환 없이 마우스 좌표를
        // 그대로 렌더러 좌표로 쓰는 단순한 창이라, high_dpi 켜짐으로 인한
        // 논리/물리 픽셀 불일치 여지를 아예 없애는 쪽이 더 안전하다.
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(PanelStage::new()));
}
