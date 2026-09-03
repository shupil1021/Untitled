//! Official Site (WebView2 캡처를 텍스처로) — 실제 라이브 임베드 대신, 백그라운드에서
//! 페이지를 렌더링하고 주기적으로 캡처한 이미지를 텍스처로 올린다. 이러면 그 화면도
//! 다른 모든 것들처럼 CRT 셰이더를 그대로 통과한다(진짜 라이브 WebView2 창은 우리
//! 렌더 파이프라인 밖이라 안 됨).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{FilterMode, KeyCode, MipmapFilterMode, RenderingBackend, TextureId};

use crate::foundation::Settings;
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{official_site as s, t};
use crate::ui::*;
use crate::webview::WebviewInput;

use super::{App, AppAction, WinInput};

pub struct OfficialSiteApp {
    webview: crate::webview::WebviewHandle,
    tex: Option<TextureId>,
    cw: u32,
    ch: u32,
    dragging: bool, // Down 을 보낸 뒤 Up 을 보내기 전까지 true — 영역 밖으로 끌어도 놓칠 때까진 계속 따라간다
    settings: Rc<RefCell<Settings>>,
}

impl OfficialSiteApp {
    pub fn new(url: &str, cw: u32, ch: u32, settings: Rc<RefCell<Settings>>) -> OfficialSiteApp {
        OfficialSiteApp {
            webview: crate::webview::WebviewHandle::open(url.to_string(), cw, ch),
            tex: None,
            cw,
            ch,
            dragging: false,
            settings,
        }
    }
}

impl App for OfficialSiteApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(t(lang, s::TITLE).to_string())
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        if let Some((_, _, rgba)) = self.webview.take_frame() {
            match self.tex {
                Some(t) => ctx.texture_update(t, &rgba),
                None => {
                    let t = ctx.new_texture_from_rgba8(self.cw as u16, self.ch as u16, &rgba);
                    ctx.texture_set_filter(t, FilterMode::Linear, MipmapFilterMode::None);
                    self.tex = Some(t);
                }
            }
        }

        r.rect(area.x, area.y, area.w, area.h, BLACK);
        if let Some(tex) = self.tex {
            let scale = (area.w / self.cw as f32).min(area.h / self.ch as f32);
            let (dw, dh) = (self.cw as f32 * scale, self.ch as f32 * scale);
            let dx = area.x + (area.w - dw) / 2.0;
            let dy = area.y + (area.h - dh) / 2.0;
            r.sprite(tex, dx, dy, dw, dh, WHITE);

            // 클릭/드래그/스크롤을 표시 영역 안의 0..1 비율로 바꿔서 WebView2 페이지 DOM 에
            // 직접 이벤트로 dispatch 한다 — 그래야 캡처만 되는 정지화면이 아니라 실제로
            // 눌리고 스크롤된다. (Win32 합성 입력 메시지는 Chromium 이 무시해서 안 통했다.)
            // 픽셀이 아니라 비율로 넘기는 이유는 webview.rs 의 WebviewInput 주석 참고 —
            // 캡처 해상도와 실제 페이지 크기가 안 맞으면 픽셀 좌표가 틀어져 보였다.
            let hit = Rect::new(dx, dy, dw, dh);
            let over = hit.contains(win.mouse.0, win.mouse.1);
            // 드래그 중이면 영역 밖으로 나가도(마우스를 놓을 때까지) 계속 따라간다.
            if over || self.dragging {
                let fx = ((win.mouse.0 - dx) / dw).clamp(0.0, 1.0);
                let fy = ((win.mouse.1 - dy) / dh).clamp(0.0, 1.0);
                if over {
                    self.webview.send_input(WebviewInput::Move(fx, fy));
                    if win.mouse_clicked {
                        self.webview.send_input(WebviewInput::Down(fx, fy));
                        self.dragging = true;
                    }
                    if win.wheel != 0.0 {
                        // win.wheel 은 노치당 ~120 단위 원시값이라(다른 스크롤 처리들과
                        // 동일하게) 120 으로 나눠 "노치 개수"로 바꿔서 보낸다 — 안 나누면
                        // 한 번 굴렸는데 페이지 맨 아래까지 스크롤돼버린다.
                        self.webview.send_input(WebviewInput::Wheel(win.wheel / 120.0));
                    }
                }
                if self.dragging && !win.mouse_down {
                    self.webview.send_input(WebviewInput::Up(fx, fy));
                    self.dragging = false;
                }
            }

            // 키보드는 마우스 위치랑 무관하게 이 창이 포커스일 때만 전달한다(검색창
            // 등에 타이핑할 수 있게). document.activeElement 를 직접 조작하는 방식이라
            // 먼저 클릭해서 입력창에 포커스를 준 상태여야 한다.
            if win.focused {
                for &c in &win.input.typed {
                    if !c.is_control() {
                        self.webview.send_input(WebviewInput::Type(c));
                    }
                }
                if win.input.pressed(KeyCode::Backspace) {
                    self.webview.send_input(WebviewInput::Backspace);
                }
                if win.input.pressed(KeyCode::Enter) {
                    self.webview.send_input(WebviewInput::Enter);
                }
            }
        } else if self.webview.failed() {
            label(r, area.x + 10.0, area.y + 12.0, t(lang, s::LOAD_FAILED), WHITE);
            label(r, area.x + 10.0, area.y + 34.0, t(lang, s::RUNTIME_MISSING), GRAY);
        } else {
            draw_spinner(r, area.x + area.w / 2.0, area.y + area.h / 2.0, 14.0, win.time);
        }
        AppAction::None
    }
}
