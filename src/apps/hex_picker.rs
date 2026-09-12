//! HexTool 의 "이미지 선택"을 누르면 여는 창 — "My Computer"(File Explorer)와
//! 같은 격자 배치로 지금 ?????에 떠 있는 사진들을 보여준다. 아이콘 대신 실제
//! 사진을 그때그때 디코드해 축소판(썸네일)으로 보여줘서 어떤 사진인지 클릭해
//! 보기 전에 미리 알아볼 수 있다(photos.rs 의 피드와 같은 지연 로딩 요령 —
//! 화면에 한 번이라도 보인 셀만 디코드해서 텍스처로 올린다). 클릭하면(마퀴
//! 드래그 없이 단순 클릭) 그 자리에서 바로 그 사진을 HexTool 의 검수 대상으로
//! 고르고 이 창은 닫힌다 — 파일 열기 대화상자처럼 "고르면 곧장 확정" 이다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{hextool as s, t};
use crate::ui::{label, IconType, BLACK, FACE, GRAY, WHITE};

use super::widgets::{draw_thumb_or_icon, ease_scroll, scrollbar, ThumbCache};
use super::{App, AppAction, WinInput};

// 아이콘 대신 실사진을 보여주는 자리라, 정말로 아이콘 하나 크기(draw_icon 이
// icon_grid 에서 쓰는 32px)와 비슷하게 맞춘다 — 96px 로 키웠더니 "미리보기"라기
// 보다 별도의 큰 이미지 뷰어처럼 보인다는 피드백을 받았다.
const THUMB_SIZE: f32 = 32.0;
const CELL_W: f32 = 92.0; // widgets::icon_grid 의 셀 폭과 맞췄다 — 같은 게임 안에서 같은 격자 배치로 보이도록.
const CELL_H: f32 = 36.0; // 썸네일(32) + 위아래 여백
const LABEL_H: f32 = 18.0;
const GAP: f32 = 10.0;
const PAD: f32 = 12.0;

pub struct HexPickerApp {
    ids: Vec<String>, // fs.photos_current 스냅샷 — 배열 인덱스를 그대로 FileId 삼아 thumbs 캐시 키로 쓴다
    // widgets.rs::draw_thumb_or_icon 이 쓰는 지연 로딩 텍스처 캐시 — hex_picker 는
    // 진짜 fs 파일이 아니라서(고를 게 전부 사진 자체) FileId 대신 배열 인덱스를
    // 키로 쓴다.
    thumbs: ThumbCache,
    scroll: f32,
    scroll_disp: f32,
    sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl HexPickerApp {
    pub fn new(ids: Vec<String>, settings: Rc<RefCell<Settings>>) -> HexPickerApp {
        HexPickerApp { ids, thumbs: ThumbCache::new(), scroll: 0.0, scroll_disp: 0.0, sb_drag: false, settings }
    }

    // ?????가 새로 갱신되면(재연구 업무 보고 메일을 보내서) desktop.rs 가 부른다 —
    // 이 창을 열어둔 채 사진을 고르지 않고 있다가 피드가 새 배치로 바뀌면, ids 만
    // 갈아끼우지 않고 그대로 두면 이제 존재하지 않는(옛 배치) 사진을 고를 수 있게
    // 되어 그 검수 결과가 새 배치의 진행 상황(N/M)에 전혀 반영되지 않는 채로
    // 낭비된다. thumbs 캐시도 같이 비워야 한다 — 배열 인덱스를 그대로 캐시 키로
    // 쓰므로(위 필드 설명 참고), 안 비우면 같은 인덱스에 옛 사진의 썸네일이 남아
    // 새 배치의 다른 사진 위에 잘못 그려진다.
    pub(crate) fn refresh_ids(&mut self, ids: Vec<String>) {
        self.ids = ids;
        self.thumbs.clear();
    }
}

impl App for HexPickerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let lang = self.settings.borrow().language;
        if self.ids.is_empty() {
            label(r, area.x + 10.0, area.y + 10.0, t(lang, s::NO_FILES_FOUND), GRAY);
            return AppAction::None;
        }

        let list_area = Rect::new(area.x + 2.0, area.y + 2.0, area.w - 4.0, area.h - 4.0);
        r.rect(list_area.x, list_area.y, list_area.w, list_area.h, WHITE);

        let cols = (((list_area.w - PAD - 10.0) / (CELL_W + GAP)).floor() as usize).max(1);
        let cell_total_h = CELL_H + LABEL_H + GAP;
        let total_rows = self.ids.len().div_ceil(cols);
        let content_h = total_rows as f32 * cell_total_h;
        let max_scroll = (content_h - (list_area.h - PAD)).max(0.0);

        if list_area.contains(win.mouse.0, win.mouse.1) {
            self.scroll -= win.wheel / 120.0 * cell_total_h;
        }
        self.scroll = self.scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.scroll_disp, self.scroll, win.dt, smooth);

        let has_sb = max_scroll > 0.0;
        let sb_w = if has_sb { 10.0 } else { 0.0 };

        let mut result = AppAction::None;
        let outer_clip = r.clip();
        r.set_clip(Some(list_area));
        for (i, id) in self.ids.iter().enumerate() {
            let row = i / cols;
            let col = i % cols;
            let cx = list_area.x + PAD + col as f32 * (CELL_W + GAP);
            let cy = list_area.y + PAD + row as f32 * cell_total_h - self.scroll_disp;
            if cy + cell_total_h < list_area.y || cy > list_area.y + list_area.h {
                continue; // 화면 밖 행은 그리지도, 디코드 시도조차 하지 않는다
            }

            // 아이콘 자리를 그대로 대신하는 거라 아이콘과 같은 크기(THUMB_SIZE)로
            // 셀 위쪽 가운데에 그린다 — icon_grid 의 draw_icon 호출과 같은 자리.
            // 여백은 검은색으로 안 채운다 — 정사각형이 아닌 사진 옆에 검은 여백이
            // 도드라져 보인다는 피드백을 받아 draw_thumb_or_icon 자체가 letterbox
            // 를 그냥 투명하게 비워둔다.
            let cell_rect = Rect::new(cx, cy, CELL_W, CELL_H + LABEL_H);
            let hover = cell_rect.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
            if hover {
                r.rect(cell_rect.x, cell_rect.y, cell_rect.w, cell_rect.h, [0.82, 0.88, 0.98, 1.0]);
            }
            let tx = cx + (CELL_W - THUMB_SIZE) / 2.0;
            let ty = cy + 2.0;
            draw_thumb_or_icon(ctx, r, assets, &mut self.thumbs, i, Some(id), &IconType::Img, tx, ty, THUMB_SIZE);

            let name = id.rsplit('/').next().unwrap_or(id);
            let name_w = r.text_width(name, 0.75).min(CELL_W);
            r.text_clipped(cx + (CELL_W - name_w) / 2.0, cy + CELL_H, name, 0.75, BLACK, CELL_W);

            if hover && win.mouse_clicked {
                result = AppAction::SelectPhotoForHexTool(id.clone());
            }
        }
        r.set_clip(outer_clip);

        if has_sb {
            let visible_frac = ((list_area.h - PAD) / content_h).clamp(0.05, 1.0);
            scrollbar(
                r, win, list_area.x + list_area.w - sb_w - 2.0, list_area.y + 2.0, sb_w - 2.0, list_area.h - 4.0, visible_frac,
                self.scroll_disp, &mut self.scroll, max_scroll, &mut self.sb_drag,
            );
        }

        result
    }
}
