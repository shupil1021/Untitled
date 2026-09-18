// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::raycaster`(팩맨을 만들면서 뽑아낸, 앞으로 다른 1인칭 3D 미니게임에도
//! 쓸 재사용 엔진)만 따로 띄워서 확인하는 가장 작은 테스트 창이다.
//!
//! 실제 게임(crackhead.exe)이나 팩맨의 라운드/코인 규칙과는 전혀 무관하다 —
//! 미로 하나를 생성하고, 바닥 칸마다 작은 회색 구슬(빌보드 렌더링 확인용)을
//! 하나씩 놓은 뒤, WASD 로 걸어 다니며 벽 충돌/레이캐스팅/빌보드가 제대로
//! 동작하는지만 본다. `cargo run --bin raycaster_test` 로 띄운다.
//!
//! R 키를 누르면 새 미로로 다시 만든다(매번 다른 시드) — 여러 판을 빠르게
//! 훑어보면서 미로 생성 결과가 괜찮은지 확인할 때 쓴다.

use miniquad::*;

use crackhead::gfx::{Rect, Renderer};
use crackhead::raycaster::{generate_maze, Billboard, Raycaster, Rng};
use crackhead::scenes::Input;

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;
const MAP_SIZE: usize = 6; // 방 격자 한 변의 방 개수 — pacman.rs 의 3라운드 근처 규모
const FOV: f32 = std::f32::consts::PI / 3.0;
const MOVE_SPEED: f32 = 2.4;
const ROT_SPEED: f32 = 2.6;
const PLAYER_RADIUS: f32 = 0.2;
const MARKER_WORLD_DIAMETER: f32 = 0.25;

const CEILING_COLOR: [f32; 4] = [0.10, 0.10, 0.16, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.16, 0.13, 0.09, 1.0];
const WALL_BASE_COLOR: [f32; 4] = [0.55, 0.55, 0.62, 1.0];
const MARKER_COLOR: [f32; 4] = [0.4, 0.85, 0.95, 1.0]; // 코인과 구분되는 하늘색 — 여기선 "먹는" 개념이 없다

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    rng: Rng,
    rc: Raycaster,
    input: Input,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mut rng = Rng::new((date::now() * 1e6) as u64);
        let rc = new_maze(&mut rng);
        Stage { ctx, renderer, rng, rc, input: Input::default(), last_time: date::now() }
    }
}

// 새 미로를 만들고 시작 칸에서 뚫려있는 방향을 보게 한다 — pacman.rs::start_round
// 와 같은 요령.
fn new_maze(rng: &mut Rng) -> Raycaster {
    let start = (1usize, 1usize);
    let walls = generate_maze(MAP_SIZE, rng);
    let mut rc = Raycaster::new(walls, start.0 as f32 + 0.5, start.1 as f32 + 0.5);
    rc.player_dir = rc.face_open_direction(start);
    rc
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        if self.input.pressed(KeyCode::R) {
            self.rc = new_maze(&mut self.rng);
        }
        self.rc.apply_wasd(
            self.input.is_down(KeyCode::W),
            self.input.is_down(KeyCode::S),
            self.input.is_down(KeyCode::A),
            self.input.is_down(KeyCode::D),
            MOVE_SPEED,
            ROT_SPEED,
            PLAYER_RADIUS,
            dt,
        );

        self.renderer.begin(WIN_W, WIN_H);
        let area = Rect::new(0.0, 0.0, WIN_W, WIN_H);
        let col_depth = self.rc.render_walls(&mut self.renderer, area, FOV, CEILING_COLOR, FLOOR_COLOR, WALL_BASE_COLOR);

        // 바닥 칸마다 마커 하나 — render_billboards()가 여러 개를 거리순으로
        // 잘 가리는지, 벽에 가려질 때 제대로 안 그려지는지 눈으로 바로 확인하려는
        // 용도라 "먹으면 사라지는" 로직은 없다(팩맨의 코인과 달리 계속 남아있다).
        let mut markers: Vec<Billboard> = Vec::new();
        for (y, row) in self.rc.walls.iter().enumerate() {
            for (x, &is_wall) in row.iter().enumerate() {
                if !is_wall {
                    markers.push(Billboard { x: x as f32 + 0.5, y: y as f32 + 0.5, world_diameter: MARKER_WORLD_DIAMETER, color: MARKER_COLOR });
                }
            }
        }
        self.rc.render_billboards(&mut self.renderer, area, FOV, &col_depth, &markers);

        self.renderer.rect(0.0, 0.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.55]);
        self.renderer.text(6.0, 3.0, "raycaster.rs test - WASD move, R = new maze, Esc = quit", 0.7, [1.0, 1.0, 1.0, 1.0]);

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        if keycode == KeyCode::Escape {
            window::order_quit();
        }
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.input.on_key_up(keycode);
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "Raycaster Test".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        // director_panel.rs 와 같은 이유 — CRT 가상 해상도 변환이 없는 단순한
        // 창이라 high_dpi 로 인한 논리/물리 픽셀 불일치 여지를 아예 없앤다.
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
