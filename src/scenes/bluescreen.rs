//! 블루스크린(BSOD) — Windows 9x 식 "A fatal exception 0E has occurred" 화면.
//! 지금은 게임 진행 중 어디서도 자동으로 안 뜬다(연출/영상 제작 director
//! 툴에서 미리보기용으로 띄우거나, 나중에 스토리 트리거가 생기면 그때 실제
//! 게임 흐름에도 연결하면 된다) — 화면 자체와 "아무 키나 누르면 재부팅"
//! 동작만 먼저 만들어둔다.

use crate::gfx::{ADVANCE, CELL_H};
use crate::ui::border;

use super::{BootScene, Frame, Scene, Transition};

const BLUE: [f32; 4] = [0.0, 0.0, 0.55, 1.0];
const WHITE: [f32; 4] = [0.85, 0.85, 0.85, 1.0];

// 진짜 Win9x 블루스크린처럼 고정폭 글자로 줄을 맞춘다 — 언어 전환 대상이
// 아니라(이 화면 자체가 "지금 시스템이 맛이 갔다"는 각본 그대로의 텍스트라
// 원문 그대로 두는 게 더 그럴듯하다) strings.rs::t() 을 안 쓴다.
const BODY_LINES: &[&str] = &[
    "A fatal exception 0E has occurred at E01F:FC07FDFF. The",
    "current application will be terminated.",
    "",
    "*  Press any key to terminate the current application.",
    "*  Press CTRL+ALT+DEL to restart your computer. You will",
    "   lose any unsaved information in all applications.",
];

pub struct BlueScreenScene {
    t: f32,
}

impl Default for BlueScreenScene {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueScreenScene {
    pub fn new() -> BlueScreenScene {
        BlueScreenScene { t: 0.0 }
    }
}

impl Scene for BlueScreenScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        f.show_cursor = false; // 진짜 블루스크린엔 마우스 커서가 없다.
        self.t += f.dt;

        f.r.rect(0.0, 0.0, 640.0, 480.0, BLUE);

        // "Windows" 라벨 상자 — 흰 바탕에 검은 글씨, 화면 위쪽 가운데.
        let title = "Windows";
        let title_w = f.r.text_width(title, 1.0);
        let box_w = title_w + 24.0;
        let box_h = CELL_H + 10.0;
        let box_x = (640.0 - box_w) / 2.0;
        let box_y = 56.0;
        f.r.rect(box_x, box_y, box_w, box_h, WHITE);
        border(f.r, box_x, box_y, box_w, box_h, [0.05, 0.05, 0.05, 1.0]);
        f.r.text(box_x + 12.0, box_y + 5.0, title, 1.0, [0.05, 0.05, 0.05, 1.0]);

        // 본문 — 고정폭이라 여러 줄이 자간 없이 딱 맞게 정렬된다.
        let text_x = 32.0;
        let mut ty = 150.0;
        for line in BODY_LINES {
            f.r.text_mono(text_x, ty, line, 1.0, WHITE, ADVANCE);
            ty += CELL_H;
        }

        // 맨 아래 "Press any key to continue" — 0.5초 간격으로 깜빡이는 커서.
        let prompt = "Press any key to continue";
        let pw = f.r.text_width(prompt, 1.0);
        let px = (640.0 - pw - ADVANCE) / 2.0;
        f.r.text(px, 420.0, prompt, 1.0, WHITE);
        if (self.t % 1.0) < 0.5 {
            f.r.text(px + pw + 4.0, 420.0, "_", 1.0, WHITE);
        }

        // 실제 Win9x 블루스크린도 어떤 키를 누르든(재부팅 키 조합까지 포함해)
        // 결국 다시 부팅으로 이어진다 — 여기서도 아무 키(또는 클릭)나 누르면
        // 그대로 BootScene 으로 돌아간다.
        if f.input.any_key_pressed() || f.input.mouse_clicked {
            return Transition::Switch(Box::new(BootScene::new()));
        }
        Transition::None
    }
}
