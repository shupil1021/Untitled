//! 2D 미니게임 셋 — 2층 미션 방(눈/뇌/손가락)에서 "게임 받기"를 누르면 CRT 로 설치 파일이 메일로 오고,
//! 설치하면 바탕화면에 생기는 `SHY_EYE.exe` / `BRAIN_DRIVE.exe` / `PACK.exe`(FileKind::MiniGame).
//! 셋이 같은 틀을 쓴다: 안내(START) → 한 판(Round) → 실패하면 RETRY / 성공하면 CLEAR → 보상 상자에서 재료를
//! 꺼내 → 우체통에 넣으면 보상 메일(사진 첨부)이 CRT 로 간다. 판 하나는 도중에 창을 닫으면 처음부터
//! 다시지만, 클리어한 뒤의 보상 상태(cleared/reward_taken/reward_sent)는 저장돼서 다시 안 해도 이어진다.
//! 창이 포커스가 아닐 땐 시간이 멈춘다(한 번에 한 게임만 진행한다).

pub mod drive;
mod eye;
mod pack;
mod view;

use view::{centered, fill};

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{Language, MiniKind, MiniProgress, Settings};
use crate::random::Rng;
use crate::render::gfx::{Assets, Color, Rect, Renderer};
use crate::strings::{minigame as s, t};
use crate::ui::button;

use super::game3d::{RenderSlot, View};
use super::{App, AppAction, WinInput};
use drive::DriveRound;
use eye::EyeRound;
use pack::{PackRound, Phase};

// 창 크기(테두리 포함) — 클라이언트 영역이 480x360 이 되게 한다(메인 게임과 같은 계산).
pub const WIN_W: f32 = 486.0;
pub const WIN_H: f32 = 386.0;
const W: f32 = 480.0;
const H: f32 = 360.0;
const HUD_H: f32 = 24.0;

// 한 프레임의 입력 — 게임 로직(eye/drive/pack)이 키보드와 직접 얽히지 않게 한다.
#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub dx: f32,
    pub dy: f32,
    pub brake: bool,
    pub act: bool, // 눌린 순간 한 번(길게 눌러도 1회)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Running,
    Won,
    Lost,
}

pub fn new_app(kind: MiniKind, progress: MiniProgress, settings: Rc<RefCell<Settings>>) -> Box<dyn App> {
    Box::new(MiniGameApp::new(kind, progress, settings))
}

enum Round {
    Eye(EyeRound),
    Drive(DriveRound),
    Pack(PackRound),
}

impl Round {
    fn new(kind: MiniKind) -> Round {
        let rng = Rng::from_time();
        match kind {
            MiniKind::Eye => Round::Eye(EyeRound::new(rng)),
            MiniKind::Brain => Round::Drive(DriveRound::new(rng)),
            MiniKind::Finger => Round::Pack(PackRound::new()),
        }
    }

    fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        match self {
            Round::Eye(r) => r.step(dt, c),
            Round::Drive(r) => r.step(dt, c),
            Round::Pack(r) => r.step(dt, c),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Intro,
    Playing,
    Failed,
    Cleared,
    Reward,
    Sent,
}

pub struct MiniGameApp {
    kind: MiniKind,
    settings: Rc<RefCell<Settings>>,
    progress: MiniProgress,
    saved: MiniProgress,
    stage: Stage,
    round: Round,
    clock: f32,
    stage_t: f32,
    hint_shown: bool, // 첫 실패 뒤 한 번만 도움말을 보여준다
    show_hint: bool,
    pending_mail: bool,
    slot: Option<RenderSlot>, // 3D 레이싱(BRAIN_DRIVE)만 쓰는 이 창 전용 3D 렌더 타깃
}

impl MiniGameApp {
    fn new(kind: MiniKind, progress: MiniProgress, settings: Rc<RefCell<Settings>>) -> MiniGameApp {
        let stage = if progress.reward_sent {
            Stage::Sent
        } else if progress.cleared {
            Stage::Reward
        } else {
            Stage::Intro
        };
        MiniGameApp { kind, settings, saved: progress, progress, stage, round: Round::new(kind), clock: 0.0, stage_t: 0.0, hint_shown: false, show_hint: false, pending_mail: false, slot: (kind == MiniKind::Brain).then(RenderSlot::acquire) }
    }

    fn controls(&self, win: &WinInput, pack_button: Rect) -> Controls {
        // 다른 창이 앞에 있을 땐 키 입력이 이 게임에 들어오지 않는다(뒤에서 누른 키가 판정되던 문제).
        if !win.focused {
            return Controls::default();
        }
        let i = win.input;
        let (left, right) = (i.is_down(KeyCode::A) || i.is_down(KeyCode::Left), i.is_down(KeyCode::D) || i.is_down(KeyCode::Right));
        let (up, down) = (i.is_down(KeyCode::W) || i.is_down(KeyCode::Up), i.is_down(KeyCode::S) || i.is_down(KeyCode::Down));
        let axis = |neg: bool, pos: bool| (pos as i32 - neg as i32) as f32;
        let clicked_button = win.mouse_clicked && pack_button.contains(win.mouse.0, win.mouse.1);
        Controls {
            dx: axis(left, right),
            dy: axis(up, down),
            brake: down,
            act: i.pressed(KeyCode::Space) || clicked_button,
        }
    }

    fn lines(&self) -> [crate::strings::S; 3] {
        match self.kind {
            MiniKind::Eye => s::EYE_INTRO,
            MiniKind::Brain => s::DRIVE_INTRO,
            MiniKind::Finger => s::PACK_INTRO,
        }
    }

    fn hint(&self) -> crate::strings::S {
        match self.kind {
            MiniKind::Eye => s::EYE_HINT,
            MiniKind::Brain => s::DRIVE_HINT,
            MiniKind::Finger => s::PACK_HINT,
        }
    }

    fn item_name(&self) -> crate::strings::S {
        match self.kind {
            MiniKind::Eye => s::ITEM_EYE,
            MiniKind::Brain => s::ITEM_BRAIN,
            MiniKind::Finger => s::ITEM_FINGER,
        }
    }
}

impl App for MiniGameApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        let dt = if win.focused { win.dt.min(0.05) } else { 0.0 };
        self.clock += dt;
        self.stage_t += dt;
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        let o = (area.x + (area.w - W) / 2.0, area.y + (area.h - H) / 2.0);
        let panel = Rect::new(o.0, o.1, W, H);
        let name = self.kind.app_name();

        match self.stage {
            Stage::Intro => {
                self.draw_reward_backdrop_for_intro(r, o);
                centered(r, o, 40.0, name, 1.8, [1.0, 0.9, 0.7, 1.0]);
                for (i, line) in self.lines().iter().enumerate() {
                    centered(r, o, 120.0 + i as f32 * 30.0, t(lang, *line), 0.9, [0.9, 0.9, 0.95, 1.0]);
                }
                if button(r, o.0 + W / 2.0 - 60.0, o.1 + 260.0, 120.0, 34.0, t(lang, s::START), win) {
                    self.round = Round::new(self.kind);
                    self.stage = Stage::Playing;
                    self.stage_t = 0.0;
                }
            }
            Stage::Playing | Stage::Failed | Stage::Cleared => {
                let playing = self.stage == Stage::Playing;
                let pack_btn = self.pack_button_rect(o);
                if playing {
                    let c = self.controls(win, pack_btn);
                    let out = self.round.step(dt, &c);
                    match out {
                        Outcome::Running => {}
                        Outcome::Won => {
                            self.progress.cleared = true;
                            self.stage = Stage::Cleared;
                            self.stage_t = 0.0;
                        }
                        Outcome::Lost => {
                            self.stage = Stage::Failed;
                            self.stage_t = 0.0;
                            self.show_hint = !self.hint_shown;
                            self.hint_shown = true;
                        }
                    }
                }
                match &self.round {
                    Round::Eye(round) => self.draw_eye_round(r, o, round, lang, if self.stage == Stage::Cleared { (self.stage_t / 1.2).min(1.0) } else { 0.0 }),
                    Round::Drive(round) => self.draw_drive_round(ctx, r, area, round, lang),
                    Round::Pack(round) => self.draw_pack_round(r, o, round, lang, win),
                }
                // PACK 은 연습이 끝나면 START 를 눌러야 본 작업이 시작한다.
                if let Round::Pack(round) = &self.round
                    && round.phase == Phase::Ready
                    && self.stage == Stage::Playing
                    && button(r, o.0 + W / 2.0 - 60.0, o.1 + 110.0, 120.0, 34.0, t(lang, s::START), win)
                    && let Round::Pack(round) = &mut self.round
                {
                    round.step(0.0, &Controls { act: true, ..Controls::default() });
                }
                if self.stage == Stage::Failed {
                    fill(r, o, 0.0, 90.0, W, 170.0, [0.0, 0.0, 0.0, 0.8]);
                    centered(r, o, 100.0, t(lang, s::FAILED), 1.6, [1.0, 0.4, 0.4, 1.0]);
                    if let Round::Drive(round) = &self.round {
                        centered(r, o, 128.0, &format!("{} {}/{}", t(lang, s::PLACE), round.place(), drive::RACERS), 0.9, [1.0, 1.0, 1.0, 1.0]);
                    }
                    if self.show_hint {
                        let hint = t(lang, self.hint());
                        for (i, line) in crate::ui::wrap_lines(r, hint, 0.8, W - 60.0).iter().enumerate() {
                            r.text(o.0 + 30.0, o.1 + 150.0 + i as f32 * 20.0, line, 0.8, [0.9, 0.9, 0.7, 1.0]);
                        }
                    }
                    if button(r, o.0 + W / 2.0 - 60.0, o.1 + 215.0, 120.0, 34.0, t(lang, s::RETRY), win) {
                        self.round = Round::new(self.kind);
                        self.stage = Stage::Playing;
                        self.stage_t = 0.0;
                    }
                }
                if self.stage == Stage::Cleared {
                    fill(r, o, 0.0, 120.0, W, 90.0, [0.0, 0.0, 0.0, 0.75]);
                    centered(r, o, 140.0, t(lang, s::CLEAR), 2.0, [1.0, 0.95, 0.4, 1.0]);
                    if self.stage_t > 2.2 || (self.stage_t > 0.6 && win.mouse_clicked && panel.contains(win.mouse.0, win.mouse.1)) {
                        self.stage = Stage::Reward;
                        self.stage_t = 0.0;
                    }
                }
            }
            Stage::Reward | Stage::Sent => {
                self.draw_reward_backdrop(r, o);
                let chest = Rect::new(o.0 + 80.0, o.1 + 220.0, 90.0, 64.0);
                let mailbox = Rect::new(o.0 + 330.0, o.1 + 180.0, 60.0, 104.0);
                // 상자 — 꺼내기 전엔 닫혀 있고, 꺼낸 뒤엔 뚜껑이 열린 빈 상자.
                let open = self.progress.reward_taken;
                r.rect(chest.x, chest.y + 18.0, chest.w, chest.h - 18.0, [0.55, 0.36, 0.18, 1.0]);
                r.rect(chest.x, chest.y + if open { 0.0 } else { 8.0 }, chest.w, 14.0, [0.7, 0.48, 0.24, 1.0]);
                r.rect(chest.x + chest.w / 2.0 - 5.0, chest.y + 24.0, 10.0, 10.0, [0.9, 0.8, 0.3, 1.0]);
                // 우체통
                r.rect(mailbox.x, mailbox.y, mailbox.w, mailbox.h, [0.25, 0.3, 0.6, 1.0]);
                r.rect(mailbox.x + 8.0, mailbox.y + 14.0, mailbox.w - 16.0, 8.0, [0.05, 0.05, 0.1, 1.0]);
                let hover_c = chest.contains(win.mouse.0, win.mouse.1);
                let hover_m = mailbox.contains(win.mouse.0, win.mouse.1);
                let can_take = self.stage == Stage::Reward && !self.progress.reward_taken;
                let can_send = self.stage == Stage::Reward && self.progress.reward_taken && !self.progress.reward_sent;
                if (can_take && hover_c) || (can_send && hover_m) {
                    let target = if hover_c { chest } else { mailbox };
                    crate::ui::border(r, target.x - 2.0, target.y - 2.0, target.w + 4.0, target.h + 4.0, [1.0, 1.0, 0.5, 1.0]);
                }
                if win.focused && win.mouse_clicked {
                    if can_take && hover_c {
                        self.progress.reward_taken = true;
                    } else if can_send && hover_m {
                        self.progress.reward_sent = true;
                        self.pending_mail = true;
                        self.stage = Stage::Sent;
                    }
                }
                // 가진 것 — 상자에서 꺼낸 뒤 우체통에 넣기 전까지.
                if self.progress.reward_taken && !self.progress.reward_sent {
                    fill(r, o, 14.0, H - 54.0, 44.0, 44.0, [0.0, 0.0, 0.0, 0.7]);
                    self.draw_item_icon(r, o.0 + 17.0, o.1 + H - 51.0, 38.0);
                    r.text(o.0 + 66.0, o.1 + H - 46.0, &format!("{} {}", t(lang, s::GOT_ITEM), t(lang, self.item_name())), 0.8, [1.0, 1.0, 0.8, 1.0]);
                }
                let msg = if self.stage == Stage::Sent {
                    t(lang, s::REWARD_SENT)
                } else if !self.progress.reward_taken {
                    t(lang, s::REWARD_OPEN)
                } else {
                    t(lang, s::REWARD_MAIL)
                };
                centered(r, o, 20.0, msg, 0.9, [1.0, 1.0, 1.0, 1.0]);
            }
        }

        // 보낸 직후 한 번: 보상 메일을 CRT 로. 그 다음 프레임에 진행 저장.
        if self.pending_mail {
            self.pending_mail = false;
            return AppAction::DeliverMail(self.kind.reward_mail());
        }
        if self.progress != self.saved {
            self.saved = self.progress;
            return AppAction::SaveMini(self.kind, self.progress);
        }
        AppAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(kind: MiniKind, progress: MiniProgress) -> MiniGameApp {
        MiniGameApp::new(kind, progress, Rc::new(RefCell::new(Settings::default())))
    }

    // 저장된 진행으로 다시 열면 이미 한 판을 다시 하지 않고 보상 단계에서 이어진다.
    #[test]
    fn reopening_resumes_at_the_reward() {
        assert!(app(MiniKind::Eye, MiniProgress::default()).stage == Stage::Intro);
        assert!(app(MiniKind::Brain, MiniProgress { cleared: true, ..MiniProgress::default() }).stage == Stage::Reward);
        let sent = MiniProgress { cleared: true, reward_taken: true, reward_sent: true };
        assert!(app(MiniKind::Finger, sent).stage == Stage::Sent);
    }

    #[test]
    fn each_game_has_its_own_mails_and_files() {
        for kind in MiniKind::ALL {
            assert_ne!(kind.task_mail(), kind.reward_mail());
            assert!(kind.setup_name().starts_with(kind.app_name()) && kind.exe_name().starts_with(kind.app_name()));
        }
    }
}
