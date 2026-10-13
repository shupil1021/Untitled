//! 바탕화면 위의 UI 조각들 — 작업표시줄, 시작 메뉴, 우클릭 메뉴, 시계/와이파이 팝업, 우측 하단 알림(토스트), 삭제 확인창.

use super::clock::civil_date;
use super::*;

impl DesktopScene {
    pub(super) fn start_button_rect(&self) -> Rect {
        Rect::new(2.0, SCREEN_H - TASKBAR_H + 3.0, 42.0, TASKBAR_H - 6.0)
    }

    pub(super) fn clock_rect(&self) -> Rect {
        // 왼쪽에 와이파이 아이콘이 들어갈 자리(24px)를 남겨두고 그만큼 왼쪽으로 옮긴 뒤,
        // 화면 오른쪽 끝에 너무 붙지 않도록 10px 만큼 다시 오른쪽으로 뺐다. 높이는
        // 시작 버튼/창 버튼과 같은 TASKBAR_H-6 으로 맞춘다 — 예전엔 TASKBAR_H-10(18px)
        // 이었는데 글자 한 줄 높이(CELL_H*0.9 ≈ 20px)보다 낮아서 시계 숫자가 sunken
        // 박스 위아래로 살짝 삐져나와 보였다.
        const W: f32 = 70.0;
        const RIGHT_MARGIN: f32 = 10.0;
        Rect::new(SCREEN_W - RIGHT_MARGIN - W, SCREEN_H - TASKBAR_H + 3.0, W, TASKBAR_H - 6.0)
    }

    pub(super) fn wifi_rect(&self) -> Rect {
        let ck = self.clock_rect();
        Rect::new(ck.x - 22.0, ck.y, 18.0, ck.h)
    }

    // 와이파이 아이콘을 누르면 그 위에 뜨는 작은 연결 정보 팝업.
    pub(super) fn wifi_popup_rect(&self) -> Rect {
        let (w, h) = (220.0, 78.0); // "Status: Disconnected" 같은 긴 줄이 안 삐져나오게 넉넉히
        let wr = self.wifi_rect();
        let x = (wr.x + wr.w - w).max(4.0);
        Rect::new(x, SCREEN_H - TASKBAR_H - h, w, h)
    }

    pub(super) fn start_menu_rect(&self, r: &Renderer, lang: Language) -> Rect {
        let h = MENU_ITEMS.len() as f32 * 24.0 + 8.0;
        // 가장 긴 번역 문구 폭에 맞춰 너비를 정해서 텍스트가 회색 영역을 벗어나지
        // 않게 한다 — ADVANCE(고정폭) 기반 글자수 계산 대신 실제 text_width() 를
        // 쓴다(한글/일본어는 라틴 문자와 폭이 다른 글자가 섞여 있어 글자수만으론
        // 안 맞는다).
        let max_w = MENU_ITEMS.iter().map(|&k| r.text_width(menu_item_label(lang, k), 1.0)).fold(0.0f32, f32::max);
        let w = max_w + 40.0;
        Rect::new(2.0, SCREEN_H - TASKBAR_H - h, w, h)
    }

    // 우클릭한 지점 pos 에 메뉴를 띄우되, 화면(작업표시줄 위쪽) 밖으로 안 나가게 clamp.
    pub(super) fn context_menu_rect(r: &Renderer, lang: Language, pos: (f32, f32)) -> Rect {
        let h = CONTEXT_MENU_ITEMS.len() as f32 * CTX_ROW_H + 6.0;
        let max_w = CONTEXT_MENU_ITEMS.iter().map(|&k| r.text_width(menu_item_label(lang, k), CTX_TEXT_SCALE)).fold(0.0f32, f32::max);
        let w = max_w + 24.0;
        let x = pos.0.min(SCREEN_W - w).max(0.0);
        let y = pos.1.min(SCREEN_H - TASKBAR_H - h).max(0.0);
        Rect::new(x, y, w, h)
    }

    // "Erase All Memory" 확인창의 다이얼로그 박스 + Erase/Cancel 버튼 자리. 화면
    // 전체(640x480) 기준 절대좌표라 어떤 창이 위에 떠 있어도 항상 화면 정중앙에
    // 뜬다 — 입력 판정(update())과 그리기(draw_erase_confirm()) 양쪽에서 같이 써서
    // 자리가 어긋나지 않게 한다.
    pub(super) fn erase_confirm_layout() -> (Rect, Rect, Rect) {
        let dw = 220.0;
        let dh = 110.0;
        let dr = Rect::new((SCREEN_W - dw) / 2.0, (SCREEN_H - dh) / 2.0, dw, dh);
        let bw = 80.0;
        let by = dr.y + dr.h - 34.0;
        let erase_btn = Rect::new(dr.x + 12.0, by, bw, 22.0);
        let cancel_btn = Rect::new(dr.x + dr.w - 12.0 - bw, by, bw, 22.0);
        (dr, erase_btn, cancel_btn)
    }

    // 화면 전체(다른 창 포함)를 반투명 흰색 워시로 덮어 "뿌옇게 흐려진" 느낌을 낸
    // 다음, 그 위에 확인 다이얼로그를 그린다. 진짜 가우시안 블러는 렌더러가 프레임
    // 끝에 한 번에만 그리는 배치 방식이라(그리는 도중엔 이미 그려진 화면을 다시
    // 읽어 블러할 수가 없다) 안 되고, 이 정도가 엔진을 안 뜯어고치고 낼 수 있는
    // 현실적인 타협점이다.
    pub(super) fn draw_erase_confirm(&self, r: &mut Renderer, _time: f32, mouse: (f32, f32), lang: Language) {
        r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, [1.0, 1.0, 1.0, 0.4]);

        let (dr, erase_btn, cancel_btn) = Self::erase_confirm_layout();
        raised(r, dr.x, dr.y, dr.w, dr.h);
        r.text_clipped(dr.x + 12.0, dr.y + 12.0, t(lang, s::ERASE_LINE1), 0.8, BLACK, dr.w - 24.0);
        r.text_clipped(dr.x + 12.0, dr.y + 32.0, t(lang, s::ERASE_LINE2), 0.8, BLACK, dr.w - 24.0);

        // 다른 창들과 같은 raised/sunken 버튼 스타일이 아니라 여기서는 그냥 살짝
        // 눌림 효과만 흉내내는 간단한 hover 표시 — button() 헬퍼는 WinInput 이
        // 필요한데 여기는 화면 전체 모달이라 안 맞아서 직접 그린다. 버튼 자리
        // 자체는 고정 Rect(erase_confirm_layout)라 번역 문구 길이가 달라져도
        // 클릭 판정은 안 흔들린다.
        let erase_label = t(lang, s::ERASE);
        let cancel_label = t(lang, common::CANCEL);
        for (btn, label) in [(erase_btn, erase_label), (cancel_btn, cancel_label)] {
            let hover = btn.contains(mouse.0, mouse.1);
            if hover {
                sunken(r, btn.x, btn.y, btn.w, btn.h);
            } else {
                raised(r, btn.x, btn.y, btn.w, btn.h);
            }
            let tw = r.text_width(label, 1.0);
            r.text(btn.x + (btn.w - tw) / 2.0, btn.y + 1.0, label, 1.0, BLACK);
        }
    }

    pub(super) fn taskbar_buttons(&self) -> Vec<(u32, String, Rect, bool, bool)> {
        let ty = SCREEN_H - TASKBAR_H;
        let items = self.wm.taskbar_items();
        let start_x = 50.0;
        // 와이파이 아이콘이 시계보다 왼쪽에 따로 자리를 차지하고 있어서, 시계 x 좌표까지
        // 남는 공간을 다 쓰면 그 아이콘이랑 창 버튼 글자가 겹친다 — 와이파이 아이콘의
        // x 좌표를 오른쪽 경계로 써야 한다.
        let right_bound = self.wifi_rect().x;
        let avail = right_bound - start_x - 4.0;
        let n = items.len().max(1) as f32;
        let bw = (avail / n).min(140.0);
        items
            .into_iter()
            .enumerate()
            .map(|(i, (id, title, foc, min))| {
                let x = start_x + i as f32 * bw;
                (id, title, Rect::new(x, ty + 3.0, bw - 3.0, TASKBAR_H - 6.0), foc, min)
            })
            .collect()
    }

    pub(super) fn draw_taskbar(&self, r: &mut Renderer, buttons: &[(u32, String, Rect, bool, bool)], time: f32) {
        let ty = SCREEN_H - TASKBAR_H;
        raised(r, 0.0, ty, SCREEN_W, TASKBAR_H);

        // 시작 버튼 (저울 아이콘, 문구 없음)
        let sb = self.start_button_rect();
        if self.start_open {
            sunken(r, sb.x, sb.y, sb.w, sb.h);
        } else {
            raised(r, sb.x, sb.y, sb.w, sb.h);
        }
        draw_scale(r, sb.x + sb.w / 2.0 - 9.0, sb.y + 2.0, 18.0, BLACK);

        // 창 버튼들
        for (_id, title, rect, foc, _min) in buttons {
            let (rect, foc) = (*rect, *foc);
            if foc {
                sunken(r, rect.x, rect.y, rect.w, rect.h);
            } else {
                raised(r, rect.x, rect.y, rect.w, rect.h);
            }
            r.text_clipped(rect.x + 5.0, rect.y + 2.0, title, 0.9, BLACK, rect.w - 8.0);
        }

        // 와이파이 표시 — 연결돼 있으면 평소대로, 끊겨 있으면 점+X만 깜빡이며 표시.
        let wr = self.wifi_rect();
        let wifi_color = if self.wifi_connected { BLACK } else { GRAY };
        let blink_visible: bool = (time * 2.0) as i32 % 2 == 0; // 0.5초 간격으로 깜빡
        draw_wifi(r, wr.x + 2.0, wr.y + wr.h * 0.5 - 7.0, 14.0, wifi_color, self.wifi_connected, blink_visible);

        // 시계 — 그냥 표시용(클릭해도 아무 일 없음).
        let ck = self.clock_rect();
        sunken(r, ck.x, ck.y, ck.w, ck.h);
        let unix = miniquad::date::now() as u64 + 9 * 3600; // KST
        let hh = (unix / 3600) % 24;
        let mm = (unix / 60) % 60;
        let clock = format!("{:02}:{:02}", hh, mm);
        // 세로 중앙 정렬 — 예전엔 ck.y 를 그대로 써서(위쪽 정렬) 글자가 박스 위아래로
        // 삐져나와 보였다. 다른 타임바 요소들(창 버튼 제목 등)과 같은 요령으로 맞춘다.
        let ty = ck.y + (ck.h - CELL_H * 0.9) / 2.0;
        // 왼쪽 여백을 5→9px 로 늘렸다 — sunken() 테두리 바로 옆에 숫자가 거의
        // 붙어 보인다는 피드백을 받았다(오른쪽 clip 폭도 그만큼 같이 줄여 균형을 맞춘다).
        r.text_clipped(ck.x + 13.0, ty, &clock, 0.9, BLACK, ck.w - 16.0);
    }

    pub(super) fn draw_start_menu(&self, r: &mut Renderer, m: (f32, f32), lang: Language) {
        let mr = self.start_menu_rect(r, lang);
        raised(r, mr.x, mr.y, mr.w, mr.h);
        r.rect(mr.x + 3.0, mr.y + 3.0, 22.0, mr.h - 6.0, NAVY);
        for (i, &key) in MENU_ITEMS.iter().enumerate() {
            let name = menu_item_label(lang, key);
            let iy = mr.y + 4.0 + i as f32 * 24.0;
            let row = Rect::new(mr.x + 28.0, iy, mr.w - 32.0, 22.0);
            let hover = row.contains(m.0, m.1);
            if hover {
                r.rect(row.x, row.y, row.w, row.h, NAVY);
                r.text(row.x + 4.0, iy + 2.0, name, 1.0, WHITE);
            } else {
                r.text(row.x + 4.0, iy + 2.0, name, 1.0, BLACK);
            }
        }
    }

    // 시작 메뉴와 바탕화면 우클릭 메뉴가 공유하는 항목(Official Site/Credits/Settings)
    // 처리. Shut Down 은 시작 메뉴에만 있어서 호출부에서 따로 처리한다.
    pub(super) fn run_menu_action(&mut self, idx: i32, settings: Rc<RefCell<Settings>>, work: Rect) {
        let lang = settings.borrow().language;
        match idx {
            0 => {
                // Official Site: 실제 사이트를 창 안에 띄운다. WebView2 로 백그라운드에서
                // 렌더링해 주기적으로 캡처한 이미지를 텍스처로 올리는 방식이라(OfficialSiteApp
                // 참고) CRT 셰이더도 다른 화면들과 똑같이 먹는다.
                let op = Opened {
                    // 캡처 해상도를 창 표시 크기보다 훨씬 크게 잡는다 — 뷰포트가
                    // 너무 좁으면(예: 460x320) 사이트가 모바일/확대된 레이아웃으로
                    // 렌더링돼서 답답해 보인다. 넓게 그린 다음 축소해서 보여주면
                    // 일반 데스크톱 화면처럼 자연스럽게 나온다.
                    // JPEG 캡처로 바꿔서 여유가 좀 생겼지만, 그래도 해상도가 너무 크면
                    // 캡처 한 장당 걸리는 시간이 늘어나 프레임레이트가 떨어진다 — 화질과
                    // 속도 사이에서 적당한 지점으로 살짝 낮췄다.
                    app: Box::new(OfficialSiteApp::new(OFFICIAL_SITE_URL, 1000, 750, settings.clone())),
                    title: menu_item_label(lang, "Official Site").to_string(),
                    size: (480.0, 380.0),
                    maximized: false,
                    resizable: false,
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                // 이미 열려있어서 새로 스폰된 게 아니면(그냥 앞으로 가져온 거면) 위치를
                // 또 밀면 안 된다 — 안 그러면 클릭할 때마다 창이 계속 아래로 내려간다.
                if self.wm.open(op, Some(OFFICIAL_SITE_WIN), work) {
                    self.wm.nudge_last(30.0); // 다른 창들보다 살짝 아래에서 스폰
                }
            }
            1 => {
                let op = Opened {
                    app: Box::new(CreditsApp(settings.clone())),
                    title: menu_item_label(lang, "Credits").to_string(),
                    // 개발자 이름이 늘어날 때마다(지금 4명) 목록 줄 수만큼 여유가
                    // 있어야 OK 버튼과 안 겹친다 — 이름 하나 늘 때 20px 정도 더 잡는다.
                    size: (260.0, 200.0),
                    maximized: false,
                    resizable: false,
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                self.wm.open(op, Some(CREDITS_WIN), work);
            }
            2 => {
                let op = Opened {
                    app: Box::new(SettingsApp::new(settings)),
                    title: menu_item_label(lang, "Settings").to_string(),
                    size: (360.0, 260.0), // 클리핑이 제대로 고쳐져서 넘치는 내용은 스크롤로 알아서 잘리니 더 줄여도 안전하다
                    maximized: false,
                    resizable: false,   // 설정창 크기 고정
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                self.wm.open(op, Some(SETTINGS_WIN), work); // 이미 열려있으면 새로 안 열고 앞으로
            }
            _ => {}
        }
    }

    // "New Mail" 토스트 자리 — update_toast() 와 커서 판정(update()) 양쪽에서
    // 같이 써서 자리가 어긋나지 않게 한다(wifi_popup_rect 와 같은 요령).
    pub(super) fn toast_rect() -> Rect {
        const W: f32 = 190.0;
        const H: f32 = 72.0;
        let ty = SCREEN_H - TASKBAR_H;
        let x = SCREEN_W - W - 8.0;
        let y = ty - H - 8.0;
        Rect::new(x, y, W, H)
    }

    // 시스템 메시지가 새로 생기면(지금은 메일 도착) 우측 하단(작업표시줄 바로 위,
    // 와이파이/시계 트레이 근처)에 잠깐 떴다가 TOAST_DURATION 뒤 저절로 사라지는
    // 알림. 누르면 바로 Mail 을 열고 닫힌다.
    pub(super) fn update_toast(&mut self, f: &mut Frame, work: Rect, lang: Language) {
        let Some(Toast { title, line1: from, line2: subject, opens_mail }) = self.toast.clone() else { return };
        let Rect { x, y, w, h } = Self::toast_rect();
        raised(f.r, x, y, w, h);
        f.r.rect(x + 2.0, y + 2.0, w - 4.0, 16.0, DARK_GRAY);
        f.r.text(x + 6.0, y + 3.0, t(lang, title), 0.8, WHITE);
        f.r.text_clipped(x + 6.0, y + 26.0, &from, 0.8, BLACK, w - 12.0);
        f.r.text_clipped(x + 6.0, y + 46.0, &subject, 0.8, GRAY, w - 12.0);

        if Rect::new(x, y, w, h).contains(f.input.mouse.0, f.input.mouse.1) && f.input.mouse_clicked {
            self.toast = None;
            self.toast_timer = 0.0;
            if opens_mail && let Some(mail_id) = self.fs.find_by_name("Mail") {
                let op = open(&self.fs, mail_id, &f.settings);
                self.wm.open(op, Some(mail_id), work);
            }
        }
    }

    // 시계를 누르면 그 위에 뜨는 날짜 팝업 자리 — 오른쪽 끝을 시계에 맞춘다.
    pub(super) fn clock_popup_rect(&self) -> Rect {
        let (w, h) = (150.0, 50.0);
        let ck = self.clock_rect();
        Rect::new((ck.x + ck.w - w).max(4.0), SCREEN_H - TASKBAR_H - h, w, h)
    }

    pub(super) fn draw_clock_popup(&self, r: &mut Renderer, lang: Language) {
        let pr = self.clock_popup_rect();
        raised(r, pr.x, pr.y, pr.w, pr.h);
        r.rect(pr.x, pr.y, pr.w, 20.0, DARK_GRAY);
        r.text(pr.x + 6.0, pr.y + 2.0, t(lang, s::DATE_TIME), 0.85, WHITE);
        // 진짜 컴퓨터의 시스템 시간(KST) — 시계와 같은 값. 서브 게임의 "시간이 1년은 필요하다"가 이걸 본다.
        let (y, mo, d) = civil_date(miniquad::date::now() as i64 + 9 * 3600);
        r.text(pr.x + 8.0, pr.y + 26.0, &format!("{y:04}-{mo:02}-{d:02}"), 0.95, BLACK);
    }

    // 와이파이 아이콘을 누르면 뜨는 연결 정보(상태/SSID/IP) 팝업.
    pub(super) fn draw_wifi_popup(&self, r: &mut Renderer, lang: Language) {
        let pr = self.wifi_popup_rect();
        raised(r, pr.x, pr.y, pr.w, pr.h);
        r.rect(pr.x, pr.y, pr.w, 20.0, DARK_GRAY);
        r.text(pr.x + 6.0, pr.y + 2.0, t(lang, s::NETWORK), 0.85, WHITE);

        let status_word = t(lang, s::STATUS);
        let state_word = if self.wifi_connected { t(lang, s::CONNECTED) } else { t(lang, s::DISCONNECTED) };
        let status = format!("{status_word}: {state_word}");
        r.text_clipped(pr.x + 8.0, pr.y + 26.0, &status, 0.8, if self.wifi_connected { BLACK } else { GRAY }, pr.w - 16.0);

        let unknown = t(lang, s::UNKNOWN);
        let info = self.wifi_info.as_ref();
        let ssid_line = match info.and_then(|i| i.ssid.as_deref()) {
            Some(ssid) => format!("SSID: {ssid}"),
            None => format!("SSID: {unknown}"),
        };
        r.text_clipped(pr.x + 8.0, pr.y + 42.0, &ssid_line, 0.8, GRAY, pr.w - 16.0);

        let ip_line = match info.and_then(|i| i.ip.as_deref()) {
            Some(ip) => format!("IP: {ip}"),
            None => format!("IP: {unknown}"),
        };
        r.text_clipped(pr.x + 8.0, pr.y + 58.0, &ip_line, 0.8, GRAY, pr.w - 16.0);
    }

    // 아이콘 왼쪽 세로 띠가 없다는 점 빼면 시작 메뉴랑 같은 모양 — 컨텍스트 메뉴답게
    // 더 단순하게.
    pub(super) fn draw_context_menu(&self, r: &mut Renderer, pos: (f32, f32), m: (f32, f32), lang: Language) {
        let mr = Self::context_menu_rect(r, lang, pos);
        raised(r, mr.x, mr.y, mr.w, mr.h);
        for (i, &key) in CONTEXT_MENU_ITEMS.iter().enumerate() {
            let name = menu_item_label(lang, key);
            let iy = mr.y + 3.0 + i as f32 * CTX_ROW_H;
            let row = Rect::new(mr.x + 3.0, iy, mr.w - 6.0, CTX_ROW_H);
            let hover = row.contains(m.0, m.1);
            // 텍스트 셀 높이(CELL_H*스케일)를 줄 안에서 세로로 가운데 맞춰야 구분선이
            // 위/아래 글자와 같은 간격으로 보인다.
            let ty = row.y + (row.h - CELL_H * CTX_TEXT_SCALE) / 2.0;
            if hover {
                r.rect(row.x, row.y, row.w, row.h, NAVY);
                r.text(row.x + 5.0, ty, name, CTX_TEXT_SCALE, WHITE);
            } else {
                r.text(row.x + 5.0, ty, name, CTX_TEXT_SCALE, BLACK);
            }
            // 마지막 항목 빼고 아래에 구분선을 그어 버튼들을 나눈다.
            if i + 1 < CONTEXT_MENU_ITEMS.len() {
                r.rect(mr.x + 2.0, row.y + row.h, mr.w - 4.0, 1.0, GRAY);
            }
        }
    }
}
