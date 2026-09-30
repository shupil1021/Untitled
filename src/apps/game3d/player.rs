//! 1인칭 플레이어 — WASD 이동 + 마우스 시점 + 중력/충돌, 그리고 조준 판정.
//! 창 안에서 도는 3D 게임들(doors_game, maze_game)이 같이 쓴다.

use miniquad::KeyCode;

use crate::render::mesh3d::{ground_height, resolve_horizontal, v_add, v_dot, v_len, v_scale, v_sub, Box3D, Camera};
use crate::scenes::Input;

const MOVE_SPEED: f32 = 2.6;
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55;
const GRAVITY: f32 = -12.0;

// 조준 판정 — 이 거리 안 + 이 각도(코사인) 안이면 "조준 중".
const AIM_MAX_DIST: f32 = 3.0;
const AIM_MAX_COS: f32 = 0.95;

// 화면 중앙 조준선이 target 을 향하고 있으면 그 거리 — 거리 + 각도만 본다(벽에 가려져도 판정된다).
pub fn aim_dist(cam: &Camera, target: [f32; 3]) -> Option<f32> {
    let to = v_sub(target, cam.pos);
    let dist = v_len(to);
    let aimed = (0.05..=AIM_MAX_DIST).contains(&dist) && v_dot(v_scale(to, 1.0 / dist), cam.forward()) > AIM_MAX_COS;
    aimed.then_some(dist)
}

pub struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
}

impl Player {
    // feet 자리에 yaw 방향(0 이면 -Z 쪽)을 보고 선다.
    pub fn at(feet: [f32; 3], yaw: f32) -> Player {
        Player { feet, yaw, pitch: 0.0, vel_y: 0.0 }
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
