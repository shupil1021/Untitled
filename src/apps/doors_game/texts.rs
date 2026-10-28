//! 미션 방 NPC 의 대사 모음과 이름표 — 눈의 방 / 금고의 방 / 화장실의 NPC 대사.

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
        talk_first: &["바닥의 도로 위에 표지판이 서 있다.", "집까지 가는 길이야. 이 길 끝에 금고의 나머지 절반이 있어.", "표지판이 운전대 모양의 기계 쪽을 가리킨다."],
        talk_waiting: &["저 기계에서 게임을 받아."],
        machine_locked: &["운전대 기계는 꺼져 있다.", "먼저 표지판과 이야기해 보자."],
        machine_send: &["운전대 기계의 '게임 받기'를 눌렀다.", "어디선가 알림음이 울린다..."],
        machine_waiting: &["이미 게임을 받았다.", "컴퓨터에서 확인해 보자."],
        received: &["어디선가 보낸 금고 절반이 손에 쥐어져 있다.", "'금고 절반'을 얻었다.", "방에 서 있는 금고 반쪽에 맞춰 보자."],
        talk_received: &["돌아왔는데, 정말 같은 곳일까?", "창밖 풍경이 멈췄다. 방은 그대로다."],
        talk_done: &["...여기가 집이었나."],
        machine_done: &["운전대 기계는 조용하다."],
    };
    // 화장실의 기사 — 게임 받기 기계가 없고(machine_* 는 안 쓴다) 기사의 의뢰를 수락하면 PACK 이 메일로 온다.
    static FINGER: MissionText = MissionText {
        talk_first: &["갑옷 차림의 기사가 변기 앞을 서성이고 있다.", "내 칼을 잃어버려서 화장실을 못 쓰겠소.", "내 칼을 찾아주시오."],
        talk_waiting: &["이 게임의 끝에 나의 칼이 있소.", "...찾아주시오."],
        machine_locked: &[],
        machine_send: &[],
        machine_waiting: &[],
        received: &["손이 내민 변기 손잡이를 받았다.", "손은 다시 변기 속으로 들어갔다.", "'변기 손잡이'를 얻었다."],
        talk_received: &["기사가 변기 손잡이를 쓰다듬는다.", "나의 칼이 돌아왔소. 이제 화장실을 쓸 수 있겠군.", "기사는 만족스럽게 변기 앞에 자리를 잡았다."],
        talk_done: &["...사용 중이오."],
        machine_done: &[],
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
        MiniKind::Brain => "소환 조각(레이싱)",
        MiniKind::Finger => "소환 조각(리듬)",
    }
}

pub(super) fn door_name(kind: MiniKind) -> &'static str {
    match kind {
        MiniKind::Eye => "눈 표식 문",
        MiniKind::Brain => "금고가 보이는 문",
        MiniKind::Finger => "화장실 문",
    }
}
