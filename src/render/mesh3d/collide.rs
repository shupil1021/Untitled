//! 바닥 높이와 수평 충돌(상자 / 삼각형 메시).

use super::*;

// ================= 충돌/바닥 높이 =================
//
// 레이-삼각형 교차(뫌러-트룸보어) — 수직으로 아래를 향해 쏴서 "이 xz 위치, 이
// 높이보다 아래에 있는 가장 높은 walkable 상자 윗면"을 찾는 데 쓴다. 상자가
// 기울어 있어도(경사로) 삼각형 자체가 기울어 있으니 교차점의 y 가 자동으로
// 경사를 따라간다 — 경사로 전용 특수 코드가 없다.
fn ray_triangle(orig: Vec3, dir: Vec3, tri: (Vec3, Vec3, Vec3)) -> Option<f32> {
    const EPS: f32 = 1e-6;
    let (v0, v1, v2) = tri;
    let e1 = v_sub(v1, v0);
    let e2 = v_sub(v2, v0);
    let h = v_cross(dir, e2);
    let a = v_dot(e1, h);
    if a.abs() < EPS {
        return None;
    }
    let f = 1.0 / a;
    let s = v_sub(orig, v0);
    let u = f * v_dot(s, h);
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = v_cross(s, e1);
    let v = f * v_dot(dir, q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = f * v_dot(e2, q);
    if t > EPS { Some(t) } else { None }
}

// probe_y 근처(위아래 search_range 이내)에서 (x,z) 바로 아래에 있는 walkable
// 상자들의 윗면(경사로 포함) 중 가장 높은 교차 높이를 찾는다 — 없으면 None(발
// 디딜 데가 없다, 낙하시켜야 한다).
pub fn ground_height(x: f32, z: f32, probe_y: f32, search_range: f32, boxes: &[Box3D]) -> Option<f32> {
    let orig = [x, probe_y + search_range, z];
    let dir = [0.0, -1.0, 0.0];
    let mut best: Option<f32> = None;
    for b in boxes {
        if !b.walkable {
            continue;
        }
        for tri in b.top_world_triangles() {
            if let Some(t) = ray_triangle(orig, dir, tri) {
                let hit_y = orig[1] - t;
                if hit_y <= probe_y + search_range && best.is_none_or(|by| hit_y > by) {
                    best = Some(hit_y);
                }
            }
        }
    }
    best
}

// 플레이어를 수평(XZ)으로만 상자들에서 밀어낸다 — 상자 로컬 공간으로 옮겨 AABB
// 클램프한 뒤 되돌리는 "가장 가까운 점" 방식이라 회전된(기울어진) 상자에도 그대로
// 맞는다. 발~머리 구간이 상자의 로컬 세로 범위와 겹치지 않으면(예: 발밑을 이미
// 지나간 경사로, 또는 한참 위 천장) 건드리지 않는다 — 그래야 경사로를 다 오른 뒤
// 그 상자 옆을 스쳐 지나가도 막히지 않는다.
pub fn resolve_horizontal(pos: Vec3, radius: f32, feet_y: f32, height: f32, boxes: &[Box3D]) -> Vec3 {
    // 발이 이 상자의 로컬 윗면 높이 근처거나 그 위면(딱 그 위에 서 있는 경우,
    // 걸을 수 있는 경사로/발판 포함) 수평 충돌 대상이 아니다 — 그래야 경사로를
    // 오르내리는 동안 그 경사로 상자 자체에 옆으로 밀려나지 않는다. STEP_EPS 는
    // "발이 윗면에 정확히 닿아있다"를 부동소수 오차 없이 판정하기 위한 여유.
    const STEP_EPS: f32 = 0.05;
    let mut p = pos;
    for b in boxes {
        if !b.solid {
            continue;
        }
        let local_feet_y = b.world_to_local([p[0], feet_y, p[2]])[1];
        let local_head_y = b.world_to_local([p[0], feet_y + height, p[2]])[1];
        if local_feet_y >= b.half[1] - STEP_EPS {
            continue; // 이 상자 위(또는 그 위 허공)에 서 있다 — 안 막는다
        }
        if local_head_y <= -b.half[1] {
            continue; // 상자가 한참 위(천장)라 머리도 안 닿는다
        }
        let local = b.world_to_local(p);
        let dx = local[0].clamp(-b.half[0], b.half[0]) - local[0];
        let dz = local[2].clamp(-b.half[2], b.half[2]) - local[2];
        let inside_x = local[0] > -b.half[0] - radius && local[0] < b.half[0] + radius;
        let inside_z = local[2] > -b.half[2] - radius && local[2] < b.half[2] + radius;
        if !(inside_x && inside_z) {
            continue;
        }
        // 상자 안에 파묻혀 있으면(dx==dz==0) 더 좁은 쪽 축으로 밀어낸다.
        let (push_x, push_z) = if dx == 0.0 && dz == 0.0 {
            let ox = b.half[0] + radius - local[0].abs();
            let oz = b.half[2] + radius - local[2].abs();
            if ox < oz { (local[0].signum() * ox, 0.0) } else { (0.0, local[2].signum() * oz) }
        } else {
            let dist = (dx * dx + dz * dz).sqrt();
            if dist >= radius || dist < 1e-5 {
                continue;
            }
            let push = radius - dist;
            (-dx / dist * push, -dz / dist * push)
        };
        let local_pushed = [local[0] + push_x, local[1], local[2] + push_z];
        let world_pushed = b.local_to_world(local_pushed);
        p[0] = world_pushed[0];
        p[2] = world_pushed[2];
    }
    p
}

// ---- MeshObject 용 — 같은 두 가지(바닥 높이/수평 밀어내기)를 임의 삼각형
// 메시에 대해 "정밀"하게 한다. Box3D 는 상자 로컬 공간 AABB 클램프라는 지름길이
// 있었지만, 메시는 삼각형마다 방향이 제각각이라 그 지름길이 없다 — 실제
// 점-삼각형 최근접점(closest_point_on_triangle)을 계산해서 판정한다.

// 점 p 에서 삼각형(a,b,c)까지 가장 가까운 점 — 표준 알고리즘(Ericson, Real-Time
// Collision Detection 5.1.5): 정점/변/면 영역을 순서대로 걸러낸다.
fn closest_point_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = v_sub(b, a);
    let ac = v_sub(c, a);
    let ap = v_sub(p, a);
    let d1 = v_dot(ab, ap);
    let d2 = v_dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a; // 꼭짓점 a 영역
    }

    let bp = v_sub(p, b);
    let d3 = v_dot(ab, bp);
    let d4 = v_dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b; // 꼭짓점 b 영역
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return v_add(a, v_scale(ab, v)); // 변 ab 영역
    }

    let cp = v_sub(p, c);
    let d5 = v_dot(ab, cp);
    let d6 = v_dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c; // 꼭짓점 c 영역
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return v_add(a, v_scale(ac, w)); // 변 ac 영역
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return v_add(b, v_scale(v_sub(c, b), w)); // 변 bc 영역
    }

    // 면 내부 — 무게중심 좌표로.
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    v_add(a, v_add(v_scale(ab, v), v_scale(ac, w)))
}

// ground_height() 의 메시 버전 — walkable 인 메시의 모든 삼각형(경사면도 그냥
// 삼각형이라 자동으로 따라간다)에 수직 레이를 쏴서 가장 높은 교차점을 찾는다.
pub fn mesh_ground_height(x: f32, z: f32, probe_y: f32, search_range: f32, meshes: &[MeshObject]) -> Option<f32> {
    let orig = [x, probe_y + search_range, z];
    let dir = [0.0, -1.0, 0.0];
    let mut best: Option<f32> = None;
    for m in meshes {
        if !m.walkable {
            continue;
        }
        for tri in m.world_triangles() {
            if let Some(t) = ray_triangle(orig, dir, tri) {
                let hit_y = orig[1] - t;
                if hit_y <= probe_y + search_range && best.is_none_or(|by| hit_y > by) {
                    best = Some(hit_y);
                }
            }
        }
    }
    best
}

// resolve_horizontal() 의 메시 버전 — 플레이어를 발~머리 사이 높이의 중간점
// 하나로 표본화해(원기둥을 선분 하나로 근사) 그 점에서 각 삼각형까지 최근접점을
// 구하고, 그 최근접점이 발~머리 범위(STEP_EPS 여유) 안에 있으면서 수평 거리가
// radius 보다 가까우면 그만큼 수평으로 밀어낸다. Box3D 버전처럼 완전히 정확한
// 원기둥-삼각형 충돌은 아니지만(선분이 아니라 점 하나로 표본화), 이 프로젝트
// 규모의 소품 충돌에는 충분하고 훨씬 단순하다.
pub fn mesh_resolve_horizontal(pos: Vec3, radius: f32, feet_y: f32, height: f32, meshes: &[MeshObject]) -> Vec3 {
    const STEP_EPS: f32 = 0.05;
    let mut p = pos;
    for m in meshes {
        if !m.solid {
            continue;
        }
        let probe = [p[0], feet_y + height * 0.5, p[2]];
        for (a, b, c) in m.world_triangles() {
            let cp = closest_point_on_triangle(probe, a, b, c);
            if cp[1] < feet_y - STEP_EPS || cp[1] > feet_y + height + STEP_EPS {
                continue; // 발밑보다 한참 아래이거나 머리 위 — 이 삼각형은 안 막는다
            }
            let dx = cp[0] - p[0];
            let dz = cp[2] - p[2];
            let dist = (dx * dx + dz * dz).sqrt();
            if dist >= radius || dist < 1e-5 {
                continue;
            }
            let push = radius - dist;
            p[0] -= dx / dist * push;
            p[2] -= dz / dist * push;
        }
    }
    p
}
