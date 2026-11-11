//! 인벤토리 — I(또는 Tab)로 여닫는 가방 창. 칸마다 물건 이름 표식이 있고, 고른 물건의 이름과 설명이 아래에 나온다.
//! 열려 있는 동안 게임은 멈추고(호출부가 `is_open` 으로 처리), 왼쪽/오른쪽(A/D) 키로 칸을 옮긴다.
//! 가진 물건 목록은 게임마다 다르므로 호출부가 `ItemInfo` 목록으로 만들어 넘긴다.

use miniquad::KeyCode;

use crate::render::gfx::Renderer;
use crate::scenes::Input;

use super::{i18n, text_scale, View, VIEW_H, VIEW_W};

const COLS: usize = 4;
const SLOTS: usize = 8;
const SLOT: f32 = 64.0;
const GAP: f32 = 10.0;

// 가방에 든 물건 하나 — name/desc 는 한국어 원문(그릴 때 지금 언어로 바꾼다. 이미 바꾼 문구도 그대로 나간다).
pub struct ItemInfo {
    pub name: String,
    pub desc: &'static str,
}

#[derive(Default)]
pub struct Inventory {
    open: bool,
    sel: usize,
}

impl Inventory {
    pub fn is_open(&self) -> bool {
        self.open
    }

    // 열려 있는 동안의 입력(여닫기 + 칸 옮기기) — 이번 프레임 입력을 이 가방이 먹었으면 true. count 는 지금 가진 물건 수.
    pub fn handle_input(&mut self, input: &Input, count: usize) -> bool {
        let toggle = input.pressed(KeyCode::I) || input.pressed(KeyCode::Tab);
        if self.open {
            if toggle || input.pressed(KeyCode::Escape) {
                self.open = false;
            } else if input.pressed(KeyCode::Right) || input.pressed(KeyCode::D) {
                self.sel = (self.sel + 1).min(count.saturating_sub(1));
            } else if input.pressed(KeyCode::Left) || input.pressed(KeyCode::A) {
                self.sel = self.sel.saturating_sub(1);
            } else if input.pressed(KeyCode::Down) || input.pressed(KeyCode::S) {
                self.sel = (self.sel + COLS).min(count.saturating_sub(1));
            } else if input.pressed(KeyCode::Up) || input.pressed(KeyCode::W) {
                self.sel = self.sel.saturating_sub(COLS);
            }
            return true;
        }
        if toggle {
            self.open = true;
            self.sel = 0;
            return true;
        }
        false
    }

    // 닫혀 있을 때 왼쪽 위에 작게 보이는 "가방" 안내 — row 는 위에 이미 쓴 줄 수.
    pub fn draw_hint(&self, r: &mut Renderer, view: &View, row: f32, count: usize) {
        let k = text_scale();
        let text = format!("[I] {} ({})", i18n::tr("가방"), count);
        r.text(view.px(10.0), view.py(10.0 + row * 20.0 * k), &text, 0.8 * view.s * k, [0.75, 0.8, 1.0, 1.0]);
    }

    pub fn draw(&self, r: &mut Renderer, view: &View, items: &[ItemInfo]) {
        if !self.open {
            return;
        }
        let s = view.s;
        r.rect(view.rect.x, view.rect.y, view.rect.w, view.rect.h, [0.0, 0.0, 0.0, 0.7]);
        let cx = view.px(VIEW_W / 2.0);
        let title = i18n::tr("가방");
        let tw = r.text_width(&title, 1.6 * s);
        r.text(cx - tw / 2.0, view.py(34.0), &title, 1.6 * s, [1.0, 1.0, 1.0, 1.0]);

        let grid_w = COLS as f32 * SLOT + (COLS as f32 - 1.0) * GAP;
        let (gx, gy) = (VIEW_W / 2.0 - grid_w / 2.0, 80.0);
        for i in 0..SLOTS {
            let (col, row) = (i % COLS, i / COLS);
            let (x, y) = (gx + col as f32 * (SLOT + GAP), gy + row as f32 * (SLOT + GAP));
            let picked = i == self.sel && i < items.len();
            let edge = if picked { [1.0, 0.9, 0.5, 1.0] } else { [0.45, 0.45, 0.5, 1.0] };
            r.rect(view.px(x), view.py(y), SLOT * s, SLOT * s, edge);
            r.rect(view.px(x + 2.0), view.py(y + 2.0), (SLOT - 4.0) * s, (SLOT - 4.0) * s, [0.08, 0.08, 0.12, 1.0]);
            if let Some(item) = items.get(i) {
                // 이름 첫 글자를 색 표식으로 — 물건마다 다른 색.
                let seed = item.name.chars().fold(7u32, |a, c| a.wrapping_mul(31).wrapping_add(c as u32));
                let tint = [0.35 + (seed % 5) as f32 * 0.12, 0.35 + ((seed / 5) % 5) as f32 * 0.12, 0.35 + ((seed / 25) % 5) as f32 * 0.12, 1.0];
                r.rect(view.px(x + 14.0), view.py(y + 10.0), (SLOT - 28.0) * s, (SLOT - 28.0) * s, tint);
                let mark: String = i18n::tr(&item.name).chars().next().into_iter().collect();
                let mw = r.text_width(&mark, 1.4 * s);
                r.text(view.px(x + SLOT / 2.0) - mw / 2.0, view.py(y + 14.0), &mark, 1.4 * s, [1.0, 1.0, 1.0, 1.0]);
            }
        }

        let info_y = gy + 2.0 * (SLOT + GAP) + 6.0;
        if let Some(item) = items.get(self.sel) {
            let name = i18n::tr(&item.name);
            let nw = r.text_width(&name, 1.1 * s);
            r.text(cx - nw / 2.0, view.py(info_y), &name, 1.1 * s, [1.0, 0.9, 0.5, 1.0]);
            let max_w = (VIEW_W - 120.0) * s;
            for (i, line) in crate::ui::wrap_lines(r, &i18n::tr(item.desc), 0.85 * s, max_w).iter().take(3).enumerate() {
                let lw = r.text_width(line, 0.85 * s);
                r.text(cx - lw / 2.0, view.py(info_y + 30.0 + i as f32 * 20.0), line, 0.85 * s, [0.85, 0.88, 1.0, 1.0]);
            }
        } else {
            let msg = i18n::tr("아무것도 없다.");
            let mw = r.text_width(&msg, 1.0 * s);
            r.text(cx - mw / 2.0, view.py(info_y + 10.0), &msg, 1.0 * s, [0.7, 0.7, 0.75, 1.0]);
        }
        let hint = format!("[I] {}   [A/D] {}", i18n::tr("닫기"), i18n::tr("고르기"));
        let hw = r.text_width(&hint, 0.7 * s);
        r.text(cx - hw / 2.0, view.py(VIEW_H - 34.0), &hint, 0.7 * s, [0.65, 0.65, 0.7, 1.0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: KeyCode) -> Input {
        let mut input = Input::default();
        input.on_key_down(key, false);
        input
    }

    #[test]
    fn i_opens_and_closes_and_arrows_move_within_the_items() {
        let mut inv = Inventory::default();
        assert!(!inv.handle_input(&Input::default(), 3));
        assert!(inv.handle_input(&press(KeyCode::I), 3));
        assert!(inv.is_open());
        inv.handle_input(&press(KeyCode::Right), 3);
        inv.handle_input(&press(KeyCode::Right), 3);
        inv.handle_input(&press(KeyCode::Right), 3);
        assert_eq!(inv.sel, 2, "물건 수를 넘어서 옮겨지지 않는다");
        inv.handle_input(&press(KeyCode::Escape), 3);
        assert!(!inv.is_open());
    }
}
