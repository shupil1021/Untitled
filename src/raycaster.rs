//! 재사용 가능한 그리드 기반 레이캐스팅(2.5D, Wolfenstein 3D 식) 엔진 —
//! `apps/pacman.rs` 를 만들면서 생긴 코드(벽 DDA/이동/충돌/미로 생성/빌보드
//! 렌더링)를 게임 로직(라운드/코인/HUD)과 분리해서 이 모듈로 뽑아냈다. 나중에
//! 미로/복도 구조의 1인칭 3D 미니게임을 더 추가할 때, 이 모듈 하나로 "벽에
//! 안 끼면서 걷고 돌아보기 + 벽 렌더링 + 작은 물체(코인 등) 렌더링 + 미로
//! 생성"을 그대로 재사용할 수 있게 하는 게 목적이다 — 게임마다 다른 부분(맵을
//! 어떻게 채울지, 규칙, HUD, 색)은 이 모듈을 쓰는 쪽에 남겨둔다.
//!
//! `bin/raycaster_test.rs`가 이 모듈만 떼어내 쓰는 가장 작은 예시다 — 라운드도
//! 코인도 없이 미로 하나를 걸어 다니기만 하는 창.

use std::f32::consts::PI;

use miniquad::TextureId;

use crate::gfx::{Rect, Renderer};

// PacmanApp/game_installer.rs 등 여러 곳에서 각자 작게 복사해 쓰던 것과 같은
// 아주 단순한 xorshift64 의사난수 — 여기서는 미로 생성(Raycaster::generate_maze
// 의 방 편입 순서)에 쓴다.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    // 0..n 사이 정수 하나(n==0 이면 0). 진짜 균등분포는 아니지만(모듈로 편향)
    // 미로 생성 정도엔 전혀 티가 안 난다.
    pub fn gen_range(&mut self, n: usize) -> usize {
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
// generate_maze 가 매 반복 이 목록에서 하나씩 골라 쓴다.
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
// 항상 벽). 방(0,0) → 격자(1,1) 이 항상 시작 칸이다.
pub fn generate_maze(rooms: usize, rng: &mut Rng) -> Vec<Vec<bool>> {
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

// generate_maze 가 만드는 "완전미로"는 방 개수-1 개의 벽만 허물어 모든 방을
// 정확히 하나의 경로로만 잇는 스패닝 트리라, 막다른 길(dead end)이 아주 많다
// — 실제로 걸어보면 대부분의 갈림길이 결국 다시 막혀서 되돌아 나와야 하는
// "끊기는" 구조로 느껴진다. 이 함수는 그런 막다른 방마다(이웃 방 중 뚫린 게
// 정확히 하나인 방) 막혀있는 이웃 벽 하나를 chance 확률로 더 허물어서(braiding)
// 루프를 만든다 — 그만큼 경로가 서로 이어지고 되돌아 나올 필요 없이 계속
// "흐르는" 구조가 된다. chance=1.0 이면 막다른 길을 전부 없앤다(이론상
// — 한 번 허문 벽이 다른 막다른 방의 이웃이기도 했다면 그 방도 이 한 번의
// 통과 중에 자연히 같이 풀린다).
//
// generate_maze 가 이미 만든 walls 격자를 그 자리에서 고친다(추가 벽만 허물지,
// 있던 통로를 다시 막지는 않는다) — 그래서 항상 generate_maze 직후에 이어서
// 부른다.
pub fn braid_maze(walls: &mut [Vec<bool>], rooms: usize, rng: &mut Rng, chance: f32) {
    let rooms = rooms.max(1);
    let chance = chance.clamp(0.0, 1.0);
    for ry in 0..rooms {
        for rx in 0..rooms {
            let (gx, gy) = (rx * 2 + 1, ry * 2 + 1);
            let mut open = 0usize;
            let mut closed: Vec<(usize, usize)> = Vec::new(); // 아직 벽인 이웃 방향의 격자 좌표(허물 수 있는 후보)
            if rx > 0 {
                if walls[gy][gx - 1] {
                    closed.push((gx - 1, gy));
                } else {
                    open += 1;
                }
            }
            if rx + 1 < rooms {
                if walls[gy][gx + 1] {
                    closed.push((gx + 1, gy));
                } else {
                    open += 1;
                }
            }
            if ry > 0 {
                if walls[gy - 1][gx] {
                    closed.push((gx, gy - 1));
                } else {
                    open += 1;
                }
            }
            if ry + 1 < rooms {
                if walls[gy + 1][gx] {
                    closed.push((gx, gy + 1));
                } else {
                    open += 1;
                }
            }
            // 뚫린 이웃이 정확히 하나(막다른 길)이고, 허물 수 있는 벽이 남아있을
            // 때만 chance 확률로 하나를 고른다 — gen_range(1000) 을 써서 대략
            // chance 만큼의 확률을 흉내낸다(진짜 균등분포는 아니지만 이 용도엔
            // 충분하다, Rng::gen_range 주석 참고).
            if open == 1 && !closed.is_empty() && rng.gen_range(1000) < (chance * 1000.0) as usize {
                let &(wx, wy) = &closed[rng.gen_range(closed.len())];
                walls[wy][wx] = false;
            }
        }
    }
}

// 바닥에 놓인 완전히 둥근 물체(코인 등) 하나 — 항상 카메라를 향하는 평면
// (빌보드)으로 그린다. world_diameter 는 칸 크기(=1.0)를 기준으로 한 실제
// 지름이다. 둥근 물체는 어느 각도에서 봐도 실루엣이 원이라 빌보드로 그려도
// 티가 안 나지만, 책상/의자처럼 각진 물체는 옆에서 보면 납작해 보이는 게
// 뻔히 드러나서 그런 물체는 대신 Prop3D(진짜 입체, 아래 참고)를 쓴다.
pub struct Billboard {
    pub x: f32,
    pub y: f32,
    pub world_diameter: f32,
    pub color: [f32; 4],
}

// 카메라(플레이어) 눈높이 — 칸 크기(=1.0 = 바닥~천장)를 기준으로 정중앙. 벽이
// z=0(바닥)~1(천장) 전체를 항상 가득 채우는 것과 달리, Prop3D 의 위/아래 면은
// 이 값보다 위/아래에 있어야만 실제로 보인다(render_props 의 백페이스 컬링
// 판정 참고) — 그 외의 z-화면좌표 변환 공식(render_walls 등)에도 전부 이
// 기준으로 눈높이가 화면 정중앙에 오도록 깔려있다.
const EYE_HEIGHT: f32 = 0.5;

// 그리드 미로 하나 + 그 안을 돌아다니는 플레이어(위치/바라보는 각도) — 벽
// DDA 레이캐스팅, 충돌 포함 이동, 벽/빌보드 렌더링을 전부 여기서 담당한다.
pub struct Raycaster {
    pub walls: Vec<Vec<bool>>, // walls[y][x] — true 면 벽
    pub player_x: f32,
    pub player_y: f32,
    pub player_dir: f32, // 라디안, 0 = +x(동쪽)을 바라봄
    pub fog_dist: f32,   // 이보다 먼 벽은 완전히 어둡게(안개) — 보통 맵 대각선의 1.5배 정도
}

impl Raycaster {
    pub fn new(walls: Vec<Vec<bool>>, start_x: f32, start_y: f32) -> Raycaster {
        let fog_dist = walls.len() as f32 * 1.5;
        Raycaster { walls, player_x: start_x, player_y: start_y, player_dir: 0.0, fog_dist }
    }

    // 맵 밖이거나 벽이면 true(레이캐스팅/충돌 판정 둘 다 이 하나로 처리 — 밖은
    // 항상 벽 취급해서 광선이 격자 밖으로 나가 인덱스 범위를 벗어나는 일이 없다).
    pub fn is_wall(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return true;
        }
        let (mx, my) = (x as usize, y as usize);
        my >= self.walls.len() || mx >= self.walls[0].len() || self.walls[my][mx]
    }

    // 축을 따로 검사해서 벽을 따라 미끄러지듯 이동한다(둘 다 막혀있지 않은 이상
    // 완전히 멈추지 않는다) — radius 만큼은 벽에서 떨어져 있도록 이동 방향
    // 쪽으로 살짝 더 나간 지점도 같이 비어있는지 본다.
    pub fn try_move(&mut self, dx: f32, dy: f32, radius: f32) {
        if dx != 0.0 {
            let nx = self.player_x + dx;
            if !self.is_wall(nx + radius * dx.signum(), self.player_y) {
                self.player_x = nx;
            }
        }
        if dy != 0.0 {
            let ny = self.player_y + dy;
            if !self.is_wall(self.player_x, ny + radius * dy.signum()) {
                self.player_y = ny;
            }
        }
    }

    // W(전진)/S(후진)/A(좌회전)/D(우회전) 식 입력을 그대로 이동/회전에 반영한다
    // — 실제 키 상태를 어떻게 읽어올지(WinInput 이든, miniquad 를 직접 쓰는
    // 테스트 창이든)는 호출부가 정해서 bool 네 개로 넘겨주면 된다.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_wasd(&mut self, forward: bool, backward: bool, turn_left: bool, turn_right: bool, move_speed: f32, rot_speed: f32, radius: f32, dt: f32) {
        if turn_left {
            self.player_dir -= rot_speed * dt;
        }
        if turn_right {
            self.player_dir += rot_speed * dt;
        }
        let (fx, fy) = (self.player_dir.cos(), self.player_dir.sin());
        let mut dx = 0.0;
        let mut dy = 0.0;
        if forward {
            dx += fx * move_speed * dt;
            dy += fy * move_speed * dt;
        }
        if backward {
            dx -= fx * move_speed * dt;
            dy -= fy * move_speed * dt;
        }
        if dx != 0.0 || dy != 0.0 {
            self.try_move(dx, dy, radius);
        }
    }

    // cell 에서 실제로 뚫려있는(벽이 아닌) 방향을 동/남/서/북 순서로 찾아
    // 돌려준다 — 미로 생성 결과에 따라 시작 칸의 특정 방향이 막혀있을 수도
    // 있어서, "무조건 동쪽을 보고 시작"하면 벽을 마주보고 시작할 수 있다.
    // 전부 막혀있으면(방이 하나뿐인 극단적인 경우) 그냥 동쪽을 기본값으로 쓴다.
    pub fn face_open_direction(&self, cell: (usize, usize)) -> f32 {
        let (sx, sy) = (cell.0 as f32, cell.1 as f32);
        const CANDIDATES: [f32; 4] = [0.0, PI / 2.0, PI, -PI / 2.0]; // 동, 남, 서, 북
        for &dir in &CANDIDATES {
            let (dx, dy) = (dir.cos().round(), dir.sin().round());
            if !self.is_wall(sx + dx, sy + dy) {
                return dir;
            }
        }
        0.0
    }

    // 한 컬럼(광선 하나)이 맞는 벽까지의 "유클리드" 거리(플레이어 ~ 충돌점 직선
    // 거리)와, 그 벽이 동서(수직 격자선)쪽 면인지 남북(수평 격자선)쪽 면인지를
    // DDA 로 구한다. ray_dir 을 (cos, sin) 단위벡터로 만들었기 때문에(정규화된
    // 벡터) side_dist - delta_dist 는 Lodev 식 카메라-평면 레이캐스터에서처럼
    // 자동으로 fisheye 가 보정된 "수직 거리"가 아니라 진짜 유클리드 거리로 나온다
    // — 그래서 render_walls/render_billboards 가 이 값에 cos(광선각 - 플레이어각)를
    // 곱해 직접 수직 거리로 보정한다. side 는 세로띠 음영에 쓴다.
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

    // 천장/바닥을 채우고 그 위에 컬럼(화면 가로 1px)마다 광선 하나씩 DDA 로 벽
    // 세로띠를 그린다. 반환하는 컬럼별 깊이(fisheye 보정된 수직 거리)는
    // render_billboards() 가 "이 컬럼에서 벽보다 물체가 더 가까운지" 가려짐
    // 판정에 그대로 재사용한다.
    #[allow(clippy::too_many_arguments)]
    pub fn render_walls(&self, r: &mut Renderer, area: Rect, fov: f32, ceiling: [f32; 4], floor: [f32; 4], wall_base: [f32; 4]) -> Vec<f32> {
        let half = area.h / 2.0;
        r.rect(area.x, area.y, area.w, half, ceiling);
        r.rect(area.x, area.y + half, area.w, area.h - half, floor);

        let num_rays = area.w.round().max(1.0) as usize;
        let col_w = area.w / num_rays as f32;
        let half_fov = fov / 2.0;
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
            let color = [wall_base[0] * shade, wall_base[1] * shade, wall_base[2] * shade, wall_base[3]];

            let x = area.x + col as f32 * col_w;
            r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), color);
        }
        col_depth
    }

    // billboards 를 바닥에 놓인 작은 원으로 그린다. 시야각 안에 들고, 그 각도가
    // 가리키는 컬럼에서 (벽이든 render_props() 로 이미 그려둔 물체든) col_depth
    // 보다 가까울 때만 그린다. 깊이 버퍼가 따로 없는 빌보드라 먼 것부터(화가
    // 알고리즘) 그려야 한 복도 안에 여러 개가 늘어서 있을 때 가까운(큰) 것이
    // 먼(작은) 것을 제대로 가린다.
    pub fn render_billboards(&self, r: &mut Renderer, area: Rect, fov: f32, col_depth: &[f32], billboards: &[Billboard]) {
        let num_rays = col_depth.len();
        let col_w = area.w / num_rays as f32;
        let half_fov = fov / 2.0;
        let mut visible: Vec<(f32, f32, f32, f32, [f32; 4])> = Vec::new(); // (depth, screen_x, floor_y, radius, color)
        for b in billboards {
            let rel_x = b.x - self.player_x;
            let rel_y = b.y - self.player_y;
            let dist = rel_x.hypot(rel_y);
            if dist < 0.1 {
                continue; // 바로 발밑 — 호출부가 먹었어야 할 물체
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
            let col = col.min(num_rays.saturating_sub(1));
            if col_depth.get(col).is_some_and(|&d| depth >= d) {
                continue; // 중심 컬럼부터 이미 가려짐 — 폭 전체를 컬럼별로 자르는 아래
                          // 그리기 단계에서 한 번 더 정확히 판정하니, 여기선 "아예 안
                          // 보이는" 경우만 미리 걸러서 정렬 목록을 줄인다.
            }

            let wall_h = (area.h / depth).min(area.h * 4.0);
            // 벽 렌더링 쪽 top/bottom 은 r.rect 에 그대로 넘길 y/height 라서 화면
            // 안으로 clamp 가 필요하지만, 여기 floor_y 는 기준점일 뿐이다 — 이것까지
            // clamp 하면 아주 가까이 다가갔을 때 화면 끝에 고정된 채 반지름만 커져서
            // 물체가 위로 떠오르는 것처럼 보인다.
            let floor_y = area.y + (area.h + wall_h) / 2.0;
            let screen_x = area.x + (camera_x + 1.0) / 2.0 * area.w;
            let radius = (b.world_diameter / 2.0 * area.h / depth).clamp(1.0, area.h * 4.0);
            let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
            let color = [b.color[0] * fog, b.color[1] * fog, b.color[2] * fog, 1.0];
            visible.push((depth, screen_x, floor_y, radius, color));
        }
        // 먼 것부터(depth 내림차순) 그려서 가까운 물체가 항상 위에 온다.
        visible.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (depth, screen_x, floor_y, radius, color) in visible {
            // fill_circle 로 한 번에 그리면(가로줄 단위) "이 원이 벽보다 가까운지"를
            // 중심 컬럼 하나로만 판정한 게 전부라, 원의 폭이 넓을 때(가까이 다가갔을
            // 때) 중심 컬럼은 벽 뒤가 아니어도 양 옆 일부가 실제로는 벽/모서리 너머
            // (다른 복도)에 있는 코인이 그 벽 앞으로 삐져나와 보이는 문제가 있었다.
            // 벽/Prop3D 와 똑같이 컬럼별로 잘라 그리면서 컬럼마다 col_depth 와
            // 비교해야, 코인의 어느 부분이 실제로 벽에 가려지는지 정확히 반영된다.
            let center_y = floor_y - radius;
            let min_col = (((screen_x - radius - area.x) / col_w).floor().max(0.0)) as usize;
            let max_col = (((screen_x + radius - area.x) / col_w).ceil().max(0.0)) as usize;
            let max_col = max_col.min(num_rays.saturating_sub(1));
            if min_col > max_col {
                continue;
            }
            for (col, &d) in col_depth.iter().enumerate().take(max_col + 1).skip(min_col) {
                if depth >= d {
                    continue; // 이 컬럼에서는 벽/다른 물체가 이 코인보다 가깝다
                }
                let col_center_x = area.x + (col as f32 + 0.5) * col_w;
                let dx = col_center_x - screen_x;
                if dx.abs() > radius {
                    continue;
                }
                let half_h = (radius * radius - dx * dx).max(0.0).sqrt();
                let x = area.x + col as f32 * col_w;
                r.rect(x, center_y - half_h, col_w + 0.6, half_h * 2.0, color);
            }
        }
    }

    // 어디선가 광선이 이 평면(plane_coord)에 부딪히는지 본다 — is_x_plane 이면
    // x=plane_coord 인 수직 평면(허용 범위는 y ∈ [span_min,span_max]), 아니면
    // y=plane_coord 인 평면(범위는 x ∈ [span_min,span_max]). Prop3D 의 한 "면"을
    // cast_ray 의 격자 DDA 대신 단일 평면 교차로 다루는 render_props() 전용
    // 헬퍼 — 돌려주는 (t, hit_secondary) 에서 t 는 cast_ray 와 마찬가지로
    // 단위벡터 기준 유클리드 거리, hit_secondary 는 그 면을 따라 어디에
    // 맞았는지(span_min~span_max 사이 실제 좌표) — 텍스처를 입힐 때 그 지점의
    // U 좌표를 구하는 데 쓴다.
    fn intersect_plane(&self, ray_angle: f32, is_x_plane: bool, plane_coord: f32, span_min: f32, span_max: f32) -> Option<(f32, f32)> {
        let (dir_primary, dir_secondary, pos_primary, pos_secondary) = if is_x_plane {
            (ray_angle.cos(), ray_angle.sin(), self.player_x, self.player_y)
        } else {
            (ray_angle.sin(), ray_angle.cos(), self.player_y, self.player_x)
        };
        if dir_primary.abs() < 1e-6 {
            return None;
        }
        let t = (plane_coord - pos_primary) / dir_primary;
        if t <= 0.0 {
            return None;
        }
        let hit_secondary = pos_secondary + dir_secondary * t;
        if hit_secondary < span_min || hit_secondary > span_max {
            return None;
        }
        Some((t, hit_secondary))
    }

    // props(책상/의자 등 각진 상자)를 진짜 입체로 그린다 — 빌보드처럼 항상
    // 카메라를 향하는 평면이 아니라, 플레이어 위치를 기준으로 실제로 보이는
    // 면(옆면은 동/서/남/북 중 플레이어가 바깥쪽에 있는 면들, 위/아래 면은
    // 항상 시도)만 골라서 그린다. 옆면은 벽과 똑같이 광선-평면 교차로, 위/아래
    // 면은 광선이 상자 발자국(바닥 사각형)을 지나는 깊이 구간(AABB 슬래브
    // 교차)으로 구한다 — 둘 다 벽처럼 "컬럼마다 광선을 다시 쏘는" 방식이라
    // 정확하다(render_horizontal_face 주석 참고 — 처음엔 꼭짓점을 화면에
    // 투영해서 화면-공간에서 보간했는데, 이 엔진의 가로 투영이 각도-선형이라
    // 그 보간이 실제 3D 위치와 어긋나 윗면이 비거나 엉뚱하게 어두웠다). 그래서
    // 옆으로 돌아가면 실제로 옆면이, 눈높이(0.5)보다 낮은 물체는 내려다본
    // 윗면이 보인다.
    //
    // props 는 먼 것부터(화가 알고리즘) 그린다 — 안 그러면 가까운(짧은) 물체를
    // 먼저 그렸을 때, 그 뒤에 더 큰(키가 큰) 물체가 있어도 "이 컬럼엔 이미
    // 뭔가 있다"는 col_depth 검사 하나에 걸려 위로 삐져나와 보여야 할 부분까지
    // 통째로 안 그려지는 문제가 있었다 — 컬럼 하나에 깊이 값이 하나뿐이라
    // "세로로 일부만 가려진" 상황 자체를 표현 못 해서, 그리는 순서로 바로잡는다.
    //
    // col_depth 를 직접 갱신해서(그려진 컬럼만) 벽/다른 물체가 그 컬럼에서 이
    // 물체보다 가까우면 안 그려지고, 이 물체가 그려진 자리는 그 뒤
    // (render_billboards 등)에서도 올바르게 가려지게 한다 — 그래서 render_walls
    // 직후, render_billboards 이전에 호출해야 한다.
    pub fn render_props(&self, r: &mut Renderer, area: Rect, fov: f32, col_depth: &mut [f32], props: &[Prop3D]) {
        let num_rays = col_depth.len();
        let col_w = area.w / num_rays as f32;
        let half_fov = fov / 2.0;

        let mut order: Vec<&Prop3D> = props.iter().collect();
        order.sort_by(|a, b| {
            let da = (a.center_x - self.player_x).hypot(a.center_y - self.player_y);
            let db = (b.center_x - self.player_x).hypot(b.center_y - self.player_y);
            db.total_cmp(&da)
        });

        for p in order {
            let (min_x, max_x) = (p.center_x - p.width / 2.0, p.center_x + p.width / 2.0);
            let (min_y, max_y) = (p.center_y - p.depth / 2.0, p.center_y + p.depth / 2.0);
            let (z0, z1) = (p.base, p.base + p.height);

            // 플레이어가 상자 바깥쪽에 있는 면만 실제로 보인다 — 안에 있으면(그
            // 축 범위 안이면) 그 방향 두 면 다 안 보인다.
            let mut side_faces: Vec<(NetFace, bool, f32, f32, f32, f32)> = Vec::new(); // (면, x축 평면?, 평면 좌표, span_min, span_max, 음영)
            if self.player_x < min_x {
                side_faces.push((NetFace::West, true, min_x, min_y, max_y, 0.75));
            }
            if self.player_x > max_x {
                side_faces.push((NetFace::East, true, max_x, min_y, max_y, 0.75));
            }
            if self.player_y < min_y {
                side_faces.push((NetFace::North, false, min_y, min_x, max_x, 1.0));
            }
            if self.player_y > max_y {
                side_faces.push((NetFace::South, false, max_y, min_x, max_x, 1.0));
            }
            if side_faces.is_empty() {
                continue; // 플레이어가 상자 안에 들어와 있다(충돌 처리를 안 했다면) — 안팎이 뒤집혀 보일 수 있으니 그냥 생략
            }

            // 이 prop 을 그리기 시작하기 "직전"의 깊이(벽 + 이미 그려진 더 먼
            // 다른 prop들) 스냅샷 — 옆면/윗면/아랫면 세 패스 모두 가려짐 판정은
            // 이 스냅샷 하나만 기준으로 한다. 옆면(예: z 0~0.4 인 낮은 앞벽)과
            // 윗면(그 앞벽 "너머"의, z=0.4 그대로 인 채 더 먼 곳으로 이어지는
            // 지붕)은 같은 컬럼이라도 화면상 서로 다른(겹치지 않는) 세로 구간을
            // 차지하는데, 옆면을 먼저 그리며 col_depth 를 갱신해버리면 뒤이어
            // 계산하는 윗면이 "그 컬럼엔 이미 이 상자 자신이 있다"는 이유로
            // 통째로 안 그려졌다 — 그래서 위가 뻥 뚫려 보이고, 그 틈으로 원래
            // col_depth 에 있던 것(벽 등)이 계속 비쳐 보였다. 세 패스 다 이
            // 스냅샷을 기준으로 독립적으로 판정하고, 실제 col_depth 갱신은 그
            // 컬럼에서 이 prop 이 그린 것 중 가장 가까운 값으로만 한다 — 그래야
            // 나중에(더 가까이 정렬된) 다른 prop 들에게는 정확히 가려진다.
            let base_depth: Vec<f32> = col_depth.to_vec();

            for col in 0..num_rays {
                let camera_x = 2.0 * (col as f32 + 0.5) / num_rays as f32 - 1.0;
                let rel_angle = camera_x * half_fov;
                let ray_angle = self.player_dir + rel_angle;

                // 이 컬럼에서 여러 면에 동시에 맞을 수도 있다(모서리 근처) — 그중
                // 가장 가까운 것만 쓴다. hfrac 은 그 면을 따라 어디에 맞았는지를
                // 0..1 로 정규화한 것 — 텍스처가 있으면 U 좌표를 구하는 데 쓴다.
                let mut nearest: Option<(f32, f32, NetFace, f32)> = None; // (depth, 음영, 면, hfrac)
                for &(face, is_x_plane, coord, span_min, span_max, shade) in &side_faces {
                    if let Some((t, hit_secondary)) = self.intersect_plane(ray_angle, is_x_plane, coord, span_min, span_max) {
                        let depth = (t * rel_angle.cos()).max(0.0001);
                        if nearest.is_none_or(|(d, ..)| depth < d) {
                            let hfrac = (hit_secondary - span_min) / (span_max - span_min).max(1e-6);
                            nearest = Some((depth, shade, face, hfrac));
                        }
                    }
                }
                let Some((depth, side_shade, face, hfrac)) = nearest else { continue };
                if depth >= base_depth[col] {
                    continue; // 벽이든 앞서 그려진 다른(더 먼) 물체든 이미 이보다 가까운 게 있다
                }

                // render_walls() 의 top/bottom 계산과 정확히 같은 식(상대좌표로
                // 구한 뒤 area.y 를 한 번만 더한다) — z=0.5(눈높이)가 화면
                // 정중앙, z=0(바닥)/z=1(천장)이 벽 렌더링과 똑같은 위치에 오도록
                // z0/z1 을 그대로 대입한다.
                let y_for_z = |z: f32| area.h / 2.0 - (z - 0.5) * (area.h / depth);
                let top = y_for_z(z1).clamp(0.0, area.h);
                let bottom = y_for_z(z0).clamp(0.0, area.h);

                let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
                let x = area.x + col as f32 * col_w;
                match &p.texture {
                    // 컬럼 하나는 폭이 1px 남짓이라 U 폭도 사실상 0에 가깝다 —
                    // 이 컬럼이 실제로 맞은 hfrac 딱 한 지점만 샘플링한다(그
                    // 지점 값을 u0/u1 둘 다에 써서 사실상 세로선 하나를 그대로
                    // 오려온다). 컬럼마다 hfrac 이 정확히 다시 계산되므로, 옆으로
                    // 훑으며 이어붙이면 결국 텍스처 전체가 정확히 펼쳐진다 —
                    // 울펜슈타인 3D 식 텍스처 매핑의 표준 방식.
                    Some(tex) => {
                        let (u0, v0, u1, v1) = tex.uv_for(face);
                        let u = u0 + hfrac.clamp(0.0, 1.0) * (u1 - u0);
                        let tint = [fog * side_shade, fog * side_shade, fog * side_shade, 1.0];
                        r.sprite_uv(tex.texture, x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), u, v0, u, v1, tint);
                    }
                    None => {
                        let shade = fog * side_shade;
                        let color = [p.color[0] * shade, p.color[1] * shade, p.color[2] * shade, p.color[3]];
                        r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), color);
                    }
                }
                if depth < col_depth[col] {
                    col_depth[col] = depth;
                }
            }

            // 위/아래 면 — 광선-평면 교차로는 못 구해서(우리 광선엔 z 가 없다)
            // 광선이 상자 발자국을 지나는 깊이 구간으로 채운다
            // (render_horizontal_face 참고). 옆면과 마찬가지로 base_depth 스냅샷
            // 으로 판정해서, 방금 그린 이 prop 자신의 옆면이 위/아랫면을 가로막지
            // 않게 한다.
            //
            // 눈높이(EYE_HEIGHT) 보다 위에 있는 면은 실제로 못 본다 — 이건
            // 백페이스 컬링과 같은 이유다: 윗면(법선이 +z, 위쪽을 향함)은
            // 카메라가 그 면보다 "위"(z1 < 눈높이)에 있을 때만 보이고, 아랫면
            // (법선이 -z)은 카메라가 그 면보다 "아래"(z0 > 눈높이)에 있을 때만
            // 보인다. 이 확인이 빠져있어서, 바닥(z0=0)에 딱 붙어있는 보통 물체도
            // (눈높이가 항상 바닥보다 높으니 아랫면은 절대 안 보여야 하는데) 그
            // 밑면이 마치 물체 앞에 붕 떠서 카메라를 향하고 있는 것처럼 그려지고
            // 있었다.
            let top_color = [p.color[0], p.color[1], p.color[2], p.color[3]]; // 옆면(최대 1.0)보다 밝게 — 위에서 빛을 더 받는 느낌
            let bottom_color = [p.color[0] * 0.5, p.color[1] * 0.5, p.color[2] * 0.5, p.color[3]]; // 가장 어둡게
            if EYE_HEIGHT > z1 {
                self.render_horizontal_face(
                    r, area, fov, col_w, &base_depth, col_depth, min_x, max_x, min_y, max_y, z1, top_color, p.texture.as_ref().map(|t| (t, NetFace::Top)),
                );
            }
            if EYE_HEIGHT < z0 {
                self.render_horizontal_face(
                    r, area, fov, col_w, &base_depth, col_depth, min_x, max_x, min_y, max_y, z0, bottom_color,
                    p.texture.as_ref().map(|t| (t, NetFace::Bottom)),
                );
            }
        }
    }

    // 수평 평면(z=const) 하나 — 상자의 윗면/아랫면 — 을 화면에 채운다.
    //
    // 처음엔 네 꼭짓점을 화면에 각각 투영한 뒤 컬럼별로 화면-공간에서 선형보간
    // (스캔라인)하는 방식으로 짰는데, 이 엔진의 가로 투영이 진짜 원근(탄젠트
    // 기반) 이 아니라 "각도에 선형" 이라(카메라 평면 대신 각도로 컬럼을 나눈다
    // — cast_ray 의 fisheye 보정 주석 참고) 화면-공간에서 두 꼭짓점 사이를 선형
    // 보간하면 실제 3D 위치와 어긋난다 — 가까이서 보면 윗면이 비거나(각도 폭이
    // 넓어서 두 꼭짓점 사이 보간이 실제 사각형보다 훨씬 좁아짐) 엉뚱하게
    // 어두워 보였다(보간된 깊이가 실제보다 훨씬 커짐).
    //
    // 그래서 대신 벽/옆면과 똑같이 "이 컬럼의 광선이 실제로 어디를 지나는지"를
    // 컬럼마다 다시 계산한다 — 광선(2D, z 없음)이 상자의 바닥 발자국
    // [min_x,max_x]×[min_y,max_y] 을 통과하는 구간(t_near..t_far, 표준
    // AABB-레이 슬래브 교차)을 구하면, 그 구간의 가까운/먼 끝이 곧 이 컬럼에서
    // z=z 평면이 보이는 깊이 범위다(z=z 자체와는 절대 안 만나지만, "이 컬럼이
    // 상자 발자국 위를 지나는 동안"이 곧 "그 z 평면이 보이는 동안"과 같다).
    // texture 가 Some 이면(면 정보도 같이) color 대신 그 전개도 칸으로 채운다 —
    // 다만 옆면과 달리 여기는 "컬럼 하나 = 한 지점"이 아니라 "컬럼 하나 = 상자
    // 발자국을 지나는 구간 전체(t_near..t_far)"라 화면-공간 보간 없이 정확한
    // 2차원(U,V) 그라데이션을 넣으려면 컬럼을 더 잘게 쪼개야 한다 — 지금은 그
    // 구간의 중점 한 지점만 샘플링해서 컬럼당 단색 타일처럼 칠한다(옆면만큼
    // 세밀하진 않지만, 전개도가 실제로 올바른 면·자리에 입혀지는지 확인하기엔
    // 충분하다).
    #[allow(clippy::too_many_arguments)]
    fn render_horizontal_face(
        &self, r: &mut Renderer, area: Rect, fov: f32, col_w: f32, base_depth: &[f32], col_depth: &mut [f32], min_x: f32, max_x: f32,
        min_y: f32, max_y: f32, z: f32, color: [f32; 4], texture: Option<(&PropTexture, NetFace)>,
    ) {
        let num_rays = col_depth.len();
        let half_fov = fov / 2.0;

        for col in 0..num_rays {
            let camera_x = 2.0 * (col as f32 + 0.5) / num_rays as f32 - 1.0;
            let rel_angle = camera_x * half_fov;
            let ray_angle = self.player_dir + rel_angle;
            let dir_x = ray_angle.cos();
            let dir_y = ray_angle.sin();

            // 표준 슬래브(slab) 교차 — 각 축에서 [min,max] 범위에 들어가는
            // t 구간을 구해 교집합을 취한다. 광선이 그 축과 거의 평행하면
            // (dir≈0) 그 축은 아예 제한을 안 거는 것으로 친다(±무한대) —
            // 플레이어가 이미 그 축 범위 밖에 있으면 어차피 다른 축에서 걸러진다.
            let (tx0, tx1) = if dir_x.abs() < 1e-6 {
                (f32::NEG_INFINITY, f32::INFINITY)
            } else {
                let a = (min_x - self.player_x) / dir_x;
                let b = (max_x - self.player_x) / dir_x;
                (a.min(b), a.max(b))
            };
            let (ty0, ty1) = if dir_y.abs() < 1e-6 {
                (f32::NEG_INFINITY, f32::INFINITY)
            } else {
                let a = (min_y - self.player_y) / dir_y;
                let b = (max_y - self.player_y) / dir_y;
                (a.min(b), a.max(b))
            };
            let t_near = tx0.max(ty0).max(0.0001);
            let t_far = tx1.min(ty1);
            if t_near >= t_far {
                continue; // 이 컬럼의 광선은 상자 발자국을 아예 안 지난다
            }

            let cos_correction = rel_angle.cos(); // cast_ray 와 같은 fisheye 보정
            let depth_near = (t_near * cos_correction).max(0.0001);
            let depth_far = (t_far * cos_correction).max(0.0001);
            if depth_near >= base_depth[col] {
                continue; // 벽이든 앞서 그려진 다른(더 먼) 물체든 이미 이보다 가까운 게 있다
            }

            let y_for_z = |depth: f32| area.h / 2.0 - (z - 0.5) * (area.h / depth);
            let y_near = y_for_z(depth_near);
            let y_far = y_for_z(depth_far);
            let top = y_near.min(y_far).clamp(0.0, area.h);
            let bottom = y_near.max(y_far).clamp(0.0, area.h);

            let fog = (1.0 - (depth_near / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
            let x = area.x + col as f32 * col_w;
            match texture {
                Some((tex, face)) => {
                    let t_mid = (t_near + t_far) / 2.0;
                    let hit_x = ((self.player_x + ray_angle.cos() * t_mid - min_x) / (max_x - min_x).max(1e-6)).clamp(0.0, 1.0);
                    let hit_y = ((self.player_y + ray_angle.sin() * t_mid - min_y) / (max_y - min_y).max(1e-6)).clamp(0.0, 1.0);
                    let (u0, v0, u1, v1) = tex.uv_for(face);
                    let u = u0 + hit_x * (u1 - u0);
                    let v = v0 + hit_y * (v1 - v0);
                    let tint = [fog, fog, fog, 1.0];
                    r.sprite_uv(tex.texture, x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), u, v, u, v, tint);
                }
                None => {
                    let c = [color[0] * fog, color[1] * fog, color[2] * fog, color[3]];
                    r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), c);
                }
            }
            if depth_near < col_depth[col] {
                col_depth[col] = depth_near;
            }
        }
    }
}

// Prop3D 의 여섯 면 중 하나 — PropTexture::uv_for() 가 "전개도"(net) 이미지
// 안에서 그 면에 해당하는 칸을 찾는 데 쓴다.
#[derive(Clone, Copy, PartialEq, Eq)]
enum NetFace {
    Top,
    Bottom,
    North, // y 가 작은 쪽 면(min_y)
    South, // y 가 큰 쪽 면(max_y)
    West,  // x 가 작은 쪽 면(min_x)
    East,  // x 가 큰 쪽 면(max_x)
}

// Prop3D 의 여섯 면에 입힐 "전개도"(net) 텍스처 — 이미지 하나를 가로 4칸×세로
// 3칸으로 나눈 표준 십자형 레이아웃을 쓴다(수학 교과서의 정육면체 전개도와
// 같은 배치):
//
// ```
//       [ 윗면 ]
// [서][ 북 ][동][ 남 ]
//       [아랫면]
// ```
//
// (서=West/동=East 는 x, 북=North/남=South 는 y 기준 — 바로 위에서 내려다본
// 평면도라고 생각하면 된다.) 안 쓰는 나머지 4칸(첫 줄·끝 줄의 좌우, 즉
// (0,0)(2,0)(3,0)(0,2)(2,2)(3,2))은 그냥 비워둬도 상관없다.
pub struct PropTexture {
    pub texture: TextureId,
}

impl PropTexture {
    fn uv_for(&self, face: NetFace) -> (f32, f32, f32, f32) {
        let (col, row) = match face {
            NetFace::Top => (1, 0),
            NetFace::West => (0, 1),
            NetFace::North => (1, 1),
            NetFace::East => (2, 1),
            NetFace::South => (3, 1),
            NetFace::Bottom => (1, 2),
        };
        let (cw, ch) = (1.0 / 4.0, 1.0 / 3.0);
        (col as f32 * cw, row as f32 * ch, (col + 1) as f32 * cw, (row + 1) as f32 * ch)
    }
}

// 진짜 입체(축 정렬 상자)로 그리는 물체 — 책상/의자처럼 각져서 옆에서 봤을 때도
// 실제로 옆면이 보여야 하는 것에 쓴다(둥근 물체는 Billboard 로 충분하다).
// center_x/center_y 는 바닥 위 발밑 중심, width(x축)/depth(y축)/height 는 전부
// 칸 크기(=1.0)를 기준으로 한 실제 치수, base 는 바닥에서 밑면까지 띄운 높이
// (의자 등받이처럼 좌판 위에 얹힌 부분에 쓴다, 그 외엔 보통 0.0). texture 가
// None 이면 지금까지처럼 color 하나로 칠하고, Some 이면 색은 안 쓰고(단, 조명
// 처리 — 거리 안개/면별 음영 — 는 그대로 텍스처에 곱해진다) 그 전개도
// 텍스처로 각 면을 그린다.
pub struct Prop3D {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub base: f32,
    pub color: [f32; 4],
    pub texture: Option<PropTexture>,
}
