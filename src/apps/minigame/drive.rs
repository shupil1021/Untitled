//! BRAIN_DRIVE — 뇌의 방에서 받는 3차선 아케이드 레이싱(시트 SUB B+b-1 ~ 5). 자동 전진, 좌우로
//! 차선을 바꾸고 S/아래 키로 브레이크. 60초 안에 1,000m 를 가면 클리어, 충돌하면 2초 멈추고
//! 3번 부딪히면 실패(시간이 다 돼도 실패).

use crate::random::Rng;

use super::{Controls, Outcome};

pub const GOAL_M: f32 = 1000.0;
pub const LIMIT_SECS: f32 = 60.0;
pub const MAX_CRASHES: u32 = 3;
const SPEED: f32 = 20.0; // m/s
const BRAKE_SPEED: f32 = 10.0;
const STUN_SECS: f32 = 2.0;
const LANE_COOLDOWN: f32 = 0.3; // 차선을 바꾼 뒤 다음 변경까지(한 번에 두 칸을 휙 못 건넌다)
const HIT_AHEAD: f32 = 1.2; // 같은 차선에서 앞쪽으로 이 거리 안이면 충돌
const HIT_BEHIND: f32 = 0.4; // 이미 거의 지나친 장애물은 (차선을 바꿔 들어와도) 거의 안 부딪힌다
const VIEW_AHEAD: f32 = 45.0; // 이만큼 앞까지 미리 깔아 둔다

#[derive(Clone, Copy)]
pub struct Obstacle {
    pub lane: usize,
    pub s: f32, // 출발점에서 몇 m 지점인지
}

pub struct DriveRound {
    t: f32,
    pub dist: f32,
    pub lane: usize,
    pub lane_pos: f32, // 부드럽게 따라가는 화면용 차선 위치(0..2)
    crashes: u32,
    stun: f32,
    pub obstacles: Vec<Obstacle>,
    next_row: f32,
    cooldown: f32,
    rng: Rng,
}

impl DriveRound {
    pub fn new(rng: Rng) -> DriveRound {
        DriveRound { t: 0.0, dist: 0.0, lane: 1, lane_pos: 1.0, crashes: 0, stun: 0.0, obstacles: Vec::new(), next_row: 40.0, cooldown: 0.0, rng }
    }

    pub fn time(&self) -> f32 {
        self.t
    }

    pub fn crashes(&self) -> u32 {
        self.crashes
    }

    pub fn stunned(&self) -> bool {
        self.stun > 0.0
    }

    // 한 줄에 차선 하나는 항상 비워 둔다.
    fn spawn_rows(&mut self) {
        while self.next_row < self.dist + VIEW_AHEAD && self.next_row < GOAL_M + 10.0 {
            let free = (self.rng.next_u32() % 3) as usize;
            for lane in 0..3 {
                if lane != free && self.rng.unit() < 0.7 {
                    self.obstacles.push(Obstacle { lane, s: self.next_row });
                }
            }
            self.next_row += self.rng.range_f32(8.0, 14.0);
        }
    }

    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        self.t += dt;
        self.cooldown = (self.cooldown - dt).max(0.0);
        if self.cooldown <= 0.0 {
            if c.lane_left && self.lane > 0 {
                self.lane -= 1;
                self.cooldown = LANE_COOLDOWN;
            } else if c.lane_right && self.lane < 2 {
                self.lane += 1;
                self.cooldown = LANE_COOLDOWN;
            }
        }
        // 화면용 차선 위치는 목표를 향해 빠르게 따라간다.
        let target = self.lane as f32;
        self.lane_pos += (target - self.lane_pos) * (1.0 - (-14.0 * dt).exp());

        if self.stun > 0.0 {
            self.stun = (self.stun - dt).max(0.0);
        } else {
            let speed = if c.brake { BRAKE_SPEED } else { SPEED };
            self.dist += speed * dt;
        }
        self.spawn_rows();
        let dist = self.dist;
        self.obstacles.retain(|o| o.s > dist - 12.0);

        if self.stun <= 0.0 {
            let lane = self.lane;
            if let Some(i) = self.obstacles.iter().position(|o| o.lane == lane && (-HIT_BEHIND..HIT_AHEAD).contains(&(o.s - dist))) {
                self.obstacles.swap_remove(i);
                self.crashes += 1;
                self.stun = STUN_SECS;
            }
        }

        if self.crashes >= MAX_CRASHES {
            Outcome::Lost
        } else if self.dist >= GOAL_M {
            Outcome::Won
        } else if self.t >= LIMIT_SECS {
            Outcome::Lost
        } else {
            Outcome::Running
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 다음 줄에서 비어 있는 차선으로 미리 옮겨 가는 봇.
    fn dodge(r: &DriveRound) -> Controls {
        let next = r.obstacles.iter().filter(|o| o.s > r.dist).map(|o| o.s).fold(f32::MAX, f32::min);
        let blocked = |lane: usize| r.obstacles.iter().any(|o| o.lane == lane && (o.s - next).abs() < 0.1);
        let mut c = Controls::default();
        // 옆 차선으로 들어서는 순간 그 자리에 장애물이 걸쳐 있으면(지나친 직후 포함) 부딪히니 비켜 있을 때만 옮긴다.
        let clear_now = |lane: usize| !r.obstacles.iter().any(|o| o.lane == lane && (-1.0..2.0).contains(&(o.s - r.dist)));
        if next - r.dist < 22.0 && blocked(r.lane) {
            let free = (0..3).find(|&l| !blocked(l)).unwrap();
            if free < r.lane && clear_now(r.lane - 1) {
                c.lane_left = true;
            } else if free > r.lane && clear_now(r.lane + 1) {
                c.lane_right = true;
            }
        }
        c
    }

    #[test]
    fn a_careful_driver_finishes_in_time() {
        let mut r = DriveRound::new(Rng::new(11));
        let mut out = Outcome::Running;
        for _ in 0..(LIMIT_SECS / 0.016) as usize {
            let c = dodge(&r);
            out = r.step(0.016, &c);
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        assert!(matches!(out, Outcome::Won), "crashes={} dist={}", r.crashes(), r.dist);
        assert!(r.crashes() <= 2);
    }

    #[test]
    fn driving_straight_into_traffic_fails() {
        let mut r = DriveRound::new(Rng::new(5));
        let mut out = Outcome::Running;
        for _ in 0..(LIMIT_SECS / 0.016) as usize + 10 {
            out = r.step(0.016, &Controls::default());
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        assert!(matches!(out, Outcome::Lost));
    }

    #[test]
    fn crash_stops_the_car_for_two_seconds_and_braking_slows_it() {
        let mut r = DriveRound::new(Rng::new(2));
        r.obstacles.push(Obstacle { lane: 1, s: 1.0 });
        r.next_row = 1.0e9;
        for _ in 0..200 {
            r.step(0.016, &Controls::default());
            if r.stunned() {
                break;
            }
        }
        assert_eq!(r.crashes(), 1);
        let d0 = r.dist;
        r.step(1.0, &Controls::default());
        assert_eq!(r.dist, d0, "멈춰 있는 동안은 안 간다");

        let mut slow = DriveRound::new(Rng::new(2));
        let mut fast = DriveRound::new(Rng::new(2));
        slow.next_row = 1.0e9;
        fast.next_row = 1.0e9;
        slow.step(1.0, &Controls { brake: true, ..Controls::default() });
        fast.step(1.0, &Controls::default());
        assert!((slow.dist - 10.0).abs() < 1e-3 && (fast.dist - 20.0).abs() < 1e-3);
    }

    // 장애물이 가까워진(look 미터 안) 뒤에야 피하는 봇이 몇 판 중 몇 판 이기는지.
    fn win_rate(look: f32, brake_when_blocked: bool) -> usize {
        (1..=40u64)
            .filter(|&seed| {
                let mut r = DriveRound::new(Rng::new(seed));
                let mut out = Outcome::Running;
                for _ in 0..(LIMIT_SECS / 0.016) as usize + 5 {
                    let next = r.obstacles.iter().filter(|o| o.s > r.dist).map(|o| o.s).fold(f32::MAX, f32::min);
                    let blocked = |lane: usize| r.obstacles.iter().any(|o| o.lane == lane && (o.s - next).abs() < 0.1);
                    let near = |lane: usize| r.obstacles.iter().any(|o| o.lane == lane && (-1.0..2.0).contains(&(o.s - r.dist)));
                    let mut c = Controls::default();
                    if next - r.dist < look && blocked(r.lane) {
                        let free = (0..3).find(|&l| !blocked(l)).unwrap();
                        if free < r.lane && !near(r.lane - 1) {
                            c.lane_left = true;
                        } else if free > r.lane && !near(r.lane + 1) {
                            c.lane_right = true;
                        }
                        c.brake = brake_when_blocked;
                    }
                    out = r.step(0.016, &c);
                    if !matches!(out, Outcome::Running) {
                        break;
                    }
                }
                matches!(out, Outcome::Won)
            })
            .count()
    }

    // 미리 보고 피하면 이기고, 늦게 반응하면 지지만, 늦더라도 브레이크로 시간을 벌면 다시 이길 수 있다.
    #[test]
    fn difficulty_rewards_planning_and_braking() {
        let careful = win_rate(24.0, false);
        let late = win_rate(7.0, false);
        let late_braking = win_rate(8.0, true);
        assert!(careful >= 32, "미리 보고 피하면 대부분 이긴다: {careful}/40");
        assert!(late <= 12, "늦게 반응하면 자주 진다: {late}/40");
        assert!(late_braking >= 32, "브레이크로 시간을 벌면 이길 수 있다: {late_braking}/40");
    }
}
