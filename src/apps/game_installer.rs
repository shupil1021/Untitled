//! 메일로 받은 "test Setup.exe"(FileKind::GameSetup)를 열면 뜨는 설치 마법사 —
//! 진짜 설치할 건 없지만(가짜 설치) Welcome → Installing(들쭉날쭉한 진행바) →
//! Finish 세 페이지를 넘어간다. 진행바가 다 차는 순간 AppAction::InstallComplete 를
//! 한 번 돌려주는데, desktop.rs 가 받아서 fs.game_installed 를 켜고 바탕화면에
//! 게임 아이콘(test.exe, FileKind::Game)을 만든다 — 그 아이콘을 열면 게임
//! (apps/doors_game.rs)이 OS 창 안에서 뜬다. 이미 설치된 뒤 Setup.exe 를 다시
//! 열면 Welcome 부터 다시 태우지 않고 곧장 AlreadyInstalled 페이지로 연다 —
//! 중복으로 InstallComplete 를 보내 바탕화면에 아이콘이 두 개 생기는 걸 막는다.
//! (예전 팩맨 미니게임 때 쓰던 마법사를 그대로 되살려 게임 하나 전용으로 줄였다.)

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{Language, Settings};
use crate::random::{LoadCurve, Rng};
use crate::render::gfx::{Assets, Color, Rect, Renderer};
use crate::strings::{common, game_installer as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

const INSTALL_DURATION: f32 = 2.6; // 진행바가 다 차는 데 걸리는 시간(초) — 가짜 설치라 적당히 짧게
const FINISH_HOLD: f32 = 0.6; // 진행바가 100% 를 찍은 뒤 "Done." 을 잠깐 보여주는 시간(초)
const BTN_W: f32 = 74.0;
const BTN_H: f32 = 24.0;

fn status_steps(lang: Language) -> [&'static str; 5] {
    [t(lang, s::STEP_COPYING), t(lang, s::STEP_REGISTERING), t(lang, s::STEP_UPDATING), t(lang, s::STEP_VERIFYING), t(lang, s::STEP_FINALIZING)]
}

enum Page {
    AlreadyInstalled,
    Welcome,
    Installing,
    Finish,
}

pub struct GameInstallerApp {
    page: Page,
    elapsed: f32,
    progress: f32,
    finish_hold: f32,
    sent_complete: bool,
    load_curve: LoadCurve,
    settings: Rc<RefCell<Settings>>,
}

impl GameInstallerApp {
    // already_installed 면(fs.game_installed 가 이미 true) Welcome 부터 다시 태우지
    // 않고 바로 AlreadyInstalled 페이지로 연다.
    pub(super) fn new(already_installed: bool, settings: Rc<RefCell<Settings>>) -> GameInstallerApp {
                GameInstallerApp {
            page: if already_installed { Page::AlreadyInstalled } else { Page::Welcome },
            elapsed: 0.0,
            progress: 0.0,
            finish_hold: 0.0,
            sent_complete: false,
            load_curve: LoadCurve::random(&mut Rng::from_time()),
            settings,
        }
    }

    fn draw_paragraph(&self, r: &mut Renderer, x: f32, y: f32, w: f32, text: &str, color: Color) -> f32 {
        let mut ty = y;
        for para in text.split('\n') {
            if para.is_empty() {
                ty += 10.0;
                continue;
            }
            for line in crate::ui::wrap_lines(r, para, 0.8, w) {
                r.text(x, ty, &line, 0.8, color);
                ty += 17.0;
            }
        }
        ty
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_nav_row(&mut self, r: &mut Renderer, card: Rect, win: &WinInput, next_label: &str, show_cancel: bool, lang: Language) -> NavClick {
        let by = card.y + card.h - BTN_H - 12.0;
        let next_btn = Rect::new(card.x + card.w - 12.0 - BTN_W, by, BTN_W, BTN_H);
        let cancel_btn = Rect::new(next_btn.x - 8.0 - BTN_W, by, BTN_W, BTN_H);

        let mut clicked = NavClick::None;
        if show_cancel && button(r, cancel_btn.x, cancel_btn.y, cancel_btn.w, cancel_btn.h, t(lang, common::CANCEL), win) {
            clicked = NavClick::Cancel;
        }
        if button(r, next_btn.x, next_btn.y, next_btn.w, next_btn.h, next_label, win) {
            clicked = NavClick::Next;
        }
        clicked
    }
}

enum NavClick {
    None,
    Next,
    Cancel,
}

impl App for GameInstallerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        // 진짜 타이틀바가 이미 있으니 창 전체를 그대로 카드로 쓴다(installer.rs 와 같은 요령).
        let card = area;
        r.rect(card.x, card.y, card.w, card.h, FACE);
        let lang = self.settings.borrow().language;

        let page_title = match self.page {
            Page::AlreadyInstalled => t(lang, s::PAGE_ALREADY_INSTALLED),
            Page::Welcome => t(lang, s::PAGE_WELCOME),
            Page::Installing => t(lang, s::PAGE_INSTALLING),
            Page::Finish => t(lang, s::PAGE_FINISH),
        };
        r.text(card.x + 12.0, card.y + 10.0, page_title, 0.95, BLACK);
        r.rect(card.x + 10.0, card.y + 32.0, card.w - 20.0, 1.0, GRAY);

        let illus = Rect::new(card.x + 12.0, card.y + 42.0, 84.0, card.h - 42.0 - 12.0);
        r.rect(illus.x, illus.y, illus.w, illus.h, WHITE);
        border(r, illus.x, illus.y, illus.w, illus.h, GRAY);
        draw_icon(r, assets, &IconType::Exe, illus.x + illus.w / 2.0 - 26.0, illus.y + illus.h / 2.0 - 26.0, 52.0);

        let content = Rect::new(illus.x + illus.w + 14.0, card.y + 46.0, card.w - illus.w - 14.0 - 24.0, card.h - 46.0 - 12.0);

        let mut result = AppAction::None;

        match self.page {
            Page::AlreadyInstalled => {
                let y = self.draw_paragraph(r, content.x, content.y, content.w, t(lang, s::ALREADY_INSTALLED_MSG), BLACK);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_FINISH_TO_CLOSE), GRAY);

                // 이미 설치돼 있으므로 InstallComplete 를 또 보내지 않는다(보내면
                // 바탕화면에 아이콘이 하나 더 생긴다) — 그냥 창만 닫는다.
                if let NavClick::Next = self.draw_nav_row(r, card, win, t(lang, s::FINISH), false, lang) {
                    return AppAction::Close;
                }
            }
            Page::Welcome => {
                let y = self.draw_paragraph(r, content.x, content.y, content.w, t(lang, s::WELCOME_MSG), BLACK);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_INSTALL_OR_CANCEL), GRAY);

                match self.draw_nav_row(r, card, win, t(lang, s::INSTALL), true, lang) {
                    NavClick::Next => {
                        self.page = Page::Installing;
                        self.elapsed = 0.0;
                        self.progress = 0.0;
                        self.sent_complete = false;
                    }
                    NavClick::Cancel => return AppAction::Close,
                    NavClick::None => {}
                }
            }
            Page::Installing => {
                let done = self.progress >= 1.0;
                let steps = status_steps(lang);
                let status = if done { t(lang, s::DONE) } else { steps[((self.progress * steps.len() as f32) as usize).min(steps.len() - 1)] };
                let y = self.draw_paragraph(r, content.x, content.y, content.w, t(lang, s::INSTALLING_MSG), BLACK);
                let status_y = y + 10.0;
                r.text_clipped(content.x, status_y, status, 0.8, GRAY, content.w);

                if !done {
                    self.elapsed += win.dt;
                    let t_frac = (self.elapsed / INSTALL_DURATION).min(1.0);
                    self.progress = self.load_curve.sample(t_frac);
                } else {
                    if !self.sent_complete {
                        // 게이지가 다 찬 이 순간 딱 한 번만 설치 완료를 알린다 — 창은
                        // 계속 열려있다가(window_manager.rs 가 닫지 않는다) Finish
                        // 버튼을 눌러야 닫힌다.
                        self.sent_complete = true;
                        result = AppAction::InstallComplete;
                    }
                    self.finish_hold += win.dt;
                    if self.finish_hold >= FINISH_HOLD {
                        self.page = Page::Finish;
                    }
                }

                let bar_y = status_y + 28.0;
                let bar_w = content.w;
                const BAR_H: f32 = 18.0;
                sunken(r, content.x, bar_y, bar_w, BAR_H);
                let fill_w = (bar_w - 4.0) * self.progress;
                if fill_w > 0.0 {
                    r.rect(content.x + 2.0, bar_y + 2.0, fill_w, BAR_H - 4.0, NAVY);
                    for tick in 1..10 {
                        let tx = content.x + 2.0 + bar_w * tick as f32 / 10.0;
                        if tx < content.x + 2.0 + fill_w {
                            r.rect(tx, bar_y + 2.0, 1.0, BAR_H - 4.0, [0.0, 0.0, 0.3, 0.35]);
                        }
                    }
                }
                let pct = if done { "100%".to_string() } else { format!("{}%", (self.progress * 100.0) as i32) };
                r.text(content.x, bar_y + BAR_H + 6.0, &pct, 0.8, GRAY);
                // 설치 중엔 버튼 없음 — 취소도 못 하게 막는다.
            }
            Page::Finish => {
                let y = self.draw_paragraph(r, content.x, content.y, content.w, t(lang, s::FINISH_MSG), BLACK);
                // 실제로 게임 파일이 써진 위치(gamefiles.rs) — 플레이어가 탐색기로 찾아가 볼 수 있다.
                let location = format!("{} %APPDATA%\\{}", t(lang, s::INSTALLED_TO), crate::foundation::GAME_FOLDER_NAME);
                let y = self.draw_paragraph(r, content.x, y + 6.0, content.w, &location, GRAY);
                self.draw_paragraph(r, content.x, y + 8.0, content.w, t(lang, s::CLICK_FINISH_TO_CLOSE), GRAY);

                if let NavClick::Next = self.draw_nav_row(r, card, win, t(lang, s::FINISH), false, lang) {
                    return AppAction::Close;
                }
            }
        }

        result
    }
}
