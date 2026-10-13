//! 화면에 그릴 것들 — 길(삼각형 띠)과 가드레일(갈림길에서 정확히 끊기는 벽), 출발 깃발, 카트 모양.
//! 길 중심선에서 양옆으로 같은 거리에 점을 찍어 이은 띠라서 폭이 어디서나 같다.

use crate::apps::game3d::boxes::rotated;
use crate::render::mesh3d::{v_cross, v_dot, v_sub, Box3D, MeshData, MeshVertex};

use super::kart::Kart;
use super::track::{dist_to_route, Route, Track};
use super::RACERS;

pub(super) const ROAD_Y: f32 = 0.1; // 길 윗면의 높이
pub(super) const GUARD_H: f32 = 0.9; // 가드레일 높이
pub(super) const GUARD_T: f32 = 0.5; // 가드레일 두께

// 카트 한 대 — 사람 없이 단순하게: 몸통 하나와 바퀴 넷.
pub(super) fn kart_boxes(out: &mut Vec<Box3D>, k: &Kart, index: usize) {
    const COLORS: [[f32; 4]; RACERS] = [[0.9, 0.12, 0.15, 1.0], [0.2, 0.45, 0.95, 1.0], [0.95, 0.8, 0.15, 1.0], [0.25, 0.8, 0.35, 1.0]];
    let body = COLORS[index % RACERS];
    let (s, c) = k.heading.sin_cos();
    // 카트 로컬 (오른쪽 x, 위 y, 뒤 z) → 월드. 앞은 -z.
    let at = |x: f32, y: f32, z: f32| [k.pos[0] + x * c + z * s, y, k.pos[1] - x * s + z * c];
    out.push(rotated(at(0.0, 0.55, 0.0), [0.75, 0.25, 1.3], k.heading, body));
    for (x, z) in [(-0.95, -0.85), (0.95, -0.85), (-0.95, 0.9), (0.95, 0.9)] {
        out.push(rotated(at(x, 0.32, z), [0.16, 0.3, 0.3], k.heading, [0.06, 0.06, 0.07, 1.0]));
    }
}

// ---------- 길과 가드레일 메시 ----------

// 다른 길의 노면 위인지(ri 번 길 자신은 빼고). 두 길의 가장자리가 딱 겹쳐 있는 곳(갈림길의 시작/끝)은 번호가
// 작은 길의 가드레일만 남기고 큰 번호 쪽을 지운다 — 둘 다 지워져서 빈 곳이 생기는 일이 없게.
pub(super) fn inside_other_road(track: &Track, ri: usize, p: [f32; 2]) -> bool {
    track.routes.iter().enumerate().any(|(oj, other)| {
        if oj == ri {
            return false;
        }
        let d = dist_to_route(other, p);
        d < other.half_w - 0.05 || ((d - other.half_w).abs() <= 0.05 && oj < ri)
    })
}

// 길의 각 점에서 양옆으로 밀어낼 방향(왼쪽 단위 벡터)과 배율 — 꺾이는 곳에서는 앞/뒤 선분 법선의 평균을 쓰고,
// 두 선분과의 수직 거리가 똑같이 유지되게 1/cos 만큼 늘린다(너무 급한 꺾임은 1/0.5 에서 멈춘다).
pub(super) fn edge_dirs(route: &Route) -> Vec<([f32; 2], f32)> {
    let m = route.len();
    let seg_n = |i: usize| {
        let t = route.tang[i.min(m - 1)];
        [t[1], -t[0]]
    };
    (0..m)
        .map(|i| {
            let (a, b) = if route.closed {
                (seg_n((i + m - 1) % m), seg_n(i))
            } else if i == 0 {
                (seg_n(0), seg_n(0))
            } else if i == m - 1 {
                (seg_n(m - 2), seg_n(m - 2))
            } else {
                (seg_n(i - 1), seg_n(i))
            };
            let s = [a[0] + b[0], a[1] + b[1]];
            let l = (s[0] * s[0] + s[1] * s[1]).sqrt().max(1e-6);
            let nv = [s[0] / l, s[1] / l];
            let cos = (nv[0] * b[0] + nv[1] * b[1]).max(0.5);
            (nv, 1.0 / cos)
        })
        .collect()
}

pub(super) fn edge_point(route: &Route, dirs: &[([f32; 2], f32)], i: usize, d: f32) -> [f32; 2] {
    let (nv, k) = dirs[i % route.len()];
    let p = route.pts[i % route.len()];
    [p[0] + nv[0] * d * k, p[1] + nv[1] * d * k]
}

// 길 가장자리(왼쪽 -half_w 쪽이 -1, 오른쪽 +1) 한 변 위의 가드레일 조각들 — 다른 길의 노면 안으로 들어가는 곳에서
// 끊기고, 끊기는 자리는 노면 가장자리에 정확히 맞춰(이분 탐색) 잘라서 틈이나 삐져나온 끝이 없다.
// 각 점은 (안쪽 면 위치, 바깥 면 위치).
pub fn guard_runs(track: &Track) -> Vec<Vec<([f32; 2], [f32; 2])>> {
    let mut runs = Vec::new();
    for (ri, route) in track.routes.iter().enumerate() {
        let dirs = edge_dirs(route);
        let m = route.len();
        let n = if route.closed { m + 1 } else { m };
        for side in [-1.0f32, 1.0] {
            let pair = |k: usize| (edge_point(route, &dirs, k, side * route.half_w), edge_point(route, &dirs, k, side * (route.half_w + GUARD_T)));
            let lerp2 = |a: [f32; 2], b: [f32; 2], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            let kept = |k: usize| !inside_other_road(track, ri, pair(k).0);
            // 살아 있는 점 k0 와 잘린 점 k1 사이에서 경계를 찾아 그 자리의 (안, 밖) 쌍을 돌려준다.
            let cut = |k0: usize, k1: usize| {
                let (a, b) = (pair(k0), pair(k1));
                let (mut lo, mut hi) = (0.0f32, 1.0f32);
                for _ in 0..12 {
                    let mid = (lo + hi) / 2.0;
                    if inside_other_road(track, ri, lerp2(a.0, b.0, mid)) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                (lerp2(a.0, b.0, lo), lerp2(a.1, b.1, lo))
            };
            let flags: Vec<bool> = (0..n).map(kept).collect();
            let mut cur: Vec<([f32; 2], [f32; 2])> = Vec::new();
            for k in 0..n {
                if flags[k] {
                    if cur.is_empty() && k > 0 && !flags[k - 1] {
                        cur.push(cut(k, k - 1));
                    }
                    cur.push(pair(k));
                } else if !cur.is_empty() {
                    cur.push(cut(k - 1, k));
                    runs.push(std::mem::take(&mut cur));
                }
            }
            if !cur.is_empty() {
                runs.push(cur);
            }
        }
    }
    // 너무 짧은 조각은 버린다.
    runs.retain(|r| r.len() >= 2 && r.windows(2).map(|w| ((w[1].0[0] - w[0].0[0]).powi(2) + (w[1].0[1] - w[0].0[1]).powi(2)).sqrt()).sum::<f32>() > 1.5);
    runs
}

pub(super) struct MeshBuilder {
    pub(super) vertices: Vec<MeshVertex>,
    pub(super) triangles: Vec<[u32; 3]>,
}

impl MeshBuilder {
    pub(super) fn new() -> MeshBuilder {
        MeshBuilder { vertices: Vec::new(), triangles: Vec::new() }
    }

    // 사각형 하나 — 렌더러는 시계 반대(바깥에서 봤을 때)를 앞면으로 보니, outward 쪽을 향하도록 순서를 맞춘다.
    pub(super) fn quad(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], outward: [f32; 3]) {
        let n = v_cross(v_sub(b, a), v_sub(c, a));
        let pts = if v_dot(n, outward) >= 0.0 { [a, b, c, d] } else { [d, c, b, a] };
        let base = self.vertices.len() as u32;
        for p in pts {
            self.vertices.push(MeshVertex { pos: p, uv: [0.0, 0.0] });
        }
        self.triangles.push([base, base + 1, base + 2]);
        self.triangles.push([base, base + 2, base + 3]);
    }

    pub(super) fn finish(self) -> MeshData {
        MeshData { vertices: self.vertices, triangles: self.triangles }
    }
}

pub(super) fn v3(p: [f32; 2], y: f32) -> [f32; 3] {
    [p[0], y, p[1]]
}

// (길 메시, 가드레일 메시).
pub(super) fn build_meshes(track: &Track) -> (MeshData, MeshData) {
    // 길 — 중심선을 따라 양옆으로 반폭만큼 떨어진 점들을 이은 띠. 폭이 어디서나 같다.
    let mut road = MeshBuilder::new();
    for route in &track.routes {
        let dirs = edge_dirs(route);
        let m = route.len();
        let segs = if route.closed { m } else { m - 1 };
        for i in 0..segs {
            let (l0, r0) = (edge_point(route, &dirs, i, -route.half_w), edge_point(route, &dirs, i, route.half_w));
            let (l1, r1) = (edge_point(route, &dirs, i + 1, -route.half_w), edge_point(route, &dirs, i + 1, route.half_w));
            road.quad(v3(l0, ROAD_Y), v3(r0, ROAD_Y), v3(r1, ROAD_Y), v3(l1, ROAD_Y), [0.0, 1.0, 0.0]);
        }
    }
    // 가드레일 — 길 가장자리에 딱 붙은 얇은 벽(윗면, 안쪽 면, 바깥 면, 양 끝 마개).
    let mut guard = MeshBuilder::new();
    let top = ROAD_Y + GUARD_H;
    for run in guard_runs(track) {
        for w in run.windows(2) {
            let ((i0, o0), (i1, o1)) = (w[0], w[1]);
            let to_outer = [o0[0] - i0[0], 0.0, o0[1] - i0[1]];
            let inward = [-to_outer[0], 0.0, -to_outer[2]];
            guard.quad(v3(i0, top), v3(o0, top), v3(o1, top), v3(i1, top), [0.0, 1.0, 0.0]);
            guard.quad(v3(i0, ROAD_Y), v3(i1, ROAD_Y), v3(i1, top), v3(i0, top), inward);
            guard.quad(v3(o0, ROAD_Y), v3(o1, ROAD_Y), v3(o1, top), v3(o0, top), to_outer);
        }
        let (first, last) = (run[0], run[run.len() - 1]);
        let (a, b) = (run[1], run[run.len() - 2]);
        let back = [first.0[0] - a.0[0], 0.0, first.0[1] - a.0[1]];
        let fwd = [last.0[0] - b.0[0], 0.0, last.0[1] - b.0[1]];
        guard.quad(v3(first.0, ROAD_Y), v3(first.1, ROAD_Y), v3(first.1, top), v3(first.0, top), back);
        guard.quad(v3(last.0, ROAD_Y), v3(last.1, ROAD_Y), v3(last.1, top), v3(last.0, top), fwd);
    }
    (road.finish(), guard.finish())
}

// 출발선의 깃발 — 길 왼쪽 가드레일 옆에 기둥 하나와 빨간 깃발 하나.
pub(super) fn build_flag(track: &Track) -> Vec<Box3D> {
    let main = &track.routes[0];
    let t0 = main.tang[0];
    let n0 = [t0[1], -t0[0]];
    let yaw0 = main.heading_at(0);
    let c0 = main.pts[0];
    let side = main.half_w + GUARD_T + 0.5;
    let base = [c0[0] + n0[0] * side, c0[1] + n0[1] * side];
    vec![
        rotated([base[0], 2.0, base[1]], [0.12, 2.0, 0.12], yaw0, [0.9, 0.9, 0.9, 1.0]),
        rotated([base[0] - n0[0] * 0.9, 3.5, base[1] - n0[1] * 0.9], [0.9, 0.5, 0.04], yaw0, [0.9, 0.1, 0.12, 1.0]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::track::{seg_dist, ROAD_HALF_W};
    use crate::random::Rng;
    use super::super::DriveRound;

    #[test]
    fn the_road_has_the_same_width_everywhere() {
        let t = Track::new();
        for route in &t.routes {
            let dirs = edge_dirs(route);
            let mut worst = 1.0f32;
            for i in 1..route.len() - 1 {
                for side in [-1.0f32, 1.0] {
                    let e = edge_point(route, &dirs, i, side * route.half_w);
                    let d = dist_to_route(route, e);
                    // 꺾인 곳의 바깥쪽은 가장 가까운 점이 꼭짓점이라 반폭보다 조금 멀 수 있다(안쪽/곧은 곳은 정확히 반폭).
                    assert!(d > route.half_w * 0.97 && d < route.half_w * 1.25, "길 가장자리 {i}: 반폭 {} 인데 거리 {d}", route.half_w);
                    worst = worst.min(d / route.half_w);
                }
            }
            assert!(worst > 0.97);
        }
        // 모든 길의 폭이 같다.
        assert!(t.routes.iter().all(|r| (r.half_w - ROAD_HALF_W).abs() < 1e-6));
    }

    // 가드레일은 어떤 길의 노면 위에도 놓이지 않고, 길 가장자리에 붙어 있다.

    #[test]
    fn guard_rails_never_sit_on_a_road_and_hug_the_edge() {
        let t = Track::new();
        for run in guard_runs(&t) {
            for (inner, _) in &run {
                let nearest = t.routes.iter().map(|r| (dist_to_route(r, *inner) - r.half_w).abs()).fold(f32::MAX, f32::min);
                assert!(nearest < 0.35, "가드레일이 길 가장자리에서 떨어져 있다: {nearest} ({inner:?})");
                let on_road = t.routes.iter().filter(|r| dist_to_route(r, *inner) < r.half_w - 0.35).count();
                assert_eq!(on_road, 0, "가드레일이 노면 위에 놓였다: {inner:?}");
            }
        }
    }

    // 다른 길과 겹치지 않는 가장자리는 빠짐없이 가드레일이 있다(빈 곳 없음) — 길 가장자리를 0.5m 마다 찍어 본다.

    #[test]
    fn every_open_edge_has_a_guard_rail() {
        let t = Track::new();
        let runs = guard_runs(&t);
        let on_runs = |p: [f32; 2]| runs.iter().any(|run| run.windows(2).any(|w| seg_dist(p, w[0].0, w[1].0) < 0.6));
        let mut checked = 0;
        for (ri, route) in t.routes.iter().enumerate() {
            let dirs = edge_dirs(route);
            let m = route.len();
            let segs = if route.closed { m } else { m - 1 };
            for i in 0..segs {
                for side in [-1.0f32, 1.0] {
                    let (a, b) = (edge_point(route, &dirs, i, side * route.half_w), edge_point(route, &dirs, i + 1, side * route.half_w));
                    for k in 0..6 {
                        let p = [a[0] + (b[0] - a[0]) * k as f32 / 6.0, a[1] + (b[1] - a[1]) * k as f32 / 6.0];
                        // 다른 길과 겹치는 곳(과 그 가장자리 0.8m 안)은 건너뛴다.
                        let near_other = t.routes.iter().enumerate().any(|(oj, o)| oj != ri && dist_to_route(o, p) < o.half_w + 0.8);
                        if near_other {
                            continue;
                        }
                        checked += 1;
                        assert!(on_runs(p), "길 {ri} 의 가장자리에 가드레일이 비었다: {p:?}");
                    }
                }
            }
        }
        assert!(checked > 1500, "{checked}");
    }

    // 가드레일 한 조각의 양 끝은 갈림길의 노면 가장자리에 맞춰 잘려서, 끝이 노면 위로 삐져나오지 않는다.

    #[test]
    fn guard_rail_ends_stop_at_the_edge_of_the_other_road() {
        let t = Track::new();
        for run in guard_runs(&t) {
            for end in [run[0].0, run[run.len() - 1].0] {
                let into_road = t.routes.iter().map(|r| r.half_w - dist_to_route(r, end)).fold(f32::MIN, f32::max);
                assert!(into_road < 0.35, "가드레일 끝이 노면 안으로 {into_road} 만큼 들어가 있다: {end:?}");
            }
        }
    }

    // 메시가 렌더러의 정점 한도 안이고 비어 있지 않다.

    #[test]
    fn the_meshes_fit_the_renderer() {
        let r = DriveRound::new(Rng::new(9));
        let meshes = r.meshes();
        let verts: usize = meshes.iter().map(|m| m.mesh.triangles.len() * 3).sum();
        assert!(verts > 1000 && verts < 60_000, "삼각형 정점 {verts}");
        assert!(r.boxes().len() * 36 + verts < 60_000);
    }
}
