//! SHY_EYE — 눈의 방에서 받는 탄막 피하기(시트 SUB B+a-1 ~ 5). 공격 없이 피하기만 하는 2D 게임:
//! 생명 3개로 30초 버티면 클리어. 첫 15초는 위쪽 큰 눈이 한 방향 부채꼴 탄막을 쏘고, 나머지 15초는
//! 왼쪽에서도 날아와 두 방향이 된다. 탄막은 느리고 항상 빈 틈이 있다. 맞으면 잠깐 무적.

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
const INVULN_SECS: f32 = 1.0;
const FAN_COUNT: usize = 13;
const FAN_SPREAD: f32 = 1.05; // 라디안(약 60도)
const FAN_EVERY: f32 = 1.8;
const GAP_LEN: usize = 3;

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
    rng: Rng,
}

impl EyeRound {
    pub fn new(rng: Rng) -> EyeRound {
        EyeRound { t: 0.0, lives: LIVES, invuln: 0.0, px: W / 2.0, py: H - 50.0, bullets: Vec::new(), next_top: 1.0, next_left: PHASE2_AT + 1.0, rng }
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

    // origin 에서 base 방향을 중심으로 부채꼴로 퍼지는 탄막 한 줄 — 연속 GAP_LEN 개가 빠진 빈 틈이 하나 있다.
    fn fan(&mut self, origin: (f32, f32), base: f32) {
        let gap = 1 + (self.rng.next_u32() as usize) % (FAN_COUNT - GAP_LEN - 1);
        for i in 0..FAN_COUNT {
            if (gap..gap + GAP_LEN).contains(&i) {
                continue;
            }
            let a = base - FAN_SPREAD / 2.0 + FAN_SPREAD * i as f32 / (FAN_COUNT - 1) as f32;
            self.bullets.push(Bullet { x: origin.0, y: origin.1, vx: a.cos() * BULLET_SPEED, vy: a.sin() * BULLET_SPEED });
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
        if self.t >= self.next_top {
            self.next_top += FAN_EVERY;
            self.fan((W / 2.0, 12.0), std::f32::consts::FRAC_PI_2);
        }
        if self.t >= PHASE2_AT && self.t >= self.next_left {
            self.next_left += FAN_EVERY;
            self.fan((0.0, H / 2.0), 0.0);
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
                self.lives -= 1;
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

    // 탄막이 닿지 않는 구석에 가만히 서 있으면 30초를 생명 그대로 버틴다.
    #[test]
    fn corner_survives_the_whole_round() {
        let mut r = EyeRound::new(Rng::new(7));
        r.px = 6.0;
        r.py = H - 6.0;
        assert!(matches!(run(&mut r, GOAL_SECS + 1.0, Controls::default()), Outcome::Won));
        assert_eq!(r.lives(), LIVES);
    }

    // 발사구 바로 아래에 계속 서 있으면 맞고, 맞은 직후엔 잠깐 무적이라 한 번에 생명이 하나씩만 준다.
    #[test]
    fn standing_in_the_fan_costs_lives_one_at_a_time() {
        let mut r = EyeRound::new(Rng::new(3));
        r.px = W / 2.0;
        r.py = 60.0;
        let mut hits = Vec::new();
        let mut last = r.lives();
        for _ in 0..(8.0 / 0.016) as usize {
            r.step(0.016, &Controls::default());
            if r.lives() != last {
                assert_eq!(last - r.lives(), 1);
                assert!(r.invulnerable());
                hits.push(r.time());
                last = r.lives();
            }
        }
        assert!(!hits.is_empty());
        assert!(hits.windows(2).all(|w| w[1] - w[0] >= INVULN_SECS - 0.05), "무적 시간 안에는 또 안 맞는다");
    }

    #[test]
    fn second_phase_adds_a_second_direction() {
        let mut r = EyeRound::new(Rng::new(1));
        r.px = 6.0;
        r.py = H - 6.0;
        run(&mut r, PHASE2_AT - 0.5, Controls::default());
        assert!(r.bullets.iter().all(|b| b.vx.abs() < BULLET_SPEED), "전반엔 위에서만 쏜다(옆으로 빠르게 가는 탄이 없다)");
        run(&mut r, 3.0, Controls::default());
        assert!(r.bullets.iter().any(|b| b.vx > BULLET_SPEED * 0.8), "후반엔 왼쪽에서 오른쪽으로 날아오는 탄이 생긴다");
    }
}
