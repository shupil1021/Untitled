//! .tar 압축파일을 더블클릭했을 때 뜨는 창. HexTool 설치 여부(FileSystem::hex_tool_installed)
//! 에 따라 다른 안내를 보여준다 — 설치 전엔 못 연다는 메시지만, 설치 후엔 열렸다는
//! 메시지를 보여준다(실제 내용물을 풀어 보여주는 기능은 아직 없다).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::gfx::{Assets, Color, Rect, Renderer};
use crate::strings::{archive as s, common, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

pub struct ArchiveApp {
    installed: bool,
    settings: Rc<RefCell<Settings>>,
}

impl ArchiveApp {
    pub(super) fn new(installed: bool, settings: Rc<RefCell<Settings>>) -> ArchiveApp {
        ArchiveApp { installed, settings }
    }
}

impl App for ArchiveApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        r.rect(area.x, area.y, area.w, area.h, FACE);
        draw_icon(r, assets, &IconType::Tar, area.x + 14.0, area.y + 14.0, 40.0);

        // 창 폭이 좁아도 실제 폭 기준으로 줄바꿈해서 절대 안 잘리게 한다.
        let tx = area.x + 66.0;
        let text_w = (area.x + area.w - tx - 10.0).max(20.0);
        let (line1, line2) = if self.installed {
            (t(lang, s::OPENED_LINE1), t(lang, s::OPENED_LINE2))
        } else {
            (t(lang, s::NOT_INSTALLED_LINE1), t(lang, s::NOT_INSTALLED_LINE2))
        };
        let y = draw_wrapped(r, tx, area.y + 14.0, text_w, line1, BLACK);
        draw_wrapped(r, tx, y + 6.0, text_w, line2, GRAY);

        if button(r, area.x + area.w - 80.0, area.y + area.h - 32.0, 70.0, 24.0, t(lang, common::OK), win) {
            return AppAction::Close;
        }
        AppAction::None
    }
}

// 주어진 폭에 맞춰 줄바꿈해서 그리고, 마지막 줄 바로 아래 y 를 돌려준다(다음 줄을
// 겹치지 않게 이어 배치할 수 있게 — installer.rs 의 draw_paragraph 와 같은 요령).
fn draw_wrapped(r: &mut Renderer, x: f32, y: f32, w: f32, text: &str, color: Color) -> f32 {
    let mut ty = y;
    for line in wrap_lines(r, text, 1.0, w) {
        r.text(x, ty, &line, 1.0, color);
        ty += 20.0;
    }
    ty
}
