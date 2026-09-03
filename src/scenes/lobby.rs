//! 로비 화면 — 앱을 켜면 가장 먼저 뜨는 타이틀 화면. 로고 뒤로 정전기 노이즈와 이따금
//! 튀는 글리치를 깔고, 그 아래 New Start / Continue / Settings / Quit 메뉴를 띄운다.

use crate::apps::{App, AppAction, SettingsApp, WinInput};
use crate::foundation::Language;
use crate::gfx::{Rect, Renderer, ADVANCE, CELL_H};
use crate::strings::{common, lobby as s, settings, t};
use crate::ui::*;
use crate::window_manager::draw_x;

use super::boot::{LOGO, LOGO_SCALE};
use super::{BootScene, Frame, Scene, ShutdownScene, Transition};

// 아주 단순한 xorshift64 의사난수 — boot.rs 의 Rng 와 같은 용도지만, 씬마다 쓰는
// 자리가 달라서(부팅 화면은 로딩 웨이포인트, 여긴 정전기/글리치) 굳이 공유 모듈로
// 안 뽑고 각자 작게 둔다.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (self.next_u32() % 1_000_000) as f32 / 1_000_000.0 * (max - min)
    }
}

const NOISE_COUNT: usize = 220; // 프레임마다 새로 뿌리는 정전기 알갱이 개수
const GLITCH_BURST: f32 = 0.14; // 글리치가 지속되는 시간(초)
const GLITCH_GAP_MIN: f32 = 1.4; // 글리치 사이 최소 대기(초)
const GLITCH_GAP_MAX: f32 = 3.6; // 글리치 사이 최대 대기(초)

// 메뉴 항목은 로직에서 매칭 키로도 쓰이므로 영어 원문을 그대로 식별자로 남겨두고,
// 화면 표시용 문구만 menu_label() 에서 언어별로 따로 골라 돌려준다.
const MENU_ITEMS: [&str; 4] = ["New Start", "Continue", "Settings", "Quit"];
const MENU_ROW_H: f32 = 30.0;
const MENU_START_Y: f32 = 300.0;

fn menu_label(lang: Language, key: &'static str) -> &'static str {
    match key {
        "New Start" => t(lang, s::NEW_START),
        "Continue" => t(lang, s::CONTINUE),
        "Settings" => t(lang, settings::TITLE),
        "Quit" => t(lang, s::QUIT),
        _ => key,
    }
}

fn quit_opt_label(lang: Language, key: &str) -> &'static str {
    match key {
        "Yes" => t(lang, common::YES),
        _ => t(lang, common::NO),
    }
}

pub struct LobbyScene {
    t: f32,
    rng: Rng,
    glitch_timer: f32,      // 다음 글리치까지 남은 시간
    glitch_active: f32,     // 지금 글리치가 진행 중이면 남은 지속시간(> 0)
    has_save: bool,         // Continue 버튼 활성화 여부 — 저장된 진행 상태가 있는지
    settings_app: Option<SettingsApp>, // Settings 클릭 시 뜨는 오버레이 패널
    quit_confirm: bool, // Quit 을 눌렀을 때 뜨는 확인창 — 화면을 더 어둡고 불안정하게 만든다
}

impl Default for LobbyScene {
    fn default() -> Self {
        Self::new()
    }
}

impl LobbyScene {
    // 정전기/글리치는 항상 자동(설정으로 끌 수 없다) — 연출용 director 툴은
    // 이 화면 고유의 연출을 건드리는 대신 자기가 화면 위에 따로 덧그리는
    // 오버레이로 글리치/노이즈를 흉내 낸다(src/bin/director.rs 참고).
    pub fn new() -> LobbyScene {
        let mut rng = Rng::new((miniquad::date::now() * 1e6) as u64);
        let glitch_timer = rng.range_f32(GLITCH_GAP_MIN, GLITCH_GAP_MAX);
        LobbyScene {
            t: 0.0,
            rng,
            glitch_timer,
            glitch_active: 0.0,
            has_save: crate::foundation::load().is_some(),
            settings_app: None,
            quit_confirm: false,
        }
    }

    // 정전기 알갱이 + (글리치 중이면) 가로 찢김 밴드를 그린다. 로고/메뉴보다 먼저
    // 불러서 항상 뒤에 깔리게 한다.
    fn draw_noise(&mut self, r: &mut Renderer) {
        for _ in 0..NOISE_COUNT {
            let x = self.rng.range_f32(0.0, 640.0);
            let y = self.rng.range_f32(0.0, 480.0);
            let s = self.rng.range_f32(1.0, 2.0);
            let v = self.rng.range_f32(0.05, 0.35);
            r.rect(x, y, s, s, [v, v, v, 1.0]);
        }

        if self.glitch_active > 0.0 {
            let bands = 3 + (self.rng.next_u32() % 4) as usize;
            for _ in 0..bands {
                let y = self.rng.range_f32(0.0, 470.0);
                let h = self.rng.range_f32(2.0, 7.0);
                let shift = self.rng.range_f32(-18.0, 18.0);
                let w = self.rng.range_f32(120.0, 640.0);
                let x = (self.rng.range_f32(0.0, 640.0 - w * 0.3) + shift).clamp(0.0, 640.0 - w.min(640.0));
                r.rect(x, y, w.min(640.0 - x), h, [0.8, 0.95, 1.0, 0.18]); // 시안 끼가 도는 밝은 찢김
            }
        }
    }

    // 글리치 버스트를 시간에 따라 발생/진행시킨다.
    fn tick_glitch(&mut self, dt: f32) {
        if self.glitch_active > 0.0 {
            self.glitch_active -= dt;
            return;
        }
        self.glitch_timer -= dt;
        if self.glitch_timer <= 0.0 {
            self.glitch_active = GLITCH_BURST;
            self.glitch_timer = self.rng.range_f32(GLITCH_GAP_MIN, GLITCH_GAP_MAX);
        }
    }

    fn draw_logo(&mut self, r: &mut Renderer) {
        let logo_w = LOGO[0].chars().count() as f32 * ADVANCE * LOGO_SCALE;
        let logo_x = (640.0 - logo_w) / 2.0;
        let white = [0.85, 0.85, 0.85, 1.0];
        for (i, row) in LOGO.iter().enumerate() {
            // 글리치 중엔 줄마다 제각각 좌우로 흔들리게 해서 화면이 깨지는 느낌을 낸다.
            // Quit 확인창이 떠 있는 동안은 훨씬 세게, 그리고 타이머와 무관하게 매
            // 프레임 흔들어서 로고 자체가 그 순간 같이 찢어지는 것처럼 보이게 한다.
            let jitter = if self.quit_confirm {
                self.rng.range_f32(-6.0, 6.0)
            } else if self.glitch_active > 0.0 {
                self.rng.range_f32(-4.0, 4.0)
            } else {
                0.0
            };
            // boot.rs 와 같은 이유로 고정폭.
            r.text_mono(logo_x + jitter, 120.0 + i as f32 * CELL_H * LOGO_SCALE, row, LOGO_SCALE, white, ADVANCE);
        }
    }

    // 메뉴 항목의 클릭 판정 사각형(문구 폭에 맞춰 가운데 정렬). 흔들림 연출과 무관하게
    // 항상 제자리 기준으로 판정해야 클릭이 흔들림을 따라 이랬다저랬다 하지 않는다.
    fn menu_rect(r: &Renderer, i: usize, label: &str) -> Rect {
        let tw = r.text_width(label, 1.0);
        let y = MENU_START_Y + i as f32 * MENU_ROW_H;
        Rect::new(320.0 - tw / 2.0 - 10.0, y - 3.0, tw + 20.0, CELL_H + 4.0)
    }

    fn draw_menu(&mut self, r: &mut Renderer, mouse: (f32, f32), lang: Language) {
        for (i, &key) in MENU_ITEMS.iter().enumerate() {
            let label = menu_label(lang, key);
            let enabled = key != "Continue" || self.has_save;
            let rect = Self::menu_rect(r, i, label);
            // Quit 확인창이 떠 있는 동안은 클릭도 안 먹지만(update() 에서 이 메뉴
            // 처리 자체를 건너뜀), hover 강조까지 그대로 켜져 있으면 마치 뒤의
            // 메뉴가 여전히 눌리는 것처럼 보여서 헷갈린다 — hover 자체를 꺼서
            // "지금은 이 메뉴가 안 먹는다" 는 걸 시각적으로도 분명히 한다.
            let hover = enabled && !self.quit_confirm && rect.contains(mouse.0, mouse.1);
            if hover {
                r.rect(rect.x, rect.y, rect.w, rect.h, NAVY);
            }
            let color = if !enabled { [0.45, 0.45, 0.45, 1.0] } else { WHITE };
            let tw = r.text_width(label, 1.0);
            // Quit 확인창이 떠 있는 동안은 뒤에 깔린 메뉴도 로고처럼 줄마다 흔들려서
            // 글리치가 화면 전체(배경 UI 포함)에 영향을 주는 것처럼 보이게 한다.
            let jitter = if self.quit_confirm { self.rng.range_f32(-4.0, 4.0) } else { 0.0 };
            r.text(320.0 - tw / 2.0 + jitter, MENU_START_Y + i as f32 * MENU_ROW_H, label, 1.0, color);
        }
    }

    // Quit 확인창이 떠 있는 동안 화면을 더 어둡고 불안정하게 만드는 오버레이 — 평소
    // draw_noise() 는 알갱이 수/글리치가 타이머 기반이라 확인창이 떠있는 동안에도
    // 뜸했다가 말았다가 하는데, 여기선 타이머와 무관하게 매 프레임 확실히 더 어둡고
    // 화면이 실제로 찢어진 것처럼 보이는 연출을 계속 보여준다. 색은 안 쓰고(색수차
    // 느낌 대신) 흑백 톤으로만 찢김을 표현한다.
    fn draw_quit_intensify(&mut self, r: &mut Renderer) {
        r.rect(0.0, 0.0, 640.0, 480.0, [0.0, 0.0, 0.0, 0.65]); // 평소보다 어둡게

        for _ in 0..NOISE_COUNT {
            let x = self.rng.range_f32(0.0, 640.0);
            let y = self.rng.range_f32(0.0, 480.0);
            let s = self.rng.range_f32(1.0, 2.5);
            let v = self.rng.range_f32(0.08, 0.45); // 평소(0.05~0.35)보다 살짝만 밝고 굵게
            r.rect(x, y, s, s, [v, v, v, 1.0]);
        }

        // 화면이 찢어지는 느낌: 밴드마다 먼저 완전 불투명한 검은 틈(찢어져서 뒤가
        // 안 보이는 자리)을 깔고, 그 안에 좌우로 서로 다르게 어긋난 밝기의 흑백
        // 조각(색 없이 두 겹이 어긋나 보이게)을 겹쳐 그린다 — glitch_active 타이머와
        // 무관하게 매 프레임 다시 뿌려서 확인창이 떠있는 내내 계속 흔들린다.
        let bands = 4 + (self.rng.next_u32() % 4) as usize;
        for _ in 0..bands {
            let y = self.rng.range_f32(0.0, 476.0);
            let h = self.rng.range_f32(2.0, 9.0);
            r.rect(0.0, y, 640.0, h, BLACK); // 찢어진 틈

            let seg_w = self.rng.range_f32(80.0, 300.0);
            let seg_x = self.rng.range_f32(0.0, (640.0 - seg_w).max(0.0));
            let shift = self.rng.range_f32(-16.0, 16.0);
            let cx = (seg_x + shift).clamp(0.0, 640.0 - seg_w);
            let mx = (seg_x - shift).clamp(0.0, 640.0 - seg_w);
            r.rect(cx, y, seg_w, h, [0.85, 0.85, 0.85, 0.35]); // 밝은 쪽으로 어긋난 조각
            r.rect(mx, y, seg_w, h, [0.45, 0.45, 0.45, 0.28]); // 어두운 쪽으로 어긋난 조각
        }
    }

    // Quit 을 누르면 창 대화상자 대신, 로비 메뉴와 같은 언어로("New Start" 같은
    // 메뉴 항목처럼 plain 텍스트 + hover 강조) 질문을 띄운다 — 별도 박스/버튼 없이
    // 그냥 화면 자체가 로비 UI인 것처럼 "Really quit PalaceOS?" 문구 아래 Yes/No
    // 두 항목을 menu_rect 와 같은 방식으로 그린다. 문구 자체는(로고/메뉴와 달리)
    // 안 흔들리게 고정 — 흔들리는 배경 위에서 이 질문만은 또렷하게 읽혀야 한다.
    const QUIT_OPTS: [&str; 2] = ["Yes", "No"];
    const QUIT_OPT_Y: f32 = 250.0;

    fn quit_opt_rect(r: &Renderer, i: usize, label: &str) -> Rect {
        let tw = r.text_width(label, 1.0);
        let y = Self::QUIT_OPT_Y + i as f32 * MENU_ROW_H;
        Rect::new(320.0 - tw / 2.0 - 10.0, y - 3.0, tw + 20.0, CELL_H + 4.0)
    }

    fn draw_quit_confirm(&self, r: &mut Renderer, mouse: (f32, f32), lang: Language) {
        let msg = t(lang, s::CONFIRM_QUIT);
        let tw = r.text_width(msg, 1.0);
        r.text(320.0 - tw / 2.0, 190.0, msg, 1.0, WHITE);

        for (i, &key) in Self::QUIT_OPTS.iter().enumerate() {
            let label = quit_opt_label(lang, key);
            let rect = Self::quit_opt_rect(r, i, label);
            let hover = rect.contains(mouse.0, mouse.1);
            if hover {
                r.rect(rect.x, rect.y, rect.w, rect.h, NAVY);
            }
            let tw = r.text_width(label, 1.0);
            r.text(320.0 - tw / 2.0, Self::QUIT_OPT_Y + i as f32 * MENU_ROW_H, label, 1.0, WHITE);
        }
    }

    // 타이틀바 우측의 닫기(X) 버튼 자리 — window_manager.rs 의 실제 창 타이틀바와
    // 같은 자리(우측 상단)에 두어서, 창 관리자 없이 떠 있는 이 오버레이도 진짜
    // 창처럼 X 로 닫을 수 있게 한다.
    fn settings_close_btn(outer: Rect) -> Rect {
        Rect::new(outer.x + outer.w - 3.0 - 18.0, outer.y + 3.0 + 2.0, 16.0, 16.0)
    }

    // Settings 오버레이 패널 — 창 관리자 없이 SettingsApp 을 그대로 재사용한다
    // (WindowManager 가 하던 타이틀바만 직접 그려서 흉내낸다). SettingsApp 내부
    // 컨텐츠는 데스크톱의 Settings 창과 완전히 같은 코드라 그룹박스 등 최신 UI가
    // 이미 그대로 반영돼 있다 — 여기서 손보는 건 그 바깥을 감싸는 타이틀바 쪽.
    fn update_settings_panel(&mut self, f: &mut Frame) {
        let outer = Rect::new(60.0, 40.0, 520.0, 400.0);
        raised(f.r, outer.x, outer.y, outer.w, outer.h);
        f.r.rect(outer.x + 3.0, outer.y + 3.0, outer.w - 6.0, 20.0, NAVY);
        let lang = f.settings.borrow().language;
        f.r.text(outer.x + 7.0, outer.y + 4.0, t(lang, settings::TITLE), 0.9, WHITE);

        let close_btn = Self::settings_close_btn(outer);
        let close_hover = close_btn.contains(f.input.mouse.0, f.input.mouse.1);
        if close_hover {
            sunken(f.r, close_btn.x, close_btn.y, close_btn.w, close_btn.h);
        } else {
            raised(f.r, close_btn.x, close_btn.y, close_btn.w, close_btn.h);
        }
        draw_x(f.r, close_btn.x + 4.0, close_btn.y + 4.0, close_btn.h - 8.0);
        if close_hover && f.input.mouse_clicked {
            self.settings_app = None;
            return;
        }

        let content = Rect::new(outer.x + 3.0, outer.y + 25.0, outer.w - 6.0, outer.h - 28.0);

        let win = WinInput {
            mouse: f.input.mouse,
            mouse_down: f.input.mouse_down,
            // 닫기 버튼 클릭은 위에서 이미 처리했으니, 그 클릭이 아래 SettingsApp
            // 컨텐츠(예: 클릭 좌표가 겹치는 컨트롤)에 또 먹지 않게 여기서 거른다.
            mouse_clicked: f.input.mouse_clicked && !close_hover,
            focused: true,
            wheel: f.input.wheel,
            dt: f.dt,
            time: f.time,
            input: f.input,
        };
        let app = self.settings_app.as_mut().expect("update_settings_panel called without settings_app open");
        match app.update(&mut *f.ctx, f.r, f.assets, content, &win) {
            AppAction::Close => self.settings_app = None,
            AppAction::RequestErase => {
                // 설정창 안의 "Erase All Memory" — 로비에는 확인 모달을 띄워줄 데스크톱이
                // 없으니, 여기서는 바로 지우고 패널을 닫는다. 지우는 건 게임 진행
                // 상태(palaceos_save.json)뿐이다 — 언어/그래픽 취향(Settings) 은 별도
                // 파일(palaceos_settings.json)에 살고 있어서, 여기서 Settings::new() 로
                // 덮어쓰면 방금 고른 언어까지 같이 초기화돼버리는 문제가 있었다.
                crate::foundation::delete();
                self.has_save = false;
                self.settings_app = None;
            }
            _ => {}
        }
    }
}

impl Scene for LobbyScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        f.show_cursor = true; // 메뉴를 마우스로 고르니 커서가 필요하다.
        self.t += f.dt;
        self.tick_glitch(f.dt);

        f.r.rect(0.0, 0.0, 640.0, 480.0, BLACK);
        self.draw_noise(f.r);
        self.draw_logo(f.r);

        if self.settings_app.is_some() {
            self.update_settings_panel(f);
            return Transition::None;
        }

        let lang = f.settings.borrow().language;
        self.draw_menu(f.r, f.input.mouse, lang);

        // Quit 확인창이 떠 있으면 화면을 더 어둡고 불안정하게 덮은 뒤, 그 위의
        // Quit/Cancel 버튼만 입력을 받는다 — 메뉴 클릭 등 나머지는 다 막는다.
        if self.quit_confirm {
            self.draw_quit_intensify(f.r);
            self.draw_quit_confirm(f.r, f.input.mouse, lang);
            if f.input.mouse_clicked {
                for (i, &key) in Self::QUIT_OPTS.iter().enumerate() {
                    let label = quit_opt_label(lang, key);
                    if !Self::quit_opt_rect(f.r, i, label).contains(f.input.mouse.0, f.input.mouse.1) {
                        continue;
                    }
                    if key == "Yes" {
                        return Transition::Switch(Box::new(ShutdownScene::new()));
                    } else {
                        self.quit_confirm = false;
                    }
                }
            }
            return Transition::None;
        }

        if f.input.mouse_clicked {
            for (i, &key) in MENU_ITEMS.iter().enumerate() {
                let label = menu_label(lang, key);
                let enabled = key != "Continue" || self.has_save;
                if !enabled {
                    continue;
                }
                if !Self::menu_rect(f.r, i, label).contains(f.input.mouse.0, f.input.mouse.1) {
                    continue;
                }
                match key {
                    "New Start" => {
                        // 이어할 게 있어도 무시하고 처음부터 — 게임 진행 저장 파일만
                        // 지우고 평소 부팅 흐름을 그대로 탄다. 언어/그래픽 취향(Settings)
                        // 은 게임 진행과 무관한 별도 파일에 살고 있으므로 안 건드린다 —
                        // 여기서 Settings::new() 로 덮어쓰면 로비에서 방금 바꾼 언어
                        // 설정이 New Start 를 누르는 순간 English 로 되돌아가버렸다.
                        crate::foundation::delete();
                        return Transition::Switch(Box::new(BootScene::new()));
                    }
                    "Continue" => {
                        // 저장된 상태는 DesktopScene::new() 가 알아서 다시 읽어오므로
                        // 그냥 평소 부팅 흐름을 타면 된다.
                        return Transition::Switch(Box::new(BootScene::new()));
                    }
                    "Settings" => {
                        // 언어 설정이 Interface 탭 안에 있어서 로비에서도 이 탭이 보여야
                        // 한다 — 바탕화면 색상 스와치는 로비엔 안 맞지만, 그거 하나
                        // 때문에 탭 전체를 감추면 언어도 못 바꾸게 되므로 그냥 같이 둔다.
                        // has_data 는 Continue 버튼과 같은 기준(저장된 진행 상태가
                        // 있는지)으로 넘겨서, 지울 게 없을 땐 Erase All Memory 를 비활성화한다.
                        self.settings_app = Some(SettingsApp::new_with_tabs(f.settings.clone(), true, self.has_save));
                    }
                    "Quit" => {
                        self.quit_confirm = true;
                    }
                    _ => {}
                }
            }
        }

        Transition::None
    }
}
