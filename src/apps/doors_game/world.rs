//! 게임 월드 — 맵(방/문 상자들)과 플레이어 이동/충돌, 조준 판정.

use miniquad::KeyCode;

use crate::render::mesh3d::{ground_height, resolve_horizontal, v_add, v_dot, v_len, v_scale, v_sub, Box3D, Camera};
use crate::scenes::Input;

const MOVE_SPEED: f32 = 2.6;
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55;
const GRAVITY: f32 = -12.0;

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
const SPAWN: [f32; 3] = [0.0, 0.0, 1.2];

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

// 조준 판정 — 이 거리 안 + 이 각도(코사인) 안이면 "조준 중".
const AIM_MAX_DIST: f32 = 3.0;
const AIM_MAX_COS: f32 = 0.95;

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

// 조준선이 가리킬 수 있는 대상 — 문 손잡이 또는 방 안의 물건 하나.
#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Door,
    Prop(usize),
}

impl Target {
    pub fn pos(self) -> [f32; 3] {
        match self {
            Target::Door => HANDLE_POS,
            Target::Prop(i) => PROPS[i].center,
        }
    }
}

fn solid(center: [f32; 3], half: [f32; 3], color: [f32; 4], walkable: bool) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color, texture: None, walkable, solid: true }
}

// 사방이 막힌 방 — 바닥/천장/벽 4면(앞쪽 벽은 문간만 비우고 좌우 조각 + 상인방) + 닫힌 문.
pub fn build_room() -> Vec<Box3D> {
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

    boxes.push(solid([DOOR_MIN_X + DOOR_WIDTH / 2.0, DOOR_HALF_H, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, DOOR_HALF_H, DOOR_HALF_T], DOOR_COLOR, false));
    boxes.push(Box3D { center: HANDLE_POS, half: HANDLE_HALF, yaw: 0.0, pitch: 0.0, roll: 0.0, color: HANDLE_COLOR, texture: None, walkable: false, solid: false });
    for p in &PROPS {
        boxes.push(solid(p.center, p.half, p.color, false));
    }
    boxes
}

// 조준선(화면 중앙)이 향한 대상 중 가장 가까운 것 — 거리 + 각도만 본다(벽에 가려져도 판정된다).
pub fn aimed_target(cam: &Camera) -> Option<Target> {
    let candidates = std::iter::once(Target::Door).chain((0..PROPS.len()).map(Target::Prop));
    candidates
        .filter_map(|t| {
            let to = v_sub(t.pos(), cam.pos);
            let dist = v_len(to);
            let aimed = (0.05..=AIM_MAX_DIST).contains(&dist) && v_dot(v_scale(to, 1.0 / dist), cam.forward()) > AIM_MAX_COS;
            aimed.then_some((t, dist))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, _)| t)
}

pub struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
}

impl Player {
    pub fn spawn() -> Player {
        Player { feet: SPAWN, yaw: 0.0, pitch: 0.0, vel_y: 0.0 }
    }

    pub fn camera(&self) -> Camera {
        Camera { pos: [self.feet[0], self.feet[1] + EYE_OFFSET, self.feet[2]], yaw: self.yaw, pitch: self.pitch }
    }

    pub fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw -= dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    // can_move 가 false 면(창이 포커스가 아님) 키 입력은 무시하고 중력만 적용한다.
    pub fn update(&mut self, input: &Input, can_move: bool, dt: f32, boxes: &[Box3D]) {
        if can_move {
            let cam = self.camera();
            let (fwd, right) = (cam.forward_flat(), cam.right_flat());
            let mut dir = [0.0f32; 3];
            if input.is_down(KeyCode::W) {
                dir = v_add(dir, fwd);
            }
            if input.is_down(KeyCode::S) {
                dir = v_sub(dir, fwd);
            }
            if input.is_down(KeyCode::A) {
                dir = v_sub(dir, right);
            }
            if input.is_down(KeyCode::D) {
                dir = v_add(dir, right);
            }
            let len = v_len(dir);
            if len > 1e-4 {
                let step = v_scale(dir, MOVE_SPEED * dt / len);
                self.feet[0] += step[0];
                self.feet[2] += step[2];
            }
        }

        let pushed = resolve_horizontal(self.feet, PLAYER_RADIUS, self.feet[1], PLAYER_HEIGHT, boxes);
        self.feet[0] = pushed[0];
        self.feet[2] = pushed[2];

        self.vel_y += GRAVITY * dt;
        let predicted_y = self.feet[1] + self.vel_y * dt;
        match ground_height(self.feet[0], self.feet[2], self.feet[1], 60.0, boxes) {
            Some(g) if predicted_y <= g && self.vel_y <= 0.0 => {
                self.feet[1] = g;
                self.vel_y = 0.0;
            }
            _ => self.feet[1] = predicted_y,
        }
    }
}
