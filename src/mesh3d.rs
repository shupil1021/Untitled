//! 진짜 3D 메쉬 렌더러 — 예전엔 컬럼 하나에 광선 하나, 카메라 피치도 없는 2D
//! 그리드 기반 레이캐스팅 엔진(raycaster.rs)을 썼는데, 그 구조로는 바닥 높낮이나
//! 기울어진 벽을 표현할 수 없어서 이 모듈로 완전히 대체했다(raycaster.rs 는
//! 지웠다). GPU 깊이 테스트가 있는 실제 삼각형 래스터라이저(miniquad 파이프라인)
//! 라서, 벽/바닥을 전부 `Box3D`(회전 가능한 직육면체) 하나로 표현하고 그 회전
//! (yaw/pitch/roll)만으로 "기울어진 벽"도 "경사로(램프)"도 자연스럽게 나온다 —
//! 별도의 특수 케이스가 필요 없다.
//!
//! 렌더링 방식: `Box3D` 각각을 CPU 에서 월드 공간 삼각형(12개, 면마다 2개)으로 펼친
//! 뒤, 텍스처별로 묶어 GPU 에 올리고 깊이 테스트를 켠 오프스크린 렌더 타깃에 그린다.
//! 그 결과 컬러 텍스처를 `gfx::Renderer::sprite_uv`로 일반 2D 창 안에 스프라이트처럼
//! 끼워 넣으면(video.rs 가 디코딩한 영상 프레임을 텍스처로 올려 보여주는 것과 같은
//! 요령) 나머지 UI(HUD 등)는 지금까지처럼 2D 배칭 렌더러로 그대로 그릴 수 있다.

use miniquad::*;

// ================= 최소 벡터/행렬 수학 =================
// glam 같은 별도 크레이트를 새로 끌어오는 대신, 이 파일에서만 쓰는 만큼만 직접
// 구현한다 — Box3D/카메라 변환에 필요한 것만 있으면 된다.

pub type Vec3 = [f32; 3];

pub fn v_add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn v_sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn v_scale(a: Vec3, s: f32) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
pub fn v_dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn v_cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn v_len(a: Vec3) -> f32 {
    v_dot(a, a).sqrt()
}
pub fn v_norm(a: Vec3) -> Vec3 {
    let l = v_len(a);
    if l > 1e-6 { v_scale(a, 1.0 / l) } else { a }
}

// 행 우선(row-major) 4x4 — 열벡터에 왼쪽에서 곱한다(m * v). mat_mul(a, b) 는
// "a 다음에 b" 가 아니라 수학 표기 그대로 a*b(먼저 b, 그다음 a 적용)이다.
pub type Mat4 = [[f32; 4]; 4];

pub fn mat_identity() -> Mat4 {
    let mut m = [[0.0; 4]; 4];
    for (i, row) in m.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    m
}

// GLSL 의 mat4 유니폼은 열 우선(column-major) 바이트 배치를 기대한다(OpenGL 표준
// 관례 — glUniformMatrix4fv 의 transpose=false 가 "이미 열 우선"이라는 뜻). 이
// 파일의 Mat4 는 읽고 쓰기 편하라고 행 우선(m[row][col])으로 짰으니, 유니폼으로
// 넘기기 직전에 반드시 이걸로 전치해야 한다 — 안 그러면 GLSL 쪽에서 우리 행렬의
// 전치를 읽게 되어(바이트를 그대로 열 우선으로 재해석하니) 투영이 뒤틀려서
// 화면이 이상하게 늘어나 보인다.
pub fn mat_transpose(m: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[c][r] = m[r][c];
        }
    }
    out
}

pub fn mat_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut out = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
        }
    }
    out
}

// 아핀 변환(이동/회전/스케일 조합)이라고 가정하고 점 하나를 옮긴다 — w=1, 원근
// 나눗셈 없음(그건 GPU 쪽 정점 셰이더가 뷰-프로젝션 행렬에서만 한다).
pub fn mat_transform_point(m: &Mat4, p: Vec3) -> Vec3 {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    ]
}

// 방향 벡터(이동 성분은 무시) 변환 — 회전/스케일만 적용한다.
pub fn mat_transform_dir(m: &Mat4, p: Vec3) -> Vec3 {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2],
    ]
}

pub fn mat_translate(t: Vec3) -> Mat4 {
    let mut m = mat_identity();
    m[0][3] = t[0];
    m[1][3] = t[1];
    m[2][3] = t[2];
    m
}

pub fn mat_scale(s: Vec3) -> Mat4 {
    let mut m = mat_identity();
    m[0][0] = s[0];
    m[1][1] = s[1];
    m[2][2] = s[2];
    m
}

// 오른손 좌표계, Y 가 위. yaw 는 Y축(수직) 회전, pitch 는 X축(좌우), roll 은 Z축(앞뒤).
pub fn mat_rotate_y(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = mat_identity();
    m[0][0] = c;
    m[0][2] = s;
    m[2][0] = -s;
    m[2][2] = c;
    m
}
pub fn mat_rotate_x(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = mat_identity();
    m[1][1] = c;
    m[1][2] = -s;
    m[2][1] = s;
    m[2][2] = c;
    m
}
pub fn mat_rotate_z(rad: f32) -> Mat4 {
    let (s, c) = rad.sin_cos();
    let mut m = mat_identity();
    m[0][0] = c;
    m[0][1] = -s;
    m[1][0] = s;
    m[1][1] = c;
    m
}

// 표준 원근 투영(오른손 뷰공간 → 클립공간, GL 스타일 -1..1 깊이).
pub fn mat_perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y / 2.0).tan();
    let mut m = [[0.0; 4]; 4];
    m[0][0] = f / aspect;
    m[1][1] = f;
    m[2][2] = (far + near) / (near - far);
    m[2][3] = (2.0 * far * near) / (near - far);
    m[3][2] = -1.0;
    m
}

// ================= 카메라 =================

#[derive(Clone, Copy)]
pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,   // 0 = +X 를 바라봄, 반시계
    pub pitch: f32, // + 면 위를 봄
}

impl Camera {
    // 뷰 행렬 = (회전*이동)의 역행렬. 회전만 있는 행렬의 역은 전치라, 여기서는
    // "반대 순서로 반대 회전/이동을 적용"하는 형태로 직접 구성한다. 이 카메라의
    // "월드 회전"은 Ry(yaw)*Rx(pitch)(로컬 -Z 가 "정면")라고 정의하고, 이 함수는
    // 정확히 그 역행렬이다 — forward()/right() 가 이 정의와 반드시 일치해야
    // 화면에 보이는 방향과 WASD 이동/마우스 오빗이 어긋나지 않는다.
    pub fn view_matrix(&self) -> Mat4 {
        let rot = mat_mul(&mat_rotate_x(-self.pitch), &mat_rotate_y(-self.yaw));
        mat_mul(&rot, &mat_translate(v_scale(self.pos, -1.0)))
    }

    // 카메라가 실제로 보고 있는 방향(로컬 -Z 를 Ry(yaw)*Rx(pitch)로 옮긴 것) —
    // view_matrix() 와 반드시 같은 정의를 써야 한다.
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        [-sy * cp, sp, -cy * cp]
    }
    pub fn right(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        [cy, 0.0, -sy]
    }
    // 마우스 피킹(화면 좌표 → 월드 레이)에 쓰는 세 번째 축 — forward/right 와
    // 반드시 같은 회전 정의(Ry(yaw)*Rx(pitch))로 로컬 +Y 를 옮긴 것이어야 한다.
    pub fn up(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        [sy * sp, cp, cy * sp]
    }

    // 걷기 이동에 쓰는 "바닥에 붙인" 전방/우측 — pitch 는 무시(위를 본다고 하늘로
    // 날아가진 않는다).
    pub fn forward_flat(&self) -> Vec3 {
        [-self.yaw.sin(), 0.0, -self.yaw.cos()]
    }
    pub fn right_flat(&self) -> Vec3 {
        [self.yaw.cos(), 0.0, -self.yaw.sin()]
    }
}

// ================= Box3D: 유일한 지오메트리 원시 타입 =================
//
// 벽이든 바닥이든 경사로든 전부 이 상자 하나로 만든다 — 회전이 없으면 흔한
// 축정렬 상자, yaw 만 있으면 "비스듬히 놓인 벽", pitch/roll 이 있으면 "기울어진
// 벽"이나 "경사로"가 된다(경사로는 걸어 올라갈 수 있는 상자의 윗면 — 물리 쪽도
// 이 상자의 로컬 윗면(+Y)을 그대로 걷는 표면으로 취급해서 별도 처리가 필요 없다).

#[derive(Clone, Copy, PartialEq)]
pub enum NetFace {
    Top,
    Bottom,
    PosX,
    NegX,
    PosZ,
    NegZ,
}

// 예전 raycaster.rs::PropTexture 와 같은 개념(가로 4칸×세로 3칸 십자 전개도) — 한
// 이미지 하나로 상자 6면을 전부 입힌다. Top=(1,0), Bottom=(1,2), 옆 4면은
// 가운데 줄(row 1)에 -X,-Z,+X,+Z 순서로 배치한다.
#[derive(Clone, Copy)]
pub struct BoxTexture {
    pub texture: TextureId,
}

impl BoxTexture {
    fn uv_for(&self, face: NetFace) -> (f32, f32, f32, f32) {
        let (col, row) = match face {
            NetFace::Top => (1, 0),
            NetFace::NegX => (0, 1),
            NetFace::NegZ => (1, 1),
            NetFace::PosX => (2, 1),
            NetFace::PosZ => (3, 1),
            NetFace::Bottom => (1, 2),
        };
        (col as f32 / 4.0, row as f32 / 3.0, (col + 1) as f32 / 4.0, (row + 1) as f32 / 3.0)
    }
}

#[derive(Clone)]
pub struct Box3D {
    pub center: Vec3,
    pub half: Vec3, // 반너비(가로/높이/세로) — 실제 전체 크기는 이 값의 2배
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub color: [f32; 4], // 텍스처가 없으면 이 색 그대로, 있으면 곱해지는 틴트(보통 흰색)
    pub texture: Option<BoxTexture>,
    // 이 상자가 "걸을 수 있는 바닥/경사로"인지 — false 면 ground_height() 탐색에서
    // 제외한다(예: 천장처럼 붙여둔 장식 상자가 바닥 판정에 끼어들지 않게).
    pub walkable: bool,
    // 이 상자가 수평 충돌(벽처럼 막힘)에 끼는지 — false 면 resolve_horizontal()에서
    // 제외한다(예: 밟고 지나가야 하는 얇은 경사로 상자는 막히면 안 된다).
    pub solid: bool,
}

impl Box3D {
    pub fn model_matrix(&self) -> Mat4 {
        let r = mat_mul(&mat_rotate_z(self.roll), &mat_mul(&mat_rotate_x(self.pitch), &mat_rotate_y(self.yaw)));
        mat_mul(&mat_translate(self.center), &r)
    }

    fn rotation_matrix(&self) -> Mat4 {
        mat_mul(&mat_rotate_z(self.roll), &mat_mul(&mat_rotate_x(self.pitch), &mat_rotate_y(self.yaw)))
    }

    // 로컬 X/Y/Z 축을 월드 방향으로 — 맵 에디터의 이동/회전/크기조절 기즈모가
    // 축 핸들을 어느 방향으로 그릴지 정할 때 쓴다.
    pub fn local_axes(&self) -> [Vec3; 3] {
        let r = self.rotation_matrix();
        [[r[0][0], r[1][0], r[2][0]], [r[0][1], r[1][1], r[2][1]], [r[0][2], r[1][2], r[2][2]]]
    }

    // 월드 방향 벡터 → 이 상자의 로컬 방향(회전만 풀고 이동은 안 건드림). 회전
    // 행렬은 정규직교라 역행렬 = 전치.
    fn world_dir_to_local(&self, d: Vec3) -> Vec3 {
        let r = self.rotation_matrix();
        [r[0][0] * d[0] + r[1][0] * d[1] + r[2][0] * d[2], r[0][1] * d[0] + r[1][1] * d[1] + r[2][1] * d[2], r[0][2] * d[0] + r[1][2] * d[1] + r[2][2] * d[2]]
    }

    // 월드 좌표 → 이 상자의 로컬 좌표(중심이 원점, 회전이 풀린 축정렬 공간).
    fn world_to_local(&self, p: Vec3) -> Vec3 {
        self.world_dir_to_local(v_sub(p, self.center))
    }

    fn local_to_world(&self, p: Vec3) -> Vec3 {
        mat_transform_point(&self.model_matrix(), p)
    }

    // 마우스 피킹용 레이-상자 교차(로컬 공간 AABB 슬래브 테스트) — 맞으면 카메라
    // 원점에서부터의 매개변수 t(>=0, 가까운 교차점) 를 돌려준다. 상자가 회전해
    // 있어도 `world_to_local`/`world_dir_to_local`로 레이 자체를 로컬 공간으로
    // 옮겨서 풀기 때문에 그대로 맞는다(경사로/기울어진 벽과 같은 원리).
    pub fn ray_intersect(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let lo = self.world_to_local(origin);
        let ld = self.world_dir_to_local(dir);
        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;
        for ((&o, &d), &h) in lo.iter().zip(ld.iter()).zip(self.half.iter()) {
            if d.abs() < 1e-8 {
                if o < -h || o > h {
                    return None;
                }
                continue;
            }
            let (t1, t2) = ((-h - o) / d, (h - o) / d);
            let (t1, t2) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
            t_min = t_min.max(t1);
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
        if t_max < 0.0 { None } else { Some(t_min.max(0.0)) }
    }

    // 월드 공간 삼각형(12개, 면마다 2개)으로 펼친다. 그리기용 — vertex(pos, uv, color).
    pub fn to_triangles(&self) -> Vec<(Vertex, Vertex, Vertex)> {
        let h = self.half;
        let tint = self.color;
        // 로컬 좌표계 상자 8 꼭짓점.
        let corners = |sx: f32, sy: f32, sz: f32| -> Vec3 { [sx * h[0], sy * h[1], sz * h[2]] };
        // (면, 로컬법선방향 4꼭짓점 CCW(바깥에서 봤을 때), 셰이딩 계수)
        let faces: [(NetFace, [Vec3; 4], f32); 6] = [
            (NetFace::Top, [corners(-1.0, 1.0, -1.0), corners(-1.0, 1.0, 1.0), corners(1.0, 1.0, 1.0), corners(1.0, 1.0, -1.0)], 1.0),
            (NetFace::Bottom, [corners(-1.0, -1.0, 1.0), corners(-1.0, -1.0, -1.0), corners(1.0, -1.0, -1.0), corners(1.0, -1.0, 1.0)], 0.45),
            (NetFace::PosX, [corners(1.0, -1.0, -1.0), corners(1.0, 1.0, -1.0), corners(1.0, 1.0, 1.0), corners(1.0, -1.0, 1.0)], 0.85),
            (NetFace::NegX, [corners(-1.0, -1.0, 1.0), corners(-1.0, 1.0, 1.0), corners(-1.0, 1.0, -1.0), corners(-1.0, -1.0, -1.0)], 0.7),
            (NetFace::PosZ, [corners(1.0, -1.0, 1.0), corners(1.0, 1.0, 1.0), corners(-1.0, 1.0, 1.0), corners(-1.0, -1.0, 1.0)], 0.78),
            (NetFace::NegZ, [corners(-1.0, -1.0, -1.0), corners(-1.0, 1.0, -1.0), corners(1.0, 1.0, -1.0), corners(1.0, -1.0, -1.0)], 0.62),
        ];
        let model = self.model_matrix();
        let mut out = Vec::with_capacity(12);
        for (face, quad, shade) in faces {
            let (u0, v0, u1, v1) = self.texture.map(|t| t.uv_for(face)).unwrap_or((0.0, 0.0, 1.0, 1.0));
            let uvs = [(u0, v1), (u0, v0), (u1, v0), (u1, v1)];
            let color = [tint[0] * shade, tint[1] * shade, tint[2] * shade, tint[3]];
            let world: Vec<Vec3> = quad.iter().map(|&p| mat_transform_point(&model, p)).collect();
            let vtx = |i: usize| Vertex { pos: world[i], uv: [uvs[i].0, uvs[i].1], color };
            out.push((vtx(0), vtx(1), vtx(2)));
            out.push((vtx(0), vtx(2), vtx(3)));
        }
        out
    }

    // 로컬 윗면(+Y) 두 삼각형을 월드 좌표로 — 바닥 높이 탐색(ground_height)에 쓴다.
    // 로컬 윗면은 회전이 없으면 그냥 수평 천장이지만, 상자 자체가 기울어 있으면
    // (pitch/roll) 그 기울기 그대로 따라가는 경사면이 된다 — 이게 "경사로"의 정체다.
    fn top_world_triangles(&self) -> [(Vec3, Vec3, Vec3); 2] {
        let h = self.half;
        let c = |sx: f32, sz: f32| self.local_to_world([sx * h[0], h[1], sz * h[2]]);
        let (a, b, cc, d) = (c(-1.0, -1.0), c(-1.0, 1.0), c(1.0, 1.0), c(1.0, -1.0));
        [(a, b, cc), (a, cc, d)]
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

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

// ================= GPU 렌더러 =================

pub struct Mesh3D {
    pass: RenderPass,
    color_tex: TextureId,
    depth_tex: TextureId,
    pipeline: Pipeline,
    white_tex: TextureId, // 텍스처 없는(단색) 상자용 1x1 흰 텍스처
    size: (f32, f32),
    // PS1/세가새턴류 레트로 셰이딩 — 기본은 꺼짐(색상 무제한, 디더 없음)과
    // 사실상 같은 값이라 호출부가 굳이 안 건드리면 기존 화면 그대로 나온다.
    // set_retro_shading() 으로 켠다.
    dither_amount: f32, // 0=디더링 없음 .. 1=베이어 4x4 패턴 최대 세기
    color_levels: f32,  // 채널당 색 단계 수 — 낮을수록(4~8) PS1 식 밴딩이 강해진다
}

#[repr(C)]
struct Mesh3DUniform {
    view_proj: Mat4,
    dither_amount: f32,
    color_levels: f32,
}

const MESH3D_VS: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec4 in_color;
uniform mat4 view_proj;
varying highp vec2 uv;
varying lowp vec4 color;
void main() {
    gl_Position = view_proj * vec4(in_pos, 1.0);
    uv = in_uv;
    color = in_color;
}
"#;

// 디더링(색상을 양자화하기 전에 화면 좌표 기반 베이어 4x4 패턴으로 살짝
// 흔들어서, 낮은 색상 단계에서도 매끈한 그라데이션처럼 보이게 하는 기법 —
// PS1/세가새턴 시절 하드웨어가 색을 적게 표현할 수 있었던 걸 흉내낸다) +
// 색상 단계 제한(channel 당 color_levels 단계로 반올림)을 프래그먼트
// 셰이더에서 한다. 동적 배열 인덱싱은 일부 GLSL ES 100 구현에서 까다로워서
// (특히 프래그먼트 좌표에서 나온 값처럼 컴파일 타임에 알 수 없는 인덱스),
// 베이어 행렬을 배열 대신 중첩 if 사슬로 직접 풀어썼다 — 이식성이 더 좋다.
const MESH3D_FS: &str = r#"#version 100
precision highp float;
varying highp vec2 uv;
varying lowp vec4 color;
uniform sampler2D tex;
uniform float dither_amount;
uniform float color_levels;

float bayer4x4(vec2 fragCoord) {
    float x = mod(floor(fragCoord.x), 4.0);
    float y = mod(floor(fragCoord.y), 4.0);
    if (y < 1.0) {
        if (x < 1.0) return 0.0;
        if (x < 2.0) return 8.0;
        if (x < 3.0) return 2.0;
        return 10.0;
    } else if (y < 2.0) {
        if (x < 1.0) return 12.0;
        if (x < 2.0) return 4.0;
        if (x < 3.0) return 14.0;
        return 6.0;
    } else if (y < 3.0) {
        if (x < 1.0) return 3.0;
        if (x < 2.0) return 11.0;
        if (x < 3.0) return 1.0;
        return 9.0;
    } else {
        if (x < 1.0) return 15.0;
        if (x < 2.0) return 7.0;
        if (x < 3.0) return 13.0;
        return 5.0;
    }
}

void main() {
    vec4 texel = texture2D(tex, uv) * color;
    float levels = max(color_levels, 1.0);
    // 베이어 값(0..15)을 -0.5..0.5 로 정규화하고 한 단계 폭(1/levels)만큼만
    // 흔든다 — 그래야 흔드는 양이 지금 색상 단계 폭을 넘지 않아 엉뚱한 색으로
    // 안 튄다.
    float d = (bayer4x4(gl_FragCoord.xy) / 15.0 - 0.5) * dither_amount / levels;
    vec3 dithered = texel.rgb + d;
    vec3 quantized = floor(dithered * levels + 0.5) / levels;
    gl_FragColor = vec4(clamp(quantized, 0.0, 1.0), texel.a);
}
"#;

impl Mesh3D {
    pub fn new(ctx: &mut dyn RenderingBackend, vw: u32, vh: u32) -> Mesh3D {
        let color_tex = ctx.new_render_texture(TextureParams {
            width: vw,
            height: vh,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        let depth_tex = ctx.new_render_texture(TextureParams { width: vw, height: vh, format: TextureFormat::Depth, ..Default::default() });
        let pass = ctx.new_render_pass(color_tex, Some(depth_tex));

        let white_tex = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: MESH3D_VS, fragment: MESH3D_FS },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("view_proj", UniformType::Mat4),
                            UniformDesc::new("dither_amount", UniformType::Float1),
                            UniformDesc::new("color_levels", UniformType::Float1),
                        ],
                    },
                },
            )
            .expect("mesh3d 셰이더 컴파일 실패");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float3),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
                VertexAttribute::new("in_color", VertexFormat::Float4),
            ],
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                cull_face: CullFace::Back,
                ..Default::default()
            },
        );

        Mesh3D { pass, color_tex, depth_tex, pipeline, white_tex, size: (vw as f32, vh as f32), dither_amount: 0.0, color_levels: 256.0 }
    }

    // 레트로(PS1/세가새턴풍) 셰이딩 세기를 켠다 — dither_amount 는 0(없음)~1(최대
    // 베이어 4x4 세기), color_levels 는 채널당 색 단계 수(낮을수록 밴딩이 강해진다
    // — 4~8 정도가 그럴싸하다, 256 이면 사실상 무제한이라 원래 색 그대로 나온다).
    // 기본값(생성 직후)은 둘 다 꺼진 상태와 같아서, 이 메서드를 안 부르면 기존
    // 화면과 똑같이 나온다.
    pub fn set_retro_shading(&mut self, dither_amount: f32, color_levels: f32) {
        self.dither_amount = dither_amount.clamp(0.0, 1.0);
        self.color_levels = color_levels.max(1.0);
    }

    pub fn retro_shading(&self) -> (f32, f32) {
        (self.dither_amount, self.color_levels)
    }

    pub fn set_resolution(&mut self, ctx: &mut dyn RenderingBackend, w: u32, h: u32) {
        ctx.delete_render_pass(self.pass);
        ctx.delete_texture(self.color_tex);
        ctx.delete_texture(self.depth_tex);
        self.color_tex = ctx.new_render_texture(TextureParams {
            width: w,
            height: h,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Nearest,
            ..Default::default()
        });
        self.depth_tex = ctx.new_render_texture(TextureParams { width: w, height: h, format: TextureFormat::Depth, ..Default::default() });
        self.pass = ctx.new_render_pass(self.color_tex, Some(self.depth_tex));
        self.size = (w as f32, h as f32);
    }

    pub fn color_texture(&self) -> TextureId {
        self.color_tex
    }

    pub fn aspect(&self) -> f32 {
        self.size.0 / self.size.1
    }

    // camera/boxes 로 장면을 오프스크린 타깃에 그린다. 텍스처별로 묶어 드로우콜을
    // 나눈다(텍스처 없는 상자는 1x1 흰 텍스처 + 정점색으로 처리해서 파이프라인이
    // 하나로 충분하다). 상자 수가 적은(수십~수백) 스타일화된 장면이 목표라 매 프레임
      // 버텍스 버퍼를 새로 만들어도 무리 없다.
    pub fn render(&mut self, ctx: &mut dyn RenderingBackend, clear: [f32; 4], camera: &Camera, boxes: &[Box3D], fov_y: f32) {
        let proj = mat_perspective(fov_y, self.aspect(), 0.05, 200.0);
        let view = camera.view_matrix();
        let view_proj = mat_mul(&proj, &view);

        // 텍스처(TextureId 없으면 흰 텍스처)별로 정점을 모은다.
        let mut groups: std::collections::HashMap<TextureId, Vec<Vertex>> = std::collections::HashMap::new();
        for b in boxes {
            let tex = b.texture.map(|t| t.texture).unwrap_or(self.white_tex);
            let entry = groups.entry(tex).or_default();
            for (a, bb, c) in b.to_triangles() {
                entry.push(a);
                entry.push(bb);
                entry.push(c);
            }
        }

        ctx.begin_pass(Some(self.pass), PassAction::clear_color(clear[0], clear[1], clear[2], clear[3]));
        ctx.apply_viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_scissor_rect(0, 0, self.size.0 as i32, self.size.1 as i32);
        ctx.apply_pipeline(&self.pipeline);

        let mut owned_buffers = Vec::new();
        for (tex, verts) in &groups {
            if verts.is_empty() {
                continue;
            }
            let vbuf = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::slice(verts.as_slice()));
            let indices: Vec<u16> = (0..verts.len() as u16).collect();
            let ibuf = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Stream, BufferSource::slice(indices.as_slice()));
            let bindings = Bindings { vertex_buffers: vec![vbuf], index_buffer: ibuf, images: vec![*tex] };
            ctx.apply_bindings(&bindings);
            let u = Mesh3DUniform { view_proj: mat_transpose(&view_proj), dither_amount: self.dither_amount, color_levels: self.color_levels };
            ctx.apply_uniforms(UniformsSource::table(&u));
            ctx.draw(0, verts.len() as i32, 1);
            owned_buffers.push(vbuf);
            owned_buffers.push(ibuf);
        }
        ctx.end_render_pass();
        for buf in owned_buffers {
            ctx.delete_buffer(buf);
        }
    }
}
