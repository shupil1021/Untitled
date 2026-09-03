//! "Erase All Memory" 확인 후 로비 화면으로 돌아가기 전에 보여주는 삭제 연출.
//!
//! BootScene 의 BIOS POST 화면과 같은 톤(검은 화면 + 흰 콘솔 텍스트)으로 삭제 로그가
//! 찍힌다 — 실제 데이터 삭제(foundation::delete 등)는 이미 DesktopScene::update() 에서
//! 이 씬으로 넘어오기 직전에 끝난 상태고, 여기는 순수하게 연출용이다.
//!
//! 앞부분은 가짜 파일 삭제 로그가 아주 빠르게(줄당 25ms) 좌르르 쏟아지듯 스크롤되고
//! (화면에 안 들어가는 오래된 줄은 위로 밀려 사라진다), 다 쏟아진 뒤에 마무리 메시지
//! 몇 줄이 천천히 찍히면서 정리되는 두 단계 구성이다.

use crate::ui::BLACK;

use super::{Frame, LobbyScene, Scene, Transition};

const ERASE_FAST_LINES: usize = 60;
const ERASE_FAST_INTERVAL: f32 = 0.025; // 더미 삭제 로그가 좌르르 쏟아지는 속도
const ERASE_LINE_INTERVAL: f32 = 0.28;  // 마무리 메시지는 한 줄씩 천천히
const ERASE_HOLD: f32 = 0.9;            // 마지막 줄까지 나온 뒤 부팅으로 넘어가기까지 대기
const ERASE_DIRS: [&str; 6] = ["ICONS", "CACHE", "REGISTRY", "TEMP", "LOGS", "PROFILE"];
const ERASE_EXTS: [&str; 4] = ["DAT", "BIN", "TMP", "IDX"];
// 첫 줄(세이브 파일 삭제)만 실제 exe 경로를 써야 해서 따로 빼놨고, 나머지는
// 경로가 필요 없는 고정 문구라 그대로 상수로 둔다.
const ERASE_TAIL: [&str; 5] = [
    "Clearing desktop icons ... OK",
    "Emptying recycle bin ... OK",
    "Resetting settings ... OK",
    "Wiping memory ... OK",
    "Memory erased. Returning to title...",
];
const ERASE_SLOW_COUNT: usize = 1 + ERASE_TAIL.len(); // 세이브 파일 삭제 줄 + ERASE_TAIL

// 지금 실행 중인 exe 가 있는 폴더 경로 — 가짜 경로(C:\PALACEOS 등) 대신 이걸 써야
// "진짜 내 컴퓨터에서 지워지는" 느낌이 난다. 못 구하면(드문 경우) 예전처럼
// C:\PALACEOS 로 대체한다.
fn exe_dir_display() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .map(|d| d.to_string_lossy().trim_end_matches('\\').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "C:\\PALACEOS".to_string())
}

fn erase_fast_line(i: usize, root: &str) -> String {
    let dir = ERASE_DIRS[i % ERASE_DIRS.len()];
    let ext = ERASE_EXTS[(i / ERASE_DIRS.len()) % ERASE_EXTS.len()];
    format!("Deleting {root}\\{dir}\\{i:04}.{ext} ... OK")
}

fn erase_fast_end() -> f32 {
    ERASE_FAST_LINES as f32 * ERASE_FAST_INTERVAL
}

fn erase_end() -> f32 {
    erase_fast_end() + ERASE_SLOW_COUNT as f32 * ERASE_LINE_INTERVAL + ERASE_HOLD
}

// 지금까지(t 시점) 콘솔에 찍혔어야 할 줄들을 전부 만든다 — 앞부분은 빠른 더미
// 로그, 그 다음은 느린 마무리 메시지. 화면엔 이 중 마지막 몇 줄만 보여준다.
fn erase_lines_at(t: f32, root: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let fast_shown = ((t / ERASE_FAST_INTERVAL) as usize).min(ERASE_FAST_LINES);
    for i in 0..fast_shown {
        lines.push(erase_fast_line(i, root));
    }
    if t > erase_fast_end() {
        let slow_shown = (((t - erase_fast_end()) / ERASE_LINE_INTERVAL) as usize).min(ERASE_SLOW_COUNT);
        if slow_shown > 0 {
            lines.push(format!("Deleting {root}\\palaceos_save.json ... OK"));
        }
        lines.extend(ERASE_TAIL[..slow_shown.saturating_sub(1)].iter().map(|s| s.to_string()));
    }
    lines
}

pub struct EraseScene {
    t: f32,
    root: String, // exe 폴더 경로 — 씬 시작 시 한 번만 구해서 매 프레임 재조회하지 않는다
}

impl Default for EraseScene {
    fn default() -> Self {
        Self::new()
    }
}

impl EraseScene {
    pub fn new() -> EraseScene {
        EraseScene { t: 0.0, root: exe_dir_display() }
    }
}

impl Scene for EraseScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        f.show_cursor = false;
        self.t += f.dt;

        f.r.rect(0.0, 0.0, 640.0, 480.0, BLACK);
        let white = [0.85, 0.85, 0.85, 1.0];

        // 한 줄이 화면 폭을 넘으면 잘라내지 않고 진짜 콘솔처럼 다음 줄로 내려서 계속
        // 그린다 — 실제 exe 경로(특히 사용자 폴더 이름에 한글/일본어가 섞여 있으면
        // 라틴 문자보다 훨씬 넓어서 예전의 "글자 수로 딱 자르기" 방식은 640px 폭을
        // 쉽게 넘겨버렸다) 를 쓰다 보니 필요해졌다 — wrap_lines() 는 실제 렌더링
        // 폭을 재므로 언어와 무관하게 안전하다(경로엔 띄어쓰기가 거의 없어서 대부분
        // 글자 단위 폴백으로 접히는데, 그것도 wrap_lines 안에서 알아서 처리된다).
        let all_lines = erase_lines_at(self.t, &self.root);
        let wrapped: Vec<String> = all_lines.iter().flat_map(|l| crate::ui::wrap_lines(f.r, l, 1.0, 640.0 - 16.0)).collect();
        let max_visible = (480.0 / 22.0) as usize; // 화면에 들어가는 줄 수만큼만 보여주고 나머진 위로 밀려 사라진 셈
        let visible = &wrapped[wrapped.len().saturating_sub(max_visible)..];
        for (row, line) in visible.iter().enumerate() {
            f.r.text(16.0, 16.0 + row as f32 * 22.0, line, 1.0, white);
        }
        // 마무리 메시지까지 다 찍힌 뒤에는 마지막 줄 끝에서 깜빡이는 커서로
        // "콘솔이 아직 살아있는" 느낌을 준다.
        let total_target = ERASE_FAST_LINES + ERASE_SLOW_COUNT;
        if all_lines.len() == total_target && (self.t * 3.0) as i32 % 2 == 0
            && let Some(last) = visible.last()
        {
            let tw = f.r.text_width(last, 1.0);
            let row = visible.len() as f32 - 1.0;
            f.r.text(16.0 + tw + 6.0, 16.0 + row * 22.0, "_", 1.0, white);
        }

        if self.t >= erase_end() {
            return Transition::Switch(Box::new(LobbyScene::new()));
        }
        Transition::None
    }
}
