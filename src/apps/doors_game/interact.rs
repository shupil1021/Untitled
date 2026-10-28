//! 플레이어가 E 로 조사한 것들의 결과 — 시작방의 문/물건, 정글의 문/표지판/소파, 미션 방의 NPC/기계, 계단 입구,
//! 그리고 OS 쪽 사건(메일을 읽음, 꽃/보상 사진을 내려받음)에 반응하는 것. 진행 상태(progress)를 바꾸고 대화를 띄운다.

use crate::foundation::{MailId, MainPlace, MiniKind, MissionStep};

use super::floor2::{self, Gate};
use crate::apps::game3d::dialogue::Entry;
use super::rooms::Spot;
use super::sfx::Cue;
use super::texts::{door_name, gate_of, item_name, mission_text};
use super::world::{Target, PROPS};
use super::{Ask, DoorsGameApp, EXCHANGE_OPTIONS, FEED_OPTIONS, RESEND_OPTIONS};

impl DoorsGameApp {
    // 편지를 우편함에 넣은 뒤 OS 에서 그 편지 메일을 읽었고(mail_read) 이 창으로 돌아왔으면(focused)
    // 문이 달라진다 — 이때부터 문의 대사가 바뀐다. 게임은 그동안 멈추지 않고 그대로 돌아간다.
    pub(super) fn notice_mail_read(&mut self, mail_read: bool, focused: bool) -> bool {
        if self.progress.letter_sent && !self.progress.door_changed && mail_read && focused && !self.dialogue.active() {
            self.progress.door_changed = true;
            self.dialogue.start(vec![Self::line("문 쪽에서 인기척이 느껴진다...")]);
            return true;
        }
        false
    }

    // 서브 게임이 보낸 꽃이 OS 에서 내려받아졌으면 이 게임을 다시 열었을 때(창이 앞에 왔을 때) 꽃이 손에
    // 들어와 있다(SUB A+a-20). 시작방에서만, 대화 중이 아닐 때.
    pub(super) fn receive_flower(&mut self, downloaded: bool, focused: bool) -> bool {
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

    // 미니게임 보상 사진(충혈된 눈 / 금고 절반)을 내려받고 이 창으로 돌아오면 자동으로 들어온다(MAIN E-3 / R-3).
    // 리듬은 사진이 없다 — 호수에 넣은 손잡이를 화장실 변기 속 손에게서 받는다(use_toilet).
    // 어디에 있든(방이든 정글이든) 받는다 — 중간에 나갔다 와도 수령 대기가 복구된다(CRT SYS-2).
    pub(super) fn receive_ingredient(&mut self, downloaded: [bool; 3], focused: bool) -> bool {
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
    pub(super) fn interact_room(&mut self, target: Option<Target>) {
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
    pub(super) fn interact_jungle(&mut self, target: floor2::Target) {
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
    pub(super) fn talk_sofa(&mut self) {
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
                    Self::line("충혈된 눈 하나, 소환 조각 둘. 소환 조각은 레이싱 쪽과 리듬 쪽, 하나씩이야."),
                    Entry::Choice("의뢰를 받아들일래?".to_string()),
                ]);
            }
        } else if self.ingredients() == 3 {
            self.ask = Some(Ask::Exchange);
            self.dialogue.start(vec![
                Self::line("전부 가져왔어."),
                Self::line("눈, 레이싱 조각, 리듬 조각... 하나씩 다 있다."),
                Entry::Ask("Mr. POTATO CHIP으로 바꿀까?".to_string(), EXCHANGE_OPTIONS),
            ]);
        } else {
            // 부족한 재료와 그 문의 표식을 알려준다. 가진 재료는 그대로 둔다.
            let mut lines = vec![Self::line("아직 모자라.")];
            for kind in MiniKind::ALL {
                if !self.progress.missions[kind.index()].has_material(kind) {
                    lines.push(Self::line(&format!("{}이 필요해. {}으로 가 봐.", item_name(kind), door_name(kind))));
                }
            }
            self.dialogue.start(lines);
        }
    }

    // E — 미션 방 / 계단 입구에서 조준 중인 대상을 조사한다.
    pub(super) fn interact_spot(&mut self, spot: Spot) {
        match self.progress.place {
            MainPlace::MissionRoom(kind) => self.interact_mission(kind, spot),
            MainPlace::StairHall => self.interact_stairs(spot),
            _ => {}
        }
    }

    pub(super) fn interact_mission(&mut self, kind: MiniKind, spot: Spot) {
        if spot == Spot::Exit {
            self.entry_gate = Some(gate_of(kind));
            self.pending_move = Some(MainPlace::Jungle);
            return;
        }
        match kind {
            MiniKind::Eye => self.interact_eye_room(spot),
            MiniKind::Brain => self.interact_vault_room(spot),
            MiniKind::Finger => self.interact_bathroom(spot),
        }
    }

    // 게임 받기 기계(눈의 방 / 금고의 방) — 의뢰를 들은 뒤 누르면 OS 로 설치 파일이 첨부된 메일이 간다(E-2 / R-2).
    fn use_machine(&mut self, kind: MiniKind) {
        let text = mission_text(kind);
        let i = kind.index();
        match self.progress.missions[i] {
            MissionStep::Start => self.dialogue.start(Self::lines(text.machine_locked)),
            // 지금 서 있는 자리도 같이 저장된다.
            MissionStep::Talked => {
                self.progress.missions[i] = MissionStep::Sent;
                self.pending_mail = Some(kind.task_mail());
                self.progress.pose = Some(self.player.pose());
                self.dialogue.start(Self::lines(text.machine_send));
            }
            // 이미 받았다 — 파일을 잃어버렸다면 다시 받을 수 있다.
            step => {
                self.ask = Some(Ask::Resend(kind));
                let mut lines = Self::lines(if step == MissionStep::Sent { text.machine_waiting } else { text.machine_done });
                lines.push(Entry::Ask("게임 파일을 다시 받을까?".to_string(), RESEND_OPTIONS));
                self.dialogue.start(lines);
            }
        }
    }

    // 눈의 방 — 렌즈 NPC 와 이야기 → 기계에서 게임 받기 → (미니게임) → 눈 수령 → NPC 의 마지막 이야기.
    fn interact_eye_room(&mut self, spot: Spot) {
        let kind = MiniKind::Eye;
        let text = mission_text(kind);
        let i = kind.index();
        match spot {
            Spot::Npc => match self.progress.missions[i] {
                MissionStep::Start => {
                    self.progress.missions[i] = MissionStep::Talked;
                    self.dialogue.start(Self::lines(text.talk_first));
                }
                MissionStep::Talked | MissionStep::Sent => self.dialogue.start(Self::lines(text.talk_waiting)),
                MissionStep::Received => {
                    self.progress.missions[i] = MissionStep::Done;
                    self.dialogue.start(Self::lines(text.talk_received));
                }
                _ => self.dialogue.start(Self::lines(text.talk_done)),
            },
            Spot::Machine => self.use_machine(kind),
            _ => {}
        }
    }

    // 금고의 방 — 표지판 NPC 와 이야기 → 기계에서 게임 받기 → (레이싱) → 메일의 금고 절반 수령 → 방의 금고 반쪽에
    // 맞춰 완성 → 금고를 열어 소환 조각(레이싱) 획득(R-0 ~ R-5).
    fn interact_vault_room(&mut self, spot: Spot) {
        let kind = MiniKind::Brain;
        let text = mission_text(kind);
        let i = kind.index();
        match spot {
            Spot::Npc => match self.progress.missions[i] {
                MissionStep::Start => {
                    self.progress.missions[i] = MissionStep::Talked;
                    self.dialogue.start(Self::lines(text.talk_first));
                }
                MissionStep::Talked | MissionStep::Sent => self.dialogue.start(Self::lines(text.talk_waiting)),
                MissionStep::Received => self.dialogue.start(vec![Self::line("받은 금고 절반을 방의 금고에 맞춰 봐.")]),
                MissionStep::Built => self.dialogue.start(vec![Self::line("이제 열 수 있어. 다이얼을 돌려 봐.")]),
                MissionStep::Revealed | MissionStep::Fragment => {
                    self.progress.missions[i] = MissionStep::Done;
                    self.dialogue.start(Self::lines(text.talk_received));
                }
                MissionStep::Done => self.dialogue.start(Self::lines(text.talk_done)),
            },
            Spot::Machine => self.use_machine(kind),
            Spot::Safe => match self.progress.missions[i] {
                MissionStep::Start | MissionStep::Talked | MissionStep::Sent => self.dialogue.start(vec![
                    Self::line("금고가 한쪽 절반만 서 있다."),
                    Self::line("잘려 나간 단면 너머로 속이 텅 비어 보인다."),
                    Self::line("나머지 절반이 필요하다."),
                ]),
                // 받은 절반을 맞춰 금고를 완성한다(R-4) — 절반은 소비된다.
                MissionStep::Received => {
                    self.progress.missions[i] = MissionStep::Built;
                    self.dialogue.start(vec![
                        Self::line("받은 금고 절반을 잘려 나간 면에 맞췄다."),
                        Self::line("쿵, 하고 두 절반이 하나로 붙었다."),
                        Self::line("금고가 온전해졌다."),
                    ]);
                }
                // 열어서 안을 조사하면 소환 조각을 얻는다(R-5) — 한 번뿐이다.
                MissionStep::Built => {
                    self.progress.missions[i] = MissionStep::Fragment;
                    self.dialogue.start(vec![
                        Self::line("다이얼을 돌리자 육중한 금고 문이 열렸다."),
                        Self::line("안쪽에 작은 조각이 하나 들어 있다."),
                        Self::line("'소환 조각'(레이싱)을 얻었다."),
                    ]);
                }
                _ => self.dialogue.start(vec![Self::line("금고는 텅 비어 있다.")]),
            },
            _ => {}
        }
    }

    // 화장실 — 기사의 의뢰(수락하면 방귀와 나팔 연출 뒤 PACK 이 메일로 온다) → (리듬 + 호수) → 변기 속 손에게서 손잡이
    // 수령 → 조립 → 물 내리기 → 소환 조각(리듬) → 기사에게 보고(H-0 ~ H-9).
    fn interact_bathroom(&mut self, spot: Spot) {
        let kind = MiniKind::Finger;
        let text = mission_text(kind);
        let i = kind.index();
        let step = self.progress.missions[i];
        match spot {
            Spot::Npc => match step {
                MissionStep::Start | MissionStep::Talked => {
                    self.ask = Some(Ask::Knight);
                    let mut lines = Self::lines(text.talk_first);
                    lines.push(Entry::Choice("기사의 의뢰를 받아들일래?".to_string()));
                    self.dialogue.start(lines);
                }
                MissionStep::Sent => {
                    self.ask = Some(Ask::Resend(kind));
                    let mut lines = Self::lines(text.talk_waiting);
                    lines.push(Entry::Ask("기사가 보낸 게임 파일을 다시 받을까?".to_string(), RESEND_OPTIONS));
                    self.dialogue.start(lines);
                }
                MissionStep::Received => self.dialogue.start(vec![Self::line("내 칼이 돌아왔군! 그 칼을 변기에 꽂아 주시오.")]),
                MissionStep::Built | MissionStep::Revealed => self.dialogue.start(vec![Self::line("물을 내려 보시오. 칼의 힘이 깨어날 것이오.")]),
                MissionStep::Fragment => {
                    self.progress.missions[i] = MissionStep::Done;
                    self.dialogue.start(Self::lines(text.talk_received));
                }
                MissionStep::Done => self.dialogue.start(Self::lines(text.talk_done)),
            },
            Spot::Toilet => self.use_toilet(step),
            Spot::Fixture(n) => {
                let lines: &[&str] = match n {
                    0 => &["낡은 욕조다.", "물이 오래 고여 있던 자국이 남아 있다."],
                    1 => &["세면대의 수도꼭지에서 물이 한 방울씩 떨어진다."],
                    _ => &["거울 속의 내가 잠깐 한 박자 늦게 움직인 것 같다."],
                };
                self.dialogue.start(Self::lines(lines));
            }
            _ => {}
        }
    }

    // 기사의 의뢰를 수락했다(H-2) — 방귀 소리와 퀘스트 시작 자막, 중세 효과음이 한 번 나오고 PACK 설치 메일이 간다.
    pub(super) fn accept_knight(&mut self) {
        self.progress.missions[MiniKind::Finger.index()] = MissionStep::Sent;
        self.pending_mail = Some(MiniKind::Finger.task_mail());
        self.progress.pose = Some(self.player.pose());
        self.sfx_cue = Some(Cue::QuestStart);
        self.dialogue.start(vec![
            Self::line("기사가 의뢰를 받아들이자 어디선가 뿌웅, 하는 소리가 울렸다."),
            Self::line("[ 퀘스트 시작: 기사의 잃어버린 칼 ]"),
            Self::line("(중세풍 나팔 소리가 울려 퍼졌다.)"),
            Self::line("어디선가 알림음이 울린다..."),
        ]);
    }

    // 변기 — 진행에 따라 하는 일이 달라진다(H-3 ~ H-7).
    fn use_toilet(&mut self, step: MissionStep) {
        let i = MiniKind::Finger.index();
        let lake = crate::signals::lake_delivered();
        match step {
            MissionStep::Start | MissionStep::Talked => {
                self.dialogue.start(vec![Self::line("변기의 물 내리는 손잡이 자리가 비어 있다."), Self::line("먼저 기사와 이야기해 보자.")]);
            }
            MissionStep::Sent if !lake => {
                self.dialogue.start(vec![Self::line("변기에는 손잡이가 달려 있어야 할 자리가 비어 있다."), Self::line("구멍 속은 어둡고 축축하다.")]);
            }
            // 호수에 손잡이를 넣었다 — 변기 속에서 손이 나와 그 손잡이를 내민다(H-3).
            MissionStep::Sent if !self.progress.hand_out => {
                self.progress.hand_out = true;
                self.dialogue.start(vec![
                    Self::line("변기 속에서 물소리가 났다."),
                    Self::line("하얀 손이 불쑥 올라와 무언가를 내밀고 있다."),
                    Self::line("호수에 넣었던 변기 손잡이다."),
                ]);
            }
            // 손이 내민 손잡이를 받는다(H-4).
            MissionStep::Sent => {
                self.progress.hand_out = false;
                self.progress.missions[i] = MissionStep::Received;
                self.dialogue.start(Self::lines(mission_text(MiniKind::Finger).received));
            }
            // 빈 자리에 끼운다(H-5) — 손잡이는 소비된다.
            MissionStep::Received => {
                self.progress.missions[i] = MissionStep::Built;
                self.dialogue.start(vec![Self::line("비어 있던 자리에 변기 손잡이를 끼웠다."), Self::line("딸깍. 변기가 온전해졌다.")]);
            }
            // 물을 내린다(H-6) — 소환 조각이 떠오른다.
            MissionStep::Built => {
                self.progress.missions[i] = MissionStep::Revealed;
                self.sfx_cue = Some(Cue::Flush);
                self.dialogue.start(vec![
                    Self::line("손잡이를 눌렀다."),
                    Self::line("쏴아아— 물이 소용돌이치며 내려간다."),
                    Self::line("변기 안에서 무언가가 떠올랐다."),
                ]);
            }
            // 떠오른 조각을 집는다(H-7) — 물을 또 내려도 더 나오지 않는다.
            MissionStep::Revealed => {
                self.progress.missions[i] = MissionStep::Fragment;
                self.dialogue.start(vec![Self::line("물 위에 떠 있는 조각을 집어 들었다."), Self::line("'소환 조각'(리듬)을 얻었다.")]);
            }
            MissionStep::Fragment | MissionStep::Done => {
                self.sfx_cue = Some(Cue::Flush);
                self.dialogue.start(vec![Self::line("물을 내려 보았다."), Self::line("더는 아무것도 나오지 않는다.")]);
            }
        }
    }

    pub(super) fn interact_stairs(&mut self, spot: Spot) {
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
}
