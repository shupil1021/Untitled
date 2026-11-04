//! 그릴 수 있는 것들 — 상자(Box3D), OBJ 메시를 놓은 것(MeshObject), 정점(Vertex).

use miniquad::TextureId;

use super::*;

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

// 가로 4칸×세로 3칸 십자 전개도 — 한 이미지 하나로 상자 6면을 전부 입힌다. Top=(1,0), Bottom=(1,2), 옆 4면은
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

    pub(super) fn rotation_matrix(&self) -> Mat4 {
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
    pub(super) fn world_dir_to_local(&self, d: Vec3) -> Vec3 {
        let r = self.rotation_matrix();
        [r[0][0] * d[0] + r[1][0] * d[1] + r[2][0] * d[2], r[0][1] * d[0] + r[1][1] * d[1] + r[2][1] * d[2], r[0][2] * d[0] + r[1][2] * d[1] + r[2][2] * d[2]]
    }

    // 월드 좌표 → 이 상자의 로컬 좌표(중심이 원점, 회전이 풀린 축정렬 공간).
    pub(super) fn world_to_local(&self, p: Vec3) -> Vec3 {
        self.world_dir_to_local(v_sub(p, self.center))
    }

    pub(super) fn local_to_world(&self, p: Vec3) -> Vec3 {
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
    pub(super) fn top_world_triangles(&self) -> [(Vec3, Vec3, Vec3); 2] {
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

// 블렌더 등에서 만들어 OBJ 로 내보낸 소품 하나를 이 엔진 씬에 배치한 것 —
// Box3D 와 같은 자리(center/yaw/pitch/roll)에 균등하지 않은 스케일까지
// 더했고, 텍스처는 Box3D 의 4x3 전개도 대신 OBJ 가 가진 실제 UV 를 그대로
// 쓴다(단일 텍스처).
pub struct MeshObject {
    pub center: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub scale: Vec3,
    pub color: [f32; 4], // 텍스처와 곱해지는 틴트(텍스처 없으면 이 색 그대로)
    pub texture: Option<TextureId>,
    // Box3D 와 같은 뜻 — walkable=false 면 mesh_ground_height() 탐색에서,
    // solid=false 면 mesh_resolve_horizontal() 에서 제외한다.
    pub walkable: bool,
    pub solid: bool,
    pub mesh: std::rc::Rc<MeshData>,
}

impl MeshObject {
    pub fn model_matrix(&self) -> Mat4 {
        let r = mat_mul(&mat_rotate_z(self.roll), &mat_mul(&mat_rotate_x(self.pitch), &mat_rotate_y(self.yaw)));
        mat_mul(&mat_translate(self.center), &mat_mul(&r, &mat_scale(self.scale)))
    }

    // 이 메시의 모든 삼각형을 월드 좌표로 — 충돌 판정과 렌더링 둘 다 이걸 쓴다.
    // 소품 규모(수백~수천 삼각형)를 가정하고 매 프레임 새로 계산한다(Box3D 의
    // to_triangles() 와 같은 전제).
    pub fn world_triangles(&self) -> Vec<(Vec3, Vec3, Vec3)> {
        let model = self.model_matrix();
        self.mesh
            .triangles
            .iter()
            .map(|tri| {
                let v = |i: usize| mat_transform_point(&model, self.mesh.vertices[tri[i] as usize].pos);
                (v(0), v(1), v(2))
            })
            .collect()
    }

    // 그리기용 — 삼각형마다 실제 면 법선으로 플랫 셰이딩 계수를 계산해 tint 에 곱한다.
    pub fn to_triangles(&self) -> Vec<(Vertex, Vertex, Vertex)> {
        let model = self.model_matrix();
        let tint = self.color;
        self.mesh
            .triangles
            .iter()
            .enumerate()
            .map(|(n, tri)| {
                let mv = |i: usize| &self.mesh.vertices[tri[i] as usize];
                let (v0, v1, v2) = (mv(0), mv(1), mv(2));
                let (p0, p1, p2) = (mat_transform_point(&model, v0.pos), mat_transform_point(&model, v1.pos), mat_transform_point(&model, v2.pos));
                let shade = mesh_face_shade(p0, p1, p2);
                let base = self.mesh.tri_colors.get(n).map_or(tint, |c| [tint[0] * c[0], tint[1] * c[1], tint[2] * c[2], tint[3] * c[3]]);
                let color = [base[0] * shade, base[1] * shade, base[2] * shade, base[3]];
                (Vertex { pos: p0, uv: v0.uv, color }, Vertex { pos: p1, uv: v1.uv, color }, Vertex { pos: p2, uv: v2.uv, color })
            })
            .collect()
    }
}
