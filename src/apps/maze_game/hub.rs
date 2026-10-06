//! 스테이지 1 방(시트의 "B+a stage-1") — 미로에서 물뿌리개를 얻고 올라오면 나오는 방.
//! 화분 하나, 화분을 조사하면 뒤에 나타나는 사물함, 북쪽 벽의 문 3개(미로 1~3으로 이어짐)가
//! 있다. 여기서는 배치(상수)와 벽만 만들고, 무엇을 언제 보여줄지는 mod.rs 가 정한다.

use crate::render::mesh3d::Box3D;

const MIN_X: f32 = -4.0;
const MAX_X: f32 = 4.0;
const MIN_Z: f32 = -5.0; // 문이 있는 벽
const MAX_Z: f32 = 3.0;
const HEIGHT: f32 = 2.8;
const WALL_THICK: f32 = 0.15;

const FLOOR_COLOR: [f32; 4] = [0.3, 0.27, 0.22, 1.0];
const CEILING_COLOR: [f32; 4] = [0.36, 0.35, 0.33, 1.0];
const WALL_COLOR: [f32; 4] = [0.5, 0.46, 0.4, 1.0];

pub const SPAWN: [f32; 3] = [0.0, 0.0, 1.5]; // 문들을 마주보는 자리(yaw 0 이면 -Z 쪽)

// 조사할 수 있는 물건이 놓이는 자리(중심)와 크기(반너비).
pub struct Slot {
    pub center: [f32; 3],
    pub half: [f32; 3],
}

// 화분 — 동쪽 벽 쪽, 사물함은 그 바로 뒤(벽 쪽)에 생긴다.
pub const POT: Slot = Slot { center: [2.7, 0.25, -1.0], half: [0.25, 0.25, 0.25] };
pub const LOCKER: Slot = Slot { center: [3.55, 0.9, -1.0], half: [0.25, 0.9, 0.5] };
// 서쪽 벽 쪽의 우체통 — 꽃이 피면 꺾어서 여기에 넣어 보낸다(시트의 SUB A+a-17).
pub const MAILBOX: Slot = Slot { center: [-3.55, 0.6, -1.0], half: [0.25, 0.6, 0.25] };
// 화분에 꽃이 피면 그 위에 서는 꽃(가는 기둥 하나).
pub const FLOWER: Slot = Slot { center: [2.7, 0.75, -1.0], half: [0.06, 0.3, 0.06] };
// 북쪽 벽에 나란히 있는 문 3개(미로 1, 2, 3).
pub const DOORS: [Slot; 3] = [
    Slot { center: [-2.5, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
    Slot { center: [0.0, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
    Slot { center: [2.5, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
];

fn solid(center: [f32; 3], half: [f32; 3], color: [f32; 4], walkable: bool) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color, texture: None, walkable, solid: true }
}

// 바닥/천장/벽 4면.
pub fn build_walls() -> Vec<Box3D> {
    let (cx, cz) = ((MIN_X + MAX_X) / 2.0, (MIN_Z + MAX_Z) / 2.0);
    let hx = (MAX_X - MIN_X) / 2.0 + WALL_THICK;
    let hz = (MAX_Z - MIN_Z) / 2.0 + WALL_THICK;
    let hy = HEIGHT / 2.0;
    vec![
        solid([cx, -0.1, cz], [hx, 0.1, hz], FLOOR_COLOR, true),
        solid([cx, HEIGHT + 0.1, cz], [hx, 0.1, hz], CEILING_COLOR, false),
        solid([MIN_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([MAX_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([cx, hy, MIN_Z], [hx, hy, WALL_THICK], WALL_COLOR, false),
        solid([cx, hy, MAX_Z], [hx, hy, WALL_THICK], WALL_COLOR, false),
    ]
}
