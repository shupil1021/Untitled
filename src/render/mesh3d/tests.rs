//! mesh3d 단위 테스트(OBJ 파서, 충돌, 캐시 키, 드로우콜 한도).
use super::gpu::{mesh_key, MAX_DRAW_VERTS};
use super::*;

fn solid_box(center: Vec3, half: Vec3) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color: [1.0; 4], texture: None, walkable: true, solid: true }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn obj_quad_is_split_into_two_triangles() {
    let m = parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
    assert_eq!(m.triangles, vec![[0, 1, 2], [0, 2, 3]]);
    assert_eq!(m.vertices.len(), 4);
}

#[test]
fn obj_negative_indices_count_from_the_end() {
    let m = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
    assert_eq!(m.triangles, vec![[0, 1, 2]]);
    assert_eq!(m.vertices[2].pos, [0.0, 1.0, 0.0]);
}

#[test]
fn obj_uv_is_flipped_and_split_per_seam() {
    // 같은 위치라도 UV 가 다르면 정점이 둘로 갈라진다. v 는 위가 0 이 되게 뒤집힌다.
    let m = parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvt 1 0.25\nf 1/1 2/1 3/1\nf 1/2 2/2 3/2\n").unwrap();
    assert_eq!(m.vertices.len(), 6);
    assert!(close(m.vertices[0].uv[1], 1.0));
    assert!(close(m.vertices[3].uv[1], 0.75));
}

#[test]
fn obj_errors_are_reported_not_panics() {
    assert!(parse_obj("").is_err());
    assert!(parse_obj("v 0 0 0\nv 1 0 0\nf 1 2\n").is_err()); // 면이 정점 2개
    assert!(parse_obj("v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 9\n").is_err()); // 범위 밖 인덱스
    assert!(parse_obj("v 0 x 0\n").is_err()); // 숫자가 아님
}

#[test]
fn mtl_colors_follow_usemtl_and_convert_to_srgb() {
    let mtl = "newmtl Red\nKd 1.0 0.0 0.0\nnewmtl Dark\nKd 0.0 0.0 0.0\n";
    let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl Red\nf 1 2 3\nusemtl Dark\nf 1 2 3\nusemtl Missing\nf 1 2 3\n";
    let m = parse_obj_mtl(obj, mtl).unwrap();
    assert_eq!(m.tri_colors.len(), 3);
    assert!(m.tri_colors[0].iter().zip([1.0, 0.0, 0.0, 1.0]).all(|(a, b)| close(*a, b)));
    assert!(m.tri_colors[1].iter().zip([0.0, 0.0, 0.0, 1.0]).all(|(a, b)| close(*a, b)));
    assert_eq!(m.tri_colors[2], [1.0; 4]); // mtl 에 없는 이름은 흰색
    // 선형 0.5 는 sRGB 로 약 0.735 가 된다.
    let half = parse_obj_mtl("v 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl M\nf 1 2 3\n", "newmtl M\nKd 0.5 0.5 0.5\n").unwrap();
    assert!((half.tri_colors[0][0] - 0.735).abs() < 0.01);
}

#[test]
fn mesh_object_applies_scale_then_translate() {
    let mesh = std::rc::Rc::new(parse_obj("v 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 2 3\n").unwrap());
    let obj = MeshObject { center: [10.0, 0.0, 0.0], yaw: 0.0, pitch: 0.0, roll: 0.0, scale: [2.0, 2.0, 2.0], color: [1.0; 4], texture: None, walkable: true, solid: true, mesh };
    let (a, b, c) = obj.world_triangles()[0];
    assert!(close(a[0], 12.0) && close(b[1], 2.0) && close(c[2], 2.0));
}

#[test]
fn ray_hits_the_near_face_of_a_box() {
    let b = solid_box([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
    let t = b.ray_intersect([0.0, 0.0, 5.0], [0.0, 0.0, -1.0]).expect("맞아야 한다");
    assert!(close(t, 4.0));
    assert!(b.ray_intersect([3.0, 0.0, 5.0], [0.0, 0.0, -1.0]).is_none()); // 옆으로 빗나감
    assert!(b.ray_intersect([0.0, 0.0, 5.0], [0.0, 0.0, 1.0]).is_none()); // 반대 방향
}

#[test]
fn ground_height_finds_the_top_and_skips_unwalkable_boxes() {
    let floor = solid_box([0.0, -0.5, 0.0], [5.0, 0.5, 5.0]); // 윗면 y = 0
    let mut decoration = solid_box([0.0, 2.0, 0.0], [5.0, 0.1, 5.0]);
    decoration.walkable = false;
    let boxes = [floor, decoration];
    let h = ground_height(1.0, 1.0, 1.0, 1.0, &boxes).expect("발 디딜 곳이 있어야 한다");
    assert!(close(h, 0.0));
    assert!(ground_height(50.0, 50.0, 1.0, 1.0, &boxes).is_none()); // 바닥 밖은 낭떠러지
}

#[test]
fn walls_push_the_player_out_but_only_solid_ones() {
    let wall = solid_box([0.0, 1.0, 0.0], [1.0, 1.0, 1.0]);
    let p = resolve_horizontal([1.2, 0.0, 0.0], 0.5, 0.0, 1.7, std::slice::from_ref(&wall));
    assert!(p[0] >= 1.5 - 1e-3, "벽 밖으로 밀려나야 한다: {p:?}");
    let mut ghost = wall.clone();
    ghost.solid = false;
    let q = resolve_horizontal([1.2, 0.0, 0.0], 0.5, 0.0, 1.7, std::slice::from_ref(&ghost));
    assert_eq!(q, [1.2, 0.0, 0.0]);
}

#[test]
fn standing_on_top_of_a_box_is_not_blocked_by_it() {
    let step = solid_box([0.0, 0.25, 0.0], [1.0, 0.25, 1.0]); // 윗면 y = 0.5
    let p = resolve_horizontal([0.2, 0.5, 0.2], 0.4, 0.5, 1.7, std::slice::from_ref(&step));
    assert_eq!(p, [0.2, 0.5, 0.2]);
}

#[test]
fn mesh_cache_key_changes_only_when_the_look_changes() {
    let mesh = std::rc::Rc::new(parse_obj("v 1 0 0
v 0 1 0
v 0 0 1
f 1 2 3
").unwrap());
    let mk = |x: f32, color: [f32; 4]| MeshObject { center: [x, 0.0, 0.0], yaw: 0.0, pitch: 0.0, roll: 0.0, scale: [1.0; 3], color, texture: None, walkable: true, solid: true, mesh: std::rc::Rc::clone(&mesh) };
    assert!(mesh_key(&mk(1.0, [1.0; 4])) == mesh_key(&mk(1.0, [1.0; 4])));
    assert!(mesh_key(&mk(1.0, [1.0; 4])) != mesh_key(&mk(2.0, [1.0; 4])));
    assert!(mesh_key(&mk(1.0, [1.0; 4])) != mesh_key(&mk(1.0, [0.5, 1.0, 1.0, 1.0])));
    // 같은 모양이어도 다른 메시 데이터면 다른 키다.
    let other = std::rc::Rc::new(parse_obj("v 1 0 0
v 0 1 0
v 0 0 1
f 1 2 3
").unwrap());
    let mut o = mk(1.0, [1.0; 4]);
    o.mesh = other;
    assert!(mesh_key(&mk(1.0, [1.0; 4])) != mesh_key(&o));
}

#[test]
fn a_draw_call_chunk_never_overflows_u16_indices() {
    assert_eq!(MAX_DRAW_VERTS % 3, 0);
    assert!(MAX_DRAW_VERTS <= u16::MAX as usize);
    // 정점이 한도를 넘는 묶음도 조각으로 나뉘어 하나도 버려지지 않는다.
    let total = MAX_DRAW_VERTS * 2 + 30;
    let verts = vec![0u8; total];
    assert_eq!(verts.chunks(MAX_DRAW_VERTS).map(|c| c.len()).sum::<usize>(), total);
    assert!(verts.chunks(MAX_DRAW_VERTS).all(|c| c.len() <= MAX_DRAW_VERTS && c.len() % 3 == 0 || c.len() == 30));
}
