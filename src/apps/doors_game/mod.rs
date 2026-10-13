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

mod floor2;
mod rooms;
mod world;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{DoorsProgress, MailId, MainPlace, MiniKind, MissionStep};
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::ena::{ena_time, smooth};
use super::game3d::player::{aim_dist, Player};
use super::game3d::{Fade, MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use floor2::Gate;
use rooms::Spot;
use world::{aimed_target, build_room, letter_box, Target, DOOR_LIFT, EXIT_POS, HANDLE_POS, PROPS, SPAWN};

const ROOM_CLEAR: [f32; 4] = [0.02, 0.02, 0.03, 1.0];
const DOOR_OPEN_SECS: f32 = 1.4; // 자물쇠가 풀리고 문이 올라가는 데 걸리는 시간
const UNFOLD_SECS: f32 = 2.0; // 계단이 몸을 펼치는 데 걸리는 시간
const FEED_OPTIONS: &str = "[Y] 먹인다   [N] 나중에";
const EXCHANGE_OPTIONS: &str = "[Y] 교환하기   [N] 나중에";

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
}

// 미션 방의 대사 모음 — 세 방이 같은 흐름이라 문구만 다르다.
struct MissionText {
    talk_first: &'static [&'static str],   // E-1: 처음 말을 걸었을 때(의뢰)
    talk_waiting: &'static [&'static str], // 의뢰를 듣고 아직 재료를 못 받았을 때
    machine_locked: &'static [&'static str], // 의뢰를 듣기 전에 기계를 눌렀을 때
    machine_send: &'static [&'static str], // E-2: 기계에서 게임을 받았다
    machine_waiting: &'static [&'static str],
    received: &'static [&'static str],     // E-3: 보상이 손에 들어왔다
    talk_received: &'static [&'static str], // E-4: 재료를 받은 뒤 NPC 가 달라진다
    talk_done: &'static [&'static str],    // 끝난 방을 다시 찾았을 때
    machine_done: &'static [&'static str],
}

fn mission_text(kind: MiniKind) -> &'static MissionText {
    static EYE: MissionText = MissionText {
        talk_first: &["렌즈 얼굴이 천천히 이쪽을 돌아본다.", "계속 피하기만 하면 돼. 내가 대신 깜빡여 줄게.", "그는 구석의 아케이드 기계를 가리켰다."],
        talk_waiting: &["저 기계에서 게임을 받아."],
        machine_locked: &["아케이드 기계는 꺼져 있다.", "먼저 렌즈 얼굴과 이야기해 보자."],
        machine_send: &["아케이드 기계의 '게임 받기'를 눌렀다.", "어디선가 알림음이 울린다..."],
        machine_waiting: &["이미 게임을 받았다.", "컴퓨터에서 확인해 보자."],
        received: &["어디선가 보낸 충혈된 눈이 손에 쥐어져 있다.", "'충혈된 눈'을 얻었다."],
        talk_received: &["이제 나는 쉬어도 되겠네.", "렌즈가 꺼졌는데, 시선은 출구를 따라온다."],
        talk_done: &["...쉬는 중이야."],
        machine_done: &["아케이드 기계는 조용하다."],
    };
    static BRAIN: MissionText = MissionText {
        talk_first: &["바닥의 도로 위에 표지판이 서 있다.", "집까지 가는 길이야. 목적지만 바뀌지 않으면 돼.", "표지판이 운전대 모양의 기계 쪽을 가리킨다."],
        talk_waiting: &["저 기계에서 게임을 받아."],
        machine_locked: &["운전대 기계는 꺼져 있다.", "먼저 표지판과 이야기해 보자."],
        machine_send: &["운전대 기계의 '게임 받기'를 눌렀다.", "어디선가 알림음이 울린다..."],
        machine_waiting: &["이미 게임을 받았다.", "컴퓨터에서 확인해 보자."],
        received: &["어디선가 보낸 짓무른 뇌가 손에 쥐어져 있다.", "'짓무른 뇌'를 얻었다."],
        talk_received: &["돌아왔는데, 정말 같은 곳일까?", "창밖 풍경이 멈췄다. 방은 그대로다."],
        talk_done: &["...여기가 집이었나."],
        machine_done: &["운전대 기계는 조용하다."],
    };
    static FINGER: MissionText = MissionText {
        talk_first: &["장갑을 낀 담당자가 이쪽을 본다.", "손은 빌려줄게. 박자만 네가 맞춰.", "그는 포장 기계의 대기 버튼을 턱짓으로 가리켰다."],
        talk_waiting: &["저 기계 버튼을 눌러."],
        machine_locked: &["포장 기계는 멈춰 있다.", "먼저 담당자와 이야기해 보자."],
        machine_send: &["포장 기계의 대기 버튼을 눌렀다.", "어디선가 알림음이 울린다..."],
        machine_waiting: &["이미 버튼을 눌렀다.", "컴퓨터에서 확인해 보자."],
        received: &["어디선가 보낸 절단된 손가락이 손에 쥐어져 있다.", "'절단된 손가락'을 얻었다."],
        talk_received: &["손님이 손님을 만들었네.", "멈춘 기계가 한 번만 박자를 쳤다."],
        talk_done: &["...다음 손님은 아직이야."],
        machine_done: &["포장 기계는 멈춰 있다."],
    };
    match kind {
        MiniKind::Eye => &EYE,
        MiniKind::Brain => &BRAIN,
        MiniKind::Finger => &FINGER,
    }
}

fn gate_of(kind: MiniKind) -> Gate {
    match kind {
        MiniKind::Eye => Gate::Eye,
        MiniKind::Brain => Gate::Brain,
        MiniKind::Finger => Gate::Finger,
    }
}

fn item_name(kind: MiniKind) -> &'static str {
    match kind {
        MiniKind::Eye => "충혈된 눈",
        MiniKind::Brain => "짓무른 뇌",
        MiniKind::Finger => "절단된 손가락",
    }
}

fn door_name(kind: MiniKind) -> &'static str {
    match kind {
        MiniKind::Eye => "눈 표식 문",
        MiniKind::Brain => "뇌 표식 문",
        MiniKind::Finger => "손가락 표식 문",
    }
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
}

impl DoorsGameApp {
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
        }
    }

    fn jungle_scene(p: &DoorsProgress) -> Vec<Box3D> {
        floor2::build_scene(MiniKind::ALL.map(|k| p.missions[k.index()] == MissionStep::Done), p.exchanged)
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
        self.progress.missions.iter().filter(|&&m| m >= MissionStep::Received).count()
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
                        Self::line("눈, 뇌, 손가락을 한꺼번에 소파에게 건넸다."),
                        Self::line("소파의 빈 자리가 한 칸 늘어났다."),
                        Self::line("소파 위에 봉투가 하나 놓여 있다."),
                    ]);
                } else {
                    self.dialogue.start(vec![Self::line("소파가 하품을 한다.")]);
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

    // 편지를 우편함에 넣은 뒤 OS 에서 그 편지 메일을 읽었고(mail_read) 이 창으로 돌아왔으면(focused)
    // 문이 달라진다 — 이때부터 문의 대사가 바뀐다. 게임은 그동안 멈추지 않고 그대로 돌아간다.
    fn notice_mail_read(&mut self, mail_read: bool, focused: bool) -> bool {
        if self.progress.letter_sent && !self.progress.door_changed && mail_read && focused && !self.dialogue.active() {
            self.progress.door_changed = true;
            self.dialogue.start(vec![Self::line("문 쪽에서 인기척이 느껴진다...")]);
            return true;
        }
        false
    }

    // 서브 게임이 보낸 꽃이 OS 에서 내려받아졌으면 이 게임을 다시 열었을 때(창이 앞에 왔을 때) 꽃이 손에
    // 들어와 있다(SUB A+a-20). 시작방에서만, 대화 중이 아닐 때.
    fn receive_flower(&mut self, downloaded: bool, focused: bool) -> bool {
        let p = &mut self.progress;
        if downloaded
            && focused
            && !p.flower_received
            && p.place == MainPlace::StartRoom
            && !self.dialogue.active()
            && !self.fade.active()
        {
            p.flower_received = true;
            self.dialogue.start(vec![Self::line("어디선가 보낸 꽃이 도착했다."), Self::line("손에 보리지꽃이 쥐어져 있다.")]);
            return true;
        }
        false
    }

    // 미니게임 보상 사진을 내려받고 이 창으로 돌아오면 재료가 자동으로 들어온다(MAIN E-3 / R-3 / H-3).
    // 어디에 있든(방이든 정글이든) 받는다 — 중간에 나갔다 와도 수령 대기가 복구된다(CRT SYS-2).
    fn receive_ingredient(&mut self, downloaded: [bool; 3], focused: bool) -> bool {
        if !focused || self.dialogue.active() || self.fade.active() || self.pending_move.is_some() {
            return false;
        }
        for kind in MiniKind::ALL {
            let i = kind.index();
            if downloaded[i] && self.progress.missions[i] == MissionStep::Sent {
                self.progress.missions[i] = MissionStep::Received;
                self.dialogue.start(Self::lines(mission_text(kind).received));
                return true;
            }
        }
        false
    }

    // E — 시작방에서 조준 중인 대상을 조사한다.
    fn interact_room(&mut self, target: Option<Target>) {
        match target {
            // 자물쇠가 풀린 문 — 열린 문간의 빛 속으로 걸어 나간다(MAIN B-0).
            Some(Target::Door) if self.progress.tutorial_done => {
                self.dialogue.start(vec![Self::line("열린 문 너머로 환한 빛이 새어 들어온다."), Self::line("그 빛 속으로 걸어 나갔다...")]);
                self.entry_gate = None;
                self.pending_move = Some(MainPlace::Jungle);
            }
            // 꽃을 가지고 문에게 말을 걸면 꽃을 건네고, 문의 마지막 대사 뒤에 자물쇠가 풀린다(MAIN A-4/A-5).
            Some(Target::Door) if self.progress.flower_received && !self.progress.flower_given => {
                self.progress.flower_given = true;
                self.unlock_pending = true;
                self.dialogue.start(vec![
                    Self::line("문고리 앞에 보리지꽃을 내밀었다."),
                    Self::line("문고리에서 낮은 목소리가 들려왔다."),
                    Self::line("꽃은 받았어. 바깥에서도 이름을 잘 기억해."),
                    Self::line("철컥, 하고 문 안쪽에서 자물쇠가 풀리는 소리가 났다."),
                ]);
            }
            // 문이 편지를 읽은 뒤엔 달라진 말을 한다(시트의 "도어즈 대사 변경").
            Some(Target::Door) if self.progress.door_changed => {
                self.dialogue.start(vec![
                    Self::line("문고리에서 낮은 목소리가 들려왔다."),
                    Self::line("편지는 잘 읽었어."),
                    Self::line("꽃은... 조금만 더 기다려 줘."),
                ]);
            }
            // 도어즈는 보리지꽃을 달라는 말만 한다. 수락한 직후엔 문이 말이 없다(available 에서 걸러진다).
            Some(Target::Door) if !self.progress.quest_accepted => {
                // 문이 말을 걸어오는 걸 갑자기 시작하지 않고 먼저 알려준다.
                self.ask = None;
                self.dialogue.start(vec![Self::line("문고리에서 낮은 목소리가 들려왔다."), Entry::Choice("보리지꽃을 줘.".to_string())]);
            }
            // 방 물건은 그 물건의 한 줄을 보여준다. 부탁을 수락한 뒤 처음 조사하면 이어서
            // 방에 꽃이 없다는 걸 깨닫는다.
            Some(Target::Prop(i)) => {
                let mut lines = vec![Self::line(PROPS[i].text)];
                if self.progress.quest_accepted && self.progress.door_changed && !self.progress.flower_absence_checked {
                    self.progress.flower_absence_checked = true;
                    lines.push(Self::line("방에는 꽃이 없는 것 같다..."));
                    // 이어서 오는 메일(두 번째 메일)을 눈치채게 한다.
                    lines.push(Self::line("그때 어디선가 희미한 알림음이 울렸다."));
                }
                self.dialogue.start(lines);
            }
            // 책상 위의 편지 — 집으면 문이 쓴 편지의 내용이 그대로 나온다(OS 로 가는 메일과 같은 글).
            Some(Target::Letter) => {
                self.progress.has_letter = true;
                let mut lines = vec![Self::line("편지를 집어 들었다.")];
                lines.extend(crate::strings::mail::LETTER_MAIL_BODY.ko.split('\n').map(Self::line));
                self.dialogue.start(lines);
            }
            // 우편함 — 편지를 넣으면 OS 로 같은 내용의 메일이 가고, 게임은 메일을 읽고 돌아올 때까지 멈춘다.
            Some(Target::Mailbox) => {
                if self.progress.has_letter && !self.progress.letter_sent {
                    self.progress.letter_sent = true;
                    self.letter_notify_pending = true;
                    self.dialogue.start(vec![Self::line("편지를 우편함에 넣었다."), Self::line("어디선가 알림음이 울린다...")]);
                } else if self.progress.letter_sent {
                    self.dialogue.start(vec![Self::line("우편함은 비어 있다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("우편함이 있다. 지금은 넣을 게 없다.")]);
                }
            }
            _ => {}
        }
    }

    // E — 2층 정글에서 조준 중인 대상을 조사한다.
    fn interact_jungle(&mut self, target: floor2::Target) {
        match target {
            floor2::Target::Sign => {
                self.dialogue.start(vec![Self::line("낡은 표지판이다. 글씨가 거의 지워졌다."), Self::line("'어디로든 문'이라고 쓰여 있다.")]);
            }
            floor2::Target::Gate(Gate::Back) => {
                self.dialogue.start(vec![Self::line("방금 나온 문이다. 안쪽으로 낡은 방이 보인다."), Self::line("문 안으로 되돌아간다...")]);
                self.pending_move = Some(MainPlace::StartRoom);
            }
            // 계단 입구문 — 열고 들어가면 계단 NPC 가 있다(MAIN B-2).
            floor2::Target::Gate(Gate::Stairs) => {
                self.pending_move = Some(MainPlace::StairHall);
            }
            // 미션 문 — 소파 NPC 의 재료 의뢰를 수락해야 열린다(MAIN B-3).
            floor2::Target::Gate(gate) => {
                let kind = match gate {
                    Gate::Eye => MiniKind::Eye,
                    Gate::Brain => MiniKind::Brain,
                    _ => MiniKind::Finger,
                };
                if self.progress.errand_accepted {
                    self.pending_move = Some(MainPlace::MissionRoom(kind));
                } else {
                    self.dialogue.start(vec![Self::line("문은 굳게 닫혀 있다."), Self::line("지금은 열 수 없다. 소파에게 물어보면 알 수 있을까?")]);
                }
            }
            floor2::Target::Sofa => self.talk_sofa(),
            // 소파 위 봉투 — 말하는 Mr. POTATO CHIP 이 들어 있다(MAIN C-2).
            floor2::Target::Envelope => {
                self.progress.chip_taken = true;
                self.pending_mail = Some(MailId::Meal);
                self.dialogue.start(vec![
                    Self::line("봉투를 열었다. 안에서 과자 봉지가 얼굴을 내밀었다."),
                    Self::line("안녕하세요. 저는 간식입니다."),
                    Self::line("'Mr. POTATO CHIP'을 얻었다."),
                ]);
            }
        }
    }

    // 소파 NPC — 의뢰(B-3), 부족한 재료 안내(B-6), 교환(C-0/C-1).
    fn talk_sofa(&mut self) {
        let p = &self.progress;
        if p.chip_taken {
            self.dialogue.start(vec![Self::line("소파가 기지개를 켠다."), Self::line("맛있게 먹여 줘.")]);
        } else if p.exchanged {
            self.dialogue.start(vec![Self::line("소파 위의 봉투를 가져가.")]);
        } else if !p.errand_accepted {
            if !p.stair_met {
                self.dialogue.start(vec![Self::line("소파가 눈을 가늘게 뜬다."), Self::line("계단 쪽에서 왔어? 아직 아무 얘기도 못 들은 얼굴인데.")]);
            } else {
                self.ask = Some(Ask::Errand);
                self.dialogue.start(vec![
                    Self::line("Mr. POTATO CHIP? 있지. 공짜는 아니야."),
                    Self::line("충혈된 눈, 짓무른 뇌, 절단된 손가락. 하나씩 가져와."),
                    Entry::Choice("의뢰를 받아들일래?".to_string()),
                ]);
            }
        } else if self.ingredients() == 3 {
            self.ask = Some(Ask::Exchange);
            self.dialogue.start(vec![
                Self::line("전부 가져왔어."),
                Self::line("눈, 뇌, 손가락... 하나씩 다 있다."),
                Entry::Ask("Mr. POTATO CHIP으로 바꿀까?".to_string(), EXCHANGE_OPTIONS),
            ]);
        } else {
            // 부족한 재료와 그 문의 표식을 알려준다. 가진 재료는 그대로 둔다.
            let mut lines = vec![Self::line("아직 모자라.")];
            for kind in MiniKind::ALL {
                if self.progress.missions[kind.index()] < MissionStep::Received {
                    lines.push(Self::line(&format!("{}이 필요해. {}으로 가 봐.", item_name(kind), door_name(kind))));
                }
            }
            self.dialogue.start(lines);
        }
    }

    // E — 미션 방 / 계단 입구에서 조준 중인 대상을 조사한다.
    fn interact_spot(&mut self, spot: Spot) {
        match self.progress.place {
            MainPlace::MissionRoom(kind) => self.interact_mission(kind, spot),
            MainPlace::StairHall => self.interact_stairs(spot),
            _ => {}
        }
    }

    fn interact_mission(&mut self, kind: MiniKind, spot: Spot) {
        let text = mission_text(kind);
        let i = kind.index();
        let step = self.progress.missions[i];
        match spot {
            Spot::Exit => {
                self.entry_gate = Some(gate_of(kind));
                self.pending_move = Some(MainPlace::Jungle);
            }
            Spot::Npc => match step {
                MissionStep::Start => {
                    self.progress.missions[i] = MissionStep::Talked;
                    self.dialogue.start(Self::lines(text.talk_first));
                }
                MissionStep::Talked | MissionStep::Sent => self.dialogue.start(Self::lines(text.talk_waiting)),
                MissionStep::Received => {
                    self.progress.missions[i] = MissionStep::Done;
                    self.dialogue.start(Self::lines(text.talk_received));
                }
                MissionStep::Done => self.dialogue.start(Self::lines(text.talk_done)),
            },
            Spot::Machine => match step {
                MissionStep::Start => self.dialogue.start(Self::lines(text.machine_locked)),
                // 게임 받기 — OS 로 설치 파일이 첨부된 메일이 간다(MAIN E-2). 지금 서 있는 자리도 같이 저장된다.
                MissionStep::Talked => {
                    self.progress.missions[i] = MissionStep::Sent;
                    self.pending_mail = Some(kind.task_mail());
                    self.progress.pose = Some(self.player.pose());
                    self.dialogue.start(Self::lines(text.machine_send));
                }
                MissionStep::Sent => self.dialogue.start(Self::lines(text.machine_waiting)),
                MissionStep::Received | MissionStep::Done => self.dialogue.start(Self::lines(text.machine_done)),
            },
            _ => {}
        }
    }

    fn interact_stairs(&mut self, spot: Spot) {
        match spot {
            Spot::Exit => {
                self.entry_gate = Some(Gate::Stairs);
                self.pending_move = Some(MainPlace::Jungle);
            }
            // 계단 NPC — 처음엔 먹고 싶다고 하고(B-2), 빈손이면 돌려보내고(B-4), 과자를 가졌으면 먹여 달라 한다(C-3).
            Spot::Npc => {
                if !self.progress.stair_met {
                    self.progress.stair_met = true;
                    self.dialogue.start(vec![
                        Self::line("계단이 몸을 웅크린 채 길을 막고 있다."),
                        Self::line("계단 자체가 입을 열고 말했다."),
                        Self::line("배고파. Mr. POTATO CHIP이 먹고 싶어."),
                    ]);
                } else if self.has_chip() {
                    self.ask = Some(Ask::Feed);
                    self.dialogue.start(vec![Self::line("미스터가 붙은 것만 먹어. 예의가 있잖아."), Entry::Ask("계단이 입을 벌리고 있다.".to_string(), FEED_OPTIONS)]);
                } else if self.progress.errand_accepted {
                    self.dialogue.start(vec![Self::line("빈손은 바삭하지 않아.")]);
                } else {
                    self.dialogue.start(vec![Self::line("Mr. POTATO CHIP이 먹고 싶어.")]);
                }
            }
            // 펼쳐진 계단의 첫 칸 — 2층은 여기까지다(실제 하강은 다음 구간).
            Spot::StairStep => {
                if !self.progress.floor2_done {
                    self.progress.floor2_done = true;
                    self.dialogue.start(vec![Self::line("계단: 잘 먹었습니다. 이제 내려가세요."), Self::line("계단이 아래층으로 길게 이어져 있다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("계단 아래에서 희미한 소리가 올라온다..."), Self::line("아직 내려갈 준비가 되지 않았다.")]);
                }
            }
            _ => {}
        }
    }

    // 장소를 옮긴다 — 화면이 가려진 순간에 부른다.
    fn go(&mut self, to: MainPlace) {
        self.progress.place = to;
        self.progress.pose = None;
        self.aimed = None;
        let mut start = (rooms::SPAWN, 0.0);
        match to {
            MainPlace::StartRoom => {
                start = (SPAWN, 0.0);
                self.jungle = Vec::new();
                self.barriers = Vec::new();
            }
            MainPlace::Jungle => {
                start = match self.entry_gate.take() {
                    Some(gate) => (floor2::gate_front(gate), floor2::SPAWN_YAW),
                    None => (floor2::SPAWN, floor2::SPAWN_YAW),
                };
                self.jungle = Self::jungle_scene(&self.progress);
                self.barriers = floor2::barriers();
                // 처음 나왔을 때만: 갑자기 바뀐 풍경을 말로 풀어주고 메일("밖이라고?")이 온다(MAIN B-1).
                if !self.progress.jungle_visited {
                    self.progress.jungle_visited = true;
                    self.dialogue.start(vec![
                        Self::line("문 밖은 도시 한복판의 정글이었다."),
                        Self::line("빌딩 사이로 덩굴과 나무가 우거져 있다."),
                        Self::line("길 한복판에 문들이 덩그러니 서 있다."),
                    ]);
                    self.pending_mail = Some(MailId::Outside);
                }
            }
            MainPlace::MissionRoom(_) | MainPlace::StairHall => {
                self.jungle = Vec::new();
                self.barriers = Vec::new();
            }
        }
        self.player = Player::at(start.0, start.1);
    }

    // 문 올라가는 연출, 계단 펼쳐지는 연출, 장소 이동 페이드 같은 장면 진행(대화와 상관없이 매 프레임).
    fn advance_scene(&mut self, dt: f32) {
        self.clock += dt;
        if self.unlock_pending && !self.dialogue.active() {
            self.unlock_pending = false;
            self.progress.tutorial_done = true;
            self.door_anim = Some(0.0);
        }
        if let Some(a) = &mut self.door_anim {
            *a += dt;
            // ENA 식으로 뚝뚝 끊겨 올라간다.
            self.door_lift = DOOR_LIFT * smooth(ena_time(*a) / DOOR_OPEN_SECS);
            if *a >= DOOR_OPEN_SECS {
                self.door_anim = None;
                self.door_lift = DOOR_LIFT;
            }
        }
        // 먹이는 대화가 끝나면 계단이 몸을 펼친다.
        if self.progress.chip_fed
            && !self.dialogue.active()
            && let Some(a) = &mut self.unfold_anim
        {
            *a += dt;
            self.unfold = (*a / UNFOLD_SECS).min(1.0);
            if self.unfold >= 1.0 {
                self.unfold_anim = None;
            }
        }
        if !self.dialogue.active() && self.pending_move.is_some() && self.door_anim.is_none() {
            // 문 너머의 빛으로 나갈 땐 하얗게, 나머지는 검게 덮는다.
            if self.pending_move == Some(MainPlace::Jungle) && self.progress.place == MainPlace::StartRoom {
                self.fade.start_colored([1.0, 1.0, 0.92]);
            } else {
                self.fade.start();
            }
        }
        if self.fade.update(dt)
            && let Some(to) = self.pending_move.take()
        {
            self.go(to);
        }
    }

    // 다른 창이 앞에 오는 순간 지금 서 있는 자리를 기억해 둔다(CRT 에서 게임을 오갈 때 위치 유지).
    fn remember_pose_on_blur(&mut self, focused: bool) {
        if self.was_focused && !focused && !self.fade.active() {
            self.progress.pose = Some(self.player.pose());
        }
        self.was_focused = focused;
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

        // 지금 장소의 충돌 대상(collide)과 화면에 그릴 것(shown), 조사할 물건들.
        let place = self.progress.place;
        let mut room_items: Vec<crate::apps::game3d::item::Item<Spot>> = Vec::new();
        let mut jungle_items: Vec<crate::apps::game3d::item::Item<floor2::Target>> = Vec::new();
        let (clear, collide, shown) = match place {
            MainPlace::StartRoom => {
                let mut boxes = build_room(self.door_lift);
                if self.progress.quest_accepted && !self.progress.has_letter {
                    boxes.push(letter_box());
                }
                (ROOM_CLEAR, boxes.clone(), boxes)
            }
            MainPlace::Jungle => {
                jungle_items = floor2::items(self.progress.exchanged && !self.progress.chip_taken);
                let mut shown = self.jungle.clone();
                shown.extend(jungle_items.iter().filter(|it| it.draw).map(|it| it.to_box()));
                let mut collide = shown.clone();
                collide.extend(self.barriers.iter().cloned());
                (floor2::CLEAR_COLOR, collide, shown)
            }
            MainPlace::MissionRoom(kind) => {
                let step = self.progress.missions[kind.index()];
                let room = rooms::mission_room(kind, step == MissionStep::Done, step >= MissionStep::Received, self.clock);
                room_items = room.items;
                let mut collide = room.boxes.clone();
                collide.extend(room.barriers);
                (room.clear, collide, room.boxes)
            }
            MainPlace::StairHall => {
                let room = rooms::stair_hall(self.progress.chip_fed, self.unfold);
                room_items = room.items;
                let mut collide = room.boxes.clone();
                collide.extend(room.barriers);
                (room.clear, collide, room.boxes)
            }
        };

        // 대화창이 떠 있는 동안은 이동·시점·조준을 멈춘다.
        let frozen = self.dialogue.active() || self.fade.active() || self.door_anim.is_some() || self.unfold_anim.is_some();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &collide);
            let cam = self.player.camera();
            self.aimed = match place {
                MainPlace::StartRoom => {
                    let (accepted, door_changed, has_letter) = (self.progress.quest_accepted, self.progress.door_changed, self.progress.has_letter);
                    let door_pos = if self.progress.tutorial_done { EXIT_POS } else { HANDLE_POS };
                    aimed_target(&cam, door_pos, |t| match t {
                        // 수락한 직후엔 문이 말이 없다 — 편지 메일을 읽고 돌아온 뒤에야 다시 말을 한다.
                        Target::Door => !accepted || door_changed,
                        // 편지는 수락한 뒤에 책상 위에 나타나고, 집으면 사라진다.
                        Target::Letter => accepted && !has_letter,
                        _ => true,
                    })
                    .map(Aim::Room)
                }
                MainPlace::Jungle => jungle_items
                    .iter()
                    .filter_map(|it| aim_dist(&cam, it.center).map(|d| (it.target, d)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(t, _)| Aim::Jungle(t)),
                MainPlace::MissionRoom(_) | MainPlace::StairHall => room_items
                    .iter()
                    .filter_map(|it| aim_dist(&cam, it.center).map(|d| (it.target, d)))
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(t, _)| Aim::Spot(t)),
            };
        }

        let cam = self.player.camera();
        view.draw_scene(ctx, r, area, &self.slot, clear, &cam, &shown);

        if let Some(aim) = self.aimed
            && !frozen
        {
            let found = match aim {
                Aim::Room(Target::Door) if self.progress.tutorial_done => Some((EXIT_POS, "[E] Go out".to_string())),
                Aim::Room(Target::Door) => Some((HANDLE_POS, "[E] Examine".to_string())),
                Aim::Room(Target::Prop(i)) => Some((PROPS[i].center, format!("[E] Examine {}", PROPS[i].name))),
                Aim::Room(Target::Letter) => Some((Target::Letter.pos(), "[E] Take letter".to_string())),
                Aim::Room(Target::Mailbox) => Some((Target::Mailbox.pos(), "[E] Mailbox".to_string())),
                Aim::Jungle(t) => jungle_items.iter().find(|it| it.target == t).map(|it| (it.center, it.label.to_string())),
                Aim::Spot(s) => room_items.iter().find(|it| it.target == s).map(|it| (it.center, it.label.to_string())),
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
                if p.missions[kind.index()] >= MissionStep::Received {
                    lines.push(item_name(kind));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(app: &mut DoorsGameApp) {
        app.dialogue = Dialogue::new();
    }

    // 장소를 옮기는 페이드를 끝까지 돌린다.
    fn finish_move(app: &mut DoorsGameApp) {
        quiet(app);
        for _ in 0..100 {
            app.advance_scene(0.05);
        }
    }

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
        quiet(&mut app);

        app.interact_room(Some(Target::Mailbox));
        assert!(!app.progress.letter_sent, "편지가 없으면 보낼 게 없다");

        app.progress.quest_accepted = true;
        app.interact_room(Some(Target::Prop(0)));
        assert!(!app.progress.flower_absence_checked, "편지 일을 끝내기 전엔 꽃이 없다는 걸 못 깨닫는다");

        app.interact_room(Some(Target::Letter));
        assert!(app.progress.has_letter);
        app.interact_room(Some(Target::Mailbox));
        assert!(app.progress.letter_sent && app.letter_notify_pending);

        quiet(&mut app);
        assert!(!app.notice_mail_read(false, true), "메일을 안 읽었으면 문은 그대로");
        assert!(!app.notice_mail_read(true, false), "창으로 돌아오기 전엔 문은 그대로");
        assert!(app.notice_mail_read(true, true) && app.progress.door_changed);

        quiet(&mut app);
        app.interact_room(Some(Target::Prop(0)));
        assert!(app.progress.flower_absence_checked);
    }

    // 꽃이 도착 → 문에게 건넴 → 자물쇠가 풀려 문이 올라감 → 문으로 나가 2층 정글에 서고 메일이 온다.
    #[test]
    fn flower_unlocks_the_door_and_leads_outside() {
        let progress = DoorsProgress { quest_accepted: true, has_letter: true, letter_sent: true, door_changed: true, flower_absence_checked: true, ..DoorsProgress::default() };
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);

        assert!(!app.receive_flower(false, true), "꽃을 아직 내려받지 않았으면 안 들어온다");
        assert!(!app.receive_flower(true, false), "이 창이 앞에 오기 전엔 안 들어온다");
        assert!(app.receive_flower(true, true) && app.progress.flower_received);
        assert!(!app.receive_flower(true, true), "꽃은 한 번만 들어온다");

        // 꽃을 갖고 문에게 말을 걸기 전엔 문이 닫혀 있다. 건네면 대사 뒤에 자물쇠가 풀린다.
        quiet(&mut app);
        assert!(!app.progress.tutorial_done && app.door_lift == 0.0);
        app.interact_room(Some(Target::Door));
        assert!(app.progress.flower_given && app.unlock_pending && !app.progress.tutorial_done);
        app.advance_scene(0.1);
        assert!(!app.progress.tutorial_done, "대사가 끝나기 전엔 안 풀린다");
        quiet(&mut app);
        app.advance_scene(0.1);
        assert!(app.progress.tutorial_done && app.door_anim.is_some());
        app.advance_scene(DOOR_OPEN_SECS);
        assert_eq!(app.door_lift, DOOR_LIFT);
        assert!(app.door_anim.is_none());

        // 열린 문으로 나가면 페이드 뒤에 정글에 서고, 처음이라 설명 대사 + 메일이 따라온다.
        quiet(&mut app);
        app.interact_room(Some(Target::Door));
        assert_eq!(app.pending_move, Some(MainPlace::Jungle));
        finish_move(&mut app);
        assert_eq!(app.progress.place, MainPlace::Jungle);
        assert!(app.progress.jungle_visited && app.dialogue.active());
        assert_eq!(app.pending_mail, Some(MailId::Outside));
        assert!(!app.jungle.is_empty() && !app.barriers.is_empty());
    }

    // 정글에서 닫았다 다시 열면 정글에서 이어지고, 문은 이미 열려 있으며, 설명 대사는 안 나온다.
    #[test]
    fn resuming_in_the_jungle() {
        let progress = DoorsProgress {
            quest_accepted: true,
            tutorial_done: true,
            flower_received: true,
            flower_given: true,
            jungle_visited: true,
            place: MainPlace::Jungle,
            ..DoorsProgress::default()
        };
        let app = DoorsGameApp::new(progress);
        assert!(!app.dialogue.active() && !app.jungle.is_empty());
        assert_eq!(app.door_lift, DOOR_LIFT);
    }

    #[test]
    fn jungle_has_five_gates_a_sign_and_a_sofa() {
        let items = floor2::items(false);
        assert_eq!(items.iter().filter(|it| matches!(it.target, floor2::Target::Gate(_))).count(), 5);
        assert!(items.iter().any(|it| it.target == floor2::Target::Sign) && items.iter().any(|it| it.target == floor2::Target::Sofa));
        assert!(!items.iter().any(|it| it.target == floor2::Target::Envelope), "교환 전엔 봉투가 없다");
        assert!(floor2::items(true).iter().any(|it| it.target == floor2::Target::Envelope));
    }

    fn in_jungle() -> DoorsGameApp {
        let progress = DoorsProgress { quest_accepted: true, tutorial_done: true, flower_given: true, jungle_visited: true, place: MainPlace::Jungle, ..DoorsProgress::default() };
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);
        app
    }

    // 계단이 먼저 배고프다고 해야 소파가 의뢰를 건네고, 수락하면 미션 문이 열린다(장보기 목록 메일).
    #[test]
    fn errand_needs_the_stairs_first_and_unlocks_the_mission_doors() {
        let mut app = in_jungle();
        app.interact_jungle(floor2::Target::Gate(Gate::Eye));
        assert!(app.pending_move.is_none(), "의뢰 전엔 미션 문이 잠겨 있다");

        quiet(&mut app);
        app.interact_jungle(floor2::Target::Sofa);
        assert!(app.ask.is_none() && !app.progress.errand_accepted, "계단 얘기를 듣기 전엔 의뢰를 안 준다");

        app.progress.place = MainPlace::StairHall;
        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        assert!(app.progress.stair_met);
        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        assert!(app.dialogue.active(), "의뢰 전에도 같은 말을 한다");

        app.progress.place = MainPlace::Jungle;
        quiet(&mut app);
        app.interact_jungle(floor2::Target::Sofa);
        assert_eq!(app.ask, Some(Ask::Errand));
        app.answer(true);
        assert!(app.progress.errand_accepted && app.pending_mail == Some(MailId::Shopping));

        quiet(&mut app);
        app.interact_jungle(floor2::Target::Gate(Gate::Brain));
        assert_eq!(app.pending_move, Some(MainPlace::MissionRoom(MiniKind::Brain)));
    }

    // 눈의 방 한 바퀴: 의뢰 → 기계(메일) → 보상 사진 내려받음 → 재료 → 달라진 NPC.
    #[test]
    fn a_mission_runs_from_request_to_ingredient() {
        let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Eye), tutorial_done: true, ..DoorsProgress::default() };
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);

        app.interact_spot(Spot::Machine);
        assert_eq!(app.progress.missions[0], MissionStep::Start, "NPC 의뢰를 듣기 전엔 기계가 안 켜진다");
        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        assert_eq!(app.progress.missions[0], MissionStep::Talked);
        quiet(&mut app);
        app.interact_spot(Spot::Machine);
        assert_eq!(app.progress.missions[0], MissionStep::Sent);
        assert_eq!(app.pending_mail, Some(MailId::EyeTask));
        assert!(app.progress.pose.is_some(), "게임을 받는 순간 서 있는 자리도 저장된다");

        quiet(&mut app);
        assert!(!app.receive_ingredient([false; 3], true), "사진을 내려받기 전엔 재료가 없다");
        assert!(!app.receive_ingredient([true, false, false], false), "창이 앞에 오기 전엔 안 들어온다");
        assert!(app.receive_ingredient([true, false, false], true));
        assert_eq!(app.progress.missions[0], MissionStep::Received);
        assert_eq!(app.ingredients(), 1);
        quiet(&mut app);
        assert!(!app.receive_ingredient([true, false, false], true), "재료는 한 번만 받는다");

        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        assert_eq!(app.progress.missions[0], MissionStep::Done);
        quiet(&mut app);
        app.interact_spot(Spot::Machine);
        assert_eq!(app.progress.missions[0], MissionStep::Done, "끝난 방을 다시 찾아도 보상은 추가로 안 준다");
        assert_eq!(app.ingredients(), 1);
    }

    // 재료 셋 → 소파에게 교환 → 봉투(Mr. POTATO CHIP) → 계단에게 먹임 → 계단이 펼쳐지고 2층이 끝난다.
    #[test]
    fn three_ingredients_become_a_chip_that_feeds_the_stairs() {
        let progress = DoorsProgress {
            errand_accepted: true,
            stair_met: true,
            tutorial_done: true,
            place: MainPlace::Jungle,
            jungle_visited: true,
            missions: [MissionStep::Received, MissionStep::Done, MissionStep::Received],
            ..DoorsProgress::default()
        };
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);

        // 둘만 있으면 부족한 걸 알려준다(가진 재료는 그대로).
        app.progress.missions[2] = MissionStep::Talked;
        app.interact_jungle(floor2::Target::Sofa);
        assert!(app.ask.is_none() && app.ingredients() == 2);
        app.progress.missions[2] = MissionStep::Received;

        quiet(&mut app);
        app.interact_jungle(floor2::Target::Sofa);
        assert_eq!(app.ask, Some(Ask::Exchange));
        app.answer(true);
        assert!(app.progress.exchanged && app.ingredients() == 0, "세 재료가 한꺼번에 회수된다");

        quiet(&mut app);
        app.interact_jungle(floor2::Target::Envelope);
        assert!(app.progress.chip_taken && app.has_chip());
        assert_eq!(app.pending_mail, Some(MailId::Meal));
        assert!(!floor2::items(app.progress.exchanged && !app.progress.chip_taken).iter().any(|it| it.target == floor2::Target::Envelope));

        app.progress.place = MainPlace::StairHall;
        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        assert_eq!(app.ask, Some(Ask::Feed));
        app.answer(false);
        assert!(!app.progress.chip_fed && app.has_chip(), "거절하면 아이템은 그대로");
        quiet(&mut app);
        app.interact_spot(Spot::Npc);
        app.answer(true);
        assert!(app.progress.chip_fed && !app.has_chip(), "먹이면 아이템 1개 소비와 급식 완료가 같이 일어난다");

        quiet(&mut app);
        for _ in 0..100 {
            app.advance_scene(0.1);
        }
        assert_eq!(app.unfold, 1.0);
        assert!(rooms::stair_hall(true, app.unfold).items.iter().any(|it| it.target == Spot::StairStep));
        quiet(&mut app);
        app.interact_spot(Spot::StairStep);
        assert!(app.progress.floor2_done);
    }

    #[test]
    fn leaving_a_room_puts_you_in_front_of_its_door() {
        let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Finger), tutorial_done: true, ..DoorsProgress::default() };
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);
        app.interact_spot(Spot::Exit);
        finish_move(&mut app);
        assert_eq!(app.progress.place, MainPlace::Jungle);
        let front = floor2::gate_front(Gate::Finger);
        let cam = app.player.camera();
        assert!((cam.pos[0] - front[0]).abs() < 0.01 && (cam.pos[2] - front[2]).abs() < 0.01);
    }

    #[test]
    fn focus_loss_remembers_where_you_stood() {
        let mut app = in_jungle();
        assert!(app.progress.pose.is_none());
        app.remember_pose_on_blur(true);
        app.remember_pose_on_blur(false);
        let pose = app.progress.pose.expect("포커스를 잃으면 자리를 기억한다");
        let resumed = DoorsGameApp::new(DoorsProgress { pose: Some(pose), place: MainPlace::Jungle, ..app.progress.clone() });
        let cam = resumed.player.camera();
        assert!((cam.pos[0] - pose[0]).abs() < 0.01 && (cam.pos[2] - pose[2]).abs() < 0.01);
    }

    // 어느 장소든 시작 자리가 상자 속에 파묻혀 있거나 허공이 아니어야 한다(나무/소파/벽에 끼지 않고 바닥을 딛는다).
    #[test]
    fn every_spawn_stands_on_the_floor_and_clear_of_walls() {
        use crate::render::mesh3d::{ground_height, resolve_horizontal};
        let jungle = {
            let mut b = floor2::build_scene([false; 3], false);
            b.extend(floor2::items(true).iter().map(|it| it.to_box()));
            b.extend(floor2::barriers());
            b
        };
        let mut cases: Vec<(&str, [f32; 3], Vec<Box3D>)> = vec![
            ("시작방", SPAWN, build_room(0.0)),
            ("정글(시작방 문 앞)", floor2::SPAWN, jungle.clone()),
            ("계단 입구", rooms::SPAWN, rooms::stair_hall(false, 0.0).boxes),
        ];
        for gate in [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs] {
            cases.push(("정글(문 앞)", floor2::gate_front(gate), jungle.clone()));
        }
        for kind in MiniKind::ALL {
            let room = rooms::mission_room(kind, false, false, 0.0);
            let mut b = room.boxes;
            b.extend(room.barriers);
            cases.push(("미션 방", rooms::SPAWN, b));
        }
        for (name, spawn, boxes) in cases {
            let pushed = resolve_horizontal(spawn, 0.3, spawn[1], 1.7, &boxes);
            assert!((pushed[0] - spawn[0]).abs() < 0.01 && (pushed[2] - spawn[2]).abs() < 0.01, "{name}: 시작 자리가 무언가에 겹쳐 있다 {spawn:?}");
            let ground = ground_height(spawn[0], spawn[2], spawn[1] + 1.0, 60.0, &boxes);
            assert!(ground.is_some_and(|g| g.abs() < 0.01), "{name}: 시작 자리 아래에 바닥이 없다 {ground:?}");
        }
    }

    // 정글의 문과 NPC 들은 걸어서 닿을 수 있는 거리에 조준될 수 있어야 한다(문 앞 자리에서 문이 조준권 안).
    #[test]
    fn gates_are_aimable_from_the_front() {
        use crate::render::mesh3d::Camera;
        for gate in [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs, Gate::Back] {
            let front = floor2::gate_front(gate);
            let cam = Camera { pos: [front[0], 1.55, front[2] - 0.2], yaw: 0.0, pitch: 0.0 }; // 문 쪽(-Z)을 본다
            let c = gate.center();
            assert!(aim_dist(&cam, c).is_some(), "{gate:?}: 문 앞에서 문을 조준할 수 없다");
        }
    }
}
