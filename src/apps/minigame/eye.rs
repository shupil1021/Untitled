//! SHY_EYE — 눈의 방에서 받는 탄막 피하기(시트 SUB B+a-1 ~ 5). 공격 없이 피하기만 하는 2D 게임:
//! 생명 3개로 30초 버티면 클리어. 처음 8초는 위쪽 큰 눈의 부채꼴 탄막(익히는 구간), 8초부터 사방으로 퍼지는
//! 고리가 섞이고, 15초부터 왼쪽에서도 부채꼴이 날아오고, 20~25초는 눈이 좌우로 훑으며 탄을 흘리고,
//! 마지막 5초는 탄이 빨라지고 부채꼴이 잦아지는 클라이맥스다. 탄막은 항상 빈 틈이 있고, 맞으면 잠깐 무적.

use crate::random::Rng;

use super::{Controls, Outcome};

pub const W: f32 = 480.0;
pub const H: f32 = 336.0; // 위 24px 은 HUD 라서 판은 그 아래
pub const GOAL_SECS: f32 = 30.0;
pub const PHASE2_AT: f32 = 15.0;
pub const LIVES: u32 = 3;
const SPEED: f32 = 150.0;
const BULLET_SPEED: f32 = 70.0;
const BULLET_R: f32 = 4.0;
const PLAYER_R: f32 = 4.0;
pub const INVULN_SECS: f32 = 1.0;
const RING_FROM: f32 = 8.0; // 이때부터 사방으로 퍼지는 고리가 섞인다
const RING_EVERY: f32 = 3.6;
const RING_COUNT: usize = 20;
const SWEEP: (f32, f32) = (20.0, 25.0); // 눈이 좌우로 훑으며 탄을 흘리는 구간
const SWEEP_EVERY: f32 = 0.15;
pub const FINALE_AT: f32 = 25.0; // 마지막 5초 — 탄이 빨라지고 부채꼴이 잦아진다
const FINALE_BOOST: f32 = 0.35;
const FINALE_FAN_EVERY: f32 = 1.2;
const FAN_COUNT: usize = 13;
const FAN_SPREAD: f32 = 1.05; // 라디안(약 60도)
const FAN_EVERY: f32 = 1.8;
const GAP_LEN: usize = 4;

#[derive(Clone, Copy)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
}

pub struct EyeRound {
    t: f32,
    lives: u32,
    invuln: f32,
    pub px: f32,
    pub py: f32,
    pub bullets: Vec<Bullet>,
    next_top: f32,
    next_left: f32,
    next_ring: f32,
    next_sweep: f32,
    rng: Rng,
}

impl EyeRound {
    pub fn new(rng: Rng) -> EyeRound {
        EyeRound { t: 0.0, lives: LIVES, invuln: 0.0, px: W / 2.0, py: H - 50.0, bullets: Vec::new(), next_top: 1.0, next_left: PHASE2_AT + 1.0, next_ring: RING_FROM, next_sweep: SWEEP.0, rng }
    }

    pub fn time(&self) -> f32 {
        self.t
    }

    pub fn lives(&self) -> u32 {
        self.lives
    }

    pub fn invulnerable(&self) -> bool {
        self.invuln > 0.0
    }

    // 지금 탄 속도 — 마지막 5초(FINALE_AT~)에 FINALE_BOOST 만큼 서서히 빨라진다.
    fn bullet_speed(&self) -> f32 {
        let k = ((self.t - FINALE_AT) / (GOAL_SECS - FINALE_AT)).clamp(0.0, 1.0);
        BULLET_SPEED * (1.0 + FINALE_BOOST * k)
    }

    // 위쪽 눈 한가운데에서 사방(위쪽 절반은 화면 밖으로 나간다)으로 퍼지는 고리 — 부채꼴과 달리 빈 틈이 따로 없고
    // 탄 사이 간격이 넓어서 그 사이로 지나가야 한다. 매번 시작 각도가 달라진다.
    fn ring(&mut self) {
        let speed = self.bullet_speed() * 0.85;
        let offset = self.rng.unit() * std::f32::consts::TAU / RING_COUNT as f32;
        for i in 0..RING_COUNT {
            let a = offset + std::f32::consts::TAU * i as f32 / RING_COUNT as f32;
            self.bullets.push(Bullet { x: W / 2.0, y: 12.0, vx: a.cos() * speed, vy: a.sin() * speed });
        }
    }

    // 눈이 좌우로 고개를 흔들며 가는 탄 줄기를 흘린다 — 줄기가 지나가는 자리를 피해 타이밍을 맞춘다.
    fn sweep(&mut self) {
        let a = std::f32::consts::FRAC_PI_2 + (self.t * 1.3).sin() * 0.95;
        let speed = self.bullet_speed() * 1.05;
        self.bullets.push(Bullet { x: W / 2.0, y: 12.0, vx: a.cos() * speed, vy: a.sin() * speed });
    }

    // origin 에서 지금 플레이어가 있는 쪽을 중심으로 부채꼴로 퍼지는 탄막 한 줄 — 연속 GAP_LEN 개가 빠진 빈
    // 틈이 하나 있다. 틈은 중심이 아닌 곳에 생길 수 있어서 가만히 서 있으면(구석 포함) 맞는다.
    fn fan(&mut self, origin: (f32, f32)) {
        let base = (self.py - origin.1).atan2(self.px - origin.0);
        let speed = self.bullet_speed();
        let gap = 1 + (self.rng.next_u32() as usize) % (FAN_COUNT - GAP_LEN - 1);
        for i in 0..FAN_COUNT {
            if (gap..gap + GAP_LEN).contains(&i) {
                continue;
            }
            let a = base - FAN_SPREAD / 2.0 + FAN_SPREAD * i as f32 / (FAN_COUNT - 1) as f32;
            self.bullets.push(Bullet { x: origin.0, y: origin.1, vx: a.cos() * speed, vy: a.sin() * speed });
        }
    }

    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        self.t += dt;
        self.invuln = (self.invuln - dt).max(0.0);

        let len = (c.dx * c.dx + c.dy * c.dy).sqrt();
        if len > 0.01 {
            self.px = (self.px + c.dx / len * SPEED * dt).clamp(PLAYER_R, W - PLAYER_R);
            self.py = (self.py + c.dy / len * SPEED * dt).clamp(PLAYER_R, H - PLAYER_R);
        }

        // 발사 — 위쪽 눈은 계속, 왼쪽은 후반부터.
        let fan_every = if self.t >= FINALE_AT { FINALE_FAN_EVERY } else { FAN_EVERY };
        // 눈이 훑는 동안(SWEEP)에는 위쪽 부채꼴을 쉰다 — 줄기와 부채꼴이 한꺼번에 오면 피할 곳이 없다.
        let sweeping = (SWEEP.0..SWEEP.1).contains(&self.t);
        if self.t >= self.next_top {
            self.next_top += fan_every;
            if !sweeping {
                self.fan((W / 2.0, 12.0));
            }
        }
        if self.t >= PHASE2_AT && self.t >= self.next_left {
            self.next_left += fan_every;
            self.fan((0.0, H / 2.0));
        }
        if self.t >= RING_FROM && self.t < SWEEP.0 && self.t >= self.next_ring {
            self.next_ring += RING_EVERY;
            self.ring();
        }
        if sweeping && self.t >= self.next_sweep {
            self.next_sweep += SWEEP_EVERY;
            self.sweep();
        }

        for b in &mut self.bullets {
            b.x += b.vx * dt;
            b.y += b.vy * dt;
        }
        self.bullets.retain(|b| b.x > -20.0 && b.x < W + 20.0 && b.y > -20.0 && b.y < H + 20.0);

        if self.invuln <= 0.0 {
            let reach = (BULLET_R + PLAYER_R).powi(2);
            let (px, py) = (self.px, self.py);
            if let Some(i) = self.bullets.iter().position(|b| (b.x - px).powi(2) + (b.y - py).powi(2) < reach) {
                self.bullets.swap_remove(i);
                self.lives = self.lives.saturating_sub(1);
                self.invuln = INVULN_SECS;
            }
        }

        if self.lives == 0 {
            Outcome::Lost
        } else if self.t >= GOAL_SECS {
            Outcome::Won
        } else {
            Outcome::Running
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(round: &mut EyeRound, secs: f32, c: Controls) -> Outcome {
        let mut out = Outcome::Running;
        let steps = (secs / 0.016) as usize;
        for _ in 0..steps {
            out = round.step(0.016, &c);
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        out
    }

    // 탄막이 플레이어를 향하니, 구석에 가만히 서 있어도 맞고 결국 진다(예전엔 구석 캠핑으로 무피해 클리어였다).
    #[test]
    fn camping_in_a_corner_no_longer_wins() {
        for seed in 1..=20 {
            let mut r = EyeRound::new(Rng::new(seed));
            r.px = 6.0;
            r.py = H - 6.0;
            let out = run(&mut r, GOAL_SECS + 1.0, Controls::default());
            assert!(!(matches!(out, Outcome::Won) && r.lives() == LIVES), "seed {seed}: 구석 캠핑이 무피해로 통한다");
        }
    }

    // 앞으로 1.6초 가는 길을 미리 그려 보고 탄에 가장 덜 가까운 방향을 고르는 봇 — 제대로 피하면 이길 수 있어야 한다.
    fn dodger(r: &EyeRound) -> Controls {
        let dirs = [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (0.7, 0.7), (-0.7, 0.7), (0.7, -0.7), (-0.7, -0.7), (0.92, 0.38), (-0.92, 0.38), (0.92, -0.38), (-0.92, -0.38), (0.38, 0.92), (-0.38, 0.92), (0.38, -0.92), (-0.38, -0.92)];
        let mut best = (f32::MAX, Controls::default());
        for (dx, dy) in dirs {
            let mut cost = 0.0;
            for k in 1..=16 {
                let t = k as f32 * 0.1;
                let (px, py) = ((r.px + dx * SPEED * t).clamp(PLAYER_R, W - PLAYER_R), (r.py + dy * SPEED * t).clamp(PLAYER_R, H - PLAYER_R));
                for b in &r.bullets {
                    let d2 = (b.x + b.vx * t - px).powi(2) + (b.y + b.vy * t - py).powi(2);
                    if d2 < 26.0 * 26.0 {
                        cost += 26.0 * 26.0 - d2;
                    }
                }
            }
            // 가운데 쪽을 살짝 선호(구석에 몰리지 않게).
            cost += ((r.px - W / 2.0).abs() + (r.py - H / 2.0).abs()) * 0.02 + if dx == 0.0 && dy == 0.0 { 0.0 } else { 0.5 };
            if cost < best.0 {
                best = (cost, Controls { dx, dy, ..Controls::default() });
            }
        }
        best.1
    }

    #[test]
    fn a_dodger_can_still_win() {
        let mut wins = 0;
        for seed in 1..=20 {
            let mut r = EyeRound::new(Rng::new(seed));
            let mut out = Outcome::Running;
            for _ in 0..3000 {
                let c = dodger(&r);
                out = r.step(0.016, &c);
                if !matches!(out, Outcome::Running) {
                    break;
                }
            }
            if matches!(out, Outcome::Won) {
                wins += 1;
            }
        }
        assert!(wins >= 15, "잘 피하면 이길 수 있어야 한다: {wins}/20");
    }

    // 발사구 바로 아래에 계속 서 있으면 맞고, 맞은 직후엔 잠깐 무적이라 한 번에 생명이 하나씩만 준다.
    #[test]
    fn standing_in_the_fan_costs_lives_one_at_a_time() {
        let mut any_hit = false;
        for seed in 1..=10 {
            let mut r = EyeRound::new(Rng::new(seed));
            r.px = W / 2.0;
            r.py = 60.0;
            let mut hits = Vec::new();
            let mut last = r.lives();
            for _ in 0..(10.0 / 0.016) as usize {
                r.step(0.016, &Controls::default());
                if r.lives() != last {
                    assert_eq!(last - r.lives(), 1);
                    assert!(r.invulnerable());
                    hits.push(r.time());
                    last = r.lives();
                }
            }
            assert!(hits.windows(2).all(|w| w[1] - w[0] >= INVULN_SECS - 0.05), "무적 시간 안에는 또 안 맞는다");
            any_hit |= !hits.is_empty();
        }
        assert!(any_hit, "서 있으면 언젠가는 맞는다");
    }

    #[test]
    fn second_phase_adds_a_second_direction() {
        let mut r = EyeRound::new(Rng::new(1));
        r.px = W / 2.0;
        r.py = H - 20.0;
        // 전반: 위의 눈만 쏜다 — 나온 탄은 전부 위쪽 발사구(y 가 작은 곳)에서 아래로 향한다.
        let mut seen_left = false;
        for _ in 0..(PHASE2_AT / 0.016) as usize - 40 {
            r.px = W / 2.0;
            r.py = H - 20.0;
            r.step(0.016, &Controls::default());
            r.lives = LIVES;
            seen_left |= r.bullets.iter().any(|b| b.x < 40.0 && b.vx > 0.0);
        }
        assert!(!seen_left, "전반엔 왼쪽에서 오는 탄이 없다");
        for _ in 0..(4.0 / 0.016) as usize {
            r.px = W / 2.0;
            r.py = H - 20.0;
            r.step(0.016, &Controls::default());
            r.lives = LIVES;
            seen_left |= r.bullets.iter().any(|b| b.x < 40.0 && b.vx > 0.0);
        }
        assert!(seen_left, "후반엔 왼쪽에서 오른쪽으로 날아오는 탄이 생긴다");
    }

    // 가만히 서 있기만 해서는 이길 수 없다(어느 자리에 서든).
    #[test]
    fn standing_still_never_wins() {
        for seed in 1..=20 {
            for &(x, y) in &[(W / 2.0, H - 20.0), (W / 2.0, H / 2.0), (W - 30.0, H - 30.0)] {
                let mut r = EyeRound::new(Rng::new(seed));
                r.px = x;
                r.py = y;
                let out = run(&mut r, GOAL_SECS + 1.0, Controls::default());
                assert!(!matches!(out, Outcome::Won), "seed {seed} ({x},{y}): 서 있기만 했는데 이겼다");
            }
        }
    }

    // 시간이 흐르면서 패턴이 바뀐다: 고리(사방으로 퍼짐) → 훑는 줄기 → 마지막 5초는 탄이 빨라진다.
    #[test]
    fn patterns_change_over_time() {
        let mut r = EyeRound::new(Rng::new(3));
        let mut max_speed_before_finale = 0.0f32;
        let (mut saw_ring, mut saw_sweep, mut finale_speed) = (false, false, 0.0f32);
        for _ in 0..(GOAL_SECS / 0.016) as usize {
            r.px = W - 10.0; // 맞아도 상관없다 — 패턴만 본다
            r.py = H - 10.0;
            r.step(0.016, &Controls::default());
            r.lives = LIVES;
            let t = r.time();
            for b in &r.bullets {
                let speed = (b.vx * b.vx + b.vy * b.vy).sqrt();
                if t < FINALE_AT {
                    max_speed_before_finale = max_speed_before_finale.max(speed);
                } else {
                    finale_speed = finale_speed.max(speed);
                }
            }
            if (RING_FROM..RING_FROM + 1.0).contains(&t) && r.bullets.iter().any(|b| b.vy < -1.0) {
                saw_ring = true; // 위쪽(음의 y)으로도 날아가는 탄 = 사방으로 퍼지는 고리
            }
            if (SWEEP.0 + 1.0..SWEEP.1).contains(&t) && r.bullets.len() > 12 {
                saw_sweep = true;
            }
        }
        assert!(saw_ring, "8초부터 고리가 나온다");
        assert!(saw_sweep, "20~25초에는 줄기가 계속 흘러나온다");
        assert!(finale_speed > max_speed_before_finale * 1.1, "마지막 5초는 탄이 눈에 띄게 빨라진다: {finale_speed} vs {max_speed_before_finale}");
    }
}
