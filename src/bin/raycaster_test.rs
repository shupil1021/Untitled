// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::raycaster`(팩맨을 만들면서 뽑아낸, 앞으로 다른 1인칭 3D 미니게임에도
//! 쓸 재사용 엔진)만 따로 띄워서 확인하는 가장 작은 테스트 창이다.
//!
//! 실제 게임(crackhead.exe)이나 팩맨의 라운드/코인 규칙과는 전혀 무관하다 —
//! 미로가 아니라 사방이 벽으로만 둘러싸인 뻥 뚫린 방 하나를 만들고, 그 바닥에
//! 책상/의자/상자/책장처럼 서로 다른 크기·모양의 물체(Prop3D, 진짜 입체라 옆에서
//! 보면 실제로 옆면이 보인다)를 몇 개씩 무작위로 흩어놓은 뒤 WASD 로 걸어 다니며
//! 벽/물체가 서로 잘 가려지는지 확인한다. `cargo run --bin raycaster_test` 로 띄운다.
//!
//! R 키를 누르면 같은 방 안에 물체 배치만 다시 무작위로 뽑는다 — 여러 배치를
//! 빠르게 훑어보면서 가려짐/충돌이 괜찮은지 확인할 때 쓴다.

use miniquad::*;

use crackhead::gfx::{Rect, Renderer};
use crackhead::raycaster::{Prop3D, Raycaster, Rng};
use crackhead::scenes::Input;

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;
// 방 크기(테두리 벽 포함) — 미로가 아니라 안쪽이 전부 뚫린 사각형 하나다.
const ROOM_W: usize = 16;
const ROOM_H: usize = 12;
const ITEM_COUNT: usize = 8; // 방 안에 흩어놓을 물체 개수(의자는 좌판+등받이 2조각이라 실제 Prop3D 수는 더 많다)

const FOV: f32 = std::f32::consts::PI / 3.0;
const MOVE_SPEED: f32 = 2.4;
const ROT_SPEED: f32 = 2.6;
const PLAYER_RADIUS: f32 = 0.2;

const CEILING_COLOR: [f32; 4] = [0.10, 0.10, 0.16, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.16, 0.13, 0.09, 1.0];
const WALL_BASE_COLOR: [f32; 4] = [0.55, 0.55, 0.62, 1.0];
const DESK_COLOR: [f32; 4] = [0.42, 0.27, 0.14, 1.0]; // 짙은 나무색
const CHAIR_COLOR: [f32; 4] = [0.55, 0.38, 0.2, 1.0]; // 책상보다 살짝 밝은 나무색
const CRATE_COLOR: [f32; 4] = [0.5, 0.42, 0.3, 1.0]; // 나무 상자
const SHELF_COLOR: [f32; 4] = [0.28, 0.18, 0.1, 1.0]; // 어두운 원목 책장

// 사방이 벽인 한 칸짜리 테두리 + 안쪽은 전부 뚫린 방. generate_maze 처럼 방을
// 여러 개로 쪼개는 미로가 아니라 "가구를 놓고 걸어 다닐 빈 바닥"이 목적이라
// 훨씬 단순하다.
fn generate_open_room(w: usize, h: usize) -> Vec<Vec<bool>> {
    let mut walls = vec![vec![false; w]; h];
    walls[0].iter_mut().for_each(|c| *c = true);
    walls[h - 1].iter_mut().for_each(|c| *c = true);
    for row in walls.iter_mut() {
        row[0] = true;
        row[w - 1] = true;
    }
    walls
}

// 이 테스트에서 굴려볼 가구 종류 — 크기/색이 서로 달라야 "여러 사물"을 놓아본
// 것답게 눈으로 구분된다. 의자만 좌판+등받이 두 조각이라 Prop3D 를 두 개 낸다.
enum ItemKind {
    Desk,
    Chair,
    Crate,
    Shelf,
}

fn item_props(kind: &ItemKind, x: f32, y: f32) -> Vec<Prop3D> {
    match kind {
        ItemKind::Desk => vec![Prop3D { center_x: x, center_y: y, width: 0.9, depth: 0.6, height: 0.4, base: 0.0, color: DESK_COLOR }],
        ItemKind::Chair => vec![
            Prop3D { center_x: x, center_y: y, width: 0.45, depth: 0.45, height: 0.18, base: 0.0, color: CHAIR_COLOR }, // 좌판
            Prop3D { center_x: x, center_y: y, width: 0.45, depth: 0.08, height: 0.4, base: 0.18, color: CHAIR_COLOR }, // 등받이
        ],
        ItemKind::Crate => vec![Prop3D { center_x: x, center_y: y, width: 0.5, depth: 0.5, height: 0.5, base: 0.0, color: CRATE_COLOR }],
        ItemKind::Shelf => vec![Prop3D { center_x: x, center_y: y, width: 0.9, depth: 0.25, height: 0.9, base: 0.0, color: SHELF_COLOR }],
    }
}

// 테두리에서 한 칸 띄운 안쪽 바닥 칸들을 섞어서 ITEM_COUNT 곳을 고르고, 매번
// 무작위 가구 종류를 하나씩 놓는다 — 벽에 바짝 붙어서 절반이 파묻혀 보이지
// 않게 테두리는 아예 후보에서 뺀다.
fn scatter_items(rng: &mut Rng) -> Vec<Prop3D> {
    let mut cells: Vec<(f32, f32)> = Vec::new();
    for y in 2..ROOM_H - 2 {
        for x in 2..ROOM_W - 2 {
            cells.push((x as f32 + 0.5, y as f32 + 0.5));
        }
    }
    // Fisher-Yates
    for i in (1..cells.len()).rev() {
        let j = rng.gen_range(i + 1);
        cells.swap(i, j);
    }

    let mut props = Vec::new();
    for &(x, y) in cells.iter().take(ITEM_COUNT) {
        let kind = match rng.gen_range(4) {
            0 => ItemKind::Desk,
            1 => ItemKind::Chair,
            2 => ItemKind::Crate,
            _ => ItemKind::Shelf,
        };
        props.extend(item_props(&kind, x, y));
    }
    props
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    rng: Rng,
    rc: Raycaster,
    props: Vec<Prop3D>,
    input: Input,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mut rng = Rng::new((date::now() * 1e6) as u64);
        let walls = generate_open_room(ROOM_W, ROOM_H);
        let mut rc = Raycaster::new(walls, ROOM_W as f32 / 2.0, ROOM_H as f32 - 1.5);
        rc.player_dir = -std::f32::consts::PI / 2.0; // 방 남쪽 벽 앞에서 시작해서 북쪽(방 안쪽)을 보게
        let props = scatter_items(&mut rng);
        Stage { ctx, renderer, rng, rc, props, input: Input::default(), last_time: date::now() }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        if self.input.pressed(KeyCode::R) {
            self.props = scatter_items(&mut self.rng);
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
        let mut col_depth = self.rc.render_walls(&mut self.renderer, area, FOV, CEILING_COLOR, FLOOR_COLOR, WALL_BASE_COLOR);
        self.rc.render_props(&mut self.renderer, area, FOV, &mut col_depth, &self.props);

        self.renderer.rect(0.0, 0.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.55]);
        self.renderer.text(6.0, 3.0, "raycaster.rs test - WASD move, R = reshuffle items, Esc = quit", 0.7, [1.0, 1.0, 1.0, 1.0]);

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
