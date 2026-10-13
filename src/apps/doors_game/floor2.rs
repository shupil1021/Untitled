//! 2층 — 시작방 문을 열고 나오면 펼쳐지는, 도시 한복판의 정글. 문 5개가 어디로든 문처럼 길 한복판에
//! 덩그러니 서 있다(맨 오른쪽이 방금 나온 시작방, 나머지 넷은 눈의 방 / 뇌의 방 / 손가락의 방 / 계단).
//! 바깥쪽에는 눈에 안 보이는 벽(barriers)이 있어서 플레이어는 그 너머로 못 나가고, 그 너머로는 도시
//! 건물들이 멀리 보인다. 배치(상수)와 장면 만들기만 여기에 있고, 무엇을 언제 할지는 mod.rs 가 정한다.

use crate::render::mesh3d::Box3D;

use super::super::game3d::boxes::{deco, solid};
use super::super::game3d::item::Item;

pub const CLEAR_COLOR: [f32; 4] = [0.1, 0.17, 0.2, 1.0];

const GROUND_COLOR: [f32; 4] = [0.16, 0.3, 0.14, 1.0];
const WALK_LIMIT: f32 = 20.0; // 눈에 안 보이는 벽이 서는 곳(정사각형 한 변의 절반)
const ROW_Z: f32 = -8.0; // 문들이 늘어선 줄

// 시작방에서 나와 서는 자리 — 문들을 등지고 정글을 바라본다(yaw = π 면 +Z 쪽).
pub const SPAWN: [f32; 3] = [GATE_X[4], 0.0, ROW_Z + 3.5];
pub const SPAWN_YAW: f32 = std::f32::consts::PI;

// 줄에 선 문 다섯 개의 x 좌표 — 눈 / 뇌 / 손가락 / 계단 / (방금 나온) 시작방.
const GATE_X: [f32; 5] = [-9.0, -4.5, 0.0, 4.5, 9.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gate {
    Eye,
    Brain,
    Finger,
    Stairs,
    Back, // 시작방
}

impl Gate {
    const ALL: [Gate; 5] = [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs, Gate::Back];

    fn index(self) -> usize {
        Gate::ALL.iter().position(|&g| g == self).unwrap()
    }

    pub fn center(self) -> [f32; 3] {
        [GATE_X[self.index()], 1.1, ROW_Z]
    }

    fn color(self) -> [f32; 4] {
        match self {
            Gate::Eye => [0.62, 0.14, 0.17, 1.0],
            Gate::Brain => [0.78, 0.46, 0.57, 1.0],
            Gate::Finger => [0.82, 0.72, 0.56, 1.0],
            Gate::Stairs => [0.24, 0.21, 0.29, 1.0],
            Gate::Back => [0.35, 0.24, 0.16, 1.0],
        }
    }

    fn label(self) -> &'static str {
        match self {
            Gate::Eye => "[E] Door (eye)",
            Gate::Brain => "[E] Door (brain)",
            Gate::Finger => "[E] Door (finger)",
            Gate::Stairs => "[E] Door (stairs)",
            Gate::Back => "[E] Door (the room)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    Gate(Gate),
    Sign,
    Sofa,     // 작은 거실의 소파 NPC
    Envelope, // 교환이 끝나면 소파 위에 놓이는 봉투(Mr. POTATO CHIP)
}

pub const SIGN_POS: [f32; 3] = [SPAWN[0] - 2.2, 1.2, SPAWN[2] - 0.5];

// 정글 한구석의 작은 거실 — 러그 위에 소파와 램프가 덩그러니 놓여 있다. 소파가 NPC 다.
const SOFA: [f32; 3] = [-10.0, 0.0, 9.0];
pub const SOFA_POS: [f32; 3] = [SOFA[0], 0.9, SOFA[2]];
const ENVELOPE_POS: [f32; 3] = [SOFA[0] + 0.3, 0.8, SOFA[2] - 0.1];

// 한 미션 문 앞(정글 쪽) — 방에서 나오면 이 자리에 서서 문을 등진다.
pub fn gate_front(gate: Gate) -> [f32; 3] {
    let c = gate.center();
    [c[0], 0.0, c[2] + 1.8]
}

// 매번 똑같은 숲이 되게 하는 작은 난수(고정 시드).
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

// 문 근처나 시작 자리는 나무를 심지 않는다.
fn clear_zone(x: f32, z: f32) -> bool {
    let in_row = (z - ROW_Z).abs() < 3.0 && x.abs() < 13.0;
    let near_spawn = (x - SPAWN[0]).abs() < 4.0 && (z - SPAWN[2]).abs() < 4.0;
    let near_sofa = (x - SOFA[0]).abs() < 4.0 && (z - SOFA[2]).abs() < 4.0;
    in_row || near_spawn || near_sofa
}

// 장면 전체(보이는 것) — 땅, 나무, 덤불, 먼 도시 건물, 문틀과 표지판, 작은 거실. 문 판때기는 items() 가 따로 낸다.
// done 은 눈/뇌/손가락 미션을 끝냈는지 — 끝낸 문 옆에는 초록 표식이 붙는다. exchanged 는 재료를 건넸는지(소파 자리가 늘어난다).
pub fn build_scene(done: [bool; 3], exchanged: bool) -> Vec<Box3D> {
    let mut boxes = vec![solid([0.0, -0.1, 0.0], [WALK_LIMIT + 6.0, 0.1, WALK_LIMIT + 6.0], GROUND_COLOR, true)];
    let mut rng = Lcg(0x5EED_1234);

    // 나무 — 줄기(막힌다) + 잎 덩어리 둘(머리 위라 안 막힌다).
    for _ in 0..70 {
        let (x, z) = (rng.range(-WALK_LIMIT, WALK_LIMIT), rng.range(-WALK_LIMIT, WALK_LIMIT));
        let h = rng.range(2.5, 5.5);
        let shade = rng.range(0.75, 1.15);
        if clear_zone(x, z) {
            continue;
        }
        boxes.push(solid([x, h / 2.0, z], [0.25, h / 2.0, 0.25], [0.3, 0.2, 0.12, 1.0], false));
        boxes.push(deco([x, h + 0.5, z], [1.1, 0.8, 1.1], [0.12 * shade, 0.38 * shade, 0.16 * shade, 1.0]));
        boxes.push(deco([x + 0.3, h + 1.4, z - 0.2], [0.7, 0.6, 0.7], [0.16 * shade, 0.45 * shade, 0.2 * shade, 1.0]));
    }
    // 덤불 — 낮고 넓다(스치고 지나갈 수 있다).
    for _ in 0..50 {
        let (x, z) = (rng.range(-WALK_LIMIT, WALK_LIMIT), rng.range(-WALK_LIMIT, WALK_LIMIT));
        let s = rng.range(0.4, 0.9);
        if clear_zone(x, z) {
            continue;
        }
        boxes.push(deco([x, s / 2.0, z], [s, s / 2.0, s], [0.14, 0.4, 0.17, 1.0]));
    }
    // 울타리 밖 도시 — 바닥에서 솟은 크고 어두운 건물들(창문 불빛 몇 개).
    for i in 0..44 {
        let angle = i as f32 / 44.0 * std::f32::consts::TAU;
        let dist = rng.range(30.0, 46.0);
        let (x, z) = (angle.cos() * dist, angle.sin() * dist);
        let (w, h) = (rng.range(2.5, 5.5), rng.range(10.0, 34.0));
        let tone = rng.range(0.16, 0.3);
        boxes.push(deco([x, h / 2.0, z], [w, h / 2.0, w], [tone, tone + 0.03, tone + 0.07, 1.0]));
        if rng.next() > 0.45 {
            let ly = rng.range(3.0, h - 1.0);
            boxes.push(deco([x - angle.cos() * (w + 0.05), ly, z - angle.sin() * (w + 0.05)], [0.4, 0.3, 0.4], [0.95, 0.85, 0.5, 1.0]));
        }
    }

    // 문틀(문 판때기 양옆 기둥 + 윗보) — 판때기는 items() 에서.
    for g in Gate::ALL {
        let c = g.center();
        boxes.push(solid([c[0] - 0.7, 1.15, c[2]], [0.1, 1.15, 0.12], [0.2, 0.15, 0.1, 1.0], false));
        boxes.push(solid([c[0] + 0.7, 1.15, c[2]], [0.1, 1.15, 0.12], [0.2, 0.15, 0.1, 1.0], false));
        boxes.push(solid([c[0], 2.3, c[2]], [0.8, 0.1, 0.12], [0.2, 0.15, 0.1, 1.0], false));
    }
    // 문 위의 표식 — 눈(흰 바탕에 검은 눈동자), 뇌(분홍 덩어리 둘), 손가락(막대 셋).
    let eye = Gate::Eye.center();
    boxes.push(deco([eye[0], 2.85, eye[2] + 0.05], [0.45, 0.22, 0.05], [0.95, 0.95, 0.95, 1.0]));
    boxes.push(deco([eye[0], 2.85, eye[2] + 0.12], [0.12, 0.12, 0.05], [0.05, 0.05, 0.08, 1.0]));
    let brain = Gate::Brain.center();
    boxes.push(deco([brain[0] - 0.2, 2.85, brain[2] + 0.05], [0.22, 0.2, 0.06], [0.85, 0.5, 0.6, 1.0]));
    boxes.push(deco([brain[0] + 0.2, 2.85, brain[2] + 0.05], [0.22, 0.2, 0.06], [0.78, 0.42, 0.55, 1.0]));
    let finger = Gate::Finger.center();
    for k in -1..=1 {
        boxes.push(deco([finger[0] + k as f32 * 0.22, 2.85, finger[2] + 0.05], [0.07, 0.25, 0.05], [0.88, 0.76, 0.6, 1.0]));
    }
    // 끝낸 미션 문 — 문 옆 기둥에 초록 표식.
    for (i, gate) in [Gate::Eye, Gate::Brain, Gate::Finger].into_iter().enumerate() {
        if done[i] {
            let c = gate.center();
            boxes.push(deco([c[0] + 0.7, 2.45, c[2] + 0.14], [0.16, 0.16, 0.04], [0.25, 0.9, 0.4, 1.0]));
        }
    }
    // 작은 거실 — 러그, 소파(팔걸이/등받이), 램프. 소파 얼굴은 등받이 앞면에. 교환하면 자리가 한 칸 늘어난다.
    let seat_hw = if exchanged { 1.6 } else { 1.2 };
    boxes.push(deco([SOFA[0], 0.02, SOFA[2] - 1.8], [2.6, 0.02, 2.2], [0.5, 0.2, 0.22, 1.0]));
    boxes.push(solid([SOFA[0], 0.3, SOFA[2]], [seat_hw, 0.3, 0.55], [0.35, 0.18, 0.22, 1.0], false));
    boxes.push(solid([SOFA[0], 0.85, SOFA[2] + 0.5], [seat_hw, 0.45, 0.1], [0.4, 0.2, 0.25, 1.0], false));
    boxes.push(solid([SOFA[0] - seat_hw - 0.1, 0.5, SOFA[2]], [0.12, 0.5, 0.55], [0.3, 0.15, 0.18, 1.0], false));
    boxes.push(solid([SOFA[0] + seat_hw + 0.1, 0.5, SOFA[2]], [0.12, 0.5, 0.55], [0.3, 0.15, 0.18, 1.0], false));
    boxes.push(deco([SOFA[0] - 0.35, 0.95, SOFA[2] + 0.39], [0.1, 0.12, 0.03], [0.05, 0.03, 0.03, 1.0]));
    boxes.push(deco([SOFA[0] + 0.35, 0.95, SOFA[2] + 0.39], [0.1, 0.12, 0.03], [0.05, 0.03, 0.03, 1.0]));
    boxes.push(deco([SOFA[0], 0.65, SOFA[2] + 0.39], [0.3, 0.05, 0.03], [0.05, 0.03, 0.03, 1.0]));
    boxes.push(solid([SOFA[0] + 2.4, 0.9, SOFA[2] + 0.3], [0.05, 0.9, 0.05], [0.25, 0.22, 0.2, 1.0], false));
    boxes.push(deco([SOFA[0] + 2.4, 1.9, SOFA[2] + 0.3], [0.3, 0.2, 0.3], [0.95, 0.85, 0.5, 1.0]));
    // 표지판 — 기둥 하나에 판때기 하나.
    boxes.push(solid([SIGN_POS[0], 0.6, SIGN_POS[2]], [0.06, 0.6, 0.06], [0.3, 0.22, 0.14, 1.0], false));
    boxes.push(solid(SIGN_POS, [0.45, 0.25, 0.04], [0.55, 0.45, 0.3, 1.0], false));
    boxes
}

// 안 보이는 벽 — 충돌에만 쓴다(그리지 않는다). 그 너머로 도시 배경이 보인다.
pub fn barriers() -> Vec<Box3D> {
    let l = WALK_LIMIT;
    let t = 0.5;
    let h = 4.0;
    let wall = |c: [f32; 3], half: [f32; 3]| solid(c, half, [0.0; 4], false);
    vec![
        wall([0.0, h, -l - t], [l + 1.0, h, t]),
        wall([0.0, h, l + t], [l + 1.0, h, t]),
        wall([-l - t, h, 0.0], [t, h, l + 1.0]),
        wall([l + t, h, 0.0], [t, h, l + 1.0]),
    ]
}

// 조사할 수 있는 것들 — 문 5개, 표지판, 소파 NPC, (교환이 끝났고 아직 안 집었으면) 소파 위 봉투.
pub fn items(envelope: bool) -> Vec<Item<Target>> {
    let mut items: Vec<Item<Target>> = Gate::ALL
        .into_iter()
        .map(|g| Item::new(Target::Gate(g), g.center(), [0.6, 1.1, 0.06], g.color(), g.label()))
        .collect();
    items.push(Item::marker(Target::Sign, SIGN_POS, [0.45, 0.25, 0.1], "[E] Sign"));
    items.push(Item::marker(Target::Sofa, SOFA_POS, [1.5, 0.9, 0.8], "[E] Sofa"));
    if envelope {
        items.push(Item::new(Target::Envelope, ENVELOPE_POS, [0.22, 0.03, 0.15], [0.96, 0.93, 0.8, 1.0], "[E] Envelope"));
    }
    items
}
