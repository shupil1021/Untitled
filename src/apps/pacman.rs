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
use crate::ui::fill_circle;

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

const COIN_WORLD_DIAMETER: f32 = 0.28; // 코인의 실제 월드 크기(칸 크기=1.0 기준) — 벽 투영과 같은 척도로 원근감을 준다
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
                let dist = if side_is_ns { side_dist_y - delta_dist_y } else { side_dist_x - delta_dist_x };
                return (dist.max(0.0001), side_is_ns);
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

        // 컬럼(화면 가로 1px)마다 광선 하나 — DDA 로 벽까지 거리를 구해 세로띠로
        // 그린다. 나중에 코인(빌보드)을 그릴 때 "이 컬럼에서 벽보다 코인이 더
        // 가까운지" 가려짐 판정을 해야 해서, 컬럼별 깊이를 따로 기억해둔다.
        let num_rays = area.w.round().max(1.0) as usize;
        let col_w = area.w / num_rays as f32;
        let half_fov = FOV / 2.0;
        let mut col_depth = vec![MAX_DIST; num_rays];
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

            let fog = (1.0 - (depth / MAX_DIST).clamp(0.0, 1.0) * 0.75).max(0.18);
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
        // 가린다 — 격자 순서 그대로 그리면 먼 코인이 나중에 그려져 가까운 코인
        // 위로 삐져나와 보이는 문제가 있었다.
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
                let floor_y = area.y + ((area.h + wall_h) / 2.0).clamp(0.0, area.h);
                let screen_x = area.x + (camera_x + 1.0) / 2.0 * area.w;
                // 벽 높이(wall_h = 1 월드유닛 / depth * area.h)와 같은 식으로 코인도
                // "지름 COIN_WORLD_DIAMETER 월드유닛짜리 공"이라고 보고 투영한다.
                // 위쪽 clamp 를 너무 좁게 두면(예전 area.h*0.5) 아주 가까이 다가갔을
                // 때 자라다 말고 그 크기에서 멈춘 것처럼 보인다 — 창 밖으로 자연스럽게
                // 넘쳐서 window_manager 의 클립(area)에 잘리도록 넉넉히 풀어둔다.
                let radius = (COIN_WORLD_DIAMETER / 2.0 * area.h / depth).clamp(1.0, area.h * 4.0);
                let fog = (1.0 - (depth / MAX_DIST).clamp(0.0, 1.0) * 0.75).max(0.18);
                let color = [COIN_COLOR[0] * fog, COIN_COLOR[1] * fog, COIN_COLOR[2] * fog, 1.0];
                visible_coins.push((depth, screen_x, floor_y, radius, color));
            }
        }
        // 먼 것부터(depth 내림차순) 그려서 가까운 코인이 항상 위에 온다.
        visible_coins.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, screen_x, floor_y, radius, color) in visible_coins {
            fill_circle(r, screen_x, floor_y - radius, radius, color);
        }

        // 상단 HUD — 지금 먹은 코인 / 맵의 전체 코인.
        let hud = format!("{}/{}", self.coins_collected, self.coins_total);
        r.rect(area.x, area.y, area.w, 20.0, [0.0, 0.0, 0.0, 0.55]);
        r.text(area.x + 8.0, area.y + 4.0, &hud, 0.9, COIN_COLOR);

        AppAction::None
    }
}
