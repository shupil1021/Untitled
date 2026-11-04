//! 1인칭 플레이어 — WASD 이동(Shift 로 달리기) + 마우스 시점 + 중력/충돌, 그리고 조준 판정.
//! 창 안에서 도는 3D 게임들(doors_game, maze_game)이 같이 쓴다.

use crate::foundation::Action;

use super::keys;

use crate::render::mesh3d::{ground_height, resolve_horizontal, v_add, v_dot, v_len, v_scale, v_sub, Box3D, Camera};
use crate::scenes::Input;

const MOVE_SPEED: f32 = 2.6;
const SPRINT_MULT: f32 = 1.75; // Shift 를 누르고 있으면 이 배율로 달린다
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55;
const GRAVITY: f32 = -12.0;

// 조준 판정 — 이 거리 안이고, 조준선에서 벗어난 각도가 허용 각도 안이면 "조준 중". 허용 각도는 기본 18° 에 물체
// 크기(반지름 AIM_RADIUS)가 가까울수록 크게 보이는 만큼을 더한 것이라, 가까이 다가가면 더 널널해진다(작은 물건도
// 정확히 안 맞춰도 눌린다).
const AIM_MAX_DIST: f32 = 3.2;
const AIM_BASE_ANGLE: f32 = 0.32; // 약 18°
const AIM_RADIUS: f32 = 0.3; // m — 조사 대상 하나의 대략적인 반지름

// 화면 중앙 조준선이 target 을 향하고 있으면 "점수"(작을수록 더 정확히 조준한 것: 조준선에서 벗어난 각도 + 거리의 아주 작은
// 가중치). 여러 후보 중 점수가 가장 작은 것을 고르면 된다 — 가까운 것이 아니라 조준선에 가장 가까운 것이 이긴다.
// 거리 + 각도만 본다(벽에 가려져도 판정된다).
pub fn aim_score(cam: &Camera, target: [f32; 3]) -> Option<f32> {
    let to = v_sub(target, cam.pos);
    let dist = v_len(to);
    if !(0.05..=AIM_MAX_DIST).contains(&dist) {
        return None;
    }
    let angle = v_dot(v_scale(to, 1.0 / dist), cam.forward()).clamp(-1.0, 1.0).acos();
    let allowed = AIM_BASE_ANGLE + (AIM_RADIUS / dist).atan();
    (angle <= allowed).then_some(angle + dist * 0.02)
}

// 상호작용 때 카메라가 대상을 바라보는 데 걸리는 속도 — 클수록 빠르다(지수 보간의 계수).
const FOCUS_RATE: f32 = 9.0;
const FOCUS_DONE: f32 = 0.004; // 이만큼(라디안) 가까워지면 다 돌았다고 친다
// 멀리 있는 대상과 상호작용하면 카메라가 살짝 당겨진다(시야각을 1/(1+zoom) 로 좁힌다). 대상이 ZOOM_NEAR 안이면 안 당기고,
// ZOOM_FAR(조준 가능 거리 끝)에서 가장 많이(ZOOM_MAX) 당긴다.
const ZOOM_NEAR: f32 = 1.8;
const ZOOM_FAR: f32 = 3.2;
const ZOOM_MAX: f32 = 0.3;
const ZOOM_RATE: f32 = 6.0; // 확대/원상복구 속도(지수 보간 계수)

pub struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
    zoom: f32,      // 지금 확대 정도(0 = 기본 시야)
    zoom_goal: f32, // 확대 목표 — 대화가 끝나면 0 으로 돌아간다
    focus: Option<[f32; 3]>, // 카메라가 천천히 돌아서 바라볼 점(상호작용한 대상) — 플레이어가 움직이거나 시점을 돌리면 풀린다
}

impl Player {
    // feet 자리에 yaw 방향(0 이면 -Z 쪽)을 보고 선다.
    pub fn at(feet: [f32; 3], yaw: f32) -> Player {
        Player { feet, yaw, pitch: 0.0, vel_y: 0.0, zoom: 0.0, zoom_goal: 0.0, focus: None }
    }

    // 지금 서 있는 자리와 보는 방향(x, y, z, yaw) — 저장해 뒀다가 Player::at 로 되살린다.
    pub fn pose(&self) -> [f32; 4] {
        [self.feet[0], self.feet[1], self.feet[2], self.yaw]
    }

    pub fn camera(&self) -> Camera {
        Camera { pos: [self.feet[0], self.feet[1] + EYE_OFFSET, self.feet[2]], yaw: self.yaw, pitch: self.pitch }
    }

    // 이 점을 바라보게 한다 — 카메라가 부드럽게 돌아서 그쪽을 향한다(update_focus 가 매 프레임 조금씩 돌린다).
    // 대화 중처럼 입력이 멈춰 있는 동안에도 돌아가고, 플레이어가 직접 움직이거나 시점을 돌리면 그 순간 풀린다.
    // 대상이 멀수록(ZOOM_NEAR~ZOOM_FAR) 카메라가 조금 더 당겨진다 — 가까우면 확대 없이 돌아서기만 한다.
    pub fn focus_on(&mut self, target: [f32; 3]) {
        self.focus = Some(target);
        let eye = self.camera().pos;
        let dist = v_len(v_sub(target, eye));
        self.zoom_goal = ((dist - ZOOM_NEAR) / (ZOOM_FAR - ZOOM_NEAR)).clamp(0.0, 1.0) * ZOOM_MAX;
    }

    // 지금 확대 정도 — View::zoom 에 넘긴다.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    // 지금 카메라를 부드럽게 돌리는 중인지.
    #[cfg(test)]
    pub fn is_focusing(&self) -> bool {
        self.focus.is_some()
    }

    // 포커스 점을 향해 시점(yaw/pitch)을 조금 돌린다 — 가장 짧은 각도로, 다 돌았으면 포커스를 푼다. 매 프레임 한 번.
    // hold 가 true 인 동안(대화가 떠 있는 동안)은 확대를 유지하고, 포커스가 끝났는데 hold 가 풀리면 부드럽게 원래 시야로 돌아간다.
    pub fn update_focus(&mut self, dt: f32, hold: bool) {
        if !hold && self.focus.is_none() {
            self.zoom_goal = 0.0;
        }
        self.zoom += (self.zoom_goal - self.zoom) * (1.0 - (-ZOOM_RATE * dt.max(0.0)).exp());
        if self.zoom.abs() < 1e-3 && self.zoom_goal == 0.0 {
            self.zoom = 0.0;
        }
        let Some(target) = self.focus else { return };
        let eye = self.camera().pos;
        let (dx, dy, dz) = (target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]);
        let horizontal = (dx * dx + dz * dz).sqrt();
        if horizontal < 1e-3 && dy.abs() < 1e-3 {
            self.focus = None; // 바로 그 자리 — 바라볼 방향이 없다
            return;
        }
        // 카메라가 보는 방향은 [-sin(yaw)cos(pitch), sin(pitch), -cos(yaw)cos(pitch)] 라서 거꾸로 풀면 아래와 같다.
        let want_yaw = (-dx).atan2(-dz);
        let want_pitch = dy.atan2(horizontal).clamp(-MAX_PITCH, MAX_PITCH);
        let mut dyaw = (want_yaw - self.yaw) % std::f32::consts::TAU;
        if dyaw > std::f32::consts::PI {
            dyaw -= std::f32::consts::TAU;
        } else if dyaw < -std::f32::consts::PI {
            dyaw += std::f32::consts::TAU;
        }
        let dpitch = want_pitch - self.pitch;
        let k = 1.0 - (-FOCUS_RATE * dt.max(0.0)).exp();
        self.yaw += dyaw * k;
        self.pitch += dpitch * k;
        if dyaw.abs() < FOCUS_DONE && dpitch.abs() < FOCUS_DONE {
            self.yaw = want_yaw;
            self.pitch = want_pitch;
            self.focus = None;
        }
    }

    pub fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.focus = None; // 플레이어가 직접 시점을 돌리면 자동 포커싱은 멈춘다
        self.zoom_goal = 0.0;
        self.yaw -= dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    // can_move 가 false 면(창이 포커스가 아님) 키 입력은 무시하고 중력만 적용한다.
    pub fn update(&mut self, input: &Input, can_move: bool, dt: f32, boxes: &[Box3D]) {
        if can_move {
            let cam = self.camera();
            let (fwd, right) = (cam.forward_flat(), cam.right_flat());
            let mut dir = [0.0f32; 3];
            if keys::down(input, Action::Forward) {
                dir = v_add(dir, fwd);
            }
            if keys::down(input, Action::Back) {
                dir = v_sub(dir, fwd);
            }
            if keys::down(input, Action::Left) {
                dir = v_sub(dir, right);
            }
            if keys::down(input, Action::Right) {
                dir = v_add(dir, right);
            }
            let len = v_len(dir);
            if len > 1e-4 {
                self.focus = None; // 걸어가기 시작하면 자동 포커싱은 멈춘다
                self.zoom_goal = 0.0;
                let sprint = keys::down(input, Action::Sprint);
                let speed = if sprint { MOVE_SPEED * SPRINT_MULT } else { MOVE_SPEED };
                let step = v_scale(dir, speed * dt / len);
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

#[cfg(test)]
mod tests {
    use miniquad::KeyCode;

    use super::*;

    // 1초 동안 W 를 누르고 앞으로 걸은 거리(shift 면 달린다). 바닥이 없어 떨어지지만 수평 이동은 그대로다.
    fn walked(shift: bool) -> f32 {
        let mut input = Input::default();
        input.on_key_down(KeyCode::W, false);
        if shift {
            input.on_key_down(KeyCode::LeftShift, false);
        }
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        for _ in 0..60 {
            p.update(&input, true, 1.0 / 60.0, &[]);
        }
        p.pose()[2].abs()
    }

    #[test]
    fn holding_shift_runs_faster() {
        let (walk, run) = (walked(false), walked(true));
        assert!((walk - MOVE_SPEED).abs() < 0.05, "걷는 거리: {walk}");
        assert!((run - MOVE_SPEED * SPRINT_MULT).abs() < 0.05, "달린 거리: {run}");
    }

    #[test]
    fn shift_alone_does_not_move() {
        let mut input = Input::default();
        input.on_key_down(KeyCode::LeftShift, false);
        let mut p = Player::at([1.0, 0.0, 2.0], 0.0);
        for _ in 0..30 {
            p.update(&input, true, 1.0 / 60.0, &[]);
        }
        assert_eq!((p.pose()[0], p.pose()[2]), (1.0, 2.0));
    }

    #[test]
    fn an_unfocused_window_ignores_sprint() {
        let mut input = Input::default();
        input.on_key_down(KeyCode::W, false);
        input.on_key_down(KeyCode::LeftShift, false);
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        p.update(&input, false, 1.0, &[]);
        assert_eq!(p.pose()[2], 0.0);
    }

    fn cam_at(pos: [f32; 3]) -> Camera {
        Camera { pos, yaw: 0.0, pitch: 0.0 } // yaw 0 은 -Z 쪽을 본다
    }

    #[test]
    fn straight_ahead_within_range_is_aimed_and_far_or_behind_is_not() {
        let cam = cam_at([0.0, 0.0, 0.0]);
        assert!(aim_score(&cam, [0.0, 0.0, -2.0]).is_some());
        assert!(aim_score(&cam, [0.0, 0.0, -(AIM_MAX_DIST + 0.5)]).is_none(), "너무 멀다");
        assert!(aim_score(&cam, [0.0, 0.0, 2.0]).is_none(), "등 뒤다");
        assert!(aim_score(&cam, [4.0, 0.0, 0.0]).is_none(), "옆으로 한참 벗어났다");
    }

    #[test]
    fn up_close_the_aim_is_more_forgiving() {
        let cam = cam_at([0.0, 0.0, 0.0]);
        // 옆으로 0.5m 벗어난 물건 — 1m 앞에서는 약 27° 라 예전 판정(18°)으로는 못 맞췄지만 지금은 맞는다.
        assert!(aim_score(&cam, [0.5, 0.0, -1.0]).is_some());
        // 같은 0.5m 어긋남이라도 3m 앞에서는 약 9.5° 라 원래도 맞는다.
        assert!(aim_score(&cam, [0.5, 0.0, -3.0]).is_some());
        // 하지만 터무니없이 어긋난 건(1m 앞, 옆 1.5m) 여전히 안 맞는다.
        assert!(aim_score(&cam, [1.5, 0.0, -1.0]).is_none());
    }

    #[test]
    fn the_best_aimed_target_wins_not_the_nearest() {
        let cam = cam_at([0.0, 0.0, 0.0]);
        let centered_far = aim_score(&cam, [0.0, 0.0, -2.8]).unwrap();
        let off_center_near = aim_score(&cam, [0.45, 0.0, -1.0]).unwrap();
        assert!(centered_far < off_center_near, "조준선 한가운데 있는 먼 물건이, 가깝지만 비껴 있는 물건보다 우선이다");
    }

    fn forward_of(p: &Player) -> [f32; 3] {
        p.camera().forward()
    }

    fn settle(p: &mut Player) {
        for _ in 0..120 {
            p.update_focus(1.0 / 60.0, false);
        }
    }

    #[test]
    fn focusing_turns_the_camera_toward_the_target_and_then_stops() {
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0); // -Z 쪽을 보고 서 있다
        let eye_y = p.camera().pos[1];
        p.focus_on([4.0, eye_y, 0.0]); // 동쪽(+X)
        p.update_focus(1.0 / 60.0, false);
        assert!(p.is_focusing(), "한 프레임으로는 다 안 돈다");
        settle(&mut p);
        let f = forward_of(&p);
        assert!((f[0] - 1.0).abs() < 0.01 && f[2].abs() < 0.01, "동쪽을 본다: {f:?}");
        assert!(!p.is_focusing(), "다 돌면 포커스가 풀린다");
        // 풀린 뒤에는 시점이 그대로 남는다.
        p.update_focus(1.0 / 60.0, false);
        assert!((forward_of(&p)[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn focusing_looks_up_and_down_but_never_past_the_pitch_limit() {
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        let eye = p.camera().pos;
        p.focus_on([0.0, eye[1] + 2.0, -2.0]); // 앞쪽 위
        settle(&mut p);
        assert!(forward_of(&p)[1] > 0.5, "위를 본다");
        p.focus_on([0.0, eye[1] + 50.0, -0.01]); // 거의 머리 바로 위
        settle(&mut p);
        let pitch = forward_of(&p)[1].asin();
        assert!(pitch <= MAX_PITCH + 1e-3, "시점 한계를 넘지 않는다: {pitch}");
    }

    #[test]
    fn the_camera_takes_the_short_way_around() {
        // 거의 한 바퀴 돌아 있는 시점(yaw ≈ 3)에서 반대편(yaw ≈ -3 과 같은 방향)을 볼 때 긴 쪽으로 빙 돌지 않는다.
        let mut p = Player::at([0.0, 0.0, 0.0], 3.0);
        let eye = p.camera().pos;
        let want = (-3.0f32).sin_cos(); // yaw -3 일 때 시선 = [-sin(-3), ., -cos(-3)]
        p.focus_on([eye[0] - want.0 * 5.0, eye[1], eye[2] - want.1 * 5.0]);
        p.update_focus(1.0 / 60.0, false);
        let moved = (p.pose()[3] - 3.0).abs();
        assert!(moved < 0.2, "짧은 쪽(약 0.28 라디안 차이)으로 돈다: {moved}");
    }

    #[test]
    fn walking_or_looking_cancels_the_focus_and_a_target_underfoot_is_ignored() {
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        let eye = p.camera().pos;
        p.focus_on([3.0, eye[1], 0.0]);
        p.look(0.1, 0.0);
        assert!(!p.is_focusing(), "직접 시점을 돌리면 풀린다");
        p.focus_on([3.0, eye[1], 0.0]);
        let mut input = Input::default();
        input.on_key_down(KeyCode::W, false);
        p.update(&input, true, 1.0 / 60.0, &[]);
        assert!(!p.is_focusing(), "걸어가면 풀린다");
        p.focus_on(p.camera().pos);
        p.update_focus(1.0 / 60.0, false);
        assert!(!p.is_focusing(), "제 자리에 있는 점은 바라볼 방향이 없다");
    }

    #[test]
    fn far_targets_zoom_in_a_little_and_near_ones_do_not_then_it_eases_back() {
        let far = |dist: f32| {
            let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
            let eye = p.camera().pos;
            p.focus_on([dist, eye[1], 0.0]);
            for _ in 0..180 {
                p.update_focus(1.0 / 60.0, true); // 대화가 떠 있는 동안은 확대를 유지한다
            }
            p
        };
        assert_eq!(far(1.0).zoom(), 0.0, "가까우면 확대 없이 돌아서기만 한다");
        let mid = far(2.5).zoom();
        let edge = far(3.2).zoom();
        assert!(mid > 0.0 && edge > mid, "멀수록 더 당긴다: {mid} < {edge}");
        assert!((edge - ZOOM_MAX).abs() < 0.01 && edge <= ZOOM_MAX + 1e-3, "한계를 넘지 않는다: {edge}");
        // 한 프레임에 확 당겨지지 않고 부드럽게 다가간다.
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        let eye = p.camera().pos;
        p.focus_on([3.2, eye[1], 0.0]);
        p.update_focus(1.0 / 60.0, true);
        assert!(p.zoom() > 0.0 && p.zoom() < ZOOM_MAX * 0.5, "첫 프레임: {}", p.zoom());
        // 대화가 끝나면(hold=false) 원래 시야로 돌아온다.
        for _ in 0..240 {
            p.update_focus(1.0 / 60.0, true);
        }
        assert!(p.zoom() > ZOOM_MAX * 0.9);
        for _ in 0..240 {
            p.update_focus(1.0 / 60.0, false);
        }
        assert_eq!(p.zoom(), 0.0, "다시 기본 시야로");
    }

    #[test]
    fn walking_or_looking_drops_the_zoom() {
        let mut p = Player::at([0.0, 0.0, 0.0], 0.0);
        let eye = p.camera().pos;
        p.focus_on([3.2, eye[1], 0.0]);
        for _ in 0..120 {
            p.update_focus(1.0 / 60.0, true);
        }
        assert!(p.zoom() > 0.1);
        p.look(0.05, 0.0); // 직접 시점을 돌렸다 — hold 가 true 여도 확대 목표는 사라진다
        for _ in 0..240 {
            p.update_focus(1.0 / 60.0, true);
        }
        assert_eq!(p.zoom(), 0.0);
    }
}

