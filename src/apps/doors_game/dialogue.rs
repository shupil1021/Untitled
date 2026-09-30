//! 화면 아래쪽 대화창 — 타자기처럼 한 글자씩(글자마다 살짝 다른 간격으로) 나타나는
//! 대사 줄(Line)과 `[Y] 수락 / [N] 거부` 선택지(Choice). 좌표/크기는 전부 게임 화면
//! 기준 해상도(VIEW_W x VIEW_H) 값이고, 그릴 때 배율을 곱한다.

use crate::random::Rng;
use crate::render::gfx::{Renderer, CELL_H};

use super::{VIEW_H, VIEW_W};

const SIDE_MARGIN: f32 = 24.0;
const BOTTOM_MARGIN: f32 = 36.0;
const HEIGHT: f32 = VIEW_H / 3.0; // 화면 아래쪽 대략 1/3을 덮는다
const TEXT_SCALE: f32 = 1.4;
const LINE_H: f32 = CELL_H * TEXT_SCALE;
const CHAR_DELAY_MIN: f32 = 0.02;
const CHAR_DELAY_MAX: f32 = 0.09;

pub enum Entry {
    Line(String),
    Choice(String),
}

pub struct Dialogue {
    entries: Vec<Entry>,
    index: usize,         // 몇 번째 항목을 보여주는 중인지 — len 이면 대화 끝(안 뜸)
    visible_chars: usize, // 그 항목에서 지금까지 드러낸 글자 수
    next_char_in: f32,    // 다음 글자를 드러내기까지 남은 시간(초)
    rng: Rng,             // 글자 간격을 흔드는 난수
}

impl Dialogue {
    pub fn new() -> Dialogue {
        Dialogue { entries: Vec::new(), index: 0, visible_chars: 0, next_char_in: 0.0, rng: Rng::from_time() }
    }

    pub fn active(&self) -> bool {
        self.index < self.entries.len()
    }

    fn text(&self) -> &str {
        match self.entries.get(self.index) {
            Some(Entry::Line(s) | Entry::Choice(s)) => s,
            None => "",
        }
    }

    fn fully_typed(&self) -> bool {
        self.visible_chars >= self.text().chars().count()
    }

    // 지금 항목이 다 타이핑된 선택지인지 — 이때만 Y/N 이 먹는다.
    pub fn choice_ready(&self) -> bool {
        matches!(self.entries.get(self.index), Some(Entry::Choice(_))) && self.fully_typed()
    }

    pub fn start(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        self.index = 0;
        self.visible_chars = 0;
        self.next_char_in = 0.0;
    }

    fn next_entry(&mut self) {
        self.index += 1;
        self.visible_chars = 0;
        self.next_char_in = 0.0;
    }

    // "아무 입력" — 타이핑 중이면 그 줄을 다 보여주고, 다 보여준 평범한 줄이면 다음으로.
    // 선택지는 여기서 절대 안 넘어가고 choose() 로만 넘어간다.
    pub fn advance(&mut self) {
        if !self.fully_typed() {
            self.visible_chars = self.text().chars().count();
        } else if matches!(self.entries.get(self.index), Some(Entry::Line(_))) {
            self.next_entry();
        }
    }

    // 다 타이핑된 선택지에서 Y/N 을 골랐을 때 — 다음 항목으로 넘어간다.
    pub fn choose(&mut self) {
        if self.choice_ready() {
            self.next_entry();
        }
    }

    // 타자기 효과 — 느려진 프레임 뒤에 한 번에 여러 글자 밀려도 되게 while 로 따라잡는다.
    pub fn tick(&mut self, dt: f32) {
        if !self.active() {
            return;
        }
        let full_len = self.text().chars().count();
        self.next_char_in -= dt;
        while self.visible_chars < full_len && self.next_char_in <= 0.0 {
            self.visible_chars += 1;
            self.next_char_in += CHAR_DELAY_MIN + self.rng.unit() * (CHAR_DELAY_MAX - CHAR_DELAY_MIN);
        }
    }

    // (ox, oy) 는 게임 화면의 왼쪽 위, s 는 VIEW 기준 → 실제 화면 배율.
    pub fn draw(&self, r: &mut Renderer, ox: f32, oy: f32, s: f32, time: f32) {
        if !self.active() {
            return;
        }
        let shown: String = self.text().chars().take(self.visible_chars).collect();
        let (x, w) = (ox + SIDE_MARGIN * s, (VIEW_W - SIDE_MARGIN * 2.0) * s);
        let y = oy + (VIEW_H - BOTTOM_MARGIN - HEIGHT) * s;
        let h = HEIGHT * s;
        r.rect(x, y, w, h, [0.0, 0.0, 0.0, 0.82]);
        r.rect(x, y, w, 2.0 * s, [0.6, 0.6, 0.65, 0.9]);
        r.text(x + 16.0 * s, y + 16.0 * s, &shown, TEXT_SCALE * s, [1.0, 1.0, 1.0, 1.0]);
        let bottom_y = y + h - (LINE_H + 10.0) * s;
        if self.choice_ready() {
            r.text(x + 16.0 * s, bottom_y, "[Y] 수락   [N] 거부", TEXT_SCALE * s, [1.0, 0.9, 0.5, 1.0]);
        } else if self.fully_typed() && (time * 2.2).sin() > 0.0 {
            // 다 나온 평범한 줄이면 깜빡이는 화살표로 "아무 키나 눌러 계속" 신호.
            r.text(x + w - 30.0 * s, bottom_y, "v", TEXT_SCALE * s, [0.8, 0.8, 0.85, 1.0]);
        }
    }
}
