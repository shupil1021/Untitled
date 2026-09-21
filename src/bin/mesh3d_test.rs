// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::mesh3d`(진짜 3D 메쉬 렌더러)만 따로 띄워서 확인하는 테스트 창.
//! `cargo run --bin mesh3d_test` 로 띄운다.
//!
//! 장면은 전부 `Box3D`(회전 가능한 직육면체) 하나로만 만든다: 평평한 바닥,
//! 기울어진 경사로(램프, pitch 회전), 그 위 높은 발판, 옆으로 기운 벽(roll 회전)
//! — 바닥 높낮이도 기울어진 벽/경사로도 전부 같은 상자 타입 하나로 표현된다는 걸
//! 보여주는 게 목적이다. 바닥엔 작은 아이템(열쇠/쪽지/손전등, 전부 이 프로젝트의
//! 기존 디자인 그대로 `Box3D` 하나로 표현) 몇 개를 흩어놨다.
//!
//! 조작: W/S 전진/후진, A/D 좌우 이동(strafe), Space 로 점프. **마우스를
//! 움직이면 시점이 돈다**(FPS 게임처럼 커서를 숨기고 창에 가둔다 —
//! `raw_mouse_motion`, OS 커서 가속/클램프의 영향을 안 받는 원시 입력이라
//! 회전이 매끄럽다). ↑/↓ 로도 피치를 돌릴 수 있다(키보드만으로도 확인할 수
//! 있게 남겨뒀다).
//!
//! 아이템에 조준선(화면 중앙)을 가까이 대면 그 옆에 "[E] Inspect 이름"이 뜬다 —
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
use crackhead::mesh3d::{ground_height, resolve_horizontal, v_dot, v_sub, Box3D, Camera, Mesh3D};
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
}

fn build_items() -> Vec<Item> {
    vec![
        Item { name: "Key", pos: [-1.2, 0.06, -2.3], half: [0.18, 0.06, 0.06], color: [0.85, 0.7, 0.2, 1.0], yaw: 0.4, pitch: 0.0, roll: 0.0 },
        Item { name: "Note", pos: [-1.8, 0.015, -1.8], half: [0.14, 0.015, 0.18], color: [0.9, 0.88, 0.75, 1.0], yaw: 0.2, pitch: 0.0, roll: 0.0 },
        Item { name: "Flashlight", pos: [-2.2, 0.05, -2.6], half: [0.05, 0.05, 0.22], color: [0.3, 0.3, 0.33, 1.0], yaw: -0.5, pitch: 0.0, roll: 0.0 },
    ]
}

// 월드에 놓인 모습 그대로의 Box3D.
fn item_world_box(item: &Item) -> Box3D {
    Box3D { center: item.pos, half: item.half, yaw: item.yaw, pitch: item.pitch, roll: item.roll, color: item.color, texture: None, walkable: false, solid: false }
}

// 확대 창 안에서 원점에 두고 보여줄 Box3D — extra_yaw/pitch 는 오른쪽 드래그로
// 사용자가 더한 회전(아이템 자체의 기본 방향에 얹는다).
fn item_inspect_box(item: &Item, extra_yaw: f32, extra_pitch: f32) -> Box3D {
    Box3D {
        center: [0.0, 0.0, 0.0],
        half: item.half,
        yaw: item.yaw + extra_yaw,
        pitch: item.pitch + extra_pitch,
        roll: item.roll,
        color: item.color,
        texture: None,
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

// 화면 중앙 조준선 기준으로 가장 가까운(각도·거리 조건을 만족하는) 아이템.
fn find_aimed_item(cam: &Camera, items: &[Item]) -> Option<usize> {
    let fwd = cam.forward();
    let mut best: Option<(usize, f32)> = None;
    for (i, item) in items.iter().enumerate() {
        let to_item = v_sub(item.pos, cam.pos);
        let dist = crackhead::mesh3d::v_len(to_item);
        if !(0.05..=AIM_MAX_DIST).contains(&dist) {
            continue;
        }
        let dir = crackhead::mesh3d::v_scale(to_item, 1.0 / dist);
        let cos_angle = v_dot(dir, fwd);
        if cos_angle > AIM_MAX_COS && best.is_none_or(|(_, bd)| dist < bd) {
            best = Some((i, dist));
        }
    }
    best.map(|(i, _)| i)
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
        let items = build_items();
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
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        if self.inspecting.is_none() {
            self.player.update(&self.input, dt, &self.boxes);
            self.aimed_item = find_aimed_item(&self.player.camera(), &self.items);
        }

        let mut world_boxes = self.boxes.clone();
        world_boxes.extend(self.items.iter().map(item_world_box));
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
        self.renderer.text(6.0, 3.0, "mesh3d.rs test - mouse/WASD move, Space jump, E inspect", 0.7, [1.0, 1.0, 1.0, 1.0]);
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
        if !repeat && keycode == KeyCode::E {
            if self.inspecting.is_some() {
                self.close_inspect();
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
        } else {
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
