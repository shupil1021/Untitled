//! HexTool 의 "이미지 선택"을 누르면 여는 창 — "My Computer"(File Explorer)와
//! 같은 격자 배치로 지금 ?????에 떠 있는 사진들을 보여준다. 아이콘 대신 실제
//! 사진을 그때그때 디코드해 축소판(썸네일)으로 보여줘서 어떤 사진인지 클릭해
//! 보기 전에 미리 알아볼 수 있다(photos.rs 의 피드와 같은 지연 로딩 요령 —
//! 화면에 한 번이라도 보인 셀만 디코드해서 텍스처로 올린다). 클릭하면(마퀴
//! 드래그 없이 단순 클릭) 그 자리에서 바로 그 사진을 HexTool 의 검수 대상으로
//! 고르고 이 창은 닫힌다 — 파일 열기 대화상자처럼 "고르면 곧장 확정" 이다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{RenderingBackend, TextureId};

use crate::foundation::Settings;
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{hextool as s, t};
use crate::ui::{label, BLACK, FACE, GRAY, WHITE};

use super::photos::{find_photo_dir, load_scaled_texture};
use super::widgets::{ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

const CELL_W: f32 = 96.0;
const CELL_H: f32 = 96.0; // 썸네일 정사각형 한 변
const LABEL_H: f32 = 18.0;
const GAP: f32 = 10.0;
const PAD: f32 = 12.0;

pub struct HexPickerApp {
    ids: Vec<String>,                             // fs.photos_current 스냅샷
    thumbs: Vec<Option<(TextureId, u32, u32)>>,   // ids 와 같은 길이 — 화면에 한 번이라도 보인 것만 Some(디코드 성공 시)
    tried: Vec<bool>,                              // ids 와 같은 길이 — 한 번이라도 디코드를 시도했는지(실패해도 다시 안 건드리게)
    scroll: f32,
    scroll_disp: f32,
    sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl HexPickerApp {
    pub fn new(ids: Vec<String>, settings: Rc<RefCell<Settings>>) -> HexPickerApp {
        let n = ids.len();
        HexPickerApp { ids, thumbs: vec![None; n], tried: vec![false; n], scroll: 0.0, scroll_disp: 0.0, sb_drag: false, settings }
    }
}

impl App for HexPickerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
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

        let photo_dir = find_photo_dir();
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

            if !self.tried[i] {
                self.tried[i] = true;
                if let Some(dir) = &photo_dir {
                    self.thumbs[i] = load_scaled_texture(ctx, &dir.join(id), Some(CELL_H - 4.0));
                }
            }

            let thumb_rect = Rect::new(cx, cy, CELL_W, CELL_H);
            r.rect(thumb_rect.x, thumb_rect.y, thumb_rect.w, thumb_rect.h, BLACK);
            if let Some((tex, w, h)) = self.thumbs[i] {
                let (iw, ih) = (w as f32, h as f32);
                let scale = (thumb_rect.w / iw).min(thumb_rect.h / ih);
                let (dw, dh) = (iw * scale, ih * scale);
                let dx = thumb_rect.x + (thumb_rect.w - dw) / 2.0;
                let dy = thumb_rect.y + (thumb_rect.h - dh) / 2.0;
                r.sprite(tex, dx, dy, dw, dh, WHITE);
            }
            let hover = thumb_rect.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
            if hover {
                crate::ui::border(r, thumb_rect.x, thumb_rect.y, thumb_rect.w, thumb_rect.h, [1.0, 0.9, 0.2, 1.0]);
            }

            let name = id.rsplit('/').next().unwrap_or(id);
            let name_w = r.text_width(name, 0.75).min(CELL_W);
            r.text_clipped(cx + (CELL_W - name_w) / 2.0, cy + CELL_H + 2.0, name, 0.75, BLACK, CELL_W);

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
