//! 장면에 박스를 만드는 작은 도구들 — 방/정글/미로/코스가 각자 같은 모양의 함수를 따로 들고 있던 걸 모았다.

use crate::render::mesh3d::Box3D;

// 막히는 박스(충돌 있음). walkable 이 true 면 위를 걸을 수 있는 바닥으로도 쓴다.
pub fn solid(center: [f32; 3], half: [f32; 3], color: [f32; 4], walkable: bool) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color, texture: None, walkable, solid: true }
}

// 막히기만 하는 박스(바닥 아님).
pub fn block(center: [f32; 3], half: [f32; 3], color: [f32; 4]) -> Box3D {
    solid(center, half, color, false)
}

// 지나갈 수 있는 장식 박스(그리기만 한다).
pub fn deco(center: [f32; 3], half: [f32; 3], color: [f32; 4]) -> Box3D {
    Box3D { solid: false, ..solid(center, half, color, false) }
}

// yaw 만큼 돌린 장식 박스 — 레이싱 카트/길 조각처럼 방향이 있는 것들.
pub fn rotated(center: [f32; 3], half: [f32; 3], yaw: f32, color: [f32; 4]) -> Box3D {
    Box3D { yaw, ..deco(center, half, color) }
}
