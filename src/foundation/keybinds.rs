//! 3D 게임/미니게임의 조작 키 — 설정에서 다시 지정할 수 있다. 저장 파일에는 키 이름(문자열)으로 들어가고(미니쿼드의
//! KeyCode 는 저장 형식이 아니다), 알 수 없는 이름은 기본 키로 되돌아간다. 방향키는 이동의 보조 키로 항상 같이 먹는다.

use miniquad::KeyCode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Interact,
    Sprint,
}

impl Action {
    pub const ALL: [Action; 6] = [Action::Forward, Action::Back, Action::Left, Action::Right, Action::Interact, Action::Sprint];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&a| a == self).expect("ALL 에 모든 동작이 있다")
    }

    fn default_key(self) -> KeyCode {
        match self {
            Action::Forward => KeyCode::W,
            Action::Back => KeyCode::S,
            Action::Left => KeyCode::A,
            Action::Right => KeyCode::D,
            Action::Interact => KeyCode::E,
            Action::Sprint => KeyCode::LeftShift,
        }
    }
}

// 다시 지정할 수 있는 키들 — (이름, 키). 이름이 저장 형식이자 설정창에 보이는 글자다.
pub const BINDABLE: &[(&str, KeyCode)] = &[
    ("A", KeyCode::A), ("B", KeyCode::B), ("C", KeyCode::C), ("D", KeyCode::D), ("E", KeyCode::E), ("F", KeyCode::F),
    ("G", KeyCode::G), ("H", KeyCode::H), ("I", KeyCode::I), ("J", KeyCode::J), ("K", KeyCode::K), ("L", KeyCode::L),
    ("M", KeyCode::M), ("N", KeyCode::N), ("O", KeyCode::O), ("P", KeyCode::P), ("Q", KeyCode::Q), ("R", KeyCode::R),
    ("S", KeyCode::S), ("T", KeyCode::T), ("U", KeyCode::U), ("V", KeyCode::V), ("W", KeyCode::W), ("X", KeyCode::X),
    ("Z", KeyCode::Z),
    ("Up", KeyCode::Up), ("Down", KeyCode::Down), ("Left", KeyCode::Left), ("Right", KeyCode::Right),
    ("Space", KeyCode::Space), ("Enter", KeyCode::Enter), ("Tab", KeyCode::Tab),
    ("LeftShift", KeyCode::LeftShift), ("RightShift", KeyCode::RightShift), ("LeftControl", KeyCode::LeftControl),
];

pub fn key_from_name(name: &str) -> Option<KeyCode> {
    BINDABLE.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

pub fn name_of(key: KeyCode) -> Option<&'static str> {
    BINDABLE.iter().find(|(_, k)| *k == key).map(|(n, _)| *n)
}

// 키 하나를 눌렀는지 이름 없이도 알아볼 수 있는 화면 표기(설정창 버튼 글자).
pub fn key_label(key: KeyCode) -> String {
    match name_of(key) {
        Some("LeftShift") => "Shift".to_string(),
        Some("RightShift") => "R-Shift".to_string(),
        Some("LeftControl") => "Ctrl".to_string(),
        Some(n) => n.to_string(),
        None => "?".to_string(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBinds {
    names: [String; 6], // Action::index() 순서
}

impl Default for KeyBinds {
    fn default() -> Self {
        KeyBinds { names: Action::ALL.map(|a| name_of(a.default_key()).expect("기본 키는 다시 지정할 수 있는 키다").to_string()) }
    }
}

impl KeyBinds {
    // 이 동작에 지정된 키 — 저장 파일에 알 수 없는 이름이 있으면 기본 키.
    pub fn key(&self, action: Action) -> KeyCode {
        key_from_name(&self.names[action.index()]).unwrap_or_else(|| action.default_key())
    }

    // 모든 동작의 키(게임 코드가 매 프레임 읽는 형태).
    pub fn resolved(&self) -> [KeyCode; 6] {
        Action::ALL.map(|a| self.key(a))
    }

    // 이 동작의 키를 바꾼다. 이미 다른 동작이 그 키를 쓰고 있으면 서로 바꿔서(스왑) 한 키가 두 동작을 맡지 않게 한다.
    // 다시 지정할 수 없는 키면 false.
    pub fn set(&mut self, action: Action, key: KeyCode) -> bool {
        let Some(name) = name_of(key) else { return false };
        let old = self.names[action.index()].clone();
        for other in Action::ALL {
            if other != action && self.key(other) == key {
                self.names[other.index()] = old.clone();
            }
        }
        self.names[action.index()] = name.to_string();
        true
    }

    pub fn reset(&mut self) {
        *self = KeyBinds::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_wasd_e_and_shift() {
        let k = KeyBinds::default();
        assert_eq!(k.resolved(), [KeyCode::W, KeyCode::S, KeyCode::A, KeyCode::D, KeyCode::E, KeyCode::LeftShift]);
    }

    #[test]
    fn every_bindable_name_round_trips() {
        for &(name, key) in BINDABLE {
            assert_eq!(key_from_name(name), Some(key));
            assert_eq!(name_of(key), Some(name));
        }
        assert_eq!(key_from_name("Nonsense"), None);
    }

    #[test]
    fn rebinding_a_used_key_swaps_instead_of_duplicating() {
        let mut k = KeyBinds::default();
        assert!(k.set(Action::Interact, KeyCode::F));
        assert_eq!(k.key(Action::Interact), KeyCode::F);
        // 앞으로 가기를 이미 앞에서 쓰는 F(상호작용)로 바꾸면 둘이 서로 바뀐다.
        assert!(k.set(Action::Forward, KeyCode::F));
        assert_eq!(k.key(Action::Forward), KeyCode::F);
        assert_eq!(k.key(Action::Interact), KeyCode::W, "빼앗긴 동작은 상대가 쓰던 키를 받는다");
        let keys = k.resolved();
        let mut sorted = keys.to_vec();
        sorted.sort_by_key(|c| *c as u32);
        sorted.dedup();
        assert_eq!(sorted.len(), 6, "어느 키도 두 동작에 묶이지 않는다");
    }

    #[test]
    fn unbindable_keys_are_refused_and_unknown_saved_names_fall_back() {
        let mut k = KeyBinds::default();
        assert!(!k.set(Action::Left, KeyCode::Escape), "Esc 는 시점 해제용이라 지정할 수 없다");
        assert_eq!(k.key(Action::Left), KeyCode::A);
        let broken: KeyBinds = serde_json::from_str(r#"{"names":["???","S","A","D","E","LeftShift"]}"#).unwrap();
        assert_eq!(broken.key(Action::Forward), KeyCode::W, "알 수 없는 이름은 기본 키");
    }

    #[test]
    fn old_saves_without_keys_load_with_defaults_and_reset_restores_them() {
        let k: KeyBinds = serde_json::from_str("{}").unwrap();
        assert_eq!(k, KeyBinds::default());
        let mut k = KeyBinds::default();
        k.set(Action::Sprint, KeyCode::Space);
        k.reset();
        assert_eq!(k, KeyBinds::default());
    }
}
