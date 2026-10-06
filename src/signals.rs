//! OS(DesktopScene)와 창 안 게임이 주고받는 가벼운 상태 신호 — 게임 앱은 FileSystem 을 직접
//! 못 보기 때문에(open() 이 만들 때 한 번 넘겨받는 값은 열려 있는 동안 안 바뀐다), OS 가 매
//! 프레임 최신 값을 여기 써두고 게임이 읽는다. (마우스 시점 요청이 scenes::request_mouse_look 으로
//! 반대 방향으로 신호를 보내는 것과 같은 요령.)

use std::sync::atomic::{AtomicBool, Ordering};

static LETTER_MAIL_READ: AtomicBool = AtomicBool::new(false);

// 문(도어즈)의 편지 메일을 받은편지함에서 읽었는지 — 문 게임이 일시 정지에서 풀릴 조건.
pub fn set_letter_mail_read(read: bool) {
    LETTER_MAIL_READ.store(read, Ordering::Relaxed);
}

pub fn letter_mail_read() -> bool {
    LETTER_MAIL_READ.load(Ordering::Relaxed)
}
