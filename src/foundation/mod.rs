//! 자주 안 바뀌는 기반 모듈들 — 가짜 파일 시스템(fs), 게임 진행 상태(progress), 공유 설정값(settings),
//! 저장/불러오기(save). 다른 곳에서는 `crate::foundation::X` 로 그대로 가져다 쓴다(아래 `pub use` 가 한곳으로
//! 다시 내보낸다). 씬 프레임워크(Scene/Frame/SceneManager)는 scenes/mod.rs 에 있다.

mod fs;
mod progress;
mod save;
mod settings;

pub use fs::*;
pub use progress::*;
pub use save::*;
pub use settings::*;
