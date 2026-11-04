//! .txt 메모장 — 읽기만 하는 게 아니라 직접 고칠 수 있다. 클릭으로 커서를 옮기고, 타이핑/Backspace(꾹 누르기 반복)/Delete/Enter/
//! 방향키/Home/End, **글자 범위 선택**(마우스 드래그, Shift+클릭, Shift+방향키/Home/End)과 Ctrl+A(전체 선택)/Ctrl+C(복사)/Ctrl+X(잘라내기)/
//! Ctrl+V(붙여넣기)가 되고, 한글/일본어 IME 조합은 메일 작성창과 같은 text_edit::EditState 로 처리한다. 선택한 채로 글자를 치거나 지우면
//! 선택한 부분이 바뀐다. 고친 내용은 잠깐 쉬면(AUTOSAVE_SECS) 또는 창이 포커스를 잃거나 닫힐 때 자동으로 저장된다(AppAction::SaveText).
//! 고친 파일은 "플레이어가 고친 파일"로 기록돼서 게임이 더는 덮어쓰지 않는다(gamefiles::sync_game_files).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{FileId, Settings};
use crate::render::gfx::{Assets, Rect, Renderer, CELL_H};
use crate::ui::*;

use super::mail::OFFSCREEN;
use super::text_edit::{insert_char_at, EditState};
use super::widgets::{ease_scroll, scrollbar, Scrollbar};
use super::{App, AppAction, WinInput};

const AUTOSAVE_SECS: f32 = 0.8; // 마지막으로 고친 뒤 이만큼 쉬면 저장한다
pub const MAX_CHARS: usize = 20_000; // 글 하나의 최대 글자 수(저장 파일이 불어나지 않게)
const SELECTION_COLOR: [f32; 4] = [0.62, 0.76, 0.98, 1.0];
const MIN_SCALE: f32 = 0.5; // Ctrl+휠로 줄일 수 있는 최소/최대 글자 배율
const MAX_SCALE: f32 = 3.0;
const SCALE_STEP: f32 = 0.1; // 휠 한 칸당 바뀌는 양

pub struct NotepadApp {
    file: FileId,
    content: String,
    cursor: usize,         // 글자 인덱스(0..=길이)
    anchor: Option<usize>, // 선택의 시작점(커서와 다르면 anchor..cursor 가 선택된 범위)
    dragging: Option<usize>, // 마우스 드래그로 선택하는 중이면 그 시작 글자 인덱스
    unsaved_for: Option<f32>, // 저장 안 된 고침이 있으면 마지막으로 고친 뒤 흐른 시간
    scroll: f32,      // 목표 스크롤(맨 위에서부터 줄 수)
    scroll_disp: f32, // 화면표시용(smooth 켜져 있으면 목표를 부드럽게 따라감)
    sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
    edit: EditState, // 글자 입력/Backspace/IME 조합 — 메일 작성창과 같이 쓴다
}

// 글자 인덱스 범위를 바이트 범위로.
fn byte_range(s: &str, a: usize, b: usize) -> (usize, usize) {
    let at = |n: usize| s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
    (at(a), at(b))
}

impl NotepadApp {
    pub(super) fn new(file: FileId, content: String, settings: Rc<RefCell<Settings>>) -> NotepadApp {
        NotepadApp {
            file,
            cursor: 0,
            anchor: None,
            dragging: None,
            content,
            unsaved_for: None,
            scroll: 0.0,
            scroll_disp: 0.0,
            sb_drag: false,
            settings,
            edit: EditState::default(),
        }
    }

    fn edited(&mut self) {
        self.unsaved_for = Some(0.0);
    }

    // 저장 안 된 내용이 있으면 (파일, 내용)을 돌려주고 "저장됨"으로 바꾼다.
    fn take_unsaved(&mut self) -> Option<(FileId, String)> {
        self.unsaved_for.take().map(|_| (self.file, self.content.clone()))
    }

    fn len(&self) -> usize {
        self.content.chars().count()
    }

    // 선택된 범위(작은 쪽, 큰 쪽) — 선택이 없으면(anchor 가 없거나 커서와 같으면) None.
    fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    fn selected_text(&self) -> Option<String> {
        let (a, b) = self.selection()?;
        let (x, y) = byte_range(&self.content, a, b);
        Some(self.content[x..y].to_string())
    }

    // 선택한 부분을 지운다(커서는 그 자리로). 지웠으면 true.
    fn delete_selection(&mut self) -> bool {
        let Some((a, b)) = self.selection() else { return false };
        let (x, y) = byte_range(&self.content, a, b);
        self.content.replace_range(x..y, "");
        self.cursor = a;
        self.anchor = None;
        self.edited();
        true
    }

    // 글을 커서 자리에 끼워 넣는다 — 선택이 있으면 그 부분을 대신한다.
    fn insert_text(&mut self, text: &str) {
        self.delete_selection();
        for c in text.chars().filter(|c| !c.is_control() || *c == '\n') {
            insert_char_at(&mut self.content, &mut self.cursor, c, MAX_CHARS);
        }
        self.edited();
    }

    // 커서를 옮긴다 — extend(Shift)면 선택을 늘리고, 아니면 선택을 푼다.
    fn move_cursor(&mut self, to: usize, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = to.min(self.len());
    }
}

// Ctrl+휠로 글자 배율을 바꾼다 — wheel 은 휠 한 칸이 ±120(위로 굴리면 +). 5% 단위로 맞추고 MIN~MAX 안으로 자른다.
pub(super) fn scaled_by_wheel(scale: f32, wheel: f32) -> f32 {
    let next = scale + wheel / 120.0 * SCALE_STEP;
    ((next * 20.0).round() / 20.0).clamp(MIN_SCALE, MAX_SCALE)
}

// 줄 안 위치를 지킨 채 위/아래 줄로 옮긴 커서(글자 인덱스). lines 는 (줄 글자, 그 줄이 시작하는 글자 인덱스) 목록.
pub(super) fn vertical_move(lines: &[(String, usize)], cursor: usize, delta: i32) -> usize {
    let Some(cur) = lines.iter().rposition(|&(_, start)| start <= cursor) else { return cursor };
    let col = cursor - lines[cur].1;
    let target = (cur as i32 + delta).clamp(0, lines.len() as i32 - 1) as usize;
    let (text, start) = &lines[target];
    start + col.min(text.chars().count())
}

impl App for NotepadApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    // 창을 닫을 때 아직 저장 안 된 고침이 있으면 마지막으로 저장하게 한다(window_manager 가 닫기 직전에 부른다).
    fn flush_text(&mut self) -> Option<(FileId, String)> {
        self.take_unsaved()
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        const PAD: f32 = 6.0;
        const SB_W: f32 = 8.0; // 스크롤바 폭 예약
        let text_w = (area.w - PAD * 2.0 - SB_W).max(10.0);
        // Ctrl+휠 — 휠로 스크롤하는 대신 글자 크기를 바꾼다(설정에 저장돼서 다음에도 이어진다). Ctrl+0 은 기본 크기.
        let ctrl_held = win.input.is_down(KeyCode::LeftControl) || win.input.is_down(KeyCode::RightControl);
        let mut k = self.settings.borrow().notepad_scale;
        let mouse_over = area.contains(win.mouse.0, win.mouse.1);
        let wheel_zoom = ctrl_held && mouse_over && win.wheel != 0.0;
        if wheel_zoom || (win.focused && ctrl_held && win.input.pressed(KeyCode::Key0)) {
            let next = if wheel_zoom { scaled_by_wheel(k, win.wheel) } else { 1.0 };
            if next != k {
                k = next;
                let mut s = self.settings.borrow_mut();
                s.notepad_scale = k;
                crate::foundation::save_settings(&s);
            }
        }
        let line_h = 20.0 * k;
        let visible = ((area.h / line_h).floor() as usize).max(1);
        let mut cursor_moved = false;
        cursor_moved |= wheel_zoom; // 글자 크기가 바뀌면 커서가 있는 줄이 보이게 스크롤을 맞춘다
        let shift = win.input.is_down(KeyCode::LeftShift) || win.input.is_down(KeyCode::RightShift);
        let ctrl = win.input.is_down(KeyCode::LeftControl) || win.input.is_down(KeyCode::RightControl);

        // ---- 마우스: 클릭으로 커서 옮기기 / Shift+클릭으로 선택 늘리기 / 드래그로 선택 ----
        // 지금 화면에 보이는 그대로의 줄(편집 전) 기준으로 판정한다.
        let pre = wrap_with_offsets(r, &self.content, k, text_w);
        let max_scroll_pre = (pre.len() as f32 - visible as f32).max(0.0);
        let index_at_mouse = |scroll: f32, r: &Renderer| -> usize {
            let row = ((win.mouse.1 - (area.y + 4.0)) / line_h + scroll.clamp(0.0, max_scroll_pre)).floor().max(0.0) as usize;
            match pre.get(row.min(pre.len().saturating_sub(1))) {
                Some((line, start)) => start + char_index_at_x(r, line, k, win.mouse.0 - (area.x + PAD)),
                None => 0,
            }
        };
        if win.focused {
            let in_text = area.contains(win.mouse.0, win.mouse.1) && win.mouse.0 < area.x + area.w - SB_W;
            if win.mouse_clicked && in_text {
                let pos = index_at_mouse(self.scroll_disp, r);
                if shift {
                    self.move_cursor(pos, true);
                } else {
                    self.cursor = pos.min(self.len());
                    self.anchor = None;
                    self.dragging = Some(self.cursor);
                }
                cursor_moved = true;
            } else if let Some(start) = self.dragging {
                if win.mouse_down {
                    let pos = index_at_mouse(self.scroll_disp, r).min(self.len());
                    self.anchor = (pos != start).then_some(start);
                    self.cursor = pos;
                    // 드래그가 위/아래로 벗어나면 그 방향으로 스크롤해서 더 선택할 수 있게 한다.
                    if win.mouse.1 < area.y {
                        self.scroll -= 12.0 * win.dt;
                    } else if win.mouse.1 > area.y + area.h {
                        self.scroll += 12.0 * win.dt;
                    }
                } else {
                    self.dragging = None;
                }
            }
        } else {
            self.dragging = None;
        }

        // ---- 키보드 ----
        if win.focused {
            let before = self.content.clone();
            if ctrl {
                if win.input.pressed(KeyCode::A) {
                    self.anchor = Some(0);
                    self.cursor = self.len();
                }
                if win.input.pressed(KeyCode::C)
                    && let Some(text) = self.selected_text()
                {
                    miniquad::window::clipboard_set(&text);
                }
                if win.input.pressed(KeyCode::X)
                    && let Some(text) = self.selected_text()
                {
                    miniquad::window::clipboard_set(&text);
                    self.delete_selection();
                }
                if win.input.pressed(KeyCode::V)
                    && let Some(text) = miniquad::window::clipboard_get()
                {
                    self.insert_text(&text.replace("\r\n", "\n"));
                }
            } else {
                // 선택이 있는 채로 글자를 치거나 Enter/Backspace/Delete 를 누르면 먼저 선택한 부분을 지운다(그 위에 쓰는 셈).
                let typed_text = win.input.typed.iter().any(|c| !c.is_control());
                let deleting = win.input.pressed(KeyCode::Backspace) || win.input.pressed(KeyCode::Delete);
                let mut suppress_backspace = false;
                if self.selection().is_some() && (typed_text || deleting || win.input.pressed(KeyCode::Enter)) {
                    self.delete_selection();
                    suppress_backspace = deleting; // 선택을 지운 것으로 이번 삭제는 끝 — 한 글자 더 지우지 않는다
                }
                self.edit.edit(&mut self.content, &mut self.cursor, MAX_CHARS, win, suppress_backspace);
                let composing = crate::platform::ime::composition_preview().is_some_and(|p| !p.is_empty());
                if !composing {
                    if win.input.pressed(KeyCode::Delete) && !suppress_backspace && self.cursor < self.len() {
                        let (x, y) = byte_range(&self.content, self.cursor, self.cursor + 1);
                        self.content.replace_range(x..y, "");
                    }
                    if win.input.pressed(KeyCode::Enter) {
                        insert_char_at(&mut self.content, &mut self.cursor, '\n', MAX_CHARS);
                    }
                    // 좌우 이동 — 선택이 있고 Shift 가 아니면 선택의 가장자리로 접는다.
                    if win.input.pressed(KeyCode::Left) {
                        match (self.selection(), shift) {
                            (Some((a, _)), false) => self.move_cursor(a, false),
                            _ => self.move_cursor(self.cursor.saturating_sub(1), shift),
                        }
                    }
                    if win.input.pressed(KeyCode::Right) {
                        match (self.selection(), shift) {
                            (Some((_, b)), false) => self.move_cursor(b, false),
                            _ => self.move_cursor(self.cursor + 1, shift),
                        }
                    }
                }
            }
            if self.content != before {
                self.edited();
            }
            cursor_moved |= !win.input.typed.is_empty()
                || [KeyCode::Backspace, KeyCode::Delete, KeyCode::Enter, KeyCode::Left, KeyCode::Right, KeyCode::Up, KeyCode::Down, KeyCode::Home, KeyCode::End]
                    .iter()
                    .any(|&k| win.input.pressed(k) || k == KeyCode::Backspace && win.input.is_down(k))
                || (ctrl && (win.input.pressed(KeyCode::V) || win.input.pressed(KeyCode::X) || win.input.pressed(KeyCode::A)));
        }
        let len = self.len();
        self.cursor = self.cursor.min(len);
        self.anchor = self.anchor.map(|a| a.min(len));

        // ---- 보여줄 글(조합 중인 글자를 캐럿 자리에 끼워서) 접기 ----
        let preview = if win.focused { crate::platform::ime::composition_preview() } else { None };
        let composing = preview.as_ref().is_some_and(|p| !p.is_empty());
        let (display, effective_cursor) = match &preview {
            Some(p) if !p.is_empty() => {
                let (byte, _) = byte_range(&self.content, self.cursor, self.cursor);
                (format!("{}{}{}", &self.content[..byte], p, &self.content[byte..]), self.cursor + p.chars().count())
            }
            _ => (self.content.clone(), self.cursor),
        };
        let lines = wrap_with_offsets(r, &display, k, text_w);

        // 위/아래/Home/End 는 접힌 줄 기준으로 커서를 옮긴다(Shift 와 같이 누르면 선택을 늘린다).
        if win.focused && !composing {
            if win.input.pressed(KeyCode::Up) {
                self.move_cursor(vertical_move(&lines, self.cursor, -1), shift);
            }
            if win.input.pressed(KeyCode::Down) {
                self.move_cursor(vertical_move(&lines, self.cursor, 1), shift);
            }
            if let Some(i) = lines.iter().rposition(|&(_, start)| start <= self.cursor) {
                if win.input.pressed(KeyCode::Home) {
                    self.move_cursor(lines[i].1, shift);
                }
                if win.input.pressed(KeyCode::End) {
                    self.move_cursor(lines[i].1 + lines[i].0.chars().count(), shift);
                }
            }
        }
        let cursor_line = lines.iter().rposition(|&(_, start)| start <= effective_cursor).unwrap_or(0);

        // ---- 스크롤: 휠, 그리고 커서가 움직였으면 커서 줄이 보이게 ----
        let max_scroll = (lines.len() as f32 - visible as f32).max(0.0);
        if cursor_moved && self.dragging.is_none() {
            if (cursor_line as f32) < self.scroll {
                self.scroll = cursor_line as f32;
            } else if (cursor_line as f32) > self.scroll + (visible as f32 - 1.0) {
                self.scroll = (cursor_line as f32 - (visible as f32 - 1.0)).max(0.0);
            }
        }
        if mouse_over && !ctrl_held {
            self.scroll -= win.wheel / 120.0 * 3.0;
        }
        self.scroll = self.scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.scroll_disp, self.scroll, win.dt, smooth);

        // ---- 그리기(선택 영역 → 글자 → 캐럿) ----
        let line_off = self.scroll_disp * line_h;
        let first = self.scroll_disp as usize;
        let selection = if composing { None } else { self.selection() };
        r.set_clip(Some(area));
        let mut ty = area.y + 4.0 - (line_off - first as f32 * line_h);
        for (i, (line, start)) in lines.iter().enumerate().skip(first) {
            if ty > area.y + area.h {
                break;
            }
            if let Some((sa, sb)) = selection {
                let line_chars = line.chars().count();
                let (la, lb) = (*start, start + line_chars);
                let (from, to) = (sa.max(la), sb.min(lb));
                // 줄바꿈 글자까지 선택에 들어가면 줄 끝에 작은 칸을 더 칠해서 "줄이 이어서 선택됐다"는 걸 보여 준다.
                let through_newline = sb > lb && sa <= lb;
                if from < to || through_newline {
                    let (bx, _) = byte_range(line, from.saturating_sub(la).min(line_chars), 0);
                    let (by, _) = byte_range(line, to.saturating_sub(la).min(line_chars), 0);
                    let x0 = area.x + PAD + r.text_width(&line[..bx], k);
                    let x1 = area.x + PAD + r.text_width(&line[..by], k) + if through_newline { 6.0 } else { 0.0 };
                    r.rect(x0, ty, (x1 - x0).max(0.0), line_h - 2.0, SELECTION_COLOR);
                }
            }
            r.text(area.x + PAD, ty, line, k, BLACK);
            if win.focused && i == cursor_line {
                let col = effective_cursor.saturating_sub(*start).min(line.chars().count());
                let (byte, _) = byte_range(line, col, 0);
                let cx = area.x + PAD + r.text_width(&line[..byte], k);
                // IME 의 자체 조합창/언어 표시줄은 화면 밖으로 치운다(메일 작성창과 같은 이유).
                crate::platform::ime::set_composition_pos(OFFSCREEN, OFFSCREEN);
                crate::platform::ime::hide_cicero_windows();
                if (win.time % 1.0) < 0.5 {
                    r.rect(cx + 1.0, ty + 1.0, 2.0, (CELL_H * k - 4.0).max(4.0), BLACK);
                }
            }
            ty += line_h;
        }
        r.set_clip(None);

        // 스크롤바 (내용이 넘칠 때) — 마우스로 드래그해서 스크롤 가능.
        if max_scroll > 0.0 {
            let sb_x = area.x + area.w - SB_W;
            let frac = visible as f32 / lines.len() as f32;
            scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, area.y, SB_W, area.h), visible_frac: frac, disp: self.scroll_disp, max_scroll }, &mut self.scroll, &mut self.sb_drag);
        }

        // ---- 자동 저장: 잠깐 쉬었거나, 창이 포커스를 잃었으면 ----
        if let Some(t) = self.unsaved_for.as_mut() {
            *t += win.dt;
        }
        if self.unsaved_for.is_some_and(|t| t >= AUTOSAVE_SECS) || (!win.focused && self.unsaved_for.is_some()) {
            if let Some((file, text)) = self.take_unsaved() {
                return AppAction::SaveText(file, text);
            }
        }
        AppAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(text: &str) -> NotepadApp {
        NotepadApp::new(7, text.to_string(), Rc::new(RefCell::new(Settings::default())))
    }

    fn lines(items: &[(&str, usize)]) -> Vec<(String, usize)> {
        items.iter().map(|(t, s)| (t.to_string(), *s)).collect()
    }

    #[test]
    fn vertical_moves_keep_the_column_and_clamp_at_the_ends() {
        // "hello" / "hi" / "world!!" — 줄이 시작하는 글자 인덱스 0, 6, 9.
        let l = lines(&[("hello", 0), ("hi", 6), ("world!!", 9)]);
        assert_eq!(vertical_move(&l, 3, 1), 6 + 2, "아래 줄이 짧으면 줄 끝으로");
        assert_eq!(vertical_move(&l, 8, 1), 9 + 2, "그 끝의 열을 이어서(2열)");
        assert_eq!(vertical_move(&l, 11, -1), 6 + 2, "위 줄이 짧으면 줄 끝으로");
        assert_eq!(vertical_move(&l, 3, -1), 3, "맨 위 줄에서는 그대로");
        assert_eq!(vertical_move(&l, 12, 1), 12, "맨 아래 줄에서는 그대로");
    }

    #[test]
    fn unsaved_edits_are_flushed_once() {
        let mut app = app("abc");
        assert_eq!(app.flush_text(), None, "고친 게 없으면 저장할 것도 없다");
        app.content.push('d');
        app.edited();
        assert_eq!(app.flush_text(), Some((7, "abcd".to_string())));
        assert_eq!(app.flush_text(), None, "한 번 저장하면 다시 안 낸다");
    }

    #[test]
    fn shift_moves_grow_a_selection_and_plain_moves_clear_it() {
        let mut a = app("hello world");
        a.move_cursor(2, false);
        a.move_cursor(5, true);
        assert_eq!(a.selection(), Some((2, 5)));
        assert_eq!(a.selected_text().as_deref(), Some("llo"));
        a.move_cursor(1, true); // 앵커(2)를 기준으로 반대쪽으로도 늘어난다
        assert_eq!(a.selection(), Some((1, 2)));
        a.move_cursor(3, false);
        assert_eq!(a.selection(), None);
        assert_eq!(a.cursor, 3);
    }

    #[test]
    fn deleting_or_typing_over_a_selection_replaces_it() {
        let mut a = app("hello world");
        a.anchor = Some(0);
        a.cursor = 6;
        assert!(a.delete_selection());
        assert_eq!((a.content.as_str(), a.cursor, a.selection()), ("world", 0, None));
        assert!(a.unsaved_for.is_some(), "지우면 저장 대기 상태가 된다");
        // 붙여넣기는 선택한 부분을 대신한다(줄바꿈은 살리고 다른 제어 문자는 버린다).
        a.anchor = Some(1);
        a.cursor = 4;
        a.insert_text("A\r\nB\u{7}");
        assert_eq!(a.content, "wA\nBd");
        assert_eq!(a.cursor, 4);
    }

    #[test]
    fn selections_count_characters_not_bytes() {
        let mut a = app("한글 메모장");
        a.anchor = Some(1);
        a.cursor = 4;
        assert_eq!(a.selected_text().as_deref(), Some("글 메"));
        a.delete_selection();
        assert_eq!(a.content, "한모장");
        // 전체 선택 범위
        a.anchor = Some(0);
        a.cursor = a.len();
        assert_eq!(a.selection(), Some((0, 3)));
        assert_eq!(a.selected_text().as_deref(), Some("한모장"));
    }

    #[test]
    fn an_empty_range_is_not_a_selection_and_pasting_respects_the_limit() {
        let mut a = app("abc");
        a.anchor = Some(2);
        a.cursor = 2;
        assert_eq!(a.selection(), None);
        assert!(!a.delete_selection());
        let mut full = app(&"x".repeat(MAX_CHARS));
        full.cursor = 0;
        full.insert_text("yyy");
        assert_eq!(full.len(), MAX_CHARS, "한도를 넘게 붙여 넣어도 잘린다");
    }

    #[test]
    fn the_wheel_changes_the_scale_in_steps_and_stays_in_range() {
        assert_eq!(scaled_by_wheel(1.0, 120.0), 1.1, "위로 한 칸 — 커진다");
        assert_eq!(scaled_by_wheel(1.0, -120.0), 0.9, "아래로 한 칸 — 작아진다");
        assert_eq!(scaled_by_wheel(1.0, 0.0), 1.0);
        let mut k = 1.0;
        for _ in 0..100 {
            k = scaled_by_wheel(k, 120.0);
        }
        assert_eq!(k, MAX_SCALE, "최대를 넘지 않는다");
        for _ in 0..100 {
            k = scaled_by_wheel(k, -120.0);
        }
        assert_eq!(k, MIN_SCALE, "최소 밑으로 내려가지 않는다");
        // 노치가 크게 들어와도(한 번에 여러 칸) 범위 안이다.
        assert_eq!(scaled_by_wheel(1.0, 1200.0), 2.0);
    }

    #[test]
    fn old_settings_files_get_the_default_scale() {
        let s: Settings = serde_json::from_str("{}").map(|s: Settings| s).unwrap_or_else(|_| Settings::default());
        assert_eq!(s.notepad_scale, 1.0);
        assert_eq!(Settings::default().notepad_scale, 1.0);
    }
}

