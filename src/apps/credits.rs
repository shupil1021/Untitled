//! 크레딧 (시작 메뉴에서 바로 연다).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{Settings, CREDITS};
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{common, credits as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

// Rc<RefCell<Settings>> 를 들고 있어서 창이 열려있는 동안 언어를 바꿔도(다른 앱들과
// 마찬가지로 매 프레임 새로 읽으므로) 그 자리에서 바로 반영된다.
pub struct CreditsApp(pub Rc<RefCell<Settings>>);

impl App for CreditsApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.0.borrow().language;
        Some(t(lang, s::TITLE).to_string())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.0.borrow().language;
        r.rect(area.x, area.y, area.w, area.h, FACE);
        label(r, area.x + 14.0, area.y + 12.0, t(lang, s::DEVELOPERS), BLACK);
        for (i, name) in CREDITS.iter().enumerate() {
            r.text(area.x + 28.0, area.y + 38.0 + i as f32 * 24.0, name, 1.0, NAVY);
        }
        if button(r, area.x + area.w - 80.0, area.y + area.h - 32.0, 70.0, 24.0, t(lang, common::OK), win) {
            return AppAction::Close;
        }
        AppAction::None
    }
}
