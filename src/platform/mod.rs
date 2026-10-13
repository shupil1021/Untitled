//! 운영체제 전용 기능을 감싼 모듈 묶음 — IME 조합 상태(ime), Media Foundation/WASAPI
//! 영상·오디오 재생(video), WebView2 로 실제 웹페이지를 캡처하는 창(webview).
//!
//! 세 모듈 모두 Windows 에서만 진짜 구현이 있다. macOS/Linux 에서는 같은 공개
//! 인터페이스를 가진 `*_stub.rs` 가 대신 컴파일돼서, 영상/오디오/Official Site/IME
//! 조합 표시만 빠진 채로 나머지 게임은 그대로 돌아간다.

#[cfg(windows)]
pub mod ime;
#[cfg(not(windows))]
#[path = "ime_stub.rs"]
pub mod ime;

#[cfg(windows)]
pub mod video;
#[cfg(not(windows))]
#[path = "video_stub.rs"]
pub mod video;

#[cfg(windows)]
pub mod webview;
#[cfg(not(windows))]
#[path = "webview_stub.rs"]
pub mod webview;
