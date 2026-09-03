//! .lock 비밀번호 대화상자.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{FileId, Settings};
use crate::gfx::{ADVANCE, Assets, Rect, Renderer};
use crate::strings::{common, password as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

pub struct PasswordApp {
    id: FileId,
    password: String,
    entry: String,
    error: bool,
    settings: Rc<RefCell<Settings>>,
}

impl PasswordApp {
    pub(super) fn new(id: FileId, password: String, settings: Rc<RefCell<Settings>>) -> PasswordApp {
        PasswordApp { id, password, entry: String::new(), error: false, settings }
    }
}

impl App for PasswordApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let lang = self.settings.borrow().language;
        label(r, area.x + 10.0, area.y + 10.0, t(lang, s::ENTER_PASSWORD), BLACK);

        // 입력 처리 (포커스일 때만)
        if win.focused {
            for &c in &win.input.typed {
                if !c.is_control() && self.entry.len() < 24 {
                    self.entry.push(c);
                    self.error = false;
                }
            }
            if win.input.pressed(KeyCode::Backspace) {
                self.entry.pop();
                self.error = false;
            }
        }

        // 입력창 (별표로 표시)
        let fx = area.x + 10.0;
        let fy = area.y + 34.0;
        let fw = area.w - 20.0;
        sunken(r, fx, fy, fw, 24.0);
        let stars: String = "*".repeat(self.entry.chars().count());
        r.text(fx + 5.0, fy + 4.0, &stars, 1.0, BLACK);
        if win.focused && (win.time % 1.0) < 0.5 {
            let cx = fx + 5.0 + stars.chars().count() as f32 * ADVANCE;
            r.rect(cx, fy + 6.0, 2.0, 14.0, BLACK);
        }

        // 에러 문구 자리는 항상 비워두어(표시 여부와 무관하게) OK 버튼 위치가 흔들리지 않게 한다.
        let err_y = fy + 32.0;
        if self.error {
            label(r, area.x + 10.0, err_y, t(lang, s::WRONG_PASSWORD), [0.7, 0.0, 0.0, 1.0]);
        }

        // OK 버튼: 창 맨 아래가 아니라 입력창/에러 문구 바로 아래에 붙인다 (아래쪽 빈 공간 방지).
        let ok = button(r, area.x + area.w - 80.0, err_y + 22.0, 70.0, 24.0, t(lang, common::OK), win);
        let enter = win.focused && win.input.pressed(KeyCode::Enter);
        if ok || enter {
            if self.entry == self.password {
                return AppAction::Unlock(self.id);
            } else {
                self.error = true;
                self.entry.clear();
            }
        }
        AppAction::None
    }
}
