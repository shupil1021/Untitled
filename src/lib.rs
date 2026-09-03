//! PalaceOS 의 실제 로직(씬/앱/렌더러/파일시스템 등)을 담은 라이브러리 크레이트.
//!
//! `src/main.rs`(실제 게임)와 `src/bin/director.rs`(연출/영상 제작용 컨트롤러,
//! 특정 씬을 바로 띄우거나 로비 글리치를 껐다 켰다 할 수 있는 별도 실행 파일)
//! 둘 다 이 크레이트의 모듈을 그대로 가져다 쓴다 — 그래야 director 툴이 게임
//! 본체(main.rs)의 코드를 복사하지 않고 재사용할 수 있다. main.rs 자체는 이
//! 분리 이전과 동작이 완전히 같다(그냥 `mod X;` 선언들이 여기로 옮겨온 것뿐).

pub mod apps;
pub mod crt;
// director/director_panel(연출·영상 제작용 별개 실행 파일 두 개) 끼리만 쓰는
// 작은 파일 기반 통신 — 실제 게임(crackhead.exe)은 이 모듈을 참조하지 않는다.
pub mod director_ipc;
pub mod foundation;
pub mod gfx;
pub mod ime;
pub mod scenes;
pub mod secrets;
pub mod strings;
pub mod ui;
pub mod video;
pub mod webview;
pub mod window_manager;
