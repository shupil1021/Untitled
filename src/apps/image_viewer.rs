//! 이미지 뷰어 — 지금은 이미지에 입힐 텍스처(그림)가 없어서, 모든 이미지를 이미지 파일 아이콘으로
//! 미리보기처럼 크게 보여주고 그 아래에 파일 이름과 크기를 적는다(게임 폴더의 이미지나 메일로 받은 사진
//! 모두 같다). 그림이 준비되면 여기서 다시 그리게 하면 된다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::strings::{image_viewer as s, t};
use crate::ui::{draw_icon, wrap_lines, IconType, WHITE};

use super::{App, AppAction, WinInput};

pub struct ImageViewerApp {
    photo_idx: usize,
    name: String,
    settings: Rc<RefCell<Settings>>,
}

impl ImageViewerApp {
    pub fn new(photo_idx: usize, name: String, settings: Rc<RefCell<Settings>>) -> ImageViewerApp {
        ImageViewerApp { photo_idx, name, settings }
    }
}

impl App for ImageViewerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, _win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, [0.1, 0.1, 0.1, 1.0]);
        let lang = self.settings.borrow().language;
        // 가운데에 이미지 아이콘(미리보기), 그 아래에 파일 이름(크게)과 "이미지 파일 (가로 x 세로)".
        let mut lines = vec![(self.name.clone(), 1.2f32, WHITE)];
        let kind = match assets.photos.get(self.photo_idx) {
            Some(&(_, w, h)) => format!("{} ({w} x {h})", t(lang, s::IMAGE_FILE)),
            None => t(lang, s::IMAGE_FILE).to_string(),
        };
        lines.push((kind, 0.85, [0.65, 0.65, 0.7, 1.0]));
        let total_h: f32 = lines.iter().map(|(_, scale, _)| 22.0 * scale + 6.0).sum();
        // 아이콘은 창 크기에 맞춰 키운다(작은 창에서도 글자가 들어갈 자리를 남긴다).
        let icon = (area.h - total_h - 24.0).min(area.w - 24.0).clamp(24.0, 128.0);
        let mut y = area.y + (area.h - total_h - icon - 12.0) / 2.0;
        draw_icon(r, assets, &IconType::Img, area.x + (area.w - icon) / 2.0, y.max(area.y + 6.0), icon);
        y = y.max(area.y + 6.0) + icon + 12.0;
        for (text, scale, color) in lines {
            for line in wrap_lines(r, &text, scale, (area.w - 20.0).max(40.0)) {
                let tw = r.text_width(&line, scale);
                r.text(area.x + (area.w - tw) / 2.0, y, &line, scale, color);
                y += 22.0 * scale + 6.0;
            }
        }
        AppAction::None
    }
}
