//! 2층의 방들 — 미션 방 셋(눈 / 뇌 / 손가락)과 계단 입구. 방마다 같은 틀이다: 방 가운데쯤에 서서 시작하고,
//! 안쪽에 NPC 와 "게임 받기" 기계(Spot::Npc / Spot::Machine)가 있고, 시작한 자리 뒤쪽 벽에 나가는 문
//! (Spot::Exit)이 있다. 배치와 모양만 여기에 있고 무엇을 언제 할지는 mod.rs 가 정한다.

use crate::foundation::MiniKind;
use crate::render::mesh3d::Box3D;

use super::super::game3d::ena::{ena_time, smooth};
use super::super::game3d::boxes::{block, deco};
use super::super::game3d::item::Item;

const MIN_X: f32 = -5.0;
const MAX_X: f32 = 5.0;
const MIN_Z: f32 = -6.0;
const MAX_Z: f32 = 3.0;
const HEIGHT: f32 = 3.0;
const THICK: f32 = 0.15;

pub const SPAWN: [f32; 3] = [0.0, 0.0, 1.4];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spot {
    Npc,
    Machine,
    Exit,
    StairStep, // 펼쳐진 계단의 첫 칸
}

pub struct Room {
    pub boxes: Vec<Box3D>,
    pub items: Vec<Item<Spot>>,
    pub clear: [f32; 4],
    // 충돌에만 쓰는 눈에 안 보이는 벽(그리지 않는다) — 계단 아래 낭떠러지로 못 떨어지게 한다.
    pub barriers: Vec<Box3D>,
}

// 바닥/천장/벽 4면. floor_to_z 는 바닥이 어디까지 깔려 있는지(계단 입구는 안쪽이 낭떠러지).
fn shell(wall: [f32; 4], floor: [f32; 4], floor_min_z: f32) -> Vec<Box3D> {
    let (cx, cz) = ((MIN_X + MAX_X) / 2.0, (MIN_Z + MAX_Z) / 2.0);
    let hx = (MAX_X - MIN_X) / 2.0 + THICK;
    let hz = (MAX_Z - MIN_Z) / 2.0 + THICK;
    let hy = HEIGHT / 2.0;
    let floor_cz = (floor_min_z + MAX_Z) / 2.0;
    let floor_hz = (MAX_Z - floor_min_z) / 2.0 + THICK;
    vec![
        Box3D { walkable: true, ..block([cx, -0.1, floor_cz], [hx, 0.1, floor_hz], floor) },
        block([cx, HEIGHT + 0.1, cz], [hx, 0.1, hz], [wall[0] * 0.8, wall[1] * 0.8, wall[2] * 0.8, 1.0]),
        block([MIN_X, hy, cz], [THICK, hy, hz], wall),
        block([MAX_X, hy, cz], [THICK, hy, hz], wall),
        block([cx, hy, MIN_Z], [hx, hy, THICK], wall),
        block([cx, hy, MAX_Z], [hx, hy, THICK], wall),
    ]
}

fn exit_door(items: &mut Vec<Item<Spot>>, boxes: &mut Vec<Box3D>) {
    // 나가는 문 — 시작한 자리 뒤(남쪽 벽) 가운데.
    boxes.push(block([0.0, 1.1, MAX_Z - THICK - 0.06], [0.6, 1.1, 0.06], [0.35, 0.24, 0.16, 1.0]));
    items.push(Item::marker(Spot::Exit, [0.0, 1.1, MAX_Z - THICK - 0.1], [0.65, 1.1, 0.2], "[E] Exit"));
}

// 미션 방. done 은 그 미션의 NPC 와 마지막 이야기까지 마쳤는지(방이 달라 보이는 곳이 있다), clock 은 움직이는 풍경용.
pub fn mission_room(kind: MiniKind, done: bool, received: bool, clock: f32) -> Room {
    match kind {
        MiniKind::Eye => eye_room(done, received),
        MiniKind::Brain => brain_room(done, clock),
        MiniKind::Finger => finger_room(done),
    }
}

fn eye_room(done: bool, received: bool) -> Room {
    let mut boxes = shell([0.28, 0.1, 0.13, 1.0], [0.18, 0.08, 0.09, 1.0], MIN_Z);
    let mut items = Vec::new();
    // 큰 눈 간판 — 눈의 방을 마치면(재료를 받으면) 눈이 감긴다.
    boxes.push(deco([0.0, 2.0, MIN_Z + THICK + 0.05], [2.2, 0.85, 0.05], [0.95, 0.9, 0.9, 1.0]));
    boxes.push(deco([0.0, 2.0, MIN_Z + THICK + 0.1], [0.75, 0.75, 0.05], [0.4, 0.13, 0.1, 1.0]));
    boxes.push(deco([0.0, 2.0, MIN_Z + THICK + 0.15], [0.32, 0.32, 0.05], [0.02, 0.0, 0.0, 1.0]));
    if received {
        boxes.push(deco([0.0, 2.45, MIN_Z + THICK + 0.22], [2.25, 0.45, 0.06], [0.35, 0.1, 0.12, 1.0]));
    }
    // 렌즈 얼굴 NPC — 몸통, 머리, 렌즈. 끝나면 렌즈가 꺼진다.
    boxes.push(block([-2.5, 0.9, -4.0], [0.35, 0.9, 0.25], [0.3, 0.32, 0.4, 1.0]));
    boxes.push(block([-2.5, 2.0, -4.0], [0.42, 0.42, 0.4], [0.4, 0.42, 0.5, 1.0]));
    let lens = if done { [0.12, 0.12, 0.14, 1.0] } else { [0.7, 0.9, 1.0, 1.0] };
    boxes.push(deco([-2.5, 2.0, -3.57], [0.3, 0.3, 0.04], lens));
    items.push(Item::marker(Spot::Npc, [-2.5, 1.7, -4.0], [0.6, 1.0, 0.6], "[E] Talk"));
    // 아케이드 기계
    boxes.push(block([2.8, 0.9, -4.5], [0.5, 0.9, 0.4], [0.1, 0.1, 0.16, 1.0]));
    boxes.push(deco([2.8, 1.35, -4.09], [0.38, 0.3, 0.03], [0.3, 0.95, 1.0, 1.0]));
    boxes.push(deco([2.8, 0.85, -4.07], [0.3, 0.06, 0.04], [0.9, 0.2, 0.25, 1.0]));
    items.push(Item::marker(Spot::Machine, [2.8, 1.2, -4.1], [0.6, 0.9, 0.5], "[E] Arcade machine"));
    exit_door(&mut items, &mut boxes);
    Room { boxes, items, clear: [0.1, 0.03, 0.05, 1.0], barriers: Vec::new() }
}

fn brain_room(done: bool, clock: f32) -> Room {
    let mut boxes = shell([0.12, 0.26, 0.28, 1.0], [0.14, 0.15, 0.16, 1.0], MIN_Z);
    let mut items = Vec::new();
    // 바닥에 그려진 도로와 점선
    boxes.push(deco([0.0, 0.015, -1.5], [2.4, 0.015, 4.5], [0.08, 0.08, 0.09, 1.0]));
    for i in 0..7 {
        boxes.push(deco([0.0, 0.035, -5.2 + i as f32 * 1.35], [0.07, 0.01, 0.4], [0.9, 0.9, 0.6, 1.0]));
    }
    // 동쪽 벽의 창 — 풍경 줄무늬가 뚝뚝 끊겨 지나가다(ENA 식), 끝나면 멈춘다.
    boxes.push(deco([MAX_X - THICK - 0.04, 1.7, -2.0], [0.04, 0.85, 1.7], [0.45, 0.6, 0.75, 1.0]));
    let phase = if done { 0.0 } else { ena_time(clock) * 0.9 };
    for i in 0..6 {
        let z = -3.5 + ((i as f32 * 0.6 + phase) % 3.4);
        let h = 0.3 + 0.5 * ((i * 37 % 5) as f32 / 4.0);
        boxes.push(deco([MAX_X - THICK - 0.09, 0.9 + h / 2.0, z], [0.03, h / 2.0, 0.22], [0.2 + 0.1 * i as f32 / 6.0, 0.3, 0.35, 1.0]));
    }
    // 도로 표지판 NPC — 기둥 하나에 둥근 판(박스).
    boxes.push(block([-2.6, 0.9, -4.2], [0.08, 0.9, 0.08], [0.5, 0.5, 0.52, 1.0]));
    boxes.push(block([-2.6, 1.95, -4.2], [0.55, 0.4, 0.07], [0.15, 0.4, 0.2, 1.0]));
    let arrow = if done { [0.9, 0.5, 0.5, 1.0] } else { [0.95, 0.95, 0.95, 1.0] };
    boxes.push(deco([-2.6, 1.95, -4.12], [0.35, 0.07, 0.02], arrow));
    items.push(Item::marker(Spot::Npc, [-2.6, 1.6, -4.2], [0.65, 1.0, 0.5], "[E] Talk"));
    // 운전대 기계
    boxes.push(block([2.8, 0.55, -4.5], [0.5, 0.55, 0.4], [0.18, 0.15, 0.2, 1.0]));
    boxes.push(deco([2.8, 1.25, -4.2], [0.42, 0.04, 0.42], [0.1, 0.1, 0.1, 1.0]));
    boxes.push(deco([2.8, 1.25, -4.2], [0.08, 0.06, 0.08], [0.9, 0.2, 0.25, 1.0]));
    items.push(Item::marker(Spot::Machine, [2.8, 1.1, -4.2], [0.6, 0.7, 0.5], "[E] Driving machine"));
    exit_door(&mut items, &mut boxes);
    Room { boxes, items, clear: [0.04, 0.1, 0.11, 1.0], barriers: Vec::new() }
}

fn finger_room(done: bool) -> Room {
    let mut boxes = shell([0.52, 0.44, 0.34, 1.0], [0.3, 0.26, 0.2, 1.0], MIN_Z);
    let mut items = Vec::new();
    // 컨베이어와 쌓인 상자들
    boxes.push(block([0.0, 0.35, -3.2], [3.4, 0.35, 0.55], [0.18, 0.18, 0.2, 1.0]));
    for i in 0..5 {
        boxes.push(deco([-2.4 + i as f32 * 1.1, 0.95, -3.2], [0.3, 0.25, 0.3], [0.68, 0.5, 0.3, 1.0]));
    }
    for k in 0..3 {
        boxes.push(block([-4.2, 0.3 + k as f32 * 0.6, -5.2], [0.5, 0.3, 0.5], [0.6, 0.44, 0.26, 1.0]));
    }
    // 장갑 NPC — 몸통, 머리, 하얀 장갑 두 개
    boxes.push(block([-2.8, 0.9, -4.6], [0.35, 0.9, 0.25], [0.25, 0.3, 0.45, 1.0]));
    boxes.push(block([-2.8, 1.95, -4.6], [0.32, 0.32, 0.3], [0.75, 0.6, 0.5, 1.0]));
    boxes.push(deco([-3.3, 1.2, -4.2], [0.18, 0.2, 0.15], [0.97, 0.97, 0.97, 1.0]));
    boxes.push(deco([-2.3, 1.2, -4.2], [0.18, 0.2, 0.15], [0.97, 0.97, 0.97, 1.0]));
    items.push(Item::marker(Spot::Npc, [-2.8, 1.6, -4.6], [0.6, 1.0, 0.5], "[E] Talk"));
    // 포장 기계 — 멈춰 있고, 앞에 대기 버튼.
    boxes.push(block([2.8, 1.0, -4.8], [0.7, 1.0, 0.6], [0.3, 0.32, 0.3, 1.0]));
    let lamp = if done { [0.3, 0.9, 0.4, 1.0] } else { [0.9, 0.7, 0.2, 1.0] };
    boxes.push(deco([2.8, 1.1, -4.18], [0.2, 0.2, 0.05], [0.85, 0.15, 0.18, 1.0]));
    boxes.push(deco([2.8, 1.7, -4.18], [0.1, 0.06, 0.03], lamp));
    items.push(Item::marker(Spot::Machine, [2.8, 1.1, -4.2], [0.6, 0.9, 0.5], "[E] Standby button"));
    exit_door(&mut items, &mut boxes);
    Room { boxes, items, clear: [0.12, 0.1, 0.07, 1.0], barriers: Vec::new() }
}

// 계단 입구 — 안쪽(북쪽)은 낭떠러지고, 계단 NPC 가 웅크려 길을 막고 있다. 먹고 나면 몸을 펼쳐 아래로
// 가는 계단이 된다. unfold 는 0(웅크림) ~ 1(다 펼쳐짐), 계단 칸이 하나씩 뚝뚝 끊겨 생긴다(ENA 식).
const STAIR_STEPS: usize = 8;

pub fn stair_hall(fed: bool, unfold: f32) -> Room {
    let pit_z = -3.0;
    let mut boxes = shell([0.17, 0.13, 0.22, 1.0], [0.2, 0.17, 0.25, 1.0], pit_z);
    let mut items = Vec::new();
    // 낭떠러지 쪽 가장자리의 난간 느낌(낮은 턱)은 두지 않는다 — 계단 NPC 가 길을 막는다.
    if !fed || unfold < 1.0 {
        // 웅크린 계단 NPC: 계단 모양 덩어리 + 눈 둘 + 입.
        let squash = if fed { 1.0 - smooth(unfold) } else { 1.0 };
        let h = 0.5 + 0.9 * squash;
        boxes.push(block([0.0, h, pit_z - 0.6], [1.7 * (0.6 + 0.4 * squash), h, 0.7], [0.32, 0.28, 0.4, 1.0]));
        boxes.push(deco([-0.5, h * 1.6, pit_z + 0.12], [0.16, 0.16, 0.04], [0.95, 0.95, 0.8, 1.0]));
        boxes.push(deco([0.5, h * 1.6, pit_z + 0.12], [0.16, 0.16, 0.04], [0.95, 0.95, 0.8, 1.0]));
        let mouth = if fed { 0.18 } else { 0.07 };
        boxes.push(deco([0.0, h * 0.8, pit_z + 0.12], [0.6, mouth, 0.04], [0.05, 0.0, 0.02, 1.0]));
        if !fed {
            items.push(Item::marker(Spot::Npc, [0.0, h * 1.2, pit_z + 0.1], [1.5, 1.0, 0.6], "[E] Stairs"));
        }
    }
    if fed {
        // 펼쳐진 계단 — unfold 에 따라 칸이 하나씩 생긴다.
        let shown = ((ena_time(unfold) * STAIR_STEPS as f32).ceil() as usize).min(STAIR_STEPS);
        for n in 0..shown {
            let y = -0.32 * n as f32 - 0.15;
            boxes.push(Box3D { walkable: true, ..block([0.0, y, pit_z - 0.6 - 1.0 * n as f32], [1.4, 0.15, 0.5], [0.3 + 0.02 * n as f32, 0.26, 0.36, 1.0]) });
        }
        if unfold >= 1.0 {
            items.push(Item::marker(Spot::StairStep, [0.0, 0.6, pit_z + 0.4], [1.2, 0.8, 0.8], "[E] Stairs"));
        }
    }
    exit_door(&mut items, &mut boxes);
    // 낭떠러지 입구는 막혀 있고, 다 펼쳐진 뒤엔 계단 폭만큼만 열린다(양옆과 끝은 벽).
    let wall = |c: [f32; 3], half: [f32; 3]| block(c, half, [0.0; 4]);
    let mut barriers = vec![wall([-3.4, 1.5, pit_z - 0.2], [1.7, 1.5, 0.2]), wall([3.4, 1.5, pit_z - 0.2], [1.7, 1.5, 0.2])];
    if fed && unfold >= 1.0 {
        let end_z = pit_z - 0.6 - STAIR_STEPS as f32;
        barriers.push(wall([-1.95, 1.5, (pit_z + end_z) / 2.0], [0.2, 1.5, (pit_z - end_z) / 2.0 + 0.5]));
        barriers.push(wall([1.95, 1.5, (pit_z + end_z) / 2.0], [0.2, 1.5, (pit_z - end_z) / 2.0 + 0.5]));
        barriers.push(wall([0.0, 1.5, end_z - 0.4], [2.0, 1.5, 0.2]));
    } else {
        barriers.push(wall([0.0, 1.5, pit_z - 0.2], [1.8, 1.5, 0.2]));
    }
    Room { boxes, items, clear: [0.05, 0.04, 0.08, 1.0], barriers }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_room_has_an_exit_and_a_way_to_talk() {
        for kind in MiniKind::ALL {
            let room = mission_room(kind, false, false, 0.0);
            for spot in [Spot::Npc, Spot::Machine, Spot::Exit] {
                assert!(room.items.iter().any(|it| it.target == spot), "{kind:?} 방에 {spot:?} 가 없다");
            }
        }
        let hall = stair_hall(false, 0.0);
        assert!(hall.items.iter().any(|it| it.target == Spot::Npc) && hall.items.iter().any(|it| it.target == Spot::Exit));
    }

    #[test]
    fn stairs_unfold_into_a_walkable_path() {
        let closed = stair_hall(false, 0.0);
        let open = stair_hall(true, 1.0);
        assert!(closed.items.iter().all(|it| it.target != Spot::StairStep));
        assert!(open.items.iter().any(|it| it.target == Spot::StairStep) && open.items.iter().all(|it| it.target != Spot::Npc));
        assert!(open.boxes.iter().filter(|b| b.walkable).count() > closed.boxes.iter().filter(|b| b.walkable).count() + 4);
        // 펼치는 도중에는 칸이 일부만 생겨 있다.
        let mid = stair_hall(true, 0.4);
        let steps = |r: &Room| r.boxes.iter().filter(|b| b.walkable).count();
        assert!(steps(&mid) > steps(&closed) && steps(&mid) < steps(&open));
    }

    #[test]
    fn done_changes_the_look_of_the_room() {
        let a = mission_room(MiniKind::Eye, false, false, 0.0);
        let b = mission_room(MiniKind::Eye, true, true, 0.0);
        assert!(b.boxes.len() > a.boxes.len(), "눈이 감긴다(눈꺼풀 상자가 늘어난다)");
    }
}
