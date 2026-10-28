//! 지금 장소의 장면 — 그릴 상자(shown), 충돌 상자(collide), 조사할 물건들을 만들고, 조준선이 향한 대상을 찾는다.
//! 장면은 바뀔 때(문이 올라가는 중, 계단이 펼쳐지는 중, 진행 상태가 달라졌을 때)만 다시 만든다 — 예전엔 매
//! 프레임 방/정글의 상자 수백 개를 새로 만들고 복사했다.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::apps::game3d::ena::ena_time;
use crate::apps::game3d::item::Item;
use crate::apps::game3d::player::aim_dist;
use crate::foundation::{MainPlace, MiniKind};
use crate::render::mesh3d::{Box3D, MeshObject};

use super::floor2;
use super::rooms::{self, Spot};
use super::furniture::room_meshes;
use super::world::{aimed_target, build_room, invisible_colliders, letter_box, room_visible, Target, EXIT_POS, HANDLE_POS};
use super::{Aim, DoorsGameApp, ROOM_CLEAR};

pub(super) struct Scene {
    pub(super) key: u64,
    pub(super) clear: [f32; 4],
    pub(super) collide: Vec<Box3D>,
    pub(super) shown: Vec<Box3D>,
    // 상자가 아닌 모델(가구 등). 충돌은 collide 의 상자들이 맡는다.
    pub(super) meshes: Vec<MeshObject>,
    pub(super) room_items: Vec<Item<Spot>>,
    pub(super) jungle_items: Vec<Item<floor2::Target>>,
}

impl DoorsGameApp {
    // 장면을 다시 만들어야 하는지 가리는 값 — 장면 모양에 영향을 주는 상태만 해시한다.
    pub(super) fn scene_key(&self) -> u64 {
        let p = &self.progress;
        let mut h = DefaultHasher::new();
        p.place.hash(&mut h);
        self.door_lift.to_bits().hash(&mut h);
        self.unfold.to_bits().hash(&mut h);
        (p.quest_accepted, p.has_letter, p.tutorial_done, p.exchanged, p.chip_taken, p.chip_fed, p.hand_out).hash(&mut h);
        p.missions.hash(&mut h);
        // 뇌의 방 창밖 풍경은 ENA 식으로 초당 8번만 바뀌니 그때만 다시 만든다.
        if p.place == MainPlace::MissionRoom(MiniKind::Brain) {
            ena_time(self.clock).to_bits().hash(&mut h);
        }
        h.finish()
    }

    pub(super) fn build_scene(&self, key: u64) -> Scene {
        let p = &self.progress;
        match p.place {
            MainPlace::StartRoom => {
                let mut collide = build_room(self.door_lift);
                collide.extend(invisible_colliders());
                let mut shown = room_visible(self.door_lift);
                if p.quest_accepted && !p.has_letter {
                    collide.push(letter_box());
                    shown.push(letter_box());
                }
                Scene { key, clear: ROOM_CLEAR, collide, shown, meshes: room_meshes(self.door_lift), room_items: Vec::new(), jungle_items: Vec::new() }
            }
            MainPlace::Jungle => {
                let jungle_items = floor2::items(p.exchanged && !p.chip_taken);
                let mut shown = self.jungle.clone();
                shown.extend(jungle_items.iter().filter(|it| it.draw).map(|it| it.to_box()));
                let mut collide = shown.clone();
                collide.extend(self.barriers.iter().cloned());
                Scene { key, clear: floor2::CLEAR_COLOR, collide, shown, meshes: Vec::new(), room_items: Vec::new(), jungle_items }
            }
            MainPlace::MissionRoom(kind) => {
                let step = p.missions[kind.index()];
                let room = rooms::mission_room(kind, step, p.hand_out, self.clock);
                let mut collide = room.boxes.clone();
                collide.extend(room.barriers);
                Scene { key, clear: room.clear, collide, shown: room.boxes, meshes: Vec::new(), room_items: room.items, jungle_items: Vec::new() }
            }
            MainPlace::StairHall => {
                let room = rooms::stair_hall(p.chip_fed, self.unfold);
                let mut collide = room.boxes.clone();
                collide.extend(room.barriers);
                Scene { key, clear: room.clear, collide, shown: room.boxes, meshes: Vec::new(), room_items: room.items, jungle_items: Vec::new() }
            }
        }
    }

    // 조준선(화면 중앙)이 향한 가장 가까운 조사 대상.
    pub(super) fn compute_aim(&self, scene: &Scene) -> Option<Aim> {
        let cam = self.player.camera();
        let nearest = |centers: &mut dyn Iterator<Item = (usize, [f32; 3])>| centers.filter_map(|(i, c)| aim_dist(&cam, c).map(|d| (i, d))).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i);
        match self.progress.place {
            MainPlace::StartRoom => {
                let (accepted, door_changed, has_letter) = (self.progress.quest_accepted, self.progress.door_changed, self.progress.has_letter);
                let door_pos = if self.progress.tutorial_done { EXIT_POS } else { HANDLE_POS };
                aimed_target(&cam, door_pos, |t| match t {
                    // 수락한 직후엔 문이 말이 없다 — 편지 메일을 읽고 돌아온 뒤에야 다시 말을 한다.
                    Target::Door => !accepted || door_changed,
                    // 편지는 수락한 뒤에 책상 위에 나타나고, 집으면 사라진다.
                    Target::Letter => accepted && !has_letter,
                    _ => true,
                })
                .map(Aim::Room)
            }
            MainPlace::Jungle => nearest(&mut scene.jungle_items.iter().enumerate().map(|(i, it)| (i, it.center))).map(|i| Aim::Jungle(scene.jungle_items[i].target)),
            MainPlace::MissionRoom(_) | MainPlace::StairHall => nearest(&mut scene.room_items.iter().enumerate().map(|(i, it)| (i, it.center))).map(|i| Aim::Spot(scene.room_items[i].target)),
        }
    }
}
