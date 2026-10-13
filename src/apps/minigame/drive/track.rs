//! 코스 — 닫힌 큰길 하나와 거기서 갈라졌다 합쳐지는 샛길 셋(길 중심선의 점들), 갈림길 정의, "이 점이
//! 어느 길의 어디쯤인가"(locate)와 "앞으로 n 칸 가면 어디인가"(advance). 길 폭(반폭)은 모든 길이 같다.

use super::SAMPLES_PER_SEGMENT;

// 큰길의 뼈대 — 시계 반대 방향으로 한 바퀴(미터). 시작/결승선은 첫 점이다.
pub(super) const CONTROL: [[f32; 2]; 14] = [
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

// 샛길 정의 — 큰길의 entry~exit 구간을 옆으로(offset, 왼쪽이 +) 밀어낸 길이다. 밀어내는 양은 양 끝의
// ramp 칸 동안 부드럽게 0 에서 늘었다 줄어서, 큰길에서 같은 방향으로 완만하게 갈라지고 완만하게 합쳐진다.
// 안쪽(−)으로 밀면 코너를 질러가는 지름길, 바깥쪽(+)으로 밀면 넓게 도는 긴 길이 된다.
pub(super) struct DetourDef {
    pub(super) entry: usize,
    pub(super) exit: usize,
    pub(super) offset: f32,
    pub(super) ramp: usize,
    pub(super) half_w: f32,
}

pub(super) const DETOURS: [DetourDef; 3] = [
    // 1: 동쪽 안쪽 지름길 — 짧다.
    DetourDef { entry: 28, exit: 80, offset: -18.0, ramp: 18, half_w: ROAD_HALF_W },
    // 2: 동쪽 바깥길 — 완만하지만 길다.
    DetourDef { entry: 28, exit: 80, offset: 22.0, ramp: 18, half_w: ROAD_HALF_W },
    // 3: 서쪽 안쪽 지름길 — 짧다(큰길의 서쪽 코너가 급해서 안쪽으로 많이 못 민다).
    DetourDef { entry: 108, exit: 162, offset: -13.0, ramp: 18, half_w: ROAD_HALF_W },
];
// 모든 길의 폭은 같다(반폭 5m).
pub(super) const ROAD_HALF_W: f32 = 5.0;
pub(super) const MAIN_HALF_W: f32 = ROAD_HALF_W;

// 갈림길: 어디서 갈라지고 어떤 길들이 고를 수 있는지(0 은 큰길 그대로), 그리고 AI 가 고르는 비중.
pub(super) struct ForkDef {
    pub(super) entry: usize,
    pub(super) options: &'static [(usize, f32)],
}
pub(super) const FORKS: [ForkDef; 2] = [ForkDef { entry: 28, options: &[(0, 0.4), (1, 0.4), (2, 0.2)] }, ForkDef { entry: 108, options: &[(0, 0.5), (3, 0.5)] }];

pub(super) fn wrap_angle(a: f32) -> f32 {
    let mut a = a % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

// 앞으로 (-sin h, -cos h) — mesh3d 의 Camera/Box3D 와 같은 방향 정의.
pub(super) fn forward(h: f32) -> [f32; 2] {
    [-h.sin(), -h.cos()]
}

pub(super) fn catmull(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let (t2, t3) = (t * t, t * t * t);
    let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * ((2.0 * b) + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
    [f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1])]
}

// 길 하나 — 큰길(닫힌 순환)이거나 샛길(열린 구간). 샛길의 progress 는 큰길의 갈라지는 점~합쳐지는 점
// 구간에 비율로 대응시킨다(그래야 어느 길로 가든 순위를 같은 잣대로 비교한다).
pub struct Route {
    pub pts: Vec<[f32; 2]>,
    pub(super) tang: Vec<[f32; 2]>,
    pub(super) s: Vec<f32>, // 이 길 안에서 시작점부터의 누적 거리
    pub length: f32,
    pub half_w: f32,
    pub(super) p0: f32, // 큰길 기준 진행 거리: 시작
    pub(super) p1: f32, // 끝
    pub(super) entry: usize,
    pub(super) exit: usize,
    pub(super) closed: bool,
}

impl Route {
    pub(super) fn from_points(pts: Vec<[f32; 2]>, closed: bool, half_w: f32) -> Route {
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

    pub fn is_empty(&self) -> bool {
        self.pts.is_empty()
    }

    pub(super) fn heading_at(&self, i: usize) -> f32 {
        let t = self.tang[i.min(self.len() - 1)];
        (-t[0]).atan2(-t[1])
    }
}

pub struct Track {
    pub routes: Vec<Route>, // 0 번이 큰길, 나머지는 샛길
    pub length: f32,        // 큰길 한 바퀴 길이
}

// locate 의 결과: 어느 길의 몇 번째 점 근처인지, 중심선에서 옆으로 떨어진 거리(+ 가 왼쪽), 큰길 기준 진행 거리.
#[derive(Clone, Copy)]
pub(super) struct Loc {
    pub(super) route: usize,
    pub(super) idx: usize,
    pub(super) lateral: f32,
    pub(super) progress: f32,
}

impl Default for Track {
    fn default() -> Self {
        Track::new()
    }
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
            // 큰길의 점마다 왼쪽 법선 방향으로 밀어낸 점들 — 샛길의 i 번째 점은 큰길의 entry+i 번째 점에 대응한다.
            let smooth = |x: f32| {
                let x = x.clamp(0.0, 1.0);
                x * x * (3.0 - 2.0 * x)
            };
            let mut pts = Vec::new();
            for i in def.entry..=def.exit {
                let (p, t) = (routes[0].pts[i], routes[0].tang[i]);
                let n = [t[1], -t[0]];
                let amount = def.offset * smooth((i - def.entry) as f32 / def.ramp as f32) * smooth((def.exit - i) as f32 / def.ramp as f32);
                pts.push([p[0] + n[0] * amount, p[1] + n[1] * amount]);
            }
            let mut route = Route::from_points(pts, false, def.half_w);
            route.entry = def.entry;
            route.exit = def.exit;
            route.p0 = routes[0].s[def.entry];
            route.p1 = routes[0].s[def.exit];
            routes.push(route);
        }
        Track { routes, length }
    }

    // 점 p 에서 가장 가까운 길 점. 모든 길의 모든 점을 훑는다(점이 수백 개라 충분히 싸다).
    pub(super) fn locate(&self, p: [f32; 2]) -> Loc {
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
    pub(super) fn advance(&self, mut route: usize, mut idx: usize, steps: usize, plan: &[usize]) -> (usize, usize) {
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

// 점 p 에서 선분 a-b 까지의 거리.
pub(super) fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (abx, abz) = (b[0] - a[0], b[1] - a[1]);
    let len2 = abx * abx + abz * abz;
    let t = if len2 < 1e-9 { 0.0 } else { (((p[0] - a[0]) * abx + (p[1] - a[1]) * abz) / len2).clamp(0.0, 1.0) };
    ((p[0] - a[0] - abx * t).powi(2) + (p[1] - a[1] - abz * t).powi(2)).sqrt()
}

// 점 p 에서 길 중심선(꺾은선)까지의 거리.
pub(super) fn dist_to_route(route: &Route, p: [f32; 2]) -> f32 {
    let m = route.len();
    let segs = if route.closed { m } else { m - 1 };
    (0..segs).map(|i| seg_dist(p, route.pts[i], route.pts[(i + 1) % m])).fold(f32::MAX, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(t.routes[1].length < seg(28, 80) - 20.0 && t.routes[3].length < seg(108, 162) - 20.0);
        assert!(t.routes[2].length > seg(28, 80) + 30.0);
    }
}
