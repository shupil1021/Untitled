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

use crate::gfx::{Rect, Renderer};
use crate::ui::fill_circle;

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
                continue; // 가려짐(폭이 있는 물체도 중심 컬럼 하나만으로 판정하는 근사치)
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
        for (_, screen_x, floor_y, radius, color) in visible {
            fill_circle(r, screen_x, floor_y - radius, radius, color);
        }
    }

    // 어디선가 광선이 이 평면(plane_coord)에 부딪히는지 본다 — is_x_plane 이면
    // x=plane_coord 인 수직 평면(허용 범위는 y ∈ [span_min,span_max]), 아니면
    // y=plane_coord 인 평면(범위는 x ∈ [span_min,span_max]). Prop3D 의 한 "면"을
    // cast_ray 의 격자 DDA 대신 단일 평면 교차로 다루는 render_props() 전용
    // 헬퍼 — 돌려주는 t 는 cast_ray 와 마찬가지로 단위벡터 기준 유클리드 거리다.
    fn intersect_plane(&self, ray_angle: f32, is_x_plane: bool, plane_coord: f32, span_min: f32, span_max: f32) -> Option<f32> {
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
        Some(t)
    }

    // props(책상/의자 등 각진 상자)를 진짜 입체로 그린다 — 빌보드처럼 항상
    // 카메라를 향하는 평면이 아니라, 플레이어 위치를 기준으로 실제로 보이는
    // 면(옆면은 동/서/남/북 중 플레이어가 바깥쪽에 있는 면들, 위/아래 면은
    // 항상 시도)만 골라서 그린다. 옆면은 벽과 똑같은 컬럼별 레이-평면 교차라
    // 정확하고, 위/아래 면은 우리 광선(z 없이 xy 평면 위에서만 움직인다)으로는
    // 절대 못 만나는 수평 평면이라 대신 꼭짓점 4개를 화면에 투영해서 컬럼별로
    // 채우는 스캔라인(render_horizontal_face)을 쓴다 — 그래서 옆으로 돌아가면
    // 실제로 옆면이, 눈높이(0.5)보다 낮은 물체는 내려다본 윗면이 보인다(둘 다
    // 그리기 전엔 그 자리가 그냥 비어 보여서 뒤가 훤히 비쳐 보였다).
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
            let mut side_faces: Vec<(bool, f32, f32, f32, f32)> = Vec::new(); // (x축 평면?, 평면 좌표, span_min, span_max, 음영)
            if self.player_x < min_x {
                side_faces.push((true, min_x, min_y, max_y, 0.75));
            }
            if self.player_x > max_x {
                side_faces.push((true, max_x, min_y, max_y, 0.75));
            }
            if self.player_y < min_y {
                side_faces.push((false, min_y, min_x, max_x, 1.0));
            }
            if self.player_y > max_y {
                side_faces.push((false, max_y, min_x, max_x, 1.0));
            }
            if side_faces.is_empty() {
                continue; // 플레이어가 상자 안에 들어와 있다(충돌 처리를 안 했다면) — 안팎이 뒤집혀 보일 수 있으니 그냥 생략
            }

            for (col, depth_slot) in col_depth.iter_mut().enumerate() {
                let camera_x = 2.0 * (col as f32 + 0.5) / num_rays as f32 - 1.0;
                let rel_angle = camera_x * half_fov;
                let ray_angle = self.player_dir + rel_angle;

                // 이 컬럼에서 여러 면에 동시에 맞을 수도 있다(모서리 근처) — 그중
                // 가장 가까운 것만 쓴다.
                let mut nearest: Option<(f32, f32)> = None; // (depth, 음영)
                for &(is_x_plane, coord, span_min, span_max, shade) in &side_faces {
                    if let Some(t) = self.intersect_plane(ray_angle, is_x_plane, coord, span_min, span_max) {
                        let depth = (t * rel_angle.cos()).max(0.0001);
                        if nearest.is_none_or(|(d, _)| depth < d) {
                            nearest = Some((depth, shade));
                        }
                    }
                }
                let Some((depth, side_shade)) = nearest else { continue };
                if depth >= *depth_slot {
                    continue; // 벽이든 앞서 그려진 다른 물체든 이미 이보다 가까운 게 있다
                }

                // render_walls() 의 top/bottom 계산과 정확히 같은 식(상대좌표로
                // 구한 뒤 area.y 를 한 번만 더한다) — z=0.5(눈높이)가 화면
                // 정중앙, z=0(바닥)/z=1(천장)이 벽 렌더링과 똑같은 위치에 오도록
                // z0/z1 을 그대로 대입한다.
                let y_for_z = |z: f32| area.h / 2.0 - (z - 0.5) * (area.h / depth);
                let top = y_for_z(z1).clamp(0.0, area.h);
                let bottom = y_for_z(z0).clamp(0.0, area.h);

                let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
                let shade = fog * side_shade;
                let color = [p.color[0] * shade, p.color[1] * shade, p.color[2] * shade, p.color[3]];

                let x = area.x + col as f32 * col_w;
                r.rect(x, area.y + top, col_w + 0.6, (bottom - top).max(0.0), color);
                *depth_slot = depth;
            }

            // 위/아래 면 — 광선-평면 교차로는 못 구해서(우리 광선엔 z 가 없다)
            // 꼭짓점 투영 스캔라인으로 채운다. 둘 다 항상 시도한다 — 눈높이
            // (0.5)보다 낮은 물체는 위에서 내려다본 윗면이, 눈높이보다 높이 떠
            // 있는 물체(바닥에서 띄운 등받이 등)는 밑면이 이 방식 하나로 자연히
            // 나온다(둘 다 보이지 않는 각도에선 그냥 화면 밖으로 투영되거나
            // 다른 것에 가려져서 그려지지 않는다).
            let top_color = [p.color[0] * 0.9, p.color[1] * 0.9, p.color[2] * 0.9, p.color[3]];
            let bottom_color = [p.color[0] * 0.6, p.color[1] * 0.6, p.color[2] * 0.6, p.color[3]];
            self.render_horizontal_face(r, area, fov, col_w, col_depth, min_x, max_x, min_y, max_y, z1, top_color);
            self.render_horizontal_face(r, area, fov, col_w, col_depth, min_x, max_x, min_y, max_y, z0, bottom_color);
        }
    }

    // 플레이어 기준 각도/거리로 world 좌표 한 점을 화면 좌표로 투영한다 —
    // render_billboards 가 쓰는 것과 같은 공식이지만, 여기서는 임의의 z(높이)
    // 까지 받아서 위/아래 면의 꼭짓점 투영에 쓴다. 카메라 뒤에 있으면 None.
    fn project_point(&self, area: Rect, fov: f32, wx: f32, wy: f32, wz: f32) -> Option<(f32, f32, f32)> {
        let rel_x = wx - self.player_x;
        let rel_y = wy - self.player_y;
        let dist = rel_x.hypot(rel_y);
        if dist < 1e-4 {
            return None;
        }
        let mut rel_angle = rel_y.atan2(rel_x) - self.player_dir;
        while rel_angle > PI {
            rel_angle -= 2.0 * PI;
        }
        while rel_angle < -PI {
            rel_angle += 2.0 * PI;
        }
        let depth = dist * rel_angle.cos();
        if depth <= 0.0001 {
            return None; // 카메라 뒤
        }
        let camera_x = rel_angle / (fov / 2.0);
        let screen_x = area.x + (camera_x + 1.0) / 2.0 * area.w;
        let screen_y = area.y + area.h / 2.0 - (wz - 0.5) * (area.h / depth);
        Some((screen_x, screen_y, depth))
    }

    // 한 변(p0→p1, 둘 다 (화면x, 화면y, 깊이))이 화면 x=x_at 를 지나는 지점의
    // (화면y, 깊이)를 선형보간으로 구한다 — render_horizontal_face 의 스캔라인
    // 채우기에 쓰는 보조 함수.
    fn edge_at_x(p0: (f32, f32, f32), p1: (f32, f32, f32), x_at: f32) -> Option<(f32, f32)> {
        let (x0, y0, d0) = p0;
        let (x1, y1, d1) = p1;
        if !((x0 <= x_at && x_at <= x1) || (x1 <= x_at && x_at <= x0)) {
            return None;
        }
        let t = if (x1 - x0).abs() < 1e-6 { 0.0 } else { (x_at - x0) / (x1 - x0) };
        Some((y0 + (y1 - y0) * t, d0 + (d1 - d0) * t))
    }

    // 수평 평면(z=const) 하나 — 상자의 윗면/아랫면 — 을 화면에 채운다. 네
    // 꼭짓점을 각각 투영한 뒤(project_point), 그 투영이 걸치는 컬럼마다
    // 사각형의 위/아래 변을 선형보간(edge_at_x)으로 구해 세로띠를 그리는 아주
    // 단순한 스캔라인 채우기다 — 꼭짓점 하나라도 카메라 바로 뒤로 넘어가면
    // (아주 가까이 붙어서 보는 드문 경우) 이번 프레임엔 그냥 생략한다.
    #[allow(clippy::too_many_arguments)]
    fn render_horizontal_face(
        &self, r: &mut Renderer, area: Rect, fov: f32, col_w: f32, col_depth: &mut [f32], min_x: f32, max_x: f32, min_y: f32, max_y: f32,
        z: f32, color: [f32; 4],
    ) {
        let corners_world = [(min_x, min_y), (max_x, min_y), (max_x, max_y), (min_x, max_y)];
        let mut proj = [(0.0f32, 0.0f32, 0.0f32); 4];
        for (i, &(wx, wy)) in corners_world.iter().enumerate() {
            match self.project_point(area, fov, wx, wy, z) {
                Some(p) => proj[i] = p,
                None => return,
            }
        }

        let num_rays = col_depth.len();
        let min_screen_x = proj.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_screen_x = proj.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_col = (((min_screen_x - area.x) / col_w).floor().max(0.0)) as usize;
        let max_col = (((max_screen_x - area.x) / col_w).ceil().max(0.0)) as usize;
        let max_col = max_col.min(num_rays.saturating_sub(1));
        if min_col > max_col {
            return;
        }

        for (col, depth_slot) in col_depth.iter_mut().enumerate().take(max_col + 1).skip(min_col) {
            let cx = area.x + (col as f32 + 0.5) * col_w;
            let mut hits: Vec<(f32, f32)> = Vec::new(); // (화면y, 깊이)
            for i in 0..4 {
                if let Some(hit) = Self::edge_at_x(proj[i], proj[(i + 1) % 4], cx) {
                    hits.push(hit);
                }
            }
            if hits.len() < 2 {
                continue;
            }
            hits.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (y_top, d_top) = hits[0];
            let (y_bottom, d_bottom) = hits[hits.len() - 1];
            let depth = ((d_top + d_bottom) / 2.0).max(0.0001);
            if depth >= *depth_slot {
                continue;
            }

            let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
            let c = [color[0] * fog, color[1] * fog, color[2] * fog, color[3]];
            let top = y_top.clamp(area.y, area.y + area.h);
            let bottom = y_bottom.clamp(area.y, area.y + area.h);
            let x = area.x + col as f32 * col_w;
            r.rect(x, top, col_w + 0.6, (bottom - top).max(0.0), c);
            *depth_slot = depth;
        }
    }
}

// 진짜 입체(축 정렬 상자)로 그리는 물체 — 책상/의자처럼 각져서 옆에서 봤을 때도
// 실제로 옆면이 보여야 하는 것에 쓴다(둥근 물체는 Billboard 로 충분하다).
// center_x/center_y 는 바닥 위 발밑 중심, width(x축)/depth(y축)/height 는 전부
// 칸 크기(=1.0)를 기준으로 한 실제 치수, base 는 바닥에서 밑면까지 띄운 높이
// (의자 등받이처럼 좌판 위에 얹힌 부분에 쓴다, 그 외엔 보통 0.0).
pub struct Prop3D {
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub depth: f32,
    pub height: f32,
    pub base: f32,
    pub color: [f32; 4],
}
