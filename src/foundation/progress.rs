//! 게임 진행 상태와 이름표 — 창 안 게임들(메인/서브/미니게임)의 저장되는 진행, 받은편지함에 올 수 있는 메일의
//! 종류(MailId), 설치되는 게임 파일 이름 상수.

use serde::{Deserialize, Serialize};

// ---- 창 안 게임들의 진행 상황(저장 파일에 실려서 창을 닫았다 다시 열어도 이어진다) ----
// 앱은 열릴 때 fs 의 값을 복사해 받고, 바뀔 때마다 AppAction::SaveDoors/SaveMaze 로 돌려줘서
// desktop.rs 가 fs 에 쓰고 저장한다. "Erase All Memory" 는 저장 파일을 지우므로 같이 초기화된다.

// 서브 게임 열쇠 개수(문 3개, 열쇠 3개).
pub const MAZE_KEYS: usize = 3;

// 메인 게임(방 + 문) 진행.
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct DoorsProgress {
    pub quest_accepted: bool,        // 도어즈의 부탁(보리지꽃)을 수락했다
    pub has_letter: bool,            // 책상 위 편지를 집었다
    pub letter_sent: bool,           // 편지를 우편함에 넣었다
    pub door_changed: bool,          // 편지 메일을 읽고 돌아와서 문의 대사가 바뀌었다
    pub flower_absence_checked: bool, // 방에 꽃이 없다는 걸 확인했다
    #[serde(default)]
    pub flower_received: bool,       // 서브 게임에서 보낸 꽃이 인벤토리에 들어왔다(SUB A+a-20)
    #[serde(default)]
    pub flower_given: bool,          // 도어즈에게 꽃을 줬다(MAIN A-4)
    #[serde(default)]
    pub tutorial_done: bool,         // 문 잠금이 풀리고 1장 튜토리얼이 끝났다(MAIN A-5)
    #[serde(default)]
    pub place: MainPlace,            // 지금 있는 곳 — 창을 닫았다 열어도 거기서 이어진다
    #[serde(default)]
    pub jungle_visited: bool,        // 2층 정글에 이미 나와 봤다(처음엔 상황 설명 대사 + 메일)
    #[serde(default)]
    pub stair_met: bool,             // 계단 NPC 와 처음 이야기했다(MAIN B-2)
    #[serde(default)]
    pub errand_accepted: bool,       // 소파 NPC 의 재료 의뢰를 수락했다(MAIN B-3) — 미션 문 셋이 열린다
    #[serde(default)]
    pub missions: [MissionStep; 3],  // 눈 / 뇌 / 손가락 미션 진행(MiniKind::index())
    #[serde(default)]
    pub exchanged: bool,             // 재료 셋을 소파 NPC 에게 건넸다(MAIN C-1)
    #[serde(default)]
    pub chip_taken: bool,            // 소파 위 봉투에서 Mr. POTATO CHIP 을 받았다(MAIN C-2)
    #[serde(default)]
    pub chip_fed: bool,              // 계단 NPC 에게 먹였다(MAIN C-4)
    #[serde(default)]
    pub floor2_done: bool,           // 2층을 끝냈다(MAIN C-5)
    #[serde(default)]
    pub pose: Option<[f32; 4]>,      // 창이 뒤로 가기 직전 서 있던 자리(x, y, z, yaw) — 이어서 할 때 거기서 시작
}

// 2층 미션 하나의 진행 단계.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum MissionStep {
    #[default]
    Start,    // 아직 NPC 와 이야기하지 않았다
    Talked,   // NPC 의 의뢰를 들었다(E-1)
    Sent,     // 기계에서 게임을 받았다 — CRT 로 설치 파일 메일이 갔다(E-2)
    Received, // 보상 사진을 내려받아 재료가 인벤토리에 들어왔다(E-3)
    Done,     // NPC 의 마지막 이야기까지 마쳤다(E-4)
}

// 메인 게임의 장소.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MainPlace {
    #[default]
    StartRoom, // 1장의 시작방
    Jungle,    // 2층 정글 허브
    MissionRoom(MiniKind), // 눈 / 뇌 / 손가락의 방
    StairHall, // 계단 입구
}

// 서브 게임(미로 + 방) 진행.
#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct MazeProgress {
    pub has_can: bool,
    pub pot_checked: bool,    // 화분을 조사했다 → 플레이어가 등을 돌렸을 때 사물함이 나타난다
    #[serde(default)]
    pub locker_seen: bool,    // 사물함이 나타났다(화분 조사 뒤, 시야 밖에서 생긴다)
    pub locker_checked: bool, // 사물함을 조사했다 → 문 3개가 열린다
    pub keys: [bool; MAZE_KEYS],
    pub seed: bool,
    pub planted: bool,
    pub planted_at: Option<f64>, // 씨앗을 심은 때(시스템 시간, 유닉스 초)
    pub bloomed: bool,           // 시스템 시간이 1년 넘게 흘러 꽃이 폈다
    pub has_flower: bool,        // 꽃을 꺾어서 가지고 있다
    pub flower_sent: bool,       // 꽃을 우체통에 넣어 보냈다
    pub hub_visited: bool,       // 방에 이미 한 번 올라와 봤다(처음엔 상황 설명 대사가 나온다)
}

// 2층 미션 방 셋에서 받는 2D 미니게임들 — 눈(탄막) / 뇌(레이싱) / 손가락(포장 리듬).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum MiniKind {
    Eye,
    Brain,
    Finger,
}

impl MiniKind {
    pub const ALL: [MiniKind; 3] = [MiniKind::Eye, MiniKind::Brain, MiniKind::Finger];

    pub fn index(self) -> usize {
        match self {
            MiniKind::Eye => 0,
            MiniKind::Brain => 1,
            MiniKind::Finger => 2,
        }
    }

    // 게임 이름(.exe 뗀 것) — 설치 마법사 문구와 AppData 폴더 이름에 쓴다.
    pub fn app_name(self) -> &'static str {
        match self {
            MiniKind::Eye => "SHY_EYE",
            MiniKind::Brain => "BRAIN_DRIVE",
            MiniKind::Finger => "PACK",
        }
    }

    pub fn exe_name(self) -> &'static str {
        match self {
            MiniKind::Eye => "SHY_EYE.exe",
            MiniKind::Brain => "BRAIN_DRIVE.exe",
            MiniKind::Finger => "PACK.exe",
        }
    }

    pub fn setup_name(self) -> &'static str {
        match self {
            MiniKind::Eye => "SHY_EYE Setup.exe",
            MiniKind::Brain => "BRAIN_DRIVE Setup.exe",
            MiniKind::Finger => "PACK Setup.exe",
        }
    }

    // 보상 사진 파일 이름과 gamefiles::generated_images() 안의 인덱스.
    pub fn image_name(self) -> &'static str {
        match self {
            MiniKind::Eye => "bloodshot_eye.png",
            MiniKind::Brain => "worn_brain.png",
            MiniKind::Finger => "severed_finger.png",
        }
    }

    pub fn image_index(self) -> usize {
        crate::gamefiles::REWARD_IMAGE_BASE + self.index()
    }

    // 이 미니게임의 안내 메일 / 보상 메일.
    pub fn task_mail(self) -> MailId {
        match self {
            MiniKind::Eye => MailId::EyeTask,
            MiniKind::Brain => MailId::BrainTask,
            MiniKind::Finger => MailId::FingerTask,
        }
    }

    pub fn reward_mail(self) -> MailId {
        match self {
            MiniKind::Eye => MailId::EyeReward,
            MiniKind::Brain => MailId::BrainReward,
            MiniKind::Finger => MailId::FingerReward,
        }
    }
}

// 미니게임 하나의 진행 — 중간에 창을 닫아도 유지된다(판 하나가 도중에 끊기면 그 판만 처음부터).
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MiniProgress {
    pub cleared: bool,       // 클리어해서 보상 상자가 나왔다
    pub reward_taken: bool,  // 상자에서 재료를 꺼냈다
    pub reward_sent: bool,   // 재료를 우체통에 넣어 보냈다
}

// 받은편지함에 도착할 수 있는 메일들 — 도착 순서는 게임 진행에 따라 달라서(mail_log) 번호가
// 아니라 이 이름으로 구분한다.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MailId {
    Friend, // 친구가 보낸 게임 설치 파일 메일(타이머로 도착)
    Letter, // 문(도어즈)의 편지 — 방의 우편함에 넣으면 도착
    Flower, // "내가 꽃을 구할 수 있는 곳을 알고 있어" — 방에 꽃이 없다는 걸 확인하면 도착
    Hint,   // "바탕화면에서 시간을 돌려도..." — 서브 게임에서 씨앗을 심으면 도착
    Photo,  // 꽃 사진 첨부 메일 — 꽃을 우체통에 넣으면 도착
    Outside, // "밖이라고?" — 2층 정글에 처음 나오면 도착(MAIN B-1)
    Shopping, // "장보기 목록" — 소파 NPC 의 재료 의뢰를 수락하면 도착(MAIN B-3)
    EyeTask,  // 눈의 방 탄막 게임 설치 파일 메일(MAIN E-2 → CRT E-0)
    EyeReward,
    BrainTask,
    BrainReward,
    FingerTask,
    FingerReward,
    Meal, // "식사 준비" — 소파 NPC 에게서 Mr. POTATO CHIP 을 받으면 도착(MAIN C-2)
}
// File Explorer 의 "AppData" 탭 = 이 이름의 폴더 노드의 내용(탭 이름도 같은 문자열).
pub const APPDATA_NAME: &str = "AppData";
// 첫 메일에 첨부돼 오는 게임 설치 파일(FileKind::GameSetup)과, 설치가 끝나면
// 바탕화면에 생기는 게임 아이콘(FileKind::Game)의 fs 이름.
pub const GAME_SETUP_NAME: &str = "test Setup.exe";
pub const GAME_FILE_NAME: &str = "test.exe";
// 설치할 때 게임 안 AppData 폴더 밑에 만들어지는 게임 폴더 이름(gamefiles.rs) — 게임 이름(.exe 뗀 것).
pub const GAME_FOLDER_NAME: &str = "test";
pub const SUB_GAME_NAME: &str = "test2.exe";

