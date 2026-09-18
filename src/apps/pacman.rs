//! 설치 마법사를 끝내고 바탕화면에 생긴 게임 아이콘(FileKind::GameInstalled(GameKind::Pacman))을
//! 열면 뜨는 창 — 레이캐스팅(Wolfenstein 3D 식 2.5D)으로 미로를 보여주는 1인칭
//! 팩맨이다. 실제 텍스처/유령 같은 건 아직 없고, 미로를 걸어 다니며 그 라운드의
//! 코인을 전부 먹으면 다음 라운드로 넘어가는 5라운드 구성까지 만들었다.
//!
//! 조작: W/S 로 바라보는 방향으로 앞/뒤 이동, A/D 로 좌우 회전(스트레이프 아님 —
//! 옛날 울펜슈타인처럼 몸 전체가 도는 방식). 창이 포커스를 잃으면(win.focused
//! false) 키 입력을 아예 안 읽는다 — 다른 창을 조작하다가 실수로 팩맨이 움직이는
//! 것을 막는다.
//!
//! 미로는 1~4라운드는 매번 랜덤 Prim 알고리즘으로 새로 생성하고(ROOMS×ROOMS 개의
//! 방을 완전미로 — 루프 없이 전부 연결된 트리 — 로 파서 (2×ROOMS+1) 크기 격자에
//! 옮겨 담는다), 5라운드만 고정 시드로 생성해 항상 같은 미로가 나오게 한다. 랜덤
//! Prim 은 재귀 백트래커보다 짧은 막다른 길과 급한 방향 전환이 훨씬 많이 나와서
//! 매판 더 꼬여 보인다(generate_maze 주석 참고). 코인은 시작 칸을 뺀 바닥 칸
//! 전부에 놓는다.
//!
//! 렌더링은 컬럼(화면 x 좌표)마다 광선 하나씩 DDA(Digital Differential Analysis)로
//! 쏴서 가장 가까운 벽까지 거리를 구하고, 그 거리에 반비례하는 높이의 세로띠를
//! 그리는 고전적인 레이캐스터 방식이다(텍스처 없이 거리/면 방향에 따른 음영만).

use std::f32::consts::PI;

use miniquad::{KeyCode, RenderingBackend};

use crate::gfx::{Assets, Rect, Renderer};
use crate::ui::{fill_circle, WHITE};

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

const MOVE_SPEED: f32 = 2.4; // 초당 이동 칸 수
const ROT_SPEED: f32 = 2.6;  // 초당 회전 라디안
const PLAYER_RADIUS: f32 = 0.2; // 벽에서 이만큼은 떨어져 있도록(코너에 끼지 않게)
const FOV: f32 = PI / 3.0; // 시야각 60도
const ROUND_CLEAR_HOLD: f32 = 2.0; // "Round Clear" 화면을 보여주는 시간(초)

const COIN_WORLD_DIAMETER: f32 = 0.28; // 코인의 실제 월드 크기(칸 크기=1.0 기준) — 벽 투영과 같은 척도로 원근감을 준다
const COIN_COLOR: [f32; 4] = [0.95, 0.82, 0.15, 1.0]; // 게임 아이콘과 같은 노란색
const CEILING_COLOR: [f32; 4] = [0.10, 0.10, 0.16, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.16, 0.13, 0.09, 1.0];

// installer.rs/game_installer.rs 의 로딩바 waypoint 생성에 쓰던 것과 같은 아주
// 단순한 xorshift64 의사난수 — 여기서는 미로 생성(방문 순서)과 코인 자리
// 셔플(Fisher–Yates)에 쓴다. PacmanApp 이 하나 들고 있다가 라운드가 바뀔 때마다
// 계속 이어서 쓰므로, 매 라운드 미로/코인 배치가 서로 다르게 나온다.
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
    // 0..n 사이 정수 하나(n==0 이면 0). 진짜 균등분포는 아니지만(모듈로 편향)
    // 미로 생성/코인 셔플 정도엔 전혀 티가 안 난다.
    fn gen_range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            self.next_u32() as usize % n
        }
    }
}

// (from 방, to 방) 간선 하나 — generate_maze(랜덤 Prim)의 frontier 목록 원소.
type RoomEdge = ((usize, usize), (usize, usize));

// room 의 아직 안 가본 이웃 방들을 (from, to) 간선으로 frontier 에 추가한다 —
// generate_maze(랜덤 Prim) 가 매 반복 이 목록에서 하나씩 골라 쓴다.
fn push_frontier(rooms: usize, room: (usize, usize), visited: &[Vec<bool>], frontier: &mut Vec<RoomEdge>) {
    let (x, y) = room;
    let mut neighbors: Vec<(usize, usize)> = Vec::new();
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < rooms {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < rooms {
        neighbors.push((x, y + 1));
    }
    for n in neighbors {
        if !visited[n.1][n.0] {
            frontier.push((room, n));
        }
    }
}

// 랜덤 Prim 알고리즘으로 완전미로(루프 없는 스패닝 트리)를 만든다. rooms×rooms
// 개의 "방" 중 지금까지 미로에 편입된 방들의 "경계에 걸친" 간선들을 frontier
// 에 모아두고, 그중 완전히 무작위로 하나를 뽑아 편입시키는 과정을 반복한다 —
// 재귀 백트래커(한 번 뚫은 방향으로 갈 수 있는 데까지 쭉 파고들어서 길게 뻗은
// 복도가 되기 쉽다)와 달리, "지금 미로 전체의 어느 가장자리에서" 다음 칸을
// 파도 상관없어서 짧은 막다른 길과 급한 방향 전환이 훨씬 많이 나온다 — 매판이
// 훨씬 더 꼬여 보이는 이유. 반환하는 격자는 (2*rooms+1) 크기이고, 방은 홀수
// 좌표(2r+1, 2c+1)에, 방 사이 벽은 그 중간 짝수 좌표에 온다(바깥 테두리는
// 항상 벽).
fn generate_maze(rooms: usize, rng: &mut Rng) -> Vec<Vec<bool>> {
    let rooms = rooms.max(1);
    let dim = rooms * 2 + 1;
    let mut walls = vec![vec![true; dim]; dim];
    let mut visited = vec![vec![false; rooms]; rooms];

    visited[0][0] = true;
    walls[1][1] = false;
    let mut frontier: Vec<RoomEdge> = Vec::new();
    push_frontier(rooms, (0, 0), &visited, &mut frontier);

    while !frontier.is_empty() {
        // swap_remove 면 순서가 흐트러지지만 어차피 매번 무작위로 고르므로 상관없고,
        // Vec::remove 처럼 뒤 원소들을 매번 당겨오지 않아도 돼서 더 빠르다.
        let i = rng.gen_range(frontier.len());
        let (from, to) = frontier.swap_remove(i);
        if visited[to.1][to.0] {
            continue; // 그 사이 다른 간선으로 이미 편입된 방이면 버린다
        }
        // 두 방 grid 좌표가 (2fx+1,2fy+1)/(2tx+1,2ty+1) 이므로, 그 중간(허물 벽)은
        // 항상 (fx+tx+1, fy+ty+1) — 가로/세로 어느 쪽이든 이 식 하나로 된다.
        walls[from.1 + to.1 + 1][from.0 + to.0 + 1] = false;
        walls[2 * to.1 + 1][2 * to.0 + 1] = false;
        visited[to.1][to.0] = true;
        push_frontier(rooms, to, &visited, &mut frontier);
    }
    walls
}

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
    walls: Vec<Vec<bool>>, // walls[y][x] — true 면 벽
    coins: Vec<Vec<bool>>, // coins[y][x] — true 면 아직 안 먹은 코인이 있음
    coins_total: usize,
    coins_collected: usize,
    fog_dist: f32, // 이 라운드 맵 크기에 맞춘 안개 거리 — 클수록 안 보일 때까지 더 멀리 봐야 함
    player_x: f32,
    player_y: f32,
    player_dir: f32,
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
            walls: vec![vec![false]],
            coins: vec![vec![false]],
            coins_total: 0,
            coins_collected: 0,
            fog_dist: 16.0,
            player_x: 1.5,
            player_y: 1.5,
            player_dir: 0.0,
        };
        app.start_round(0);
        app
    }

    // round_idx 의 미로/코인을 새로 만들고 플레이어를 시작 칸으로 되돌린다 —
    // 게임을 처음 시작할 때도, 라운드를 클리어하고 다음으로 넘어갈 때도 이걸 쓴다.
    fn start_round(&mut self, round_idx: usize) {
        let map_size = ROUND_MAP_SIZES[round_idx];
        let start = (1usize, 1usize); // generate_maze 는 항상 방 (0,0) → 격자 (1,1) 에서 시작한다
        // 마지막 라운드(5라운드)만 매번 같은 미로가 나오도록 고정 시드를 쓴다 —
        // 그 외 라운드는 self.rng(시간 기반, 이어 쓰는 상태)를 써서 플레이할
        // 때마다 다르게 나온다. 코인은 바닥 칸 전부에 놓으므로(place_coins) 랜덤
        // 요소가 없다 — 미로 생성에만 rng 가 필요하다.
        self.walls = if round_idx == ROUND_MAP_SIZES.len() - 1 {
            generate_maze(map_size, &mut Rng::new(ROUND5_SEED))
        } else {
            generate_maze(map_size, &mut self.rng)
        };
        let (coins, placed) = place_coins(&self.walls, start);
        self.coins = coins;
        self.coins_total = placed;
        self.coins_collected = 0;
        self.round = round_idx;
        self.fog_dist = self.walls.len() as f32 * 1.5;
        self.player_x = 1.5;
        self.player_y = 1.5;
        self.player_dir = 0.0;
    }

    // 맵 밖이거나 벽이면 true(레이캐스팅/충돌 판정 둘 다 이 하나로 처리 — 밖은
    // 항상 벽 취급해서 광선이 격자 밖으로 나가 인덱스 범위를 벗어나는 일이 없다).
    fn is_wall(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return true;
        }
        let (mx, my) = (x as usize, y as usize);
        my >= self.walls.len() || mx >= self.walls[0].len() || self.walls[my][mx]
    }

    // 축을 따로 검사해서 벽을 따라 미끄러지듯 이동한다(둘 다 막혀있지 않은 이상
    // 완전히 멈추지 않는다) — PLAYER_RADIUS 만큼은 벽에서 떨어져 있도록 이동
    // 방향 쪽으로 살짝 더 나간 지점도 같이 비어있는지 본다.
    fn try_move(&mut self, dx: f32, dy: f32) {
        if dx != 0.0 {
            let nx = self.player_x + dx;
            if !self.is_wall(nx + PLAYER_RADIUS * dx.signum(), self.player_y) {
                self.player_x = nx;
            }
        }
        if dy != 0.0 {
            let ny = self.player_y + dy;
            if !self.is_wall(self.player_x, ny + PLAYER_RADIUS * dy.signum()) {
                self.player_y = ny;
            }
        }
    }

    fn collect_coin_here(&mut self) {
        let (cx, cy) = (self.player_x as usize, self.player_y as usize);
        if let Some(has_coin) = self.coins.get_mut(cy).and_then(|row| row.get_mut(cx))
            && *has_coin
        {
            *has_coin = false;
            self.coins_collected += 1;
        }
    }

    // 한 컬럼(광선 하나)이 맞는 벽까지의 "유클리드" 거리(플레이어 ~ 충돌점 직선
    // 거리)와, 그 벽이 동서(수직 격자선)쪽 면인지 남북(수평 격자선)쪽 면인지를
    // DDA 로 구한다. ray_dir 을 (cos, sin) 단위벡터로 만들었기 때문에(정규화된
    // 벡터) side_dist - delta_dist 는 Lodev 식 카메라-평면 레이캐스터에서처럼
    // 자동으로 fisheye 가 보정된 "수직 거리"가 아니라 진짜 유클리드 거리로 나온다
    // — 그래서 호출부(update())가 이 값에 cos(광선각 - 플레이어각)를 곱해 직접
    // 수직 거리로 보정한다. side 는 세로띠 음영에 쓴다(같은 벽이라도 면 방향에
    // 따라 살짝 다르게 보이도록).
    fn cast_ray(&self, angle: f32) -> (f32, bool) {
        let ray_dir_x = angle.cos();
        let ray_dir_y = angle.sin();
        let mut map_x = self.player_x.floor() as i32;
        let mut map_y = self.player_y.floor() as i32;

        let delta_dist_x = if ray_dir_x.abs() < 1e-6 { f32::INFINITY } else { (1.0 / ray_dir_x).abs() };
        let delta_dist_y = if ray_dir_y.abs() < 1e-6 { f32::INFINITY } else { (1.0 / ray_dir_y).abs() };

        let (step_x, mut side_dist_x) = if ray_dir_x < 0.0 {
            (-1, (self.player_x - map_x as f32) * delta_dist_x)
        } else {
            (1, (map_x as f32 + 1.0 - self.player_x) * delta_dist_x)
        };
        let (step_y, mut side_dist_y) = if ray_dir_y < 0.0 {
            (-1, (self.player_y - map_y as f32) * delta_dist_y)
        } else {
            (1, (map_y as f32 + 1.0 - self.player_y) * delta_dist_y)
        };

        let mut side_is_ns = false; // 마지막으로 건넌 격자선이 남북(수평) 방향인지
        for _ in 0..1024 {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                side_is_ns = false;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                side_is_ns = true;
            }
            if self.is_wall(map_x as f32, map_y as f32) {
                let dist = if side_is_ns { side_dist_y - delta_dist_y } else { side_dist_x - delta_dist_x };
                return (dist.max(0.0001), side_is_ns);
            }
        }
        (self.fog_dist, side_is_ns) // 벽에 안 부딪히면(맵이 뚫려있을 리는 없지만) 안개 끝까지
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
            if win.input.is_down(KeyCode::A) {
                self.player_dir -= ROT_SPEED * win.dt;
            }
            if win.input.is_down(KeyCode::D) {
                self.player_dir += ROT_SPEED * win.dt;
            }
            let (fx, fy) = (self.player_dir.cos(), self.player_dir.sin());
            let mut dx = 0.0;
            let mut dy = 0.0;
            if win.input.is_down(KeyCode::W) {
                dx += fx * MOVE_SPEED * win.dt;
                dy += fy * MOVE_SPEED * win.dt;
            }
            if win.input.is_down(KeyCode::S) {
                dx -= fx * MOVE_SPEED * win.dt;
                dy -= fy * MOVE_SPEED * win.dt;
            }
            if dx != 0.0 || dy != 0.0 {
                self.try_move(dx, dy);
            }
        }
        self.collect_coin_here();

        // 천장/바닥 — 화면을 반으로 갈라 한 번씩만 채우고, 그 위에 벽 세로띠를 덮는다.
        let half = area.h / 2.0;
        r.rect(area.x, area.y, area.w, half, CEILING_COLOR);
        r.rect(area.x, area.y + half, area.w, area.h - half, FLOOR_COLOR);

        // 컬럼(화면 가로 1px)마다 광선 하나 — DDA 로 벽까지 거리를 구해 세로띠로
        // 그린다. 나중에 코인(빌보드)을 그릴 때 "이 컬럼에서 벽보다 코인이 더
        // 가까운지" 가려짐 판정을 해야 해서, 컬럼별 깊이를 따로 기억해둔다.
        let num_rays = area.w.round().max(1.0) as usize;
        let col_w = area.w / num_rays as f32;
        let half_fov = FOV / 2.0;
        let mut col_depth = vec![self.fog_dist; num_rays];
        for (col, depth_slot) in col_depth.iter_mut().enumerate() {
            // -1.0(왼쪽 끝) .. 1.0(오른쪽 끝) — 화면 가운데가 플레이어가 보는 방향.
            let camera_x = 2.0 * (col as f32 + 0.5) / num_rays as f32 - 1.0;
            let rel_angle = camera_x * half_fov;
            let (ray_dist, side_is_ns) = self.cast_ray(self.player_dir + rel_angle);
            // cast_ray 는 단위벡터 방향으로 쏜 광선의 "진짜" 유클리드 거리를
            // 돌려준다 — 화면 중앙에서 먼 컬럼일수록 광선이 더 비스듬해서 벽까지
            // 실제 거리가 더 길게 나오는데, 그 값을 그대로 벽 높이 계산에 쓰면
            // (화면 중앙 기준으로 봤을 때) 평평한 벽도 가운데가 볼록 튀어나온
            // 것처럼 휘어 보인다(어안 렌즈 효과). rel_angle 의 코사인을 곱해
            // 플레이어가 보는 방향 축에 투영한 "수직 거리(depth)"로 바꿔야
            // 벽이 실제로 평평하게 보인다.
            let depth = (ray_dist * rel_angle.cos()).max(0.0001);
            *depth_slot = depth;

            let wall_h = (area.h / depth).min(area.h * 4.0);
            let top = ((area.h - wall_h) / 2.0).clamp(0.0, area.h);
            let bottom = ((area.h + wall_h) / 2.0).clamp(0.0, area.h);

            let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
            let side_shade = if side_is_ns { 0.7 } else { 1.0 };
            let shade = fog * side_shade;
            let color = [0.55 * shade, 0.55 * shade, 0.62 * shade, 1.0];

            let x = area.x + col as f32 * col_w;
            r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), color);
        }

        // 코인 — 바닥에 놓인 작은 원(빌보드)으로 그린다. 플레이어 기준 상대각이
        // 시야각 안에 들고, 그 각도가 가리키는 컬럼에서 벽보다 코인이 더 가까울
        // 때만(안 가려졌을 때만) 그린다. 세로 위치는 "그 깊이에서 바닥이 화면의
        // 어디에 보이는지"(위 벽 렌더링의 bottom 과 같은 식)를 그대로 재사용해서
        // 벽과 어긋나 붕 떠 보이지 않게 한다. 깊이 버퍼가 따로 없는 빌보드라
        // 코인끼리는 먼저 전부 모아서 먼 것부터(화가 알고리즘) 그려야 한 복도
        // 안에 여러 개가 늘어서 있을 때 가까운(큰) 코인이 먼(작은) 코인을 제대로
        // 가린다.
        let mut visible_coins: Vec<(f32, f32, f32, f32, [f32; 4])> = Vec::new(); // (depth, screen_x, floor_y, radius, color)
        for (y, row) in self.coins.iter().enumerate() {
            for (x, &has_coin) in row.iter().enumerate() {
                if !has_coin {
                    continue;
                }
                let rel_x = x as f32 + 0.5 - self.player_x;
                let rel_y = y as f32 + 0.5 - self.player_y;
                let dist = rel_x.hypot(rel_y);
                if dist < 0.1 {
                    continue; // 바로 발밑 — collect_coin_here() 가 이미 처리했어야 함
                }
                let mut rel_angle = rel_y.atan2(rel_x) - self.player_dir;
                while rel_angle > PI {
                    rel_angle -= 2.0 * PI;
                }
                while rel_angle < -PI {
                    rel_angle += 2.0 * PI;
                }
                if rel_angle.abs() >= half_fov {
                    continue; // 시야 밖
                }
                let depth = (dist * rel_angle.cos()).max(0.0001);
                let camera_x = rel_angle / half_fov;
                let col = (((camera_x + 1.0) / 2.0) * num_rays as f32) as usize;
                let col = col.min(num_rays - 1);
                if depth >= col_depth[col] {
                    continue; // 벽에 가려짐
                }

                let wall_h = (area.h / depth).min(area.h * 4.0);
                // 벽 렌더링 쪽 top/bottom 은 r.rect 에 그대로 넘길 y/height 라서 화면
                // 안으로 clamp 가 필요하지만, 여기 floor_y 는 원 중심을 잡는 기준점일
                // 뿐이다 — 이것까지 clamp 하면 아주 가까이 다가갔을 때 화면 끝에
                // 고정된 채 반지름만 커져서 코인이 위로 떠오르는 것처럼 보인다.
                let floor_y = area.y + (area.h + wall_h) / 2.0;
                let screen_x = area.x + (camera_x + 1.0) / 2.0 * area.w;
                let radius = (COIN_WORLD_DIAMETER / 2.0 * area.h / depth).clamp(1.0, area.h * 4.0);
                let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
                let color = [COIN_COLOR[0] * fog, COIN_COLOR[1] * fog, COIN_COLOR[2] * fog, 1.0];
                visible_coins.push((depth, screen_x, floor_y, radius, color));
            }
        }
        // 먼 것부터(depth 내림차순) 그려서 가까운 코인이 항상 위에 온다.
        visible_coins.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, screen_x, floor_y, radius, color) in visible_coins {
            fill_circle(r, screen_x, floor_y - radius, radius, color);
        }

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
