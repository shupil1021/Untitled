//! OS(DesktopScene)와 창 안 게임이 주고받는 가벼운 상태 신호 — 게임 앱은 FileSystem 을 직접
//! 못 보기 때문에(open() 이 만들 때 한 번 넘겨받는 값은 열려 있는 동안 안 바뀐다), OS 가 매
//! 프레임 최신 값을 여기 써두고 게임이 읽는다. (마우스 시점 요청이 scenes::request_mouse_look 으로
//! 반대 방향으로 신호를 보내는 것과 같은 요령.)

use std::sync::atomic::{AtomicBool, Ordering};

static LETTER_MAIL_READ: AtomicBool = AtomicBool::new(false);

// 문(도어즈)의 편지 메일을 받은편지함에서 읽었는지 — 문 게임에서 문이 달라지는 조건.
pub fn set_letter_mail_read(read: bool) {
    LETTER_MAIL_READ.store(read, Ordering::Relaxed);
}

pub fn letter_mail_read() -> bool {
    LETTER_MAIL_READ.load(Ordering::Relaxed)
}

static FLOWER_DOWNLOADED: AtomicBool = AtomicBool::new(false);

// 서브 게임이 보낸 꽃 사진(flower.png)을 메일에서 내려받았는지 — 메인 게임을 열면 꽃이 인벤토리에
// 들어오는 조건이다(SUB A+a-20).
pub fn set_flower_downloaded(done: bool) {
    FLOWER_DOWNLOADED.store(done, Ordering::Relaxed);
}

pub fn flower_downloaded() -> bool {
    FLOWER_DOWNLOADED.load(Ordering::Relaxed)
}

static REWARD_DOWNLOADED: [AtomicBool; 3] = [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)];

// 미니게임 보상 사진(충혈된 눈 / 짓무른 뇌 / 절단된 손가락)을 메일에서 내려받았는지 — 메인 게임이 그 재료를
// 자동으로 받는 조건이다(MAIN E-3 / R-3 / H-3). 인덱스는 MiniKind::index().
pub fn set_reward_downloaded(kind: usize, done: bool) {
    REWARD_DOWNLOADED[kind].store(done, Ordering::Relaxed);
}

pub fn reward_downloaded(kind: usize) -> bool {
    REWARD_DOWNLOADED[kind].load(Ordering::Relaxed)
}

static LAKE_DELIVERED: AtomicBool = AtomicBool::new(false);

// 리듬 미니게임에서 변기 손잡이를 호수에 넣었는지 — 메인 게임 화장실에서 변기 속 손이 나타나는 조건이다(MAIN H-3).
pub fn set_lake_delivered(done: bool) {
    LAKE_DELIVERED.store(done, Ordering::Relaxed);
}

pub fn lake_delivered() -> bool {
    LAKE_DELIVERED.load(Ordering::Relaxed)
}
