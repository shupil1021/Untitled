// PalaceOS — Windows 9x 풍 가짜 데스크톱 OS (miniquad).
//
// 실제 로직은 src/lib.rs 가 선언하는 라이브러리 크레이트(crackhead) 쪽에 다
// 있다 — src/bin/director.rs(연출/영상 제작용 컨트롤러)도 같은 라이브러리를
// 가져다 쓴다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::*;

use crackhead::crt::{viewport_4x3, warp, Crt};
use crackhead::foundation;
use crackhead::foundation::{Settings, FPS_OPTS, RES_OPTS};
use crackhead::gfx::{Assets, Renderer, VIRTUAL_H as VH, VIRTUAL_W as VW};
use crackhead::scenes::{lobby::LobbyScene, Frame, Input, SceneManager};
use crackhead::ui::draw_cursor;

// 가상 해상도. 씬/UI 는 항상 이 좌표계(640x480)에서 그린다.
// (오프스크린 렌더 해상도는 설정에 따라 달라지지만, 레이아웃 좌표는 항상 640x480)

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    crt: Crt,
    assets: Assets,
    scenes: SceneManager,
    input: Input,
    settings: Rc<RefCell<Settings>>,
    crt_res: usize,
    start_time: f64,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        // 일부 PC(그래픽 드라이버 등)에서 OS 커서가 안 보이는 문제가 있어, OS 커서는 아예
        // 숨기고 항상 우리가 직접 그려서(draw_cursor) 확실히 보이게 한다.
        window::show_mouse(false);
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        // 취향 설정(언어 등)은 palaceos_settings.json 에 즉시 저장되는 전용 파일이라
        // 그쪽을 최우선으로 쓰고, 그게 없으면(예: 이 기능 이전에 만들어진 예전
        // 저장 파일만 있는 경우) 게임 진행 저장 안에 같이 들어있던 settings 로,
        // 그것도 없으면 기본값으로 시작한다.
        let mut settings =
            foundation::load_settings().or_else(|| foundation::load().map(|s| s.settings)).unwrap_or_default();
        // 해상도/프레임레이트 옵션 목록 개수가 나중에 바뀔 수 있으므로, 예전에
        // 저장된 인덱스가 지금은 범위를 벗어날 수 있다 — 그대로 쓰면 배열 밖
        // 인덱싱으로 패닉난다.
        if settings.res_idx >= RES_OPTS.len() {
            settings.res_idx = Settings::new().res_idx;
        }
        if settings.fps_idx >= FPS_OPTS.len() {
            settings.fps_idx = Settings::new().fps_idx;
        }
        if settings.bg_color_idx >= foundation::BG_COLORS.len() {
            settings.bg_color_idx = Settings::new().bg_color_idx;
        }
        let res_idx = settings.res_idx;
        let (_, w, h) = RES_OPTS[res_idx];
        let crt = Crt::new(ctx.as_mut(), w, h);
        let assets = Assets::load(ctx.as_mut());
        let scenes = SceneManager::new(Box::new(LobbyScene::new()));
        let now = date::now();
        Stage {
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
        }
    }

    // 실제 창 좌표 → 가상 해상도(640x480) 좌표.
    // CRT 셰이더가 배럴 왜곡을 적용하므로, 커서 아래 "보이는" 위치로 매핑하려면
    // 여기서도 똑같은 왜곡을 적용해야 한다 (안 그러면 가장자리에서 클릭 위치가 어긋난다).
    fn to_virtual(&self, x: f32, y: f32) -> (f32, f32) {
        let (sw, sh) = window::screen_size();
        let (ox, oy, vw, vh) = viewport_4x3(sw, sh);
        let uv = ((x - ox) / vw, (y - oy) / vh);
        let (wx, wy) = warp(uv.0, uv.1);
        (wx * VW as f32, wy * VH as f32)
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        // 프레임이 한 번 크게 끊기면(창 포커스 복귀, 첫 프레임, OS 가 잠깐 시간을
        // 안 준 경우 등) dt 가 순간적으로 아주 커질 수 있어서 500ms 로 상한을
        // 둔다 — 그래도 Backspace 꾹 누르기 반복 삭제처럼 "dt 누적치"를 그대로
        // 반복 횟수로 바꾸는 로직은 이 상한만으론 부족해서(500ms 도 반복
        // 간격보다 훨씬 크다) mail.rs 쪽에 따로 더 촘촘한 상한을 뒀다
        // (BACKSPACE_MAX_DT_PER_FRAME 참고).
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;
        let elapsed = (now - self.start_time) as f32;

        // 설정 해상도가 바뀌면 오프스크린 렌더 타깃을 다시 만든다.
        let res_idx = self.settings.borrow().res_idx;
        if res_idx != self.crt_res {
            let (_, w, h) = RES_OPTS[res_idx];
            self.crt.set_resolution(self.ctx.as_mut(), w, h);
            self.crt_res = res_idx;
        }

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
                cursor: crackhead::ui::CursorKind::Arrow,
                show_cursor: true,
            };
            let quit = self.scenes.update(&mut frame);
            (quit, frame.cursor, frame.show_cursor)
        };

        // 씬 위에 커서를 맨 마지막으로 그려서 항상 모든 UI 위에 보이게 한다.
        // (부팅 화면처럼 마우스가 필요 없는 씬은 show_cursor 를 꺼서 아예 안 그린다.)
        if show_cursor {
            let cursor_scale = self.settings.borrow().cursor_scale;
            draw_cursor(&mut self.renderer, self.assets.cursor, cursor, self.input.mouse.0, self.input.mouse.1, cursor_scale);
        }

        self.crt.begin(self.ctx.as_mut());
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();

        let (ca_amount, crt_intensity) = {
            let s = self.settings.borrow();
            (s.chromatic_aberration, s.crt_intensity)
        };
        self.crt.present(self.ctx.as_mut(), elapsed, ca_amount, crt_intensity);
        self.ctx.commit_frame();

        self.input.end_frame();

        // Settings 의 Frame rate 값을 실제로 적용 — miniquad 가 vsync 를 직접 노출하지
        // 않아서, 프레임에 남는 시간만큼 수동으로 재워서 목표 fps 를 맞춘다. 이게 없으면
        // 화면이 낼 수 있는 만큼 무제한으로 계속 그려서 GPU 점유율이 불필요하게 높아진다.
        let fps_idx = self.settings.borrow().fps_idx;
        let target_fps = FPS_OPTS.get(fps_idx).map_or(60, |(_, f)| *f);
        if target_fps > 0 {
            // 0 = "Unlimited" — 제한 없이 그대로 둔다.
            let target_dt = 1.0 / target_fps as f64;
            let frame_elapsed = date::now() - now;
            if frame_elapsed < target_dt {
                std::thread::sleep(std::time::Duration::from_secs_f64(target_dt - frame_elapsed));
            }
        }

        if quit {
            // 열려있는 웹뷰가 있으면 여기서 멈추라는 신호를 보내고 실제로 다 끝날
            // 때까지 기다린 뒤 지운다 — Official Site 를 켜둔 채로 꺼도 확실히 지워진다.
            crackhead::webview::shutdown_all_and_cleanup();
            window::order_quit();
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
    let conf = conf::Conf {
        window_title: "PalaceOS".to_owned(),
        fullscreen: true,
        high_dpi: true,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
