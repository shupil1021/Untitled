//! 카트 한 대의 물리 — 가속/브레이크/핸들, 가드레일 충돌, 지금 어디쯤인지(바퀴 수와 진행 거리).

use super::ai::Ai;
use super::track::{forward, wrap_angle, Track};
use super::{ACCEL, BRAKE, DRAG, LAPS, MAX_SPEED, STEER_RATE, WALL_INSET};

#[derive(Clone)]
pub struct Kart {
    pub pos: [f32; 2],
    pub heading: f32,
    pub speed: f32,
    pub(super) route: usize,
    pub(super) idx: usize,
    pub(super) last_p: f32,
    pub(super) laps: i32, // 결승선을 넘은 횟수 - 1 (출발선 뒤에서 시작해서 -1)
    pub progress: f32, // 바퀴 수 * 길이 + 큰길 기준 진행 거리 — 순위를 매기는 값
    pub(super) finish_time: Option<f32>,
    pub(super) bump_cooldown: f32,
    pub(super) ai: Option<Ai>,
}

// 한 프레임의 조작(사람이든 AI 든 같은 형태).
#[derive(Clone, Copy, Default)]
pub(super) struct Drive {
    pub(super) gas: bool,
    pub(super) brake: bool,
    pub(super) steer: f32, // + 가 왼쪽
}

impl Kart {
    pub(super) fn new(track: &Track, slot: usize, ai: Option<Ai>) -> Kart {
        // 출발선 뒤에 2열로 선다(큰길 위).
        let main = &track.routes[0];
        let m = main.len();
        let row = slot / 2;
        let idx = (m - 4 - row * 3) % m;
        let side = if slot.is_multiple_of(2) { 2.2 } else { -2.2 };
        let t = main.tang[idx];
        let pos = [main.pts[idx][0] + t[1] * side, main.pts[idx][1] - t[0] * side];
        let loc = track.locate(pos);
        Kart { pos, heading: main.heading_at(idx), speed: 0.0, route: 0, idx, last_p: loc.progress, laps: -1, progress: loc.progress - track.length, finish_time: None, bump_cooldown: 0.0, ai }
    }

    pub fn lap_number(&self) -> i32 {
        (self.laps + 1).clamp(1, LAPS)
    }

    pub(super) fn step(&mut self, track: &Track, d: Drive, dt: f32) {
        let mut cap = MAX_SPEED;
        if let Some(ai) = &self.ai {
            cap *= ai.eff.max(0.5);
        }
        if d.gas {
            self.speed += ACCEL * dt;
        } else if self.speed > 0.0 {
            self.speed = (self.speed - DRAG * dt).max(0.0);
        } else {
            self.speed = (self.speed + DRAG * dt).min(0.0);
        }
        if d.brake {
            self.speed -= BRAKE * dt;
            self.speed = self.speed.max(-6.0);
        }
        if self.speed > cap {
            self.speed = (self.speed - 30.0 * dt).max(cap);
        }

        let grip = (self.speed.abs() / 7.0).min(1.0);
        let rate = STEER_RATE * grip * if self.speed < 0.0 { -1.0 } else { 1.0 };
        self.heading += d.steer * rate * dt;
        let f = forward(self.heading);
        self.pos[0] += f[0] * self.speed * dt;
        self.pos[1] += f[1] * self.speed * dt;

        // 가드레일 — 안쪽으로 밀어 넣고 속도를 크게 깎는다.
        self.bump_cooldown = (self.bump_cooldown - dt).max(0.0);
        let loc = track.locate(self.pos);
        let route = &track.routes[loc.route];
        let wall = route.half_w - WALL_INSET;
        if loc.lateral.abs() > wall {
            let t = route.tang[loc.idx];
            let n = [t[1], -t[0]]; // 왼쪽 법선
            let push = loc.lateral - loc.lateral.signum() * wall;
            self.pos[0] -= n[0] * push;
            self.pos[1] -= n[1] * push;
            if self.bump_cooldown <= 0.0 {
                self.speed *= 0.55;
                self.bump_cooldown = 0.4;
            }
            // 비스듬히 박혀 못 빠져나오지 않게 길 방향으로 살짝 돌려놓는다.
            let diff = wrap_angle(route.heading_at(loc.idx) - self.heading);
            self.heading += diff * (4.0 * dt).min(1.0);
        }

        // 지금 어디쯤인지 — 큰길 기준 진행 거리가 끝에서 처음으로 넘어가면(결승선) 바퀴가 늘고, 거꾸로면 줄어든다.
        let p = loc.progress;
        if self.last_p > track.length * 0.7 && p < track.length * 0.3 {
            self.laps += 1;
        } else if self.last_p < track.length * 0.3 && p > track.length * 0.7 {
            self.laps -= 1;
        }
        self.last_p = p;
        self.route = loc.route;
        self.idx = loc.idx;
        self.progress = self.laps as f32 * track.length + p;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random::Rng;
    use super::super::DriveRound;

    #[test]
    fn the_guard_rail_holds_the_kart_in_and_slows_it() {
        let r = DriveRound::new(Rng::new(5));
        let t = &r.track;
        let main = &t.routes[0];
        let i = 20;
        let mut k = Kart::new(t, 0, None);
        k.pos = main.pts[i];
        k.heading = main.heading_at(i) + std::f32::consts::FRAC_PI_2; // 길과 직각으로 왼쪽
        let mut slowed = false;
        for _ in 0..600 {
            k.step(t, Drive { gas: true, ..Drive::default() }, 0.016);
            slowed |= k.speed < 12.0;
        }
        let loc = t.locate(k.pos);
        assert!(loc.lateral.abs() <= t.routes[loc.route].half_w - WALL_INSET + 0.5, "가드레일 밖으로 나갔다: {}", loc.lateral);
        assert!(slowed, "부딪히면 속도가 줄어든다");
    }

    // AI 는 레이스마다 다르게 달린다: 1등으로 들어오는 카트가 매번 같지 않고, 한 레이스 안에서도 카트들의 기록이 제각각이다.
}
