//! AI 카트 — 성격(컨디션, 달리는 줄, 출발 반응, 코너 감속, 헛손질)과 갈림길에서 고르는 길, 그리고 한 프레임의
//! 조작(핸들/가속/브레이크)을 정하는 ai_drive. 사람 카트는 Controls 로 직접 조작하고 이 모듈은 쓰지 않는다.

use crate::random::Rng;

use super::kart::Drive;
use super::track::{wrap_angle, FORKS};
use super::{DriveRound, MAX_SPEED};

#[derive(Clone)]
pub(super) struct Ai {
    pub(super) skill: f32, // 타고난 실력(최고 속도 비율)
    pub(super) eff: f32,   // 지금 컨디션이 반영된 실력 — 수 초마다 오르내린다
    pub(super) form: f32,
    pub(super) form_target: f32,
    pub(super) form_in: f32,
    pub(super) offset: f32, // 길 중심에서 옆으로 얼마나 떨어져 달리는지(지금 값)
    pub(super) offset_target: f32,
    pub(super) offset_in: f32,
    pub(super) wander: f32,   // 길 폭의 몇 배까지 이리저리 달리는지(성격)
    pub(super) delay: f32,    // 출발 신호 뒤에 반응하기까지 걸리는 시간
    pub(super) caution: f32,  // 코너에서 얼마나 크게 감속하는지(성격)
    pub(super) mistake_in: f32,
    pub(super) mistake_gap: [f32; 2], // 헛손질 사이의 간격 범위(초)
    pub(super) mistake_len: f32,
    pub(super) mistake_power: f32,
    pub(super) mistake_left: f32,
    pub(super) mistake_dir: f32,
    pub(super) plan: Vec<usize>, // 갈림길마다 고른 길(0 은 큰길)
    pub(super) plan_lap: i32,    // 이 계획을 세운 바퀴 — 바퀴가 바뀌면 다시 고른다
    pub(super) locked: bool,     // true 면 계획을 다시 안 고른다(테스트용 고정 경로)
}

impl Ai {
    // 카트마다 성격이 다르다: 컨디션이 흔들리고, 이리저리 옆으로 달리고, 출발 반응이 다르고, 코너를 겁내는
    // 정도/헛손질의 잦기와 세기가 제각각이라 같은 실력이라도 레이스마다 다르게 달린다.
    pub(super) fn new(rng: &mut Rng, skill: f32, plan: Vec<usize>) -> Ai {
        let skill = skill * rng.range_f32(0.96, 1.04);
        Ai {
            skill,
            eff: skill,
            form: 0.0,
            form_target: 0.0,
            form_in: rng.range_f32(1.0, 4.0),
            offset: 0.0,
            offset_target: 0.0,
            offset_in: rng.range_f32(0.5, 2.0),
            wander: rng.range_f32(0.15, 0.55),
            delay: rng.range_f32(0.0, 0.7),
            caution: rng.range_f32(0.4, 0.7),
            mistake_in: rng.range_f32(3.0, 8.0),
            mistake_gap: [rng.range_f32(3.0, 5.0), rng.range_f32(7.0, 11.0)],
            mistake_len: rng.range_f32(0.3, 0.8),
            mistake_power: rng.range_f32(0.5, 1.0),
            mistake_left: 0.0,
            mistake_dir: 0.0,
            plan,
            plan_lap: -1,
            locked: false,
        }
    }

    // 무작위 없이 항상 같은 솜씨로 달리는 AI — 사람 대신 돌려 보는 테스트용 봇.
    #[cfg(test)]
    pub(super) fn steady(skill: f32, plan: Vec<usize>) -> Ai {
        Ai {
            skill,
            eff: skill,
            form: 0.0,
            form_target: 0.0,
            form_in: 1.0e9,
            offset: 0.0,
            offset_target: 0.0,
            offset_in: 1.0e9,
            wander: 0.0,
            delay: 0.0,
            caution: 0.55,
            mistake_in: 1.0e9,
            mistake_gap: [1.0e9, 1.0e9],
            mistake_len: 0.0,
            mistake_power: 0.0,
            mistake_left: 0.0,
            mistake_dir: 0.0,
            plan,
            plan_lap: 999,
            locked: true,
        }
    }
}

// 갈림길마다 가중치대로 길을 하나씩 고른다.
pub(super) fn roll_plan(rng: &mut Rng, skill: f32) -> Vec<usize> {
    FORKS
        .iter()
        .map(|fork| {
            // 실력이 낮은 AI 는 좁은 지름길을 덜 고른다.
            let weights: Vec<f32> = fork.options.iter().map(|&(r, w)| if r == 0 || r == 2 { w } else { w * ((skill - 0.7) / 0.18).clamp(0.2, 1.0) }).collect();
            let total: f32 = weights.iter().sum();
            let mut pick = rng.unit() * total;
            for (i, w) in weights.iter().enumerate() {
                if pick < *w {
                    return fork.options[i].0;
                }
                pick -= w;
            }
            fork.options[0].0
        })
        .collect()
}

impl DriveRound {
    // AI 한 대의 조작 — 앞쪽 길을 보고 핸들을 꺾고, 코너가 급하면 속도를 줄인다. 가끔 헛손질도 한다.
    pub(super) fn ai_drive(&mut self, k: usize, dt: f32, player_progress: f32) -> Drive {
        let track = &self.track;
        let kart = &mut self.karts[k];
        let Some(mut ai) = kart.ai.take() else { return Drive::default() };
        // 바퀴가 바뀌면 갈림길에서 어느 길로 갈지 다시 정한다.
        if !ai.locked && ai.plan_lap != kart.laps {
            ai.plan = roll_plan(&mut self.rng, ai.skill);
            ai.plan_lap = kart.laps;
        }
        // 출발 신호를 보고 반응하기까지 시간이 걸린다.
        if ai.delay > 0.0 {
            ai.delay -= dt;
            kart.ai = Some(ai);
            return Drive::default();
        }
        // 컨디션: 몇 초마다 새 목표로 천천히 오르내린다.
        ai.form_in -= dt;
        if ai.form_in <= 0.0 {
            ai.form_target = self.rng.range_f32(-0.05, 0.035);
            ai.form_in = self.rng.range_f32(3.0, 7.0);
        }
        ai.form += (ai.form_target - ai.form) * (0.8 * dt).min(1.0);
        ai.eff = ai.skill * (1.0 + ai.form);
        let look = 4 + (kart.speed.max(0.0) * 0.28) as usize;
        let (lr, li) = track.advance(kart.route, kart.idx, look, &ai.plan);
        let route = &track.routes[lr];
        let t = route.tang[li];
        let n = [t[1], -t[0]];
        // 달리는 줄: 몇 초마다 길 폭 안에서 새 줄을 골라 옆으로 슬금슬금 옮겨 간다.
        ai.offset_in -= dt;
        if ai.offset_in <= 0.0 {
            ai.offset_target = (self.rng.unit() * 2.0 - 1.0) * ai.wander * route.half_w;
            ai.offset_in = self.rng.range_f32(2.0, 5.0);
        }
        ai.offset += (ai.offset_target - ai.offset) * (1.5 * dt).min(1.0);
        let off = ai.offset.clamp(-route.half_w * 0.55, route.half_w * 0.55);
        let target = [route.pts[li][0] + n[0] * off, route.pts[li][1] + n[1] * off];
        let want = (-(target[0] - kart.pos[0])).atan2(-(target[1] - kart.pos[1]));
        let mut steer = (wrap_angle(want - kart.heading) * 2.6).clamp(-1.0, 1.0);

        // 헛손질: 간격/길이/세기가 제각각인 순간에 핸들이 엉뚱하게 꺾인다.
        ai.mistake_in -= dt;
        if ai.mistake_left > 0.0 {
            ai.mistake_left -= dt;
            steer = (steer + ai.mistake_dir).clamp(-1.0, 1.0);
        } else if ai.mistake_in <= 0.0 {
            ai.mistake_in = self.rng.range_f32(ai.mistake_gap[0], ai.mistake_gap[1]);
            ai.mistake_left = ai.mistake_len * self.rng.range_f32(0.7, 1.3);
            ai.mistake_dir = if self.rng.unit() < 0.5 { -ai.mistake_power } else { ai.mistake_power };
        }

        // 코너 정도 — 앞쪽 두 지점의 진행 방향 차이.
        let (ar, ai_idx) = track.advance(kart.route, kart.idx, look + 8, &ai.plan);
        let bend = wrap_angle(track.routes[ar].heading_at(ai_idx) - track.routes[kart.route].heading_at(kart.idx)).abs();
        // 고무줄: 사람과 너무 벌어지면 살짝 눌러주거나 밀어준다.
        let gap = kart.progress - player_progress;
        let rubber = if gap > 60.0 { 0.93 } else if gap < -60.0 { 1.06 } else { 1.0 };
        let target_speed = MAX_SPEED * ai.eff * rubber * (1.0 - (bend * ai.caution).min(0.5));
        let drive = Drive { gas: kart.speed < target_speed, brake: kart.speed > target_speed + 4.0, steer };
        kart.ai = Some(ai);
        drive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::minigame::Controls;
    use crate::apps::minigame::Outcome;
    use crate::random::Rng;
    use super::super::RACERS;

    #[test]
    fn ai_karts_use_different_routes() {
        let mut routes_seen = std::collections::HashSet::new();
        for seed in 1..=8u64 {
            let mut r = DriveRound::new(Rng::new(seed));
            for _ in 0..(120.0 / 0.016) as usize {
                r.step(0.016, &Controls::default());
                for k in r.karts.iter().skip(1) {
                    routes_seen.insert(k.route);
                }
            }
        }
        assert!(routes_seen.len() >= 3, "AI 가 여러 길을 탄다: {routes_seen:?}");
    }

    #[test]
    fn ai_karts_race_differently_each_time() {
        let mut winners = std::collections::HashSet::new();
        let mut spread_ok = 0;
        for seed in 1..=14u64 {
            let mut r = DriveRound::new(Rng::new(seed));
            let mut out = Outcome::Running;
            for _ in 0..(400.0 / 0.016) as usize {
                out = r.step(0.016, &Controls::default());
                if !matches!(out, Outcome::Running) {
                    break;
                }
            }
            assert!(matches!(out, Outcome::Lost));
            let first = (1..RACERS).min_by(|&a, &b| r.karts[a].finish_time.unwrap_or(1e9).total_cmp(&r.karts[b].finish_time.unwrap_or(1e9))).unwrap();
            winners.insert(first);
            let mut p: Vec<f32> = (1..RACERS).map(|i| r.karts[i].progress).collect();
            p.sort_by(|a, b| a.total_cmp(b));
            if p.windows(2).all(|w| w[1] - w[0] > 1.0) {
                spread_ok += 1;
            }
        }
        assert!(winners.len() >= 2, "항상 같은 카트가 1등: {winners:?}");
        assert!(spread_ok >= 12, "카트들이 한 덩어리로 같이 달린다: {spread_ok}/14");
    }

    // 같은 시드라도 성격(옆으로 달리는 줄, 출발 반응)이 카트마다 다르다.

    #[test]
    fn each_ai_has_its_own_personality() {
        let r = DriveRound::new(Rng::new(7));
        let ais: Vec<&Ai> = r.karts.iter().skip(1).filter_map(|k| k.ai.as_ref()).collect();
        let distinct = |f: &dyn Fn(&Ai) -> f32| {
            let mut v: Vec<f32> = ais.iter().map(|a| f(a)).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            v.windows(2).all(|w| w[1] - w[0] > 1e-3)
        };
        assert!(distinct(&|a| a.wander) && distinct(&|a| a.delay) && distinct(&|a| a.caution));
    }

    // 길의 폭은 어디서나 같다: 길 가장자리 점들은 중심선에서 반폭만큼(코너에서도) 떨어져 있다.
}
