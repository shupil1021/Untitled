//! Wavefront OBJ/MTL 파서와 메시 데이터.

use super::*;

// ================= MeshObject: 블렌더 등에서 만든 임의 메시(OBJ) =================
//
// Box3D 는 상자 하나뿐이라 블렌더에서 만든 임의의 폴리곤 형태(곡면/복잡한
// 소품 등)는 표현할 수 없다 — 그런 걸 그대로 쓰고 싶으면 실제 삼각형 메시가
// 필요하다. glam 을 새로 끌어온 게 아니듯, gltf/obj 파싱 크레이트도 새로
// 끌어오는 대신 이 파일 안에서 OBJ(Wavefront .obj, 블렌더 기본 내보내기
// 포맷 중 하나 — 텍스트 기반이라 파서가 단순하다)만 최소한으로 직접 읽는다.
// 블렌더 OBJ 내보내기 기본값(Forward -Z, Up Y)이 이 엔진의 좌표계(Y 가 위)와
// 그대로 맞아서 축 변환이 따로 필요 없다.
//
// `vn`(법선)은 안 읽는다 — 이 엔진은 애초에 Box3D 도 "면마다 고정된 셰이딩
// 계수"로 흉내만 낸 플랫 셰이딩이라, 메시도 삼각형마다 그 면의 실제 법선을
// 계산해 고정 광원 방향과 내적한 값으로 같은 스타일의 플랫 셰이딩을 준다
// (아래 mesh_face_shade).

pub struct MeshVertex {
    pub pos: Vec3,
    pub uv: [f32; 2],
}

// 파싱된 메시 데이터 — 여러 MeshObject 인스턴스가 `Rc`로 하나를 공유할 수
// 있다(같은 소품을 여러 개 배치해도 정점 데이터를 중복으로 안 들고 있게).
pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub triangles: Vec<[u32; 3]>,
    // 삼각형마다의 재질 색(OBJ 의 usemtl + MTL 의 Kd). 비어 있으면 재질 없음 — MeshObject.color 한 색만 쓴다.
    pub tri_colors: Vec<[f32; 4]>,
}

impl MeshData {
    pub fn load(path: &str) -> std::io::Result<MeshData> {
        let text = std::fs::read_to_string(path)?;
        parse_obj(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

// MTL 텍스트에서 재질 이름 → 색(Kd)을 읽는다. Blender 의 Kd 는 선형 값이라 sRGB 로 바꿔서 쓴다
// (이 엔진의 색은 화면에 그대로 나가는 값이다).
fn parse_mtl_colors(mtl: &str) -> std::collections::HashMap<String, [f32; 4]> {
    let to_srgb = |c: f32| if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    let mut out = std::collections::HashMap::new();
    let mut cur: Option<String> = None;
    for line in mtl.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("newmtl") => cur = it.next().map(str::to_string),
            Some("Kd") => {
                let v: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                if let (Some(name), [r, g, b, ..]) = (cur.clone(), v.as_slice()) {
                    out.insert(name, [to_srgb(*r), to_srgb(*g), to_srgb(*b), 1.0]);
                }
            }
            _ => {}
        }
    }
    out
}

// 아주 단순한 OBJ 파서 — `v`(위치)/`vt`(UV)/`f`(면, v/vt/vn 또는 v/vt 또는 v 단독
// 다 받는다)만 읽고 나머지 지시어(vn/mtllib/usemtl/o/g/s/# 주석 등)는 무시한다.
// 삼각형이 아닌 면(사각형 이상)은 팬(fan) 방식으로 삼각형화한다. OBJ 는
// (위치 인덱스, UV 인덱스) 조합별로 정점이 갈릴 수 있어서(같은 위치라도 UV
// 이음매에서 다른 UV를 써야 할 수 있다), 그 조합을 키로 중복 없이 새 정점을
// 만들어 인덱스 메시로 합친다.
pub fn parse_obj(text: &str) -> Result<MeshData, String> {
    parse_obj_mtl(text, "")
}

// parse_obj + 재질 색 — `usemtl 이름` 이후의 면은 mtl 텍스트에서 그 이름의 Kd 색을 삼각형마다 갖는다
// (mtl 에 없는 이름은 흰색). mtl 이 비어 있으면 재질 색 없이 읽는다.
pub fn parse_obj_mtl(text: &str, mtl: &str) -> Result<MeshData, String> {
    let colors = parse_mtl_colors(mtl);
    let mut cur_color = [1.0f32; 4];
    let mut tri_colors: Vec<[f32; 4]> = Vec::new();
    let mut positions: Vec<Vec3> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut vertices: Vec<MeshVertex> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    // (위치 인덱스, uv 인덱스) → 이미 만들어둔 출력 정점 인덱스.
    let mut cache: std::collections::HashMap<(i64, i64), u32> = std::collections::HashMap::new();

    let parse_f32 = |s: &str| -> Result<f32, String> { s.parse::<f32>().map_err(|_| format!("숫자가 아님: {s}")) };

    // "v_idx/vt_idx/vn_idx" 형태(뒤 두 개는 선택) 하나를 파싱해 (v,vt) 1-based
    // 인덱스만 뽑는다 — OBJ 는 음수 인덱스(파일 끝 기준 상대)도 허용해서 같이 처리한다.
    let resolve_index = |raw: i64, len: usize| -> i64 { if raw < 0 { len as i64 + raw + 1 } else { raw } };

    let get_or_add_vertex = |pos_idx: i64, uv_idx: i64, positions: &[Vec3], uvs: &[[f32; 2]], vertices: &mut Vec<MeshVertex>, cache: &mut std::collections::HashMap<(i64, i64), u32>| -> Result<u32, String> {
        let key = (pos_idx, uv_idx);
        if let Some(&idx) = cache.get(&key) {
            return Ok(idx);
        }
        let pos = *positions.get((pos_idx - 1) as usize).ok_or_else(|| format!("정점 인덱스 범위 밖: {pos_idx}"))?;
        let uv = if uv_idx > 0 { *uvs.get((uv_idx - 1) as usize).ok_or_else(|| format!("UV 인덱스 범위 밖: {uv_idx}"))? } else { [0.0, 0.0] };
        let idx = vertices.len() as u32;
        vertices.push(MeshVertex { pos, uv });
        cache.insert(key, idx);
        Ok(idx)
    };

    for (line_no, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut it = line.split_whitespace();
        let Some(tag) = it.next() else { continue };
        match tag {
            "v" => {
                let parts: Vec<&str> = it.collect();
                if parts.len() < 3 {
                    return Err(format!("{}번째 줄: v 는 x y z 세 값이 필요함", line_no + 1));
                }
                positions.push([parse_f32(parts[0])?, parse_f32(parts[1])?, parse_f32(parts[2])?]);
            }
            "vt" => {
                let parts: Vec<&str> = it.collect();
                if parts.len() < 2 {
                    return Err(format!("{}번째 줄: vt 는 u v 두 값이 필요함", line_no + 1));
                }
                // OBJ 의 UV 는 아래가 0 — 이 엔진의 텍스처 좌표는 miniquad 관례상
                // 위가 0 이라(다른 곳(Mesh3D 오프스크린 합성 등)도 다 v 를 뒤집어
                // 쓴다) 여기서 한 번 뒤집어두면 나중에 또 신경 안 써도 된다.
                uvs.push([parse_f32(parts[0])?, 1.0 - parse_f32(parts[1])?]);
            }
            "f" => {
                let mut face_verts: Vec<u32> = Vec::new();
                for token in it {
                    let mut comps = token.split('/');
                    let v_raw: i64 = comps.next().unwrap_or("").parse().map_err(|_| format!("{}번째 줄: 정점 인덱스 파싱 실패({token})", line_no + 1))?;
                    let vt_raw: i64 = match comps.next() {
                        Some(s) if !s.is_empty() => s.parse().map_err(|_| format!("{}번째 줄: UV 인덱스 파싱 실패({token})", line_no + 1))?,
                        _ => 0,
                    };
                    let pos_idx = resolve_index(v_raw, positions.len());
                    let uv_idx = if vt_raw != 0 { resolve_index(vt_raw, uvs.len()) } else { 0 };
                    face_verts.push(get_or_add_vertex(pos_idx, uv_idx, &positions, &uvs, &mut vertices, &mut cache)?);
                }
                if face_verts.len() < 3 {
                    return Err(format!("{}번째 줄: 면은 정점 3개 이상이어야 함", line_no + 1));
                }
                // 팬 삼각형화 — v0 을 축으로 (v0,v1,v2),(v0,v2,v3)... (볼록 다각형
                // 기준. 블렌더 기본 내보내기는 삼각형/사각형이 대부분이라 충분하다).
                for i in 1..face_verts.len() - 1 {
                    triangles.push([face_verts[0], face_verts[i], face_verts[i + 1]]);
                    if !colors.is_empty() {
                        tri_colors.push(cur_color);
                    }
                }
            }
            "usemtl" => cur_color = it.next().and_then(|n| colors.get(n)).copied().unwrap_or([1.0; 4]),
            _ => {} // vn/mtllib/o/g/s 등은 무시
        }
    }

    if vertices.is_empty() || triangles.is_empty() {
        return Err("정점/면이 하나도 없음(빈 OBJ?)".to_string());
    }
    Ok(MeshData { vertices, triangles, tri_colors })
}

// 고정 광원 방향(위+약간 앞) — Box3D 각 면의 고정 셰이딩 계수와 같은 취지로,
// 삼각형 실제 법선(뒤섞인 임의 메시라 면마다 다르다)에 내적해 플랫 셰이딩
// 계수를 만든다. Box3D 면들의 셰이딩 범위(0.45~1.0)와 맞춘다.
const MESH_LIGHT_DIR: Vec3 = [0.35, 0.82, 0.45];

pub(super) fn mesh_face_shade(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    let normal = v_norm(v_cross(v_sub(b, a), v_sub(c, a)));
    let light = v_norm(MESH_LIGHT_DIR);
    0.45 + 0.55 * v_dot(normal, light).max(0.0)
}
