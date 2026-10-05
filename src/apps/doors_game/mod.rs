//! 메인 게임 "DOORS" — 친구가 메일로 보낸 크랙 게임(STORY.md 7-1절). 메일 첨부
//! `test Setup.exe`로 설치(apps/game_installer.rs)하면 바탕화면에 생기는
//! `test.exe`(FileKind::Game)를 열었을 때 다른 앱들처럼 PalaceOS 안의 창 하나로
//! 뜬다(별도 실행 파일/OS 창이 아니다). 화면/시점/플레이어/대화창은 apps/game3d 의
//! 공용 도구를 쓴다.
//!
//! 맵은 사방(+바닥/천장)이 막힌 방 하나, 플레이어 정면 벽에 닫힌 문 하나(world.rs). 그
//! 문이 도어즈(Doors, 말하는 문 NPC)다 — 손잡이를 조준하고 `E`를 누르면 보리지꽃을
//! 달라고 하고 `[Y] 수락 / [N] 거부` 선택지가 뜬다. 수락하면 퀘스트가 "수락됨"으로
//! 기록되고(더는 말을 걸 수 없다, 거부하면 다시 걸 수 있다), 그 뒤 방 물건을 처음
//! 조사하면 "방에는 꽃이 없는 것 같다..."가 나오면서 OS 에 두 번째 메일을 요청한다.
//!
//! 조작: 게임 화면을 클릭하면 마우스 시점 모드, `Esc` 나 다른 창으로 포커스를 옮기면
//! 풀린다. W/S/A/D 이동, E 상호작용(창이 포커스일 때만).

mod world;

use miniquad::{KeyCode, RenderingBackend};

use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::player::Player;
use super::game3d::{MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use world::{aimed_target, build_room, Target, PROPS, SPAWN};

const CLEAR_COLOR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

pub struct DoorsGameApp {
    boxes: Vec<Box3D>,
    player: Player,
    slot: RenderSlot, // 이 창 전용 3D 렌더 타깃(창끼리 화면이 섞이지 않게)
    look: MouseLook,
    aimed: Option<Target>, // 지금 조준선이 향한 대상(문 손잡이/방 안 물건)
    dialogue: Dialogue,
    quest_accepted: bool, // 도어즈의 부탁(보리지꽃)을 수락했는지 — 그 뒤론 더 말을 걸 수 없다
    // 수락한 뒤 방 물건을 조사해서 "방에 꽃이 없다"는 걸 확인했는지 — 다음 이벤트(꽃 있는
    // 곳을 안다는 두 번째 메일)의 조건이다.
    flower_absence_checked: bool,
    second_mail_sent: bool, // 그 확인을 OS 에 알려서 두 번째 메일을 요청했는지(한 번만)
}

impl DoorsGameApp {
    pub(super) fn new() -> DoorsGameApp {
        DoorsGameApp {
            boxes: build_room(),
            player: Player::at(SPAWN, 0.0),
            slot: RenderSlot::acquire(),
            look: MouseLook::new(),
            aimed: None,
            dialogue: Dialogue::new(),
            quest_accepted: false,
            flower_absence_checked: false,
            second_mail_sent: false,
        }
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        if self.look.update(win, in_view) {
            return;
        }
        if self.dialogue.active() {
            // 선택지에 수락하면 퀘스트 수락만 기록한다.
            if self.dialogue.handle_input(win, in_view) == Some(true) {
                self.quest_accepted = true;
            }
            return;
        }
        if win.input.pressed(KeyCode::E) {
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
        let view = View::fit(area);
        let in_view = view.rect.contains(win.mouse.0, win.mouse.1);

        self.handle_input(win, in_view);
        self.look.request();
        self.dialogue.tick(win.dt);

        // 대화창이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue.active();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &self.boxes);
            self.aimed = aimed_target(&self.player.camera()).filter(|t| !(*t == Target::Door && self.quest_accepted));
        }

        let cam = self.player.camera();
        view.draw_scene(ctx, r, area, &self.slot, CLEAR_COLOR, &cam, &self.boxes);

        if let Some(target) = self.aimed
            && !frozen
        {
            let label = match target {
                Target::Door => "[E] Examine".to_string(),
                Target::Prop(i) => format!("[E] Examine {}", PROPS[i].name),
            };
            view.draw_prompt(r, &cam, target.pos(), &label);
        }
        if !self.look.captured() && !frozen {
            view.draw_capture_hint(r);
        }
        self.dialogue.draw(r, view.rect.x, view.rect.y, view.s, win.time);

        // 방에 꽃이 없다는 걸 확인했으면(대화가 다 끝난 뒤) OS 에 두 번째 메일을 요청한다.
        if self.flower_absence_checked && !self.second_mail_sent && !frozen {
            self.second_mail_sent = true;
            return AppAction::FlowerAbsenceChecked;
        }
        AppAction::None
    }
}
