//! 3D 게임 장면 안에 놓인, 조준해서 E 로 조사할 수 있는 물건 하나 — Box3D 로 그려지고 충돌이 있다.
//! T 는 그 게임의 "조사 대상" 종류(enum).

use crate::render::mesh3d::Box3D;

pub struct Item<T> {
    pub target: T,
    pub center: [f32; 3],
    pub half: [f32; 3],
    pub color: [f32; 4],
    pub label: &'static str,
    pub yaw: f32,
    pub roll: f32,
    pub draw: bool,  // false 면 조준 대상일 뿐 상자로는 안 그린다(스프라이트/장식이 대신 보여줌)
    pub solid: bool, // false 면 지나갈 수 있다
}

impl<T> Item<T> {
    pub fn new(target: T, center: [f32; 3], half: [f32; 3], color: [f32; 4], label: &'static str) -> Item<T> {
        Item { target, center, half, color, label, yaw: 0.0, roll: 0.0, draw: true, solid: true }
    }

    // 조준 대상일 뿐 그리지도 막지도 않는 물건.
    pub fn marker(target: T, center: [f32; 3], half: [f32; 3], label: &'static str) -> Item<T> {
        Item { draw: false, solid: false, ..Item::new(target, center, half, [1.0; 4], label) }
    }

    pub fn to_box(&self) -> Box3D {
        Box3D { center: self.center, half: self.half, yaw: self.yaw, pitch: 0.0, roll: self.roll, color: self.color, texture: None, walkable: false, solid: self.solid }
    }
}
