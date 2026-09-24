//! 메인 게임 "DOORS" — 친구가 메일로 보낸 크랙 게임(STORY.md 7-1절). 메일 첨부
//! `DOORS.exe`(FileKind::Game)를 받아서 열면 다른 앱들처럼 PalaceOS 안의 창 하나로
//! 뜬다(별도 실행 파일/OS 창이 아니다). 3D 장면은 `mesh3d.rs` 로 오프스크린에 그린
//! 뒤, 그 결과를 창 안에 4:3 비율로 끼워 넣는다(남는 곳은 검은 띠). CRT 효과는
//! 바깥 OS 화면 전체에 이미 걸려 있어서 따로 안 입힌다.
//!
//! 맵은 지금 가장 단순한 형태 하나: 사방(+바닥/천장)이 막힌 방, 플레이어 정면 벽에
//! 닫힌 문 하나. 그 문이 도어즈(Doors, 말하는 문 NPC)다 — 손잡이를 조준하고 `E`를
//! 누르면 보리지꽃을 달라고 하고 `[Y] 수락 / [N] 거부` 선택지가 뜬다. 수락하면 꽃
//! 퀘스트 스텁 화면(진짜 미니게임은 아직 없음, `Enter`로 클리어 처리)이 뜬다.
//!
//! 조작: 게임 화면을 클릭하면 마우스 시점 모드(커서가 사라지고 마우스로 시점이
//! 돈다 — main.rs 가 scenes::request_mouse_look() 요청을 받아 처리), `Esc` 나 다른
//! 창으로 포커스를 옮기면 풀린다. W/S/A/D 이동, E 상호작용(창이 포커스일 때만).

use std::cell::RefCell;

use miniquad::{KeyCode, RenderingBackend};

use crate::gfx::{Assets, Rect, Renderer, CELL_H};
use crate::mesh3d::{ground_height, resolve_horizontal, v_add, v_dot, v_len, v_scale, v_sub, Box3D, Camera, Mesh3D};
use crate::scenes::Input;

use super::{App, AppAction, WinInput};

// 3D 오프스크린 해상도 — 이 크기(4:3) 기준으로 HUD 좌표도 잡고, 실제 창 크기에
// 맞춰 통째로 배율(view.w / VIEW_W)을 곱해서 그린다.
const VIEW_W: f32 = 640.0;
const VIEW_H: f32 = 480.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;

const MOVE_SPEED: f32 = 2.6;
const MOUSE_SENS: f32 = 0.0032; // 실제 화면 픽셀당 라디안
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55;
const GRAVITY: f32 = -12.0;

const CLEAR_COLOR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];
const FLOOR_COLOR: [f32; 4] = [0.28, 0.26, 0.24, 1.0];
const CEILING_COLOR: [f32; 4] = [0.33, 0.33, 0.32, 1.0];
const WALL_COLOR: [f32; 4] = [0.45, 0.43, 0.4, 1.0];

// 방 — 문은 앞쪽(-Z) 벽 가운데. 플레이어는 뒤쪽(+Z)에서 문을 바라보고 시작한다
// (yaw=0 이면 정면이 -Z, mesh3d.rs::Camera::forward_flat 참고).
const ROOM_MIN_X: f32 = -3.0;
const ROOM_MAX_X: f32 = 3.0;
const ROOM_MIN_Z: f32 = -3.5; // 문이 있는 벽
const ROOM_MAX_Z: f32 = 2.5;
const ROOM_HEIGHT: f32 = 2.6;
const WALL_THICK: f32 = 0.15;
const SPAWN: [f32; 3] = [0.0, 0.0, 1.2];

// 문 — 앞쪽 벽의 [DOOR_MIN_X, DOOR_MIN_X+DOOR_WIDTH] 틈을 정확히 채운다.
const DOOR_WIDTH: f32 = 1.5;
const DOOR_MIN_X: f32 = -DOOR_WIDTH / 2.0;
const DOOR_HALF_H: f32 = 1.15;
const DOOR_HALF_T: f32 = 0.06;
const DOOR_COLOR: [f32; 4] = [0.35, 0.24, 0.16, 1.0];
const HANDLE_HALF: [f32; 3] = [0.05, 0.05, 0.05];
const HANDLE_COLOR: [f32; 4] = [0.8, 0.72, 0.45, 1.0];
// 손잡이 — 문의 오른쪽 가장자리 근처, 방 안쪽(+Z) 면에서 살짝 튀어나온 자리.
const HANDLE_POS: [f32; 3] = [DOOR_MIN_X + DOOR_WIDTH - 0.15, DOOR_HALF_H - 0.45, ROOM_MIN_Z + 0.09];

const AIM_MAX_DIST: f32 = 3.0;
const AIM_MAX_COS: f32 = 0.95;

// 대화창 — 전부 VIEW_W x VIEW_H 기준 값(그릴 때 배율을 곱한다).
const DIALOGUE_SIDE_MARGIN: f32 = 24.0;
const DIALOGUE_BOTTOM_MARGIN: f32 = 36.0;
const DIALOGUE_HEIGHT: f32 = VIEW_H / 3.0;
const DIALOGUE_TEXT_SCALE: f32 = 1.4;
const DIALOGUE_LINE_H: f32 = CELL_H * DIALOGUE_TEXT_SCALE;
const DIALOGUE_CHAR_DELAY_MIN: f32 = 0.02;
const DIALOGUE_CHAR_DELAY_MAX: f32 = 0.09;
const DIM_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

// Mesh3D(오프스크린 타깃 + 셰이더/파이프라인)는 창을 열 때마다 새로 만들면 닫을
// 때 지울 방법이 없어(App 은 Drop 에서 ctx 를 못 받는다) GPU 자원이 계속 쌓인다 —
// 그래서 한 번 만든 걸 스레드 전역에 두고 다시 여는 창들이 재사용한다(이 게임
// 창은 한 번에 하나만 열린다 — desktop.rs 가 같은 파일의 창을 중복으로 안 연다).
thread_local! {
    static MESH3D: RefCell<Option<Mesh3D>> = const { RefCell::new(None) };
}

// 대화 한 줄(Line)과 수락/거부 선택지(Choice).
enum DialogueEntry {
    Line(String),
    Choice { prompt: String, on_accept: DialogueAction },
}

#[derive(Clone, Copy, PartialEq)]
enum DialogueAction {
    None,
    StartFlowerQuest,
}

// 꽃 퀘스트를 수락하면 뜨는 화면 — 진짜 미니게임 대신 자리만 잡아둔 스텁.
#[derive(PartialEq)]
enum GameOverlay {
    None,
    SubMinigameStub,
}

fn rand01(seed: &mut u64) -> f32 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *seed = x;
    (x % 1_000_000) as f32 / 1_000_000.0
}

fn solid(center: [f32; 3], half: [f32; 3], color: [f32; 4], walkable: bool) -> Box3D {
    Box3D { center, half, yaw: 0.0, pitch: 0.0, roll: 0.0, color, texture: None, walkable, solid: true }
}

// 사방이 막힌 방 — 바닥/천장/벽 4면(앞쪽 벽은 문간만 비우고 좌우 조각 + 상인방) + 닫힌 문.
fn build_room() -> Vec<Box3D> {
    let cx = (ROOM_MIN_X + ROOM_MAX_X) / 2.0;
    let cz = (ROOM_MIN_Z + ROOM_MAX_Z) / 2.0;
    let hx = (ROOM_MAX_X - ROOM_MIN_X) / 2.0 + WALL_THICK;
    let hz = (ROOM_MAX_Z - ROOM_MIN_Z) / 2.0 + WALL_THICK;
    let hy = ROOM_HEIGHT / 2.0;

    let mut boxes = vec![
        solid([cx, -0.1, cz], [hx, 0.1, hz], FLOOR_COLOR, true),
        solid([cx, ROOM_HEIGHT + 0.1, cz], [hx, 0.1, hz], CEILING_COLOR, false),
        solid([ROOM_MIN_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([ROOM_MAX_X, hy, cz], [WALL_THICK, hy, hz], WALL_COLOR, false),
        solid([cx, hy, ROOM_MAX_Z], [hx, hy, WALL_THICK], WALL_COLOR, false),
    ];

    let door_max_x = DOOR_MIN_X + DOOR_WIDTH;
    let left_hw = (DOOR_MIN_X - ROOM_MIN_X) / 2.0;
    let right_hw = (ROOM_MAX_X - door_max_x) / 2.0;
    boxes.push(solid([ROOM_MIN_X + left_hw, hy, ROOM_MIN_Z], [left_hw, hy, WALL_THICK], WALL_COLOR, false));
    boxes.push(solid([door_max_x + right_hw, hy, ROOM_MIN_Z], [right_hw, hy, WALL_THICK], WALL_COLOR, false));
    let door_top = DOOR_HALF_H * 2.0;
    let lintel_hy = (ROOM_HEIGHT - door_top) / 2.0;
    boxes.push(solid([0.0, door_top + lintel_hy, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, lintel_hy, WALL_THICK], WALL_COLOR, false));

    boxes.push(solid([DOOR_MIN_X + DOOR_WIDTH / 2.0, DOOR_HALF_H, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, DOOR_HALF_H, DOOR_HALF_T], DOOR_COLOR, false));
    boxes.push(Box3D { center: HANDLE_POS, half: HANDLE_HALF, yaw: 0.0, pitch: 0.0, roll: 0.0, color: HANDLE_COLOR, texture: None, walkable: false, solid: false });
    boxes
}

// 월드 좌표 → VIEW_W x VIEW_H 기준 화면 좌표. 카메라 뒤면 None.
fn world_to_view(cam: &Camera, p: [f32; 3]) -> Option<(f32, f32)> {
    let rel = v_sub(p, cam.pos);
    let depth = v_dot(rel, cam.forward());
    if depth <= 0.05 {
        return None;
    }
    let half_h = (FOV_Y / 2.0).tan();
    let half_w = half_h * (VIEW_W / VIEW_H);
    let ndc_x = v_dot(rel, cam.right()) / (depth * half_w);
    let ndc_y = v_dot(rel, cam.up()) / (depth * half_h);
    Some(((ndc_x * 0.5 + 0.5) * VIEW_W, (1.0 - (ndc_y * 0.5 + 0.5)) * VIEW_H))
}

fn aimed_at(cam: &Camera, target: [f32; 3]) -> bool {
    let to = v_sub(target, cam.pos);
    let dist = v_len(to);
    (0.05..=AIM_MAX_DIST).contains(&dist) && v_dot(v_scale(to, 1.0 / dist), cam.forward()) > AIM_MAX_COS
}

struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
}

impl Player {
    fn camera(&self) -> Camera {
        Camera { pos: [self.feet[0], self.feet[1] + EYE_OFFSET, self.feet[2]], yaw: self.yaw, pitch: self.pitch }
    }

    fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw -= dyaw;
        self.pitch = (self.pitch + dpitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    // can_move 가 false 면(창이 포커스가 아님) 키 입력은 무시하고 중력만 적용한다.
    fn update(&mut self, input: &Input, can_move: bool, dt: f32, boxes: &[Box3D]) {
        if can_move {
            let cam = self.camera();
            let (fwd, right) = (cam.forward_flat(), cam.right_flat());
            let mut dir = [0.0f32; 3];
            if input.is_down(KeyCode::W) {
                dir = v_add(dir, fwd);
            }
            if input.is_down(KeyCode::S) {
                dir = v_sub(dir, fwd);
            }
            if input.is_down(KeyCode::A) {
                dir = v_sub(dir, right);
            }
            if input.is_down(KeyCode::D) {
                dir = v_add(dir, right);
            }
            let len = v_len(dir);
            if len > 1e-4 {
                let step = v_scale(dir, MOVE_SPEED * dt / len);
                self.feet[0] += step[0];
                self.feet[2] += step[2];
            }
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
            }
            _ => self.feet[1] = predicted_y,
        }
    }
}

pub struct DoorsGameApp {
    boxes: Vec<Box3D>,
    player: Player,
    captured: bool, // 마우스 시점 모드인지(게임 화면을 클릭하면 켜지고 Esc/포커스 잃으면 꺼진다)
    door_aimed: bool,
    dialogue_entries: Vec<DialogueEntry>,
    dialogue_index: usize,
    dialogue_visible_chars: usize,
    dialogue_next_char_in: f32, // 다음 글자를 드러내기까지 남은 시간(초)
    dialogue_rng: u64,
    pending_action: DialogueAction,
    doors_fulfilled: bool, // 보리지꽃을 이미 건넸는지 — 그 뒤론 더 말을 걸 수 없다
    flowers: u32,
    overlay: GameOverlay,
}

impl DoorsGameApp {
    pub(super) fn new() -> DoorsGameApp {
        DoorsGameApp {
            boxes: build_room(),
            player: Player { feet: SPAWN, yaw: 0.0, pitch: 0.0, vel_y: 0.0 },
            captured: false,
            door_aimed: false,
            dialogue_entries: Vec::new(),
            dialogue_index: 0,
            dialogue_visible_chars: 0,
            dialogue_next_char_in: 0.0,
            dialogue_rng: 0x9E3779B97F4A7C15,
            pending_action: DialogueAction::None,
            doors_fulfilled: false,
            flowers: 0,
            overlay: GameOverlay::None,
        }
    }

    fn dialogue_active(&self) -> bool {
        self.dialogue_index < self.dialogue_entries.len()
    }

    fn current_dialogue_text(&self) -> &str {
        match self.dialogue_entries.get(self.dialogue_index) {
            Some(DialogueEntry::Line(s)) => s,
            Some(DialogueEntry::Choice { prompt, .. }) => prompt,
            None => "",
        }
    }

    fn dialogue_choice_ready(&self) -> bool {
        match self.dialogue_entries.get(self.dialogue_index) {
            Some(DialogueEntry::Choice { prompt, .. }) => self.dialogue_visible_chars >= prompt.chars().count(),
            _ => false,
        }
    }

    fn start_dialogue(&mut self, entries: Vec<DialogueEntry>) {
        self.dialogue_entries = entries;
        self.dialogue_index = 0;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_in = 0.0;
    }

    // 타이핑 중이면 그 줄을 다 보여주고, 다 보여준 평범한 줄이면 다음으로.
    // 선택지는 Y/N(resolve_choice)으로만 넘어간다.
    fn advance_dialogue(&mut self) {
        let full_len = self.current_dialogue_text().chars().count();
        if self.dialogue_visible_chars < full_len {
            self.dialogue_visible_chars = full_len;
            return;
        }
        if matches!(self.dialogue_entries.get(self.dialogue_index), Some(DialogueEntry::Choice { .. })) {
            return;
        }
        self.dialogue_index += 1;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_in = 0.0;
    }

    // 고른 결과의 동작은 대화창이 닫힌 뒤(update() 에서) 실행한다.
    fn resolve_choice(&mut self, accepted: bool) {
        if let Some(DialogueEntry::Choice { on_accept, .. }) = self.dialogue_entries.get(self.dialogue_index) {
            self.pending_action = if accepted { *on_accept } else { DialogueAction::None };
            self.dialogue_index += 1;
        }
    }

    // 도어즈는 보리지꽃을 달라는 말만 한다.
    fn talk_to_doors(&mut self) {
        self.start_dialogue(vec![DialogueEntry::Choice { prompt: "보리지꽃을 줘.".to_string(), on_accept: DialogueAction::StartFlowerQuest }]);
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        let input = win.input;
        if !win.focused {
            self.captured = false;
            return;
        }
        if input.pressed(KeyCode::Escape) {
            self.captured = false;
        }
        if win.mouse_clicked && in_view && !self.captured {
            // 첫 클릭은 시점 모드로 들어가는 데만 쓴다(대화 넘기기 등으로 안 샌다).
            self.captured = true;
            return;
        }

        if self.overlay == GameOverlay::SubMinigameStub {
            if input.pressed(KeyCode::Enter) || input.pressed(KeyCode::KpEnter) {
                self.flowers += 1;
                self.doors_fulfilled = true;
                self.overlay = GameOverlay::None;
                self.start_dialogue(vec![DialogueEntry::Line("보리지꽃을 손에 넣었다...".to_string())]);
            }
            return;
        }
        if self.dialogue_active() {
            if self.dialogue_choice_ready() {
                if input.pressed(KeyCode::Y) {
                    self.resolve_choice(true);
                } else if input.pressed(KeyCode::N) {
                    self.resolve_choice(false);
                }
            } else if (input.any_key_pressed() && !input.pressed(KeyCode::Escape)) || (win.mouse_clicked && in_view) {
                self.advance_dialogue();
            }
            return;
        }
        if input.pressed(KeyCode::E) && self.door_aimed && !self.doors_fulfilled {
            self.talk_to_doors();
        }
    }
}

impl App for DoorsGameApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        // 창 안에 4:3 으로 맞춰 넣을 게임 화면 영역(남는 곳은 검은 띠).
        let s = (area.w / VIEW_W).min(area.h / VIEW_H);
        let (vw, vh) = (VIEW_W * s, VIEW_H * s);
        let view = Rect::new(area.x + (area.w - vw) / 2.0, area.y + (area.h - vh) / 2.0, vw, vh);
        let in_view = view.contains(win.mouse.0, win.mouse.1);

        self.handle_input(win, in_view);
        if self.captured {
            crate::scenes::request_mouse_look();
        }

        // 타자기 효과.
        if self.dialogue_active() {
            let full_len = self.current_dialogue_text().chars().count();
            self.dialogue_next_char_in -= win.dt;
            while self.dialogue_visible_chars < full_len && self.dialogue_next_char_in <= 0.0 {
                self.dialogue_visible_chars += 1;
                self.dialogue_next_char_in += DIALOGUE_CHAR_DELAY_MIN + rand01(&mut self.dialogue_rng) * (DIALOGUE_CHAR_DELAY_MAX - DIALOGUE_CHAR_DELAY_MIN);
            }
        }
        if !self.dialogue_active() && self.pending_action != DialogueAction::None {
            if self.pending_action == DialogueAction::StartFlowerQuest {
                self.overlay = GameOverlay::SubMinigameStub;
            }
            self.pending_action = DialogueAction::None;
        }

        // 대화창/스텁 화면이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue_active() || self.overlay != GameOverlay::None;
        if !frozen {
            if self.captured {
                let (dx, dy) = win.input.look_delta;
                self.player.look(dx * MOUSE_SENS, -dy * MOUSE_SENS);
            }
            self.player.update(win.input, win.focused, win.dt, &self.boxes);
            self.door_aimed = aimed_at(&self.player.camera(), HANDLE_POS);
        }

        let cam = self.player.camera();
        let tex = MESH3D.with(|cell| {
            let mut slot = cell.borrow_mut();
            let mesh = slot.get_or_insert_with(|| Mesh3D::new(ctx, VIEW_W as u32, VIEW_H as u32));
            mesh.render(ctx, CLEAR_COLOR, &cam, &self.boxes, FOV_Y);
            mesh.color_texture()
        });

        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        // 오프스크린 텍스처는 위아래가 뒤집혀 있어 v 를 뒤집어 붙인다(crt.rs 와 같은 이유).
        r.sprite_uv(tex, view.x, view.y, view.w, view.h, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        // 이하 HUD — VIEW 기준 좌표/크기에 배율 s 를 곱해 view 위에 그린다.
        let px = |x: f32| view.x + x * s;
        let py = |y: f32| view.y + y * s;

        r.rect(px(VIEW_W / 2.0 - 1.5), py(VIEW_H / 2.0 - 1.5), 3.0 * s, 3.0 * s, [1.0, 1.0, 1.0, 0.7]);

        if self.door_aimed
            && !frozen
            && !self.doors_fulfilled
            && let Some((sx, sy)) = world_to_view(&cam, HANDLE_POS)
        {
            let label = "[E] Examine";
            let tw = r.text_width(label, 0.7 * s);
            r.rect(px(sx + 10.0), py(sy - 10.0), tw + 8.0 * s, 16.0 * s, [0.0, 0.0, 0.0, 0.6]);
            r.text(px(sx + 14.0), py(sy - 8.0), label, 0.7 * s, [0.6, 0.9, 1.0, 1.0]);
        }

        if self.flowers > 0 {
            r.text(px(10.0), py(10.0), &format!("보리지꽃 x{}", self.flowers), 0.8 * s, [0.75, 0.8, 1.0, 1.0]);
        }

        if !self.captured && !frozen {
            let hint = "클릭해서 시점 조작 (Esc: 해제)";
            let tw = r.text_width(hint, 0.7 * s);
            r.rect(px(VIEW_W / 2.0) - tw / 2.0 - 6.0 * s, py(VIEW_H - 30.0), tw + 12.0 * s, 20.0 * s, [0.0, 0.0, 0.0, 0.6]);
            r.text(px(VIEW_W / 2.0) - tw / 2.0, py(VIEW_H - 27.0), hint, 0.7 * s, [0.9, 0.9, 0.9, 1.0]);
        }

        if self.overlay == GameOverlay::SubMinigameStub {
            r.rect(view.x, view.y, view.w, view.h, DIM_COLOR);
            let msg = "[스텁] 보리지꽃 미니게임 자리";
            let tw = r.text_width(msg, 0.9 * s);
            r.text(px(VIEW_W / 2.0) - tw / 2.0, py(VIEW_H / 2.0 - 20.0), msg, 0.9 * s, [1.0, 1.0, 1.0, 1.0]);
            let hint = "Enter: 클리어 처리(테스트용)";
            let hw = r.text_width(hint, 0.7 * s);
            r.text(px(VIEW_W / 2.0) - hw / 2.0, py(VIEW_H / 2.0 + 10.0), hint, 0.7 * s, [0.8, 0.9, 1.0, 1.0]);
        }

        if self.dialogue_active() {
            let full_text = self.current_dialogue_text();
            let full_len = full_text.chars().count();
            let shown: String = full_text.chars().take(self.dialogue_visible_chars).collect();
            let box_x = DIALOGUE_SIDE_MARGIN;
            let box_w = VIEW_W - DIALOGUE_SIDE_MARGIN * 2.0;
            let box_y = VIEW_H - DIALOGUE_BOTTOM_MARGIN - DIALOGUE_HEIGHT;
            r.rect(px(box_x), py(box_y), box_w * s, DIALOGUE_HEIGHT * s, [0.0, 0.0, 0.0, 0.82]);
            r.rect(px(box_x), py(box_y), box_w * s, 2.0 * s, [0.6, 0.6, 0.65, 0.9]);
            r.text(px(box_x + 16.0), py(box_y + 16.0), &shown, DIALOGUE_TEXT_SCALE * s, [1.0, 1.0, 1.0, 1.0]);
            let bottom_y = py(box_y + DIALOGUE_HEIGHT - DIALOGUE_LINE_H - 10.0);
            if self.dialogue_choice_ready() {
                r.text(px(box_x + 16.0), bottom_y, "[Y] 수락   [N] 거부", DIALOGUE_TEXT_SCALE * s, [1.0, 0.9, 0.5, 1.0]);
            } else if self.dialogue_visible_chars >= full_len && (win.time * 2.2).sin() > 0.0 {
                r.text(px(box_x + box_w - 30.0), bottom_y, "v", DIALOGUE_TEXT_SCALE * s, [0.8, 0.8, 0.85, 1.0]);
            }
        }

        AppAction::None
    }
}
