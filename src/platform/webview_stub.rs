//! Windows 가 아닌 곳의 Official Site 웹뷰 대체 구현 — WebView2 가 없으니 처음부터 실패 상태다.

pub enum WebviewInput {
    Move(f32, f32),
    Down(f32, f32),
    Up(f32, f32),
    Wheel(f32),
    Type(char),
    Backspace,
    Enter,
}

pub fn shutdown_all_and_cleanup() {}

pub struct WebviewHandle;

impl WebviewHandle {
    pub fn open(_url: String, _w: u32, _h: u32) -> WebviewHandle {
        WebviewHandle
    }

    pub fn take_frame(&self) -> Option<(u32, u32, Vec<u8>)> {
        None
    }

    pub fn failed(&self) -> bool {
        true
    }

    pub fn send_input(&self, _ev: WebviewInput) {}
}
