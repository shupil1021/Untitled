//! 미로 생성 — 셀 n x n 짜리 완전 미로를 깊이 우선 탐색(재귀 백트래커)으로 만든 뒤, 벽 몇 개를 더 허물어
//! 갈림길/순환 길(loops)을 늘려서 막다른 길과 돌아가는 길이 많은 복잡한 미로로 만들고, 벽/바닥/천장을
//! `Box3D` 로 세운다. 출발 칸에서 걸어서 가장 먼 칸(너비 우선 탐색)이 도착 지점이다.

use crate::random::Rng;
use crate::render::mesh3d::Box3D;

use super::super::game3d::boxes::solid;

const TILE: f32 = 2.0; // 타일 한 칸의 실제 크기(m) — 복도 폭
const WALL_HALF_Y: f32 = 1.3;

// 미로 하나의 크기와 색.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MazeStyle {
    pub n: usize,     // 셀 개수(한 변)
    pub loops: usize, // 완전 미로에서 추가로 허무는 벽 개수 — 많을수록 갈림길/순환 길이 늘어 복잡해진다
    pub wall: [f32; 4],
    pub floor: [f32; 4],
    pub ceiling: [f32; 4],
}

// 데모의 단 하나뿐인 미로 — 일곱 칸 한 변에 순환 길을 여섯 개 뚫어서, 짧지만 헷갈리게 만든다.
pub fn style() -> MazeStyle {
    MazeStyle { n: 7, loops: 6, wall: [0.25, 0.32, 0.27, 1.0], floor: [0.2, 0.2, 0.18, 1.0], ceiling: [0.15, 0.17, 0.16, 1.0] } // 이끼 낀 회녹색
}

pub struct Maze {
    pub boxes: Vec<Box3D>,
    pub start: [f32; 3],
    pub start_yaw: f32, // 출발할 때 열린 복도 쪽을 보게
    pub goal: [f32; 3], // 출발에서 가장 먼 칸의 중심(바닥 높이)
}

// 타일 (tx, tz) 의 중심 월드 좌표(x, z).
fn tile_center(tx: usize, tz: usize) -> (f32, f32) {
    (tx as f32 * TILE, tz as f32 * TILE)
}

type Walls = Vec<Vec<bool>>; // walls[tz][tx] — true 면 벽

// 미로를 파낸다 — (벽 지도, 도착 칸 (gx, gz)). n 은 셀 개수(한 변), 타일은 벽까지 포함해 2n+1 개다.
fn carve(rng: &mut Rng, n: usize, loops: usize) -> (Walls, (usize, usize)) {
    let tiles = 2 * n + 1;
    let mut wall = vec![vec![true; tiles]; tiles]; // wall[tz][tx]
    let mut visited = vec![vec![false; n]; n]; // visited[cz][cx]
    let mut stack = vec![(0usize, 0usize)];
    visited[0][0] = true;
    wall[1][1] = false;

    while let Some(&(cx, cz)) = stack.last() {
        let mut options = Vec::new();
        if cx > 0 && !visited[cz][cx - 1] {
            options.push((cx - 1, cz));
        }
        if cx + 1 < n && !visited[cz][cx + 1] {
            options.push((cx + 1, cz));
        }
        if cz > 0 && !visited[cz - 1][cx] {
            options.push((cx, cz - 1));
        }
        if cz + 1 < n && !visited[cz + 1][cx] {
            options.push((cx, cz + 1));
        }
        if options.is_empty() {
            stack.pop();
            continue;
        }
        let (nx, nz) = options[rng.next_u32() as usize % options.len()];
        visited[nz][nx] = true;
        wall[2 * nz + 1][2 * nx + 1] = false; // 새 칸
        wall[cz + nz + 1][cx + nx + 1] = false; // 두 칸 사이 벽
        stack.push((nx, nz));
    }

    // 두 칸 사이의 벽 중 몇 개를 허물어 순환 길을 만든다(완전 미로의 "길이 딱 하나"를 깬다).
    let mut candidates = Vec::new();
    for tz in 1..tiles - 1 {
        for tx in 1..tiles - 1 {
            if !wall[tz][tx] || (tx % 2 == 1) == (tz % 2 == 1) {
                continue; // 이미 뚫렸거나, 칸/기둥 자리(칸 사이 벽이 아니다)
            }
            let (a, b) = if tx % 2 == 0 { ((tx - 1, tz), (tx + 1, tz)) } else { ((tx, tz - 1), (tx, tz + 1)) };
            if !wall[a.1][a.0] && !wall[b.1][b.0] {
                candidates.push((tx, tz));
            }
        }
    }
    for _ in 0..loops.min(candidates.len()) {
        let i = rng.next_u32() as usize % candidates.len();
        let (tx, tz) = candidates.swap_remove(i);
        wall[tz][tx] = false;
    }

    // 도착 지점 = 출발 칸에서 걸어서 가장 먼 칸(너비 우선 탐색 — 순환 길이 생겼으니 깊이 우선 깊이로는 틀린다).
    let mut dist = vec![vec![usize::MAX; tiles]; tiles];
    let mut queue = std::collections::VecDeque::from([(1usize, 1usize)]);
    dist[1][1] = 0;
    let (mut gx, mut gz, mut best) = (0, 0, 0);
    while let Some((x, z)) = queue.pop_front() {
        let d = dist[z][x];
        if x % 2 == 1 && z % 2 == 1 && d > best {
            (gx, gz, best) = ((x - 1) / 2, (z - 1) / 2, d);
        }
        for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
            if !wall[nz][nx] && dist[nz][nx] == usize::MAX {
                dist[nz][nx] = d + 1;
                queue.push_back((nx, nz));
            }
        }
    }
    (wall, (gx, gz))
}

impl Maze {
    pub fn generate(rng: &mut Rng, style: &MazeStyle) -> Maze {
        let (wall, (gx, gz)) = carve(rng, style.n, style.loops);

        let tiles = 2 * style.n + 1;
        let mut boxes = Vec::new();
        let span = (tiles - 1) as f32 * TILE / 2.0; // 미로 중심 좌표(x, z 둘 다)
        let half = tiles as f32 * TILE / 2.0;
        boxes.push(solid([span, -0.1, span], [half, 0.1, half], style.floor, true));
        boxes.push(solid([span, WALL_HALF_Y * 2.0 + 0.1, span], [half, 0.1, half], style.ceiling, false));
        for (tz, row) in wall.iter().enumerate() {
            for (tx, &is_wall) in row.iter().enumerate() {
                if is_wall {
                    let (x, z) = tile_center(tx, tz);
                    boxes.push(solid([x, WALL_HALF_Y, z], [TILE / 2.0, WALL_HALF_Y, TILE / 2.0], style.wall, false));
                }
            }
        }

        // 출발 칸(1,1)에서 열려 있는 쪽을 본다 — yaw 0 이면 -Z, forward = [-sin, 0, -cos].
        let start_yaw = if !wall[2][1] { std::f32::consts::PI } else { -std::f32::consts::FRAC_PI_2 };
        let (sx, sz) = tile_center(1, 1);
        let (goal_x, goal_z) = tile_center(2 * gx + 1, 2 * gz + 1);
        Maze { boxes, start: [sx, 0.0, sz], start_yaw, goal: [goal_x, 0.0, goal_z] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 어떤 시드로 만들어도 출발 칸에서 도착 칸까지 벽 없이 걸어갈 수 있어야 하고, 도착은 출발과 달라야 한다.
    #[test]
    fn goal_is_reachable_from_start() {
        let st = style();
        for seed in 1..200u64 {
            let (wall, (gx, gz)) = carve(&mut Rng::new(seed), st.n, st.loops);
            assert_ne!((gx, gz), (0, 0), "seed {seed}");
            let tiles = 2 * st.n + 1;
            let mut seen = vec![vec![false; tiles]; tiles];
            let mut stack = vec![(1usize, 1usize)];
            seen[1][1] = true;
            while let Some((x, z)) = stack.pop() {
                for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
                    if !wall[nz][nx] && !seen[nz][nx] {
                        seen[nz][nx] = true;
                        stack.push((nx, nz));
                    }
                }
            }
            assert!(seen[2 * gz + 1][2 * gx + 1], "seed {seed}: goal unreachable");
            // 출발 칸에서 시선 방향(열린 쪽)이 실제로 열려 있는지 — start_yaw 계산과 같은 규칙.
            assert!(!wall[2][1] || !wall[1][2], "seed {seed}: start is boxed in");
        }
    }

    // 벽을 더 허물면 순환 길이 생긴다 — 열린 칸 사이 연결 수가 "칸 수 - 1"(완전 미로)보다 많아진다.
    #[test]
    fn extra_openings_make_loops() {
        let n = 7;
        let count = |wall: &Walls| -> (usize, usize) {
            let tiles = 2 * n + 1;
            let open = wall.iter().flatten().filter(|&&w| !w).count();
            (open, tiles)
        };
        let (perfect, _) = count(&carve(&mut Rng::new(5), n, 0).0);
        let (braided, _) = count(&carve(&mut Rng::new(5), n, 6).0);
        assert_eq!(braided, perfect + 6, "허문 만큼 열린 칸이 늘어난다");
    }

    // 도착 지점은 출발에서 충분히 멀다(너무 짧지도, 한 줄 길도 아니게).
    #[test]
    fn the_goal_is_far_but_not_endless() {
        let st = style();
        let maze = Maze::generate(&mut Rng::new(9), &st);
        let d = (maze.goal[0] - maze.start[0]).abs() + (maze.goal[2] - maze.start[2]).abs();
        assert!(d >= 2.0 * 6.0, "직선거리로도 여섯 칸 이상: {d}");
        assert!(maze.boxes.len() > 40);
    }
}
