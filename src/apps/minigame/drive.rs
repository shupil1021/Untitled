//! BRAIN_DRIVE — 뇌의 방에서 받는 3D 카트 레이스(카트라이더 느낌). 3바퀴를 돌아서 AI 카트 셋보다
//! 먼저(1등으로) 들어오면 클리어. W/↑ 가속, S/↓ 브레이크, A·D/←→ 핸들, Shift(또는 Space 꾹) 드리프트 —
//! 코너에서 드리프트를 하다가 놓으면 부스터가 붙는다. 코스 밖 잔디에서는 느려지고 바깥 벽에 부딪히면 속도가
//! 크게 줄어든다. AI 가 먼저 결승선을 넘으면 그 순간 1등은 불가능하니 바로 실패다.
//!
//! 코스는 큰 순환로(main) 하나에 샛길이 붙은 모양이다. 갈림길이 둘: 동쪽은 세 갈래(가운데 큰길 / 안쪽 좁은
//! 지름길 / 바깥쪽 넓고 긴 길 + 부스터 발판), 서쪽은 두 갈래(바깥 큰길 / 안쪽 꺾이는 지름길). 샛길은 갈라지는
//! 점에서 큰길과 같은 방향으로 시작해서 합쳐지는 점에서 다시 큰길로 이어진다. 카트는 길 점들 가운데 가장
//! 가까운 곳으로 "지금 어디쯤인지"를 세는데, 샛길에서도 큰길 기준 진행 거리로 바꿔서 순위를 매긴다.
//! 화면은 apps/game3d 의 3D 렌더 도구를 그대로 쓴다.

use crate::random::Rng;
use crate::render::mesh3d::{Box3D, Camera};

use super::{Controls, Outcome};

pub const LAPS: i32 = 3;
pub const RACERS: usize = 4;
const WALL_GAP: f32 = 4.0; // 길 가장자리에서 바깥 벽까지
const MAX_SPEED: f32 = 26.0;
const GRASS_SPEED: f32 = 9.0;
const ACCEL: f32 = 15.0;
const BRAKE: f32 = 30.0;
const DRAG: f32 = 5.0;
const STEER_RATE: f32 = 2.1;
const DRIFT_MIN_SPEED: f32 = 12.0;
const DRIFT_CHARGE_SECS: f32 = 1.4;
const BOOST_BONUS: f32 = 8.0;
const KART_R: f32 = 1.1;
const COUNTDOWN: f32 = 3.0;
const SAMPLES_PER_SEGMENT: usize = 12;
const PAD_R: f32 = 3.2; // 부스터 발판에 닿는 거리
// 이 값이 AI 의 실력(최고 속도 비율)이다 — 사람이 코너를 잘 돌면 이길 만하게.
const AI_SKILL: [f32; RACERS - 1] = [0.76, 0.82, 0.88];

// 큰길의 뼈대 — 시계 반대 방향으로 한 바퀴(미터). 시작/결승선은 첫 점이다.
const CONTROL: [[f32; 2]; 14] = [
    [0.0, 0.0],
    [40.0, -5.0],
    [80.0, -10.0],
    [115.0, 5.0],
    [135.0, 40.0],
    [125.0, 80.0],
    [90.0, 100.0],
    [50.0, 95.0],
    [30.0, 120.0],
    [0.0, 135.0],
    [-35.0, 120.0],
    [-80.0, 85.0],
    [-75.0, 45.0],
    [-25.0, 25.0],
];

// 샛길 정의 — (큰길에서 갈라지는 점, 합쳐지는 점, 사이를 지나는 점들, 길 반폭, 곡선 색 종류).
struct DetourDef {
    entry: usize,
    exit: usize,
    via: &'static [[f32; 2]],
    half_w: f32,
}

const DETOURS: [DetourDef; 3] = [
    // 1: 동쪽 안쪽 지름길 — 좁고 짧다.
    DetourDef { entry: 36, exit: 72, via: &[[109.0, 30.0], [103.0, 54.0], [97.0, 77.0]], half_w: 4.5 },
    // 2: 동쪽 바깥길 — 넓지만 길다. 부스터 발판이 깔려 있다.
    DetourDef { entry: 36, exit: 72, via: &[[146.0, 14.0], [163.0, 44.0], [150.0, 86.0]], half_w: 7.5 },
    // 3: 서쪽 지름길 — 좁고 지그재그로 꺾인다.
    DetourDef { entry: 120, exit: 156, via: &[[-26.0, 98.0], [-36.0, 74.0], [-24.0, 50.0]], half_w: 4.5 },
];
const MAIN_HALF_W: f32 = 6.0;

// 갈림길: 어디서 갈라지고 어떤 길들이 고를 수 있는지(0 은 큰길 그대로), 그리고 AI 가 고르는 비중.
struct ForkDef {
    entry: usize,
    options: &'static [(usize, f32)],
}
const FORKS: [ForkDef; 2] = [ForkDef { entry: 36, options: &[(0, 0.4), (1, 0.4), (2, 0.2)] }, ForkDef { entry: 120, options: &[(0, 0.5), (3, 0.5)] }];
// 바깥길(2번 샛길)에 깔린 부스터 발판 — 샛길 위의 점 번호.
const PAD_AT: [usize; 3] = [10, 24, 38];

fn wrap_angle(a: f32) -> f32 {
    let mut a = a % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

// 앞으로 (-sin h, -cos h) — mesh3d 의 Camera/Box3D 와 같은 방향 정의.
fn forward(h: f32) -> [f32; 2] {
    [-h.sin(), -h.cos()]
}

fn catmull(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let (t2, t3) = (t * t, t * t * t);
    let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
    [f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1])]
}

// 길 하나 — 큰길(닫힌 순환)이거나 샛길(열린 구간). 샛길의 progress 는 큰길의 갈라지는 점~합쳐지는 점
// 구간에 비율로 대응시킨다(그래야 어느 길로 가든 순위를 같은 잣대로 비교한다).
pub struct Route {
    pub pts: Vec<[f32; 2]>,
    tang: Vec<[f32; 2]>,
    s: Vec<f32>, // 이 길 안에서 시작점부터의 누적 거리
    pub length: f32,
    pub half_w: f32,
    p0: f32, // 큰길 기준 진행 거리: 시작
    p1: f32, // 끝
    entry: usize,
    exit: usize,
    closed: bool,
}

impl Route {
    fn from_points(pts: Vec<[f32; 2]>, closed: bool, half_w: f32) -> Route {
        let m = pts.len();
        let mut tang = Vec::with_capacity(m);
        let mut s = Vec::with_capacity(m);
        let mut acc = 0.0;
        for i in 0..m {
            let next = if closed { pts[(i + 1) % m] } else { pts[(i + 1).min(m - 1)] };
            let prev = if closed || i + 1 < m { pts[i] } else { pts[i - 1] };
            let (dx, dz) = (next[0] - prev[0], next[1] - prev[1]);
            let len = (dx * dx + dz * dz).sqrt().max(1e-6);
            tang.push([dx / len, dz / len]);
            s.push(acc);
            if closed || i + 1 < m {
                acc += len;
            }
        }
        Route { pts, tang, s, length: acc, half_w, p0: 0.0, p1: 0.0, entry: 0, exit: 0, closed }
    }

    pub fn len(&self) -> usize {
        self.pts.len()
    }

    fn heading_at(&self, i: usize) -> f32 {
        let t = self.tang[i.min(self.len() - 1)];
        (-t[0]).atan2(-t[1])
    }
}

pub struct Track {
    pub routes: Vec<Route>, // 0 번이 큰길, 나머지는 샛길
    pub length: f32,        // 큰길 한 바퀴 길이
    pub pads: Vec<[f32; 2]>, // 부스터 발판 위치
}

// locate 의 결과: 어느 길의 몇 번째 점 근처인지, 중심선에서 옆으로 떨어진 거리(+ 가 왼쪽), 큰길 기준 진행 거리.
#[derive(Clone, Copy)]
struct Loc {
    route: usize,
    idx: usize,
    lateral: f32,
    progress: f32,
}

impl Track {
    pub fn new() -> Track {
        let n = CONTROL.len();
        let mut main_pts = Vec::with_capacity(n * SAMPLES_PER_SEGMENT);
        for i in 0..n {
            let (p0, p1, p2, p3) = (CONTROL[(i + n - 1) % n], CONTROL[i], CONTROL[(i + 1) % n], CONTROL[(i + 2) % n]);
            for k in 0..SAMPLES_PER_SEGMENT {
                main_pts.push(catmull(p0, p1, p2, p3, k as f32 / SAMPLES_PER_SEGMENT as f32));
            }
        }
        let mut main = Route::from_points(main_pts, true, MAIN_HALF_W);
        main.p1 = main.length;
        let length = main.length;
        let mut routes = vec![main];
        for def in &DETOURS {
            // 앞뒤에 큰길 이웃 점을 가짜 제어점으로 붙여서 갈라지는/합쳐지는 곳이 큰길과 같은 방향으로 이어지게 한다.
            let m = routes[0].len();
            let mut ctrl = vec![routes[0].pts[def.entry - SAMPLES_PER_SEGMENT], routes[0].pts[def.entry]];
            ctrl.extend(def.via.iter().copied());
            ctrl.push(routes[0].pts[def.exit]);
            ctrl.push(routes[0].pts[(def.exit + SAMPLES_PER_SEGMENT) % m]);
            let mut pts = Vec::new();
            for i in 1..ctrl.len() - 2 {
                for k in 0..SAMPLES_PER_SEGMENT {
                    pts.push(catmull(ctrl[i - 1], ctrl[i], ctrl[i + 1], ctrl[i + 2], k as f32 / SAMPLES_PER_SEGMENT as f32));
                }
            }
            pts.push(routes[0].pts[def.exit]);
            let mut route = Route::from_points(pts, false, def.half_w);
            route.entry = def.entry;
            route.exit = def.exit;
            route.p0 = routes[0].s[def.entry];
            route.p1 = routes[0].s[def.exit];
            routes.push(route);
        }
        let pads = PAD_AT.iter().map(|&i| routes[2].pts[i]).collect();
        Track { routes, length, pads }
    }

    // 점 p 에서 가장 가까운 길 점. 모든 길의 모든 점을 훑는다(점이 수백 개라 충분히 싸다).
    fn locate(&self, p: [f32; 2]) -> Loc {
        let mut best = (0usize, 0usize, f32::MAX);
        for (ri, r) in self.routes.iter().enumerate() {
            for (i, q) in r.pts.iter().enumerate() {
                let d = (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
                if d < best.2 {
                    best = (ri, i, d);
                }
            }
        }
        let (ri, i, _) = best;
        let r = &self.routes[ri];
        let t = r.tang[i];
        let rel = [p[0] - r.pts[i][0], p[1] - r.pts[i][1]];
        let along = rel[0] * t[0] + rel[1] * t[1];
        // 왼쪽 법선: 진행 방향 (tx, tz) 를 위에서 볼 때 반시계로 90도 돌린 것 = (tz, -tx).
        let lateral = rel[0] * t[1] - rel[1] * t[0];
        let own = r.s[i] + along;
        let progress = if r.closed { own } else { r.p0 + (r.p1 - r.p0) * (own / r.length).clamp(0.0, 1.0) };
        Loc { route: ri, idx: i, lateral, progress }
    }

    // (route, idx) 에서 steps 칸 앞으로 가 본 자리 — 갈림길에서는 plan(갈림길별로 고른 길)을 따른다.
    fn advance(&self, mut route: usize, mut idx: usize, steps: usize, plan: &[usize]) -> (usize, usize) {
        for _ in 0..steps {
            idx += 1;
            if route == 0 {
                idx %= self.routes[0].len();
                for (fi, fork) in FORKS.iter().enumerate() {
                    if idx == fork.entry && plan[fi] != 0 {
                        route = plan[fi];
                        idx = 0;
                        break;
                    }
                }
            } else if idx >= self.routes[route].len() - 1 {
                idx = self.routes[route].exit;
                route = 0;
            }
        }
        (route, idx)
    }
}

#[derive(Clone)]
struct Ai {
    skill: f32,
    offset: f32,
    mistake_in: f32,
    mistake_left: f32,
    mistake_dir: f32,
    plan: Vec<usize>, // 갈림길마다 고른 길(0 은 큰길)
    plan_lap: i32,    // 이 계획을 세운 바퀴 — 바퀴가 바뀌면 다시 고른다
    locked: bool,     // true 면 계획을 다시 안 고른다(테스트용 고정 경로)
}

#[derive(Clone)]
pub struct Kart {
    pub pos: [f32; 2],
    pub heading: f32,
    pub speed: f32,
    route: usize,
    idx: usize,
    last_p: f32,
    laps: i32, // 결승선을 넘은 횟수 - 1 (출발선 뒤에서 시작해서 -1)
    pub progress: f32, // 바퀴 수 * 길이 + 큰길 기준 진행 거리 — 순위를 매기는 값
    finish_time: Option<f32>,
    boost: f32,
    drift_charge: f32,
    drifting: bool,
    bump_cooldown: f32,
    ai: Option<Ai>,
}

// 한 프레임의 조작(사람이든 AI 든 같은 형태).
#[derive(Clone, Copy, Default)]
struct Drive {
    gas: bool,
    brake: bool,
    steer: f32, // + 가 왼쪽
    drift: bool,
}

impl Kart {
    fn new(track: &Track, slot: usize, ai: Option<Ai>) -> Kart {
        // 출발선 뒤에 2열로 선다(큰길 위).
        let main = &track.routes[0];
        let m = main.len();
        let row = slot / 2;
        let idx = (m - 4 - row * 3) % m;
        let side = if slot.is_multiple_of(2) { 2.2 } else { -2.2 };
        let t = main.tang[idx];
        let pos = [main.pts[idx][0] + t[1] * side, main.pts[idx][1] - t[0] * side];
        let loc = track.locate(pos);
        Kart { pos, heading: main.heading_at(idx), speed: 0.0, route: 0, idx, last_p: loc.progress, laps: -1, progress: loc.progress - track.length, finish_time: None, boost: 0.0, drift_charge: 0.0, drifting: false, bump_cooldown: 0.0, ai }
    }

    pub fn lap_number(&self) -> i32 {
        (self.laps + 1).clamp(1, LAPS)
    }

    fn step(&mut self, track: &Track, d: Drive, dt: f32) {
        let loc = track.locate(self.pos);
        let half_w = track.routes[loc.route].half_w;
        let on_grass = loc.lateral.abs() > half_w;
        let mut cap = if on_grass { GRASS_SPEED } else { MAX_SPEED };
        if let Some(ai) = &self.ai {
            cap *= ai.skill.max(0.5);
        }
        // 부스터 발판을 밟으면 한동안 부스터.
        if track.pads.iter().any(|p| (p[0] - self.pos[0]).powi(2) + (p[1] - self.pos[1]).powi(2) < PAD_R * PAD_R) {
            self.boost = self.boost.max(0.9);
        }
        if self.boost > 0.0 {
            cap += BOOST_BONUS;
            self.speed += 30.0 * dt;
            self.boost = (self.boost - dt).max(0.0);
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

        // 드리프트 — 코너에서 꾹 누르면 더 크게 돌고 충전된다. 놓으면 충전량만큼 부스터.
        let drifting = d.drift && d.steer.abs() > 0.3 && self.speed > DRIFT_MIN_SPEED && !on_grass;
        if drifting {
            self.drift_charge = (self.drift_charge + dt / DRIFT_CHARGE_SECS).min(1.0);
        } else if self.drifting {
            if self.drift_charge > 0.35 {
                self.boost = 0.6 + self.drift_charge * 1.0;
            }
            self.drift_charge = 0.0;
        }
        self.drifting = drifting;

        let grip = (self.speed.abs() / 7.0).min(1.0);
        let rate = STEER_RATE * grip * if drifting { 1.45 } else { 1.0 } * if self.speed < 0.0 { -1.0 } else { 1.0 };
        self.heading += d.steer * rate * dt;
        let f = forward(self.heading);
        self.pos[0] += f[0] * self.speed * dt;
        self.pos[1] += f[1] * self.speed * dt;

        // 바깥 벽 — 안쪽으로 밀어 넣고 속도를 크게 깎는다.
        self.bump_cooldown = (self.bump_cooldown - dt).max(0.0);
        let loc = track.locate(self.pos);
        let route = &track.routes[loc.route];
        let wall = route.half_w + WALL_GAP;
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
            // 벽에 비스듬히 박혀 못 빠져나오지 않게 길 방향으로 살짝 돌려놓는다.
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

pub struct DriveRound {
    pub track: Track,
    pub karts: Vec<Kart>, // 0 번이 플레이어
    t: f32,
    finished: Option<Outcome>,
    cam_heading: f32,
    scene: Vec<Box3D>, // 길/벽/배경 — 매 프레임 그대로 쓰는 고정 장면
    rng: Rng,
}

// 갈림길마다 가중치대로 길을 하나씩 고른다.
fn roll_plan(rng: &mut Rng, skill: f32) -> Vec<usize> {
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
    pub fn new(mut rng: Rng) -> DriveRound {
        let track = Track::new();
        let mut karts = vec![Kart::new(&track, 0, None)];
        for (i, &skill) in AI_SKILL.iter().enumerate() {
            let plan = roll_plan(&mut rng, skill);
            let ai = Ai { skill, offset: (rng.unit() - 0.5) * 3.0, mistake_in: rng.range_f32(4.0, 9.0), mistake_left: 0.0, mistake_dir: 0.0, plan, plan_lap: -1, locked: false };
            karts.push(Kart::new(&track, i + 1, Some(ai)));
        }
        let scene = build_scene(&track, &mut rng);
        let cam_heading = karts[0].heading;
        DriveRound { track, karts, t: 0.0, finished: None, cam_heading, scene, rng }
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

    pub fn boost_ratio(&self) -> f32 {
        (self.karts[0].boost / 1.6).clamp(0.0, 1.0)
    }

    pub fn drift_charge(&self) -> f32 {
        self.karts[0].drift_charge
    }

    // AI 한 대의 조작 — 앞쪽 길을 보고 핸들을 꺾고, 코너가 급하면 속도를 줄인다. 가끔 헛손질도 한다.
    fn ai_drive(&mut self, k: usize, dt: f32, player_progress: f32) -> Drive {
        let track = &self.track;
        let kart = &mut self.karts[k];
        let Some(mut ai) = kart.ai.take() else { return Drive::default() };
        // 바퀴가 바뀌면 갈림길에서 어느 길로 갈지 다시 정한다.
        if !ai.locked && ai.plan_lap != kart.laps {
            ai.plan = roll_plan(&mut self.rng, ai.skill);
            ai.plan_lap = kart.laps;
        }
        let look = 4 + (kart.speed.max(0.0) * 0.28) as usize;
        let (lr, li) = track.advance(kart.route, kart.idx, look, &ai.plan);
        let route = &track.routes[lr];
        let t = route.tang[li];
        let n = [t[1], -t[0]];
        let off = ai.offset.clamp(-route.half_w * 0.4, route.half_w * 0.4);
        let target = [route.pts[li][0] + n[0] * off, route.pts[li][1] + n[1] * off];
        let want = (-(target[0] - kart.pos[0])).atan2(-(target[1] - kart.pos[1]));
        let mut steer = (wrap_angle(want - kart.heading) * 2.6).clamp(-1.0, 1.0);

        // 헛손질: 몇 초에 한 번 잠깐 핸들이 엉뚱하게 꺾인다.
        ai.mistake_in -= dt;
        if ai.mistake_left > 0.0 {
            ai.mistake_left -= dt;
            steer = (steer + ai.mistake_dir).clamp(-1.0, 1.0);
        } else if ai.mistake_in <= 0.0 {
            ai.mistake_in = self.rng.range_f32(5.0, 11.0);
            ai.mistake_left = 0.5;
            ai.mistake_dir = if self.rng.unit() < 0.5 { -0.9 } else { 0.9 };
        }

        // 코너 정도 — 앞쪽 두 지점의 진행 방향 차이.
        let (ar, ai_idx) = track.advance(kart.route, kart.idx, look + 8, &ai.plan);
        let bend = wrap_angle(track.routes[ar].heading_at(ai_idx) - track.routes[kart.route].heading_at(kart.idx)).abs();
        // 고무줄: 사람과 너무 벌어지면 살짝 눌러주거나 밀어준다.
        let gap = kart.progress - player_progress;
        let rubber = if gap > 60.0 { 0.93 } else if gap < -60.0 { 1.06 } else { 1.0 };
        let target_speed = MAX_SPEED * ai.skill * rubber * (1.0 - (bend * 0.55).min(0.5));
        let drive = Drive { gas: kart.speed < target_speed, brake: kart.speed > target_speed + 4.0, steer, drift: ai.skill > 0.84 && bend > 0.5 };
        kart.ai = Some(ai);
        drive
    }

    pub fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        self.t += dt;
        if let Some(out) = self.finished {
            return out;
        }
        if self.countdown() > 0.0 {
            return Outcome::Running;
        }
        let me = Drive { gas: c.dy < 0.0, brake: c.dy > 0.0 || c.brake, steer: -c.dx, drift: c.hold };
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
        let pull = 6.5 + if me.boost > 0.0 { 1.2 } else { 0.0 };
        Camera { pos: [me.pos[0] - f[0] * pull, 3.2, me.pos[1] - f[1] * pull], yaw: self.cam_heading, pitch: -0.2 }
    }

    // 그릴 상자 전부 — 고정 장면 + 카트들.
    pub fn boxes(&self) -> Vec<Box3D> {
        let mut out = self.scene.clone();
        for (i, k) in self.karts.iter().enumerate() {
            kart_boxes(&mut out, k, i);
        }
        out
    }
}

fn b3(center: [f32; 3], half: [f32; 3], yaw: f32, color: [f32; 4]) -> Box3D {
    Box3D { center, half, yaw, pitch: 0.0, roll: 0.0, color, texture: None, walkable: false, solid: false }
}

// 카트 한 대 — 사람 없이 단순하게: 납작한 몸통, 앞 범퍼, 바퀴 넷, 뒤 날개 하나. 오프셋은 카트가 향한 쪽에 맞춰 돌린다.
fn kart_boxes(out: &mut Vec<Box3D>, k: &Kart, index: usize) {
    const COLORS: [[f32; 4]; RACERS] = [[0.9, 0.12, 0.15, 1.0], [0.2, 0.45, 0.95, 1.0], [0.95, 0.8, 0.15, 1.0], [0.25, 0.8, 0.35, 1.0]];
    let body = COLORS[index % RACERS];
    let (s, c) = k.heading.sin_cos();
    // 카트 로컬 (오른쪽 x, 위 y, 뒤 z) → 월드. 앞은 -z.
    let at = |x: f32, y: f32, z: f32| [k.pos[0] + x * c + z * s, y, k.pos[1] - x * s + z * c];
    let glow = if k.boost > 0.0 { [1.0, 0.7, 0.2, 1.0] } else { [0.12, 0.12, 0.14, 1.0] };
    out.push(b3(at(0.0, 0.55, 0.0), [0.75, 0.25, 1.3], k.heading, body));
    out.push(b3(at(0.0, 0.45, -1.4), [0.55, 0.12, 0.2], k.heading, [0.9, 0.9, 0.92, 1.0]));
    for (x, z) in [(-0.95, -0.85), (0.95, -0.85), (-0.95, 0.9), (0.95, 0.9)] {
        out.push(b3(at(x, 0.32, z), [0.16, 0.3, 0.3], k.heading, [0.06, 0.06, 0.07, 1.0]));
    }
    out.push(b3(at(0.0, 1.05, 1.2), [0.7, 0.06, 0.18], k.heading, body));
    out.push(b3(at(0.0, 0.55, 1.4), [0.3, 0.1, 0.12], k.heading, glow));
}

// 길 / 연석 / 바깥 벽 / 잔디 바닥 / 부스터 발판 / 먼 배경 건물 — 한 번만 만든다.
fn build_scene(track: &Track, rng: &mut Rng) -> Vec<Box3D> {
    let mut out = vec![b3([40.0, -0.1, 60.0], [260.0, 0.1, 260.0], 0.0, [0.22, 0.5, 0.2, 1.0])];
    for (ri, route) in track.routes.iter().enumerate() {
        let m = route.len();
        let segs = if route.closed { m } else { m - 1 };
        // 길 종류별 연석 색: 큰길 빨강/흰색, 안쪽 지름길 파랑/흰색, 바깥길 노랑/검정.
        let (c1, c2) = match ri {
            0 => ([0.9, 0.15, 0.15, 1.0], [0.95, 0.95, 0.95, 1.0]),
            2 => ([0.95, 0.8, 0.1, 1.0], [0.1, 0.1, 0.1, 1.0]),
            _ => ([0.15, 0.4, 0.95, 1.0], [0.95, 0.95, 0.95, 1.0]),
        };
        for i in 0..segs {
            let (a, b) = (route.pts[i], route.pts[(i + 1) % m]);
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let t = route.tang[i];
            let n = [t[1], -t[0]];
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt() * 0.62 + 0.02;
            let yaw = route.heading_at(i);
            let shade = if i % 2 == 0 { 0.34 } else { 0.31 };
            out.push(b3([mid[0], 0.05, mid[1]], [route.half_w, 0.05, len], yaw, [shade, shade, shade + 0.02, 1.0]));
            let curb = if (i / 2) % 2 == 0 { c1 } else { c2 };
            for side in [-1.0f32, 1.0] {
                let off = side * (route.half_w + 0.5);
                out.push(b3([mid[0] + n[0] * off, 0.07, mid[1] + n[1] * off], [0.5, 0.07, len], yaw, curb));
            }
            // 바깥 벽(세 점에 하나) — 다른 길과 겹치는 자리에는 세우지 않는다(샛길 입구를 막지 않게).
            if i % 3 == 0 {
                for side in [-1.0f32, 1.0] {
                    let off = side * (route.half_w + WALL_GAP + 0.4);
                    let wp = [mid[0] + n[0] * off, mid[1] + n[1] * off];
                    let blocked = track.routes.iter().enumerate().any(|(oj, other)| oj != ri && other.pts.iter().any(|q| (q[0] - wp[0]).powi(2) + (q[1] - wp[1]).powi(2) < (other.half_w + WALL_GAP + 2.0).powi(2)));
                    if !blocked {
                        out.push(b3([wp[0], 0.6, wp[1]], [0.4, 0.6, len * 3.0], yaw, [0.55, 0.55, 0.6, 1.0]));
                    }
                }
            }
        }
    }
    // 출발/결승선 — 길을 가로지르는 체커 띠와 위의 아치.
    let main = &track.routes[0];
    let t0 = main.tang[0];
    let n0 = [t0[1], -t0[0]];
    let yaw0 = main.heading_at(0);
    out.push(b3([main.pts[0][0], 0.11, main.pts[0][1]], [MAIN_HALF_W, 0.02, 0.5], yaw0, [0.95, 0.95, 0.95, 1.0]));
    for k in -2..=2 {
        let o = k as f32 * 2.4;
        out.push(b3([main.pts[0][0] + n0[0] * o, 0.125, main.pts[0][1] + n0[1] * o], [1.2, 0.02, 0.25], yaw0, [0.05, 0.05, 0.05, 1.0]));
    }
    for side in [-1.0f32, 1.0] {
        let off = side * (MAIN_HALF_W + 1.2);
        out.push(b3([main.pts[0][0] + n0[0] * off, 2.0, main.pts[0][1] + n0[1] * off], [0.25, 2.0, 0.25], yaw0, [0.9, 0.9, 0.9, 1.0]));
    }
    out.push(b3([main.pts[0][0], 4.2, main.pts[0][1]], [MAIN_HALF_W + 1.4, 0.45, 0.2], yaw0, [0.9, 0.2, 0.25, 1.0]));
    // 갈림길 표지 — 갈라지는 점 위에 선 알록달록한 아치(어느 길인지는 연석 색으로 구분한다).
    for fork in &FORKS {
        let p = main.pts[fork.entry];
        let yaw = main.heading_at(fork.entry);
        out.push(b3([p[0], 5.0, p[1]], [MAIN_HALF_W + 3.0, 0.35, 0.2], yaw, [0.95, 0.85, 0.2, 1.0]));
    }
    // 부스터 발판 — 바닥에 깔린 밝은 판.
    for pad in &track.pads {
        out.push(b3([pad[0], 0.13, pad[1]], [1.8, 0.02, 1.8], 0.0, [1.0, 0.6, 0.1, 1.0]));
        out.push(b3([pad[0], 0.15, pad[1]], [0.9, 0.02, 0.9], 0.0, [1.0, 0.95, 0.4, 1.0]));
    }
    // 길에서 충분히 떨어진 곳의 나무와 건물(속도감용 배경).
    let near_track = |p: [f32; 2]| track.routes.iter().any(|r| r.pts.iter().any(|q| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) < 22.0 * 22.0));
    let mut placed = 0;
    let mut tries = 0;
    while placed < 50 && tries < 900 {
        tries += 1;
        let p = [rng.range_f32(-170.0, 280.0), rng.range_f32(-120.0, 250.0)];
        if near_track(p) {
            continue;
        }
        placed += 1;
        if rng.unit() < 0.35 {
            let h = rng.range_f32(8.0, 24.0);
            let tone = rng.range_f32(0.25, 0.42);
            out.push(b3([p[0], h / 2.0, p[1]], [rng.range_f32(3.0, 6.0), h / 2.0, rng.range_f32(3.0, 6.0)], 0.0, [tone, tone + 0.03, tone + 0.07, 1.0]));
        } else {
            let h = rng.range_f32(3.0, 6.0);
            out.push(b3([p[0], h / 2.0, p[1]], [0.3, h / 2.0, 0.3], 0.0, [0.3, 0.2, 0.12, 1.0]));
            out.push(b3([p[0], h + 1.0, p[1]], [1.6, 1.2, 1.6], 0.0, [0.12, 0.4, 0.16, 1.0]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use super::*;

    // 사람 자리(0 번)에 AI 와 같은 운전 솜씨를 넣어 돌려 보는 봇. plan 으로 갈림길에서 어느 길로 갈지도 정할 수 있다.
    fn run_with_bot(seed: u64, skill: f32, drifts: bool, plan: Option<Vec<usize>>) -> (Outcome, usize, f32) {
        let mut r = DriveRound::new(Rng::new(seed));
        let fixed = plan.clone().unwrap_or_else(|| vec![0; FORKS.len()]);
        r.karts[0].ai = Some(Ai { skill, offset: 0.0, mistake_in: 1.0e9, mistake_left: 0.0, mistake_dir: 0.0, plan: fixed.clone(), plan_lap: 999, locked: true });
        let mut out = Outcome::Running;
        let mut time = 0.0;
        for _ in 0..(400.0 / 0.016) as usize {
            // 사람 카트 조작은 AI 컨트롤러가 정한 걸 그대로 쓴다(드리프트 여부와 갈 길만 고른다).
            let pp = r.karts[1].progress;
            let mut d = r.ai_drive(0, 0.016, pp);
            d.drift &= drifts;
            let c = Controls { dx: -d.steer, dy: if d.gas { -1.0 } else if d.brake { 1.0 } else { 0.0 }, hold: d.drift, ..Controls::default() };
            out = r.step(0.016, &c);
            time += 0.016;
            if !matches!(out, Outcome::Running) {
                break;
            }
        }
        (out, r.place(), time)
    }

    #[test]
    fn the_track_has_three_detours_branching_off_and_rejoining_the_main_road() {
        let t = Track::new();
        assert_eq!(t.routes.len(), 4, "큰길 + 샛길 셋");
        assert!(t.length > 400.0 && t.length < 900.0, "한 바퀴 길이 {}", t.length);
        let main = &t.routes[0];
        for r in &t.routes[1..] {
            let (a, b) = (main.pts[r.entry], main.pts[r.exit]);
            let (s, e) = (r.pts[0], r.pts[r.len() - 1]);
            assert!(((s[0] - a[0]).powi(2) + (s[1] - a[1]).powi(2)).sqrt() < 0.01, "갈라지는 점이 큰길 위에 있다");
            assert!(((e[0] - b[0]).powi(2) + (e[1] - b[1]).powi(2)).sqrt() < 0.01, "합쳐지는 점이 큰길 위에 있다");
        }
        // 갈림길은 두 군데이고, 동쪽은 세 갈래다.
        assert_eq!(FORKS.len(), 2);
        assert_eq!(FORKS[0].options.len(), 3);
        // 지름길은 큰길 같은 구간보다 짧고, 바깥길은 더 길다.
        let seg = |a: usize, b: usize| main.s[b] - main.s[a];
        assert!(t.routes[1].length < seg(36, 72) - 10.0 && t.routes[3].length < seg(120, 156) - 20.0);
        assert!(t.routes[2].length > seg(36, 72) + 30.0);
    }

    #[test]
    fn separate_roads_keep_their_walls_apart() {
        let t = Track::new();
        // 서로 다른 길(또는 같은 길의 먼 구간)은, 갈림길 근처(샛길의 처음/끝 14점)를 빼고는 벽끼리 닿지 않는다.
        let near_junction = |r: &Route, i: usize| r.closed.not() && (i < 14 || i + 14 >= r.len());
        for (ri, a) in t.routes.iter().enumerate() {
            for (rj, b) in t.routes.iter().enumerate().skip(ri) {
                for (i, p) in a.pts.iter().enumerate() {
                    for (j, q) in b.pts.iter().enumerate() {
                        if near_junction(a, i) || near_junction(b, j) {
                            continue;
                        }
                        if ri == rj {
                            let gap = (i as i32 - j as i32).unsigned_abs() as usize;
                            if gap.min(a.len() - gap) <= 30 {
                                continue;
                            }
                        }
                        let dist = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
                        // 샛길이 큰길과 나란히 붙는 구간(갈라지는/합쳐지는 점 근처의 큰길)은 큰길 쪽에서 막아 준다.
                        if (ri == 0) != (rj == 0) {
                            let (main_idx, det) = if ri == 0 { (i, b) } else { (j, a) };
                            if main_idx + 14 >= det.entry && main_idx <= det.exit + 14 && dist < 20.0 {
                                continue;
                            }
                        }
                        let need = a.half_w + b.half_w + 2.0 * WALL_GAP - 4.0;
                        assert!(dist > need, "길 {ri}[{i}] 와 {rj}[{j}] 가 너무 가깝다: {dist} < {need}");
                    }
                }
            }
        }
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
                let (out, _, time) = run_with_bot(4, 1.0, true, Some(vec![east, west]));
                assert!(!matches!(out, Outcome::Running), "경로 {east},{west}: 끝까지 못 갔다");
                times.push((east, west, time));
            }
        }
        // 바깥길은 더 길어서(발판이 있어도) 큰길보다 느리다. 지름길의 이득은 거리(위 테스트)로 확인한다 —
        // 이 봇은 코너 앞에서 보수적으로 감속해서 좁은 길의 이득을 사람보다 적게 본다.
        let time_of = |e: usize, w: usize| times.iter().find(|t| t.0 == e && t.1 == w).unwrap().2;
        assert!(time_of(2, 0) > time_of(0, 0), "바깥길은 큰길보다 느리다: {times:?}");
    }

    #[test]
    fn a_skilled_driver_wins_and_a_slow_one_loses() {
        let plans = [vec![0, 0], vec![1, 3], vec![0, 3], vec![1, 0]];
        let skilled = (1..=12u64).filter(|&s| matches!(run_with_bot(s, 1.0, true, Some(plans[s as usize % 4].clone())).0, Outcome::Won)).count();
        let slow = (1..=12u64).filter(|&s| matches!(run_with_bot(s, 0.7, false, Some(plans[s as usize % 4].clone())).0, Outcome::Won)).count();
        assert!(skilled >= 10, "잘 타면 1등: {skilled}/12");
        assert!(slow == 0, "느리면 1등 못 한다: {slow}/12");
    }

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

    #[test]
    fn leaving_the_track_slows_the_kart_and_the_wall_stops_it() {
        let r = DriveRound::new(Rng::new(5));
        // 길 한가운데에서 쭉 가속하는 것과 길 밖(잔디)에서 가속하는 것을 비교.
        let t = &r.track;
        let main = &t.routes[0];
        let i = 20;
        let n = [main.tang[i][1], -main.tang[i][0]];
        let mut on = Kart::new(t, 0, None);
        let mut off = Kart::new(t, 0, None);
        on.pos = main.pts[i];
        on.heading = main.heading_at(i);
        off.pos = [main.pts[i][0] + n[0] * (MAIN_HALF_W + 1.5), main.pts[i][1] + n[1] * (MAIN_HALF_W + 1.5)];
        off.heading = main.heading_at(i);
        for _ in 0..120 {
            on.step(t, Drive { gas: true, ..Drive::default() }, 0.016);
            off.step(t, Drive { gas: true, ..Drive::default() }, 0.016);
        }
        assert!(on.speed > 18.0, "길 위: {}", on.speed);
        assert!(off.speed < GRASS_SPEED + 0.5, "잔디: {}", off.speed);
        // 벽을 향해 계속 밀면 벽 밖으로 못 나간다.
        let mut w = Kart::new(t, 0, None);
        w.pos = main.pts[i];
        w.heading = main.heading_at(i) + std::f32::consts::FRAC_PI_2; // 길과 직각으로 왼쪽
        for _ in 0..600 {
            w.step(t, Drive { gas: true, ..Drive::default() }, 0.016);
        }
        let loc = t.locate(w.pos);
        assert!(loc.lateral.abs() <= t.routes[loc.route].half_w + WALL_GAP + 0.5, "벽 밖으로 나갔다: {}", loc.lateral);
    }

    #[test]
    fn drifting_then_releasing_gives_a_boost() {
        let r = DriveRound::new(Rng::new(2));
        let t = &r.track;
        let mut k = Kart::new(t, 0, None);
        k.pos = t.routes[0].pts[40];
        k.heading = t.routes[0].heading_at(40);
        // 지그재그로 꺾으며 드리프트를 누르고 있으면(길 밖으로 안 나가게) 충전된다.
        for n in 0..60 {
            k.speed = 20.0;
            let steer = if (n / 6) % 2 == 0 { 0.6 } else { -0.6 };
            k.step(t, Drive { gas: true, steer, drift: true, ..Drive::default() }, 0.016);
        }
        assert!(k.drift_charge > 0.5, "충전량 {}", k.drift_charge);
        k.step(t, Drive { gas: true, steer: 0.6, drift: false, ..Drive::default() }, 0.016);
        assert!(k.boost > 0.5, "놓으면 부스터가 붙는다");
    }

    #[test]
    fn pads_give_a_boost() {
        let r = DriveRound::new(Rng::new(2));
        let t = &r.track;
        let mut k = Kart::new(t, 0, None);
        k.pos = t.pads[0];
        k.speed = 15.0;
        k.step(t, Drive { gas: true, ..Drive::default() }, 0.016);
        assert!(k.boost > 0.5, "발판을 밟으면 부스터");
    }

    #[test]
    fn scene_stays_within_the_renderer_vertex_limit() {
        let r = DriveRound::new(Rng::new(9));
        let boxes = r.boxes();
        assert!(boxes.len() * 36 < 60_000, "상자 {}개는 한 번에 못 그린다(정점 u16 한도)", boxes.len());
    }
}
