//! Windows 가 아닌 곳의 IME 대체 구현 — 조합 중 문자열을 읽을 방법이 없으니 항상 "조합 없음".

pub fn composition_preview() -> Option<String> {
    None
}

pub fn set_composition_pos(_x: i32, _y: i32) {}

pub fn hide_cicero_windows() {}

pub fn cancel_composition() {}
