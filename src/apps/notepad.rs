//! .txt 메모장.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::Settings;
use crate::gfx::{Assets, Rect, Renderer};
use crate::ui::*;

use super::widgets::{ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

pub struct NotepadApp {
    content: String,
    scroll: f32,      // 목표 스크롤(맨 위에서부터 줄 수)
    scroll_disp: f32, // 화면표시용(smooth 켜져 있으면 목표를 부드럽게 따라감)
    sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
    wrapped: Vec<String>, // wrap_lines 결과 캐시 — 내용은 안 바뀌므로 창 너비가 실제로
    wrapped_max_w: i32,   // 바뀔 때만(반올림한 폭 픽셀 기준) 다시 계산한다(매 프레임 재계산은 낭비).
}

impl NotepadApp {
    pub(super) fn new(content: String, settings: Rc<RefCell<Settings>>) -> NotepadApp {
        NotepadApp {
            content,
            scroll: 0.0,
            scroll_disp: 0.0,
            sb_drag: false,
            settings,
            wrapped: Vec::new(),
            wrapped_max_w: i32::MIN,
        }
    }
}

impl App for NotepadApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        const LINE_H: f32 = 20.0;
        const PAD: f32 = 6.0;
        const SB_W: f32 = 8.0; // 스크롤바 폭 예약

        // 현재 창 너비에 맞춰 줄바꿈(워드랩). 창이 좁아지면 줄이 늘어난다. 실수(f32)
        // 폭을 캐시 키로 직접 비교하면 부동소수점 오차로 같은 폭인데도 매 프레임
        // 다시 계산할 위험이 있어 정수 픽셀로 반올림해서 비교한다.
        let text_w = area.w - PAD * 2.0 - SB_W;
        let w_key = text_w.round() as i32;
        if w_key != self.wrapped_max_w {
            self.wrapped = wrap_lines(r, &self.content, 1.0, text_w);
            self.wrapped_max_w = w_key;
        }
        let lines = &self.wrapped;

        let visible = (area.h / LINE_H).floor() as usize;
        let max_scroll = (lines.len() as f32 - visible as f32).max(0.0);

        // 스크롤: 휠(노치당 ±120 이라 정규화, 포커스 없이 마우스만 올려도 동작 —
        // win.wheel 자체가 winman.rs 에서 이미 "마우스가 올라간 창"으로 걸러져서
        // 오지만, 그건 창 전체(타이틀바 포함) 기준이라 콘텐츠 영역(area) 위에 있을
        // 때만으로 한 번 더 좁힌다) 또는 위/아래 방향키(이쪽은 실제 키보드 포커스가
        // 있는 창에서만).
        if area.contains(win.mouse.0, win.mouse.1) {
            self.scroll -= win.wheel / 120.0 * 3.0;
        }
        if win.focused {
            if win.input.pressed(KeyCode::Down) {
                self.scroll += 1.0;
            }
            if win.input.pressed(KeyCode::Up) {
                self.scroll -= 1.0;
            }
        }
        self.scroll = self.scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.scroll_disp, self.scroll, win.dt, smooth);

        // 픽셀 단위로 그려서 줄 단위로 딱딱 끊기지 않고 자연스럽게 스크롤되게 한다.
        let line_off = self.scroll_disp * LINE_H;
        let first = self.scroll_disp as usize;
        r.set_clip(Some(area));
        let mut ty = area.y + 4.0 - (line_off - first as f32 * LINE_H);
        for line in lines.iter().skip(first) {
            if ty > area.y + area.h {
                break;
            }
            r.text(area.x + PAD, ty, line, 1.0, BLACK);
            ty += LINE_H;
        }
        r.set_clip(None);

        // 스크롤바 (내용이 넘칠 때) — 마우스로 드래그해서 스크롤 가능.
        if max_scroll > 0.0 {
            let sb_x = area.x + area.w - SB_W;
            let frac = visible as f32 / lines.len() as f32;
            scrollbar(r, win, sb_x, area.y, SB_W, area.h, frac, self.scroll_disp, &mut self.scroll, max_scroll, &mut self.sb_drag);
        }
        AppAction::None
    }
}
