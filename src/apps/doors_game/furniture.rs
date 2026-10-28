//! 시작 방의 방/문/가구 — Blender 로 만든 OBJ 모델(assets/models, kim1e/3Doobject)을 불러와 책상/책장/우편함 자리에
//! 메시로 놓는다. 모델은 실행 파일에 같이 넣어서(include_str!) 파일 없이도 뜬다. 충돌은 모양 그대로가 아니라
//! 눈에 안 보이는 상자들(world::invisible_colliders)로 한다.

use std::cell::RefCell;
use std::rc::Rc;

use crate::render::mesh3d::{parse_obj_mtl, MeshData, MeshObject};

use super::world::{DESK_ORIGIN, DRESSER_ORIGIN, MAILBOX_ORIGIN};

// 모델 크기 — 책상 높이 2.96(Blender 단위)을 0.8 로 맞춘 배율(편지가 책상 위 0.8 에 놓인다).
const FURNITURE_SCALE: f32 = 0.27;
// 우편함은 사람 가슴 높이쯤(1.2)이 되게 조금 키운다.
const MAILBOX_SCALE: f32 = 0.32;
// 책장은 키 1.85 쯤으로.
const BOOKCASE_SCALE: f32 = 0.237;
// 방/문 모델 배율 — 방 안쪽 5.7 x 5.7, 문 1.57 x 2.33 이 되는 값(Blender 에서 방 24 x 24, 문 6.6 x 9.8 로 만들었다).
const ROOM_SCALE: f32 = 0.2375;
// 방 모델의 안쪽 가운데가 게임 방의 가운데(z -0.5)에 오고, 문이 있는 벽이 앞(-Z)을 보게 돌린다.
const ROOM_ORIGIN: [f32; 3] = [0.0, 0.0, -0.5];
// 문 모델이 서는 자리(문간 벽 가운데) — 문고리는 방 안쪽(+Z)을 본다. 모델 바닥이 바닥에서 살짝(0.1) 떠 있다.
const DOOR_ORIGIN: [f32; 3] = [0.0, 0.1 * ROOM_SCALE, -3.43];
const WHITE: [f32; 4] = [1.0; 4];

thread_local! {
    // 파싱은 한 번만 — 장면이 다시 만들어질 때마다 OBJ 를 또 읽지 않는다.
    static MODELS: RefCell<Option<[Rc<MeshData>; 5]>> = const { RefCell::new(None) };
}

fn load(obj: &str, mtl: &str) -> Rc<MeshData> {
    Rc::new(parse_obj_mtl(obj, mtl).expect("내장 모델 OBJ 가 잘못됨"))
}

fn models() -> [Rc<MeshData>; 5] {
    MODELS.with(|m| {
        m.borrow_mut()
            .get_or_insert_with(|| {
                [
                    load(include_str!("../../../assets/models/desk.obj"), include_str!("../../../assets/models/desk.mtl")),
                    load(include_str!("../../../assets/models/dresser.obj"), include_str!("../../../assets/models/dresser.mtl")),
                    load(include_str!("../../../assets/models/mailbox.obj"), include_str!("../../../assets/models/mailbox.mtl")),
                    load(include_str!("../../../assets/models/room.obj"), include_str!("../../../assets/models/room.mtl")),
                    load(include_str!("../../../assets/models/door.obj"), include_str!("../../../assets/models/door.mtl")),
                ]
            })
            .clone()
    })
}

fn object(mesh: &Rc<MeshData>, origin: [f32; 3], yaw: f32, scale: f32) -> MeshObject {
    MeshObject { center: origin, yaw, pitch: 0.0, roll: 0.0, scale: [scale; 3], color: WHITE, texture: None, walkable: false, solid: false, mesh: Rc::clone(mesh) }
}

// 방 껍데기와 문, 책상(서쪽 벽, 서랍이 방 안쪽으로 열려 있다), 책장(동쪽 벽, 열린 면이 방 쪽), 우편함(서쪽 벽, 문이 방 쪽).
// door_lift 는 문이 올라간 높이.
pub fn room_meshes(door_lift: f32) -> Vec<MeshObject> {
    let [desk, dresser, mailbox, room, door] = models();
    let door_at = [DOOR_ORIGIN[0], DOOR_ORIGIN[1] + door_lift, DOOR_ORIGIN[2]];
    vec![
        object(&room, ROOM_ORIGIN, std::f32::consts::PI, ROOM_SCALE),
        object(&door, door_at, 0.0, ROOM_SCALE),
        object(&desk, DESK_ORIGIN, std::f32::consts::FRAC_PI_2, FURNITURE_SCALE),
        object(&dresser, DRESSER_ORIGIN, std::f32::consts::PI, BOOKCASE_SCALE),
        object(&mailbox, MAILBOX_ORIGIN, 0.0, MAILBOX_SCALE),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_models_load_with_material_colors() {
        let meshes = room_meshes(0.0);
        assert_eq!(meshes.len(), 5);
        for m in &meshes {
            assert!(!m.mesh.triangles.is_empty());
            assert_eq!(m.mesh.tri_colors.len(), m.mesh.triangles.len());
        }
    }
}
