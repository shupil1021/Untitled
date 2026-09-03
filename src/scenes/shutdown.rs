//! 종료 연출 — Shut Down 확정 후, 곧바로 프로세스를 끄지 않고 BIOS POST 처럼 종료
//! 로그가 한 줄씩 왼쪽 위에 찍히는 화면을 보여준다. 실제 저장은 DesktopScene::update()
//! 에서 이 씬으로 넘어오기 직전에 이미 끝난 상태라, 여기는 순수하게 연출용이고 다
//! 끝나면 Transition::Quit 을 반환해서 진짜로 종료시킨다(Stage::draw() 가 그걸 보고
//! 웹뷰 정리 + window::order_quit() 을 부른다).

use crate::gfx::{SCREEN_H, SCREEN_W};
use crate::ui::BLACK;

use super::{Frame, Scene, Transition};

const SHUTDOWN_LOG_LINE_INTERVAL: f32 = 0.12; // 종료 로그가 한 줄씩 나타나는 간격
const SHUTDOWN_LOG_HOLD: f32 = 0.5;           // 마지막 로그 줄이 뜬 뒤 종료까지 대기
const SHUTDOWN_LOG_LINES: &[&str] = &[
    "Saving desktop state ............ Done",
    "Stopping WindowManager .......... Done",
    "Closing open windows ............ Done",
    "Unmounting virtual drives ....... Done",
    "Flushing write cache ............ Done",
    "Stopping audio services ......... Done",
    "",
    "System halted.",
];

fn shutdown_log_end() -> f32 {
    SHUTDOWN_LOG_LINES.len() as f32 * SHUTDOWN_LOG_LINE_INTERVAL + SHUTDOWN_LOG_HOLD
}

pub struct ShutdownScene {
    t: f32,
}

impl Default for ShutdownScene {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownScene {
    pub fn new() -> ShutdownScene {
        ShutdownScene { t: 0.0 }
    }
}

impl Scene for ShutdownScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        f.show_cursor = false;
        self.t += f.dt;
        f.r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, BLACK);
        let white = [0.85, 0.85, 0.85, 1.0];

        let shown = ((self.t / SHUTDOWN_LOG_LINE_INTERVAL) as usize).min(SHUTDOWN_LOG_LINES.len());
        for (row, line) in SHUTDOWN_LOG_LINES[..shown].iter().enumerate() {
            f.r.text(16.0, 10.0 + row as f32 * 22.0, line, 1.0, white);
        }

        if self.t >= shutdown_log_end() {
            return Transition::Quit;
        }
        Transition::None
    }
}
