// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! PICOCAD 스타일의 아주 작은 맵 에디터 — `mesh3d.rs`(진짜 3D 메쉬 렌더러) 위에
//! 상자(`Box3D`)를 놓고 옮기고 회전/크기조절해서 `mapfile::MapScene`(JSON)으로
//! 저장한다. 이 게임 전용 최소 기능만 넣었다: 여러 오브젝트 조합/정점 단위 편집
//! 같은 진짜 PICOCAD 급 범용 기능은 없다. 텍스처 지정(`BoxTexture`)은 데이터
//! 구조/저장 포맷까지는 이미 지원하지만, 이 에디터에는 아직 텍스처 고르는 UI가
//! 없다(다음 단계) — 지금은 색상만 편집할 수 있다.
//!
//! `cargo run --bin map_editor` 로 띄운다. 저장/불러오기 경로는 고정으로
//! `maps/scene.json`(실행 파일 기준 상대 경로)을 쓴다.
//!
//! 조작:
//! - 마우스 왼쪽 드래그: 궤도 카메라 회전, 휠: 확대/축소
//! - Tab / Shift+Tab: 다음/이전 상자 선택, N: 새 상자, Delete: 선택 삭제
//! - W/A/S/D: 선택한 상자를 세계 X/Z 로 이동, Q/E: Y(위/아래) 이동
//! - 방향키 ←/→: yaw 회전, ↑/↓: pitch 회전, ,/. : roll 회전
//! - U/J, I/K, O/L: 각각 가로/높이/세로 반너비 확대/축소
//! - C: 색상 팔레트 순환, F: walkable 토글, G: solid 토글
//! - P: 플레이어 시작 위치/방향을 지금 선택한 상자 중심으로 설정
//! - Ctrl+S: 저장, Ctrl+O: 불러오기, Esc: 종료

use miniquad::*;

use crackhead::gfx::Renderer;
use crackhead::mapfile::{MapBoxData, MapScene};
use crackhead::mesh3d::{Box3D, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 600.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const SAVE_PATH: &str = "maps/scene.json";

const SKY_COLOR: [f32; 4] = [0.12, 0.13, 0.17, 1.0];
const PALETTE: [[f32; 4]; 6] = [
    [0.7, 0.7, 0.75, 1.0],
    [0.5, 0.42, 0.3, 1.0],
    [0.32, 0.45, 0.3, 1.0],
    [0.55, 0.3, 0.3, 1.0],
    [0.35, 0.35, 0.6, 1.0],
    [0.8, 0.75, 0.35, 1.0],
];

const MOVE_SPEED: f32 = 2.5;
const ROT_SPEED: f32 = 1.4;
const SCALE_SPEED: f32 = 1.0;
const ORBIT_SPEED: f32 = 0.008;
const ZOOM_SPEED: f32 = 0.6;
const MIN_HALF: f32 = 0.05;

fn default_box(center: [f32; 3]) -> Box3D {
    Box3D { center, half: [0.5, 0.5, 0.5], yaw: 0.0, pitch: 0.0, roll: 0.0, color: PALETTE[0], texture: None, walkable: true, solid: true }
}

struct Orbit {
    target: [f32; 3],
    distance: f32,
    yaw: f32,
    pitch: f32,
}

impl Orbit {
    fn camera(&self) -> Camera {
        let (cy, sy) = (self.pitch.cos(), self.pitch.sin());
        let (cx, sx) = (self.yaw.cos(), self.yaw.sin());
        let forward = [cy * cx, sy, cy * sx];
        let pos = crackhead::mesh3d::v_sub(self.target, crackhead::mesh3d::v_scale(forward, self.distance));
        Camera { pos, yaw: self.yaw + std::f32::consts::PI, pitch: -self.pitch }
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    mesh3d: Mesh3D,
    boxes: Vec<Box3D>,
    selected: usize,
    player_start: [f32; 3],
    player_start_yaw: f32,
    orbit: Orbit,
    dragging: bool,
    last_mouse: (f32, f32),
    input: Input,
    ctrl_down: bool,
    status: String,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mesh3d = Mesh3D::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        let boxes = vec![default_box([0.0, 0.0, 0.0])];
        Stage {
            ctx,
            renderer,
            mesh3d,
            boxes,
            selected: 0,
            player_start: [0.0, 1.0, -3.0],
            player_start_yaw: std::f32::consts::FRAC_PI_2,
            orbit: Orbit { target: [0.0, 0.5, 0.0], distance: 10.0, yaw: -std::f32::consts::FRAC_PI_2, pitch: 0.5 },
            dragging: false,
            last_mouse: (0.0, 0.0),
            input: Input::default(),
            ctrl_down: false,
            status: "새 장면".to_string(),
            last_time: date::now(),
        }
    }

    fn selected_box(&mut self) -> Option<&mut Box3D> {
        self.boxes.get_mut(self.selected)
    }

    fn save(&mut self) {
        let scene = MapScene {
            boxes: self.boxes.iter().map(|b| MapBoxData::from_box3d(b, None)).collect(),
            player_start: self.player_start,
            player_start_yaw: self.player_start_yaw,
        };
        let _ = std::fs::create_dir_all("maps");
        match scene.save(SAVE_PATH) {
            Ok(()) => self.status = format!("저장함: {SAVE_PATH} ({}개)", self.boxes.len()),
            Err(e) => self.status = format!("저장 실패: {e}"),
        }
    }

    fn load(&mut self) {
        match MapScene::load(SAVE_PATH) {
            Ok(scene) => {
                let mut resolve = |_p: &str| -> TextureId { self.mesh3d.color_texture() }; // 텍스처 UI 는 아직 없음 — 자리표시자
                self.boxes = scene.boxes.iter().map(|b| b.to_box3d(&mut resolve)).collect();
                if self.boxes.is_empty() {
                    self.boxes.push(default_box([0.0, 0.0, 0.0]));
                }
                self.player_start = scene.player_start;
                self.player_start_yaw = scene.player_start_yaw;
                self.selected = 0;
                self.status = format!("불러옴: {SAVE_PATH} ({}개)", self.boxes.len());
            }
            Err(e) => self.status = format!("불러오기 실패: {e}"),
        }
    }

    fn handle_input(&mut self, dt: f32) {
        if self.input.pressed(KeyCode::Tab) && !self.boxes.is_empty() {
            let n = self.boxes.len();
            self.selected = if self.input.is_down(KeyCode::LeftShift) || self.input.is_down(KeyCode::RightShift) {
                (self.selected + n - 1) % n
            } else {
                (self.selected + 1) % n
            };
        }
        if self.input.pressed(KeyCode::N) {
            let near = self.orbit.target;
            self.boxes.push(default_box(near));
            self.selected = self.boxes.len() - 1;
        }
        if self.input.pressed(KeyCode::Delete) && !self.boxes.is_empty() {
            self.boxes.remove(self.selected);
            if self.selected >= self.boxes.len() && self.selected > 0 {
                self.selected -= 1;
            }
            if self.boxes.is_empty() {
                self.boxes.push(default_box([0.0, 0.0, 0.0]));
                self.selected = 0;
            }
        }
        if self.input.pressed(KeyCode::P) && let Some(b) = self.boxes.get(self.selected) {
            self.player_start = [b.center[0], b.center[1] + b.half[1] + 0.1, b.center[2]];
            self.player_start_yaw = self.orbit.yaw;
            self.status = "플레이어 시작 위치 설정함".to_string();
        }
        if self.ctrl_down && self.input.pressed(KeyCode::S) {
            self.save();
        }
        if self.ctrl_down && self.input.pressed(KeyCode::O) {
            self.load();
        }

        let move_step = MOVE_SPEED * dt;
        let rot_step = ROT_SPEED * dt;
        let scale_step = SCALE_SPEED * dt;
        let color_cycle = self.input.pressed(KeyCode::C);
        let toggle_walk = self.input.pressed(KeyCode::F);
        let toggle_solid = self.input.pressed(KeyCode::G);
        let (w, a, s, d, q, e) = (
            self.input.is_down(KeyCode::W),
            self.input.is_down(KeyCode::A),
            self.input.is_down(KeyCode::S),
            self.input.is_down(KeyCode::D),
            self.input.is_down(KeyCode::Q),
            self.input.is_down(KeyCode::E),
        );
        let (left, right, up, down) = (
            self.input.is_down(KeyCode::Left),
            self.input.is_down(KeyCode::Right),
            self.input.is_down(KeyCode::Up),
            self.input.is_down(KeyCode::Down),
        );
        let (comma, period) = (self.input.is_down(KeyCode::Comma), self.input.is_down(KeyCode::Period));
        let (u, j, i, k, o, l) = (
            self.input.is_down(KeyCode::U),
            self.input.is_down(KeyCode::J),
            self.input.is_down(KeyCode::I),
            self.input.is_down(KeyCode::K),
            self.input.is_down(KeyCode::O),
            self.input.is_down(KeyCode::L),
        );

        if let Some(b) = self.selected_box() {
            if w {
                b.center[2] -= move_step;
            }
            if s {
                b.center[2] += move_step;
            }
            if a {
                b.center[0] -= move_step;
            }
            if d {
                b.center[0] += move_step;
            }
            if q {
                b.center[1] -= move_step;
            }
            if e {
                b.center[1] += move_step;
            }
            if left {
                b.yaw -= rot_step;
            }
            if right {
                b.yaw += rot_step;
            }
            if up {
                b.pitch += rot_step;
            }
            if down {
                b.pitch -= rot_step;
            }
            if comma {
                b.roll -= rot_step;
            }
            if period {
                b.roll += rot_step;
            }
            if u {
                b.half[0] = (b.half[0] + scale_step).max(MIN_HALF);
            }
            if j {
                b.half[0] = (b.half[0] - scale_step).max(MIN_HALF);
            }
            if i {
                b.half[1] = (b.half[1] + scale_step).max(MIN_HALF);
            }
            if k {
                b.half[1] = (b.half[1] - scale_step).max(MIN_HALF);
            }
            if o {
                b.half[2] = (b.half[2] + scale_step).max(MIN_HALF);
            }
            if l {
                b.half[2] = (b.half[2] - scale_step).max(MIN_HALF);
            }
            if color_cycle {
                let cur = PALETTE.iter().position(|c| *c == b.color).unwrap_or(0);
                b.color = PALETTE[(cur + 1) % PALETTE.len()];
            }
            if toggle_walk {
                b.walkable = !b.walkable;
            }
            if toggle_solid {
                b.solid = !b.solid;
            }
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        self.handle_input(dt);

        self.mesh3d.render(self.ctx.as_mut(), SKY_COLOR, &self.orbit.camera(), &self.boxes, FOV_Y);

        self.renderer.begin(WIN_W, WIN_H);
        let tex = self.mesh3d.color_texture();
        self.renderer.sprite_uv(tex, 0.0, 0.0, WIN_W, WIN_H, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        self.renderer.rect(0.0, 0.0, WIN_W, 76.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(6.0, 4.0, "Map Editor - drag: orbit, wheel: zoom, Tab: select, N: new, Del: delete", 0.68, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(6.0, 20.0, "WASD/QE move, arrows+,./ rotate, U/J I/K O/L scale, C color, F walk, G solid", 0.68, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(6.0, 36.0, "Ctrl+S save, Ctrl+O load, P = set player start here, Esc quit", 0.68, [1.0, 1.0, 1.0, 1.0]);
        if let Some(b) = self.boxes.get(self.selected) {
            let info = format!(
                "sel {}/{}  pos=({:.2},{:.2},{:.2}) half=({:.2},{:.2},{:.2}) rot=({:.2},{:.2},{:.2}) walk={} solid={}",
                self.selected + 1,
                self.boxes.len(),
                b.center[0], b.center[1], b.center[2],
                b.half[0], b.half[1], b.half[2],
                b.yaw, b.pitch, b.roll,
                b.walkable, b.solid,
            );
            self.renderer.text(6.0, 54.0, &info, 0.65, [1.0, 1.0, 0.6, 1.0]);
        }
        self.renderer.rect(0.0, WIN_H - 18.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(6.0, WIN_H - 15.0, &self.status, 0.68, [0.7, 1.0, 0.7, 1.0]);

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, mods: KeyMods, repeat: bool) {
        if keycode == KeyCode::Escape {
            window::order_quit();
        }
        self.ctrl_down = mods.ctrl;
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, mods: KeyMods) {
        self.ctrl_down = mods.ctrl;
        self.input.on_key_up(keycode);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        if self.dragging {
            let dx = x - self.last_mouse.0;
            let dy = y - self.last_mouse.1;
            self.orbit.yaw += dx * ORBIT_SPEED;
            self.orbit.pitch = (self.orbit.pitch + dy * ORBIT_SPEED).clamp(-1.5, 1.5);
        }
        self.last_mouse = (x, y);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.dragging = true;
            self.last_mouse = (x, y);
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Left {
            self.dragging = false;
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.orbit.distance = (self.orbit.distance - y * ZOOM_SPEED).clamp(1.5, 60.0);
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "PalaceOS Map Editor".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
