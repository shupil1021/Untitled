//! OS(DesktopScene)가 창 안 게임에게 알려 주는 "지금 이런 상태다" 스냅숏 — 게임 앱은 FileSystem 을 직접
//! 못 보기 때문에(open() 이 만들 때 한 번 넘겨받는 값은 열려 있는 동안 안 바뀐다), OS 가 매 프레임 최신 값을
//! `publish` 로 써두고 게임이 `current` 로 읽는다. (마우스 시점 요청이 scenes::request_mouse_look 으로 반대
//! 방향으로 신호를 보내는 것과 같은 요령.)
//!
//! 일회성 이벤트 큐가 아니라 **상태 스냅숏**인 이유: 진실은 fs(저장되는 쪽)에 있고, 게임 창이 닫혀 있다가
//! 다시 열려도 "이미 메일을 읽었다 / 사진을 내려받았다"를 바로 알아야 한다(CRT SYS-2 중단 복구). 이벤트는 창이
//! 닫힌 사이에 사라지지만 상태는 매 프레임 다시 쓰이므로 놓치는 일이 없다. 새 신호가 필요하면 `OsState` 에 필드
//! 하나만 더하면 된다.

use std::sync::Mutex;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct OsState {
    /// 문(도어즈)의 편지 메일을 받은편지함에서 읽었다 — 문 게임에서 문이 달라지는 조건.
    pub letter_mail_read: bool,
    /// 서브 게임이 보낸 꽃 사진(flower.png)을 메일에서 내려받았다 — 메인 게임을 열면 꽃이 인벤토리에 들어오는 조건(SUB A+a-20).
    pub flower_downloaded: bool,
    /// 미니게임 보상 사진(충혈된 눈 / 금고 절반 / …)을 내려받았다 — 메인 게임이 그 재료를 자동으로 받는 조건. 인덱스는 MiniKind::index().
    pub reward_downloaded: [bool; 3],
    /// 리듬 미니게임에서 변기 손잡이를 호수에 넣었다 — 화장실 변기 속 손이 나타나는 조건(MAIN H-3).
    pub lake_delivered: bool,
    /// 가상 시계 차이(초) — 게임 안 "지금 시각"은 실제 PC 시간 + 이 값이다(fs.clock_offset_secs).
    pub clock_offset_secs: i64,
    /// 플레이어가 고친 game.ini 가 실제 진행과 다르면 그 모양 — 문이 한마디 한다(gamefiles::ini_edit).
    pub ini_edit: crate::gamefiles::IniEdit,
}

static STATE: Mutex<OsState> = Mutex::new(OsState {
    letter_mail_read: false,
    flower_downloaded: false,
    reward_downloaded: [false; 3],
    lake_delivered: false,
    clock_offset_secs: 0,
    ini_edit: crate::gamefiles::IniEdit::None,
});

// 락이 독이 되어도(다른 스레드가 패닉) 값 자체는 멀쩡하므로 그대로 쓴다.
fn lock() -> std::sync::MutexGuard<'static, OsState> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// OS 쪽(desktop/timers.rs): 이번 프레임의 최신 상태로 통째로 덮어쓴다.
pub fn publish(state: OsState) {
    *lock() = state;
}

/// 게임 쪽: 가장 최근에 발행된 상태.
pub fn current() -> OsState {
    *lock()
}

/// 일부 필드만 바꾼다(테스트용 — 실제 게임에서는 OS 가 `publish` 로만 쓴다).
pub fn update(f: impl FnOnce(&mut OsState)) {
    f(&mut lock());
}

// 실제 시각(유닉스 초)에 가상 시계 차이를 더한 "게임 안 지금".
pub fn effective_at(real_now: f64, offset_secs: i64) -> f64 {
    real_now + offset_secs as f64
}

/// 서브 게임의 시간 퍼즐이 보는 지금(유닉스 초) — 실제 PC 시계를 돌려도, 가상 시계를 맞춰도 앞으로 간다.
pub fn effective_now() -> f64 {
    effective_at(miniquad::date::now(), current().clock_offset_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_time_adds_the_virtual_offset() {
        assert_eq!(effective_at(1000.0, 0), 1000.0);
        assert_eq!(effective_at(1000.0, 365 * 86_400), 1000.0 + 31_536_000.0);
        assert_eq!(effective_at(1000.0, -500), 500.0);
    }
}
