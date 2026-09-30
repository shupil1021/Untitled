//! 미로 생성 — 셀 N x N 짜리 완전 미로(어느 두 칸 사이든 길이 딱 하나)를 깊이 우선 탐색
//! (재귀 백트래커)으로 만들고, 벽/바닥/천장을 `Box3D` 로 세운다. 출발 칸에서 가장 먼 칸이
//! 도착 지점이다.

use crate::random::Rng;
use crate::render::mesh3d::Box3D;

const N: usize = 5; // 셀 개수(한 변)
const TILES: usize = 2 * N + 1; // 벽까지 포함한 타일 개수(한 변)
const TILE: f32 = 2.0; // 타일 한 칸의 실제 크기(m) — 복도 폭
const WALL_HALF_Y: f32 = 1.3;

const WALL_COLOR: [f32; 4] = [0.25, 0.32, 0.27, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.2, 0.2, 0.18, 1.0];
const CEILING_COLOR: [f32; 4] = [0.15, 0.17, 0.16, 1.0];

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

fn solid(center: [f32; 3], half: [f32; 3], color: [f32; 4], walkable: bool) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color, texture: None, walkable, solid: true }
}

type Walls = [[bool; TILES]; TILES]; // walls[tz][tx] — true 면 벽

// 미로를 파낸다 — (벽 지도, 도착 칸 (gx, gz)).
fn carve(rng: &mut Rng) -> (Walls, (usize, usize)) {
    let mut wall = [[true; TILES]; TILES]; // wall[tz][tx]
    let mut visited = [[false; N]; N]; // visited[cz][cx]
    let mut depth = [[0usize; N]; N];
    let mut stack = vec![(0usize, 0usize)];
    visited[0][0] = true;
    wall[1][1] = false;

    while let Some(&(cx, cz)) = stack.last() {
        let mut options = Vec::new();
        if cx > 0 && !visited[cz][cx - 1] {
            options.push((cx - 1, cz));
        }
        if cx + 1 < N && !visited[cz][cx + 1] {
            options.push((cx + 1, cz));
        }
        if cz > 0 && !visited[cz - 1][cx] {
            options.push((cx, cz - 1));
        }
        if cz + 1 < N && !visited[cz + 1][cx] {
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
    pub fn generate(rng: &mut Rng) -> Maze {
        let (wall, (gx, gz)) = carve(rng);

        let mut boxes = Vec::new();
        let span = (TILES - 1) as f32 * TILE / 2.0; // 미로 중심 좌표(x, z 둘 다)
        let half = TILES as f32 * TILE / 2.0;
        boxes.push(solid([span, -0.1, span], [half, 0.1, half], FLOOR_COLOR, true));
        boxes.push(solid([span, WALL_HALF_Y * 2.0 + 0.1, span], [half, 0.1, half], CEILING_COLOR, false));
        for (tz, row) in wall.iter().enumerate() {
            for (tx, &is_wall) in row.iter().enumerate() {
                if is_wall {
                    let (x, z) = tile_center(tx, tz);
                    boxes.push(solid([x, WALL_HALF_Y, z], [TILE / 2.0, WALL_HALF_Y, TILE / 2.0], WALL_COLOR, false));
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
        for seed in 1..200u64 {
            let (wall, (gx, gz)) = carve(&mut Rng::new(seed));
            assert_ne!((gx, gz), (0, 0), "seed {seed}");
            let mut seen = [[false; TILES]; TILES];
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
}
