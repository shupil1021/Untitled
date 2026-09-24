// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/mesh3d_test.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! 메인 게임 "DOORS" — 친구가 메일로 보낸 크랙 게임(STORY.md 7-1절). PalaceOS 안에서
//! 메일 첨부 `DOORS.exe`(FileKind::Game)를 받아 열면 desktop.rs::launch_if_game 이
//! 이 실행 파일(`doors.exe`, 같은 폴더)을 별도 프로세스로 띄운다. 따로 확인할 땐
//! `cargo run --bin doors`.
//!
//! 지금 맵은 가장 단순한 형태 하나뿐이다: 사방(+바닥/천장)이 막힌 방, 플레이어
//! 정면 벽에 문 하나. 그 문이 도어즈(Doors, 말하는 문 NPC)다 — 손잡이를 조준하고
//! `E`를 누르면 부탁(보리지꽃) → `[Y] 수락 / [N] 거부` 선택지 → 수락하면 꽃 퀘스트
//! 스텁 화면(진짜 미니게임은 아직 없음, `Enter`로 클리어 처리) 흐름이 이어진다.
//! 대화/선택지/퀘스트 스텁 로직은 프로토타입이던 mesh3d_test.rs 에서 그대로
//! 가져왔다(그쪽은 테스트용으로 그대로 남겨둔다 — 앞으로 게임 쪽은 이 파일에서
//! 키워 나간다). 문은 지금 열리지 않는 닫힌 장애물이다(엔딩의 "현관문을 여는"
//! 연출은 나중 일).
//!
//! 조작: 마우스로 시점, W/S/A/D 이동, E 상호작용. 화면은 main.rs 와 같은 CRT
//! 셰이더(crt.rs)를 거쳐 나온다.

use miniquad::*;

use crackhead::crt::Crt;
use crackhead::gfx::Renderer;
use crackhead::mesh3d::{ground_height, resolve_horizontal, v_add, v_dot, v_len, v_scale, v_sub, Box3D, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const CHROMATIC_ABERRATION: f32 = 0.5;
const CRT_INTENSITY: f32 = 1.0;

const MOVE_SPEED: f32 = 2.6;
const MOUSE_SENS: f32 = 0.0032;
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

const DIALOGUE_SIDE_MARGIN: f32 = 24.0;
const DIALOGUE_BOTTOM_MARGIN: f32 = 36.0;
const DIALOGUE_HEIGHT: f32 = WIN_H / 3.0;
const DIALOGUE_TEXT_SCALE: f32 = 1.4;
// 글자 한 줄의 실제 높이 — 선택지 안내/계속 화살표를 대화창 아래쪽에 붙일 때,
// 글자가 커져도 상자 밖으로 삐져나가지 않게 이 높이만큼 위로 올려 그린다.
const DIALOGUE_LINE_H: f32 = crackhead::gfx::CELL_H * DIALOGUE_TEXT_SCALE;
const DIALOGUE_CHAR_DELAY_MIN: f64 = 0.02;
const DIALOGUE_CHAR_DELAY_MAX: f64 = 0.09;
const DIM_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

// 대화 한 줄(Line)과 수락/거부 선택지(Choice) — mesh3d_test.rs 와 같은 구조.
enum DialogueEntry {
    Line(String),
    Choice { prompt: String, accept_lines: Vec<String>, decline_lines: Vec<String>, on_accept: DialogueAction, on_decline: DialogueAction },
}

#[derive(Clone, Copy, PartialEq)]
enum DialogueAction {
    None,
    StartFlowerQuest,
    MarkDoorsDeclined,
}

// 도어즈 — 지금은 이 방의 문 하나뿐.
struct DoorsNpc {
    request_lines: Vec<String>,
    accept_lines: Vec<String>,
    decline_lines: Vec<String>,
    fulfilled: bool,
}

fn build_doors() -> DoorsNpc {
    DoorsNpc {
        request_lines: vec!["...누구야.".to_string(), "나가고 싶은데 몸이 없어서 못 나가.".to_string(), "보리지꽃을 가져다 줄래?".to_string()],
        accept_lines: vec!["...정말? 고마워.".to_string()],
        decline_lines: vec!["...그래.".to_string()],
        fulfilled: false,
    }
}

struct QuestState {
    flowers: u32,
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

// 사방이 막힌 방 — 바닥/천장/벽 4면(앞쪽 벽은 문간만 비우고 두 조각) + 닫힌 문.
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

    // 앞쪽 벽 — 문 왼쪽/오른쪽 조각 + 문 위 상인방(문 높이 위로 남는 부분).
    let door_max_x = DOOR_MIN_X + DOOR_WIDTH;
    let left_hw = (DOOR_MIN_X - ROOM_MIN_X) / 2.0;
    let right_hw = (ROOM_MAX_X - door_max_x) / 2.0;
    boxes.push(solid([ROOM_MIN_X + left_hw, hy, ROOM_MIN_Z], [left_hw, hy, WALL_THICK], WALL_COLOR, false));
    boxes.push(solid([door_max_x + right_hw, hy, ROOM_MIN_Z], [right_hw, hy, WALL_THICK], WALL_COLOR, false));
    let door_top = DOOR_HALF_H * 2.0;
    let lintel_hy = (ROOM_HEIGHT - door_top) / 2.0;
    boxes.push(solid([0.0, door_top + lintel_hy, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, lintel_hy, WALL_THICK], WALL_COLOR, false));

    // 문짝 + 손잡이.
    boxes.push(solid([DOOR_MIN_X + DOOR_WIDTH / 2.0, DOOR_HALF_H, ROOM_MIN_Z], [DOOR_WIDTH / 2.0, DOOR_HALF_H, DOOR_HALF_T], DOOR_COLOR, false));
    boxes.push(Box3D { center: HANDLE_POS, half: HANDLE_HALF, yaw: 0.0, pitch: 0.0, roll: 0.0, color: HANDLE_COLOR, texture: None, walkable: false, solid: false });
    boxes
}

fn world_to_screen(cam: &Camera, p: [f32; 3]) -> Option<(f32, f32)> {
    let rel = v_sub(p, cam.pos);
    let depth = v_dot(rel, cam.forward());
    if depth <= 0.05 {
        return None;
    }
    let half_h = (FOV_Y / 2.0).tan();
    let half_w = half_h * (WIN_W / WIN_H);
    let ndc_x = v_dot(rel, cam.right()) / (depth * half_w);
    let ndc_y = v_dot(rel, cam.up()) / (depth * half_h);
    Some(((ndc_x * 0.5 + 0.5) * WIN_W, (1.0 - (ndc_y * 0.5 + 0.5)) * WIN_H))
}

// 화면 중앙 조준선이 target 을 향하고 있는지(거리 + 각도만 본다).
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

    fn update(&mut self, input: &Input, dt: f32, boxes: &[Box3D]) {
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

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    mesh3d: Mesh3D,
    crt: Crt,
    boxes: Vec<Box3D>,
    player: Player,
    door_aimed: bool,
    dialogue_entries: Vec<DialogueEntry>,
    dialogue_index: usize,
    dialogue_visible_chars: usize,
    dialogue_next_char_at: f64,
    dialogue_rng: u64,
    pending_action: DialogueAction,
    doors: DoorsNpc,
    quest: QuestState,
    overlay: GameOverlay,
    input: Input,
    start_time: f64,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mesh3d = Mesh3D::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        let crt = Crt::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        let now = date::now();
        window::show_mouse(false);
        window::set_cursor_grab(true);
        recenter_cursor();
        Stage {
            ctx,
            renderer,
            mesh3d,
            crt,
            boxes: build_room(),
            player: Player { feet: SPAWN, yaw: 0.0, pitch: 0.0, vel_y: 0.0 },
            door_aimed: false,
            dialogue_entries: Vec::new(),
            dialogue_index: 0,
            dialogue_visible_chars: 0,
            dialogue_next_char_at: 0.0,
            dialogue_rng: 0x9E3779B97F4A7C15,
            pending_action: DialogueAction::None,
            doors: build_doors(),
            quest: QuestState { flowers: 0 },
            overlay: GameOverlay::None,
            input: Input::default(),
            start_time: now,
            last_time: now,
        }
    }

    fn dialogue_active(&self) -> bool {
        self.dialogue_index < self.dialogue_entries.len()
    }

    fn current_dialogue_text(&self) -> Option<&str> {
        match self.dialogue_entries.get(self.dialogue_index) {
            Some(DialogueEntry::Line(s)) => Some(s.as_str()),
            Some(DialogueEntry::Choice { prompt, .. }) => Some(prompt.as_str()),
            None => None,
        }
    }

    fn dialogue_choice_ready(&self) -> bool {
        match self.dialogue_entries.get(self.dialogue_index) {
            Some(DialogueEntry::Choice { prompt, .. }) => self.dialogue_visible_chars >= prompt.chars().count(),
            _ => false,
        }
    }

    fn start_dialogue(&mut self, lines: Vec<String>) {
        self.start_dialogue_entries(lines.into_iter().map(DialogueEntry::Line).collect());
    }

    fn start_dialogue_entries(&mut self, entries: Vec<DialogueEntry>) {
        self.dialogue_entries = entries;
        self.dialogue_index = 0;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_at = date::now();
    }

    // 타이핑 중이면 그 줄을 다 보여주고, 다 보여준 평범한 줄이면 다음으로.
    // 선택지는 Y/N(resolve_choice)으로만 넘어간다.
    fn advance_dialogue(&mut self) {
        if !self.dialogue_active() {
            return;
        }
        let full_len = self.current_dialogue_text().map(|s| s.chars().count()).unwrap_or(0);
        if self.dialogue_visible_chars < full_len {
            self.dialogue_visible_chars = full_len;
            return;
        }
        if matches!(self.dialogue_entries[self.dialogue_index], DialogueEntry::Choice { .. }) {
            return;
        }
        self.dialogue_index += 1;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_at = date::now();
    }

    // 선택 결과의 동작은 반응 대사가 다 끝난 뒤(draw() 에서) 실행한다.
    fn resolve_choice(&mut self, accepted: bool) {
        let Some(DialogueEntry::Choice { accept_lines, decline_lines, on_accept, on_decline, .. }) = self.dialogue_entries.get(self.dialogue_index) else {
            return;
        };
        let (action, reaction_lines) = if accepted { (*on_accept, accept_lines.clone()) } else { (*on_decline, decline_lines.clone()) };
        self.pending_action = action;
        let insert_at = self.dialogue_index + 1;
        for (i, line) in reaction_lines.into_iter().enumerate() {
            self.dialogue_entries.insert(insert_at + i, DialogueEntry::Line(line));
        }
        self.dialogue_index += 1;
        self.dialogue_visible_chars = 0;
        self.dialogue_next_char_at = date::now();
    }

    fn run_dialogue_action(&mut self, action: DialogueAction) {
        match action {
            DialogueAction::None | DialogueAction::MarkDoorsDeclined => {}
            DialogueAction::StartFlowerQuest => self.overlay = GameOverlay::SubMinigameStub,
        }
    }

    fn talk_to_doors(&mut self) {
        if self.doors.fulfilled {
            self.start_dialogue(vec!["...고마워.".to_string()]);
            return;
        }
        let mut entries: Vec<DialogueEntry> = self.doors.request_lines.iter().cloned().map(DialogueEntry::Line).collect();
        entries.push(DialogueEntry::Choice {
            prompt: "부탁을 들어줄래?".to_string(),
            accept_lines: self.doors.accept_lines.clone(),
            decline_lines: self.doors.decline_lines.clone(),
            on_accept: DialogueAction::StartFlowerQuest,
            on_decline: DialogueAction::MarkDoorsDeclined,
        });
        self.start_dialogue_entries(entries);
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        if self.dialogue_active() {
            let full_len = self.current_dialogue_text().map(|s| s.chars().count()).unwrap_or(0);
            while self.dialogue_visible_chars < full_len && now >= self.dialogue_next_char_at {
                self.dialogue_visible_chars += 1;
                let delay = DIALOGUE_CHAR_DELAY_MIN + rand01(&mut self.dialogue_rng) as f64 * (DIALOGUE_CHAR_DELAY_MAX - DIALOGUE_CHAR_DELAY_MIN);
                self.dialogue_next_char_at = now + delay;
            }
        }

        if !self.dialogue_active() && self.pending_action != DialogueAction::None {
            let action = self.pending_action;
            self.pending_action = DialogueAction::None;
            self.run_dialogue_action(action);
        }

        // 대화창/스텁 화면이 떠 있는 동안은 이동·조준 갱신을 멈춘다.
        if !self.dialogue_active() && self.overlay == GameOverlay::None {
            self.player.update(&self.input, dt, &self.boxes);
            self.door_aimed = aimed_at(&self.player.camera(), HANDLE_POS);
        }

        let cam = self.player.camera();
        self.mesh3d.render(self.ctx.as_mut(), CLEAR_COLOR, &cam, &self.boxes, FOV_Y);

        self.renderer.begin(WIN_W, WIN_H);
        let tex = self.mesh3d.color_texture();
        self.renderer.sprite_uv(tex, 0.0, 0.0, WIN_W, WIN_H, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        let (cx, cy) = (WIN_W / 2.0, WIN_H / 2.0);
        self.renderer.rect(cx - 1.5, cy - 1.5, 3.0, 3.0, [1.0, 1.0, 1.0, 0.7]);

        if self.door_aimed
            && !self.dialogue_active()
            && self.overlay == GameOverlay::None
            && let Some((sx, sy)) = world_to_screen(&cam, HANDLE_POS)
        {
            let label = "[E] Examine";
            let tw = self.renderer.text_width(label, 0.7);
            self.renderer.rect(sx + 10.0, sy - 10.0, tw + 8.0, 16.0, [0.0, 0.0, 0.0, 0.6]);
            self.renderer.text(sx + 14.0, sy - 8.0, label, 0.7, [0.6, 0.9, 1.0, 1.0]);
        }

        if self.quest.flowers > 0 {
            let inv = format!("보리지꽃 x{}", self.quest.flowers);
            self.renderer.text(10.0, 10.0, &inv, 0.7, [0.75, 0.8, 1.0, 1.0]);
        }

        if self.overlay == GameOverlay::SubMinigameStub {
            self.renderer.rect(0.0, 0.0, WIN_W, WIN_H, DIM_COLOR);
            let msg = "[스텁] 보리지꽃 미니게임 자리";
            let tw = self.renderer.text_width(msg, 0.9);
            self.renderer.text((WIN_W - tw) / 2.0, WIN_H / 2.0 - 20.0, msg, 0.9, [1.0, 1.0, 1.0, 1.0]);
            let hint = "Enter: 클리어 처리(테스트용)";
            let hw = self.renderer.text_width(hint, 0.7);
            self.renderer.text((WIN_W - hw) / 2.0, WIN_H / 2.0 + 10.0, hint, 0.7, [0.8, 0.9, 1.0, 1.0]);
        }

        if self.dialogue_active() {
            let full_text = self.current_dialogue_text().unwrap_or("");
            let full_len = full_text.chars().count();
            let shown: String = full_text.chars().take(self.dialogue_visible_chars).collect();
            let box_x = DIALOGUE_SIDE_MARGIN;
            let box_w = WIN_W - DIALOGUE_SIDE_MARGIN * 2.0;
            let box_y = WIN_H - DIALOGUE_BOTTOM_MARGIN - DIALOGUE_HEIGHT;
            self.renderer.rect(box_x, box_y, box_w, DIALOGUE_HEIGHT, [0.0, 0.0, 0.0, 0.82]);
            self.renderer.rect(box_x, box_y, box_w, 2.0, [0.6, 0.6, 0.65, 0.9]);
            self.renderer.text(box_x + 16.0, box_y + 16.0, &shown, DIALOGUE_TEXT_SCALE, [1.0, 1.0, 1.0, 1.0]);
            if self.dialogue_choice_ready() {
                self.renderer.text(box_x + 16.0, box_y + DIALOGUE_HEIGHT - DIALOGUE_LINE_H - 10.0, "[Y] 수락   [N] 거부", DIALOGUE_TEXT_SCALE, [1.0, 0.9, 0.5, 1.0]);
            } else if self.dialogue_visible_chars >= full_len && (now * 2.2).sin() > 0.0 {
                self.renderer.text(box_x + box_w - 30.0, box_y + DIALOGUE_HEIGHT - DIALOGUE_LINE_H - 10.0, "v", DIALOGUE_TEXT_SCALE, [0.8, 0.8, 0.85, 1.0]);
            }
        }

        // main.rs::draw() 와 같은 순서 — 2D 합성본 전체를 CRT 오프스크린에 흘려보낸 뒤
        // 곡률/색수차를 입혀 실제 화면에 그린다.
        self.crt.begin(self.ctx.as_mut());
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        let elapsed = (now - self.start_time) as f32;
        self.crt.present(self.ctx.as_mut(), elapsed, CHROMATIC_ABERRATION, CRT_INTENSITY);
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        if !repeat && self.overlay == GameOverlay::SubMinigameStub && matches!(keycode, KeyCode::Enter | KeyCode::KpEnter) {
            self.quest.flowers += 1;
            self.doors.fulfilled = true;
            self.overlay = GameOverlay::None;
            self.start_dialogue(vec!["보리지꽃을 손에 넣었다...".to_string()]);
        } else if !repeat && self.dialogue_active() {
            if self.dialogue_choice_ready() {
                if keycode == KeyCode::Y {
                    self.resolve_choice(true);
                } else if keycode == KeyCode::N {
                    self.resolve_choice(false);
                }
            } else {
                self.advance_dialogue();
            }
        } else if !repeat && keycode == KeyCode::E && self.door_aimed && self.overlay == GameOverlay::None {
            self.talk_to_doors();
        }
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.input.on_key_up(keycode);
    }

    fn mouse_button_down_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {
        if self.dialogue_active() && !self.dialogue_choice_ready() {
            self.advance_dialogue();
        }
    }

    fn mouse_enter_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {
        recenter_cursor();
    }

    // 커서를 화면 중앙에 고정해두고, 벗어난 만큼만 시점 회전에 반영한다
    // (mesh3d_test.rs 와 같은 방식 — raw_mouse_motion 이 안 들어오는 환경 대비).
    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let (dx, dy) = (x - WIN_W / 2.0, y - WIN_H / 2.0);
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return;
        }
        if !self.dialogue_active() && self.overlay == GameOverlay::None {
            self.player.look(dx * MOUSE_SENS, -dy * MOUSE_SENS);
        }
        recenter_cursor();
    }
}

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
        window_title: "DOORS".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        window_resizable: false,
        fullscreen: false,
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
