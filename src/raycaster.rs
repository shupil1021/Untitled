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

// 빌보드(항상 카메라를 향하는 평면 하나)로 그릴 모양 — 진짜 입체는 아니고
// "정면에서 본 실루엣"만 원근감 있게 투영한다. 옆에서 보면 납작해 보이는 건
// 이 엔진(벽 DDA + 평면 스프라이트)의 근본적인 한계다 — 각도에 따라 진짜
// 다른 면이 보이는 물체(책상/의자를 옆에서 봤을 때 등)가 필요해지면 벽처럼
// 격자에 다시 박아 넣는(부분 높이 벽 등) 훨씬 큰 확장이 필요하다.
pub enum BillboardShape {
    Circle, // 코인처럼 완전히 둥근 물체 — world_width 를 지름으로 쓴다(world_height 무시)
    Rect,   // 책상/의자처럼 각진 물체 — world_width × world_height 사각형을 그대로 그린다
}

// 바닥 위(또는 바닥에서 world_base 만큼 띄운 자리)에 놓인 작은 물체 하나.
// world_width/world_height/world_base 는 전부 칸 크기(=1.0)를 기준으로 한 실제
// 치수다. render_billboards() 가 벽과 같은 척도로 투영해서 원근감 있게 그린다 —
// 의자의 등받이처럼 바닥에서 살짝 뜬 부분을 표현하고 싶으면 world_base 를 쓴다.
pub struct Billboard {
    pub x: f32,
    pub y: f32,
    pub world_width: f32,
    pub world_height: f32,
    pub world_base: f32, // 바닥 ~ 이 물체의 밑면까지 띄운 높이 — 0 이면 바닥에 붙어있다
    pub color: [f32; 4],
    pub shape: BillboardShape,
}

impl Billboard {
    // 코인처럼 바닥에 붙은 원형 물체 — 지금까지 쓰던 3-필드짜리 생성 코드를
    // 그대로 대체한다.
    pub fn coin(x: f32, y: f32, world_diameter: f32, color: [f32; 4]) -> Billboard {
        Billboard { x, y, world_width: world_diameter, world_height: world_diameter, world_base: 0.0, color, shape: BillboardShape::Circle }
    }

    // 책상/의자 등받이처럼 각진 물체 — base 는 바닥에서 밑면까지 띄운 높이(의자
    // 등받이면 좌판 높이만큼, 그 외엔 보통 0.0).
    pub fn prop(x: f32, y: f32, world_width: f32, world_height: f32, base: f32, color: [f32; 4]) -> Billboard {
        Billboard { x, y, world_width, world_height, world_base: base, color, shape: BillboardShape::Rect }
    }
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
    // 가리키는 컬럼에서 벽보다 가까울 때만(col_depth — render_walls() 가 돌려준
    // 값) 그린다. 깊이 버퍼가 따로 없는 빌보드라 먼 것부터(화가 알고리즘) 그려야
    // 한 복도 안에 여러 개가 늘어서 있을 때 가까운(큰) 것이 먼(작은) 것을
    // 제대로 가린다.
    pub fn render_billboards(&self, r: &mut Renderer, area: Rect, fov: f32, col_depth: &[f32], billboards: &[Billboard]) {
        let num_rays = col_depth.len();
        let half_fov = fov / 2.0;
        let mut visible: Vec<VisibleBillboard> = Vec::new();
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
                continue; // 벽에 가려짐(폭이 넓은 물체도 중심 컬럼 하나만으로 판정하는 근사치)
            }

            let wall_h = (area.h / depth).min(area.h * 4.0);
            // 벽 렌더링 쪽 top/bottom 은 r.rect 에 그대로 넘길 y/height 라서 화면
            // 안으로 clamp 가 필요하지만, 여기 floor_y 는 기준점일 뿐이다 — 이것까지
            // clamp 하면 아주 가까이 다가갔을 때 화면 끝에 고정된 채 크기만 커져서
            // 물체가 위로 떠오르는 것처럼 보인다.
            let floor_y = area.y + (area.h + wall_h) / 2.0;
            let screen_x = area.x + (camera_x + 1.0) / 2.0 * area.w;
            let scale = area.h / depth; // 벽/바닥과 같은 척도 — 이 값을 곱하면 월드 유닛이 화면 픽셀이 된다
            let screen_w = (b.world_width * scale).clamp(1.0, area.h * 4.0);
            let screen_h = (b.world_height * scale).clamp(1.0, area.h * 4.0);
            let bottom_y = floor_y - b.world_base * scale; // world_base 만큼 바닥에서 띄운 밑면 위치
            let fog = (1.0 - (depth / self.fog_dist).clamp(0.0, 1.0) * 0.75).max(0.18);
            let color = [b.color[0] * fog, b.color[1] * fog, b.color[2] * fog, 1.0];
            visible.push(VisibleBillboard { depth, screen_x, bottom_y, screen_w, screen_h, color, shape: &b.shape });
        }
        // 먼 것부터(depth 내림차순) 그려서 가까운 물체가 항상 위에 온다.
        visible.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        for v in visible {
            match v.shape {
                BillboardShape::Circle => {
                    let radius = v.screen_w / 2.0;
                    fill_circle(r, v.screen_x, v.bottom_y - radius, radius, v.color);
                }
                BillboardShape::Rect => {
                    r.rect(v.screen_x - v.screen_w / 2.0, v.bottom_y - v.screen_h, v.screen_w, v.screen_h, v.color);
                }
            }
        }
    }
}

// render_billboards() 내부에서만 쓰는, 화면에 투영까지 끝난 빌보드 하나 — bottom_y
// 는 "이 물체 밑면"이 화면에서 어디 보이는지(Circle 이면 원 밑점, Rect 면 사각형
// 아랫변)다.
struct VisibleBillboard<'a> {
    depth: f32,
    screen_x: f32,
    bottom_y: f32,
    screen_w: f32,
    screen_h: f32,
    color: [f32; 4],
    shape: &'a BillboardShape,
}
