//! 설치 마법사를 끝내고 바탕화면에 생긴 게임 아이콘(FileKind::GameInstalled(GameKind::Pacman))을
//! 열면 뜨는 창 — 레이캐스팅(Wolfenstein 3D 식 2.5D)으로 미로를 보여주는 1인칭
//! 팩맨이다. 실제 텍스처/유령 같은 건 아직 없고, 미로를 걸어 다니며 그 라운드의
//! 코인을 전부 먹으면 다음 라운드로 넘어가는 5라운드 구성까지 만들었다.
//!
//! 벽 DDA 레이캐스팅/충돌 이동/미로 생성/빌보드 렌더링 같은 엔진 부분은 전부
//! `crate::raycaster`(앞으로 다른 1인칭 3D 미니게임을 추가할 때도 같이 쓸 재사용
//! 모듈)로 옮겼다 — 여기 남은 건 라운드 진행/코인/HUD 처럼 팩맨만의 규칙이다.
//!
//! 조작: W/S 로 바라보는 방향으로 앞/뒤 이동, A/D 로 좌우 회전(스트레이프 아님 —
//! 옛날 울펜슈타인처럼 몸 전체가 도는 방식). 창이 포커스를 잃으면(win.focused
//! false) 키 입력을 아예 안 읽는다 — 다른 창을 조작하다가 실수로 팩맨이 움직이는
//! 것을 막는다.
//!
//! 미로는 1~4라운드는 매번 랜덤 Prim 알고리즘(raycaster::generate_maze)으로 새로
//! 생성해서 플레이할 때마다 다르게 나오고, 5라운드만 고정 시드로 생성해 항상
//! 같은 미로가 나오게 한다. 코인은 시작 칸을 뺀 바닥 칸 전부에 놓는다.

use miniquad::{KeyCode, RenderingBackend};

use crate::gfx::{Assets, Rect, Renderer};
use crate::raycaster::{braid_maze, generate_maze, Billboard, Raycaster, Rng};
use crate::ui::WHITE;

use super::{App, AppAction, WinInput};

// 라운드별 맵 크기 — generate_maze(랜덤 Prim)가 만들 "방" 격자의 한 변 길이(방 개수)다.
// 실제 미로 격자 크기는 (2*map_size+1) — 방 사이사이에 벽을 끼워 넣는 표준적인
// 미로-생성 표현이라, 방이 N개면 격자는 2N+1칸이 된다. map_size 는 1~14 사이로
// 쓴다(요청받은 범위). 코인은 라운드별 목표 개수를 따로 정하지 않고 그 라운드
// 맵의 바닥 칸 전부에 놓는다(place_coins 참고) — 그래서 실제 coins_total 은
// 맵 크기에 따라 자연히 정해진다.
const ROUND_MAP_SIZES: [usize; 5] = [4, 5, 7, 9, 14];
// 5라운드(마지막, 인덱스 4)만 매번 같은 미로가 나오도록 고정 시드를 쓴다 —
// 나머지 1~4라운드는 PacmanApp 이 들고 있는 rng(시간 기반 시드)를 그대로 써서
// 플레이할 때마다 다르게 나온다.
const ROUND5_SEED: u64 = 0x50AC_11A5_FE1D_5EED;
const START_CELL: (usize, usize) = (1, 1); // generate_maze 는 항상 방 (0,0) → 격자 (1,1) 에서 시작한다
// generate_maze 가 만든 완전미로(막다른 길 천지)에 braid_maze 로 루프를 더하는
// 확률 — 1.0 이면 막다른 길을 최대한 다 없애서 "끊기는" 구조가 아니라 서로
// 이어지는 구조가 되게 한다(braid_maze 주석 참고).
const MAZE_BRAID_CHANCE: f32 = 1.0;

const MOVE_SPEED: f32 = 2.4; // 초당 이동 칸 수
const ROT_SPEED: f32 = 2.6;  // 초당 회전 라디안
const PLAYER_RADIUS: f32 = 0.2; // 벽에서 이만큼은 떨어져 있도록(코너에 끼지 않게)
const FOV: f32 = std::f32::consts::PI / 3.0; // 시야각 60도
const ROUND_CLEAR_HOLD: f32 = 2.0; // "Round Clear" 화면을 보여주는 시간(초)

const COIN_WORLD_DIAMETER: f32 = 0.28; // 코인의 실제 월드 크기(칸 크기=1.0 기준) — 벽 투영과 같은 척도로 원근감을 준다
const COIN_COLOR: [f32; 4] = [0.95, 0.82, 0.15, 1.0]; // 게임 아이콘과 같은 노란색
const CEILING_COLOR: [f32; 4] = [0.10, 0.10, 0.16, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.16, 0.13, 0.09, 1.0];
const WALL_BASE_COLOR: [f32; 4] = [0.55, 0.55, 0.62, 1.0];

// 시작 칸을 뺀 바닥 칸 전부에 코인을 놓는다. 실제로 놓인 개수(=이 라운드의
// coins_total)도 같이 돌려준다.
fn place_coins(walls: &[Vec<bool>], start: (usize, usize)) -> (Vec<Vec<bool>>, usize) {
    let mut floor_cells: Vec<(usize, usize)> = Vec::new();
    for (y, row) in walls.iter().enumerate() {
        for (x, &is_wall) in row.iter().enumerate() {
            if !is_wall && (x, y) != start {
                floor_cells.push((x, y));
            }
        }
    }

    let mut coins = vec![vec![false; walls[0].len()]; walls.len()];
    for &(x, y) in &floor_cells {
        coins[y][x] = true;
    }
    (coins, floor_cells.len())
}

enum RoundPhase {
    Playing,
    // 라운드를 막 클리어한 뒤 "Round Clear" 화면을 보여주는 동안 지난 시간(초) —
    // ROUND_CLEAR_HOLD 가 지나면 다음 라운드를 시작한다.
    Clear(f32),
}

pub struct PacmanApp {
    rng: Rng,
    round: usize, // ROUND_MAP_SIZES 인덱스(0부터) — HUD 에는 +1 해서 보여준다
    phase: RoundPhase,
    rc: Raycaster,
    coins: Vec<Vec<bool>>, // coins[y][x] — true 면 아직 안 먹은 코인이 있음
    coins_total: usize,
    coins_collected: usize,
}

impl Default for PacmanApp {
    fn default() -> Self {
        Self::new()
    }
}

impl PacmanApp {
    pub fn new() -> PacmanApp {
        let mut app = PacmanApp {
            rng: Rng::new((miniquad::date::now() * 1e6) as u64),
            round: 0,
            phase: RoundPhase::Playing,
            rc: Raycaster::new(vec![vec![false]], 1.5, 1.5),
            coins: vec![vec![false]],
            coins_total: 0,
            coins_collected: 0,
        };
        app.start_round(0);
        app
    }

    // round_idx 의 미로/코인을 새로 만들고 플레이어를 시작 칸으로 되돌린다 —
    // 게임을 처음 시작할 때도, 라운드를 클리어하고 다음으로 넘어갈 때도 이걸 쓴다.
    fn start_round(&mut self, round_idx: usize) {
        let map_size = ROUND_MAP_SIZES[round_idx];
        // 마지막 라운드(5라운드)만 매번 같은 미로가 나오도록 고정 시드를 쓴다 —
        // 그 외 라운드는 self.rng(시간 기반, 이어 쓰는 상태)를 써서 플레이할
        // 때마다 다르게 나온다. 코인은 바닥 칸 전부에 놓으므로(place_coins) 랜덤
        // 요소가 없다 — 미로 생성/브레이딩에만 rng 가 필요하다. 5라운드는 미로
        // 생성과 브레이딩 둘 다 같은 고정 시드 Rng 하나를 이어 써야(따로따로 새
        // Rng 를 만들면 각자 첫 값부터 다시 시작해 브레이딩 패턴이 흐트러진다)
        // 매번 정확히 같은 결과가 나온다.
        let mut round5_rng = Rng::new(ROUND5_SEED);
        let rng: &mut Rng = if round_idx == ROUND_MAP_SIZES.len() - 1 { &mut round5_rng } else { &mut self.rng };
        let mut walls = generate_maze(map_size, rng);
        // generate_maze 는 완전미로(스패닝 트리)라 막다른 길이 아주 많다 — 그대로
        // 두면 갈림길마다 결국 되돌아 나와야 하는 "끊기는" 구조로 느껴진다.
        // braid_maze 로 막다른 방마다 벽을 하나씩 더 허물어서(MAZE_BRAID_CHANCE
        // 확률) 루프를 만들고, 경로가 서로 이어지는(끊기지 않는) 구조로 바꾼다.
        braid_maze(&mut walls, map_size, rng, MAZE_BRAID_CHANCE);
        let (coins, placed) = place_coins(&walls, START_CELL);
        self.rc = Raycaster::new(walls, START_CELL.0 as f32 + 0.5, START_CELL.1 as f32 + 0.5);
        self.rc.player_dir = self.rc.face_open_direction(START_CELL);
        self.coins = coins;
        self.coins_total = placed;
        self.coins_collected = 0;
        self.round = round_idx;
    }

    fn collect_coin_here(&mut self) {
        let (cx, cy) = (self.rc.player_x as usize, self.rc.player_y as usize);
        if let Some(has_coin) = self.coins.get_mut(cy).and_then(|row| row.get_mut(cx))
            && *has_coin
        {
            *has_coin = false;
            self.coins_collected += 1;
        }
    }

    // 화면 전체를 검은 배경으로 덮고 가운데에 "Round Clear" 를 띄운다.
    fn draw_round_clear(&self, r: &mut Renderer, area: Rect) {
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        let title = "Round Clear";
        let scale = 1.6;
        let tw = r.text_width(title, scale);
        r.text(area.x + (area.w - tw) / 2.0, area.y + area.h / 2.0 - 12.0, title, scale, WHITE);
    }
}

impl App for PacmanApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        if let RoundPhase::Clear(elapsed) = &mut self.phase {
            *elapsed += win.dt;
            let elapsed = *elapsed;
            self.draw_round_clear(r, area);
            if elapsed >= ROUND_CLEAR_HOLD {
                // 마지막 라운드를 깼으면 일단 처음 라운드로 되돌아간다 — "올 클리어"
                // 화면은 아직 없다(다음에 채울 자리).
                let next = (self.round + 1) % ROUND_MAP_SIZES.len();
                self.start_round(next);
                self.phase = RoundPhase::Playing;
            }
            return AppAction::None;
        }

        // 다른 창을 조작하는 중엔 키 입력을 아예 안 읽는다.
        if win.focused {
            self.rc.apply_wasd(
                win.input.is_down(KeyCode::W),
                win.input.is_down(KeyCode::S),
                win.input.is_down(KeyCode::A),
                win.input.is_down(KeyCode::D),
                MOVE_SPEED,
                ROT_SPEED,
                PLAYER_RADIUS,
                win.dt,
            );
        }
        self.collect_coin_here();

        let col_depth = self.rc.render_walls(r, area, FOV, CEILING_COLOR, FLOOR_COLOR, WALL_BASE_COLOR);

        let mut billboards: Vec<Billboard> = Vec::new();
        for (y, row) in self.coins.iter().enumerate() {
            for (x, &has_coin) in row.iter().enumerate() {
                if has_coin {
                    billboards.push(Billboard { x: x as f32 + 0.5, y: y as f32 + 0.5, world_diameter: COIN_WORLD_DIAMETER, color: COIN_COLOR });
                }
            }
        }
        self.rc.render_billboards(r, area, FOV, &col_depth, &billboards);

        // 상단 HUD — (라운드)Round (지금 먹은 코인)/(이 라운드 전체 코인).
        let hud = format!("{}Round {}/{}", self.round + 1, self.coins_collected, self.coins_total);
        r.rect(area.x, area.y, area.w, 20.0, [0.0, 0.0, 0.0, 0.55]);
        r.text(area.x + 8.0, area.y + 4.0, &hud, 0.9, WHITE);

        // 이 라운드의 코인을 다 먹었으면(그리고 애초에 코인이 하나라도 있었으면)
        // 다음 프레임부터 "Round Clear" 화면으로 넘어간다.
        if self.coins_total > 0 && self.coins_collected >= self.coins_total {
            self.phase = RoundPhase::Clear(0.0);
        }

        AppAction::None
    }
}
