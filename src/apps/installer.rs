//! HexTool Setup.exe — 압축 해제 프로그램을 설치하는 척하는 설치 마법사. 창
//! 관리자가 이미 진짜 타이틀바("HexTool Setup.exe" + 최소/최대/닫기 버튼)를 그려
//! 주므로, 안에서 또 다른 타이틀바 딸린 "대화상자"를 얹으면 창 속에 창이 있는
//! 것처럼 보인다 — 그래서 내용이 창 전체(area)를 그대로 채운다: 맨 위에 페이지
//! 부제 한 줄 + 밑줄, 그 아래 왼쪽 삽화 + 오른쪽 안내문, 맨 아래 버튼.
//! 이미 설치돼 있으면(FileSystem::hex_tool_installed) Welcome/License/Installing
//! 을 건너뛰고 바로 AlreadyInstalled 페이지로 연다. 아니면 Welcome → License(약관
//! 박스는 스크롤 가능, 동의 체크해야 Next 가 풀림) → Installing(들쭉날쭉한 진행바)
//! → Finish 네 페이지를 넘어간다. Finish 에서 끝내면 AppAction::InstallComplete 를
//! 돌려주는데, desktop.rs 가 받아서 FileSystem::hex_tool_installed 를 true 로
//! 바꾸고 바탕화면에 HexTool 아이콘을 만든다(AlreadyInstalled 에서는 이미 설치돼
//! 있으므로 그냥 AppAction::Close 만 돌려줘서 아이콘이 중복으로 안 생긴다).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{Language, Settings};
use crate::gfx::{Assets, Color, Rect, Renderer};
use crate::strings::{common, installer as s, t};
use crate::ui::*;

use super::widgets::{ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

const INSTALL_DURATION: f32 = 2.6; // 진행바가 다 차는 데 걸리는 시간(초) — 가짜 설치라 적당히 짧게
const FINISH_HOLD: f32 = 0.6; // 진행바가 100% 를 찍은 뒤 "Done." 을 잠깐 보여주는 시간(초)
const BTN_W: f32 = 74.0;
const BTN_H: f32 = 24.0;
const LICENSE_LINE_H: f32 = 16.0;

// 아주 단순한 xorshift64 의사난수 — boot.rs 의 Rng 와 같은 용도(로딩 바를 들쭉날쭉
// 하게 만드는 waypoint 생성)지만, 씬이 아니라 앱이라 굳이 공유 모듈로 안 뽑고
// 여기 작게 둔다.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (self.next_u32() % 1_000_000) as f32 / 1_000_000.0 * (max - min)
    }
}

// 로딩 바가 일정한 속도로 차지 않고 멈칫거리다 훅 튀도록, (경과비율, 진행비율)
// 웨이포인트를 랜덤 생성한다. boot.rs 의 build_load_waypoints/sample_load 와 같은
// 방식 — (0,0) 에서 시작해 (1,1) 로 끝나며 사이 구간마다 속도가 들쭉날쭉하다.
fn build_load_waypoints(rng: &mut Rng) -> Vec<(f32, f32)> {
    const SEGMENTS: usize = 7;
    let mut xd: Vec<f32> = (0..SEGMENTS).map(|_| rng.range_f32(0.4, 1.6)).collect();
    let xsum: f32 = xd.iter().sum();
    for v in xd.iter_mut() {
        *v /= xsum;
    }
    // 진행량은 제곱을 줘서 절반은 거의 멈춘 듯 조금씩, 절반은 훅 튀도록 편차를 크게 만든다.
    let mut yd: Vec<f32> = (0..SEGMENTS).map(|_| rng.range_f32(0.05, 1.0).powf(2.0)).collect();
    let ysum: f32 = yd.iter().sum();
    for v in yd.iter_mut() {
        *v /= ysum;
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut out = vec![(0.0, 0.0)];
    for i in 0..SEGMENTS {
        x += xd[i];
        y += yd[i];
        out.push((x, y));
    }
    // xd/yd 는 합이 1이 되도록 나눴지만 부동소수점 오차 때문에 누적합이 (1.0, 1.0)
    // 에 딱 안 맞고 0.999998 처럼 살짝 모자랄 수 있다 — 그러면 Page::Installing 의
    // `done = self.progress >= 1.0` 판정이 영영 안 켜져서 진행바가 99% 근처에서
    // 멈춰버린다. 마지막 점을 강제로 정확히 (1.0, 1.0) 으로 맞춰 이 문제를 없앤다.
    if let Some(last) = out.last_mut() {
        *last = (1.0, 1.0);
    }
    out
}

// waypoints 사이를 선형보간해 경과비율 t(0..1) 에서의 진행비율을 구한다.
fn sample_load(waypoints: &[(f32, f32)], t: f32) -> f32 {
    for w in waypoints.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if t <= x1 {
            let seg_t = if x1 > x0 { ((t - x0) / (x1 - x0)).clamp(0.0, 1.0) } else { 1.0 };
            return y0 + (y1 - y0) * seg_t;
        }
    }
    1.0
}

// 설치 중 단계를 흉내내는 가짜 상태 문구 — 진행률에 맞춰 순서대로 바뀐다.
fn status_steps(lang: Language) -> [&'static str; 5] {
    [
        t(lang, s::STEP_COPYING),
        t(lang, s::STEP_REGISTERING),
        t(lang, s::STEP_UPDATING),
        t(lang, s::STEP_VERIFYING),
        t(lang, s::STEP_FINALIZING),
    ]
}

// 라이선스 약관 본문 — 다른 UI 문구는 다 언어별 문구(지금은 strings.rs::t())로
// 세 언어를 지원하는데 이 긴 약관 텍스트만 영어 고정이었다("발신되는 메일"
// 번역 요청 때와 같은 종류의 누락). 제목("HEXTOOL LICENSE AGREEMENT")도 이
// 문단 안에 같이 들어있어서 통째로 옮겼다.
fn license_text(lang: Language) -> &'static str {
    t(lang, s::LICENSE_TEXT)
}

enum Page {
    AlreadyInstalled,
    Welcome,
    License,
    Installing,
    Finish,
}

pub struct InstallerApp {
    page: Page,
    license_accepted: bool,
    license_scroll: f32,
    license_scroll_disp: f32,
    license_sb_drag: bool,
    elapsed: f32,      // Installing 페이지에 들어온 뒤 지난 시간(초) — waypoints 를 샘플링하는 기준
    progress: f32,     // 0.0..1.0, elapsed 로부터 매 프레임 다시 계산됨(들쭉날쭉하게 차오름)
    finish_hold: f32,  // progress 가 1.0 을 찍은 뒤 Finish 로 넘어가기 전 "Done." 을 보여주는 남은 시간
    sent_complete: bool, // 진행바가 다 찼을 때 AppAction::InstallComplete 를 딱 한 번만 보내려는 플래그
    load_waypoints: Vec<(f32, f32)>, // 부팅 화면 로딩 바와 같은 요령의 들쭉날쭉한 진행 곡선
    settings: Rc<RefCell<Settings>>,
}

impl InstallerApp {
    // already_installed 면(FileSystem::hex_tool_installed 가 이미 true) Welcome
    // 부터 다시 태우지 않고 바로 AlreadyInstalled 페이지로 연다.
    pub(super) fn new(settings: Rc<RefCell<Settings>>, already_installed: bool) -> InstallerApp {
        let mut rng = Rng::new((miniquad::date::now() * 1e6) as u64);
        InstallerApp {
            page: if already_installed { Page::AlreadyInstalled } else { Page::Welcome },
            license_accepted: false,
            license_scroll: 0.0,
            license_scroll_disp: 0.0,
            license_sb_drag: false,
            elapsed: 0.0,
            progress: 0.0,
            finish_hold: 0.0,
            sent_complete: false,
            load_waypoints: build_load_waypoints(&mut rng),
            settings,
        }
    }

    // 카드 오른쪽 문단 — 폭에 맞춰 자동 줄바꿈(wrap_lines)해서 그린다. 문장을 손으로
    // 미리 잘라두면 카드 폭이 좁아질 때 그 줄 자체가 넘쳐서 글자가 잘려 보이니,
    // 실제 폭 기준으로 다시 감싼다. 줄바꿈 문자(\n\n 등)가 있으면 그 자리에서 단락도 나눈다.
    // 다 그린 뒤 마지막 줄 바로 아래 y 좌표를 돌려줘서, 호출부가 그 아래에 다음
    // 요소를 겹치지 않게 이어 배치할 수 있게 한다.
    fn draw_paragraph(&self, r: &mut Renderer, x: f32, y: f32, w: f32, text: &str, color: Color) -> f32 {
        // 실제 렌더링 폭(0.8 스케일)을 기준으로 줄바꿈한다 — 예전엔 ADVANCE(라틴
        // 문자 기준 고정폭)로 어림잡은 글자 수로 끊었는데, 한글/한자/가나처럼 라틴
        // 문자보다 훨씬 넓은 글자가 섞이면 한 줄에 너무 많이 욱여넣어 카드 폭 밖으로
        // 넘쳐 잘려 보였다 — crate::ui::wrap_lines() 가 실제 글자 폭을 재서 언어와
        // 무관하게 정확히 접어준다.
        let mut ty = y;
        for para in text.split('\n') {
            if para.is_empty() {
                ty += 10.0;
                continue;
            }
            for line in crate::ui::wrap_lines(r, para, 0.8, w) {
                r.text(x, ty, &line, 0.8, color);
                ty += 17.0;
            }
        }
        ty
    }

    // LICENSE_TEXT 를 폭에 맞게 줄 단위로 미리 감싸둔다(단락 사이 빈 줄도 그대로
    // 한 줄로 포함) — 스크롤 총 높이를 계산하려면 실제로 몇 줄이 되는지 먼저 알아야
    // 해서, draw_paragraph 처럼 그리면서 바로 계산하는 대신 별도로 뽑아둔다.
    fn license_lines(&self, r: &Renderer, w: f32, lang: Language) -> Vec<String> {
        let mut lines = Vec::new();
        for para in license_text(lang).split('\n') {
            if para.is_empty() {
                lines.push(String::new());
                continue;
            }
            lines.extend(crate::ui::wrap_lines(r, para, 0.8, w));
        }
        lines
    }

    // 약관 sunken 박스 — 내용이 넘치면(대부분 넘친다) 마우스 휠로 스크롤되고, 다
    // 안 보일 땐 오른쪽에 스크롤바도 뜬다. 박스 위에 있을 때만 휠을 먹는다(다른
    // 스크롤 영역과 안 섞이게 하는 관례).
    fn draw_license_box(&mut self, r: &mut Renderer, win: &WinInput, box_rect: Rect) {
        let lang = self.settings.borrow().language;
        sunken(r, box_rect.x, box_rect.y, box_rect.w, box_rect.h);
        let has_sb = {
            let lines = self.license_lines(r, box_rect.w - 12.0 - 10.0, lang);
            let total_h = lines.len() as f32 * LICENSE_LINE_H;
            total_h > box_rect.h - 6.0
        };
        let text_w = box_rect.w - 12.0 - if has_sb { 10.0 } else { 0.0 };
        let lines = self.license_lines(r, text_w, lang);
        let total_h = lines.len() as f32 * LICENSE_LINE_H;
        let max_scroll = (total_h - (box_rect.h - 6.0)).max(0.0);

        if box_rect.contains(win.mouse.0, win.mouse.1) {
            self.license_scroll -= win.wheel / 120.0 * (LICENSE_LINE_H * 2.0);
        }
        self.license_scroll = self.license_scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.license_scroll_disp, self.license_scroll, win.dt, smooth);

        let text_area = Rect::new(box_rect.x + 6.0, box_rect.y + 3.0, text_w, box_rect.h - 6.0);
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - self.license_scroll_disp;
        for line in &lines {
            if ty > text_area.y - LICENSE_LINE_H && ty < text_area.y + text_area.h && !line.is_empty() {
                r.text(text_area.x, ty, line, 0.8, BLACK);
            }
            ty += LICENSE_LINE_H;
        }
        r.set_clip(None);

        if max_scroll > 0.0 {
            let sb_x = box_rect.x + box_rect.w - 9.0;
            let frac = (box_rect.h / total_h).min(1.0);
            scrollbar(
                r, win, sb_x, box_rect.y + 2.0, 7.0, box_rect.h - 4.0, frac,
                self.license_scroll_disp, &mut self.license_scroll, max_scroll, &mut self.license_sb_drag,
            );
        }
    }

    // 카드 하단 Back/Next-or-Finish/Cancel 버튼 — 페이지마다 자리는 똑같고 문구/
    // 활성 여부만 다르다. Back 은 첫 페이지(Welcome)에선 비활성, next_enabled 가
    // false 면 Next 도 눌러도 반응 없이 회색으로만 그린다(License 동의 전 등).
    #[allow(clippy::too_many_arguments)]
    fn draw_nav_row(
        &mut self, r: &mut Renderer, card: Rect, win: &WinInput, next_label: &str, next_enabled: bool, show_back: bool, show_cancel: bool,
        lang: Language,
    ) -> NavClick {
        let by = card.y + card.h - BTN_H - 12.0;
        let next_btn = Rect::new(card.x + card.w - 12.0 - BTN_W, by, BTN_W, BTN_H);
        let cancel_btn = Rect::new(next_btn.x - 8.0 - BTN_W, by, BTN_W, BTN_H);
        let back_btn = Rect::new(cancel_btn.x - 8.0 - BTN_W, by, BTN_W, BTN_H);

        let mut clicked = NavClick::None;

        if show_back {
            raised(r, back_btn.x, back_btn.y, back_btn.w, back_btn.h);
            let back_label = t(lang, s::BACK);
            let tw = r.text_width(back_label, 1.0);
            r.text(back_btn.x + (back_btn.w - tw) / 2.0, back_btn.y + 1.0, back_label, 1.0, BLACK);
            if back_btn.contains(win.mouse.0, win.mouse.1) && win.mouse_clicked {
                clicked = NavClick::Back;
            }
        }
        if show_cancel && button(r, cancel_btn.x, cancel_btn.y, cancel_btn.w, cancel_btn.h, t(lang, common::CANCEL), win) {
            clicked = NavClick::Cancel;
        }
        if next_enabled {
            if button(r, next_btn.x, next_btn.y, next_btn.w, next_btn.h, next_label, win) {
                clicked = NavClick::Next;
            }
        } else {
            raised(r, next_btn.x, next_btn.y, next_btn.w, next_btn.h);
            let tw = r.text_width(next_label, 1.0);
            r.text(next_btn.x + (next_btn.w - tw) / 2.0, next_btn.y + 1.0, next_label, 1.0, [0.55, 0.55, 0.55, 1.0]);
        }
        clicked
    }
}

enum NavClick {
    None,
    Back,
    Next,
    Cancel,
}

impl App for InstallerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        // 창 전체를 그대로 카드로 쓴다 — 진짜 창 타이틀바가 이미 있으니 그 안에
        // 또 배경/제목/카드 여러 겹을 두르지 않는다.
        let card = area;
        r.rect(card.x, card.y, card.w, card.h, FACE);
        let lang = self.settings.borrow().language;

        let page_title = match self.page {
            Page::AlreadyInstalled => t(lang, s::PAGE_ALREADY_INSTALLED),
            Page::Welcome => t(lang, s::PAGE_WELCOME),
            Page::License => t(lang, s::PAGE_LICENSE),
            Page::Installing => t(lang, s::PAGE_INSTALLING),
            Page::Finish => t(lang, s::PAGE_FINISH),
        };
        r.text(card.x + 12.0, card.y + 10.0, page_title, 0.95, BLACK);
        r.rect(card.x + 10.0, card.y + 32.0, card.w - 20.0, 1.0, GRAY); // 부제 밑 구분선

        // 왼쪽 삽화 칸 — 이미 있는 Installer 아이콘(모니터+화살표)을 크게 그려서 대신한다.
        let illus = Rect::new(card.x + 12.0, card.y + 42.0, 84.0, card.h - 42.0 - 12.0);
        r.rect(illus.x, illus.y, illus.w, illus.h, WHITE);
        border(r, illus.x, illus.y, illus.w, illus.h, GRAY);
        draw_installer_icon(r, illus.x + illus.w / 2.0 - 26.0, illus.y + illus.h / 2.0 - 26.0, 52.0);

        let content = Rect::new(illus.x + illus.w + 14.0, card.y + 46.0, card.w - illus.w - 14.0 - 24.0, card.h - 46.0 - 12.0);

        // Installing 페이지에서 진행바가 막 100% 를 찍은 그 프레임에 딱 한 번
        // AppAction::InstallComplete 를 보내려고 쓴다(바탕화면 아이콘이 Finish
        // 버튼을 누르는 시점이 아니라 "설치가 실제로 끝난 순간" 에 생기게).
        let mut result = AppAction::None;

        match self.page {
            Page::AlreadyInstalled => {
                let msg1 = t(lang, s::ALREADY_INSTALLED_MSG);
                let y = self.draw_paragraph(r, content.x, content.y, content.w, msg1, BLACK);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_FINISH_TO_CLOSE), GRAY);

                // InstallComplete 를 또 보내면 데스크톱 아이콘이 하나 더 생기니(이미
                // 설치돼 있는 상태라 새로 만들 게 없다), 그냥 창만 닫는다.
                if let NavClick::Next = self.draw_nav_row(r, card, win, t(lang, s::FINISH), true, false, false, lang) {
                    return AppAction::Close;
                }
            }
            Page::Welcome => {
                let msg1 = t(lang, s::WELCOME_MSG);
                let y = self.draw_paragraph(r, content.x, content.y, content.w, msg1, BLACK);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_NEXT_OR_CANCEL), GRAY);

                match self.draw_nav_row(r, card, win, t(lang, s::NEXT), true, false, true, lang) {
                    NavClick::Next => self.page = Page::License,
                    NavClick::Cancel => return AppAction::Close,
                    NavClick::Back | NavClick::None => {}
                }
            }
            Page::License => {
                // 아래쪽에서 위로: 버튼 줄(BTN_H+12 여백) → 그 위 체크박스 한 줄
                // (~20px) → 남는 공간을 전부 약관 박스로.
                let nav_top = card.y + card.h - BTN_H - 12.0;
                let cb_y = nav_top - 12.0 - 20.0;
                let box_h = (cb_y - 10.0 - content.y).max(20.0);

                self.draw_license_box(r, win, Rect::new(content.x, content.y, content.w, box_h));

                checkbox(r, content.x, cb_y, t(lang, s::ACCEPT_TERMS), &mut self.license_accepted, win);

                match self.draw_nav_row(r, card, win, t(lang, s::NEXT), self.license_accepted, true, true, lang) {
                    NavClick::Next => {
                        self.page = Page::Installing;
                        self.elapsed = 0.0;
                        self.progress = 0.0;
                        self.sent_complete = false;
                    }
                    NavClick::Back => self.page = Page::Welcome,
                    NavClick::Cancel => return AppAction::Close,
                    NavClick::None => {}
                }
            }
            Page::Installing => {
                let done = self.progress >= 1.0;
                let steps = status_steps(lang);
                let status = if done { t(lang, s::DONE) } else { steps[((self.progress * steps.len() as f32) as usize).min(steps.len() - 1)] };
                let msg1 = t(lang, s::INSTALLING_MSG);
                let y = self.draw_paragraph(r, content.x, content.y, content.w, msg1, BLACK);
                let status_y = y + 10.0;
                r.text_clipped(content.x, status_y, status, 0.8, GRAY, content.w);

                if !done {
                    self.elapsed += win.dt;
                    let t_frac = (self.elapsed / INSTALL_DURATION).min(1.0);
                    self.progress = sample_load(&self.load_waypoints, t_frac);
                } else {
                    if !self.sent_complete {
                        // 게이지가 다 찬 바로 이 순간 데스크톱에 아이콘을 만들어달라고
                        // 요청한다 — Finish 버튼을 누르기 전이라도 창은 계속 열려있는다
                        // (window_manager.rs 가 이 액션에 대해선 창을 안 닫는다).
                        self.sent_complete = true;
                        result = AppAction::InstallComplete;
                    }
                    self.finish_hold += win.dt;
                    if self.finish_hold >= FINISH_HOLD {
                        self.page = Page::Finish;
                    }
                }

                let bar_y = status_y + 28.0;
                let bar_w = content.w;
                const BAR_H: f32 = 18.0;
                sunken(r, content.x, bar_y, bar_w, BAR_H);
                let fill_w = (bar_w - 4.0) * self.progress;
                if fill_w > 0.0 {
                    r.rect(content.x + 2.0, bar_y + 2.0, fill_w, BAR_H - 4.0, NAVY);
                    // 10% 단위 눈금 — 고전 설치 마법사 진행바 특유의 분절된 느낌.
                    for tick in 1..10 {
                        let tx = content.x + 2.0 + bar_w * tick as f32 / 10.0;
                        if tx < content.x + 2.0 + fill_w {
                            r.rect(tx, bar_y + 2.0, 1.0, BAR_H - 4.0, [0.0, 0.0, 0.3, 0.35]);
                        }
                    }
                }
                let pct = if done { "100%".to_string() } else { format!("{}%", (self.progress * 100.0) as i32) };
                r.text(content.x, bar_y + BAR_H + 6.0, &pct, 0.8, GRAY);
                // 설치 진행 중엔 버튼 없음 — 취소도 못하게 막는다(설치 마법사 특유의 긴장감).
            }
            Page::Finish => {
                let msg1 = t(lang, s::FINISH_MSG);
                let y = self.draw_paragraph(r, content.x, content.y, content.w, msg1, BLACK);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_FINISH_TO_CLOSE), GRAY);

                // 바탕화면 아이콘은 이미 진행바가 다 찼을 때 만들어졌으니, 여기서는
                // 그냥 창만 닫는다(InstallComplete 를 또 보내면 아이콘이 중복 생긴다).
                if let NavClick::Next = self.draw_nav_row(r, card, win, t(lang, s::FINISH), true, false, false, lang) {
                    return AppAction::Close;
                }
            }
        }

        result
    }
}
