//! HexTool 의 "이미지 선택"을 누르면 여는 창 — "My Computer"(File Explorer)와
//! 같은 아이콘 그리드로 지금 ?????에 떠 있는 사진들을 보여준다. 클릭하면(마퀴
//! 드래그가 아닌 단순 클릭) 그 자리에서 바로 그 사진을 HexTool 의 검수 대상으로
//! 고르고 이 창은 닫힌다 — 파일 열기 대화상자처럼 "고르면 곧장 확정" 이다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{FileId, Settings};
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{hextool as s, t};
use crate::ui::{label, IconType, FACE, GRAY};

use super::widgets::icon_grid;
use super::{App, AppAction, WinInput};

pub struct HexPickerApp {
    ids: Vec<String>, // fs.photos_current 스냅샷 — items 와 같은 순서(인덱스로 대응)
    // icon_grid() 는 항목마다 FileId 를 요구하지만 내부적으로는 안 쓴다(이름/아이콘
    // 렌더링과 클릭된 인덱스 반환에만 쓰인다) — 그래서 이 자리엔 그냥 인덱스를
    // 채워 넣고, 실제 식별자는 클릭된 인덱스로 ids 에서 다시 찾는다.
    items: Vec<(FileId, String, IconType)>,
    selected: Vec<usize>,
    marquee_start: Option<(f32, f32)>,
    prev_down: bool,
    scroll: f32,
    scroll_disp: f32,
    sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl HexPickerApp {
    pub fn new(ids: Vec<String>, settings: Rc<RefCell<Settings>>) -> HexPickerApp {
        let items =
            ids.iter().enumerate().map(|(i, id)| (i, id.rsplit('/').next().unwrap_or(id).to_string(), IconType::Img)).collect();
        HexPickerApp {
            ids,
            items,
            selected: Vec::new(),
            marquee_start: None,
            prev_down: false,
            scroll: 0.0,
            scroll_disp: 0.0,
            sb_drag: false,
            settings,
        }
    }
}

impl App for HexPickerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        r.rect(area.x, area.y, area.w, area.h, FACE);
        if self.items.is_empty() {
            label(r, area.x + 10.0, area.y + 10.0, t(lang, s::NO_FILES_FOUND), GRAY);
            return AppAction::None;
        }
        let smooth = self.settings.borrow().smooth_scroll;
        let clicked = icon_grid(
            r, assets, win, area, &self.items, &mut self.selected, &mut self.marquee_start, &mut self.prev_down, &mut self.scroll,
            &mut self.scroll_disp, smooth, &mut self.sb_drag, lang,
        );
        if let Some((i, _)) = clicked
            && let Some(id) = self.ids.get(i)
        {
            return AppAction::SelectPhotoForHexTool(id.clone());
        }
        AppAction::None
    }
}
