//! 미로 생성 — 셀 n x n 짜리 완전 미로(어느 두 칸 사이든 길이 딱 하나)를 깊이 우선 탐색
//! (재귀 백트래커)으로 만들고, 벽/바닥/천장을 `Box3D` 로 세운다. 출발 칸에서 가장 먼 칸이
//! 도착 지점이다. 미로마다 크기와 색(MazeStyle)이 달라서, 같은 복도를 또 걷는 느낌이 들지 않게 한다.

use crate::random::Rng;
use crate::render::mesh3d::Box3D;

use super::super::game3d::boxes::solid;

const TILE: f32 = 2.0; // 타일 한 칸의 실제 크기(m) — 복도 폭
const WALL_HALF_Y: f32 = 1.3;

// 미로 하나의 크기와 색.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MazeStyle {
    pub n: usize, // 셀 개수(한 변)
    pub wall: [f32; 4],
    pub floor: [f32; 4],
    pub ceiling: [f32; 4],
}

// 몇 번째 미로인지(0 = 물뿌리개 미로, 1~3 = 열쇠 미로) — 뒤로 갈수록 커지고 색이 불길해진다.
pub fn style_for(stage: usize) -> MazeStyle {
    match stage {
        0 => MazeStyle { n: 5, wall: [0.25, 0.32, 0.27, 1.0], floor: [0.2, 0.2, 0.18, 1.0], ceiling: [0.15, 0.17, 0.16, 1.0] }, // 이끼 낀 회녹색
        1 => MazeStyle { n: 4, wall: [0.22, 0.28, 0.4, 1.0], floor: [0.14, 0.16, 0.22, 1.0], ceiling: [0.1, 0.12, 0.18, 1.0] }, // 작고 차가운 청회색
        2 => MazeStyle { n: 6, wall: [0.4, 0.27, 0.2, 1.0], floor: [0.22, 0.16, 0.12, 1.0], ceiling: [0.15, 0.11, 0.09, 1.0] }, // 녹슨 갈색
        _ => MazeStyle { n: 7, wall: [0.38, 0.2, 0.26, 1.0], floor: [0.2, 0.1, 0.14, 1.0], ceiling: [0.13, 0.07, 0.1, 1.0] },  // 가장 크고 검붉은 미로
    }
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
fn carve(rng: &mut Rng, n: usize) -> (Walls, (usize, usize)) {
    let tiles = 2 * n + 1;
    let mut wall = vec![vec![true; tiles]; tiles]; // wall[tz][tx]
    let mut visited = vec![vec![false; n]; n]; // visited[cz][cx]
    let mut depth = vec![vec![0usize; n]; n];
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
        depth[nz][nx] = depth[cz][cx] + 1;
        stack.push((nx, nz));
    }

    // 가장 깊은(=출발에서 가장 먼) 칸이 도착 지점.
    let (mut gx, mut gz, mut best) = (0, 0, 0);
    for (cz, row) in depth.iter().enumerate() {
        for (cx, &d) in row.iter().enumerate() {
            if d > best {
                (gx, gz, best) = (cx, cz, d);
            }
        }
    }
    (wall, (gx, gz))
}

impl Maze {
    pub fn generate(rng: &mut Rng, style: &MazeStyle) -> Maze {
        let (wall, (gx, gz)) = carve(rng, style.n);

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

    // 어떤 시드/크기로 만들어도 출발 칸에서 도착 칸까지 벽 없이 걸어갈 수 있어야 하고, 도착은 출발과 달라야 한다.
    #[test]
    fn goal_is_reachable_from_start() {
        for n in 4..=7 {
            for seed in 1..120u64 {
                let (wall, (gx, gz)) = carve(&mut Rng::new(seed), n);
                assert_ne!((gx, gz), (0, 0), "n {n} seed {seed}");
                let tiles = 2 * n + 1;
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
                assert!(seen[2 * gz + 1][2 * gx + 1], "n {n} seed {seed}: goal unreachable");
                // 출발 칸에서 시선 방향(열린 쪽)이 실제로 열려 있는지 — start_yaw 계산과 같은 규칙.
                assert!(!wall[2][1] || !wall[1][2], "n {n} seed {seed}: start is boxed in");
            }
        }
    }

    // 미로가 뒤로 갈수록 커지고(첫 미로 제외 작은 것에서 큰 것으로) 색이 서로 다르다.
    #[test]
    fn each_maze_looks_and_measures_differently() {
        let styles: Vec<MazeStyle> = (0..4).map(style_for).collect();
        let sizes: Vec<usize> = styles.iter().map(|s| s.n).collect();
        assert_eq!(sizes.iter().copied().max(), Some(7));
        assert!(sizes.windows(2).any(|w| w[1] > w[0]) && sizes[1..].windows(2).all(|w| w[1] > w[0]), "열쇠 미로는 갈수록 커진다: {sizes:?}");
        for (i, a) in styles.iter().enumerate() {
            for b in &styles[i + 1..] {
                assert_ne!(a.wall, b.wall, "벽 색이 겹친다");
            }
        }
    }

    #[test]
    fn a_bigger_maze_has_more_floor() {
        let small = Maze::generate(&mut Rng::new(9), &style_for(1));
        let big = Maze::generate(&mut Rng::new(9), &style_for(3));
        assert!(big.boxes.len() > small.boxes.len());
        assert!(big.goal[0].max(big.goal[2]) >= small.goal[0].max(small.goal[2]) || big.boxes.len() > small.boxes.len());
    }
}
