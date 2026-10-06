//! 부팅 화면 — BIOS POST 흉내 → 화면 정리 → 로고/Welcome/로딩 바, 끝나면 DesktopScene 으로.

use crate::random::{LoadCurve, Rng};
use crate::render::gfx::{ADVANCE, CELL_H, SCREEN_H, SCREEN_W};
use crate::strings::{boot as s, t};
use crate::ui::BLACK;

use super::{DesktopScene, Frame, Scene, Transition};

pub struct BootScene {
    t: f32,
    welcome_delay: f32,               // 로고가 뜬 뒤 Welcome 문구가 나오기까지의 랜덤 대기(초)
    load_curve: LoadCurve,            // 로딩 바가 들쭉날쭉하게 차는 곡선
}

impl Default for BootScene {
    fn default() -> Self {
        Self::new()
    }
}

impl BootScene {
    pub fn new() -> BootScene {
        let mut rng = Rng::from_time();
        BootScene {
            t: 0.0,
            welcome_delay: rng.range_f32(0.3, 0.9),
            load_curve: LoadCurve::random(&mut rng),
        }
    }
}

// BIOS POST 화면에 고정 딜레이로 하나씩 나타나는 줄들 (메모리 테스트 줄은 별도 애니메이션).
const POST_LINES: &[&str] = &[
    "Untitled BIOS v4.51PG, An Award Software, Inc.",
    "Copyright (C) 1996-2026, Award Software Inc.",
    "",
    "CPU : Untitled Virtual CPU 486DX2-66",
    "Detecting IDE drives ...",
    "  Primary Master   : PALACE-HD01",
    "  Primary Slave    : None",
    "  Secondary Master : PALACE-CD01",
    "  Secondary Slave  : None",
    "",
    "Award Plug and Play BIOS Extension v1.0A",
    "Initializing Plug and Play Cards ... Done",
    "Verifying DMI Pool Data ........ Done",
    "",
    "Boot from Hard Disk ...",
];
const POST_LINE_INTERVAL: f32 = 0.09; // POST 줄이 하나씩 나타나는 간격
const MEM_TOTAL: u32 = 65536; // 가짜 메모리 테스트 총량(KB)
const MEM_TEST_DURATION: f32 = 0.6; // 메모리 카운터가 0→MEM_TOTAL 로 올라가는 시간
const POST_HOLD: f32 = 0.35; // 마지막 POST 줄이 뜬 뒤 잠깐 멈춤

// "Untitled" 피겨렛 로고. (7줄) — LobbyScene 도 같은 로고를 쓰므로 pub(super).
pub(super) const LOGO: [&str; 7] = [
    r"__  __               __              __       ___                __      ",
    r"/\ \/\ \             /\ \__    __    /\ \__   /\_ \              /\ \    ",
    r"\ \ \ \ \     ___    \ \ ,_\  /\_\   \ \ ,_\  \//\ \       __    \_\ \   ",
    r" \ \ \ \ \  /' _ `\   \ \ \/  \/\ \   \ \ \/    \ \ \    /'__`\  /'_` \  ",
    r"  \ \ \_\ \ /\ \/\ \   \ \ \_  \ \ \   \ \ \_    \_\ \_ /\  __/ /\ \L\ \ ",
    r"   \ \_____\\ \_\ \_\   \ \__\  \ \_\   \ \__\   /\____\\ \____\\ \___,_\",
    r"    \/_____/ \/_/\/_/    \/__/   \/_/    \/__/   \/____/ \/____/ \/__,_ /",
];
pub(super) const LOGO_SCALE: f32 = 0.62;

// 단계별 타이밍(초).
const CLEAR_HOLD: f32 = 0.15;         // 화면 정리 잠깐
const AFTER_WELCOME_GAP: f32 = 0.35;  // Welcome 문구 → 로딩 바 사이 간격
const LOAD_DURATION: f32 = 2.2;       // 로딩 바가 0%→100% 차오르는 시간
const LOAD_HOLD: f32 = 0.3;           // 로딩 바가 다 찬 뒤 데스크톱으로 넘어가기까지

// BIOS POST 단계 총 길이.
fn post_end() -> f32 {
    MEM_TEST_DURATION + POST_LINES.len() as f32 * POST_LINE_INTERVAL + POST_HOLD
}

impl Scene for BootScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        f.show_cursor = false; // 부팅 화면에서는 마우스가 필요 없으니 아예 안 보이게.
        self.t += f.dt;
        f.r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, BLACK);
        let white = [0.85, 0.85, 0.85, 1.0];

        let post_end = post_end();
        let clear_end = post_end + CLEAR_HOLD;
        let welcome_start = clear_end + self.welcome_delay;
        let load_start = welcome_start + AFTER_WELCOME_GAP;
        let load_fill_end = load_start + LOAD_DURATION;
        let final_end = load_fill_end + LOAD_HOLD;

        if self.t < post_end {
            // 1단계: BIOS POST 흉내 — 메모리 카운터가 올라가고, 장치 인식 줄이 하나씩 나타난다.
            let mem_t = (self.t / MEM_TEST_DURATION).min(1.0);
            let mem_kb = (mem_t * MEM_TOTAL as f32) as u32;
            let mem_line = if mem_t >= 1.0 {
                format!("Memory Test : {MEM_TOTAL}K OK")
            } else {
                format!("Memory Test : {mem_kb}K")
            };
            f.r.text(16.0, 10.0, &mem_line, 1.0, white);

            if self.t >= MEM_TEST_DURATION {
                let shown = (((self.t - MEM_TEST_DURATION) / POST_LINE_INTERVAL) as usize).min(POST_LINES.len());
                for (row, line) in POST_LINES[..shown].iter().enumerate() {
                    f.r.text(16.0, 10.0 + (row + 2) as f32 * 22.0, line, 1.0, white);
                }
            }
        } else if self.t < clear_end {
            // 2단계: 화면 정리. (위 rect 로 이미 지워짐)
        } else {
            // 3단계: 로고 + Welcome + 로딩 바.
            let logo_w = LOGO[0].chars().count() as f32 * ADVANCE * LOGO_SCALE;
            let logo_x = (SCREEN_W - logo_w) / 2.0;
            for (i, row) in LOGO.iter().enumerate() {
                // 여러 줄에 걸친 정렬이 필요한 아스키 아트라 고정폭으로 그린다.
                f.r.text_mono(logo_x, 120.0 + i as f32 * CELL_H * LOGO_SCALE, row, LOGO_SCALE, white, ADVANCE);
            }
            if self.t >= welcome_start {
                // BIOS POST 줄들(위)은 진짜 BIOS 화면처럼 언어와 무관하게 항상 영어로
                // 남겨두지만, 이 Welcome 문구는 OS 자체가 사용자에게 건네는 인사라
                // 진짜 Windows 처럼 시스템 언어를 따라간다.
                let lang = f.settings.borrow().language;
                let msg = t(lang, s::WELCOME);
                let mw = f.r.text_width(msg, 1.0);
                f.r.text((SCREEN_W - mw) / 2.0, 300.0, msg, 1.0, white);
            }
            if self.t >= load_start {
                const BAR_CHARS: usize = 24;
                let t_frac = ((self.t - load_start) / LOAD_DURATION).min(1.0);
                let frac = self.load_curve.sample(t_frac);
                let filled = ((frac * BAR_CHARS as f32).round() as usize).min(BAR_CHARS);
                let bar = format!(
                    "[{}{}] {:>3}%",
                    "|".repeat(filled),
                    " ".repeat(BAR_CHARS - filled),
                    (frac * 100.0).round() as u32
                );
                let bw = f.r.text_width(&bar, 1.0);
                f.r.text((SCREEN_W - bw) / 2.0, 336.0, &bar, 1.0, white);
            }
        }

        if self.t > final_end {
            return Transition::Switch(Box::new(DesktopScene::new()));
        }
        Transition::None
    }
}
