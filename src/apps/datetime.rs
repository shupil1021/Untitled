//! 날짜 및 시간 조정 — 시계를 눌러 열리는 팝업의 버튼으로 연다. 실제 PC 시간은 건드리지 않고, 그 위에 더해지는
//! 가상 시계 차이(fs.clock_offset_secs)를 맞춘다. 서브 게임의 꽃은 "PC 시간 + 이 차이"로 자라서, 시스템 시계를 바꿀
//! 수 없는 PC(권한이 없거나 macOS/Linux)에서도 시간 퍼즐을 풀 수 있다. 실제 PC 시계를 돌려도 똑같이 통한다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::scenes::desktop::clock::{civil_date, hour_minute, local_unix_now};
use crate::strings::{common, datetime as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

const DAY: i64 = 86_400;
const MAX_OFFSET: i64 = 100 * 365 * DAY; // 이 이상은 의미가 없다(오버플로와 이상한 날짜 방지)

// 눌러서 더하는 양 — 라벨과 초. (월은 30일, 연은 365일로 센다.)
const STEPS: [(&str, i64); 6] = [("-1y", -365 * DAY), ("-1m", -30 * DAY), ("-1d", -DAY), ("+1d", DAY), ("+1m", 30 * DAY), ("+1y", 365 * DAY)];

// 차이에 step 을 더하되 ±100년을 넘지 않게 한다.
pub fn apply_step(offset: i64, step: i64) -> i64 {
    (offset + step).clamp(-MAX_OFFSET, MAX_OFFSET)
}

fn stamp(local_unix: i64) -> String {
    let (y, mo, d) = civil_date(local_unix);
    let (hh, mm) = hour_minute(local_unix);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}")
}

pub struct DateTimeApp {
    offset: i64,
    settings: Rc<RefCell<Settings>>,
}

impl DateTimeApp {
    pub fn new(offset: i64, settings: Rc<RefCell<Settings>>) -> DateTimeApp {
        DateTimeApp { offset, settings }
    }
}

impl App for DateTimeApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(t(lang, s::TITLE).to_string())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let real = local_unix_now();
        let x = area.x + 14.0;

        label(r, x, area.y + 12.0, t(lang, s::GAME_CLOCK), BLACK);
        r.text(x + 8.0, area.y + 32.0, &stamp(real + self.offset), 1.1, NAVY);
        label(r, x, area.y + 62.0, t(lang, s::PC_CLOCK), GRAY);
        r.text(x + 8.0, area.y + 80.0, &stamp(real), 0.9, GRAY);
        let days = self.offset / DAY;
        let sign = if days > 0 { "+" } else { "" };
        r.text(x, area.y + 102.0, &t(lang, s::OFFSET_DAYS).replace("{n}", &format!("{sign}{days}")), 0.85, BLACK);

        // 눌러서 시계를 앞뒤로 — 한 줄에 여섯 개.
        let gap = 4.0;
        let bw = (area.w - 28.0 - gap * 5.0) / 6.0;
        let by = area.y + 124.0;
        let mut result = AppAction::None;
        for (i, (text, step)) in STEPS.iter().enumerate() {
            if button(r, x + i as f32 * (bw + gap), by, bw, 24.0, text, win) {
                self.offset = apply_step(self.offset, *step);
                result = AppAction::SetClockOffset(self.offset);
            }
        }
        if button(r, x, by + 32.0, 90.0, 24.0, t(lang, s::RESET), win) && self.offset != 0 {
            self.offset = 0;
            result = AppAction::SetClockOffset(0);
        }
        r.text_clipped(x, area.y + area.h - 52.0, t(lang, s::NOTE), 0.75, GRAY, area.w - 28.0);
        if button(r, area.x + area.w - 80.0, area.y + area.h - 32.0, 70.0, 24.0, t(lang, common::OK), win) {
            return AppAction::Close;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_add_up_and_stay_in_range() {
        assert_eq!(apply_step(0, 365 * DAY), 365 * DAY);
        assert_eq!(apply_step(365 * DAY, -365 * DAY), 0);
        assert_eq!(apply_step(MAX_OFFSET, 365 * DAY), MAX_OFFSET, "100년을 넘지 않는다");
        assert_eq!(apply_step(-MAX_OFFSET, -365 * DAY), -MAX_OFFSET);
    }

    #[test]
    fn one_year_of_steps_is_exactly_what_the_flower_needs() {
        // 서브 게임의 "1년"(365일)과 같은 양이라 +1y 한 번이면 꽃이 필 조건을 채운다.
        let one_year = STEPS.iter().find(|(l, _)| *l == "+1y").unwrap().1;
        assert_eq!(one_year as f64, 365.0 * 86_400.0);
    }

    #[test]
    fn the_stamp_shows_date_and_time() {
        assert_eq!(stamp(1_759_708_800 + 13 * 3600 + 5 * 60), "2025-10-06 13:05");
    }
}
