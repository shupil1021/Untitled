//! 부팅 화면 — BIOS POST 흉내 → 화면 정리 → 로고/Welcome/로딩 바, 끝나면 DesktopScene 으로.

use crate::gfx::{ADVANCE, CELL_H, SCREEN_H, SCREEN_W};
use crate::strings::{boot as s, t};
use crate::ui::BLACK;

use super::{DesktopScene, Frame, Scene, Transition};

pub struct BootScene {
    t: f32,
    welcome_delay: f32,               // 로고가 뜬 뒤 Welcome 문구가 나오기까지의 랜덤 대기(초)
    load_waypoints: Vec<(f32, f32)>,  // 로딩 바가 (경과비율, 진행비율)을 들쭉날쭉하게 지나가는 지점들
}

impl Default for BootScene {
    fn default() -> Self {
        Self::new()
    }
}

impl BootScene {
    pub fn new() -> BootScene {
        let mut rng = Rng::new((miniquad::date::now() * 1e6) as u64);
        BootScene {
            t: 0.0,
            welcome_delay: rng.range_f32(0.3, 0.9),
            load_waypoints: build_load_waypoints(&mut rng),
        }
    }
}

// 로딩 바가 일정한 속도로 차지 않고 멈칫거리다 훅 튀도록, (경과비율, 진행비율) 웨이포인트를
// 랜덤하게 생성한다. (0,0) 에서 시작해 (1,1) 로 끝나며 사이 구간마다 속도가 들쭉날쭉하다.
fn build_load_waypoints(rng: &mut Rng) -> Vec<(f32, f32)> {
    const SEGMENTS: usize = 7;
    let mut xd: Vec<f32> = (0..SEGMENTS).map(|_| rng.range_f32(0.4, 1.6)).collect();
    let xsum: f32 = xd.iter().sum();
    for v in xd.iter_mut() {
        *v /= xsum;
    }
    // 진행량은 제곱을 줘서 절반은 거의 멈춘 듯 조금씩, 절반은 훅 튀도록 편차를 크게 만든다.
    let mut yd: Vec<f32> = (0..SEGMENTS).map(|_| rng.range_f32(0.05, 1.0).powf(2.0)).collect();
    let ysum: f32 = yd.iter().sum();
    for v in yd.iter_mut() {
        *v /= ysum;
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut out = vec![(0.0, 0.0)];
    for i in 0..SEGMENTS {
        x += xd[i];
        y += yd[i];
        out.push((x, y));
    }
    // 부동소수점 누적 오차로 마지막 점이 (1.0, 1.0) 에 딱 안 맞을 수 있어 강제로 맞춘다
    // (installer.rs 의 같은 함수에서 이 오차가 진행바를 99%에 멈추게 하는 버그가 있었다).
    if let Some(last) = out.last_mut() {
        *last = (1.0, 1.0);
    }
    out
}

// waypoints 사이를 선형보간해 경과비율 t(0..1) 에서의 진행비율을 구한다.
fn sample_load(waypoints: &[(f32, f32)], t: f32) -> f32 {
    for w in waypoints.windows(2) {
        let (x0, y0) = w[0];
        let (x1, y1) = w[1];
        if t <= x1 {
            let seg_t = if x1 > x0 { ((t - x0) / (x1 - x0)).clamp(0.0, 1.0) } else { 1.0 };
            return y0 + (y1 - y0) * seg_t;
        }
    }
    1.0
}

// BIOS POST 화면에 고정 딜레이로 하나씩 나타나는 줄들 (메모리 테스트 줄은 별도 애니메이션).
const POST_LINES: &[&str] = &[
    "PalaceOS BIOS v4.51PG, An Award Software, Inc.",
    "Copyright (C) 1996-2026, Award Software Inc.",
    "",
    "CPU : PalaceOS Virtual CPU 486DX2-66",
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

// 아주 단순한 xorshift64 의사난수. 암호학적 품질은 필요 없고, 매 부팅마다 다른
// 헥스덤프/대기시간을 만들어내는 연출용이라 이 정도면 충분하다.
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
    fn range(&mut self, max: u32) -> u32 {
        self.next_u32() % max
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (self.range(1_000_000) as f32 / 1_000_000.0) * (max - min)
    }
}

// "PalaceOS" 피겨렛 로고. (7줄) — LobbyScene 도 같은 로고를 쓰므로 pub(super).
pub(super) const LOGO: [&str; 7] = [
    r" ____               ___                                 _____       ____       ",
    r"/\  _`\            /\_ \                               /\  __`\    /\  _`\     ",
    r"\ \ \L\ \   __     \//\ \       __       ___      __   \ \ \/\ \   \ \,\L\_\   ",
    r" \ \ ,__/ /'__`\     \ \ \    /'__`\    /'___\  /'__`\  \ \ \ \ \   \/_\__ \   ",
    r"  \ \ \/ /\ \L\.\_    \_\ \_ /\ \L\.\_ /\ \__/ /\  __/   \ \ \_\ \    /\ \L\ \ ",
    r"   \ \_\ \ \__/.\_\   /\____\\ \__/.\_\\ \____\\ \____\   \ \_____\   \ `\____\",
    r"    \/_/  \/__/\/_/   \/____/ \/__/\/_/ \/____/ \/____/    \/_____/    \/_____/",
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
                let frac = sample_load(&self.load_waypoints, t_frac);
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
