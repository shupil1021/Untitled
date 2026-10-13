//! 받은편지함 화면 — 메일 목록(From/Subject/미리보기)과 메일 하나를 읽는 화면(첨부 다운로드 버튼 포함).

use super::*;

impl MailApp {
    // pane 폭 안에 폴더 내용을 그린다 — update() 에서 분리한 이유는 이 함수 안에서
    // 여러 곳에 `return` 을 자유롭게 쓰기 위해서다(update() 자체에 흩어두면 그 뒤에
    // 그려야 하는 상태바를 매번 같이 호출해줘야 해서 번거롭다).
    pub(super) fn draw_pane_content(&mut self, r: &mut Renderer, assets: &Assets, pane: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        // 참고 이미지처럼 오른쪽 내용 패널은 항상 흰 배경 — 이전엔 이 밑칠이 없어서
        // 빈 상태 화면들이 회색(FACE) 바탕 위에 떠 있었다.
        r.rect(pane.x, pane.y, pane.w, pane.h, WHITE);
        let Some(folder) = self.folder else {
            let msg = t(lang, s::SELECT_FOLDER_MSG);
            draw_empty_state(r, assets, pane, IconType::Mail, msg);
            return AppAction::None;
        };
        if folder == MailFolder::Compose {
            return self.draw_new_compose(r, assets, pane, win, smooth, lang);
        }
        if folder == MailFolder::Sent {
            return self.draw_sent_pane(r, assets, pane, win, smooth, lang);
        }

        // ---------------- Inbox 선택됨 ----------------
        if self.messages.is_empty() {
            // 아직 첫 메일이 도착하기 전 — 받은편지함이 빈 상태.
            draw_empty_state(r, assets, pane, IconType::Envelope, t(lang, s::NO_NEW_MESSAGES));
            return AppAction::None;
        }

        // 메시지를 아직 안 골랐으면 목록이 pane 전체 폭을 쓰고, 하나를 고르면 그
        // 내용이 pane 전체 폭을 쓴다 — 목록/읽기가 서로 다른 화면이라 각자 넓게 쓴다.
        let Some(msg_idx) = self.selected else {
            // 헤더 — 고전 메일 클라이언트의 From/Subject 컬럼 헤더를 흉내낸다.
            // My Computer(explorer.rs) 의 Name/Size 헤더처럼 raised 바 두 칸으로
            // 튀어나온 느낌을 살렸다(전에는 이 느낌이 과하다는 피드백에 flat 바로
            // 바꿨었는데, 이번엔 반대로 My Computer 처럼 튀어나오게 해달라는 요청).
            // "받은 날짜는 없어도 되니까 글자 짤리는 걸 중점적으로 고쳐줘" 요청으로
            // Received 칸을 뺐다 — 그 칸이 차지하던 폭을 From/Subject 가 그대로
            // 돌려받아서 더 안 잘리게 된다.
            let from_col_w = (pane.w * FROM_COL_FRAC).clamp(FROM_COL_MIN, FROM_COL_MAX).min(pane.w * 0.6);
            let subject_col_x = pane.x + from_col_w;
            let subject_col_w = (pane.w - from_col_w).max(0.0);
            raised(r, pane.x, pane.y, from_col_w, LIST_HEADER_H);
            raised(r, subject_col_x, pane.y, subject_col_w, LIST_HEADER_H);
            let header_ty = pane.y + (LIST_HEADER_H - CELL_H * 0.85) / 2.0;
            r.text(pane.x + 6.0, header_ty, t(lang, s::COL_FROM), 0.85, BLACK);
            r.text(subject_col_x + 6.0, header_ty, t(lang, s::COL_SUBJECT), 0.85, BLACK);

            // 행 안쪽에서 글자가 시작/끝나는 좌우 여백.
            const TEXT_INSET: f32 = 6.0;
            let list_area = Rect::new(pane.x, pane.y + LIST_HEADER_H, pane.w, pane.h - LIST_HEADER_H);
            let line_h = CELL_H * TEXT_SCALE + 2.0;
            let preview_line_h = CELL_H * PREVIEW_SCALE + 2.0;
            let row_w = pane.w - ROW_PAD * 2.0;
            let from_max_w = from_col_w - TEXT_INSET * 2.0;
            let subject_max_w = subject_col_w - TEXT_INSET * 2.0;
            // 헤더 한 줄(From/Subject) 아래에 본문 미리보기 최대 두 줄 — 참고
            // 이미지의 "AutoPreview" 느낌. 미리보기는 Subject 칸과 같은 x 에서
            // 시작해서 행 끝까지 넓게 쓴다(From 칸 밑으로는 안 내려온다).
            let preview_max_w = (row_w - from_col_w - TEXT_INSET).max(10.0);
            let row_h = line_h + preview_line_h * PREVIEW_LINES as f32 + ROW_PAD * 2.0;
            let total_h = (row_h + ROW_GAP) * self.messages.len() as f32;
            let max_list_scroll = (total_h - list_area.h).max(0.0);
            if list_area.contains(win.mouse.0, win.mouse.1) {
                self.list_scroll -= win.wheel / 120.0 * (line_h * 2.0);
            }
            self.list_scroll = self.list_scroll.clamp(0.0, max_list_scroll);
            ease_scroll(&mut self.list_scroll_disp, self.list_scroll, win.dt, smooth);

            r.set_clip(Some(list_area));
            let mut clicked_row = None;
            let mut ry = list_area.y - self.list_scroll_disp;
            // 최신 메일이 맨 위 — 도착 순서(messages)를 거꾸로 훑는다. i 는 원래 번호라 읽음 기록/선택은 그대로 맞는다.
            for i in (0..self.messages.len()).rev() {
                let msg = &self.messages[i];
                if ry + row_h >= list_area.y && ry <= list_area.y + list_area.h {
                    let row = Rect::new(pane.x + ROW_PAD, ry, row_w, row_h - ROW_GAP);
                    let hover = row.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
                    // 이 목록은 selected == None 일 때만 그려지므로(그 아래 `let Some
                    // (msg_idx) = self.selected else` 분기) 여기 자체엔 "선택된" 행이
                    // 있을 수 없다 — hover 만 표시한다. 예전엔 행마다 sunken() 으로
                    // 회색 박스를 그렸는데, 목록 전체를 흰색으로 해달라는 요청으로
                    // 뺐다 — pane 배경이 이미 흰색이라 행 자체엔 따로 안 칠해도 된다.
                    if hover {
                        r.rect(row.x + 2.0, row.y + 2.0, row.w - 4.0, row.h - 4.0, [0.0, 0.0, 0.4, 0.08]);
                    }
                    // 안 읽은 메일은 굵게 — 참고 이미지의 "Welcome!" 행처럼. 한 줄로
                    // 강제 — 다 안 들어가면 잘라내고 "..." 를 붙인다.
                    let unread = !self.read.get(i).copied().unwrap_or(true);
                    let from = truncate_ellipsis(r, msg.from, TEXT_SCALE, from_max_w);
                    let subject = truncate_ellipsis(r, msg.subject, TEXT_SCALE, subject_max_w);
                    let ty = row.y + ROW_PAD;
                    if unread {
                        draw_bold_text(r, row.x + TEXT_INSET, ty, &from, TEXT_SCALE, BLACK);
                        draw_bold_text(r, subject_col_x + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    } else {
                        r.text(row.x + TEXT_INSET, ty, &from, TEXT_SCALE, BLACK);
                        r.text(subject_col_x + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    }
                    r.rect(row.x + from_col_w, row.y, 1.0, row.h, [0.85, 0.85, 0.85, 1.0]); // 컬럼 구분선

                    // 본문 미리보기 — 회색, 굵지 않게(안 읽음이어도). 두 줄 안에 다
                    // 못 들어가면 마지막 줄 끝에 "..." 를 붙여서, 문장 중간에서
                    // 그냥 뚝 잘린 것처럼 안 보이게 한다.
                    let preview = preview_lines(r, msg.body, preview_max_w);
                    let mut pty = ty + line_h;
                    for pline in &preview {
                        r.text(subject_col_x + TEXT_INSET, pty, pline, PREVIEW_SCALE, GRAY);
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
                self.select(i);
            }
            if max_list_scroll > 0.0 {
                let sb_x = pane.x + pane.w - 8.0;
                let frac = (list_area.h / total_h).min(1.0);
                scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, list_area.y, 8.0, list_area.h), visible_frac: frac, disp: self.list_scroll_disp, max_scroll: max_list_scroll }, &mut self.list_scroll, &mut self.list_sb_drag);
            }
            // 예전엔 여기서 헤더+목록만 따로 한 번 더 회색 테두리로 감쌌는데, 이제
            // update() 가 트리+내용 전체를 이미 한 번 감싸므로 이중 테두리가 되어
            // 뺐다.
            // 방금 고른 메시지가 있으면 desktop.rs 에게 fs.mail_read 에 기록해달라고
            // 요청한다 — 화면(self.read)은 select() 가 이미 바로 갱신했지만, 그건
            // 새로고침 때마다 사라지는 인스턴스 상태라 진짜 기록은 fs 쪽에 남겨야 한다.
            return match clicked_row {
                Some(i) => AppAction::MarkMailRead(i),
                None => AppAction::None,
            };
        };

        // ---------------- 메시지 하나를 골랐음: pane 전체 폭을 다 쓰는 읽기 화면.
        // 맨 위에 목록으로 돌아가는 링크를 붙인다(폴더 트리에서 Inbox 를 다시 눌러도
        // 똑같이 목록으로 돌아가지만, 이쪽이 더 눈에 잘 띈다). ----------------
        let back_row = Rect::new(pane.x, pane.y, pane.w, BACK_ROW_H);
        let back_hover = back_row.contains(win.mouse.0, win.mouse.1);
        let back_color = if back_hover { NAVY } else { BLACK };
        let back_label = t(lang, s::BACK_TO_INBOX);
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

        let has_attachment = self.messages[msg_idx].attachment.is_some();
        let cx = content.x + 8.0;
        let field_w = content.w - 16.0;

        // From/To/Cc/Subject 라벨 필드 블록 — 한 줄씩, 값 전체 폭을 다 쓴다("받은
        // 날짜는 없어도 되니까 글자 짤리는 걸 고쳐줘" 요청으로 Sent 필드를 빼고
        // From 을 다시 한 줄 전체 폭으로 되돌렸다 — 반으로 나눠 쓰던 때는
        // "PalaceCompany@email.com" 같은 긴 주소가 훨씬 더 심하게 잘렸다). 맨
        // 끝에 얇은 구분선을 하나 그어서 본문 상자와 분리한다.
        const FIELD_ROW_H: f32 = 20.0;
        let mut fy = content.y + 6.0;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_FROM), self.messages[msg_idx].from);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_TO), self.messages[msg_idx].to);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_CC), self.messages[msg_idx].cc);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_SUBJECT), self.messages[msg_idx].subject);
        fy += FIELD_ROW_H + 6.0;
        r.rect(content.x + 4.0, fy, content.w - 8.0, 1.0, GRAY);
        fy += 6.0;

        // 첨부파일이 있어도 이제 따로 자리를 안 빼고(예전엔 attach_h 만큼 밑을
        // 잘라 항상 하단에 고정했었다) 본문 상자 전체를 텍스트 스크롤 영역으로
        // 쓴다 — 첨부 박스는 본문 마지막 줄 바로 아래에 이어지는 "콘텐츠의 일부"로
        // 취급해서 텍스트와 같이 스크롤된다.
        let body_area = Rect::new(content.x, fy, content.w, content.y + content.h - fy);
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        const ATTACH_BOX_H: f32 = 38.0;
        const ATTACH_GAP: f32 = 10.0; // 본문 마지막 줄과 첨부 박스 사이 여백
        // draw_field() 와 같은 이유로 sunken() 대신 흰 배경 + 얇은 회색 테두리.
        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, [0.6, 0.6, 0.62, 1.0]);
        let text_w = body_area.w - 20.0 - SB_W;
        // 캐시 무효화 키에 폭을 그대로(f32) 쓰지 않고 정수 픽셀로 반올림해 담는다 —
        // 실수를 직접 비교하면 부동소수점 오차로 같은 폭인데도 매 프레임 다시
        // 계산해버릴 위험이 있다.
        let key = (msg_idx, text_w.round() as i32);
        if key != self.body_wrapped_key {
            self.body_wrapped = wrap_lines(r, self.messages[msg_idx].body, 1.0, text_w);
            self.body_wrapped_key = key;
        }
        let lines = &self.body_wrapped;
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 2.0, body_area.w - 12.0, body_area.h - 4.0);
        let visible = (text_area.h / LINE_H).floor() as usize;
        // 첨부 박스 몫(있으면 여백+박스 높이)을 "줄 수"로 환산해서 스크롤 가능
        // 범위에 더한다 — 그래야 마지막 줄까지 다 내려도 첨부 박스가 화면에
        // 다 들어올 때까지 더 스크롤된다.
        let attach_extra_lines = if has_attachment { (ATTACH_BOX_H + ATTACH_GAP) / LINE_H } else { 0.0 };
        let total_lines = lines.len() as f32 + attach_extra_lines;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);
        // 마우스가 본문 박스 위에 있을 때만 휠을 먹는다(목록 쪽과 대칭되는 이유).
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

        // 첨부파일 — 본문 텍스트와 같은 좌표계(스크롤 오프셋 line_off) 위에서
        // 마지막 줄 바로 다음 "가상의 줄" 자리에 그린다. 다운로드 진행 타이머는
        // 화면 밖으로 스크롤돼도 멈추면 안 되니(눌러놓고 스크롤해서 딴 걸 보는
        // 동안에도 계속 받아지고 있어야 자연스럽다) 그리기 여부와 상관없이 항상
        // 갱신하고, 실제 UI(아이콘/버튼)만 보이는 범위일 때만 그린다.
        let mut finished_download = None;
        if let Some((id, name, icon)) = self.messages[msg_idx].attachment.clone() {
            if let Some(elapsed) = self.downloading[msg_idx] {
                let elapsed = elapsed + win.dt;
                if elapsed >= DOWNLOAD_DELAY {
                    self.downloading[msg_idx] = None;
                    self.downloaded[msg_idx] = true;
                    finished_download = Some(id);
                } else {
                    self.downloading[msg_idx] = Some(elapsed);
                }
            }

            let box_y = text_area.y + lines.len() as f32 * LINE_H + ATTACH_GAP - line_off;
            // 스크롤바(SB_W)가 뜨는 오른쪽 끝 자리는 항상 비워둔다 — 안 그러면
            // 스크롤이 필요한 긴 메일에서 첨부 박스의 다운로드 버튼이 스크롤바와
            // 겹쳐 보인다.
            let box_w = content.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, &icon, cx + 5.0, box_y + 3.0, 26.0);

                let dl_label = t(lang, s::DOWNLOAD);
                let dl_ing_label = t(lang, s::DOWNLOADING);
                let dl_done_label = t(lang, s::DOWNLOADED);
                // 세 상태(Download/Downloading/Downloaded)가 언어마다 폭이 다른데
                // (한국어/일본어가 영어보다 김) 예전엔 90px 로 고정해뒀다 — 한국어
                // "다운로드"/"다운로드 중" 같은 문구가 버튼 밖으로 삐져나와 스크롤바
                // 쪽과 겹쳐 보이는 문제로 이어졌다(제보받음). 세 상태 각각 실제로
                // 필요한 폭을 계산해 가장 넓은 쪽에 맞춘다 — 그러면 상태가 바뀔 때
                // 버튼이 다시 커지거나 하지 않고, 어떤 언어에서도 항상 다 들어간다.
                let btn_w = (r.text_width(dl_label, 1.0) + 16.0)
                    .max(22.0 + r.text_width(dl_ing_label, 0.75) + 6.0)
                    .max(6.0 + r.text_width(dl_done_label, 0.8) + 6.0)
                    .max(70.0);
                let btn_h = 22.0;
                let btn_x = cx + box_w - btn_w - 6.0;
                let btn_y = box_y + (box_h - btn_h) / 2.0;
                let name_max_w = (btn_x - (cx + 38.0) - 6.0).max(10.0); // 버튼과 안 겹치게 그 앞까지만
                r.text_clipped(cx + 38.0, box_y + 11.0, &name, 0.85, BLACK, name_max_w);
                // 다운로드 버튼을 누르면 바로 완료 처리하지 않고 DOWNLOAD_DELAY 초 동안
                // 스피너 + "Downloading..." 를 보여주는 연출을 먼저 거친다 — 그래야 클릭
                // 즉시 파일이 나타나는 대신 뭔가 "받아오는 중"이라는 느낌이 든다.
                if self.downloaded[msg_idx] {
                    r.text(btn_x + 6.0, btn_y + 3.0, dl_done_label, 0.8, GRAY);
                } else if self.downloading[msg_idx].is_some() {
                    sunken(r, btn_x, btn_y, btn_w, btn_h);
                    draw_spinner(r, btn_x + 12.0, btn_y + btn_h / 2.0, 6.0, win.time);
                    r.text_clipped(btn_x + 22.0, btn_y + 3.0, dl_ing_label, 0.75, GRAY, btn_w - 26.0);
                } else if button(r, btn_x, btn_y, btn_w, btn_h, dl_label, win) {
                    self.downloading[msg_idx] = Some(0.0);
                }
            }
        }
        r.set_clip(None);
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 8.0;
            let frac = visible as f32 / total_lines;
            scrollbar(r, win, Scrollbar { track: Rect::new(sb_x, body_area.y + 2.0, SB_W, body_area.h - 4.0), visible_frac: frac, disp: self.body_scroll_disp, max_scroll: max_body_scroll }, &mut self.body_scroll, &mut self.body_sb_drag);
        }

        if let Some(id) = finished_download {
            return AppAction::Download(id);
        }

        AppAction::None
    }
}
