//! 스테이지 1 방(시트의 "B+a stage-1") — 미로에서 물뿌리개를 얻고 올라오면 나오는 방.
//! 화분 하나, 화분을 조사한 뒤 플레이어가 등을 돌렸을 때 남쪽 벽에 나타나는 사물함, 북쪽 벽의
//! 문 3개(미로 1~3으로 이어짐)가 있다. 방은 처음보다 넓게(14x14) 잡았다. 여기서는 배치(상수)와 벽만 만들고, 무엇을 언제 보여줄지는 mod.rs 가 정한다.

use crate::render::mesh3d::Box3D;

use super::super::game3d::boxes::solid;

const MIN_X: f32 = -7.0;
const MAX_X: f32 = 7.0;
const MIN_Z: f32 = -9.0; // 문이 있는 벽
const MAX_Z: f32 = 5.0;
const HEIGHT: f32 = 2.8;
const WALL_THICK: f32 = 0.15;

const FLOOR_COLOR: [f32; 4] = [0.3, 0.27, 0.22, 1.0];
const CEILING_COLOR: [f32; 4] = [0.36, 0.35, 0.33, 1.0];
const WALL_COLOR: [f32; 4] = [0.5, 0.46, 0.4, 1.0];

pub const SPAWN: [f32; 3] = [0.0, 0.0, 3.0]; // 문들을 마주보는 자리(yaw 0 이면 -Z 쪽)

// 조사할 수 있는 물건이 놓이는 자리(중심)와 크기(반너비).
pub struct Slot {
    pub center: [f32; 3],
    pub half: [f32; 3],
}

// 화분 — 동쪽 벽 쪽.
pub const POT: Slot = Slot { center: [5.5, 0.25, -3.0], half: [0.25, 0.25, 0.25] };
// 사물함 — 남쪽 벽(문들을 마주 보는 벽, 스폰 뒤). 화분을 보는 동안엔 시야 밖이고, 플레이어가 화면에서
// 이 자리를 벗어난 순간에 생긴다(mod.rs).
pub const LOCKER: Slot = Slot { center: [-2.5, 0.9, MAX_Z - 0.3], half: [0.5, 0.9, 0.25] };
// 서쪽 벽 쪽의 우체통 — 꽃이 피면 꺾어서 여기에 넣어 보낸다(시트의 SUB A+a-17).
pub const MAILBOX: Slot = Slot { center: [-6.55, 0.6, -3.0], half: [0.25, 0.6, 0.25] };
// 화분에 꽃이 피면 그 위에 서는 꽃(가는 기둥 하나).
pub const FLOWER: Slot = Slot { center: [5.5, 0.8, -3.0], half: [0.15, 0.3, 0.15] };
// 북쪽 벽에 나란히 있는 문 3개(미로 1, 2, 3).
pub const DOORS: [Slot; 3] = [
    Slot { center: [-4.5, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
    Slot { center: [0.0, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
    Slot { center: [4.5, 1.1, MIN_Z + WALL_THICK + 0.06], half: [0.6, 1.1, 0.06] },
];

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

// 씨앗/꽃 스프라이트가 서는 자리(화분 위 가운데).
pub const SPROUT_POS: [f32; 3] = [POT.center[0], POT.center[1] * 2.0, POT.center[2]];
