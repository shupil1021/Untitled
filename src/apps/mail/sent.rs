//! 보낸편지함 화면 — "Write Mail" 에서 실제로 보낸 메일 목록과 읽기 화면.

use super::*;

impl MailApp {
    // 보낸편지함(Sent Items) — Inbox 의 목록/읽기 구조를 그대로 본떴지만 훨씬
    // 단순하다: 안 읽음 배지/굵은 글씨가 없고(자기가 보낸 걸 "안 읽음" 취급할
    // 이유가 없다), 첨부는 다운로드 버튼 없이 그냥 정보로만 보여준다(이미 내가
    // 가지고 있던 파일이라 "받을" 게 없다). 목록 헤더도 From 대신 To.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_sent_pane(&mut self, r: &mut Renderer, assets: &Assets, pane: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        if self.sent.is_empty() {
            draw_empty_state(r, assets, pane, IconType::Envelope, t(lang, s::NO_SENT_MESSAGES));
            return AppAction::None;
        }

        let Some(msg_idx) = self.selected else {
            let from_col_w = (pane.w * FROM_COL_FRAC).clamp(FROM_COL_MIN, FROM_COL_MAX).min(pane.w * 0.6);
            raised(r, pane.x, pane.y, from_col_w, LIST_HEADER_H);
            raised(r, pane.x + from_col_w, pane.y, pane.w - from_col_w, LIST_HEADER_H);
            let header_ty = pane.y + (LIST_HEADER_H - CELL_H * 0.85) / 2.0;
            r.text(pane.x + 6.0, header_ty, t(lang, s::COL_TO), 0.85, BLACK);
            r.text(pane.x + from_col_w + 6.0, header_ty, t(lang, s::COL_SUBJECT), 0.85, BLACK);

            const TEXT_INSET: f32 = 6.0;
            let list_area = Rect::new(pane.x, pane.y + LIST_HEADER_H, pane.w, pane.h - LIST_HEADER_H);
            let line_h = CELL_H * TEXT_SCALE + 2.0;
            let preview_line_h = CELL_H * PREVIEW_SCALE + 2.0;
            let row_w = pane.w - ROW_PAD * 2.0;
            let from_max_w = from_col_w - TEXT_INSET * 2.0;
            let subject_max_w = row_w - from_col_w - TEXT_INSET * 2.0;
            let preview_max_w = (row_w - from_col_w - TEXT_INSET).max(10.0);
            let row_h = line_h + preview_line_h * PREVIEW_LINES as f32 + ROW_PAD * 2.0;
            let total_h = (row_h + ROW_GAP) * self.sent.len() as f32;
            let max_list_scroll = (total_h - list_area.h).max(0.0);
            if list_area.contains(win.mouse.0, win.mouse.1) {
                self.list_scroll -= win.wheel / 120.0 * (line_h * 2.0);
            }
            self.list_scroll = self.list_scroll.clamp(0.0, max_list_scroll);
            ease_scroll(&mut self.list_scroll_disp, self.list_scroll, win.dt, smooth);

            r.set_clip(Some(list_area));
            let mut clicked_row = None;
            let mut ry = list_area.y - self.list_scroll_disp;
            for (i, msg) in self.sent.iter().enumerate() {
                if ry + row_h >= list_area.y && ry <= list_area.y + list_area.h {
                    let row = Rect::new(pane.x + ROW_PAD, ry, row_w, row_h - ROW_GAP);
                    let hover = row.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
                    if hover {
                        r.rect(row.x + 2.0, row.y + 2.0, row.w - 4.0, row.h - 4.0, [0.0, 0.0, 0.4, 0.08]);
                    }
                    let to = truncate_ellipsis(r, &msg.to, TEXT_SCALE, from_max_w);
                    let subject = truncate_ellipsis(r, &msg.subject, TEXT_SCALE, subject_max_w);
                    let ty = row.y + ROW_PAD;
                    r.text(row.x + TEXT_INSET, ty, &to, TEXT_SCALE, BLACK);
                    r.text(row.x + from_col_w + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    r.rect(row.x + from_col_w, row.y, 1.0, row.h, [0.85, 0.85, 0.85, 1.0]);

                    let preview = preview_lines(r, &msg.body, preview_max_w);
                    let mut pty = ty + line_h;
                    for pline in &preview {
                        r.text(row.x + from_col_w + TEXT_INSET, pty, pline, PREVIEW_SCALE, GRAY);
                        pty += preview_line_h;
                    }

                    if win.mouse_clicked && hover {
                        clicked_row = Some(i);
                    }
                }
                ry += row_h + ROW_GAP;
            }
            r.set_clip(None);
            if let Some(i) = clicked_row {
                self.selected = Some(i);
                self.body_scroll = 0.0;
                self.body_scroll_disp = 0.0;
            }
            if max_list_scroll > 0.0 {
                let sb_x = pane.x + pane.w - 8.0;
                let frac = (list_area.h / total_h).min(1.0);
                scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, list_area.y, 8.0, list_area.h), visible_frac: frac, disp: self.list_scroll_disp, max_scroll: max_list_scroll }, &mut self.list_scroll, &mut self.list_sb_drag);
            }
            return AppAction::None;
        };

        // ---------------- 보낸 메일 하나를 골랐음: 읽기 화면 ----------------
        let back_row = Rect::new(pane.x, pane.y, pane.w, BACK_ROW_H);
        let back_hover = back_row.contains(win.mouse.0, win.mouse.1);
        let back_color = if back_hover { NAVY } else { BLACK };
        let back_label = t(lang, s::BACK_TO_SENT);
        r.text(back_row.x + 6.0, back_row.y + (BACK_ROW_H - CELL_H) / 2.0, back_label, 1.0, back_color);
        if back_hover {
            let tw = r.text_width(back_label, 1.0);
            r.rect(back_row.x + 6.0, back_row.y + BACK_ROW_H - 4.0, tw, 1.0, back_color);
        }
        if win.mouse_clicked && back_hover {
            self.selected = None;
        }

        r.rect(pane.x, pane.y + BACK_ROW_H - 1.0, pane.w, 1.0, GRAY);
        let content = Rect::new(pane.x, pane.y + BACK_ROW_H, pane.w, pane.h - BACK_ROW_H);

        let attach_count = self.sent[msg_idx].attachments.len();
        let cx = content.x + 8.0;
        let field_w = content.w - 16.0;

        const FIELD_ROW_H: f32 = 20.0;
        let mut fy = content.y + 6.0;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_TO), &self.sent[msg_idx].to);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_SUBJECT), &self.sent[msg_idx].subject);
        fy += FIELD_ROW_H + 6.0;
        r.rect(content.x + 4.0, fy, content.w - 8.0, 1.0, GRAY);
        fy += 6.0;

        // Inbox 읽기 화면과 같은 요령 — 첨부 박스도 따로 자리를 안 빼고 본문
        // 텍스트 마지막 줄 바로 다음 "가상의 줄" 로 취급해서 같이 스크롤된다.
        let body_area = Rect::new(content.x, fy, content.w, content.y + content.h - fy);
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        const ATTACH_BOX_H: f32 = 38.0;
        const ATTACH_GAP: f32 = 10.0;
        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, [0.6, 0.6, 0.62, 1.0]);
        let text_w = body_area.w - 20.0 - SB_W;
        let key = (msg_idx, text_w.round() as i32);
        if key != self.sent_wrapped_key {
            self.sent_wrapped = wrap_lines(r, &self.sent[msg_idx].body, 1.0, text_w);
            self.sent_wrapped_key = key;
        }
        let lines = &self.sent_wrapped;
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 2.0, body_area.w - 12.0, body_area.h - 4.0);
        let visible = (text_area.h / LINE_H).floor() as usize;
        let attach_unit_lines = (ATTACH_BOX_H + ATTACH_GAP) / LINE_H;
        let total_lines = lines.len() as f32 + attach_unit_lines * attach_count as f32;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);
        if body_area.contains(win.mouse.0, win.mouse.1) {
            self.body_scroll -= win.wheel / 120.0 * 3.0;
        }
        self.body_scroll = self.body_scroll.clamp(0.0, max_body_scroll);
        ease_scroll(&mut self.body_scroll_disp, self.body_scroll, win.dt, smooth);

        let line_off = self.body_scroll_disp * LINE_H;
        let first = self.body_scroll_disp as usize;
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - (line_off - first as f32 * LINE_H);
        for line in lines.iter().skip(first) {
            if ty > text_area.y + text_area.h {
                break;
            }
            r.text(text_area.x, ty, line, 1.0, BLACK);
            ty += LINE_H;
        }

        // 첨부파일들 — 다운로드 버튼 없이 그냥 아이콘+이름만(이미 내가 가진
        // 파일이라 "받을" 개념이 없다). 본문 마지막 줄 다음부터 하나씩 이어
        // 붙여서 같이 스크롤된다.
        for (i, (_, name, icon)) in self.sent[msg_idx].attachments.iter().enumerate() {
            let box_y = text_area.y + (lines.len() as f32 + attach_unit_lines * i as f32) * LINE_H + ATTACH_GAP - line_off;
            // 스크롤바(SB_W)가 뜨는 오른쪽 끝 자리는 항상 비워둔다 — 안 그러면
            // 스크롤이 필요한 긴 메일에서 첨부 박스가 스크롤바와 겹쳐 보인다.
            let box_w = content.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, icon, cx + 5.0, box_y + 3.0, 26.0);
                r.text_clipped(cx + 38.0, box_y + 11.0, name, 0.85, BLACK, box_w - 44.0);
            }
        }
        r.set_clip(None);
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 8.0;
            let frac = visible as f32 / total_lines;
            scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, body_area.y + 2.0, SB_W, body_area.h - 4.0), visible_frac: frac, disp: self.body_scroll_disp, max_scroll: max_body_scroll }, &mut self.body_scroll, &mut self.body_sb_drag);
        }

        AppAction::None
    }
}
