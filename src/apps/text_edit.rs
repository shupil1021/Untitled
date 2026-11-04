//! 글자 입력 공용 처리 — 메일 작성창과 메모장이 같이 쓴다. 커서(글자 인덱스) 자리에 글자를 끼워 넣고 지우는 도우미와,
//! 한 프레임의 키보드 입력(확정된 글자, IME 조합 중 Backspace, Backspace 꾹 누르기 반복)을 처리하는 `EditState`.
//! 한/일 문자는 여러 바이트라 커서를 바이트 인덱스로 쓰지 않고 char_indices 로 몇 번째 글자가 어느 바이트에서 시작하는지
//! 먼저 찾는다(안 그러면 글자 중간을 끊어 버린다).

use miniquad::KeyCode;

use super::WinInput;

// 커서(글자 인덱스) 자리에 끼워 넣는다 — max_chars 를 넘으면 무시한다.
pub(crate) fn insert_char_at(s: &mut String, cursor: &mut usize, c: char, max_chars: usize) {
    if s.chars().count() >= max_chars {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor).map(|(b, _)| b).unwrap_or(s.len());
    s.insert(byte_idx, c);
    *cursor += 1;
}

// 커서 바로 앞 글자를 지운다.
pub(crate) fn backspace_at(s: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor - 1).map(|(b, _)| b).unwrap_or(0);
    s.remove(byte_idx);
    *cursor -= 1;
}

// Backspace 를 꾹 누르고 있을 때 지금까지 몇 번 지워졌어야 하는지 — 누른 순간 한 번(즉시), REPEAT_DELAY 초까지 그대로 있으면 그 뒤로
// REPEAT_INTERVAL 마다 하나씩 더. hold 는 지금 이 키가 계속 눌려있던 시간(초).
pub(crate) const BACKSPACE_REPEAT_DELAY: f32 = 0.3;
pub(crate) const BACKSPACE_REPEAT_INTERVAL: f32 = 0.035;
// 한 프레임에 hold 누적치를 이보다 많이는 안 늘린다 — main.rs 의 전역 dt 상한(500ms)만으론 이 반복 판정 기준으로 여전히 너무 커서,
// 큰 프레임 끊김이 한 번 있으면 그 한 프레임에 반복 삭제가 몰아서 발생할 수 있었다.
pub(crate) const BACKSPACE_MAX_DT_PER_FRAME: f32 = 0.02;
pub(crate) fn backspace_repeats_due(hold: f32) -> u32 {
    if hold <= 0.0 {
        return 0;
    }
    if hold < BACKSPACE_REPEAT_DELAY {
        return 1;
    }
    2 + ((hold - BACKSPACE_REPEAT_DELAY) / BACKSPACE_REPEAT_INTERVAL) as u32
}

// 한 입력창이 프레임을 넘어 기억해야 하는 키보드 입력 상태.
#[derive(Default)]
pub(crate) struct EditState {
    // Backspace 를 꾹 누르고 있는 시간(초)과, 이 hold 동안 이미 처리한 삭제 횟수(중복 처리 방지).
    backspace_hold: f32,
    backspace_fired: u32,
    // 지금 이어지는 한 번의 누름(키를 떼기 전까지)이 IME 조합 중에 시작됐으면 true — 사람이 키를 누르고 있는 몇 프레임 사이에 IME 가
    // 조합을 끝내버려도(그 뒤 프레임엔 composing 이 false 로 보여도) 같은 한 번의 누름 안에서는 IME 가 이미 처리한 것으로 치고 우리가
    // 또 지우지 않는다.
    backspace_owned_by_ime: bool,
    // 지금 이어지는 조합(composition) 세션 동안 IME 가 화면에 보여준 조합 문자열의 스냅샷들 — "ㅎ" → "호" → "화". Backspace 를 눌렀을 때
    // "한 단계 전엔 뭐였는지"를 (IME 내부 동작이 그 사이 뭘 했든 상관없이) 이 기록에서 그대로 꺼내 쓴다. 조합이 끝나면 비운다.
    composition_history: Vec<String>,
}

impl EditState {
    // 이번 프레임의 확정된 글자 입력과 Backspace(IME 조합 중 포함, 꾹 누르기 반복 포함)를 text 에 반영한다. 글자가 바뀌었으면 true.
    // 결합 전 자모 여러 개(예: "ㄲ" 이전의 "ㄱㄱ")를 조합 중일 때 Backspace 한 번 = 한글 IME 가 자모 전체를 한 번에 지워버리는 문제는
    // IME 내부 동작을 계산/가로채서 맞추려던 시도가 전부 실패했다(경위는 README). 대신 이 조합 세션 동안 IME 가 실제로 보여준 화면을
    // 기록해뒀다가 "한 단계 전 기록"을 그대로 꺼내 쓴다 — 계산이 아니라 조회라 틀릴 여지가 없다. 조합 버퍼에 다시 넣는 건 안 먹히므로
    // 조합을 취소하고(`ime::cancel_composition`) 그 값을 평범한 확정 문자로 직접 밀어 넣는다. 짧게 한 번 누른 경우만 처리한다.
    //
    // suppress_backspace 가 true 면 이번 프레임의 Backspace 는 호출한 쪽이 이미 처리했다(예: 메모장에서 선택 영역을 지웠다) —
    // 한 글자 더 지우지 않는다.
    pub(crate) fn edit(&mut self, text: &mut String, cursor: &mut usize, max_chars: usize, win: &WinInput, suppress_backspace: bool) -> bool {
        let mut changed = false;
        for &c in &win.input.typed {
            if !c.is_control() {
                insert_char_at(text, cursor, c, max_chars);
                changed = true;
            }
        }
        let backspace_pressed = win.input.pressed(KeyCode::Backspace) && !suppress_backspace;
        let backspace_down = win.input.is_down(KeyCode::Backspace) && !suppress_backspace;
        let cur_composition = crate::platform::ime::composition_preview().unwrap_or_default();
        if backspace_pressed && !self.composition_history.is_empty() {
            let target = if self.composition_history.len() >= 2 { self.composition_history[self.composition_history.len() - 2].clone() } else { String::new() };
            crate::platform::ime::cancel_composition();
            for ch in target.chars() {
                insert_char_at(text, cursor, ch, max_chars);
            }
            changed |= !target.is_empty();
            // 취소했으니 이 조합 세션은 끝났다 — 기록도 같이 비운다.
            self.composition_history.clear();
        } else if cur_composition.is_empty() {
            self.composition_history.clear();
        } else if self.composition_history.last() != Some(&cur_composition) {
            self.composition_history.push(cur_composition);
        }
        let composing = crate::platform::ime::composition_preview().is_some_and(|p| !p.is_empty());
        if !backspace_down {
            self.backspace_hold = 0.0;
            self.backspace_fired = 0;
            self.backspace_owned_by_ime = false;
        } else if composing {
            self.backspace_owned_by_ime = true;
            self.backspace_hold = 0.0;
            self.backspace_fired = 0;
        } else if !self.backspace_owned_by_ime {
            // win.dt 자체엔 500ms 상한(main.rs)이 있지만 이 기준으론 여전히 너무 크다 — "누적된 hold 시간 → 반복 횟수"로 바로 환산하는
            // 로직이라 큰 dt 하나가 "그동안 계속 눌려있던 것"으로 오인되면 한 프레임에 반복 삭제가 몰아서 일어난다. 그래서 한 프레임에
            // 반영하는 dt 를 별도 상한(BACKSPACE_MAX_DT_PER_FRAME)으로 한 번 더 눌러 둔다.
            self.backspace_hold += win.dt.min(BACKSPACE_MAX_DT_PER_FRAME);
        }
        let due = backspace_repeats_due(self.backspace_hold);
        let fire = due.saturating_sub(self.backspace_fired).min(6);
        if fire > 0 {
            self.backspace_fired = due;
            for _ in 0..fire {
                backspace_at(text, cursor);
            }
            changed = true;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_backspace_work_on_char_positions_not_bytes() {
        let mut s = String::from("한글");
        let mut cur = 1;
        insert_char_at(&mut s, &mut cur, 'ㅁ', 10);
        assert_eq!((s.as_str(), cur), ("한ㅁ글", 2));
        backspace_at(&mut s, &mut cur);
        assert_eq!((s.as_str(), cur), ("한글", 1));
        backspace_at(&mut s, &mut cur);
        backspace_at(&mut s, &mut cur); // 맨 앞에서는 더 안 지워진다
        assert_eq!((s.as_str(), cur), ("글", 0));
    }

    #[test]
    fn insert_respects_the_character_limit() {
        let mut s = String::from("ab");
        let mut cur = 2;
        insert_char_at(&mut s, &mut cur, 'c', 2);
        assert_eq!((s.as_str(), cur), ("ab", 2));
    }

    #[test]
    fn holding_backspace_repeats_after_a_delay() {
        assert_eq!(backspace_repeats_due(0.0), 0);
        assert_eq!(backspace_repeats_due(0.05), 1, "누른 순간 한 번");
        assert_eq!(backspace_repeats_due(BACKSPACE_REPEAT_DELAY - 0.01), 1);
        assert_eq!(backspace_repeats_due(BACKSPACE_REPEAT_DELAY), 2);
        assert!(backspace_repeats_due(BACKSPACE_REPEAT_DELAY + BACKSPACE_REPEAT_INTERVAL * 10.0) >= 11);
    }
}
