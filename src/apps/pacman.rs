//! 설치 마법사를 끝내고 바탕화면에 생긴 게임 아이콘(FileKind::GameInstalled(GameKind::Pacman))을
//! 열면 뜨는 창 — 첫 버전은 레이캐스팅(Wolfenstein 3D 식 2.5D)으로 미로를 보여주는
//! 1인칭 팩맨이다. 실제 텍스처/유령/스코어 저장 같은 건 아직 없고, 미로를 걸어
//! 다니며 코인을 먹는 것까지만 만든다.
//!
//! 조작: W/S 로 바라보는 방향으로 앞/뒤 이동, A/D 로 좌우 회전(스트레이프 아님 —
//! 옛날 울펜슈타인처럼 몸 전체가 도는 방식). 창이 포커스를 잃으면(win.focused
//! false) 키 입력을 아예 안 읽는다 — 다른 창을 조작하다가 실수로 팩맨이 움직이는
//! 것을 막는다.
//!
//! 미로는 문자 배열(MAP, '#'=벽 '.'=바닥)로 정의하고 new() 에서 bool 격자로 바꾼다.
//! 렌더링은 컬럼(화면 x 좌표)마다 광선 하나씩 DDA(Digital Differential Analysis)로
//! 쏴서 가장 가까운 벽까지 거리를 구하고, 그 거리에 반비례하는 높이의 세로띠를
//! 그리는 고전적인 레이캐스터 방식이다(텍스처 없이 거리/면 방향에 따른 음영만).

use std::f32::consts::PI;

use miniquad::{KeyCode, RenderingBackend};

use crate::gfx::{Assets, Rect, Renderer};

use super::{App, AppAction, WinInput};

// '#' = 벽, '.' = 바닥(코인이 놓일 수 있는 자리). 플레이어는 (1,1) 에서 시작한다
// (아래 START_X/START_Y). 모든 행의 길이가 같아야 한다 — new() 에서 그대로
// bool 격자로 옮겨 담는다.
const MAP: [&str; 13] = [
    "#############",
    "#...#...#...#",
    "#.#.#.#.#.#.#",
    "#.#.....#...#",
    "#.#.###.#.###",
    "#.#.#...#...#",
    "#.#.#.#####.#",
    "#...#.#.....#",
    "###.#.#.###.#",
    "#...#.#.#...#",
    "#.###.#.#.#.#",
    "#.....#...#.#",
    "#############",
];
const START_X: f32 = 1.5;
const START_Y: f32 = 1.5;
const START_DIR: f32 = 0.0; // 라디안, 0 = +x(동쪽)을 바라봄

const MOVE_SPEED: f32 = 2.4; // 초당 이동 칸 수
const ROT_SPEED: f32 = 2.6;  // 초당 회전 라디안
const PLAYER_RADIUS: f32 = 0.2; // 벽에서 이만큼은 떨어져 있도록(코너에 끼지 않게)
const FOV: f32 = PI / 3.0; // 시야각 60도
const MAX_DIST: f32 = 16.0; // 이보다 먼 벽은 완전히 어둡게(안개) — 13x13 미로 대각선보다 조금 크게

const COIN_COLOR: [f32; 4] = [0.95, 0.82, 0.15, 1.0]; // 게임 아이콘과 같은 노란색
const CEILING_COLOR: [f32; 4] = [0.10, 0.10, 0.16, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.16, 0.13, 0.09, 1.0];

pub struct PacmanApp {
    walls: Vec<Vec<bool>>, // walls[y][x] — true 면 벽
    coins: Vec<Vec<bool>>, // coins[y][x] — true 면 아직 안 먹은 코인이 있음
    coins_total: usize,
    coins_collected: usize,
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
        let walls: Vec<Vec<bool>> = MAP.iter().map(|row| row.bytes().map(|b| b == b'#').collect()).collect();
        let start_cell = (START_X.floor() as usize, START_Y.floor() as usize);
        let mut coins = vec![vec![false; walls[0].len()]; walls.len()];
        let mut coins_total = 0;
        for (y, row) in walls.iter().enumerate() {
            for (x, &is_wall) in row.iter().enumerate() {
                if !is_wall && (x, y) != start_cell {
                    coins[y][x] = true;
                    coins_total += 1;
                }
            }
        }
        PacmanApp {
            walls,
            coins,
            coins_total,
            coins_collected: 0,
            player_x: START_X,
            player_y: START_Y,
            player_dir: START_DIR,
        }
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

    // 한 컬럼(광선 하나)이 맞는 벽까지의 수직(fisheye 보정된) 거리와, 그 벽이
    // 동서(수직 격자선)쪽 면인지 남북(수평 격자선)쪽 면인지를 DDA 로 구한다.
    // side 는 세로띠 음영에 쓴다(같은 벽이라도 면 방향에 따라 살짝 다르게 보이도록).
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
        for _ in 0..256 {
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
                let perp = if side_is_ns { side_dist_y - delta_dist_y } else { side_dist_x - delta_dist_x };
                return (perp.max(0.0001), side_is_ns);
            }
        }
        (MAX_DIST, side_is_ns) // 벽에 안 부딪히면(맵이 뚫려있을 리는 없지만) 안개 끝까지
    }
}

impl App for PacmanApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
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

        // 컬럼(화면 가로 1px)마다 광선 하나 — DDA 로 벽까지 거리를 구해 세로띠로 그린다.
        let num_rays = area.w.round().max(1.0) as usize;
        let col_w = area.w / num_rays as f32;
        for col in 0..num_rays {
            // -1.0(왼쪽 끝) .. 1.0(오른쪽 끝) — 화면 가운데가 플레이어가 보는 방향.
            let camera_x = 2.0 * (col as f32 + 0.5) / num_rays as f32 - 1.0;
            let ray_angle = self.player_dir + camera_x * (FOV / 2.0);
            // cast_ray 가 돌려주는 거리는 DDA 과정에서 이미 카메라 평면에 대한
            // 수직 거리(perpendicular distance)로 나온다 — side_dist 에서
            // delta_dist 를 뺀 값 자체가 fisheye 보정까지 겸하는 Lodev 레이캐스터의
            // 표준 트릭이라, 여기서 각도로 또 한 번 cos 보정을 하면 오히려 가장자리
            // 벽이 과하게 휘어 보이는 이중보정 버그가 된다.
            let (perp_dist, side_is_ns) = self.cast_ray(ray_angle);

            let wall_h = (area.h / perp_dist).min(area.h * 4.0);
            let top = ((area.h - wall_h) / 2.0).clamp(0.0, area.h);
            let bottom = ((area.h + wall_h) / 2.0).clamp(0.0, area.h);

            let fog = (1.0 - (perp_dist / MAX_DIST).clamp(0.0, 1.0) * 0.75).max(0.18);
            let side_shade = if side_is_ns { 0.7 } else { 1.0 };
            let shade = fog * side_shade;
            let color = [0.55 * shade, 0.55 * shade, 0.62 * shade, 1.0];

            let x = area.x + col as f32 * col_w;
            r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), color);
        }

        // 상단 HUD — 지금 먹은 코인 / 맵의 전체 코인.
        let hud = format!("{}/{}", self.coins_collected, self.coins_total);
        r.rect(area.x, area.y, area.w, 20.0, [0.0, 0.0, 0.0, 0.55]);
        r.text(area.x + 8.0, area.y + 4.0, &hud, 0.9, COIN_COLOR);

        AppAction::None
    }
}
