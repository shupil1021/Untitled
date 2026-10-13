//! 도어즈 게임의 월드 — 맵(방/문/물건 상자들)과 조준 대상.

use crate::render::mesh3d::{Box3D, Camera};

use super::super::game3d::boxes::solid;

use super::super::game3d::player::aim_dist;

const FLOOR_COLOR: [f32; 4] = [0.28, 0.26, 0.24, 1.0];
const CEILING_COLOR: [f32; 4] = [0.33, 0.33, 0.32, 1.0];
const WALL_COLOR: [f32; 4] = [0.45, 0.43, 0.4, 1.0];

// 방 — 문은 앞쪽(-Z) 벽 가운데. 플레이어는 뒤쪽(+Z)에서 문을 바라보고 시작한다
// (yaw=0 이면 정면이 -Z, mesh3d.rs::Camera::forward_flat 참고).
const ROOM_MIN_X: f32 = -3.0;
const ROOM_MAX_X: f32 = 3.0;
const ROOM_MIN_Z: f32 = -3.5; // 문이 있는 벽
const ROOM_MAX_Z: f32 = 2.5;
const ROOM_HEIGHT: f32 = 2.6;
const WALL_THICK: f32 = 0.15;
pub const SPAWN: [f32; 3] = [0.0, 0.0, 1.2];

// 문 — 앞쪽 벽의 [DOOR_MIN_X, DOOR_MIN_X+DOOR_WIDTH] 틈을 정확히 채운다.
const DOOR_WIDTH: f32 = 1.5;
const DOOR_MIN_X: f32 = -DOOR_WIDTH / 2.0;
const DOOR_HALF_H: f32 = 1.15;
const DOOR_HALF_T: f32 = 0.06;
const DOOR_COLOR: [f32; 4] = [0.35, 0.24, 0.16, 1.0];
const HANDLE_HALF: [f32; 3] = [0.05, 0.05, 0.05];
const HANDLE_COLOR: [f32; 4] = [0.8, 0.72, 0.45, 1.0];
// 손잡이 — 문의 오른쪽 가장자리 근처, 방 안쪽(+Z) 면에서 살짝 튀어나온 자리.
pub const HANDLE_POS: [f32; 3] = [DOOR_MIN_X + DOOR_WIDTH - 0.15, DOOR_HALF_H - 0.45, ROOM_MIN_Z + 0.09];
// 문이 열린 뒤 밖으로 나갈 때 조준하는 자리(문간 가운데).
pub const EXIT_POS: [f32; 3] = [0.0, DOOR_HALF_H, ROOM_MIN_Z + 0.2];
// 문이 올라가는 높이(문 위의 상인방 속으로 들어간다).
pub const DOOR_LIFT: f32 = DOOR_HALF_H * 2.0 - 0.1;

// 방 안의 조사할 수 있는 물건 — 상자 하나로 표현하고(Box3D, 충돌 있음), 조준 판정은
// 상자 중심을 기준으로 한다. text 는 조사했을 때 나오는 한 줄.
pub struct Prop {
    pub name: &'static str,
    pub center: [f32; 3],
    pub half: [f32; 3],
    pub color: [f32; 4],
    pub text: &'static str,
}

// 벽 쪽에 붙여서 스폰↔문 사이 통로는 비워둔다.
pub const PROPS: [Prop; 4] = [
    Prop { name: "Desk", center: [-2.3, 0.4, -1.5], half: [0.5, 0.4, 0.3], color: [0.38, 0.28, 0.18, 1.0], text: "낡은 책상이다. 서랍은 텅 비어 있다." },
    Prop { name: "Crate", center: [2.3, 0.3, -2.5], half: [0.3, 0.3, 0.3], color: [0.45, 0.35, 0.2, 1.0], text: "먼지 쌓인 나무 상자다. 뚜껑이 못으로 박혀 있다." },
    Prop { name: "Shelf", center: [2.6, 0.9, 0.5], half: [0.25, 0.9, 0.6], color: [0.3, 0.25, 0.2, 1.0], text: "빈 선반이다. 먼지 자국만 남아 있다." },
    Prop { name: "Trash", center: [-2.4, 0.15, 1.6], half: [0.4, 0.15, 0.4], color: [0.3, 0.32, 0.22, 1.0], text: "구겨진 종이와 쓰레기가 쌓여 있다." },
];

// 방 서쪽 벽 쪽의 우편함 — 편지를 넣어 보내는 곳(시트 오른쪽 표의 "우편함").
pub const MAILBOX_CENTER: [f32; 3] = [-2.6, 0.6, 0.4];
const MAILBOX_HALF: [f32; 3] = [0.25, 0.6, 0.25];
const MAILBOX_COLOR: [f32; 4] = [0.3, 0.35, 0.5, 1.0];
// 부탁을 수락하면 책상 위에 나타나는 편지(작고 납작한 흰 상자).
pub const LETTER_CENTER: [f32; 3] = [-2.3, 0.81, -1.5];
const LETTER_HALF: [f32; 3] = [0.12, 0.01, 0.09];
const LETTER_COLOR: [f32; 4] = [0.92, 0.9, 0.78, 1.0];

pub fn letter_box() -> Box3D {
    Box3D { center: LETTER_CENTER, half: LETTER_HALF, yaw: 0.2, pitch: 0.0, roll: 0.0, color: LETTER_COLOR, texture: None, walkable: false, solid: false }
}

// 조준선이 가리킬 수 있는 대상 — 문 손잡이, 방 안의 물건 하나, 책상 위의 편지, 우편함.
#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Door,
    Prop(usize),
    Letter,
    Mailbox,
}

impl Target {
    pub fn pos(self) -> [f32; 3] {
        match self {
            Target::Door => HANDLE_POS,
            Target::Prop(i) => PROPS[i].center,
            Target::Letter => LETTER_CENTER,
            Target::Mailbox => MAILBOX_CENTER,
        }
    }
}

// 사방이 막힌 방 — 바닥/천장/벽 4면(앞쪽 벽은 문간만 비우고 좌우 조각 + 상인방) + 문. lift 는 문이
// 얼마나 위로 올라갔는지(0 = 닫힘, DOOR_LIFT = 다 열림) — 열리면 문 뒤로 하얀 빛이 보인다.
pub fn build_room(lift: f32) -> Vec<Box3D> {
    let cx = (ROOM_MIN_X + ROOM_MAX_X) / 2.0;
    let cz = (ROOM_MIN_Z + ROOM_MAX_Z) / 2.0;
    let hx = (ROOM_MAX_X - ROOM_MIN_X) / 2.0 + WALL_THICK;
    let hz = (ROOM_MAX_Z - ROOM_MIN_Z) / 2.0 + WALL_THICK;
    let hy = ROOM_HEIGHT / 2.0;

    let mut boxes = vec![
        solid([cx, -0.1, cz], [hx, 0.1, hz], FLOOR_COLOR, true),
        solid([cx, ROOM_HEIGHT + 0.1, cz], [hx, 0.1, hz], CEILING_COLOR, false),
        solid([ROOM_MIN_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([ROOM_MAX_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([cx, hy, ROOM_MAX_Z], [hx, hy, WALL_THICK], WALL_COLOR, false),
    ];

    let door_max_x = DOOR_MIN_X + DOOR_WIDTH;
    let left_hw = (DOOR_MIN_X - ROOM_MIN_X) / 2.0;
    let right_hw = (ROOM_MAX_X - door_max_x) / 2.0;
    boxes.push(solid([ROOM_MIN_X + left_hw, hy, ROOM_MIN_Z], [left_hw, hy, WALL_THICK], WALL_COLOR, false));
    boxes.push(solid([door_max_x + right_hw, hy, ROOM_MIN_Z], [right_hw, hy, WALL_THICK], WALL_COLOR, false));
    let door_top = DOOR_HALF_H * 2.0;
    let lintel_hy = (ROOM_HEIGHT - door_top) / 2.0;
    boxes.push(solid([0.0, door_top + lintel_hy, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, lintel_hy, WALL_THICK], WALL_COLOR, false));

    // 문은 열려 있는 동안(조금이라도 올라간 뒤) 막지 않는다 — 문간으로 걸어 들어갈 수 있다.
    let open = lift > 0.05;
    let mut door = solid([DOOR_MIN_X + DOOR_WIDTH / 2.0, DOOR_HALF_H + lift, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, DOOR_HALF_H, DOOR_HALF_T], DOOR_COLOR, false);
    door.solid = !open;
    boxes.push(door);
    boxes.push(Box3D { center: [HANDLE_POS[0], HANDLE_POS[1] + lift, HANDLE_POS[2]], half: HANDLE_HALF, yaw: 0.0, pitch: 0.0, roll: 0.0, color: HANDLE_COLOR, texture: None, walkable: false, solid: false });
    if open {
        // 문 뒤의 빛 — 문간 안쪽에 선 눈부신 벽. 걸어 들어가다 막히는 자리이기도 하다.
        boxes.push(solid([0.0, DOOR_HALF_H, ROOM_MIN_Z - 0.5], [DOOR_WIDTH / 2.0, DOOR_HALF_H, 0.1], [1.0, 0.97, 0.85, 1.0], false));
    }
    for p in &PROPS {
        boxes.push(solid(p.center, p.half, p.color, false));
    }
    boxes.push(solid(MAILBOX_CENTER, MAILBOX_HALF, MAILBOX_COLOR, false));
    boxes
}

// 조준선(화면 중앙)이 향한 대상 중 가장 가까운 것 — available 이 false 인 대상은 후보에서 뺀다
// (아직 안 나타난 편지, 지금은 말을 못 거는 문 등).
pub fn aimed_target(cam: &Camera, door_pos: [f32; 3], available: impl Fn(Target) -> bool) -> Option<Target> {
    let candidates = [Target::Door, Target::Letter, Target::Mailbox].into_iter().chain((0..PROPS.len()).map(Target::Prop));
    candidates
        .filter(|&t| available(t))
        .filter_map(|t| aim_dist(cam, if t == Target::Door { door_pos } else { t.pos() }).map(|d| (t, d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, _)| t)
}
