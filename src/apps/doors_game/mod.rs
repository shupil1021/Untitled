//! 메인 게임 "DOORS" — 친구가 메일로 보낸 크랙 게임(STORY.md 7-1절). 메일 첨부
//! `test Setup.exe`로 설치(apps/game_installer.rs)하면 바탕화면에 생기는
//! `test.exe`(FileKind::Game)를 열었을 때 다른 앱들처럼 PalaceOS 안의 창 하나로
//! 뜬다(별도 실행 파일/OS 창이 아니다). 3D 장면은 `mesh3d.rs` 로 오프스크린에 그린
//! 뒤, 그 결과를 창 안에 4:3 비율로 끼워 넣는다(남는 곳은 검은 띠). CRT 효과는
//! 바깥 OS 화면 전체에 이미 걸려 있어서 따로 안 입힌다.
//!
//! 맵은 지금 가장 단순한 형태 하나: 사방(+바닥/천장)이 막힌 방, 플레이어 정면 벽에
//! 닫힌 문 하나(world.rs). 그 문이 도어즈(Doors, 말하는 문 NPC)다 — 손잡이를
//! 조준하고 `E`를 누르면 보리지꽃을 달라고 하고 `[Y] 수락 / [N] 거부` 선택지가
//! 뜬다(dialogue.rs). 지금은 수락하면 퀘스트가 "수락됨"으로 기록되기만 한다(꽃을
//! 구하는 미니게임/보상은 아직 없다) — 수락한 뒤로는 문에 더 말을 걸 수 없고,
//! 거부하면 다시 말을 걸 수 있다.
//!
//! 조작: 게임 화면을 클릭하면 마우스 시점 모드(커서가 사라지고 마우스로 시점이
//! 돈다 — main.rs 가 scenes::request_mouse_look() 요청을 받아 처리), `Esc` 나 다른
//! 창으로 포커스를 옮기면 풀린다. W/S/A/D 이동, E 상호작용(창이 포커스일 때만).

mod dialogue;
mod world;

use std::cell::RefCell;

use miniquad::{KeyCode, RenderingBackend};

use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::{v_dot, v_sub, Box3D, Camera, Mesh3D};

use super::{App, AppAction, WinInput};
use dialogue::{Dialogue, Entry};
use world::{aimed_target, build_room, Player, Target, PROPS};

// 3D 오프스크린 해상도 — 이 크기(4:3) 기준으로 HUD 좌표도 잡고, 실제 창 크기에
// 맞춰 통째로 배율(view.w / VIEW_W)을 곱해서 그린다.
const VIEW_W: f32 = 640.0;
const VIEW_H: f32 = 480.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const MOUSE_SENS: f32 = 0.0032; // 실제 화면 픽셀당 라디안
const CLEAR_COLOR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

// Mesh3D(오프스크린 타깃 + 셰이더/파이프라인)는 창을 열 때마다 새로 만들면 닫을
// 때 지울 방법이 없어(App 은 Drop 에서 ctx 를 못 받는다) GPU 자원이 계속 쌓인다 —
// 그래서 한 번 만든 걸 스레드 전역에 두고 다시 여는 창들이 재사용한다(이 게임
// 창은 한 번에 하나만 열린다 — desktop.rs 가 같은 파일의 창을 중복으로 안 연다).
thread_local! {
    static MESH3D: RefCell<Option<Mesh3D>> = const { RefCell::new(None) };
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

pub struct DoorsGameApp {
    boxes: Vec<Box3D>,
    player: Player,
    captured: bool, // 마우스 시점 모드인지(게임 화면을 클릭하면 켜지고 Esc/포커스 잃으면 꺼진다)
    aimed: Option<Target>, // 지금 조준선이 향한 대상(문 손잡이/방 안 물건)
    dialogue: Dialogue,
    quest_accepted: bool, // 도어즈의 부탁(보리지꽃)을 수락했는지 — 그 뒤론 더 말을 걸 수 없다
    // 수락한 뒤 방 물건을 조사해서 "방에 꽃이 없다"는 걸 확인했는지 — 다음 이벤트(꽃 있는
    // 곳을 안다는 두 번째 메일)의 조건이다.
    flower_absence_checked: bool,
}

impl DoorsGameApp {
    pub(super) fn new() -> DoorsGameApp {
        DoorsGameApp {
            boxes: build_room(),
            player: Player::spawn(),
            captured: false,
            aimed: None,
            dialogue: Dialogue::new(),
            quest_accepted: false,
            flower_absence_checked: false,
        }
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

        if self.dialogue.active() {
            if self.dialogue.choice_ready() {
                // 수락이면 퀘스트 수락만 기록한다.
                if input.pressed(KeyCode::Y) {
                    self.quest_accepted = true;
                    self.dialogue.choose();
                } else if input.pressed(KeyCode::N) {
                    self.dialogue.choose();
                }
            } else if (input.any_key_pressed() && !input.pressed(KeyCode::Escape)) || (win.mouse_clicked && in_view) {
                self.dialogue.advance();
            }
            return;
        }
        if input.pressed(KeyCode::E) {
            self.interact();
        }
    }

    // E — 조준 중인 대상을 조사한다.
    fn interact(&mut self) {
        match self.aimed {
            // 도어즈는 보리지꽃을 달라는 말만 한다. 수락한 뒤로는 더 말을 걸 수 없다.
            Some(Target::Door) if !self.quest_accepted => {
                self.dialogue.start(vec![Entry::Choice("보리지꽃을 줘.".to_string())]);
            }
            // 방 물건은 그 물건의 한 줄을 보여준다. 부탁을 수락한 뒤 처음 조사하면 이어서
            // 방에 꽃이 없다는 걸 깨닫는다.
            Some(Target::Prop(i)) => {
                let mut lines = vec![Entry::Line(PROPS[i].text.to_string())];
                if self.quest_accepted && !self.flower_absence_checked {
                    self.flower_absence_checked = true;
                    lines.push(Entry::Line("방에는 꽃이 없는 것 같다...".to_string()));
                }
                self.dialogue.start(lines);
            }
            _ => {}
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
        self.dialogue.tick(win.dt);

        // 대화창이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue.active();
        if !frozen {
            if self.captured {
                let (dx, dy) = win.input.look_delta;
                self.player.look(dx * MOUSE_SENS, -dy * MOUSE_SENS);
            }
            self.player.update(win.input, win.focused, win.dt, &self.boxes);
            self.aimed = aimed_target(&self.player.camera()).filter(|t| !(*t == Target::Door && self.quest_accepted));
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

        if let Some(target) = self.aimed
            && !frozen
            && let Some((sx, sy)) = world_to_view(&cam, target.pos())
        {
            let label = match target {
                Target::Door => "[E] Examine".to_string(),
                Target::Prop(i) => format!("[E] Examine {}", PROPS[i].name),
            };
            let tw = r.text_width(&label, 0.7 * s);
            r.rect(px(sx + 10.0), py(sy - 10.0), tw + 8.0 * s, 16.0 * s, [0.0, 0.0, 0.0, 0.6]);
            r.text(px(sx + 14.0), py(sy - 8.0), &label, 0.7 * s, [0.6, 0.9, 1.0, 1.0]);
        }

        if !self.captured && !frozen {
            let hint = "클릭해서 시점 조작 (Esc: 해제)";
            let tw = r.text_width(hint, 0.7 * s);
            r.rect(px(VIEW_W / 2.0) - tw / 2.0 - 6.0 * s, py(VIEW_H - 30.0), tw + 12.0 * s, 20.0 * s, [0.0, 0.0, 0.0, 0.6]);
            r.text(px(VIEW_W / 2.0) - tw / 2.0, py(VIEW_H - 27.0), hint, 0.7 * s, [0.9, 0.9, 0.9, 1.0]);
        }

        self.dialogue.draw(r, view.x, view.y, s, win.time);

        AppAction::None
    }
}
