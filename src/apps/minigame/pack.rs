//! PACK — 손가락의 방에서 받는 포장 리듬 게임(시트 SUB B+c-1 ~ 5). 상자가 컨베이어를 타고 판정선(바)에
//! 정확히 걸쳐진 동안 Space/버튼을 한 번 누른다. 채점 없는 연습 3번 → START → 본 작업 12개(상자가 바와
//! 겹쳐 있는 짧은 순간에 누르면 성공 1회) → 8개 이상이면 합격. 버튼을 길게 눌러도 1회로만 센다(입력은 눌린 순간만 본다).
//! 판정 구간 밖에서 성급하게/늦게 누르면(구간의 0.6초 안) 그 상자는 실패다 — 마구 연타로는 못 깬다.

use super::{Controls, Outcome};

pub const PRACTICE_N: usize = 3;
pub const WORK_N: usize = 12;
pub const PASS_AT: usize = 8;
// 화면 배치 — 상자와 바의 크기가 곧 판정이다: 상자의 어느 부분이 바에 닿아 있는 동안만 성공이다.
pub const LINE_X: f32 = 320.0; // 바의 x
pub const SPAWN_X: f32 = -40.0; // 상자가 나타나는 x
pub const BOX_HALF: f32 = 18.0; // 상자 반너비
pub const LINE_HALF: f32 = 1.5; // 바 반두께
pub const SPEED: f32 = (LINE_X - SPAWN_X) / TRAVEL; // 상자가 컨베이어를 가는 속도(px/s)
pub const WINDOW: f32 = (BOX_HALF + LINE_HALF) / SPEED; // 바와 겹쳐 있는 시간의 절반(초) — 약 0.11초
const MISFIRE: f32 = 0.6; // 판정 구간 밖이어도 이 안에서 누르면 가장 가까운 상자가 실패
pub const TRAVEL: f32 = 2.0; // 상자가 나타나서 판정선에 닿기까지(초)
const GAP: f32 = 1.1; // 상자 간격(초)
const FIRST_AT: f32 = 1.6;
const TAIL: f32 = 0.9; // 마지막 상자가 지나간 뒤 결과를 보여주기까지

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Practice,
    Ready, // 연습이 끝났다 — START 를 기다린다
    Work,
    Done,
}

#[derive(Clone, Copy)]
pub struct Crate {
    pub arrive: f32,           // 판정선에 닿는 시각(현재 구간 시작부터)
    pub result: Option<bool>, // 성공/실패(연습은 점수에 안 든다)
}

pub struct PackRound {
    pub phase: Phase,
    pub t: f32, // 현재 구간 시작부터 흐른 시간
    pub crates: Vec<Crate>,
    pub flash: f32, // 방금 눌렀을 때 판정선이 번쩍이는 시간
}

fn lineup(n: usize) -> Vec<Crate> {
    (0..n).map(|i| Crate { arrive: FIRST_AT + GAP * i as f32, result: None }).collect()
}

impl PackRound {
    pub fn new() -> PackRound {
        PackRound { phase: Phase::Practice, t: 0.0, crates: lineup(PRACTICE_N), flash: 0.0 }
    }

    pub fn hits(&self) -> usize {
        self.crates.iter().filter(|c| c.result == Some(true)).count()
    }

    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        self.flash = (self.flash - dt).max(0.0);
        match self.phase {
            Phase::Practice | Phase::Work => {
                self.t += dt;
                if c.act {
                    self.flash = 0.15;
                    let t = self.t;
                    if let Some(b) = self.crates.iter_mut().find(|b| b.result.is_none() && (t - b.arrive).abs() <= WINDOW) {
                        b.result = Some(true);
                    } else if let Some(b) = self
                        .crates
                        .iter_mut()
                        .filter(|b| b.result.is_none() && (t - b.arrive).abs() <= MISFIRE)
                        .min_by(|a, b| (t - a.arrive).abs().total_cmp(&(t - b.arrive).abs()))
                    {
                        b.result = Some(false); // 성급하게 눌러 놓친 상자
                    }
                }
                let t = self.t;
                for b in &mut self.crates {
                    if b.result.is_none() && t > b.arrive + WINDOW {
                        b.result = Some(false);
                    }
                }
                let last = self.crates.last().map_or(0.0, |b| b.arrive);
                if self.t > last + TAIL {
                    self.phase = if self.phase == Phase::Practice { Phase::Ready } else { Phase::Done };
                }
                Outcome::Running
            }
            Phase::Ready => {
                if c.act {
                    self.phase = Phase::Work;
                    self.t = 0.0;
                    self.crates = lineup(WORK_N);
                }
                Outcome::Running
            }
            Phase::Done => {
                if self.hits() >= PASS_AT {
                    Outcome::Won
                } else {
                    Outcome::Lost
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press() -> Controls {
        Controls { act: true, ..Controls::default() }
    }

    // 정해진 시각에만 누르는 봇으로 한 판을 돌린다. offsets 는 상자마다 눌러야 할 시각에서 얼마나 어긋나게 누를지(None 이면 안 누름).
    fn play(offsets: &[Option<f32>]) -> (PackRound, Outcome) {
        let mut r = PackRound::new();
        while r.phase == Phase::Practice {
            r.step(0.01, &Controls::default());
        }
        assert_eq!(r.phase, Phase::Ready);
        r.step(0.01, &press());
        assert_eq!(r.phase, Phase::Work);
        let mut presses: Vec<f32> = offsets.iter().enumerate().filter_map(|(i, o)| o.map(|d| FIRST_AT + GAP * i as f32 + d)).collect();
        presses.reverse();
        let mut out = Outcome::Running;
        for _ in 0..100_000 {
            let due = presses.last().is_some_and(|&p| r.t >= p);
            if due {
                presses.pop();
            }
            out = r.step(0.005, &Controls { act: due, ..Controls::default() });
            if r.phase == Phase::Done {
                out = r.step(0.005, &Controls::default());
                break;
            }
        }
        (r, out)
    }

    #[test]
    fn eight_of_twelve_is_the_passing_edge() {
        let mut offsets = vec![Some(0.0); 8];
        offsets.extend(vec![None; 4]);
        let (r, out) = play(&offsets);
        assert_eq!(r.hits(), 8);
        assert!(matches!(out, Outcome::Won));

        let mut seven = vec![Some(0.0); 7];
        seven.extend(vec![None; 5]);
        let (r, out) = play(&seven);
        assert_eq!(r.hits(), 7);
        assert!(matches!(out, Outcome::Lost));
    }

    #[test]
    fn only_the_moment_the_box_overlaps_the_bar_counts() {
        let (r, _) = play(&[Some(0.08), Some(-0.08), Some(0.2), Some(-0.2)]);
        assert_eq!(r.hits(), 2, "바에 걸쳐 있을 때(±0.1초)만 성공, 그 밖은 실패");
        const { assert!(WINDOW > 0.09 && WINDOW < 0.13) };
    }

    // 마구 연타하면 구간 밖에서 누른 것이 상자를 망쳐 합격 못 한다.
    #[test]
    fn mashing_the_button_does_not_pass() {
        let mut r = PackRound::new();
        while r.phase == Phase::Practice {
            r.step(0.01, &Controls::default());
        }
        r.step(0.01, &press());
        let mut acc = 0.0;
        let mut out = Outcome::Running;
        for _ in 0..100_000 {
            acc += 0.005;
            let act = acc >= 0.1;
            if act {
                acc = 0.0;
            }
            out = r.step(0.005, &Controls { act, ..Controls::default() });
            if r.phase == Phase::Done {
                out = r.step(0.005, &Controls::default());
                break;
            }
        }
        assert!(matches!(out, Outcome::Lost), "hits={}", r.hits());
    }

    #[test]
    fn practice_is_not_scored_and_pressing_early_is_ignored() {
        let mut r = PackRound::new();
        // 연습 중 마구 눌러도 본 작업에 영향이 없다.
        for _ in 0..100 {
            r.step(0.01, &press());
        }
        while r.phase == Phase::Practice {
            r.step(0.01, &Controls::default());
        }
        r.step(0.01, &press());
        assert_eq!(r.phase, Phase::Work);
        assert_eq!(r.hits(), 0);
        assert_eq!(r.crates.len(), WORK_N);
    }
}
