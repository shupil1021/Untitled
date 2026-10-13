//! 미션 방 NPC 의 대사 모음과 이름표 — 눈 / 뇌 / 손가락의 방이 같은 흐름이고 문구만 다르다.

use crate::foundation::MiniKind;

use super::floor2::Gate;

// 미션 방의 대사 모음 — 세 방이 같은 흐름이라 문구만 다르다.
pub(super) struct MissionText {
    pub(super) talk_first: &'static [&'static str],   // E-1: 처음 말을 걸었을 때(의뢰)
    pub(super) talk_waiting: &'static [&'static str], // 의뢰를 듣고 아직 재료를 못 받았을 때
    pub(super) machine_locked: &'static [&'static str], // 의뢰를 듣기 전에 기계를 눌렀을 때
    pub(super) machine_send: &'static [&'static str], // E-2: 기계에서 게임을 받았다
    pub(super) machine_waiting: &'static [&'static str],
    pub(super) received: &'static [&'static str],     // E-3: 보상이 손에 들어왔다
    pub(super) talk_received: &'static [&'static str], // E-4: 재료를 받은 뒤 NPC 가 달라진다
    pub(super) talk_done: &'static [&'static str],    // 끝난 방을 다시 찾았을 때
    pub(super) machine_done: &'static [&'static str],
}

pub(super) fn mission_text(kind: MiniKind) -> &'static MissionText {
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

pub(super) fn gate_of(kind: MiniKind) -> Gate {
    match kind {
        MiniKind::Eye => Gate::Eye,
        MiniKind::Brain => Gate::Brain,
        MiniKind::Finger => Gate::Finger,
    }
}

pub(super) fn item_name(kind: MiniKind) -> &'static str {
    match kind {
        MiniKind::Eye => "충혈된 눈",
        MiniKind::Brain => "짓무른 뇌",
        MiniKind::Finger => "절단된 손가락",
    }
}

pub(super) fn door_name(kind: MiniKind) -> &'static str {
    match kind {
        MiniKind::Eye => "눈 표식 문",
        MiniKind::Brain => "뇌 표식 문",
        MiniKind::Finger => "손가락 표식 문",
    }
}
