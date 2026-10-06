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

use crate::foundation::DoorsProgress;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::player::Player;
use super::game3d::{MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use world::{aimed_target, build_room, letter_box, Target, PROPS, SPAWN};

const CLEAR_COLOR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

pub struct DoorsGameApp {
    boxes: Vec<Box3D>,
    player: Player,
    slot: RenderSlot, // 이 창 전용 3D 렌더 타깃(창끼리 화면이 섞이지 않게)
    look: MouseLook,
    aimed: Option<Target>, // 지금 조준선이 향한 대상(문 손잡이/방 안 물건)
    dialogue: Dialogue,
    // 진행 상황 — 시트 오른쪽 표: 수락하면 책상 위에 편지가 나타난다 → 집으면(has_letter) 문이 쓴 편지를
    // 읽게 되고 → 우편함에 넣으면(letter_sent) OS 로 같은 내용의 메일이 간다 → 그 메일을 읽고 게임으로
    // 돌아오면(door_changed) 문의 대사가 달라지고 방 물건을 조사해 꽃이 없다는 걸 깨달을 수 있다.
    // 바뀔 때마다 OS 에 저장을 요청해서(saved 와 비교) 창을 닫았다 다시 열어도 이어진다.
    progress: DoorsProgress,
    saved: DoorsProgress,
    letter_notify_pending: bool, // 편지를 넣은 걸 OS 에 아직 못 알렸다(대화가 끝나면 알린다)
    second_mail_sent: bool, // 그 확인을 OS 에 알려서 두 번째 메일을 요청했는지(한 번만)
}

impl DoorsGameApp {
    pub(super) fn new(progress: DoorsProgress) -> DoorsGameApp {
        // 처음 열었을 때만 어디에 서 있는지 한마디로 알려준다(이어서 하는 거면 생략).
        let mut dialogue = Dialogue::new();
        if progress == DoorsProgress::default() {
            dialogue.start(vec![Entry::Line("낡은 방 안이다.".to_string()), Entry::Line("정면에 문이 하나 있다.".to_string())]);
        }
        DoorsGameApp {
            boxes: build_room(),
            player: Player::at(SPAWN, 0.0),
            slot: RenderSlot::acquire(),
            look: MouseLook::new(),
            aimed: None,
            dialogue,
            // 이어서 하는 거면 이미 보낸 편지/꽃 위치 확인을 OS 에 다시 한 번 알린다(메일은 중복으로 안 온다).
            letter_notify_pending: progress.letter_sent,
            saved: progress.clone(),
            progress,
            second_mail_sent: false,
        }
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        if self.look.update(win, in_view) {
            return;
        }
        if self.dialogue.active() {
            // 선택지에 답하면 문의 반응을 한 줄 보여준다 — 수락하면 퀘스트 수락만 기록한다.
            match self.dialogue.handle_input(win, in_view) {
                Some(true) => {
                    self.progress.quest_accepted = true;
                    self.dialogue.start(vec![
                        Entry::Line("문 너머에서 작은 한숨이 새어 나왔다.".to_string()),
                        // 다음에 뭘 해야 하는지 — 책상 위에 편지가 나타난 걸 짚어준다.
                        Entry::Line("그러고 보니 책상 위에 편지가 한 통 놓여 있다.".to_string()),
                    ]);
                }
                Some(false) => self.dialogue.start(vec![Entry::Line("문은 아무 말도 하지 않았다.".to_string())]),
                None => {}
            }
            return;
        }
        if win.input.pressed(KeyCode::E) {
            self.interact(self.aimed);
        }
    }

    // 편지를 우편함에 넣은 뒤 OS 에서 그 편지 메일을 읽었고(mail_read) 이 창으로 돌아왔으면(focused)
    // 문이 달라진다 — 이때부터 문의 대사가 바뀐다. 게임은 그동안 멈추지 않고 그대로 돌아간다.
    fn notice_mail_read(&mut self, mail_read: bool, focused: bool) -> bool {
        if self.progress.letter_sent && !self.progress.door_changed && mail_read && focused && !self.dialogue.active() {
            self.progress.door_changed = true;
            self.dialogue.start(vec![Entry::Line("문 쪽에서 인기척이 느껴진다...".to_string())]);
            return true;
        }
        false
    }

    // E — 조준 중인 대상을 조사한다.
    fn interact(&mut self, target: Option<Target>) {
        match target {
            // 문이 편지를 읽은 뒤엔 달라진 말을 한다(시트의 "도어즈 대사 변경").
            Some(Target::Door) if self.progress.door_changed => {
                self.dialogue.start(vec![
                    Entry::Line("문고리에서 낮은 목소리가 들려왔다.".to_string()),
                    Entry::Line("편지는 잘 읽었어.".to_string()),
                    Entry::Line("꽃은... 조금만 더 기다려 줘.".to_string()),
                ]);
            }
            // 도어즈는 보리지꽃을 달라는 말만 한다. 수락한 직후엔 문이 말이 없다(available 에서 걸러진다).
            Some(Target::Door) if !self.progress.quest_accepted => {
                // 문이 말을 걸어오는 걸 갑자기 시작하지 않고 먼저 알려준다.
                self.dialogue.start(vec![Entry::Line("문고리에서 낮은 목소리가 들려왔다.".to_string()), Entry::Choice("보리지꽃을 줘.".to_string())]);
            }
            // 방 물건은 그 물건의 한 줄을 보여준다. 부탁을 수락한 뒤 처음 조사하면 이어서
            // 방에 꽃이 없다는 걸 깨닫는다.
            Some(Target::Prop(i)) => {
                let mut lines = vec![Entry::Line(PROPS[i].text.to_string())];
                if self.progress.quest_accepted && self.progress.door_changed && !self.progress.flower_absence_checked {
                    self.progress.flower_absence_checked = true;
                    lines.push(Entry::Line("방에는 꽃이 없는 것 같다...".to_string()));
                    // 이어서 오는 메일(두 번째 메일)을 눈치채게 한다.
                    lines.push(Entry::Line("그때 어디선가 희미한 알림음이 울렸다.".to_string()));
                }
                self.dialogue.start(lines);
            }
            // 책상 위의 편지 — 집으면 문이 쓴 편지의 내용이 그대로 나온다(OS 로 가는 메일과 같은 글).
            Some(Target::Letter) => {
                self.progress.has_letter = true;
                let mut lines = vec![Entry::Line("편지를 집어 들었다.".to_string())];
                lines.extend(crate::strings::mail::LETTER_MAIL_BODY.ko.split('\n').map(|l| Entry::Line(l.to_string())));
                self.dialogue.start(lines);
            }
            // 우편함 — 편지를 넣으면 OS 로 같은 내용의 메일이 가고, 게임은 메일을 읽고 돌아올 때까지 멈춘다.
            Some(Target::Mailbox) => {
                if self.progress.has_letter && !self.progress.letter_sent {
                    self.progress.letter_sent = true;
                    self.letter_notify_pending = true;
                    self.dialogue.start(vec![
                        Entry::Line("편지를 우편함에 넣었다.".to_string()),
                        Entry::Line("어디선가 알림음이 울린다...".to_string()),
                    ]);
                } else if self.progress.letter_sent {
                    self.dialogue.start(vec![Entry::Line("우편함은 비어 있다.".to_string())]);
                } else {
                    self.dialogue.start(vec![Entry::Line("우편함이다. 지금은 넣을 게 없다.".to_string())]);
                }
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

        // 편지 메일을 읽고 이 창으로 돌아왔으면 문이 달라진다.
        self.notice_mail_read(crate::signals::letter_mail_read(), win.focused);
        self.handle_input(win, in_view);
        self.look.request();
        self.dialogue.tick(win.dt);

        // 대화창이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue.active();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &self.boxes);
            let (accepted, door_changed, has_letter) = (self.progress.quest_accepted, self.progress.door_changed, self.progress.has_letter);
            self.aimed = aimed_target(&self.player.camera(), |t| match t {
                // 수락한 직후엔 문이 말이 없다 — 편지 메일을 읽고 돌아온 뒤에야 다시 말을 한다.
                Target::Door => !accepted || door_changed,
                // 편지는 수락한 뒤에 책상 위에 나타나고, 집으면 사라진다.
                Target::Letter => accepted && !has_letter,
                _ => true,
            });
        }

        let cam = self.player.camera();
        let mut scene = self.boxes.clone();
        if self.progress.quest_accepted && !self.progress.has_letter {
            scene.push(letter_box());
        }
        view.draw_scene(ctx, r, area, &self.slot, CLEAR_COLOR, &cam, &scene);

        if let Some(target) = self.aimed
            && !frozen
        {
            let label = match target {
                Target::Door => "[E] Examine".to_string(),
                Target::Prop(i) => format!("[E] Examine {}", PROPS[i].name),
                Target::Letter => "[E] Take letter".to_string(),
                Target::Mailbox => "[E] Mailbox".to_string(),
            };
            view.draw_prompt(r, &cam, target.pos(), &label);
        }
        if !self.look.captured() && !frozen {
            view.draw_capture_hint(r);
        }
        self.dialogue.draw(r, view.rect.x, view.rect.y, view.s, win.time);

        // 편지를 넣은 걸(대화가 끝난 뒤) OS 에 알린다 — 같은 내용의 메일이 도착한다.
        if self.letter_notify_pending && !self.dialogue.active() {
            self.letter_notify_pending = false;
            return AppAction::LetterSent;
        }

        // 방에 꽃이 없다는 걸 확인했으면(대화가 다 끝난 뒤) OS 에 두 번째 메일을 요청한다.
        if self.progress.flower_absence_checked && !self.second_mail_sent && !frozen {
            self.second_mail_sent = true;
            return AppAction::FlowerAbsenceChecked;
        }
        // 진행 상황이 바뀌었으면 OS 에 저장을 요청한다.
        if self.progress != self.saved {
            self.saved = self.progress.clone();
            return AppAction::SaveDoors(self.saved.clone());
        }
        AppAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 시트 오른쪽 표의 흐름 — 수락 → 편지 집기 → 우편함 → 메일을 읽고 돌아옴 → 문 대사 변경 →
    // 그 뒤에야 방 물건으로 "꽃이 없다"를 확인할 수 있다.
    // 저장된 진행으로 다시 열면 처음 인사 대사 없이 그 상태 그대로 이어진다.
    #[test]
    fn resuming_keeps_progress_and_skips_the_intro() {
        let fresh = DoorsGameApp::new(DoorsProgress::default());
        assert!(fresh.dialogue.active(), "처음 열면 상황 설명 대사가 나온다");

        let saved = DoorsProgress { quest_accepted: true, has_letter: true, letter_sent: true, ..DoorsProgress::default() };
        let app = DoorsGameApp::new(saved.clone());
        assert!(!app.dialogue.active());
        assert_eq!(app.progress, saved);
        assert_eq!(app.saved, saved, "불러온 값은 바뀐 게 아니라서 저장을 다시 요청하지 않는다");
        assert!(app.letter_notify_pending, "이미 보낸 편지는 OS 에 다시 한 번 알린다(메일은 중복으로 안 온다)");
    }

    #[test]
    fn letter_loop_gates_the_flower_check() {
        let mut app = DoorsGameApp::new(DoorsProgress::default());
        app.dialogue = Dialogue::new();

        app.interact(Some(Target::Mailbox));
        assert!(!app.progress.letter_sent, "편지가 없으면 보낼 게 없다");

        app.progress.quest_accepted = true;
        app.interact(Some(Target::Prop(0)));
        assert!(!app.progress.flower_absence_checked, "편지 일을 끝내기 전엔 꽃이 없다는 걸 못 깨닫는다");

        app.interact(Some(Target::Letter));
        assert!(app.progress.has_letter);
        app.interact(Some(Target::Mailbox));
        assert!(app.progress.letter_sent && app.letter_notify_pending);

        app.dialogue = Dialogue::new();
        assert!(!app.notice_mail_read(false, true), "메일을 안 읽었으면 문은 그대로");
        assert!(!app.notice_mail_read(true, false), "창으로 돌아오기 전엔 문은 그대로");
        assert!(app.notice_mail_read(true, true) && app.progress.door_changed);

        app.dialogue = Dialogue::new();
        app.interact(Some(Target::Prop(0)));
        assert!(app.progress.flower_absence_checked);
    }
}
