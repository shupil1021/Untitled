//! 화면 아래쪽 대화창 — 타자기처럼 한 글자씩(글자마다 살짝 다른 간격으로) 나타나는
//! 대사 줄(Line)과 `[Y] 수락 / [N] 거부` 선택지(Choice). 좌표/크기는 전부 게임 화면
//! 기준 해상도(VIEW_W x VIEW_H) 값이고, 그릴 때 배율을 곱한다.
//!
//! 건너뛰기: Space 를 0.25초 이상 누르고 있으면 대사가 바로 다 나오고, **이미 한 번 본 줄**은 저절로 넘어간다(NPC 에게
//! 다시 말을 걸 때마다 같은 말을 끝까지 넘기지 않게). 처음 보는 줄은 다 나온 채로 멈추고, 선택지는 절대 건너뛰지 않는다.

use miniquad::KeyCode;

use crate::random::Rng;
use crate::render::gfx::{Renderer, CELL_H};

use super::super::WinInput;
use super::{i18n, VIEW_H, VIEW_W};

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
    // 선택지인데 [Y]/[N] 옆에 붙는 말을 직접 정한다("[Y] 먹인다   [N] 나중에").
    Ask(String, &'static str),
}

// Space 를 이만큼(초) 이상 누르고 있어야 건너뛰기 — 한 번 톡 눌러서 줄을 넘기는 것과 구분한다.
const SKIP_HOLD_SECS: f32 = 0.25;

pub struct Dialogue {
    seen: std::collections::HashSet<String>, // 끝까지 다 나왔던 줄들 — 건너뛰기가 이것만 자동으로 넘긴다
    hold: f32,                               // Space 를 누르고 있는 시간
    marked: bool,                            // 지금 줄이 다 나온 순간을 이미 처리했다(본 줄 기록은 줄마다 한 번만)
    was_seen: bool,                          // 지금 줄이 이번에 다 나오기 전에 이미 본 줄이었다
    entries: Vec<Entry>,
    index: usize,         // 몇 번째 항목을 보여주는 중인지 — len 이면 대화 끝(안 뜸)
    visible_chars: usize, // 그 항목에서 지금까지 드러낸 글자 수
    next_char_in: f32,    // 다음 글자를 드러내기까지 남은 시간(초)
    rng: Rng,             // 글자 간격을 흔드는 난수
}

impl Dialogue {
    pub fn new() -> Dialogue {
        Dialogue { seen: std::collections::HashSet::new(), hold: 0.0, marked: false, was_seen: false, entries: Vec::new(), index: 0, visible_chars: 0, next_char_in: 0.0, rng: Rng::from_time() }
    }

    // 지금 대화에 든 항목(줄/선택지) 수 — 테스트에서 "숨은 한 줄이 붙었나"를 볼 때 쓴다.
    #[cfg(test)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn active(&self) -> bool {
        self.index < self.entries.len()
    }

    fn text(&self) -> &str {
        match self.entries.get(self.index) {
            Some(Entry::Line(s) | Entry::Choice(s) | Entry::Ask(s, _)) => s,
            None => "",
        }
    }

    fn fully_typed(&self) -> bool {
        self.visible_chars >= self.text().chars().count()
    }

    // 지금 항목이 다 타이핑된 선택지인지 — 이때만 Y/N 이 먹는다.
    pub fn choice_ready(&self) -> bool {
        matches!(self.entries.get(self.index), Some(Entry::Choice(_) | Entry::Ask(..))) && self.fully_typed()
    }

    // 대화가 떠 있는 동안의 입력 처리(선택지면 Y/N, 아니면 아무 키/클릭으로 넘기기) —
    // 이번 프레임에 선택지에 답했으면 Some(수락 여부). 대화가 안 떠 있으면 아무것도 안 한다.
    pub fn handle_input(&mut self, win: &WinInput, in_view: bool) -> Option<bool> {
        if !self.active() {
            return None;
        }
        let input = win.input;
        self.update_skip(input.is_down(KeyCode::Space), win.dt);
        if self.choice_ready() {
            let answer = if input.pressed(KeyCode::Y) {
                Some(true)
            } else if input.pressed(KeyCode::N) {
                Some(false)
            } else {
                None
            };
            if answer.is_some() {
                self.choose();
            }
            return answer;
        }
        if (input.any_key_pressed() && !input.pressed(KeyCode::Escape)) || (win.mouse_clicked && in_view) {
            self.advance();
        }
        None
    }

    // 건너뛰기 키(Space)를 누르고 있는 상태를 알려 준다 — 매 프레임 한 번.
    pub fn update_skip(&mut self, key_down: bool, dt: f32) {
        self.hold = if key_down { self.hold + dt } else { 0.0 };
    }

    fn skipping(&self) -> bool {
        self.hold >= SKIP_HOLD_SECS
    }

    pub fn start(&mut self, entries: Vec<Entry>) {
        // 대사는 한국어로 적혀 있고, 시작할 때의 언어로 번역해서 보여준다(i18n.rs).
        self.entries = entries
            .into_iter()
            .map(|e| match e {
                Entry::Line(s) => Entry::Line(i18n::tr(&s)),
                Entry::Choice(s) => Entry::Choice(i18n::tr(&s)),
                Entry::Ask(s, options) => Entry::Ask(i18n::tr(&s), i18n::tr_static(options)),
            })
            .collect();
        self.index = 0;
        self.visible_chars = 0;
        self.next_char_in = 0.0;
        self.marked = false;
    }

    fn next_entry(&mut self) {
        self.marked = false;
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
        // 건너뛰는 중이면 줄이 한꺼번에 다 나온다.
        if self.skipping() {
            self.visible_chars = full_len;
        }
        self.next_char_in -= dt;
        while self.visible_chars < full_len && self.next_char_in <= 0.0 {
            self.visible_chars += 1;
            self.next_char_in += CHAR_DELAY_MIN + self.rng.unit() * (CHAR_DELAY_MAX - CHAR_DELAY_MIN);
        }
        // 끝까지 나온 평범한 줄은 "본 줄"로 기억하고, 건너뛰는 중이면 이미 본 줄만 저절로 넘어간다.
        if self.visible_chars >= full_len
            && let Some(Entry::Line(text)) = self.entries.get(self.index)
        {
            if !self.marked {
                self.marked = true;
                self.was_seen = !self.seen.insert(text.clone());
            }
            if self.was_seen && self.skipping() {
                self.next_entry();
            }
        }
    }

    // (ox, oy) 는 게임 화면의 왼쪽 위, s 는 VIEW 기준 → 실제 화면 배율.
    pub fn draw(&self, r: &mut Renderer, ox: f32, oy: f32, s: f32, time: f32) {
        if !self.active() {
            return;
        }
        let (x, w) = (ox + SIDE_MARGIN * s, (VIEW_W - SIDE_MARGIN * 2.0) * s);
        let y = oy + (VIEW_H - BOTTOM_MARGIN - HEIGHT) * s;
        let h = HEIGHT * s;
        r.rect(x, y, w, h, [0.0, 0.0, 0.0, 0.82]);
        r.rect(x, y, w, 2.0 * s, [0.6, 0.6, 0.65, 0.9]);
        let bottom_y = y + h - (LINE_H + 10.0) * s;
        // 긴 대사는 상자 폭에 맞춰 접고, 그래도 높이에 안 들어가면 글자를 조금씩 줄인다(상자 밖으로 안 튀어나가게).
        // 줄바꿈은 전체 글 기준으로 먼저 정하고 글자만 차례로 드러내서, 타자 도중에 단어가 이리저리 옮겨 다니지 않는다.
        let max_w = w - 32.0 * s;
        let zone_h = bottom_y - (y + 16.0 * s) - 2.0 * s;
        let mut k = TEXT_SCALE;
        let mut lines = crate::ui::wrap_lines(r, self.text(), k * s, max_w);
        while lines.len() as f32 * CELL_H * k * s > zone_h && k > 0.8 {
            k -= 0.1;
            lines = crate::ui::wrap_lines(r, self.text(), k * s, max_w);
        }
        let mut left = self.visible_chars;
        for (i, line) in lines.iter().enumerate() {
            if left == 0 {
                break;
            }
            let n = line.chars().count();
            let shown: String = line.chars().take(left).collect();
            left -= n.min(left);
            r.text(x + 16.0 * s, y + 16.0 * s + i as f32 * CELL_H * k * s, &shown, k * s, [1.0, 1.0, 1.0, 1.0]);
        }
        if self.choice_ready() {
            let options = match self.entries.get(self.index) {
                Some(Entry::Ask(_, options)) => options,
                _ => i18n::tr_static("[Y] 수락   [N] 거부"),
            };
            r.text(x + 16.0 * s, bottom_y, options, TEXT_SCALE * s, [1.0, 0.9, 0.5, 1.0]);
        } else if self.fully_typed() && (time * 2.2).sin() > 0.0 {
            // 다 나온 평범한 줄이면 깜빡이는 화살표로 "아무 키나 눌러 계속" 신호.
            r.text(x + w - 30.0 * s, bottom_y, "v", TEXT_SCALE * s, [0.8, 0.8, 0.85, 1.0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(texts: &[&str]) -> Vec<Entry> {
        texts.iter().map(|t| Entry::Line(t.to_string())).collect()
    }

    // 대사를 처음부터 끝까지 읽는다(키를 눌러 한 줄씩 넘긴다).
    fn read_through(d: &mut Dialogue) {
        for _ in 0..50 {
            if !d.active() {
                break;
            }
            for _ in 0..200 {
                d.tick(0.05);
            }
            d.advance();
        }
    }

    #[test]
    fn seen_lines_skip_themselves_while_the_key_is_held_but_new_lines_stop() {
        let mut d = Dialogue::new();
        d.start(lines(&["안녕.", "또 왔네."]));
        read_through(&mut d);
        assert!(!d.active());

        // 두 줄은 이미 봤고, 세 번째는 처음 보는 줄이다.
        d.start(lines(&["안녕.", "또 왔네.", "처음 듣는 말."]));
        d.update_skip(true, 0.5);
        for _ in 0..20 {
            d.tick(0.016); // 처음 보는 줄에서 멈춘 뒤로도 계속 시간이 흘러도 넘어가지 않는다
        }
        assert!(d.active());
        assert_eq!(d.text(), "처음 듣는 말.", "본 줄은 저절로 넘어가고 처음 보는 줄에서 멈춘다");
        assert!(d.fully_typed(), "멈춘 줄은 이미 다 나와 있다(읽을 수 있게)");
    }

    #[test]
    fn a_quick_tap_is_not_a_skip() {
        let mut d = Dialogue::new();
        d.start(lines(&["안녕."]));
        read_through(&mut d);
        d.start(lines(&["안녕.", "둘째."]));
        d.update_skip(true, SKIP_HOLD_SECS / 2.0); // 짧게만 눌렀다
        d.tick(0.016);
        assert_eq!(d.text(), "안녕.", "짧게 눌러서는 줄이 저절로 넘어가지 않는다");
        d.update_skip(false, 0.016);
        assert!(!d.skipping(), "키를 떼면 건너뛰기가 풀린다");
    }

    #[test]
    fn choices_are_never_skipped() {
        let mut d = Dialogue::new();
        d.start(lines(&["부탁이 있어."]));
        read_through(&mut d); // 첫 줄은 이미 본 줄이 된다
        d.start(vec![Entry::Line("부탁이 있어.".to_string()), Entry::Choice("들어줄래?".to_string())]);
        d.update_skip(true, 1.0);
        for _ in 0..10 {
            d.tick(0.016);
        }
        assert!(d.active(), "선택지에서 멈춘다");
        assert!(d.choice_ready(), "선택지는 다 나온 채 답을 기다린다");
    }

    #[test]
    fn unseen_lines_are_remembered_after_they_finish() {
        let mut d = Dialogue::new();
        d.start(lines(&["새 대사."]));
        for _ in 0..200 {
            d.tick(0.05);
        }
        assert!(d.seen.contains("새 대사."));
    }
}
