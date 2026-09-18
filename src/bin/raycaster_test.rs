// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::raycaster`(팩맨을 만들면서 뽑아낸, 앞으로 다른 1인칭 3D 미니게임에도
//! 쓸 재사용 엔진)만 따로 띄워서 확인하는 가장 작은 테스트 창이다.
//!
//! 실제 게임(crackhead.exe)이나 팩맨의 라운드/코인 규칙과는 전혀 무관하다 —
//! 미로 하나를 생성하고, 바닥 칸 몇 군데에 책상/의자처럼 각진 물체(BillboardShape::Rect)
//! 를 놓아본 뒤(의자는 좌판+등받이 두 조각), 나머지 바닥 칸에는 작은 원형 마커를
//! 깔아 WASD 로 걸어 다니며 벽 충돌/레이캐스팅/빌보드 가려짐이 제대로 동작하는지
//! 본다. `cargo run --bin raycaster_test` 로 띄운다.
//!
//! ⚠ 책상/의자는 어디까지나 "각진 실루엣의 평면 빌보드"다 — 이 엔진은 벽만
//! 진짜 입체(DDA)로 그리고 나머지 물체는 전부 카메라를 향하는 평면이라, 정면
//! 에서는 그럴듯해도 옆으로 돌아가면 그대로 납작하게 보인다. 실제로 옆면이
//! 따로 보이는 가구가 필요해지면 벽처럼 격자에 부분 높이로 박아 넣는 훨씬 큰
//! 확장이 있어야 한다(raycaster.rs::BillboardShape 주석 참고).
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
const DESK_COLOR: [f32; 4] = [0.42, 0.27, 0.14, 1.0]; // 짙은 나무색
const CHAIR_COLOR: [f32; 4] = [0.55, 0.38, 0.2, 1.0]; // 책상보다 살짝 밝은 나무색

// 책상(넓고 낮은 상자) 하나 + 의자(좁고 낮은 좌판 + 그 위에 얹힌 등받이) 하나를
// 만들어 낼 빌보드들 — world_base 로 등받이를 좌판 높이만큼 띄운다.
fn furniture_billboards(desk: (f32, f32), chair: (f32, f32)) -> Vec<Billboard> {
    vec![
        Billboard::prop(desk.0, desk.1, 0.9, 0.4, 0.0, DESK_COLOR),
        Billboard::prop(chair.0, chair.1, 0.45, 0.18, 0.0, CHAIR_COLOR), // 좌판
        Billboard::prop(chair.0, chair.1, 0.45, 0.4, 0.18, CHAIR_COLOR), // 등받이(좌판 위에 얹힘)
    ]
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    rng: Rng,
    rc: Raycaster,
    // 책상/의자(BillboardShape::Rect) + 나머지 바닥 칸의 원형 마커(Circle) 를
    // 한 목록에 같이 담아둔다 — render_billboards() 한 번 호출로 거리순 가려짐
    // (가구가 마커를 가리는 경우 포함)까지 전부 처리되게 하려는 것.
    props: Vec<Billboard>,
    input: Input,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mut rng = Rng::new((date::now() * 1e6) as u64);
        let (rc, props) = new_maze(&mut rng);
        Stage { ctx, renderer, rng, rc, props, input: Input::default(), last_time: date::now() }
    }
}

// 새 미로를 만들고 시작 칸에서 뚫려있는 방향을 보게 한다(pacman.rs::start_round
// 와 같은 요령) — 그리고 시작 칸을 뺀 바닥 칸 중 처음 둘은 책상/의자 자리로,
// 나머지는 전부 원형 마커 자리로 쓴다(마커 자체는 pacman.rs 의 코인과 달리
// "먹으면 사라지는" 로직이 없다, 그냥 빌보드가 잘 그려지는지만 본다).
fn new_maze(rng: &mut Rng) -> (Raycaster, Vec<Billboard>) {
    let start = (1usize, 1usize);
    let walls = generate_maze(MAP_SIZE, rng);

    let mut floor_cells: Vec<(f32, f32)> = Vec::new();
    for (y, row) in walls.iter().enumerate() {
        for (x, &is_wall) in row.iter().enumerate() {
            if !is_wall && (x, y) != start {
                floor_cells.push((x as f32 + 0.5, y as f32 + 0.5));
            }
        }
    }
    let mut props = match (floor_cells.first(), floor_cells.get(1)) {
        (Some(&desk), Some(&chair)) => furniture_billboards(desk, chair),
        _ => Vec::new(), // 맵이 너무 작아 바닥 칸이 둘도 안 되면(map_size 1 등) 그냥 생략
    };
    props.extend(floor_cells.iter().skip(2).map(|&(x, y)| Billboard::coin(x, y, MARKER_WORLD_DIAMETER, MARKER_COLOR)));

    let mut rc = Raycaster::new(walls, start.0 as f32 + 0.5, start.1 as f32 + 0.5);
    rc.player_dir = rc.face_open_direction(start);
    (rc, props)
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        if self.input.pressed(KeyCode::R) {
            let (rc, props) = new_maze(&mut self.rng);
            self.rc = rc;
            self.props = props;
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

        self.rc.render_billboards(&mut self.renderer, area, FOV, &col_depth, &self.props);

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
