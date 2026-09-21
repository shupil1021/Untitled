// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::mesh3d`(진짜 3D 메쉬 렌더러)만 따로 띄워서 확인하는 테스트 창.
//! `cargo run --bin mesh3d_test` 로 띄운다.
//!
//! 장면은 전부 `Box3D`(회전 가능한 직육면체) 하나로만 만든다: 평평한 바닥,
//! 기울어진 경사로(램프, pitch 회전), 그 위 높은 발판, 옆으로 기운 벽(roll 회전)
//! — 바닥 높낮이도 기울어진 벽/경사로도 전부 같은 상자 타입 하나로 표현된다는 걸
//! 보여주는 게 목적이다. 스폰 지점은 쓰레기 무더기가 쌓인 방(`build_room`) 안이고,
//! 이 방을 나가는 문간은 문짝(`door_box`)이 경첩(문간 서쪽 가장자리를 지나는
//! 수직축, `hinge_rotate`)을 축으로 회전할 수 있게 만들어뒀지만, 지금은 항상
//! 닫힌 채로 진짜 장애물 역할만 한다(손잡이 상호작용은 대화창 참고). 바닥엔
//! 작은 아이템(열쇠/쪽지/손전등, 전부 이 프로젝트의 기존 디자인 그대로
//! `Box3D` 하나로 표현) 몇 개를 흩어놨다.
//!
//! 조작: W/S 전진/후진, A/D 좌우 이동(strafe), Space 로 점프. **마우스를
//! 움직이면 시점이 돈다**(FPS 게임처럼 커서를 숨기고 창에 가둔다 —
//! `raw_mouse_motion`, OS 커서 가속/클램프의 영향을 안 받는 원시 입력이라
//! 회전이 매끄럽다). ↑/↓ 로도 피치를 돌릴 수 있다(키보드만으로도 확인할 수
//! 있게 남겨뒀다).
//!
//! 문 손잡이에 조준선을 가까이 대면 "[E] Examine"이 뜨고, `E`를 누르면 화면
//! 아래쪽에 짧은 대화창(지금은 "그냥 문고리다..." 고정 한 줄, `dialogue_lines`
//! 참고 — 나중에 여러 줄로 늘어날 걸 대비한 구조)이 타자기처럼 한 글자씩
//! (글자마다 살짝 무작위한 간격으로) 나타난다. 대화창이 떠 있는 동안은
//! 이동·시점 회전이 전부 멈추고, 아무 키/마우스 버튼이나 누르면 지금 줄을
//! 즉시 다 보여주거나(타이핑 중이었으면) 다음 줄로 넘어간다(더 없으면
//! 닫힌다). 아이템에 조준선(화면 중앙)을 가까이 대면 그 옆에
//! "[E] Inspect 이름"이 뜬다 —
//! `E` 를 누르면 화면 가운데에 그 아이템만 확대해서 보여주는 작은 창이 뜨고,
//! 그 동안 플레이어는 멈추고 **마우스 오른쪽 버튼을 누른 채 드래그**하면 그
//! 아이템을 그 자리에서 돌려가며 볼 수 있다. `E`나 `Esc`를 다시 누르면 닫힌다
//! (평소엔 `Esc`가 바로 종료).
//!
//! 실제 게임처럼 CRT 셰이더(곡률/스캔라인/새도마스크/비네팅) + 색수차를
//! 씌운다 — `crt.rs`(`main.rs`가 쓰는 것과 완전히 같은 모듈)를 그대로
//! 가져다 쓴다: `mesh3d.render()`로 3D 장면을 그 자신의 오프스크린 타깃에
//! 그리고, 그 결과 텍스처 + HUD(+ 아이템 확대 창) 를 2D 렌더러로 한 번 더
//! 합성한 뒤, 그 합성본 전체를 `Crt`의 오프스크린 타깃에 흘려보내
//! (`renderer.flush`의 대상을 `crt.begin()`이 그쪽으로 돌려놓는다) 마지막에
//! `crt.present()`가 곡면 왜곡 + 색수차를 입혀 진짜 화면에 그린다
//! (main.rs::draw()와 같은 순서).

use miniquad::*;

use crackhead::crt::Crt;
use crackhead::gfx::Renderer;
use crackhead::mesh3d::{ground_height, resolve_horizontal, v_dot, v_sub, Box3D, BoxTexture, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
// 실제 게임의 기본값(foundation.rs::Settings::default)과 맞춘다.
const CHROMATIC_ABERRATION: f32 = 0.5;
const CRT_INTENSITY: f32 = 1.0;

const MOVE_SPEED: f32 = 3.2;
const LOOK_SPEED: f32 = 1.6; // 키보드(↑/↓)용
const MOUSE_SENS: f32 = 0.0032; // 마우스 1px(원시 입력) 당 라디안
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55; // 발 기준 눈 높이
const GRAVITY: f32 = -12.0;
const JUMP_SPEED: f32 = 4.6;

const SKY_COLOR: [f32; 4] = [0.55, 0.65, 0.78, 1.0];
const GROUND_COLOR: [f32; 4] = [0.32, 0.45, 0.3, 1.0];
const RAMP_COLOR: [f32; 4] = [0.5, 0.42, 0.3, 1.0];
const PLATFORM_COLOR: [f32; 4] = [0.45, 0.45, 0.5, 1.0];
const WALL_COLOR: [f32; 4] = [0.55, 0.3, 0.3, 1.0];
const LEANING_WALL_COLOR: [f32; 4] = [0.35, 0.35, 0.6, 1.0];

// 스폰 지점을 감싸는 쓰레기 방 — 북쪽(+Z) 벽에 문 하나를 뚫어서 기존 경사로
// 코스(ramp_start_z=3.0 부터)로 이어지게 한다. 서쪽 벽을 충분히 멀리 둬서
// 기존 "옆으로 기운 벽"(LEANING_WALL_COLOR, x=-4.5) 데모도 그대로 방 안에
// 들어오게 했다 — 방 안의 또 다른 잡동사니처럼 보인다.
const ROOM_MIN_X: f32 = -8.0; // 기존 "옆으로 기운 벽" 데모(x=-4.5, 대각선으로 튀어나온 회전체)가 안 걸리게 넉넉히
const ROOM_MAX_X: f32 = 3.0;
const ROOM_MIN_Z: f32 = -6.0;
const ROOM_MAX_Z: f32 = 0.4;
const ROOM_WALL_HALF_Y: f32 = 1.3;
const ROOM_WALL_THICK: f32 = 0.15;
const ROOM_WALL_COLOR: [f32; 4] = [0.4, 0.38, 0.35, 1.0];

// 문 — 경첩(DOOR_HINGE_X, ROOM_MAX_Z 위치의 수직선)을 축으로 문짝과 손잡이
// 둘 다 회전시킨다(door_box/door_handle_pos 가 매 프레임 새로 계산). 북쪽
// 벽의 문간은 정확히 [DOOR_HINGE_X, DOOR_HINGE_X+DOOR_WIDTH] 만큼 뚫려있고
// (build_room 참고), 닫힌 문짝이 그 틈을 정확히 채운다.
const DOOR_HINGE_X: f32 = -0.8; // 문간(대략 x=0 기준)의 서쪽 가장자리 — 경첩 위치
const DOOR_WIDTH: f32 = 1.5;
const DOOR_HALF_W: f32 = DOOR_WIDTH / 2.0;
const DOOR_HALF_H: f32 = 1.15;
const DOOR_HALF_T: f32 = 0.06;
const DOOR_Y: f32 = DOOR_HALF_H;
const DOOR_COLOR: [f32; 4] = [0.35, 0.24, 0.16, 1.0]; // 나무색
const DOOR_OPEN_ANGLE: f32 = std::f32::consts::PI * 0.55; // 약 99도
const DOOR_ANIM_SPEED: f32 = 2.2; // door_anim(0~1) 초당 변화량 — 완전히 열리는 데 ~0.45초

// 손잡이 상호작용 — 일단은 문을 실제로 열지 않고 짧은 대화 문구만 띄운다.
const DOORKNOB_MESSAGE: &str = "그냥 문고리다...";

// 대화창 — 타자기처럼 한 글자씩, 글자마다 살짝 다른(무작위) 간격으로
// 나타난다. 화면 맨 아래에 딱 붙이지 않고 위로 좀 띄우고 좌우/아래 여백을
// 둔다. 대화창이 떠 있는 동안은 시점 회전·이동이 전부 멈추고, 아무 입력이나
// 오면 지금 줄을 즉시 다 보여주거나(아직 타이핑 중이면) 다음 줄로 넘어간다
// (더 없으면 닫힌다).
const DIALOGUE_SIDE_MARGIN: f32 = 24.0;
const DIALOGUE_BOTTOM_MARGIN: f32 = 36.0;
const DIALOGUE_HEIGHT: f32 = 58.0;
const DIALOGUE_TEXT_SCALE: f32 = 0.95;
const DIALOGUE_CHAR_DELAY_MIN: f64 = 0.02;
const DIALOGUE_CHAR_DELAY_MAX: f64 = 0.09;

const HANDLE_HALF: [f32; 3] = [0.05, 0.05, 0.05];
const HANDLE_COLOR: [f32; 4] = [0.8, 0.72, 0.45, 1.0]; // 놋쇠색

fn move_toward(cur: f32, target: f32, max_delta: f32) -> f32 {
    if (target - cur).abs() <= max_delta {
        target
    } else {
        cur + max_delta * (target - cur).signum()
    }
}

// 아주 작은 xorshift64 PRNG — 대화창 타자기 효과의 글자별 간격을 살짝씩
// 흔드는 용도라 암호학적으로 안전할 필요가 없다(외부 rand 크레이트 없이도
// 충분). 0.0..1.0 사이 값을 돌려주고 seed 를 그 자리에서 갱신한다.
fn rand01(seed: &mut u64) -> f32 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *seed = x;
    (x % 1_000_000) as f32 / 1_000_000.0
}

// hinge 를 지나는 수직축 기준으로 로컬 오프셋(문이 닫혀있을 때 기준의 상대
// 위치)을 angle 만큼 돌린 "지금" 월드 위치 — mesh3d.rs::mat_rotate_y 와 같은
// 회전 부호를 써야 문짝의 실제 렌더링 회전(yaw=angle)과 이 위치 계산이
// 어긋나지 않는다(안 맞으면 경첩이 아니라 이상한 축으로 미끄러지듯 움직여
// 보인다).
fn hinge_rotate(hinge: [f32; 3], local_offset: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    let x = local_offset[0] * c + local_offset[2] * s;
    let z = -local_offset[0] * s + local_offset[2] * c;
    [hinge[0] + x, hinge[1] + local_offset[1], hinge[2] + z]
}

fn door_hinge() -> [f32; 3] {
    [DOOR_HINGE_X, DOOR_Y, ROOM_MAX_Z]
}

fn door_angle(anim: f32) -> f32 {
    anim * DOOR_OPEN_ANGLE
}

// anim(0=닫힘~1=열림)에 맞는 문짝의 "지금" Box3D — 매 프레임 새로 계산해서
// 충돌 목록/렌더 목록에 넣는다(닫혀 있으면 진짜로 막는 장애물이 된다).
fn door_box(anim: f32) -> Box3D {
    let angle = door_angle(anim);
    let center = hinge_rotate(door_hinge(), [DOOR_HALF_W, 0.0, 0.0], angle);
    Box3D { center, half: [DOOR_HALF_W, DOOR_HALF_H, DOOR_HALF_T], yaw: angle, pitch: 0.0, roll: 0.0, color: DOOR_COLOR, texture: None, walkable: false, solid: true }
}

// 손잡이는 문의 경첩 반대쪽(먼) 가장자리, 방 안쪽 면에서 살짝 튀어나온
// 자리 — 문과 똑같은 경첩 회전을 타므로 문이 도는 대로 같이 따라 돈다.
fn door_handle_pos(anim: f32) -> [f32; 3] {
    let angle = door_angle(anim);
    hinge_rotate(door_hinge(), [DOOR_WIDTH - 0.15, -0.45, -0.09], angle)
}

fn door_handle_box(anim: f32) -> Box3D {
    let angle = door_angle(anim);
    let center = door_handle_pos(anim);
    Box3D { center, half: HANDLE_HALF, yaw: angle, pitch: 0.0, roll: 0.0, color: HANDLE_COLOR, texture: None, walkable: false, solid: false }
}

// 스폰을 감싸는 방 — 벽 4면(북쪽은 문간을 남기고 두 조각으로) + 쓰레기 더미.
fn build_room() -> Vec<Box3D> {
    let mut boxes = Vec::new();
    let center_x = (ROOM_MIN_X + ROOM_MAX_X) / 2.0;
    let center_z = (ROOM_MIN_Z + ROOM_MAX_Z) / 2.0;
    let half_x = (ROOM_MAX_X - ROOM_MIN_X) / 2.0;
    let half_z = (ROOM_MAX_Z - ROOM_MIN_Z) / 2.0;

    let wall = |center: [f32; 3], half: [f32; 3]| Box3D {
        center,
        half,
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: ROOM_WALL_COLOR,
        texture: None,
        walkable: false,
        solid: true,
    };

    // 서쪽/동쪽 벽.
    boxes.push(wall([ROOM_MIN_X, ROOM_WALL_HALF_Y, center_z], [ROOM_WALL_THICK, ROOM_WALL_HALF_Y, half_z + ROOM_WALL_THICK]));
    boxes.push(wall([ROOM_MAX_X, ROOM_WALL_HALF_Y, center_z], [ROOM_WALL_THICK, ROOM_WALL_HALF_Y, half_z + ROOM_WALL_THICK]));
    // 남쪽 벽(막힌 벽).
    boxes.push(wall([center_x, ROOM_WALL_HALF_Y, ROOM_MIN_Z], [half_x + ROOM_WALL_THICK, ROOM_WALL_HALF_Y, ROOM_WALL_THICK]));
    // 북쪽 벽 — 가운데 문간(폭 DOOR_GAP_HALF*2)만 비우고 좌우 두 조각으로.
    let left_w = (DOOR_HINGE_X - ROOM_MIN_X) / 2.0;
    boxes.push(wall([ROOM_MIN_X + left_w, ROOM_WALL_HALF_Y, ROOM_MAX_Z], [left_w, ROOM_WALL_HALF_Y, ROOM_WALL_THICK]));
    let right_start = DOOR_HINGE_X + DOOR_WIDTH;
    let right_w = (ROOM_MAX_X - right_start) / 2.0;
    boxes.push(wall([right_start + right_w, ROOM_WALL_HALF_Y, ROOM_MAX_Z], [right_w, ROOM_WALL_HALF_Y, ROOM_WALL_THICK]));

    // 쓰레기 더미 — 벽 쪽에 몰아서 스폰↔문 사이 통로는 비워둔다.
    // (위치, 반너비, 색, yaw, roll)
    type TrashSpec = ([f32; 3], [f32; 3], [f32; 4], f32, f32);
    let trash: &[TrashSpec] = &[
        ([-5.2, 0.2, -5.2], [0.4, 0.2, 0.35], [0.42, 0.32, 0.18, 1.0], 0.3, 0.0),
        ([-4.6, 0.15, -4.6], [0.3, 0.15, 0.3], [0.25, 0.3, 0.18, 1.0], -0.6, 0.15),
        ([-5.5, 0.25, -2.0], [0.35, 0.25, 0.4], [0.4, 0.4, 0.42, 1.0], 1.1, 0.0),
        ([-2.6, 0.18, -5.5], [0.3, 0.18, 0.25], [0.5, 0.28, 0.15, 1.0], 0.4, 0.0),
        ([-0.8, 0.15, -5.3], [0.25, 0.15, 0.3], [0.4, 0.3, 0.2, 1.0], -0.2, 0.0),
        ([2.0, 0.2, -5.0], [0.35, 0.2, 0.3], [0.28, 0.34, 0.2, 1.0], 0.7, 0.0),
        ([2.3, 0.15, -3.0], [0.25, 0.15, 0.25], [0.42, 0.42, 0.44, 1.0], -0.9, 0.0),
        ([2.2, 0.3, -1.0], [0.4, 0.3, 0.35], [0.45, 0.32, 0.2, 1.0], 0.15, 0.1),
        ([-5.7, 0.2, -0.8], [0.25, 0.2, 0.3], [0.5, 0.3, 0.16, 1.0], -0.3, 0.0),
        ([-3.3, 0.15, -0.9], [0.3, 0.15, 0.25], [0.27, 0.33, 0.19, 1.0], 0.5, 0.0),
    ];
    for &(pos, half, color, yaw, roll) in trash {
        boxes.push(Box3D { center: pos, half, yaw, pitch: 0.0, roll, color, texture: None, walkable: false, solid: true });
    }

    boxes
}

// 조준(화면 중앙, 카메라 정면 방향) 판정 — 이 거리 안 + 이 각도(코사인) 안에
// 있는 아이템 중 가장 가까운 것 하나만 "조준 중"으로 친다. 벽에 가려져 있어도
// 뚫고 판정되는 단순화된 방식이다(진짜 레이캐스트 대신 각도+거리만 본다).
const AIM_MAX_DIST: f32 = 3.0;
const AIM_MAX_COS: f32 = 0.95; // 대략 앞쪽 ±18도

const ITEM_ROTATE_SENS: f32 = 0.008; // 아이템 확대 창에서 오른쪽 드래그 픽셀당 라디안
// 휠로 조절하는 건 카메라 거리가 아니라 "화면에 그려지는 크기"(0=제일 작게,
// 1=CRT 화면 가득) 그 자체다 — 최소/최대 둘 다 같은 4:3 비율(WIN_W:WIN_H 와
// 똑같음)이라 커지고 작아져도 아이템이 찌그러지지 않는다.
const INSPECT_ZOOM_SENS: f32 = 0.05; // 휠 한 칸(y=1.0)당 0~1 배율 변화 — 이전(0.15)보다 완만하게
const INSPECT_SIZE_MIN_W: f32 = 200.0;
const INSPECT_SIZE_MIN_H: f32 = 150.0;
const INSPECT_SIZE_MAX_W: f32 = WIN_W; // 제한을 CRT 화면 크기까지 — 다 키우면 화면을 가득 채운다
const INSPECT_SIZE_MAX_H: f32 = WIN_H;
const DIM_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6]; // 확대 창 떠 있을 때 화면 전체를 덮는 어둠

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// 바닥 높낮이(경사로 → 발판)와 기울어진 벽을 한 장면에 같이 두고 확인한다.
fn build_scene() -> Vec<Box3D> {
    let mut boxes = Vec::new();

    // 바닥 — 아주 넓은 평평한 상자 하나.
    boxes.push(Box3D {
        center: [0.0, -0.25, 0.0],
        half: [15.0, 0.25, 15.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: GROUND_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 경사로 — pitch 회전 하나로 "바닥 높이가 서서히 올라가는" 구간을 만든다.
    // 로컬 +Z 축이 pitch 만큼 기울어지면서 월드 +Z 로 갈수록 y 가 올라간다.
    let ramp_start_z: f32 = 3.0;
    let ramp_run: f32 = 6.0; // 이 구간의 세계-Z 방향 길이
    let ramp_rise: f32 = 1.6; // 이 구간에서 올라가는 높이
    let ramp_angle = ramp_rise.atan2(ramp_run);
    let ramp_half_len = (ramp_run * ramp_run + ramp_rise * ramp_rise).sqrt() / 2.0;
    boxes.push(Box3D {
        center: [0.0, ramp_rise / 2.0, ramp_start_z + ramp_run / 2.0],
        half: [2.2, 0.15, ramp_half_len],
        yaw: 0.0,
        pitch: -ramp_angle,
        roll: 0.0,
        color: RAMP_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 경사로 꼭대기로 이어지는 높은 발판.
    boxes.push(Box3D {
        center: [0.0, ramp_rise - 0.25, ramp_start_z + ramp_run + 3.0],
        half: [3.0, 0.25, 3.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: PLATFORM_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 발판 가장자리에 낮은 난간 벽(평범한 축정렬 벽 — 비교용).
    boxes.push(Box3D {
        center: [3.0, ramp_rise + 0.5, ramp_start_z + ramp_run + 3.0],
        half: [0.15, 0.75, 3.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: WALL_COLOR,
        texture: None,
        walkable: false,
        solid: true,
    });

    // 옆으로 기운 벽 — roll 회전만으로 "기울어진 벽"이 그대로 나온다는 걸 보여준다.
    // walkable=false 라 그 위로 올라타지지 않고 순수하게 막는 장애물로만 동작한다.
    boxes.push(Box3D {
        center: [-4.5, 1.2, -3.0],
        half: [2.2, 1.6, 0.15],
        yaw: 0.4,
        pitch: 0.0,
        roll: 0.3,
        color: LEANING_WALL_COLOR,
        texture: None,
        walkable: false,
        solid: true,
    });

    boxes.extend(build_room());
    boxes
}

// 바닥에 놓인 작은 조사(inspect) 가능 아이템 — 새 에셋/메시 타입 없이 지금
// 있는 Box3D 하나로만 표현한다(이 프로젝트가 지금까지 만들어온 디자인 그대로).
struct Item {
    name: &'static str,
    pos: [f32; 3],
    half: [f32; 3],
    color: [f32; 4],
    yaw: f32,
    pitch: f32,
    roll: f32,
    texture: Option<BoxTexture>, // 텍스처 테스트용 — Note 아이템에만 채워 넣는다
}

fn build_items() -> Vec<Item> {
    vec![
        Item { name: "Key", pos: [-1.2, 0.06, -2.3], half: [0.18, 0.06, 0.06], color: [0.85, 0.7, 0.2, 1.0], yaw: 0.4, pitch: 0.0, roll: 0.0, texture: None },
        Item { name: "Note", pos: [-1.8, 0.015, -1.8], half: [0.14, 0.015, 0.18], color: [0.9, 0.88, 0.75, 1.0], yaw: 0.2, pitch: 0.0, roll: 0.0, texture: None },
        Item { name: "Flashlight", pos: [-2.2, 0.05, -2.6], half: [0.05, 0.05, 0.22], color: [0.3, 0.3, 0.33, 1.0], yaw: -0.5, pitch: 0.0, roll: 0.0, texture: None },
    ]
}

// assets/icon_folder.png(32x32 아이콘) 하나를 BoxTexture 가 기대하는 4x3
// 전개도(가로 4칸×세로 3칸, mesh3d.rs::BoxTexture::uv_for 참고) 형태로 복제해
// 채운 텍스처를 만든다 — 어느 면을 보든 같은 아이콘이 그대로 보이게. 로드
// 실패하면 조용히 None(무늬 없이 color 만 쓴다).
fn load_note_texture(ctx: &mut dyn RenderingBackend, path: &str) -> Option<BoxTexture> {
    let icon = image::open(path).ok()?.to_rgba8();
    let (iw, ih) = (icon.width(), icon.height());
    let (net_w, net_h) = (iw * 4, ih * 3);
    let mut net = image::RgbaImage::new(net_w, net_h);
    for row in 0..3 {
        for col in 0..4 {
            for y in 0..ih {
                for x in 0..iw {
                    net.put_pixel(col * iw + x, row * ih + y, *icon.get_pixel(x, y));
                }
            }
        }
    }
    let texture = ctx.new_texture_from_rgba8(net_w as u16, net_h as u16, &net);
    Some(BoxTexture { texture })
}

// 월드에 놓인 모습 그대로의 Box3D. 텍스처가 있으면 tint 색은 흰색으로 둬서
// 아이콘 원본 색이 그대로 보이게 한다.
fn item_world_box(item: &Item) -> Box3D {
    let color = if item.texture.is_some() { [1.0, 1.0, 1.0, 1.0] } else { item.color };
    Box3D { center: item.pos, half: item.half, yaw: item.yaw, pitch: item.pitch, roll: item.roll, color, texture: item.texture, walkable: false, solid: false }
}

// 확대 창 안에서 원점에 두고 보여줄 Box3D — extra_yaw/pitch 는 오른쪽 드래그로
// 사용자가 더한 회전(아이템 자체의 기본 방향에 얹는다).
fn item_inspect_box(item: &Item, extra_yaw: f32, extra_pitch: f32) -> Box3D {
    let color = if item.texture.is_some() { [1.0, 1.0, 1.0, 1.0] } else { item.color };
    Box3D {
        center: [0.0, 0.0, 0.0],
        half: item.half,
        yaw: item.yaw + extra_yaw,
        pitch: item.pitch + extra_pitch,
        roll: item.roll,
        color,
        texture: item.texture,
        walkable: false,
        solid: false,
    }
}

// 아이템 크기에 맞춰 확대 창 카메라를 자동으로 물러나 둔다 — 작은 열쇠든 큰
// 손전등이든 창 안에 비슷하게 꽉 차 보이게. 이 거리는 고정이고, 휠로 조절하는
// "크기"는 대신 화면에 그리는 사각형 자체의 크기를 바꾼다(draw() 참고).
fn inspect_camera(item: &Item) -> Camera {
    let radius = item.half[0].max(item.half[1]).max(item.half[2]);
    let dist = (radius * 4.5).max(0.6);
    Camera { pos: [0.0, 0.0, -dist], yaw: std::f32::consts::PI, pitch: 0.0 }
}

// 월드 좌표 → 이 창의 화면 좌표(0,0 이 왼쪽 위). 카메라 뒤에 있으면 None.
// map_editor.rs::Stage::world_to_screen 과 같은 계산이다.
fn world_to_screen(cam: &Camera, p: [f32; 3]) -> Option<(f32, f32)> {
    let rel = v_sub(p, cam.pos);
    let (fwd, right, up) = (cam.forward(), cam.right(), cam.up());
    let depth = v_dot(rel, fwd);
    if depth <= 0.05 {
        return None;
    }
    let half_h = (FOV_Y / 2.0).tan();
    let half_w = half_h * (WIN_W / WIN_H);
    let ndc_x = v_dot(rel, right) / (depth * half_w);
    let ndc_y = v_dot(rel, up) / (depth * half_h);
    Some(((ndc_x * 0.5 + 0.5) * WIN_W, (1.0 - (ndc_y * 0.5 + 0.5)) * WIN_H))
}

// 화면 중앙 조준선 기준으로 거리 안 + 각도 안이면 Some(거리) — 문 손잡이처럼
// 딱 하나뿐인 단일 지점 조준 판정에 쓴다.
fn aim_check(cam: &Camera, target: [f32; 3]) -> Option<f32> {
    let to = v_sub(target, cam.pos);
    let dist = crackhead::mesh3d::v_len(to);
    if !(0.05..=AIM_MAX_DIST).contains(&dist) {
        return None;
    }
    let dir = crackhead::mesh3d::v_scale(to, 1.0 / dist);
    if v_dot(dir, cam.forward()) > AIM_MAX_COS { Some(dist) } else { None }
}

// 화면 중앙 조준선 기준으로 가장 가까운(각도·거리 조건을 만족하는) 아이템과
// 그 거리 — 문 손잡이 조준과 우선순위(더 가까운 쪽)를 비교하는 데 거리가 필요.
fn find_aimed_item(cam: &Camera, items: &[Item]) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, item) in items.iter().enumerate() {
        if let Some(dist) = aim_check(cam, item.pos)
            && best.is_none_or(|(_, bd)| dist < bd)
        {
            best = Some((i, dist));
        }
    }
    best
}

struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
    grounded: bool,
}

impl Player {
    fn camera(&self) -> Camera {
        Camera { pos: [self.feet[0], self.feet[1] + EYE_OFFSET, self.feet[2]], yaw: self.yaw, pitch: self.pitch }
    }

    // 마우스(raw_mouse_motion)든 키보드(A/D, ↑/↓)든 시점 회전은 전부 여기 하나로
    // 모아서 처리한다 — forward_flat()=[-sin(yaw),0,-cos(yaw)] 기준으로, yaw 를
    // 줄이는 쪽이 화면상 오른쪽으로 도는 것이다.
    fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw -= dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    fn update(&mut self, input: &Input, dt: f32, boxes: &[Box3D]) {
        if input.is_down(KeyCode::Up) {
            self.look(0.0, LOOK_SPEED * dt);
        }
        if input.is_down(KeyCode::Down) {
            self.look(0.0, -LOOK_SPEED * dt);
        }

        let cam = self.camera();
        let fwd = cam.forward_flat();
        let right = cam.right_flat();
        let mut move_dir = [0.0f32, 0.0, 0.0];
        if input.is_down(KeyCode::W) {
            move_dir = crackhead::mesh3d::v_add(move_dir, fwd);
        }
        if input.is_down(KeyCode::S) {
            move_dir = crackhead::mesh3d::v_sub(move_dir, fwd);
        }
        // A/D 는 이제 시점 회전이 아니라 좌우 이동(strafe) — 마우스가 시점 회전을
        // 맡게 된 뒤로 키보드 회전은 ↑/↓(피치)만 남기고 A/D 는 옆으로 걷는 데 쓴다.
        if input.is_down(KeyCode::A) {
            move_dir = crackhead::mesh3d::v_sub(move_dir, right);
        }
        if input.is_down(KeyCode::D) {
            move_dir = crackhead::mesh3d::v_add(move_dir, right);
        }
        let len = crackhead::mesh3d::v_len(move_dir);
        if len > 1e-4 {
            let step = crackhead::mesh3d::v_scale(move_dir, MOVE_SPEED * dt / len);
            self.feet[0] += step[0];
            self.feet[2] += step[2];
        }

        if input.pressed(KeyCode::Space) && self.grounded {
            self.vel_y = JUMP_SPEED;
            self.grounded = false;
        }

        let pushed = resolve_horizontal(self.feet, PLAYER_RADIUS, self.feet[1], PLAYER_HEIGHT, boxes);
        self.feet[0] = pushed[0];
        self.feet[2] = pushed[2];

        self.vel_y += GRAVITY * dt;
        let predicted_y = self.feet[1] + self.vel_y * dt;
        match ground_height(self.feet[0], self.feet[2], self.feet[1], 60.0, boxes) {
            Some(g) if predicted_y <= g && self.vel_y <= 0.0 => {
                self.feet[1] = g;
                self.vel_y = 0.0;
                self.grounded = true;
            }
            _ => {
                self.feet[1] = predicted_y;
                self.grounded = false;
            }
        }
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    mesh3d: Mesh3D,
    inspect_mesh3d: Mesh3D, // 아이템 확대 창 전용 — 작은 별도 오프스크린 타깃
    crt: Crt,
    boxes: Vec<Box3D>,
    items: Vec<Item>,
    player: Player,
    aimed_item: Option<usize>,
    door_open: bool,  // 목표 상태(열림/닫힘) — 실제 각도는 door_anim 이 서서히 따라간다
    door_anim: f32,   // 0=닫힘 ~ 1=열림
    door_aimed: bool, // 지금 조준선이 문 손잡이를 향하고 있는지
    dialogue_lines: Vec<String>, // 지금 띄우는 대화의 전체 줄 목록(1줄이어도 그냥 이거)
    dialogue_index: usize,       // 몇 번째 줄을 보여주는 중인지 — len 이면 대화 끝(안 뜸)
    dialogue_visible_chars: usize, // 그 줄에서 지금까지 타자기로 드러낸 글자 수
    dialogue_next_char_at: f64,    // 다음 글자를 드러낼 시각(date::now() 기준)
    dialogue_rng: u64,             // 글자 간격을 흔드는 xorshift64 시드
    inspecting: Option<usize>,
    item_view_yaw: f32,
    item_view_pitch: f32,
    inspect_zoom: f32,
    rmb_down: bool,
    input: Input,
    start_time: f64,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mesh3d = Mesh3D::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        // CRT 화면과 같은 해상도로 만들어둔다 — 휠로 화면 가득 키워도 흐려지지 않게.
        let inspect_mesh3d = Mesh3D::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        let crt = Crt::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        let boxes = build_scene();
        let mut items = build_items();
        // 텍스처 테스트 — Note 아이템에 폴더 아이콘(assets/icon_folder.png)을 입혀본다.
        if let Some(note) = items.iter_mut().find(|it| it.name == "Note") {
            note.texture = load_note_texture(ctx.as_mut(), "assets/icon_folder.png");
        }
        let player = Player { feet: [0.0, 0.0, -3.0], yaw: std::f32::consts::FRAC_PI_2, pitch: 0.0, vel_y: 0.0, grounded: true };
        let now = date::now();
        // FPS 식 마우스룩 — 커서를 숨기고 창 안에 가둔다. raw_mouse_motion 은 이
        // 설정과 무관하게 항상 들어오지만(레지스터만 해두면 OS 가 계속 보내준다),
        // 커서를 숨기고 가둬야 실제로 화면 밖으로 안 새어나가고 자연스럽다.
        window::show_mouse(false);
        window::set_cursor_grab(true);
        recenter_cursor();
        Stage {
            ctx,
            renderer,
            mesh3d,
            inspect_mesh3d,
            crt,
            boxes,
            items,
            player,
            aimed_item: None,
            door_open: false,
            door_anim: 0.0,
            door_aimed: false,
            dialogue_lines: Vec::new(),
            dialogue_index: 0,
            dialogue_visible_chars: 0,
            dialogue_next_char_at: 0.0,
            dialogue_rng: 0x9E3779B97F4A7C15, // 아무 고정값(황금비 기반 상수) — 그냥 시작 시드
            inspecting: None,
            item_view_yaw: 0.0,
            item_view_pitch: 0.0,
            inspect_zoom: 0.3,
            rmb_down: false,
            input: Input::default(),
            start_time: now,
            last_time: now,
        }
    }

    fn close_inspect(&mut self) {
        self.inspecting = None;
        self.item_view_yaw = 0.0;
        self.item_view_pitch = 0.0;
        self.inspect_zoom = 0.3;
    }

    fn dialogue_active(&self) -> bool {
        self.dialogue_index < self.dialogue_lines.len()
    }

    // lines 를 새 대화로 시작한다(지금 보던 대화가 있었으면 덮어쓴다).
    fn start_dialogue(&mut self, lines: Vec<String>) {
        self.dialogue_lines = lines;
        self.dialogue_index = 0;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_at = date::now();
    }

    // "아무 입력"에 대응 — 아직 타이핑 중이면 그 줄을 즉시 다 보여주고, 이미
    // 다 보여준 줄이면 다음 줄로(더 없으면 대화 자체가 끝나 dialogue_active()
    // 가 false 가 된다).
    fn advance_dialogue(&mut self) {
        if !self.dialogue_active() {
            return;
        }
        let full_len = self.dialogue_lines[self.dialogue_index].chars().count();
        if self.dialogue_visible_chars < full_len {
            self.dialogue_visible_chars = full_len;
        } else {
            self.dialogue_index += 1;
            self.dialogue_visible_chars = 0;
            self.dialogue_next_char_at = date::now();
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        // 대화창 타자기 효과 — 시간이 됐으면 한 글자씩 드러낸다(느려진 프레임
        // 뒤에 한 번에 여러 칸 밀려도 되게 while 로 따라잡는다).
        if self.dialogue_active() {
            let full_len = self.dialogue_lines[self.dialogue_index].chars().count();
            while self.dialogue_visible_chars < full_len && now >= self.dialogue_next_char_at {
                self.dialogue_visible_chars += 1;
                let delay = DIALOGUE_CHAR_DELAY_MIN + rand01(&mut self.dialogue_rng) as f64 * (DIALOGUE_CHAR_DELAY_MAX - DIALOGUE_CHAR_DELAY_MIN);
                self.dialogue_next_char_at = now + delay;
            }
        }

        // 문은 목표 상태(door_open)를 향해 서서히 회전한다 — 조사 창이 떠 있어도
        // 계속 진행시켜서(플레이어가 안 보고 있어도) 어색하게 멈춰있지 않게 한다.
        let door_target = if self.door_open { 1.0 } else { 0.0 };
        self.door_anim = move_toward(self.door_anim, door_target, DOOR_ANIM_SPEED * dt);

        // 대화창이 떠 있는 동안은 이동/시점 회전/조준 갱신을 전부 멈춘다.
        if self.inspecting.is_none() && !self.dialogue_active() {
            // 문짝(지금 각도)도 같이 충돌 목록에 넣는다 — 닫혀 있으면 진짜로 막는다.
            let mut collision_boxes = self.boxes.clone();
            collision_boxes.push(door_box(self.door_anim));
            self.player.update(&self.input, dt, &collision_boxes);

            let cam = self.player.camera();
            let item_hit = find_aimed_item(&cam, &self.items);
            let handle_hit = aim_check(&cam, door_handle_pos(self.door_anim));
            match (item_hit, handle_hit) {
                (Some((i, idist)), Some(hdist)) if idist <= hdist => {
                    self.aimed_item = Some(i);
                    self.door_aimed = false;
                }
                (_, Some(_)) => {
                    self.aimed_item = None;
                    self.door_aimed = true;
                }
                (Some((i, _)), None) => {
                    self.aimed_item = Some(i);
                    self.door_aimed = false;
                }
                (None, None) => {
                    self.aimed_item = None;
                    self.door_aimed = false;
                }
            }
        }

        let mut world_boxes = self.boxes.clone();
        world_boxes.extend(self.items.iter().map(item_world_box));
        world_boxes.push(door_box(self.door_anim));
        world_boxes.push(door_handle_box(self.door_anim));
        self.mesh3d.render(self.ctx.as_mut(), SKY_COLOR, &self.player.camera(), &world_boxes, FOV_Y);

        self.renderer.begin(WIN_W, WIN_H);
        // mesh3d 오프스크린 결과를 창 전체에 스프라이트로 끼워 넣는다 — v 를 뒤집는 건
        // crt.rs 의 오프스크린 합성과 같은 이유(오프스크린 텍스처는 위아래가 뒤집혀 있다).
        let tex = self.mesh3d.color_texture();
        self.renderer.sprite_uv(tex, 0.0, 0.0, WIN_W, WIN_H, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        // 화면 중앙 조준선(작은 십자).
        let (cx, cy) = (WIN_W / 2.0, WIN_H / 2.0);
        self.renderer.rect(cx - 5.0, cy - 1.0, 10.0, 2.0, [1.0, 1.0, 1.0, 0.8]);
        self.renderer.rect(cx - 1.0, cy - 5.0, 2.0, 10.0, [1.0, 1.0, 1.0, 0.8]);

        self.renderer.rect(0.0, 0.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.55]);
        self.renderer.text(6.0, 3.0, "mesh3d.rs test - mouse/WASD move, Space jump, E interact", 0.7, [1.0, 1.0, 1.0, 1.0]);
        let status = format!(
            "pos=({:.1},{:.1},{:.1}) grounded={}",
            self.player.feet[0], self.player.feet[1], self.player.feet[2], self.player.grounded
        );
        self.renderer.text(6.0, WIN_H - 16.0, &status, 0.7, [1.0, 1.0, 0.6, 1.0]);

        // 조준 중인 아이템이 있으면(확대 창이 안 떠 있을 때만) 그 옆에 안내 문구.
        if self.inspecting.is_none()
            && let Some(i) = self.aimed_item
            && let Some((sx, sy)) = world_to_screen(&self.player.camera(), self.items[i].pos)
        {
            let label = format!("[E] Inspect {}", self.items[i].name);
            let tw = self.renderer.text_width(&label, 0.7);
            self.renderer.rect(sx + 10.0, sy - 10.0, tw + 8.0, 16.0, [0.0, 0.0, 0.0, 0.6]);
            self.renderer.text(sx + 14.0, sy - 8.0, &label, 0.7, [1.0, 1.0, 0.4, 1.0]);
        }

        // 문 손잡이를 조준 중이면 그 옆에 상호작용 안내 문구.
        if self.inspecting.is_none()
            && self.door_aimed
            && let Some((sx, sy)) = world_to_screen(&self.player.camera(), door_handle_pos(self.door_anim))
        {
            let label = "[E] Examine";
            let tw = self.renderer.text_width(label, 0.7);
            self.renderer.rect(sx + 10.0, sy - 10.0, tw + 8.0, 16.0, [0.0, 0.0, 0.0, 0.6]);
            self.renderer.text(sx + 14.0, sy - 8.0, label, 0.7, [0.6, 0.9, 1.0, 1.0]);
        }

        // 아이템 확대 보기 — 박스형 창 대신 화면 전체를 어둡게 깔고 그 위에
        // 아이템만 또렷하게 띄운다. 휠(inspect_zoom, 0~1)은 화면에 그리는
        // 사각형 자체의 크기를 최소 크기 ~ CRT 화면 전체 사이로 조절한다.
        if let Some(i) = self.inspecting {
            self.renderer.rect(0.0, 0.0, WIN_W, WIN_H, DIM_COLOR);

            let item = &self.items[i];
            let inspect_box = item_inspect_box(item, self.item_view_yaw, self.item_view_pitch);
            self.inspect_mesh3d.render(self.ctx.as_mut(), [0.0, 0.0, 0.0, 0.0], &inspect_camera(item), std::slice::from_ref(&inspect_box), FOV_Y);

            let t = self.inspect_zoom;
            let w = lerp(INSPECT_SIZE_MIN_W, INSPECT_SIZE_MAX_W, t);
            let h = lerp(INSPECT_SIZE_MIN_H, INSPECT_SIZE_MAX_H, t);
            let x = (WIN_W - w) / 2.0;
            let y = (WIN_H - h) / 2.0;
            let inspect_tex = self.inspect_mesh3d.color_texture();
            self.renderer.sprite_uv(inspect_tex, x, y, w, h, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

            // 문구는 박스 크기와 무관하게 화면 위/아래 고정 위치에 — 다 키워도 안 가려지게.
            self.renderer.text(10.0, 22.0, item.name, 0.75, [1.0, 1.0, 1.0, 1.0]);
            self.renderer.text(10.0, WIN_H - 14.0, "RMB drag: rotate  |  wheel: zoom  |  E/Esc: close", 0.6, [0.85, 0.85, 0.9, 1.0]);
        }

        // 화면 아래쪽 대화창 — 손잡이 같은 걸 조사했을 때 나오는 짧은 문구.
        // 무엇보다 위(맨 마지막에 그림)에 뜬다. 좌우/아래 여백을 두고 화면
        // 맨 밑에서 좀 띄워서(DIALOGUE_BOTTOM_MARGIN) 그린다.
        if self.dialogue_active() {
            let line = &self.dialogue_lines[self.dialogue_index];
            let shown: String = line.chars().take(self.dialogue_visible_chars).collect();
            let box_x = DIALOGUE_SIDE_MARGIN;
            let box_w = WIN_W - DIALOGUE_SIDE_MARGIN * 2.0;
            let box_y = WIN_H - DIALOGUE_BOTTOM_MARGIN - DIALOGUE_HEIGHT;
            self.renderer.rect(box_x, box_y, box_w, DIALOGUE_HEIGHT, [0.0, 0.0, 0.0, 0.82]);
            self.renderer.rect(box_x, box_y, box_w, 2.0, [0.6, 0.6, 0.65, 0.9]);
            self.renderer.text(box_x + 16.0, box_y + 18.0, &shown, DIALOGUE_TEXT_SCALE, [1.0, 1.0, 1.0, 1.0]);
            // 다 타이핑됐으면 깜빡이는 화살표로 "아무 키나 눌러 계속" 신호를 준다.
            if self.dialogue_visible_chars >= line.chars().count() && (now * 2.2).sin() > 0.0 {
                self.renderer.text(box_x + box_w - 22.0, box_y + DIALOGUE_HEIGHT - 20.0, "v", DIALOGUE_TEXT_SCALE, [0.8, 0.8, 0.85, 1.0]);
            }
        }

        // main.rs::draw() 와 같은 순서: 2D 그리기 목록은 이미 위에서 renderer 에
        // 쌓아뒀고, crt.begin() 이 그 flush 의 대상을 CRT용 오프스크린 타깃으로
        // 돌려놓은 뒤에야 실제로 흘려보낸다 — 그래야 CRT 셰이더가 이 화면
        // 전체(3D 뷰 + HUD + 확대 창) 를 한 장의 텍스처로 받아 곡률/색수차를
        // 입힐 수 있다.
        self.crt.begin(self.ctx.as_mut());
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();

        let elapsed = (now - self.start_time) as f32;
        self.crt.present(self.ctx.as_mut(), elapsed, CHROMATIC_ABERRATION, CRT_INTENSITY);
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        // 대화창이 떠 있으면 "아무 입력"을 최우선으로 먹는다 — 다른 단축키는
        // 전부 무시하고 대화만 진행시킨다(타이핑 중이면 그 줄 다 보여주기,
        // 다 보여줬으면 다음 줄/닫기).
        if !repeat && self.dialogue_active() {
            self.advance_dialogue();
            self.input.on_key_down(keycode, repeat);
            return;
        }
        if !repeat && keycode == KeyCode::E {
            if self.inspecting.is_some() {
                self.close_inspect();
            } else if self.door_aimed {
                // 일단은 문을 실제로 열지 않고 대화 문구만 띄운다(door_open 은
                // 나중에 다른 계기로 열리게 될 걸 대비해 그대로 남겨둔다).
                self.start_dialogue(vec![DOORKNOB_MESSAGE.to_string()]);
            } else if let Some(i) = self.aimed_item {
                self.inspecting = Some(i);
                self.item_view_yaw = 0.0;
                self.item_view_pitch = 0.0;
                self.inspect_zoom = 0.3;
            }
        }
        // Esc 로 이 창 전체를 끄는 단축키는 뺐다 — 확대 창을 닫는 용도로만 쓴다.
        if keycode == KeyCode::Escape && self.inspecting.is_some() {
            self.close_inspect();
        }
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.input.on_key_up(keycode);
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if self.dialogue_active() {
            self.advance_dialogue();
            return;
        }
        if button == MouseButton::Right {
            self.rmb_down = true;
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Right {
            self.rmb_down = false;
        }
    }

    // 창을 다시 포커싱해서 커서가 창 안으로 들어올 때(알트탭 복귀 등)도
    // 곧바로 에임 포인트(화면 중앙)로 되돌려둔다.
    fn mouse_enter_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {
        recenter_cursor();
    }

    // 확대 창이 떠 있는 동안 휠로 보고 있는 아이템의 크기(카메라 거리)를 조정한다.
    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if self.inspecting.is_some() {
            self.inspect_zoom = (self.inspect_zoom + y * INSPECT_ZOOM_SENS).clamp(0.0, 1.0);
        }
    }

    // 마우스 이동 — `raw_mouse_motion`(WM_INPUT 원시 입력) 대신 평범한
    // `mouse_motion_event`(절대 좌표, WM_MOUSEMOVE)를 쓴다. raw_mouse_motion 은
    // 일부 컴퓨터(마우스/터치패드 드라이버, 가상 머신 등)에서 아예 안 들어오는
    // 경우가 있었다 — WM_MOUSEMOVE 는 그런 환경에서도 항상 들어온다. 대신
    // 커서를 "에임 포인트"(화면 정중앙, 조준선 위치)에 계속 고정해두고, 그
    // 중심에서 벗어난 만큼만 회전에 반영한 뒤 다시 중심으로 되돌린다(고전적인
    // FPS 마우스룩 방식) — recenter_cursor() 참고.
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let (cx, cy) = (WIN_W / 2.0, WIN_H / 2.0);
        let (dx, dy) = (x - cx, y - cy);
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return;
        }
        if self.inspecting.is_some() {
            if self.rmb_down {
                self.item_view_yaw += dx * ITEM_ROTATE_SENS;
                self.item_view_pitch = (self.item_view_pitch - dy * ITEM_ROTATE_SENS).clamp(-MAX_PITCH, MAX_PITCH);
            }
        } else if !self.dialogue_active() {
            // 대화창이 떠 있는 동안은 화면(시점)이 돌아가지 않는다 — 그래도
            // 커서는 계속 중앙으로 되돌려서, 대화가 끝난 뒤 갑자기 큰 폭으로
            // 튀어 돌아가지 않게 한다.
            self.player.look(dx * MOUSE_SENS, -dy * MOUSE_SENS);
        }
        recenter_cursor();
    }
}

// 마우스 커서를 이 창의 클라이언트 좌표 정중앙(화면 조준선이 있는 자리)으로
// 강제로 되돌린다. miniquad 0.4 에는 커서 위치를 직접 지정하는 API가 없어서
// (set_cursor_grab 은 그냥 화면 밖으로 못 나가게 "가두기"만 한다) Win32 를
// 직접 호출한다 — 이 프로젝트는 어차피 Windows 전용(WASAPI/MediaFoundation
// 사용)이라 문제 없다.
fn recenter_cursor() {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
    unsafe {
        let hwnd = GetActiveWindow();
        if hwnd.is_invalid() {
            return;
        }
        let mut pt = POINT { x: (WIN_W / 2.0) as i32, y: (WIN_H / 2.0) as i32 };
        if ClientToScreen(hwnd, &mut pt).as_bool() {
            let _ = SetCursorPos(pt.x, pt.y);
        }
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "Mesh3D Test".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
