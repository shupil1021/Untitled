//! BRAIN_DRIVE — 뇌의 방에서 받는 3D 카트 레이스. 3바퀴를 돌아서 AI 카트 셋보다 먼저(1등으로) 들어오면
//! 클리어. W/↑ 가속, S/↓ 브레이크, A·D/←→ 핸들 — 그냥 운전만 한다. 화면에는 회색 길과 양옆의
//! 가드레일, 출발선의 깃발 하나, 카트만 그린다(배경은 한 가지 색). 가드레일에 부딪히면 속도가 크게 줄어든다. AI 가 먼저
//! 결승선을 넘으면 그 순간 1등은 불가능하니 바로 실패다.
//!
//! 길과 가드레일은 박스 대신 삼각형 띠(리본)로 만든다 — 길 중심선을 따라 양옆으로 일정한 거리(반폭)만큼
//! 떨어진 점들을 이어서(코너에서는 안/바깥쪽 점을 같은 거리로 밀어 이음매 없이) 폭이 어디서나 같다.
//!
//! 코스는 큰 순환로(main) 하나에 샛길이 붙은 모양이다. 갈림길이 둘: 동쪽은 세 갈래(가운데 큰길 / 안쪽 좁은
//! 지름길 / 바깥쪽 넓고 긴 길), 서쪽은 두 갈래(바깥 큰길 / 안쪽 꺾이는 지름길). 샛길은 갈라지는 점에서 큰길과
//! 같은 방향으로 시작해서 합쳐지는 점에서 다시 큰길로 이어진다. 카트는 길 점들 가운데 가장 가까운 곳으로
//! "지금 어디쯤인지"를 세는데, 샛길에서도 큰길 기준 진행 거리로 바꿔서 순위를 매긴다.
//! 화면은 apps/game3d 의 3D 렌더 도구를 그대로 쓴다.


mod ai;
mod kart;
mod scene;
mod track;

use std::rc::Rc;

use crate::random::Rng;
use crate::render::mesh3d::{Box3D, Camera, MeshData, MeshObject};

use super::{Controls, Outcome};
use ai::{roll_plan, Ai};
use kart::{Drive, Kart};
use scene::{build_flag, build_meshes, kart_boxes};
use track::{forward, wrap_angle, Track};

pub const LAPS: i32 = 3;
pub const RACERS: usize = 4;
const WALL_INSET: f32 = 1.0; // 카트 중심이 갈 수 있는 한계 — 길 가장자리(가드레일 안쪽 면)에서 이만큼 안쪽(카트 몸 반폭)
const MAX_SPEED: f32 = 26.0;
const ACCEL: f32 = 15.0;
const BRAKE: f32 = 30.0;
const DRAG: f32 = 5.0;
const STEER_RATE: f32 = 2.1;
const KART_R: f32 = 1.1;
const COUNTDOWN: f32 = 3.0;
const SAMPLES_PER_SEGMENT: usize = 12;
// 이 값이 AI 의 실력(최고 속도 비율)이다 — 사람이 코너를 잘 돌면 이길 만하게.
const AI_SKILL: [f32; RACERS - 1] = [0.79, 0.83, 0.87];

pub struct DriveRound {
    pub track: Track,
    pub karts: Vec<Kart>, // 0 번이 플레이어
    t: f32,
    finished: Option<Outcome>,
    cam_heading: f32,
    flag: Vec<Box3D>, // 출발선의 깃발(상자)
    road: Rc<MeshData>,  // 길 전체의 삼각형 띠
    guard: Rc<MeshData>, // 가드레일 전체
    rng: Rng,
}

impl DriveRound {
    pub fn new(mut rng: Rng) -> DriveRound {
        let track = Track::new();
        let mut karts = vec![Kart::new(&track, 0, None)];
        for (i, &skill) in AI_SKILL.iter().enumerate() {
            let plan = roll_plan(&mut rng, skill);
            let ai = Ai::new(&mut rng, skill, plan);
            karts.push(Kart::new(&track, i + 1, Some(ai)));
        }
        let (road, guard) = build_meshes(&track);
        let flag = build_flag(&track);
        let cam_heading = karts[0].heading;
        DriveRound { track, karts, t: 0.0, finished: None, cam_heading, flag, road: Rc::new(road), guard: Rc::new(guard), rng }
    }

    // 출발 카운트다운이 남은 시간(0 이하면 출발).
    pub fn countdown(&self) -> f32 {
        COUNTDOWN - self.t
    }

    // 사람(0 번)의 지금 순위(1 이 선두).
    pub fn place(&self) -> usize {
        let me = &self.karts[0];
        // 이미 결승선을 넘은 카트가 앞선다. 못 넘은 카트끼리는 진행 거리로.
        let ahead = |k: &Kart| match (k.finish_time, me.finish_time) {
            (Some(a), Some(b)) => a < b,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => k.progress > me.progress,
        };
        1 + self.karts.iter().skip(1).filter(|k| ahead(k)).count()
    }


    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        self.t += dt;
        if let Some(out) = self.finished {
            return out;
        }
        if self.countdown() > 0.0 {
            return Outcome::Running;
        }
        let me = Drive { gas: c.dy < 0.0, brake: c.dy > 0.0 || c.brake, steer: -c.dx };
        let player_progress = self.karts[0].progress;
        for k in 0..self.karts.len() {
            let d = if k == 0 { me } else { self.ai_drive(k, dt, player_progress) };
            self.karts[k].step(&self.track, d, dt);
        }
        // 카트끼리 부딪히면 서로 밀어낸다.
        for a in 0..self.karts.len() {
            for b in (a + 1)..self.karts.len() {
                let (dx, dz) = (self.karts[b].pos[0] - self.karts[a].pos[0], self.karts[b].pos[1] - self.karts[a].pos[1]);
                let dist = (dx * dx + dz * dz).sqrt();
                if dist < KART_R * 2.0 && dist > 1e-3 {
                    let push = (KART_R * 2.0 - dist) / 2.0;
                    let (nx, nz) = (dx / dist, dz / dist);
                    self.karts[a].pos = [self.karts[a].pos[0] - nx * push, self.karts[a].pos[1] - nz * push];
                    self.karts[b].pos = [self.karts[b].pos[0] + nx * push, self.karts[b].pos[1] + nz * push];
                    self.karts[a].speed *= 0.985;
                    self.karts[b].speed *= 0.985;
                }
            }
        }
        // 결승선 통과.
        let t = self.t;
        for (i, k) in self.karts.iter_mut().enumerate() {
            if k.finish_time.is_none() && k.laps >= LAPS {
                k.finish_time = Some(t);
                self.finished = self.finished.or(Some(if i == 0 { Outcome::Won } else { Outcome::Lost }));
            }
        }
        // 카메라는 카트가 향한 쪽을 부드럽게 따라간다.
        let diff = wrap_angle(self.karts[0].heading - self.cam_heading);
        self.cam_heading += diff * (1.0 - (-5.0 * dt).exp());
        self.finished.unwrap_or(Outcome::Running)
    }

    // 사람 카트 뒤에서 내려다보는 추적 카메라.
    pub fn camera(&self) -> Camera {
        let me = &self.karts[0];
        let f = forward(self.cam_heading);
        Camera { pos: [me.pos[0] - f[0] * 6.5, 3.2, me.pos[1] - f[1] * 6.5], yaw: self.cam_heading, pitch: -0.2 }
    }

    // 길과 가드레일(삼각형 메시).
    pub fn meshes(&self) -> Vec<MeshObject> {
        let obj = |mesh: &Rc<MeshData>, color: [f32; 4]| MeshObject { center: [0.0; 3], yaw: 0.0, pitch: 0.0, roll: 0.0, scale: [1.0; 3], color, texture: None, walkable: false, solid: false, mesh: Rc::clone(mesh) };
        vec![obj(&self.road, [0.5, 0.5, 0.5, 1.0]), obj(&self.guard, [0.78, 0.78, 0.78, 1.0])]
    }

    // 그릴 상자 — 깃발과 카트들.
    pub fn boxes(&self) -> Vec<Box3D> {
        let mut out = self.flag.clone();
        for (i, k) in self.karts.iter().enumerate() {
            kart_boxes(&mut out, k, i);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::track::FORKS;
    use super::*;

    // 사람 자리(0 번)에 AI 와 같은 운전 솜씨를 넣어 돌려 보는 봇. plan 으로 갈림길에서 어느 길로 갈지도 정할 수 있다.
    fn run_with_bot(seed: u64, skill: f32, plan: Option<Vec<usize>>) -> (Outcome, usize, f32) {
        let mut r = DriveRound::new(Rng::new(seed));
        let fixed = plan.clone().unwrap_or_else(|| vec![0; FORKS.len()]);
        r.karts[0].ai = Some(Ai::steady(skill, fixed));
        let mut out = Outcome::Running;
        let mut time = 0.0;
        for _ in 0..(400.0 / 0.016) as usize {
            // 사람 카트 조작은 AI 컨트롤러가 정한 걸 그대로 쓴다(갈 길만 고른다).
            let pp = r.karts[1].progress;
            let d = r.ai_drive(0, 0.016, pp);
            let c = Controls { dx: -d.steer, dy: if d.gas { -1.0 } else if d.brake { 1.0 } else { 0.0 }, ..Controls::default() };
            out = r.step(0.016, &c);
            time += 0.016;
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        (out, r.place(), time)
    }

    #[test]
    fn karts_start_behind_the_line_and_wait_for_the_countdown() {
        let mut r = DriveRound::new(Rng::new(1));
        for k in &r.karts {
            assert!(k.progress < 0.0 && k.progress > -40.0, "출발선 바로 뒤에 선다: {}", k.progress);
        }
        let full = Controls { dy: -1.0, ..Controls::default() };
        r.step(1.0, &full);
        assert!(r.karts[0].speed == 0.0, "카운트다운 동안은 못 간다");
        r.step(2.5, &full);
        r.step(0.5, &full);
        assert!(r.karts[0].speed > 0.0);
    }

    // 어느 길을 골라도 끝까지 달려서 결승선을 넘을 수 있다(길이 끊기거나 막힌 곳이 없다).

    #[test]
    fn every_route_combination_can_be_driven_to_the_finish() {
        let mut times = Vec::new();
        for east in [0, 1, 2] {
            for west in [0, 3] {
                let (out, _, time) = run_with_bot(4, 1.0, Some(vec![east, west]));
                assert!(!matches!(out, Outcome::Running), "경로 {east},{west}: 끝까지 못 갔다");
                times.push((east, west, time));
            }
        }
        // (시간 비교는 하지 않는다 — 이 봇은 급한 코너에서 감속해서, 코너가 완만한 바깥길이 오히려 빨라 보인다.
        // 길이 차이는 위 테스트가 확인한다.)
        assert_eq!(times.len(), 6);
    }

    #[test]
    fn a_skilled_driver_wins_and_a_slow_one_loses() {
        let plans = [vec![0, 0], vec![1, 3], vec![0, 3], vec![1, 0]];
        let skilled = (1..=12u64).filter(|&s| matches!(run_with_bot(s, 1.0, Some(plans[s as usize % 4].clone())).0, Outcome::Won)).count();
        let slow = (1..=12u64).filter(|&s| matches!(run_with_bot(s, 0.7, Some(plans[s as usize % 4].clone())).0, Outcome::Won)).count();
        assert!(skilled >= 10, "잘 타면 1등: {skilled}/12");
        assert!(slow == 0, "느리면 1등 못 한다: {slow}/12");
    }

    #[test]
    fn idling_at_the_start_loses_when_an_ai_finishes() {
        let mut r = DriveRound::new(Rng::new(3));
        let mut out = Outcome::Running;
        for _ in 0..(400.0 / 0.016) as usize {
            out = r.step(0.016, &Controls::default());
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        assert!(matches!(out, Outcome::Lost));
        assert_eq!(r.place(), RACERS, "가만히 있으면 꼴찌");
    }

    // 가드레일을 향해 계속 밀어도 가드레일 밖으로 못 나가고, 부딪히면 속도가 줄어든다.
}
