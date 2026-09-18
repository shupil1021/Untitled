//! 설치 마법사를 끝내고 바탕화면에 생긴 게임 아이콘(FileKind::GameInstalled(GameKind::Pacman))을
//! 열면 뜨는 창 — 지금은 "메일 → 다운로드 → 설치 마법사 → 창 띄우기" 파이프라인만
//! 만드는 단계라, 실제 팩맨류 게임 로직은 아직 없다. 그 자리를 표시하는 화면(어두운
//! 배경 + 팩맨 아이콘 + 준비 중 문구)만 그린다 — 나중에 여기에 실제 미로/유령/점수
//! 로직을 채우면 된다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{pacman as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

pub struct PacmanApp {
    settings: Rc<RefCell<Settings>>,
}

impl PacmanApp {
    pub fn new(settings: Rc<RefCell<Settings>>) -> PacmanApp {
        PacmanApp { settings }
    }
}

impl App for PacmanApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, _win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, BLACK);

        let lang = self.settings.borrow().language;
        let cx = area.x + area.w / 2.0;
        let icon_s = (area.w.min(area.h) * 0.3).clamp(32.0, 96.0);
        let icon_y = area.y + area.h * 0.32 - icon_s / 2.0;
        draw_icon(r, assets, &IconType::Game, cx - icon_s / 2.0, icon_y, icon_s);

        let title = t(lang, s::TITLE);
        let tw = r.text_width(title, 1.2);
        r.text(cx - tw / 2.0, icon_y + icon_s + 16.0, title, 1.2, [0.95, 0.82, 0.15, 1.0]);

        let hint = t(lang, s::COMING_SOON);
        let hw = r.text_width(hint, 0.8);
        r.text(cx - hw / 2.0, icon_y + icon_s + 44.0, hint, 0.8, GRAY);

        AppAction::None
    }
}
