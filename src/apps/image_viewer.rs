//! 이미지 뷰어 — 팔라스 OS가 생성한 사진을 그냥 보여준다. 지금은 검수 판정(정상=삭제/
//! 이상=체크리스트 제출) 없이 사진만 창 안에 꽉 차게(종횡비 유지) 그리는 최소 버전이다.

use miniquad::RenderingBackend;

use crate::gfx::{Assets, Rect, Renderer};
use crate::ui::WHITE;

use super::{App, AppAction, WinInput};

pub struct ImageViewerApp {
    photo_idx: usize,
}

impl ImageViewerApp {
    pub fn new(photo_idx: usize) -> ImageViewerApp {
        ImageViewerApp { photo_idx }
    }
}

impl App for ImageViewerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, _win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, [0.1, 0.1, 0.1, 1.0]); // 사진 밖 여백(레터박스)은 어둡게

        let Some(&(tex, iw, ih)) = assets.photos.get(self.photo_idx) else {
            return AppAction::None;
        };
        // 창 안에 종횡비를 유지한 채 최대한 크게 — 사진이 잘리거나 늘어나 보이면 안
        // 되므로 폭/높이 중 더 좁게 맞춰지는 쪽 배율을 쓴다(레터박스).
        let (iw, ih) = (iw as f32, ih as f32);
        let scale = (area.w / iw).min(area.h / ih);
        let (dw, dh) = (iw * scale, ih * scale);
        let dx = area.x + (area.w - dw) / 2.0;
        let dy = area.y + (area.h - dh) / 2.0;
        r.sprite(tex, dx, dy, dw, dh, WHITE);

        AppAction::None
    }
}
