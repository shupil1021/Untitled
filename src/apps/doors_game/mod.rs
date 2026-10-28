//! 메인 게임 "DOORS" — 친구가 메일로 보낸 크랙 게임(STORY.md 7-1절). 메일 첨부
//! `test Setup.exe`로 설치(apps/game_installer.rs)하면 바탕화면에 생기는
//! `test.exe`(FileKind::Game)를 열었을 때 다른 앱들처럼 PalaceOS 안의 창 하나로
//! 뜬다(별도 실행 파일/OS 창이 아니다). 화면/시점/플레이어/대화창은 apps/game3d 의
//! 공용 도구를 쓴다.
//!
//! 1장(시작방, world.rs): 사방이 막힌 방 하나, 정면 벽에 닫힌 문 하나. 그 문이 도어즈(Doors,
//! 말하는 문 NPC)다 — 손잡이를 조준하고 `E`를 누르면 보리지꽃을 달라고 하고 `[Y] 수락 / [N] 거부`
//! 선택지가 뜬다. 수락하면 책상 위에 편지가 나타나고, 편지 → 우편함 → OS 메일 → 문의 대사 변경 →
//! 방에 꽃이 없다는 확인 → 두 번째 메일로 이어진다. 서브 게임(maze_game)에서 보낸 꽃이 OS 에서
//! 내려받아지면 이 게임을 열 때 꽃이 손에 들어와 있고, 문에게 꽃을 주면 자물쇠가 풀려(문이 위로
//! 올라간다) 1장 튜토리얼이 끝난다. 열린 문으로 나가면 2층이다.
//!
//! 2층(floor2.rs, rooms.rs): 도시 한복판의 정글 — 문 5개가 길 한복판에 서 있고, 한구석에 소파가 놓인
//! 작은 거실이 있다. 계단 입구의 계단 NPC 는 Mr. POTATO CHIP 을 먹고 싶어 하고, 소파 NPC 는 눈 / 뇌 /
//! 손가락 재료 셋과 그걸 바꿔 준다. 재료는 미션 방(눈 / 뇌 / 손가락)에서 구한다: NPC 의 의뢰를 듣고 →
//! 기계에서 "게임 받기"(OS 로 설치 파일 메일이 간다) → OS 에서 설치해 미니게임(apps/minigame)을 하고 보상을
//! 우체통에 넣으면 → 보상 사진 메일이 오고 → 그 사진을 내려받은 뒤 이 게임으로 돌아오면 재료가 들어온다.
//!
//! 조작: 게임 화면을 클릭하면 마우스 시점 모드, `Esc` 나 다른 창으로 포커스를 옮기면
//! 풀린다. W/S/A/D 이동, E 상호작용(창이 포커스일 때만). 포커스가 아닌 동안은 게임 시간이 멈춘다
//! (CRT 에서 한 번에 한 게임만 진행한다 — 시트의 CRT SYS-1).

mod bgm;
mod floor2;
mod furniture;
mod interact;
mod rooms;
mod scene;
mod sfx;
mod texts;
mod travel;
mod world;

// 개발용 미리보기(examples/room_preview.rs) — 시작 방의 상자들과 가구 모델.
pub fn preview_start_room() -> (Vec<crate::render::mesh3d::Box3D>, Vec<crate::render::mesh3d::MeshObject>) {
    (world::room_visible(0.0), furniture::room_meshes(0.0))
}

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{DoorsProgress, MailId, MainPlace, MiniKind, MissionStep, Settings};
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::player::Player;
use super::game3d::{Fade, MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use floor2::Gate;
use scene::Scene;
use rooms::Spot;
use texts::item_name;
use world::{Target, DOOR_LIFT, EXIT_POS, HANDLE_POS, PROPS, SPAWN};

const ROOM_CLEAR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];
const DOOR_OPEN_SECS: f32 = 1.4; // 자물쇠가 풀리고 문이 올라가는 데 걸리는 시간
const UNFOLD_SECS: f32 = 2.0; // 계단이 몸을 펼치는 데 걸리는 시간
const FEED_OPTIONS: &str = "[Y] 먹인다   [N] 나중에";
const EXCHANGE_OPTIONS: &str = "[Y] 교환하기   [N] 나중에";
const RESEND_OPTIONS: &str = "[Y] 다시 받기   [N] 괜찮아";

// 지금 조준선이 향한 대상.
#[derive(Clone, Copy, PartialEq)]
enum Aim {
    Room(Target),          // 시작방
    Jungle(floor2::Target), // 정글
    Spot(Spot),            // 미션 방 / 계단 입구
}

// 대화 끝에 [Y]/[N] 으로 답하는 질문이 무엇인지 — None 이면 도어즈의 첫 부탁(보리지꽃)이다.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ask {
    Errand,   // 소파 NPC 의 재료 의뢰 수락
    Exchange, // 재료 셋을 건네 Mr. POTATO CHIP 과 바꾸기
    Feed,     // 계단 NPC 에게 Mr. POTATO CHIP 먹이기
    Knight,   // 화장실 기사의 칼 찾기 의뢰 수락(MAIN H-1)
    Resend(MiniKind), // 받은 게임/보상 파일을 잃어버렸을 때 "다시 받기"(CRT SYS-0)
}

pub struct DoorsGameApp {
    player: Player,
    slot: RenderSlot, // 이 창 전용 3D 렌더 타깃(창끼리 화면이 섞이지 않게)
    look: MouseLook,
    aimed: Option<Aim>,
    dialogue: Dialogue,
    // 진행 상황 — 시트 오른쪽 표: 수락하면 책상 위에 편지가 나타난다 → 집으면(has_letter) 문이 쓴 편지를
    // 읽게 되고 → 우편함에 넣으면(letter_sent) OS 로 같은 내용의 메일이 간다 → 그 메일을 읽고 게임으로
    // 돌아오면(door_changed) 문의 대사가 달라지고 방 물건을 조사해 꽃이 없다는 걸 깨달을 수 있다.
    // 바뀔 때마다 OS 에 저장을 요청해서(saved 와 비교) 창을 닫았다 다시 열어도 이어진다.
    progress: DoorsProgress,
    saved: DoorsProgress,
    letter_notify_pending: bool, // 편지를 넣은 걸 OS 에 아직 못 알렸다(대화가 끝나면 알린다)
    second_mail_sent: bool, // 그 확인을 OS 에 알려서 두 번째 메일을 요청했는지(한 번만)
    pending_mail: Option<MailId>, // 대화가 끝나면 OS 에 도착시켜 달라고 할 메일
    pending_resend: Option<MiniKind>, // 대화가 끝나면 OS 에 다시 받게 해 달라고 할 미니게임
    ask: Option<Ask>,            // 지금 떠 있는 [Y]/[N] 질문(None 이면 도어즈의 부탁)
    // 장소 이동: 대화가 다 끝나면 화면을 덮었다가 장면을 바꾼다.
    fade: Fade,
    pending_move: Option<MainPlace>,
    entry_gate: Option<Gate>, // 정글로 나올 때 어느 문 앞에 서는지(None 이면 시작방 문 앞)
    // 문(도어즈)에게 꽃을 준 대화가 끝나면 자물쇠가 풀리고 문이 위로 올라간다(MAIN A-5).
    unlock_pending: bool,
    door_lift: f32,
    door_anim: Option<f32>, // 문이 올라가는 중이면 그 시작부터 흐른 시간
    // 계단이 몸을 펼치는 연출(0 = 웅크림, 1 = 다 펼쳐짐). Mr. POTATO CHIP 을 먹이면 시작한다.
    unfold: f32,
    unfold_anim: Option<f32>,
    clock: f32,
    was_focused: bool,
    // 정글은 한 번만 만들어 둔다(나무가 많다).
    jungle: Vec<Box3D>,
    barriers: Vec<Box3D>,
    scene: Option<Rc<Scene>>, // 지금 장소의 장면(바뀔 때만 다시 만든다 — scene.rs)
    bgm: bgm::Bgm,            // 시작 방 배경음악(무한 반복)
    sfx: sfx::Sfx,            // 퀘스트 시작 / 물 내리기 효과음
    sfx_cue: Option<sfx::Cue>, // 이번 프레임에 틀 효과음
    settings: Option<Rc<RefCell<Settings>>>, // 소리 크기 설정(OS 에서 열 때만 있다)
}

impl DoorsGameApp {
    // 소리 크기 설정을 연결한다(배경음악 볼륨).
    pub(super) fn with_settings(mut self, settings: Rc<RefCell<Settings>>) -> DoorsGameApp {
        self.settings = Some(settings);
        self
    }

    pub(super) fn new(progress: DoorsProgress) -> DoorsGameApp {
        let place = progress.place;
        // 처음 열었을 때만 어디에 서 있는지 한마디로 알려준다(이어서 하는 거면 생략).
        let mut dialogue = Dialogue::new();
        if progress == DoorsProgress::default() {
            dialogue.start(vec![Entry::Line("낡은 방 안이다.".to_string()), Entry::Line("정면에 문이 하나 있다.".to_string())]);
        }
        let (feet, yaw) = match place {
            MainPlace::StartRoom => (SPAWN, 0.0),
            MainPlace::Jungle => (floor2::SPAWN, floor2::SPAWN_YAW),
            MainPlace::MissionRoom(_) | MainPlace::StairHall => (rooms::SPAWN, 0.0),
        };
        // 창이 뒤로 가기 직전에 서 있던 자리가 저장돼 있으면 거기서 이어서 시작한다.
        let player = match progress.pose {
            Some([x, y, z, yaw]) => Player::at([x, y, z], yaw),
            None => Player::at(feet, yaw),
        };
        let (jungle, barriers) = if place == MainPlace::Jungle { (Self::jungle_scene(&progress), floor2::barriers()) } else { (Vec::new(), Vec::new()) };
        DoorsGameApp {
            bgm: bgm::Bgm::default(),
            sfx: sfx::Sfx::default(),
            sfx_cue: None,
            settings: None,
            player,
            slot: RenderSlot::acquire(),
            look: MouseLook::new(),
            aimed: None,
            dialogue,
            // 이어서 하는 거면 이미 보낸 편지/꽃 위치 확인을 OS 에 다시 한 번 알린다(메일은 중복으로 안 온다).
            letter_notify_pending: progress.letter_sent,
            saved: progress.clone(),
            door_lift: if progress.tutorial_done { DOOR_LIFT } else { 0.0 },
            unfold: if progress.chip_fed { 1.0 } else { 0.0 },
            progress,
            second_mail_sent: false,
            pending_mail: None,
            pending_resend: None,
            ask: None,
            fade: Fade::new(),
            pending_move: None,
            entry_gate: None,
            unlock_pending: false,
            door_anim: None,
            unfold_anim: None,
            clock: 0.0,
            was_focused: true,
            jungle,
            barriers,
            scene: None,
        }
    }

    fn jungle_scene(p: &DoorsProgress) -> Vec<Box3D> {
        floor2::build_scene(MiniKind::ALL.map(|k| p.missions[k.index()].has_material(k)), p.exchanged)
    }

    fn line(text: &str) -> Entry {
        Entry::Line(text.to_string())
    }

    fn lines(texts: &[&str]) -> Vec<Entry> {
        texts.iter().map(|t| Self::line(t)).collect()
    }

    // 지금 가진 재료 개수(교환하기 전까지).
    fn ingredients(&self) -> usize {
        if self.progress.exchanged {
            return 0;
        }
        MiniKind::ALL.iter().filter(|&&k| self.progress.missions[k.index()].has_material(k)).count()
    }

    fn has_chip(&self) -> bool {
        self.progress.chip_taken && !self.progress.chip_fed
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        if self.look.update(win, in_view) || self.fade.active() {
            return;
        }
        if self.dialogue.active() {
            if let Some(yes) = self.dialogue.handle_input(win, in_view) {
                self.answer(yes);
            }
            return;
        }
        if win.input.pressed(KeyCode::E) {
            match self.aimed {
                Some(Aim::Room(t)) => self.interact_room(Some(t)),
                Some(Aim::Jungle(t)) => self.interact_jungle(t),
                Some(Aim::Spot(s)) => self.interact_spot(s),
                None => {}
            }
        }
    }

    // [Y]/[N] 로 질문에 답했다.
    fn answer(&mut self, yes: bool) {
        match self.ask.take() {
            // 도어즈의 첫 부탁 — 수락하면 퀘스트 수락만 기록한다.
            None => {
                if yes {
                    self.progress.quest_accepted = true;
                    self.dialogue.start(vec![
                        Self::line("문 너머에서 작은 한숨이 새어 나왔다."),
                        // 다음에 뭘 해야 하는지 — 책상 위에 편지가 나타난 걸 짚어준다.
                        Self::line("그러고 보니 책상 위에 편지가 한 통 놓여 있다."),
                    ]);
                } else {
                    self.dialogue.start(vec![Self::line("문은 아무 말도 하지 않았다.")]);
                }
            }
            // 소파 NPC 의 의뢰(MAIN B-3) — 수락하면 미션 문 셋이 열리고 장보기 목록 메일이 온다.
            Some(Ask::Errand) => {
                if yes {
                    self.progress.errand_accepted = true;
                    self.pending_mail = Some(MailId::Shopping);
                    self.dialogue.start(vec![Self::line("소파가 느긋하게 몸을 묻었다."), Self::line("멀리서 문 세 개가 열리는 소리가 들렸다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("소파는 느긋하게 기다리고 있다.")]);
                }
            }
            // 재료 셋과 Mr. POTATO CHIP 교환(MAIN C-1).
            Some(Ask::Exchange) => {
                if yes {
                    self.progress.exchanged = true;
                    self.jungle = Self::jungle_scene(&self.progress);
                    self.dialogue.start(vec![
                        Self::line("눈과 소환 조각 둘을 한꺼번에 소파에게 건넸다."),
                        Self::line("소파의 빈 자리가 한 칸 늘어났다."),
                        Self::line("소파 위에 봉투가 하나 놓여 있다."),
                    ]);
                } else {
                    self.dialogue.start(vec![Self::line("소파가 하품을 한다.")]);
                }
            }
            // 기사의 칼 찾기 의뢰(MAIN H-1 / H-2) — 수락하면 방귀 + 나팔 연출 뒤 PACK 설치 메일이 간다.
            Some(Ask::Knight) => {
                if yes {
                    self.accept_knight();
                } else {
                    self.dialogue.start(vec![Self::line("기사는 무겁게 한숨을 쉬었다.")]);
                }
            }
            // 다시 받기 — 안내 메일과 설치 파일/보상 사진이 사라졌으면 OS 가 새로 만들어 준다.
            Some(Ask::Resend(kind)) => {
                if yes {
                    self.pending_resend = Some(kind);
                    self.dialogue.start(vec![Self::line("다시 한 번 받았다."), Self::line("컴퓨터에서 확인해 보자.")]);
                } else {
                    self.dialogue.start(vec![Self::line("괜찮다고 했다.")]);
                }
            }
            // 계단에게 Mr. POTATO CHIP 먹이기(MAIN C-4) — 한 개를 쓰고 급식 완료를 같이 저장한다.
            Some(Ask::Feed) => {
                if yes {
                    self.progress.chip_fed = true;
                    self.unfold_anim = Some(0.0);
                    self.dialogue.start(vec![Self::line("계단이 입을 쩍 벌렸다."), Self::line("Mr. POTATO CHIP을 입에 넣어 주었다."), Self::line("계단이 바삭, 하고 한입에 먹었다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("계단은 입을 벌린 채 기다린다.")]);
                }
            }
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
        // 다른 창이 앞에 있는 동안은 게임 시간이 멈춘다 — 한 번에 한 게임만 진행한다.
        let dt = if win.focused { win.dt } else { 0.0 };

        // 편지 메일을 읽고 이 창으로 돌아왔으면 문이 달라지고, 꽃이 내려받아졌으면 꽃이 들어온다.
        self.notice_mail_read(crate::signals::letter_mail_read(), win.focused);
        self.receive_flower(crate::signals::flower_downloaded(), win.focused);
        self.receive_ingredient(MiniKind::ALL.map(|k| crate::signals::reward_downloaded(k.index())), win.focused);
        self.handle_input(win, in_view);
        self.look.request();
        self.dialogue.tick(dt);
        self.advance_scene(dt);
        self.remember_pose_on_blur(win.focused);

        // 지금 장소의 장면 — 바뀐 게 없으면 지난 프레임 것을 그대로 쓴다.
        let key = self.scene_key();
        if self.scene.as_ref().is_none_or(|sc| sc.key != key) {
            self.scene = Some(Rc::new(self.build_scene(key)));
        }
        let scene = Rc::clone(self.scene.as_ref().expect("방금 만들었다"));
        self.bgm.update(self.progress.place == MainPlace::StartRoom, self.settings.as_ref());
        self.sfx.update(self.sfx_cue.take(), dt, self.settings.as_ref());

        // 대화창이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue.active() || self.fade.active() || self.door_anim.is_some() || self.unfold_anim.is_some();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &scene.collide);
            self.aimed = self.compute_aim(&scene);
        }

        let cam = self.player.camera();
        if scene.meshes.is_empty() {
            view.draw_scene(ctx, r, area, &self.slot, scene.clear, &cam, &scene.shown);
        } else {
            view.draw_scene_meshes_aimed(ctx, r, area, &self.slot, scene.clear, &cam, &scene.shown, &scene.meshes);
        }

        if let Some(aim) = self.aimed
            && !frozen
        {
            let found = match aim {
                Aim::Room(Target::Door) if self.progress.tutorial_done => Some((EXIT_POS, "[E] Go out".to_string())),
                Aim::Room(Target::Door) => Some((HANDLE_POS, "[E] Examine".to_string())),
                Aim::Room(Target::Prop(i)) => Some((PROPS[i].center, format!("[E] Examine {}", PROPS[i].name))),
                Aim::Room(Target::Letter) => Some((Target::Letter.pos(), "[E] Take letter".to_string())),
                Aim::Room(Target::Mailbox) => Some((Target::Mailbox.pos(), "[E] Mailbox".to_string())),
                Aim::Jungle(t) => scene.jungle_items.iter().find(|it| it.target == t).map(|it| (it.center, it.label.to_string())),
                Aim::Spot(s) => scene.room_items.iter().find(|it| it.target == s).map(|it| (it.center, it.label.to_string())),
            };
            if let Some((pos, label)) = found {
                view.draw_prompt(r, &cam, pos, &label);
            }
        }
        if !self.look.captured() && !frozen {
            view.draw_capture_hint(r);
        }
        self.draw_inventory(r, &view);
        self.fade.draw(r, &view);
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
        // 대화가 끝나면 이 게임에서 비롯된 메일을 도착시켜 달라고 한다.
        if !self.dialogue.active()
            && let Some(id) = self.pending_mail.take()
        {
            return AppAction::DeliverMail(id);
        }
        if !self.dialogue.active()
            && let Some(kind) = self.pending_resend.take()
        {
            return AppAction::ResendMini(kind);
        }
        // 진행 상황이 바뀌었으면 OS 에 저장을 요청한다.
        if self.progress != self.saved {
            self.saved = self.progress.clone();
            return AppAction::SaveDoors(self.saved.clone());
        }
        AppAction::None
    }
}

impl DoorsGameApp {
    // 왼쪽 위에 가진 것들을 한 줄씩.
    fn draw_inventory(&self, r: &mut Renderer, view: &View) {
        let p = &self.progress;
        let mut lines: Vec<&str> = Vec::new();
        if p.flower_received && !p.flower_given {
            lines.push("보리지꽃");
        }
        if !p.exchanged {
            for kind in MiniKind::ALL {
                if let Some(name) = carried_item(kind, p.missions[kind.index()]) {
                    lines.push(name);
                }
            }
        }
        if self.has_chip() {
            lines.push("Mr. POTATO CHIP");
        }
        for (i, line) in lines.iter().enumerate() {
            r.text(view.px(10.0), view.py(10.0 + i as f32 * 20.0), line, 0.8 * view.s, [0.75, 0.8, 1.0, 1.0]);
        }
    }
}

// 지금 손에 들고 있는 그 미션의 물건 — 눈은 받자마자, 레이싱/리듬은 중간 물건(금고 절반 / 변기 손잡이)을 받았다가
// 조립하면 사라지고 소환 조각이 들어온다.
fn carried_item(kind: MiniKind, step: MissionStep) -> Option<&'static str> {
    match (kind, step) {
        (MiniKind::Brain, MissionStep::Received) => Some("금고 절반"),
        (MiniKind::Finger, MissionStep::Received) => Some("변기 손잡이"),
        _ if step.has_material(kind) => Some(item_name(kind)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
