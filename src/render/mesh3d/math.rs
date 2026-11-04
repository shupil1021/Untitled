//! 최소 벡터/행렬 수학과 카메라.

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
