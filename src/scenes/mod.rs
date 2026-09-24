//! 씬 프레임워크(Scene 트레잇, SceneManager, 입력 상태 Input/Frame) + 씬마다 나뉜 하위
//! 모듈(BootScene/EraseScene/ShutdownScene/DesktopScene 등). 그 상태(어떤 씬이 지금
//! 활성인지, 다음 씬으로 어떻게 넘어가는지)를 관리하는 공통 코드는 여기 mod.rs 에 둔다.

pub mod bluescreen;
pub mod boot;
pub mod desktop;
pub mod erase;
pub mod lobby;
pub mod shutdown;

pub use bluescreen::BlueScreenScene;
pub use boot::BootScene;
pub use desktop::DesktopScene;
pub use erase::EraseScene;
pub use lobby::LobbyScene;
pub use shutdown::ShutdownScene;

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::KeyCode;

use crate::foundation::Settings;
use crate::gfx::{Assets, Renderer};
use crate::ui::CursorKind;

// 한 프레임 동안 씬이 참조하는 것들 묶음.
pub struct Frame<'a> {
    pub ctx: &'a mut dyn miniquad::RenderingBackend,
    pub r: &'a mut Renderer,
    pub assets: &'a Assets,
    pub input: &'a Input,
    pub settings: Rc<RefCell<Settings>>,
    pub dt: f32,
    pub time: f32,
    pub cursor: CursorKind, // 씬이 상황에 맞게 갱신 — 기본은 Arrow.
    pub show_cursor: bool,  // 부팅 화면처럼 마우스가 필요 없는 씬은 false 로 끈다.
}

#[derive(Default)]
pub struct Input {
    pub mouse: (f32, f32),
    pub mouse_down: bool,
    pub mouse_clicked: bool,
    pub right_clicked: bool, // 우클릭 순간 한 프레임만 true (컨텍스트 메뉴 열기용)
    pub wheel: f32,
    // 마우스 시점(mouse look) 모드일 때 이번 프레임 동안 실제 마우스가 움직인 양
    // (실제 화면 픽셀) — main.rs 가 request_mouse_look() 요청을 받은 프레임부터
    // 커서를 화면 가운데에 계속 되돌려 놓으면서 여기에 쌓는다. 그 동안은
    // mouse(가상 커서 위치)는 멈춰 있고 커서도 안 그린다.
    pub look_delta: (f32, f32),
    pub typed: Vec<char>,
    just_pressed: Vec<KeyCode>,
    // 지금 눌려있는 키들 — OS 자동反복(repeat) 이벤트는 이미 걸러내므로(just_pressed
    // 는 처음 눌린 순간 한 번만), 계속 누르고 있는 동안 뭔가 반복시키고 싶은 곳
    // (예: Backspace 꾹 눌러 빠르게 지우기)은 이걸로 매 프레임 직접 시간을 재서
    // 자기만의 반복 속도를 만든다.
    down: std::collections::HashSet<KeyCode>,
}

impl Input {
    pub fn pressed(&self, k: KeyCode) -> bool {
        self.just_pressed.contains(&k)
    }

    // "아무 키나 누르면" 식 화면(블루스크린 등)에서 쓴다 — 이번 프레임에 새로
    // 눌린 키가 하나라도 있으면 true.
    pub fn any_key_pressed(&self) -> bool {
        !self.just_pressed.is_empty()
    }

    pub fn is_down(&self, k: KeyCode) -> bool {
        self.down.contains(&k)
    }

    pub fn on_key_down(&mut self, k: KeyCode, repeat: bool) {
        if !repeat {
            self.just_pressed.push(k);
        }
        self.down.insert(k);
    }
    pub fn on_key_up(&mut self, k: KeyCode) {
        self.down.remove(&k);
    }
    pub fn on_char(&mut self, c: char) {
        self.typed.push(c);
    }
    pub fn end_frame(&mut self) {
        self.mouse_clicked = false;
        self.right_clicked = false;
        self.wheel = 0.0;
        self.look_delta = (0.0, 0.0);
        self.just_pressed.clear();
        self.typed.clear();
    }
}

// 창 안에서 도는 3D 게임(apps/doors_game.rs)이 "이번 프레임에도 마우스 시점 모드를
// 유지해달라"고 main.rs 에 알리는 신호 — 앱이 매 프레임 다시 요청해야만 유지되고,
// main.rs 가 프레임마다 읽으면서 지운다. 그래서 창을 닫거나 최소화하거나 다른
// 창으로 포커스가 넘어가 앱이 요청을 멈추면 그 다음 프레임에 저절로 풀린다(따로
// "해제" 신호를 보낼 필요가 없다).
static MOUSE_LOOK_REQUEST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn request_mouse_look() {
    MOUSE_LOOK_REQUEST.store(true, std::sync::atomic::Ordering::Relaxed);
}

// main.rs 전용 — 이번 프레임에 요청이 있었는지 읽고 지운다.
pub fn take_mouse_look_request() -> bool {
    MOUSE_LOOK_REQUEST.swap(false, std::sync::atomic::Ordering::Relaxed)
}

pub enum Transition {
    None,
    Switch(Box<dyn Scene>),
    Quit,
}

pub trait Scene {
    // 갱신 + 그리기를 한 번에. (즉시 모드 UI 에 편하다)
    fn update(&mut self, f: &mut Frame) -> Transition;
}

pub struct SceneManager {
    current: Box<dyn Scene>,
}

impl SceneManager {
    pub fn new(start: Box<dyn Scene>) -> SceneManager {
        SceneManager { current: start }
    }
    // 씬이 스스로 Transition::Switch 를 반환하는 정상 경로 밖에서(예: 연출용
    // director 툴이 버튼 클릭으로 임의 씬을 강제로 띄울 때) 지금 씬을 즉시
    // 갈아끼운다 — 실제 게임(main.rs)은 안 쓰고 항상 정상 Transition 경로만
    // 탄다.
    pub fn set(&mut self, next: Box<dyn Scene>) {
        self.current = next;
    }
    // 종료해야 하면 true 반환.
    pub fn update(&mut self, f: &mut Frame) -> bool {
        match self.current.update(f) {
            Transition::None => false,
            Transition::Switch(next) => {
                self.current = next;
                false
            }
            Transition::Quit => true,
        }
    }
}
