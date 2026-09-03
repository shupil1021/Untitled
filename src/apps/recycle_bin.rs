//! 휴지통 — 실제 Windows 98 휴지통(주소창 + 왼쪽 안내 패널 + 오른쪽 파일 목록 +
//! 상태바)의 느낌을 재현한다. My Computer 의 일반 폴더 뷰(탭/트리)는 그대로
//! 재사용하지 않고 별도 프로그램으로 둔다 — 안의 항목은 여느 폴더처럼
//! `FileKind::Folder`(이름만 "Recycle Bin")라서 드래그로 넣고 더블클릭으로 여는
//! 동작은 다른 폴더와 똑같이 동작하고, 이 앱은 그 위에 클래식 크롬 + 영구 삭제
//! (왼쪽 패널의 "Empty Recycle Bin" 링크)만 더한다.
//!
//! File/Edit/View/Go/Favorites/Help 메뉴바와 Back/Forward/Up/Delete 툴바는 한 번
//! 만들었다가 뺐다 — 눌러도 아무 일도 안 하는(또는 항상 회색 비활성인) 장식용
//! 줄이라 그냥 "안 눌리는 버튼/메뉴 무더기"로만 보여 오히려 덜 정돈돼 보였다
//! (mail.rs 의 툴바를 뺀 것과 같은 이유).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{display_name, FileId, Language, Settings, MY_COMPUTER_NAME, RECYCLE_BIN_NAME};
use crate::gfx::{Assets, Color, Rect, Renderer, CELL_H};
use crate::strings::{common, recycle_bin as s, t};
use crate::ui::*;

use super::widgets::{ease_scroll, icon_grid, scrollbar};
use super::{App, AppAction, WinInput};

type Items = Vec<(FileId, String, IconType)>;

const ADDR_H: f32 = 24.0;
const STATUS_H: f32 = 22.0;
const SEL_ROW_H: f32 = 26.0;
const LEFT_W: f32 = 140.0;
const ICON_S: f32 = 48.0;
const LEFT_TEXT_SCALE: f32 = 0.75;
const SEL_BTN_W: f32 = 66.0;
// ui::button()/raw_button() 은 라벨을 항상 scale 1.0 으로 그려서 "Restore"(7글자,
// 77px)가 이 버튼 폭 안에 안 들어가고 테두리 밖으로 삐져나왔다 — 그래서 이 버튼만
// 공용 button() 을 안 쓰고 직접 그리며 scale 을 낮췄다(상태바/주소창과 같은 0.8).
const SEL_BTN_SCALE: f32 = 0.8;

// 선택된 항목이 있을 때만 실제로 동작하는 버튼 — 없으면 회색으로 눌리지 않는
// 상태로만 그린다(이전에 뺐던 Back/Forward/Up 처럼 "항상 죽어있는 장식"이 아니라
// 선택 여부에 따라 실제로 켜지고 꺼지는, 진짜 동작하는 컨트롤이라 다르다).
fn sel_button(r: &mut Renderer, btn: Rect, label: &str, enabled: bool, win: &WinInput) -> bool {
    let hover = enabled && btn.contains(win.mouse.0, win.mouse.1);
    let pressed = hover && win.mouse_down;
    if pressed {
        sunken(r, btn.x, btn.y, btn.w, btn.h);
    } else {
        raised(r, btn.x, btn.y, btn.w, btn.h);
    }
    // ui.rs::raw_button() 과 같은 이유 — 번역 문구가 SEL_BTN_W 보다 넓어지면
    // (예: "Restore" → 일본어 "元に戻す") 가운데 정렬 텍스트가 버튼 밖으로 새어
    // 옆 항목과 겹쳐 보이므로, 안 들어갈 땐 왼쪽으로 당기고 잘라서 최소한 버튼
    // 안에만 그리게 한다.
    let tw = r.text_width(label, SEL_BTN_SCALE);
    let pad = 3.0;
    let tx = (btn.x + (btn.w - tw) / 2.0).max(btn.x + pad);
    let avail = (btn.x + btn.w - pad - tx).max(0.0);
    let ty = btn.y + (btn.h - CELL_H * SEL_BTN_SCALE) / 2.0;
    let off = if pressed { 1.0 } else { 0.0 };
    let color = if enabled { BLACK } else { GRAY };
    r.text_clipped(tx + off, ty + off, label, SEL_BTN_SCALE, color, avail);
    hover && win.mouse_clicked
}

pub struct RecycleBinApp {
    items: Items,
    selected: Vec<usize>,
    marquee_start: Option<(f32, f32)>,
    prev_down: bool,
    grid_scroll: f32,
    grid_scroll_disp: f32,
    grid_sb_drag: bool,
    // 왼쪽 안내 패널 전용 스크롤 — 창이 좁아지면 아이콘/제목/설명 문단이 패널
    // 높이를 넘칠 수 있어서(특히 창을 세로로 줄였을 때) 따로 스크롤 가능하게 한다.
    left_scroll: f32,
    left_scroll_disp: f32,
    left_sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl RecycleBinApp {
    pub(super) fn new(items: Items, settings: Rc<RefCell<Settings>>) -> RecycleBinApp {
        RecycleBinApp {
            items,
            selected: Vec::new(),
            marquee_start: None,
            prev_down: false,
            grid_scroll: 0.0,
            grid_scroll_disp: 0.0,
            grid_sb_drag: false,
            left_scroll: 0.0,
            left_scroll_disp: 0.0,
            left_sb_drag: false,
            settings,
        }
    }

    // 주소창 — explorer.rs 의 draw_address_bar 와 같은 요령(읽기전용, 그냥 라벨).
    fn draw_address_bar(&self, r: &mut Renderer, assets: &Assets, area: Rect, lang: Language) {
        r.rect(area.x, area.y, area.w, ADDR_H, FACE);
        let label = t(lang, common::ADDRESS);
        r.text(area.x + 4.0, area.y + (ADDR_H - CELL_H * 0.8) / 2.0, label, 0.8, BLACK);
        let lw = r.text_width(label, 0.8) + 8.0;
        let field = Rect::new(area.x + lw, area.y + 2.0, area.w - lw - 4.0, ADDR_H - 6.0);
        sunken(r, field.x, field.y, field.w, field.h);
        let icon = if self.items.is_empty() { IconType::RecycleEmpty } else { IconType::RecycleFull };
        draw_icon(r, assets, &icon, field.x + 4.0, field.y + (field.h - 14.0) / 2.0, 14.0);
        r.text(field.x + 22.0, field.y + (field.h - CELL_H * 0.8) / 2.0, &display_name(lang, RECYCLE_BIN_NAME), 0.8, BLACK);
        r.rect(area.x, area.y + ADDR_H - 1.0, area.w, 1.0, GRAY);
    }

    // 왼쪽 안내 패널 — 큰 휴지통 아이콘 + 제목 + 색줄 + 설명 + "Empty Recycle Bin" 링크.
    // 내용이 패널 높이를 넘치면(창이 작을 때) 세로로 스크롤한다 — 그 전엔 좁은
    // 창에서 아래쪽 글자가 그냥 상태바 밑으로 잘려나가 안 보였다.
    fn draw_left_panel(&mut self, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput, lang: Language) -> bool {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        // 스크롤바가 나타날 수도 있으니 오른쪽에 8px 여유를 미리 빼고 줄바꿈 폭을 계산한다.
        // 예전엔 "M" 한 글자의 폭으로 글자당 평균 폭을 어림잡아 글자 수로 끊었는데,
        // 라틴 문자 기준값이라 그보다 훨씬 넓은 한글/한자/가나 글자를 쓰면 한 줄에
        // 너무 많이 욱여넣어 패널 폭 밖으로 넘쳐 잘려 보였다(스크린샷으로 제보받은
        // 버그) — wrap_lines() 가 실제 렌더링 폭을 재서 언어와 무관하게 정확히 접는다.
        let text_x = area.x + 10.0;
        let text_w = (area.w - 20.0 - 8.0).max(10.0);
        let line_h = CELL_H * LEFT_TEXT_SCALE + 2.0;
        let desc_lines = wrap_lines(r, t(lang, s::DESC), LEFT_TEXT_SCALE, text_w);
        // 참고 UI(Windows 98 휴지통)는 "Empty Recycle Bin" 이라는 문구만 문단 안에서
        // 파란 링크로 강조되지만, 언어마다 어순이 달라 특정 부분 문자열만 골라
        // 하이라이트하기 어렵다 — 그래서 링크 문단 전체(link_lines)를 하나의 큰
        // 링크로 취급한다(이미 문단 전체가 링크색으로 그려져 있어 위화감이 없다).
        let link_lines = wrap_lines(r, t(lang, s::EMPTY_LINK), LEFT_TEXT_SCALE, text_w);

        // 실제로 그리기 전에 콘텐츠 전체 높이(content space, y=0 이 패널 맨 위)를
        // 먼저 계산해서 스크롤 한계를 구한다.
        let icon_y_local = 12.0;
        let title_y_local = icon_y_local + ICON_S + 6.0;
        let bar_y_local = title_y_local + CELL_H + 2.0;
        let desc_y_local = bar_y_local + 12.0;
        let link_y_local = desc_y_local + desc_lines.len() as f32 * line_h + 6.0;
        let content_h = link_y_local + link_lines.len() as f32 * line_h + 8.0;

        let max_scroll = (content_h - area.h).max(0.0);
        if area.contains(win.mouse.0, win.mouse.1) {
            self.left_scroll -= win.wheel / 120.0 * (line_h * 3.0);
        }
        self.left_scroll = self.left_scroll.clamp(0.0, max_scroll);
        ease_scroll(&mut self.left_scroll_disp, self.left_scroll, win.dt, true);
        let scroll_px = self.left_scroll_disp;
        let ty0 = area.y - scroll_px;

        let outer_clip = r.clip();
        r.set_clip(Some(area));

        let cx = area.x + area.w / 2.0;
        let icon = if self.items.is_empty() { IconType::RecycleEmpty } else { IconType::RecycleFull };
        draw_icon(r, assets, &icon, cx - ICON_S / 2.0, ty0 + icon_y_local, ICON_S);

        let title = display_name(lang, RECYCLE_BIN_NAME);
        let tw = r.text_width(&title, 1.0);
        r.text(cx - tw / 2.0, ty0 + title_y_local, &title, 1.0, BLACK);

        // 실제 화면의 빨강/초록/파랑 3색 줄을 얇은 색 막대 셋으로 대신한다.
        let seg_w = (area.w - 16.0) / 3.0;
        let bar_colors: [Color; 3] = [[0.8, 0.2, 0.2, 1.0], [0.2, 0.6, 0.2, 1.0], [0.2, 0.3, 0.8, 1.0]];
        for (i, col) in bar_colors.into_iter().enumerate() {
            r.rect(area.x + 8.0 + i as f32 * seg_w, ty0 + bar_y_local, seg_w - 1.0, 3.0, col);
        }

        for (i, line) in desc_lines.iter().enumerate() {
            r.text(text_x, ty0 + desc_y_local + i as f32 * line_h, line, LEFT_TEXT_SCALE, BLACK);
        }

        let can_empty = !self.items.is_empty();
        let link_color: Color = if can_empty { [0.0, 0.0, 0.85, 1.0] } else { GRAY };
        let mut clicked = false;
        for (i, line) in link_lines.iter().enumerate() {
            let ty = ty0 + link_y_local + i as f32 * line_h;
            let lw = r.text_width(line, LEFT_TEXT_SCALE);
            r.text(text_x, ty, line, LEFT_TEXT_SCALE, link_color);
            if can_empty {
                let hit = Rect::new(text_x, ty, lw, CELL_H * LEFT_TEXT_SCALE);
                if hit.contains(win.mouse.0, win.mouse.1) {
                    r.rect(text_x, ty + CELL_H * LEFT_TEXT_SCALE, lw, 1.0, link_color); // 밑줄(hover)
                    if win.mouse_clicked {
                        clicked = true;
                    }
                }
            }
        }
        r.set_clip(outer_clip);

        if max_scroll > 0.0 {
            let sb_x = area.x + area.w - 9.0;
            let frac = (area.h / content_h).min(1.0);
            scrollbar(r, win, sb_x, area.y, 8.0, area.h, frac, scroll_px, &mut self.left_scroll, max_scroll, &mut self.left_sb_drag);
        }
        r.rect(area.x + area.w - 1.0, area.y, 1.0, area.h, GRAY);
        clicked
    }

    // 오른쪽 파일 격자 아래(상태바 바로 위)에 얹는 선택 항목 액션 줄 — "Restore"
    // (선택 항목을 바탕화면으로 되돌림, 실제 Windows 처럼 원래 있던 자리까지는
    // 기억 안 하지만 최소한 접근 가능한 곳으로 되돌린다는 목적은 같다) /
    // "Delete"(선택 항목만 영구 삭제). 둘 다 선택된 게 있을 때만 눌린다 — 여러
    // 개 고르면 그 여러 개가 한 번에 복구/삭제된다(icon_grid 의 마퀴 다중 선택을
    // 그대로 쓴다).
    fn draw_sel_row(&self, r: &mut Renderer, area: Rect, win: &WinInput, lang: Language) -> (bool, bool) {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        r.rect(area.x, area.y, area.w, 1.0, GRAY); // 격자와 구분하는 윗줄
        let enabled = !self.selected.is_empty();
        let by = area.y + 3.0;
        let bh = area.h - 6.0;
        let restore = sel_button(r, Rect::new(area.x + 4.0, by, SEL_BTN_W, bh), t(lang, common::RESTORE), enabled, win);
        let delete = sel_button(r, Rect::new(area.x + 8.0 + SEL_BTN_W, by, SEL_BTN_W, bh), t(lang, common::DELETE), enabled, win);
        (restore, delete)
    }

    fn draw_status_bar(&self, r: &mut Renderer, area: Rect, lang: Language) {
        let n = self.items.len();
        let sel = self.selected.len();
        let text = if sel == 0 {
            t(lang, common::OBJECT_COUNT).replace("{n}", &n.to_string())
        } else {
            t(lang, common::OBJECT_COUNT_SELECTED).replace("{n}", &n.to_string()).replace("{s}", &sel.to_string())
        };
        let ty = area.y + (STATUS_H - CELL_H * 0.8) / 2.0;
        let seg_w = (area.w * 0.6).max(80.0);
        sunken(r, area.x, area.y, seg_w, STATUS_H);
        r.text_clipped(area.x + 8.0, ty, &text, 0.8, BLACK, seg_w - 16.0);
        sunken(r, area.x + seg_w + 2.0, area.y, area.w - seg_w - 2.0, STATUS_H);
        r.text_clipped(area.x + seg_w + 10.0, ty, &display_name(lang, MY_COMPUTER_NAME), 0.8, BLACK, area.w - seg_w - 20.0);
    }
}

impl App for RecycleBinApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(display_name(lang, RECYCLE_BIN_NAME).into_owned())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        let addr_area = Rect::new(area.x, area.y, area.w, ADDR_H);
        self.draw_address_bar(r, assets, addr_area, lang);

        let body_y = addr_area.y + ADDR_H;
        let body_h = (area.y + area.h - STATUS_H - body_y).max(0.0);
        let left_w = LEFT_W.min(area.w * 0.45);
        let left_area = Rect::new(area.x, body_y, left_w, body_h);
        let empty_clicked = self.draw_left_panel(r, assets, left_area, win, lang);

        let right_x = area.x + left_w;
        let right_w = (area.w - left_w).max(0.0);
        let grid_h = (body_h - SEL_ROW_H).max(0.0);
        let grid_area = Rect::new(right_x, body_y, right_w, grid_h);
        let sel_row_area = Rect::new(right_x, body_y + grid_h, right_w, SEL_ROW_H);
        let (restore_clicked, delete_clicked) = self.draw_sel_row(r, sel_row_area, win, lang);
        // 휴지통 안의 항목은 더블클릭해도 안 연다 — 실제 Windows 휴지통도 그 안의
        // 파일을 그냥 두 번 클릭해서 여는 대신 먼저 복원하게 한다(지운 걸 열어서
        // 계속 쓰게 두면 "지웠다"는 의미가 흐려진다) — icon_grid 가 돌려주는 클릭
        // 위치는 단일 클릭 선택에만 쓰이고(icon_grid 내부에서 이미 처리됨), 더블
        // 클릭으로 여는 동작 자체를 여기서 만들지 않는다.
        icon_grid(
            r, assets, win, grid_area, &self.items, &mut self.selected, &mut self.marquee_start, &mut self.prev_down,
            &mut self.grid_scroll, &mut self.grid_scroll_disp, true, &mut self.grid_sb_drag, lang,
        );

        self.draw_status_bar(r, Rect::new(area.x, area.y + area.h - STATUS_H, area.w, STATUS_H), lang);

        let selected_ids = || self.selected.iter().filter_map(|&i| self.items.get(i).map(|(id, ..)| *id)).collect::<Vec<FileId>>();
        if restore_clicked && !self.selected.is_empty() {
            AppAction::Restore(selected_ids())
        } else if delete_clicked && !self.selected.is_empty() {
            AppAction::EmptyTrash(selected_ids())
        } else if empty_clicked {
            let ids: Vec<FileId> = self.items.iter().map(|(id, ..)| *id).collect();
            AppAction::EmptyTrash(ids)
        } else {
            AppAction::None
        }
    }
}
