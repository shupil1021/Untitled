//! "Write Mail" 탭 — 받는 사람/제목/본문 입력 상태, 글자 편집(커서 위치 삽입/삭제, Backspace 꾹 누르기 반복), IME 조합 처리,
//! 첨부 고르기, 보내기 버튼.

use super::*;

// "Write Mail" 탭에서 직접 입력하는 세 필드 중 지금 타이핑이 어디로 들어가는지.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ComposeField {
    To,
    Subject,
    Body,
}

// "Write Mail" 탭 — 받는 사람/제목/본문을 전부 직접 입력하는 새 메일 작성 상태.
// MailApp 이 살아있는 동안(창을 닫기 전까지)만 유지되는 초안이라 fs 에 저장하지는
// 않는다 — 실제 메일 클라이언트도 임시보관함에 따로 옮기지 않는 한 창을 닫으면
// 초안이 사라지는 것과 같다. "보냈다"는 사실 자체(mail_sent_count)만 fs 에 남긴다.
pub(super) struct NewMailState {
    pub(super) to: String,
    pub(super) subject: String,
    pub(super) body: String,
    pub(super) active: ComposeField,
    // 지금 활성 필드(active) 문자열 안에서 커서 위치(char 인덱스, 0..=len) —
    // 필드를 클릭하면 그 자리로, 타이핑/백스페이스/엔터를 치면 그 위치 기준으로
    // 삽입/삭제된다(끝에만 붙던 예전 방식과 달리 문장 중간을 클릭해서 그 자리부터
    // 고칠 수 있다).
    pub(super) cursor: usize,
    // 첨부 가능한 파일 중 고른 것들 — 여러 개를 붙일 수 있다(붙인 순서 그대로).
    // 보낼 때 특별한 효과는 없지만(게임 진행에 영향 없음) 첨부 UI 자체를 Outlook
    // Express 스타일로 갖추기 위한 것.
    pub(super) attachments: Vec<(FileId, String, IconType)>,
    pub(super) picker_open: bool, // "Attach..." 를 눌러서 첨부할 파일을 고르는 목록이 펼쳐져 있는지
}

impl NewMailState {
    pub(super) fn new() -> NewMailState {
        NewMailState { to: String::new(), subject: String::new(), body: String::new(), active: ComposeField::To, cursor: 0, attachments: Vec::new(), picker_open: false }
    }
}

// "Write Mail" 필드 텍스트 편집 — cursor(글자 인덱스)가 가리키는 자리에 끼워
// 넣거나 그 앞 글자를 지운다. 문자열은 바이트 인덱스라 char_indices 로 몇 번째
// 글자가 몇 번째 바이트에서 시작하는지 먼저 찾는다(한/일 문자는 여러 바이트라
// 그냥 cursor 를 바이트 인덱스로 쓰면 글자 중간을 끊어버릴 수 있다).
pub(super) fn insert_char_at(s: &mut String, cursor: &mut usize, c: char, max_chars: usize) {
    if s.chars().count() >= max_chars {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor).map(|(b, _)| b).unwrap_or(s.len());
    s.insert(byte_idx, c);
    *cursor += 1;
}

pub(super) fn backspace_at(s: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor - 1).map(|(b, _)| b).unwrap_or(0);
    s.remove(byte_idx);
    *cursor -= 1;
}

// Backspace 를 꾹 누르고 있을 때 지금까지 몇 번 지워졌어야 하는지 — 누른 순간
// 한 번(즉시), REPEAT_DELAY 초까지 그대로 있으면 그 뒤로 REPEAT_INTERVAL 마다
// 하나씩 더. hold 는 지금 이 키가 계속 눌려있던 시간(초).
pub(super) const BACKSPACE_REPEAT_DELAY: f32 = 0.3;
pub(super) const BACKSPACE_REPEAT_INTERVAL: f32 = 0.035;
// 한 프레임에 hold 누적치를 이보다 많이는 안 늘린다 — main.rs 의 전역 dt
// 상한(500ms)만으론 이 반복 판정 기준으로 여전히 너무 커서, 큰 프레임 끊김이
// 한 번 있으면 그 한 프레임에 반복 삭제가 몰아서 발생할 수 있었다.
pub(super) const BACKSPACE_MAX_DT_PER_FRAME: f32 = 0.02;
pub(super) fn backspace_repeats_due(hold: f32) -> u32 {
    if hold <= 0.0 {
        return 0;
    }
    if hold < BACKSPACE_REPEAT_DELAY {
        return 1;
    }
    2 + ((hold - BACKSPACE_REPEAT_DELAY) / BACKSPACE_REPEAT_INTERVAL) as u32
}

impl MailApp {
    // draw_field() 의 입력 가능한 버전 — 클릭하면 그 필드가 활성화되고(테두리가
    // 남색으로 바뀌고 깜빡이는 커서가 붙는다). 실제 타이핑 처리는 여기서 안 하고
    // (한 프레임에 여러 필드가 동시에 안 그려지므로, 타이핑은 호출부가 active_field
    // 기준으로 한 번에 처리하는 게 더 간단하다) 그리기 + 클릭 감지만 담당한다.
    // 커서는 항상 값의 맨 끝이 아니라 cursor(글자 인덱스)가 가리키는 자리에
    // 그린다 — 클릭했으면(Some(idx) 반환) 그 클릭 위치에 가장 가까운 글자
    // 경계를 돌려줘서, 호출부가 active_field 와 커서 위치를 같이 옮길 수 있다.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_editable_field(
        &self, r: &mut Renderer, x: f32, y: f32, w: f32, label: &str, value: &str, active: bool, cursor: usize, win: &WinInput,
    ) -> Option<usize> {
        const LABEL_W: f32 = 76.0;
        const FIELD_H: f32 = 22.0;
        r.text(x, y + (FIELD_H - CELL_H * 0.8) / 2.0, label, 0.8, BLACK);
        let field = Rect::new(x + LABEL_W, y, w - LABEL_W, FIELD_H);
        r.rect(field.x, field.y, field.w, field.h, WHITE);
        border(r, field.x, field.y, field.w, field.h, if active { NAVY } else { [0.6, 0.6, 0.62, 1.0] });
        let ty = field.y + (field.h - CELL_H * 0.8) / 2.0;
        let max_w = field.w - 12.0;
        let byte_idx = value.char_indices().nth(cursor).map(|(b, _)| b).unwrap_or(value.len());
        // 조합 중인(아직 확정 안 된) 문자열을 캐럿 위치에 끼워넣어서만 화면에
        // 보여준다 — self.new_mail.* 자체(저장/전송에 실제 쓰이는 값)는 여전히
        // char_event 로 확정된 문자만으로 채워지고 이 값의 영향을 전혀 안 받는다.
        let preview = if active && win.focused { crate::platform::ime::composition_preview() } else { None };
        let display = match &preview {
            Some(p) if !p.is_empty() => format!("{}{}{}", &value[..byte_idx], p, &value[byte_idx..]),
            _ => value.to_string(),
        };
        r.text_clipped(field.x + 6.0, ty, &display, 0.8, BLACK, max_w);
        if active {
            let tw = r.text_width(&value[..byte_idx], 0.8).min(max_w);
            let cursor_vx = field.x + 6.0 + tw;
            if win.focused {
                // Windows 자체의 IME 조합창/후보창은 화면 어디에 띄우든 우리가 그린
                // 픽셀아트 CRT 화면이랑 스타일이 안 맞아서(둥근 모서리/그림자 있는
                // 현대식 팝업) 캐럿 옆으로 옮겨놔도 어색한 상자로 튀어 보였다 —
                // 그래서 아예 화면 밖으로 치워서 안 보이게 한다. 조합/확정 자체는
                // 이 팝업 위치와 무관하게 그대로 잘 되고, 조합 중인 내용은 위에서
                // 이미 우리가 직접 그려서 보여준다.
                crate::platform::ime::set_composition_pos(OFFSCREEN, OFFSCREEN);
                // 그래도 남는 네 번째 팝업(최신 IME 의 CiceroUIWndFrame 언어
                // 표시줄)은 저 API 들로 못 옮기므로 통째로 숨긴다.
                crate::platform::ime::hide_cicero_windows();
            }
            if (win.time % 1.0) < 0.5 {
                r.rect(cursor_vx + 1.0, ty, 2.0, CELL_H * 0.8, BLACK);
            }
        }
        if win.mouse_clicked && field.contains(win.mouse.0, win.mouse.1) {
            Some(char_index_at_x(r, value, 0.8, win.mouse.0 - (field.x + 6.0)))
        } else {
            None
        }
    }

    // "Write Mail" 탭 — 받는 사람/제목도 직접 입력해야 한다. 세 필드(To/Subject/Body)
    // 중 지금 타이핑이 어디로 들어가는지는 self.new_mail.active 로 추적하고, 필드를
    // 클릭하면 그 필드로 옮겨간다. 위쪽에 진짜 Outlook Express "New Message" 창처럼
    // Send/Attach 아이콘 버튼이 늘어선 툴바를 두었다(참고 자료를 못 찾아 정확한
    // 버튼 순서/아이콘까지는 재현 못 했지만, "툴바가 필드 위에 있고 첨부는 별도
    // 버튼으로 고른다" 는 구조 자체는 그 시절 메일 클라이언트의 공통된 관례다).
    pub(super) fn draw_new_compose(&mut self, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        // 필드 배치는 실제 글자 길이와 무관하게 항상 같은 자리라(라벨/줄 높이가
        // 고정값), 그리기 전에 미리 다 계산해둘 수 있다 — 그래야 본문 클릭 판정을
        // (지금까지 그려진 걸 기준으로 뒤늦게 하는 대신) 입력 처리보다 먼저 해서,
        // "본문을 클릭해 커서를 옮기고 그 프레임에 바로 타이핑" 도 자연스럽게 된다.
        const FIELD_ROW_H: f32 = 20.0;
        const ATTACH_BOX_H: f32 = 26.0;
        const ATTACH_GAP: f32 = 2.0; // 첨부 박스끼리(그리고 본문 마지막 줄과) 거의 붙어 보이게
        const BTN_ROW_H: f32 = 30.0;
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        let cx = area.x + 8.0;
        let field_w = area.w - 16.0;
        let to_y = area.y + 6.0;
        let subject_y = to_y + FIELD_ROW_H;
        let fy = subject_y + FIELD_ROW_H + 4.0;
        // 첨부는 예전처럼 필드와 본문 사이에 따로 자리를 빼지 않는다 — Inbox/Sent
        // 읽기 화면과 같은 요령으로 본문 스크롤 영역 안, 마지막 줄 다음에 이어
        // 붙여서 같이 스크롤된다(여러 개를 붙일 수 있으니 한 줄로 고정해두면
        // 금방 좁아진다).
        let body_area = Rect::new(area.x, fy, area.w, (area.y + area.h - BTN_ROW_H - fy).max(20.0));
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 4.0, body_area.w - 14.0 - SB_W, body_area.h - 8.0);

        // 본문 클릭으로 커서를 옮기는 건 이 프레임에서 아직 편집 전(지금 화면에
        // 보이는 그대로)의 텍스트 기준으로 판정해야 한다 — 방금 그려졌던 자리를
        // 그대로 누른 것이니까. 편집이 끝난 뒤에는 줄이 다시 바뀔 수 있어서
        // 렌더링용으로 한 번 더 계산한다(아래).
        if win.mouse_clicked && win.focused && !self.new_mail.picker_open && body_area.contains(win.mouse.0, win.mouse.1) {
            let pre_lines = wrap_with_offsets(r, &self.new_mail.body, 1.0, text_area.w.max(10.0));
            let visible = ((text_area.h / LINE_H).floor() as usize).max(1);
            let max_scroll = (pre_lines.len() as f32 - visible as f32).max(0.0);
            // body_scroll(목표값) 대신 body_scroll_disp(부드럽게 따라가는 화면표시용
            // 값)을 써야 한다 — 스크롤 애니메이션이 아직 다 안 끝난 순간에 클릭하면
            // 실제로 화면에 보이는 자리와 안 맞을 수 있어서.
            let scroll = self.body_scroll_disp.clamp(0.0, max_scroll);
            let row = ((win.mouse.1 - text_area.y) / LINE_H + scroll).floor().max(0.0) as usize;
            let row = row.min(pre_lines.len().saturating_sub(1));
            if let Some((line, start)) = pre_lines.get(row) {
                let col = char_index_at_x(r, line, 1.0, win.mouse.0 - text_area.x);
                self.new_mail.active = ComposeField::Body;
                self.new_mail.cursor = start + col;
            }
        }

        // 첨부 목록이 펼쳐진 동안은 필드 자체가 안 보이니 입력을 건너뛴다.
        // 본문이 이 프레임에 편집(타이핑/백스페이스/엔터/화살표)됐으면 캐럿이
        // 보이도록 스크롤도 같이 맞춰야 하니 표시해둔다.
        let mut body_cursor_moved = false;
        if win.mouse_clicked && win.focused && !self.new_mail.picker_open && body_area.contains(win.mouse.0, win.mouse.1) {
            body_cursor_moved = true;
        }
        if win.focused && !self.new_mail.picker_open {
            let NewMailState { to, subject, body, cursor, active, .. } = &mut self.new_mail;
            let max_len_of = |f: ComposeField| if f == ComposeField::Body { COMPOSE_MAX_CHARS } else { 200 };
            for &c in &win.input.typed {
                if c.is_control() {
                    continue;
                }
                let field: &mut String = match *active {
                    ComposeField::To => &mut *to,
                    ComposeField::Subject => &mut *subject,
                    ComposeField::Body => &mut *body,
                };
                insert_char_at(field, cursor, c, max_len_of(*active));
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            // 결합 전 자모 여러 개(예: "ㄲ" 이전의 "ㄱㄱ")를 조합 중일 때 Backspace
            // 한 번 = Windows/한글 IME 가 자모 전체를 한 번에 지워버리는 문제 — IME
            // 내부 동작을 계산/가로채서 맞추려던 시도들은 전부 실패했다(경위는
            // README). 대신 이 조합 세션 동안 IME 가 실제로 보여준 화면을 우리가
            // 직접 기록해뒀다가(composition_history) "한 단계 전 기록"을 그대로
            // 꺼내 쓴다 — 계산이 아니라 조회라 틀릴 여지가 없다. 조합 버퍼에 다시
            // 넣는 건 안 먹히므로, 조합을 취소하고(`ime::cancel_composition`) 그
            // 값을 평범한 확정 문자로 직접 밀어넣는다. 짧게 한 번 누른 경우만
            // 처리한다(`pressed()` — 꾹 누르는 건 원래도 문제없었다).
            let cur_composition = crate::platform::ime::composition_preview().unwrap_or_default();
            if win.input.pressed(KeyCode::Backspace) && !self.composition_history.is_empty() {
                let target =
                    if self.composition_history.len() >= 2 { self.composition_history[self.composition_history.len() - 2].clone() } else { String::new() };
                crate::platform::ime::cancel_composition();
                if !target.is_empty() {
                    let field: &mut String = match *active {
                        ComposeField::To => &mut *to,
                        ComposeField::Subject => &mut *subject,
                        ComposeField::Body => &mut *body,
                    };
                    for ch in target.chars() {
                        insert_char_at(field, cursor, ch, max_len_of(*active));
                    }
                    if *active == ComposeField::Body {
                        body_cursor_moved = true;
                    }
                }
                // 취소했으니 이 조합 세션은 끝났다 — 기록도 같이 비운다.
                self.composition_history.clear();
            } else if cur_composition.is_empty() {
                self.composition_history.clear();
            } else if self.composition_history.last() != Some(&cur_composition) {
                self.composition_history.push(cur_composition.clone());
            }
            let composing = crate::platform::ime::composition_preview().is_some_and(|p| !p.is_empty());
            let key_down = win.input.is_down(KeyCode::Backspace);
            if !key_down {
                self.backspace_hold = 0.0;
                self.backspace_fired = 0;
                self.backspace_owned_by_ime = false;
            } else if composing {
                self.backspace_owned_by_ime = true;
                self.backspace_hold = 0.0;
                self.backspace_fired = 0;
            } else if self.backspace_owned_by_ime {
                // 조합 중에 시작된 같은 누름이 이어지는 중 — 아직 키를 안 뗐으니
                // 계속 IME 몫으로 치고 우리는 손 안 댄다.
            } else {
                // win.dt 자체엔 500ms 상한(main.rs) 이 있지만, 그건 여기 기준으론
                // 여전히 너무 크다 — "누적된 hold 시간 → 반복 횟수" 로 바로
                // 환산하는 로직이라, 큰 dt 하나가 그대로 "그동안 계속 눌려있던
                // 것"으로 오인되면 한 프레임에 반복 삭제가 몰아서 발생한다(짧게
                // 한 번 눌렀는데 여러 글자가 지워지던 버그). 그래서 여기 한
                // 프레임에 반영하는 dt 는 반복 간격(BACKSPACE_REPEAT_INTERVAL)
                // 보다도 작게 별도로 한 번 더 눌러서, 큰 프레임 끊김이 있어도
                // 절대 한 프레임에 여러 번 반복 판정이 몰리지 않게 한다.
                self.backspace_hold += win.dt.min(BACKSPACE_MAX_DT_PER_FRAME);
            }
            let due = backspace_repeats_due(self.backspace_hold);
            let fire_count = due.saturating_sub(self.backspace_fired).min(6);
            if fire_count > 0 {
                self.backspace_fired = due;
                let field: &mut String = match *active {
                    ComposeField::To => &mut *to,
                    ComposeField::Subject => &mut *subject,
                    ComposeField::Body => &mut *body,
                };
                for _ in 0..fire_count {
                    backspace_at(field, cursor);
                }
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Left) {
                *cursor = cursor.saturating_sub(1);
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Right) {
                let len = match *active {
                    ComposeField::To => to.chars().count(),
                    ComposeField::Subject => subject.chars().count(),
                    ComposeField::Body => body.chars().count(),
                };
                *cursor = (*cursor + 1).min(len);
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Enter) {
                match *active {
                    ComposeField::To => {
                        *active = ComposeField::Subject;
                        *cursor = subject.chars().count();
                    }
                    ComposeField::Subject => {
                        *active = ComposeField::Body;
                        *cursor = body.chars().count();
                        body_cursor_moved = true;
                    }
                    ComposeField::Body => {
                        insert_char_at(body, cursor, '\n', COMPOSE_MAX_CHARS);
                        body_cursor_moved = true;
                    }
                }
            }
        }

        let to = self.new_mail.to.clone();
        if let Some(idx) =
            self.draw_editable_field(r, cx, to_y, field_w, t(lang, s::FIELD_TO), &to, self.new_mail.active == ComposeField::To, self.new_mail.cursor, win)
        {
            self.new_mail.active = ComposeField::To;
            self.new_mail.cursor = idx;
        }
        let subject = self.new_mail.subject.clone();
        if let Some(idx) = self.draw_editable_field(
            r, cx, subject_y, field_w, t(lang, s::FIELD_SUBJECT), &subject, self.new_mail.active == ComposeField::Subject,
            self.new_mail.cursor, win,
        ) {
            self.new_mail.active = ComposeField::Subject;
            self.new_mail.cursor = idx;
        }

        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(
            r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h,
            if self.new_mail.active == ComposeField::Body && !self.new_mail.picker_open { NAVY } else { [0.6, 0.6, 0.62, 1.0] },
        );

        // 첨부 목록이 펼쳐져 있으면 본문 대신 그 목록을 이 자리에 보여준다 —
        // 진짜 Outlook Express 도 첨부 파일 선택 창이 뜨면 그동안 본문 입력이
        // 잠깐 가려지는 것과 비슷한 느낌. 아래쪽 Send/Attach 버튼 줄은 두 경우
        // 모두에서 그려야 하므로(첨부 목록을 보는 중에도 Attach 를 다시 눌러 닫을
        // 수 있어야 한다) 여기서 조기 반환하지 않고 아래로 이어서 그린다. 여러
        // 개를 붙일 수 있으니 하나 고른다고 바로 닫지 않는다 — 이미 붙인 파일은
        // 체크 표시로 보여주고, 다시 누르면 그 자리에서 뗄 수 있다(토글).
        if self.new_mail.picker_open {
            let list_area = Rect::new(body_area.x + 6.0, body_area.y + 4.0, body_area.w - 12.0, body_area.h - 8.0);
            if self.attachable.is_empty() {
                label(
                    r, list_area.x + 4.0, list_area.y + 4.0,
                    t(lang, s::NO_FILES_TO_ATTACH),
                    GRAY,
                );
            } else {
                const ROW_H: f32 = 22.0;
                r.set_clip(Some(list_area));
                for (i, (id, name, icon)) in self.attachable.iter().enumerate() {
                    let ry = list_area.y + i as f32 * ROW_H;
                    if ry + ROW_H > list_area.y + list_area.h {
                        break;
                    }
                    let row = Rect::new(list_area.x, ry, list_area.w, ROW_H);
                    let attached = self.new_mail.attachments.iter().any(|(aid, ..)| aid == id);
                    let hover = row.contains(win.mouse.0, win.mouse.1);
                    if attached {
                        r.rect(row.x, row.y, row.w, row.h, [0.85, 0.93, 0.85, 1.0]);
                    } else if hover {
                        r.rect(row.x, row.y, row.w, row.h, [0.82, 0.88, 0.98, 1.0]);
                    }
                    draw_icon(r, assets, icon, row.x + 2.0, row.y + 2.0, 18.0);
                    let name_color = if attached { [0.15, 0.45, 0.15, 1.0] } else { BLACK };
                    r.text_clipped(row.x + 24.0, row.y + 3.0, &display_name(lang, name), 0.85, name_color, row.w - 28.0);
                    if win.mouse_clicked && hover {
                        if attached {
                            self.new_mail.attachments.retain(|(aid, ..)| aid != id);
                        } else {
                            self.new_mail.attachments.push((*id, name.clone(), *icon));
                        }
                    }
                }
                r.set_clip(None);
            }
            return self.draw_new_compose_buttons(r, area, win, lang);
        }

        // 조합 중인(아직 확정 안 된) 문자열을 캐럿 위치에 끼워넣어서만 화면에
        // 보여준다 — self.new_mail.body 자체는 여전히 char_event 로 확정된
        // 문자만으로 채워지고 이 값의 영향을 전혀 안 받는다. 화면상 캐럿도
        // 이 미리보기 뒤(effective_cursor)로 같이 옮겨서, 지금 막 조합 중인
        // 글자 바로 뒤에 캐럿이 있는 것처럼 보이게 한다.
        let body_preview =
            if self.new_mail.active == ComposeField::Body && win.focused { crate::platform::ime::composition_preview() } else { None };
        let (display_body, effective_cursor) = match &body_preview {
            Some(p) if !p.is_empty() => {
                let byte_idx =
                    self.new_mail.body.char_indices().nth(self.new_mail.cursor).map(|(b, _)| b).unwrap_or(self.new_mail.body.len());
                let mut s = String::with_capacity(self.new_mail.body.len() + p.len());
                s.push_str(&self.new_mail.body[..byte_idx]);
                s.push_str(p);
                s.push_str(&self.new_mail.body[byte_idx..]);
                (s, self.new_mail.cursor + p.chars().count())
            }
            _ => (self.new_mail.body.clone(), self.new_mail.cursor),
        };
        // 렌더링용으로 (편집이 끝난) 최종 본문을 다시 접는다 — 위에서 클릭 판정에
        // 썼던 pre_lines 는 편집 전 스냅샷이라 여기서 재사용하면 안 된다.
        let lines = wrap_with_offsets(r, &display_body, 1.0, text_area.w.max(10.0));
        let visible = ((text_area.h / LINE_H).floor() as usize).max(1);
        let attach_unit_lines = (ATTACH_BOX_H + ATTACH_GAP) / LINE_H;
        let total_lines = lines.len() as f32 + attach_unit_lines * self.new_mail.attachments.len() as f32;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);

        // 캐럿이 있는 줄을 찾는다 — effective_cursor 이하로 시작하는 줄들 중
        // 가장 마지막 것.
        let cursor_line = lines.iter().rposition(|&(_, start)| start <= effective_cursor).unwrap_or(0);

        // 이번 프레임에 커서가 움직였으면(클릭/타이핑/백스페이스/화살표/엔터)
        // 그 줄이 화면 안에 들어오도록 스크롤을 당긴다 — 그 외엔 휠로 스크롤한
        // 값을 그대로 둔다(편집 중이 아닌데 계속 맨 아래로 끌려 내려가던 문제).
        if body_cursor_moved {
            if (cursor_line as f32) < self.body_scroll {
                self.body_scroll = cursor_line as f32;
            } else if (cursor_line as f32) > self.body_scroll + (visible as f32 - 1.0) {
                self.body_scroll = (cursor_line as f32 - (visible as f32 - 1.0)).max(0.0);
            }
        }
        if body_area.contains(win.mouse.0, win.mouse.1) {
            self.body_scroll -= win.wheel / 120.0 * 3.0;
        }
        self.body_scroll = self.body_scroll.clamp(0.0, max_body_scroll);
        ease_scroll(&mut self.body_scroll_disp, self.body_scroll, win.dt, smooth);

        let line_off = self.body_scroll_disp * LINE_H;
        let first = self.body_scroll_disp as usize;
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - (line_off - first as f32 * LINE_H);
        for (i, (line, _)) in lines.iter().enumerate().skip(first) {
            if ty > text_area.y + text_area.h {
                break;
            }
            r.text(text_area.x, ty, line, 1.0, BLACK);
            // 캐럿은 실제로 그 줄이 이번에 화면에 그려졌을 때만(스크롤 때문에
            // 화면 밖으로 밀려났으면 그리지 않는다) 그린다 — 조합 중인 글자는
            // 위에서 이미 텍스트에 직접 끼워넣어 보여줬으므로, 여기 캐럿은 그냥
            // 평소처럼 깜빡이기만 한다.
            if self.new_mail.active == ComposeField::Body && win.focused && i == cursor_line {
                let (line_text, line_start) = &lines[i];
                let col = effective_cursor.saturating_sub(*line_start).min(line_text.chars().count());
                let byte_idx = line_text.char_indices().nth(col).map(|(b, _)| b).unwrap_or(line_text.len());
                let cursor_x = text_area.x + r.text_width(&line_text[..byte_idx], 1.0);
                // IME 팝업들은 화면 밖으로 치운다 — To/Subject 필드와 같은 이유
                // (draw_editable_field 참고).
                crate::platform::ime::set_composition_pos(OFFSCREEN, OFFSCREEN);
                crate::platform::ime::hide_cicero_windows();
                if (win.time % 1.0) < 0.5 {
                    r.rect(cursor_x + 1.0, ty + 1.0, 2.0, CELL_H - 4.0, BLACK);
                }
            }
            ty += LINE_H;
        }

        // 첨부파일들 — 본문 마지막 줄 다음부터 하나씩 이어 붙여서 같이
        // 스크롤된다(Inbox/Sent 읽기 화면과 같은 요령). 각 박스에 Remove 버튼이
        // 있어서 그 자리에서 바로 뗄 수 있다.
        let mut removed = None;
        for (i, (id, name, icon)) in self.new_mail.attachments.iter().enumerate() {
            let box_y = text_area.y + (lines.len() as f32 + attach_unit_lines * i as f32) * LINE_H + ATTACH_GAP - line_off;
            let box_w = body_area.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, icon, cx + 4.0, box_y + 3.0, 18.0);
                let remove_label = t(lang, s::REMOVE);
                let remove_w = r.text_width(remove_label, 0.8) + 16.0;
                let remove_x = cx + box_w - remove_w - 4.0;
                let name_max_w = (remove_x - (cx + 28.0) - 6.0).max(10.0);
                r.text_clipped(cx + 28.0, box_y + 5.0, &display_name(lang, name), 0.85, BLACK, name_max_w);
                if button(r, remove_x, box_y + 2.0, remove_w, box_h - 4.0, remove_label, win) {
                    removed = Some(*id);
                }
            }
        }
        r.set_clip(None);
        if let Some(id) = removed {
            self.new_mail.attachments.retain(|(aid, ..)| *aid != id);
        }
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 6.0;
            let frac = (visible as f32 / total_lines).min(1.0);
            scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, body_area.y + 4.0, SB_W, body_area.h - 8.0), visible_frac: frac, disp: self.body_scroll_disp, max_scroll: max_body_scroll }, &mut self.body_scroll, &mut self.body_sb_drag);
        }

        self.draw_new_compose_buttons(r, area, win, lang)
    }

    // "Write Mail" 하단의 Send/Attach 버튼 줄 — 원래 위쪽에 큼직한 툴바로 뒀었는데
    // ("보내기랑 첨부를 아래쪽으로 옮겨줘. 그리고 버튼 사이즈도 좀 줄여줘") 요청을
    // 받고 화면 맨 아래 작은 버튼 두 개로 옮겼다. 첨부 목록이 펼쳐져 있는 중에도
    // 그려야 해서(Attach 를 다시 눌러 닫을 수 있게) draw_new_compose 끝과 첨부 목록
    // 분기 양쪽에서 공유해 부른다.
    pub(super) fn draw_new_compose_buttons(&mut self, r: &mut Renderer, area: Rect, win: &WinInput, lang: Language) -> AppAction {
        const BTN_ROW_H: f32 = 30.0;
        const BTN_H: f32 = 20.0;
        let by = area.y + area.h - BTN_ROW_H + (BTN_ROW_H - BTN_H) / 2.0;
        r.rect(area.x, area.y + area.h - BTN_ROW_H, area.w, 1.0, GRAY);

        let attach_label = t(lang, s::ATTACH_BUTTON);
        let attach_w = r.text_width(attach_label, 0.85) + 16.0;
        let send_label = t(lang, s::SEND_BUTTON);
        let send_w = r.text_width(send_label, 0.85) + 16.0;
        // 오른쪽 정렬 — Send 가 가장 오른쪽(가장 눌릴 확률이 높은 주 동작).
        let send_x = area.x + area.w - send_w - 8.0;
        let attach_x = send_x - attach_w - 6.0;

        let attach_rect = Rect::new(attach_x, by, attach_w, BTN_H);
        if self.new_mail.picker_open {
            sunken(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            let tw = r.text_width(attach_label, 0.85);
            r.text(attach_rect.x + (attach_w - tw) / 2.0 + 1.0, attach_rect.y + (BTN_H - CELL_H * 0.85) / 2.0 + 1.0, attach_label, 0.85, BLACK);
            if win.mouse_clicked && attach_rect.contains(win.mouse.0, win.mouse.1) {
                self.new_mail.picker_open = false;
            }
        } else {
            let hover = attach_rect.contains(win.mouse.0, win.mouse.1);
            let pressed = hover && win.mouse_down;
            if pressed {
                sunken(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            } else {
                raised(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            }
            let tw = r.text_width(attach_label, 0.85);
            let off = if pressed { 1.0 } else { 0.0 };
            r.text(attach_rect.x + (attach_w - tw) / 2.0 + off, attach_rect.y + (BTN_H - CELL_H * 0.85) / 2.0 + off, attach_label, 0.85, BLACK);
            if hover && win.mouse_clicked {
                self.new_mail.picker_open = true;
            }
        }

        let can_send = !self.new_mail.to.trim().is_empty() && !self.new_mail.body.trim().is_empty();
        if can_send {
            let hover = Rect::new(send_x, by, send_w, BTN_H).contains(win.mouse.0, win.mouse.1);
            let pressed = hover && win.mouse_down;
            if pressed {
                sunken(r, send_x, by, send_w, BTN_H);
            } else {
                raised(r, send_x, by, send_w, BTN_H);
            }
            let tw = r.text_width(send_label, 0.85);
            let off = if pressed { 1.0 } else { 0.0 };
            r.text(send_x + (send_w - tw) / 2.0 + off, by + (BTN_H - CELL_H * 0.85) / 2.0 + off, send_label, 0.85, BLACK);
            if hover && win.mouse_clicked {
                // 발송 확인 화면 없이 곧바로 처리한다 — Sent Items 목록에 바로
                // 하나 덧붙이고(이 자리에서 즉시 보이도록), 폼은 다음 메일을 쓸 수
                // 있게 비운다. fs.sent_mail 쪽 진짜 기록은 desktop.rs 가 아래
                // AppAction 을 받아서 남긴다.
                let to = self.new_mail.to.clone();
                let subject = self.new_mail.subject.clone();
                let body = self.new_mail.body.clone();
                let attachments_view = self.new_mail.attachments.clone();
                let attachments = attachments_view.iter().map(|(id, name, _)| (*id, name.clone())).collect();
                self.sent.push(SentMailView { to: to.clone(), subject: subject.clone(), body: body.clone(), attachments: attachments_view });
                self.new_mail = NewMailState::new();
                return AppAction::SendNewMail { to, subject, body, attachments };
            }
        } else {
            raised(r, send_x, by, send_w, BTN_H);
            let tw = r.text_width(send_label, 0.85);
            r.text(send_x + (send_w - tw) / 2.0, by + (BTN_H - CELL_H * 0.85) / 2.0, send_label, 0.85, [0.55, 0.55, 0.55, 1.0]);
        }

        AppAction::None
    }
}
