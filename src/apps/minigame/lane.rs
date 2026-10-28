//! BRAIN_DRIVE — 금고의 방에서 받는 3차선 아케이드 레이싱(시트 SUB B+b-1 ~ 4). 자동으로 앞으로 달리고(20m/s),
//! A·D(또는 ←→)로 차선을 하나씩 옮기고, S(또는 ↓)로 브레이크(10m/s)를 건다. 다른 차와 상자에 부딪히면 2초 멈추고
//! 충돌 1회로 센다(부딪힌 장애물은 사라진다). 60초 안에 1,000m 를 가면 완주, 충돌이 3회가 되면 실패
//! (같은 순간에 겹쳐 3회가 돼도 실패). 장애물은 세 차선을 한꺼번에 막지 않는다.

use crate::random::Rng;

use super::{Controls, Outcome};

pub const LANES: usize = 3;
pub const GOAL_M: f32 = 1000.0;
pub const LIMIT_SECS: f32 = 60.0;
pub const COUNTDOWN_SECS: f32 = 3.0;
pub const BASE_SPEED: f32 = 20.0; // m/s
pub const BRAKE_SPEED: f32 = 10.0;
pub const CRASH_STOP_SECS: f32 = 2.0;
pub const MAX_CRASHES: u32 = 3;
const ROW_GAP: (f32, f32) = (45.0, 75.0); // 장애물 줄 사이 간격(m)
const FIRST_ROW_AT: f32 = 90.0;
const HIT_DEPTH: f32 = 2.0; // 이 거리(m) 안에서 같은 차선이면 부딪힌다
const LANE_SLIDE: f32 = 12.0; // 화면에서 차선 사이를 미끄러지는 속도(차선/초)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObstacleKind {
    Car,
    Crate,
}

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub at: f32, // 출발선에서의 거리(m)
    pub lane: usize,
    pub kind: ObstacleKind,
    pub alive: bool,
}

pub struct LaneRound {
    pub t: f32, // 시작 신호 뒤로 흐른 시간(카운트다운 동안은 음수)
    pub dist: f32,
    pub lane: usize,
    pub lane_pos: f32, // 화면에 그리는 차선 위치(부드럽게 따라간다)
    pub crashes: u32,
    pub stop_left: f32, // 충돌 뒤 멈춰 있는 남은 시간
    pub braking: bool,
    pub obstacles: Vec<Obstacle>,
    prev_dx: f32,
}

impl LaneRound {
    pub fn new(mut rng: Rng) -> LaneRound {
        let mut obstacles = Vec::new();
        let mut at = FIRST_ROW_AT;
        while at < GOAL_M - 30.0 {
            // 한 줄에 1~2개 — 세 차선을 다 막지 않는다.
            let count = if rng.unit() < 0.35 { 2 } else { 1 };
            let first = (rng.unit() * LANES as f32) as usize % LANES;
            let mut lanes = vec![first];
            if count == 2 {
                lanes.push((first + 1 + (rng.unit() * 2.0) as usize % 2) % LANES);
            }
            for lane in lanes {
                let kind = if rng.unit() < 0.5 { ObstacleKind::Car } else { ObstacleKind::Crate };
                obstacles.push(Obstacle { at, lane, kind, alive: true });
            }
            at += rng.range_f32(ROW_GAP.0, ROW_GAP.1);
        }
        LaneRound { t: -COUNTDOWN_SECS, dist: 0.0, lane: 1, lane_pos: 1.0, crashes: 0, stop_left: 0.0, braking: false, obstacles, prev_dx: 0.0 }
    }

    // 카운트다운이 남은 시간(0 이하면 출발했다).
    pub fn countdown(&self) -> f32 {
        -self.t
    }

    pub fn time_left(&self) -> f32 {
        (LIMIT_SECS - self.t.max(0.0)).max(0.0)
    }

    pub fn speed(&self) -> f32 {
        if self.stop_left > 0.0 {
            0.0
        } else if self.braking {
            BRAKE_SPEED
        } else {
            BASE_SPEED
        }
    }

    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        // 차선 표시는 카운트다운 중에도 따라간다.
        let target = self.lane as f32;
        let d = target - self.lane_pos;
        self.lane_pos += d.clamp(-LANE_SLIDE * dt, LANE_SLIDE * dt);

        self.t += dt;
        if self.t < 0.0 {
            self.prev_dx = c.dx;
            return Outcome::Running;
        }
        // 차선 변경 — 누르는 순간 한 칸씩(길게 눌러도 한 번).
        if c.dx != 0.0 && self.prev_dx == 0.0 {
            let next = self.lane as i32 + c.dx.signum() as i32;
            self.lane = next.clamp(0, LANES as i32 - 1) as usize;
        }
        self.prev_dx = c.dx;
        self.braking = c.brake;

        let before = self.dist;
        if self.stop_left > 0.0 {
            self.stop_left = (self.stop_left - dt).max(0.0);
        } else {
            self.dist += self.speed() * dt;
        }

        // 지나온 구간(before..dist)에서 같은 차선의 장애물에 부딪혔는지 — 한 번에 여러 개도 센다.
        let lane = self.lane;
        let mut hit = 0;
        for o in &mut self.obstacles {
            if o.alive && o.lane == lane && o.at + HIT_DEPTH >= before && o.at - HIT_DEPTH <= self.dist {
                o.alive = false;
                hit += 1;
            }
        }
        if hit > 0 {
            self.crashes += hit;
            self.stop_left = CRASH_STOP_SECS;
        }

        if self.crashes >= MAX_CRASHES {
            return Outcome::Lost;
        }
        if self.dist >= GOAL_M {
            return Outcome::Won;
        }
        if self.t >= LIMIT_SECS {
            return Outcome::Lost;
        }
        Outcome::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(r: &mut LaneRound, secs: f32) -> Outcome {
        let mut out = Outcome::Running;
        let mut left = secs;
        while left > 0.0 && out == Outcome::Running {
            out = r.step(0.01, &Controls::default());
            left -= 0.01;
        }
        out
    }

    // 장애물 없는 길 — 시험에서 충돌 없이 달리는 걸 다룬다.
    fn clear_road() -> LaneRound {
        let mut r = LaneRound::new(Rng::new(7));
        r.obstacles.clear();
        r
    }

    #[test]
    fn it_waits_out_the_countdown_then_runs_at_twenty_meters_per_second() {
        let mut r = clear_road();
        run(&mut r, COUNTDOWN_SECS - 0.1);
        assert_eq!(r.dist, 0.0, "카운트다운 동안은 달리지 않는다");
        run(&mut r, 0.1 + 5.0);
        assert!((r.dist - 100.0).abs() < 1.5, "5초 동안 약 100m: {}", r.dist);
    }

    #[test]
    fn a_clean_run_finishes_inside_the_time_limit() {
        let mut r = clear_road();
        assert_eq!(run(&mut r, COUNTDOWN_SECS + LIMIT_SECS), Outcome::Won, "1,000m / 20m/s = 50초");
        assert!(r.t < LIMIT_SECS);
    }

    #[test]
    fn braking_all_the_way_runs_out_of_time() {
        let mut r = clear_road();
        let mut out = Outcome::Running;
        for _ in 0..((COUNTDOWN_SECS + LIMIT_SECS + 1.0) / 0.01) as usize {
            out = r.step(0.01, &Controls { brake: true, ..Controls::default() });
            if out != Outcome::Running {
                break;
            }
        }
        assert_eq!(out, Outcome::Lost, "10m/s 로는 60초에 600m 뿐이다");
        assert!(r.dist < GOAL_M);
    }

    #[test]
    fn lanes_change_one_step_per_press() {
        let mut r = clear_road();
        run(&mut r, COUNTDOWN_SECS);
        let press = |r: &mut LaneRound, dx: f32| {
            r.step(0.01, &Controls { dx, ..Controls::default() });
        };
        press(&mut r, -1.0);
        for _ in 0..30 {
            press(&mut r, -1.0); // 계속 누르고 있어도
        }
        assert_eq!(r.lane, 0, "한 번 눌렀을 때 한 칸만 움직인다");
        press(&mut r, 0.0);
        press(&mut r, -1.0);
        assert_eq!(r.lane, 0, "맨 왼쪽 차선 밖으로는 못 나간다");
        press(&mut r, 0.0);
        press(&mut r, 1.0);
        assert_eq!(r.lane, 1);
    }

    #[test]
    fn a_crash_stops_the_car_for_two_seconds_and_removes_the_obstacle() {
        let mut r = clear_road();
        r.obstacles.push(Obstacle { at: 20.0, lane: 1, kind: ObstacleKind::Crate, alive: true });
        run(&mut r, COUNTDOWN_SECS + 1.2);
        assert_eq!(r.crashes, 1);
        assert!(r.stop_left > 1.5 && r.speed() == 0.0);
        assert!(!r.obstacles[0].alive);
        let dist = r.dist;
        run(&mut r, 1.0);
        assert_eq!(r.dist, dist, "멈춰 있는 동안은 앞으로 안 간다");
        run(&mut r, 1.5);
        assert!(r.dist > dist, "2초 뒤에는 다시 출발한다");
    }

    #[test]
    fn three_crashes_fail_even_when_they_overlap_in_one_frame() {
        let mut r = clear_road();
        for k in 0..3 {
            let _ = k;
            r.obstacles.push(Obstacle { at: 20.0, lane: 1, kind: ObstacleKind::Car, alive: true });
        }
        assert_eq!(run(&mut r, COUNTDOWN_SECS + 3.0), Outcome::Lost);
        assert_eq!(r.crashes, 3);
    }

    #[test]
    fn a_finish_needs_at_most_two_crashes() {
        let mut r = clear_road();
        r.obstacles.push(Obstacle { at: 100.0, lane: 1, kind: ObstacleKind::Car, alive: true });
        r.obstacles.push(Obstacle { at: 300.0, lane: 1, kind: ObstacleKind::Car, alive: true });
        assert_eq!(run(&mut r, COUNTDOWN_SECS + LIMIT_SECS), Outcome::Won, "50초 + 멈춘 4초 = 54초");
        assert_eq!(r.crashes, 2);
    }

    #[test]
    fn no_row_ever_blocks_all_three_lanes() {
        for seed in 0..40u64 {
            let r = LaneRound::new(Rng::new(seed * 977 + 3));
            let mut rows: Vec<(f32, Vec<usize>)> = Vec::new();
            for o in &r.obstacles {
                match rows.last_mut() {
                    Some((at, lanes)) if *at == o.at => lanes.push(o.lane),
                    _ => rows.push((o.at, vec![o.lane])),
                }
            }
            assert!(!rows.is_empty());
            for (at, lanes) in rows {
                let mut l = lanes.clone();
                l.sort();
                l.dedup();
                assert!(l.len() == lanes.len() && l.len() < LANES, "seed {seed}: {at}m 줄이 길을 다 막았다 {lanes:?}");
            }
        }
    }
}
