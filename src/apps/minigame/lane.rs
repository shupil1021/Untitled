//! BRAIN_DRIVE — 금고의 방에서 받는 3차선 아케이드 레이싱(시트 SUB B+b-1 ~ 4). 자동으로 앞으로 달리고(20m/s),
//! A·D(또는 ←→)로 차선을 하나씩 옮기고, S(또는 ↓)로 브레이크(10m/s)를 건다. 다른 차와 상자에 부딪히면 2초 멈추고
//! 충돌 1회로 센다(부딪힌 장애물은 사라진다). 60초 안에 1,000m 를 가면 완주, 충돌이 3회가 되면 실패
//! (같은 순간에 겹쳐 3회가 돼도 실패). 장애물 줄은 세 차선을 한꺼번에 막지 않는다. 뒤로 갈수록 줄 간격이 좁아지고,
//! 가끔 옆에서 도로를 가로지르는 차가 나온다(다가가면 가로지르기 시작한다) — 차선을 옮기거나 브레이크로 지나가길 기다린다.

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
const CROSS_TRIGGER: f32 = 38.0; // 가로지르는 차는 내가 이 거리(m) 안으로 다가오면 움직이기 시작한다
const CROSS_SPEED: f32 = 1.2; // 차선/초
const HIT_LANES: f32 = 0.55; // 내 차선과 장애물 위치가 이만큼(차선) 안이면 부딪힌다
const CROSS_FROM: (f32, f32) = (-1.0, 3.0); // 가로지르는 차가 출발하는 도로 밖 위치(왼쪽 / 오른쪽)
const LANE_SLIDE: f32 = 12.0; // 화면에서 차선 사이를 미끄러지는 속도(차선/초)

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObstacleKind {
    Car,
    Crate,
}

#[derive(Clone, Copy, Debug)]
pub struct Obstacle {
    pub at: f32, // 출발선에서의 거리(m)
    pub kind: ObstacleKind,
    pub alive: bool,
    pub pos: f32, // 도로 위 가로 위치(차선 단위) — 서 있는 장애물은 lane 과 같고, 가로지르는 차는 움직인다
    pub cross: Option<f32>, // Some(+1/-1) 이면 가로지르는 차(오른쪽/왼쪽으로)
}

impl Obstacle {
    pub fn fixed(at: f32, lane: usize, kind: ObstacleKind) -> Obstacle {
        Obstacle { at, kind, alive: true, pos: lane as f32, cross: None }
    }

    // 도로 옆에서 시작해 반대편으로 가로지르는 차.
    pub fn crossing(at: f32, rightwards: bool) -> Obstacle {
        let (pos, dir) = if rightwards { (CROSS_FROM.0, 1.0) } else { (CROSS_FROM.1, -1.0) };
        Obstacle { at, kind: ObstacleKind::Car, alive: true, pos, cross: Some(dir) }
    }
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
            let progress = at / GOAL_M;
            // 뒤로 갈수록 가로지르는 차가 늘어난다(한 줄에 그 차 하나만 둔다 — 다른 차선을 비워 둔다).
            if rng.unit() < 0.1 + 0.25 * progress {
                obstacles.push(Obstacle::crossing(at, rng.unit() < 0.5));
                at += rng.range_f32(ROW_GAP.0, ROW_GAP.1) * (1.0 - 0.35 * progress);
                continue;
            }
            // 한 줄에 1~2개 — 세 차선을 다 막지 않는다.
            let count = if rng.unit() < 0.35 { 2 } else { 1 };
            let first = (rng.unit() * LANES as f32) as usize % LANES;
            let mut lanes = vec![first];
            if count == 2 {
                lanes.push((first + 1 + (rng.unit() * 2.0) as usize % 2) % LANES);
            }
            for lane in lanes {
                let kind = if rng.unit() < 0.5 { ObstacleKind::Car } else { ObstacleKind::Crate };
                obstacles.push(Obstacle::fixed(at, lane, kind));
            }
            // 뒤로 갈수록 줄 간격이 최대 35% 까지 좁아진다.
            at += rng.range_f32(ROW_GAP.0, ROW_GAP.1) * (1.0 - 0.35 * progress);
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

        // 가로지르는 차는 내가 다가오면 반대편으로 움직인다(내가 멈춰 있는 동안도 계속 움직인다).
        let dist = self.dist;
        for o in &mut self.obstacles {
            if let Some(dir) = o.cross
                && dist >= o.at - CROSS_TRIGGER
            {
                o.pos = (o.pos + dir * CROSS_SPEED * dt).clamp(CROSS_FROM.0, CROSS_FROM.1);
            }
        }

        // 지나온 구간(before..dist)에서 같은 차선의 장애물에 부딪혔는지 — 한 번에 여러 개도 센다.
        let lane = self.lane;
        let mut hit = 0;
        for o in &mut self.obstacles {
            if o.alive && (o.pos - lane as f32).abs() < HIT_LANES && o.at + HIT_DEPTH >= before && o.at - HIT_DEPTH <= self.dist {
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
        r.obstacles.push(Obstacle::fixed(20.0, 1, ObstacleKind::Crate));
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
            r.obstacles.push(Obstacle::fixed(20.0, 1, ObstacleKind::Car));
        }
        assert_eq!(run(&mut r, COUNTDOWN_SECS + 3.0), Outcome::Lost);
        assert_eq!(r.crashes, 3);
    }

    #[test]
    fn a_finish_needs_at_most_two_crashes() {
        let mut r = clear_road();
        r.obstacles.push(Obstacle::fixed(100.0, 1, ObstacleKind::Car));
        r.obstacles.push(Obstacle::fixed(300.0, 1, ObstacleKind::Car));
        assert_eq!(run(&mut r, COUNTDOWN_SECS + LIMIT_SECS), Outcome::Won, "50초 + 멈춘 4초 = 54초");
        assert_eq!(r.crashes, 2);
    }

    #[test]
    fn no_row_ever_blocks_all_three_lanes() {
        for seed in 0..40u64 {
            let r = LaneRound::new(Rng::new(seed * 977 + 3));
            let mut rows: Vec<(f32, Vec<usize>)> = Vec::new();
            for o in &r.obstacles {
                if o.cross.is_some() {
                    continue; // 가로지르는 차는 한 줄에 혼자라 따로 본다
                }
                match rows.last_mut() {
                    Some((at, lanes)) if *at == o.at => lanes.push(o.pos as usize),
                    _ => rows.push((o.at, vec![o.pos as usize])),
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

    // 가로지르는 차: 다가가기 전엔 도로 밖에 있고, 다가오면 움직여서 내 차선을 지나간다.
    #[test]
    fn a_crossing_car_waits_offroad_then_sweeps_the_road() {
        let mut r = clear_road();
        r.obstacles.push(Obstacle::crossing(300.0, true));
        run(&mut r, COUNTDOWN_SECS + 1.0);
        assert_eq!(r.obstacles[0].pos, CROSS_FROM.0, "멀리 있을 땐 도로 밖에서 기다린다");
        run(&mut r, 8.0); // 160m 지점 — 아직 38m 밖
        assert_eq!(r.obstacles[0].pos, CROSS_FROM.0);
        run(&mut r, 5.5); // 290m 지점 — trigger(262m)를 지나 1.4초 움직였다
        assert!(r.obstacles[0].pos > CROSS_FROM.0 + 0.5, "다가오면 도로를 가로지른다: {}", r.obstacles[0].pos);
    }

    #[test]
    fn braking_lets_a_crossing_car_clear_the_road() {
        // 가로지르는 차는 내 차선(1)을 막지만, 브레이크로 늦게 도착하면 이미 반대편으로 빠져 있다.
        let drive = |brake_from: f32| {
            let mut r = clear_road();
            r.obstacles.push(Obstacle::crossing(100.0, true));
            for _ in 0..((COUNTDOWN_SECS + 12.0) / 0.01) as usize {
                let brake = r.dist >= brake_from && r.dist < 100.0 - 6.0;
                if r.step(0.01, &Controls { brake, ..Controls::default() }) != Outcome::Running {
                    break;
                }
            }
            r.crashes
        };
        assert_eq!(drive(f32::MAX), 1, "그냥 달리면 한가운데서 부딪힌다");
        assert_eq!(drive(62.0), 0, "다가오기 직전부터 브레이크를 밟으면 지나간 뒤에 도착한다");
    }

    #[test]
    fn later_rows_are_tighter_and_crossing_cars_appear() {
        let (mut gaps_early, mut gaps_late, mut crossing) = (Vec::new(), Vec::new(), 0);
        for seed in 0..40u64 {
            let r = LaneRound::new(Rng::new(seed * 131 + 5));
            let ats: Vec<f32> = r.obstacles.iter().map(|o| o.at).collect();
            for w in ats.windows(2).filter(|w| w[1] > w[0]) {
                if w[0] < 300.0 {
                    gaps_early.push(w[1] - w[0]);
                } else if w[0] > 700.0 {
                    gaps_late.push(w[1] - w[0]);
                }
            }
            crossing += r.obstacles.iter().filter(|o| o.cross.is_some()).count();
        }
        let avg = |v: &Vec<f32>| v.iter().sum::<f32>() / v.len() as f32;
        assert!(avg(&gaps_late) < avg(&gaps_early) * 0.85, "뒤쪽이 더 촘촘하다: {} vs {}", avg(&gaps_late), avg(&gaps_early));
        assert!(crossing > 0, "가로지르는 차가 나온다");
    }

    // 앞의 장애물이 내가 도착할 때 어느 차선에 있을지 보고 안전한 차선으로 옮기는 운전 봇 — 제대로 운전하면 이길 수 있어야 한다.
    fn driver(r: &LaneRound, frame: usize) -> Controls {
        let danger = |lane: usize| -> bool {
            r.obstacles.iter().any(|o| {
                if !o.alive || o.at < r.dist - HIT_DEPTH || o.at - r.dist > 55.0 {
                    return false;
                }
                let arrival = ((o.at - r.dist) / BASE_SPEED).max(0.0);
                let pred = match o.cross {
                    Some(dir) => {
                        let wait = ((o.at - CROSS_TRIGGER - r.dist) / BASE_SPEED).max(0.0);
                        (o.pos + dir * CROSS_SPEED * (arrival - wait).max(0.0)).clamp(CROSS_FROM.0, CROSS_FROM.1)
                    }
                    None => o.pos,
                };
                (pred - lane as f32).abs() < 0.75
            })
        };
        let here = r.lane;
        let mut order = vec![here];
        if here > 0 {
            order.push(here - 1);
        }
        if here + 1 < LANES {
            order.push(here + 1);
        }
        let target = order.into_iter().find(|&l| !danger(l)).unwrap_or(here);
        let want = if target == here { 0.0 } else { (target as f32 - here as f32).signum() }; // signum(0.0) 은 1.0 이라 따로 거른다
        Controls { dx: if frame % 2 == 0 { want } else { 0.0 }, ..Controls::default() }
    }

    #[test]
    fn a_careful_driver_finishes_most_runs() {
        let mut wins = 0;
        for seed in 0..40u64 {
            let mut r = LaneRound::new(Rng::new(seed * 7919 + 11));
            let mut out = Outcome::Running;
            for frame in 0..((COUNTDOWN_SECS + LIMIT_SECS + 1.0) / 0.01) as usize {
                let c = driver(&r, frame);
                out = r.step(0.01, &c);
                if out != Outcome::Running {
                    break;
                }
            }
            if out == Outcome::Won {
                wins += 1;
            }
        }
        assert!(wins >= 34, "제대로 운전하면 이길 수 있어야 한다: {wins}/40");
    }

    // 반대로 차선을 한 번도 안 바꾸고 달리기만 하면 거의 못 이긴다.
    #[test]
    fn driving_straight_without_steering_mostly_fails() {
        let mut wins = 0;
        for seed in 0..40u64 {
            let mut r = LaneRound::new(Rng::new(seed * 7919 + 11));
            let out = run(&mut r, COUNTDOWN_SECS + LIMIT_SECS + 1.0);
            if out == Outcome::Won {
                wins += 1;
            }
        }
        assert!(wins <= 6, "가만히 달리면 대부분 진다: {wins}/40");
    }
}
