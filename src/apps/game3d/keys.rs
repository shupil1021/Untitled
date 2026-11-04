//! 3D 게임/미니게임이 읽는 "지금 조작 키" — 설정에서 다시 지정한 키를 main 이 프레임마다 알려 준다(i18n 의 언어와 같은
//! 요령). 방향키는 이동의 보조 키로, Enter 는 상호작용의 보조 키로 항상 같이 먹는다.

use std::sync::Mutex;

use miniquad::KeyCode;

use crate::foundation::Action;
use crate::scenes::Input;

static KEYS: Mutex<[KeyCode; 6]> = Mutex::new([KeyCode::W, KeyCode::S, KeyCode::A, KeyCode::D, KeyCode::E, KeyCode::LeftShift]);

pub fn set(resolved: [KeyCode; 6]) {
    *KEYS.lock().unwrap_or_else(|e| e.into_inner()) = resolved;
}

// 지금 이 동작에 지정된 (기본) 키 — 안내 문구에 키 이름을 보여 줄 때도 쓴다.
pub fn code(action: Action) -> KeyCode {
    KEYS.lock().unwrap_or_else(|e| e.into_inner())[action.index()]
}

// 항상 같이 먹는 보조 키.
fn alternates(action: Action) -> &'static [KeyCode] {
    match action {
        Action::Forward => &[KeyCode::Up],
        Action::Back => &[KeyCode::Down],
        Action::Left => &[KeyCode::Left],
        Action::Right => &[KeyCode::Right],
        Action::Interact => &[KeyCode::Enter],
        Action::Sprint => &[KeyCode::RightShift],
    }
}

// 눌려 있는 동안 true.
pub fn down(input: &Input, action: Action) -> bool {
    input.is_down(code(action)) || alternates(action).iter().any(|&k| input.is_down(k))
}

// 눌린 순간 한 번 true.
pub fn pressed(input: &Input, action: Action) -> bool {
    input.pressed(code(action)) || alternates(action).iter().any(|&k| input.pressed(k))
}

// 안내 문구 속 "[E]" 를 지금 상호작용 키 이름으로 바꾼다(키를 바꿔도 문구가 맞게).
pub fn localize_key_hint(text: &str) -> String {
    text.replacen("[E]", &format!("[{}]", crate::foundation::key_label(code(Action::Interact))), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 보조 키(방향키/Enter)는 어떤 지정이어도 동작한다 — 전역 상태를 건드리지 않고 보조 키 목록만 확인한다.
    #[test]
    fn arrows_always_back_up_the_movement_keys() {
        assert_eq!(alternates(Action::Forward), &[KeyCode::Up]);
        assert_eq!(alternates(Action::Left), &[KeyCode::Left]);
        assert_eq!(alternates(Action::Interact), &[KeyCode::Enter]);
        let mut input = Input::default();
        input.on_key_down(KeyCode::Up, false);
        assert!(alternates(Action::Forward).iter().any(|&k| input.is_down(k)));
    }
}
