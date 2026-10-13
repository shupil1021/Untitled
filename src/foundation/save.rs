//! 저장/불러오기 — exe 옆의 JSON 하나(palaceos_save.json)와 설정 전용 파일(palaceos_settings.json).

use serde::{Deserialize, Serialize};

use super::*;

// 설정 + 바탕화면 상태(아이콘 위치/휴지통/잠금 해제 여부) 자동 저장.
// exe 옆에 JSON 하나로 저장하고, 실행할 때 있으면 그대로 복원한다.

use std::path::PathBuf;

// fs 필드가 FileSystem 전체(노드 배열/desktop/downloads/ever_downloaded 전부)를
// 그대로 담으므로, 이름으로 다시 찾아 재구성해야 했던
// 예전 필드들(desktop 이름 목록/downloaded/downloaded_ever/unlocked/folders 등)은
// 전부 필요 없어졌다 — 있었던 상태 그대로 저장하고 그대로 복원한다.
#[derive(Serialize, Deserialize)]
pub struct SaveData {
    pub settings: Settings,
    pub fs: FileSystem,
    pub icon_pos: Vec<(f32, f32)>, // fs.desktop 과 같은 인덱스로 짝지어진 바탕화면 좌표
    // 창을 열었다 옮기거나 크기를 바꾼 적 있으면 (파일, x, y, w, h, 최대화 여부) 로
    // 기억해둔다 — 지금 열려있는 창뿐 아니라 닫혀있는 파일도 마지막으로 뒀던 자리를
    // 그대로 들고 있어서, 다음에 다시 열면 그 자리/크기 그대로 뜬다. gfx::Rect 나
    // window_manager::WinState 를 그대로 쓰지 않고 원시 튜플로 두는 이유는 foundation
    // 모듈이 다른(더 자주 바뀌는) 모듈에 의존하지 않게 하기 위해서다(icon_pos 도 같은
    // 이유로 원시 튜플). #[serde(default)] 라 이 필드가 생기기 전에 저장된 파일도
    // 문제없이 불러와진다(창 위치는 그냥 기본 캐스케이드 위치로 뜬다).
    #[serde(default)]
    pub window_geometry: Vec<(FileId, f32, f32, f32, f32, bool)>,
}

fn save_path() -> PathBuf {
    // exe 옆에 저장 — 포터블하게 폴더 하나로 들고 다닐 수 있게.
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("palaceos_save.json")))
        .unwrap_or_else(|| PathBuf::from("palaceos_save.json"))
}

// 저장 파일이 없거나 읽기/파싱에 실패하면 None (처음 실행이거나 손상된 경우 — 기본값으로 시작).
pub fn load() -> Option<SaveData> {
    let text = std::fs::read_to_string(save_path()).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(data: &SaveData) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(save_path(), json);
    }
}

// "Erase All Memory" — 저장 파일 자체를 지운다. 실패해도(애초에 없었거나 등) 조용히
// 무시 — 어차피 호출부는 지웠다고 가정하고 부팅부터 다시 시작한다.
pub fn delete() {
    let _ = std::fs::remove_file(save_path());
}

fn settings_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("palaceos_settings.json")))
        .unwrap_or_else(|| PathBuf::from("palaceos_settings.json"))
}

// 언어 등 취향 설정은 게임 진행 저장(palaceos_save.json)과 별개 파일에 즉시
// 저장한다 — 진행 저장은 5초마다(desktop.rs::AUTOSAVE_INTERVAL) 또는 특정
// 이벤트에서만 묶어서 쓰이고, 애초에 아직 게임을 시작하지 않은 로비 화면에서는
// FileSystem 이 없어 SaveData 자체를 만들 수도 없다 — 그래서 설정 변경은 이
// 훨씬 가벼운 전용 파일로 언제 어디서 바뀌든 그 즉시 디스크에 반영한다("Erase
// All Memory" 를 눌러도 이 파일은 안 건드린다 — 진행 상황을 지운다고 언어/그래픽
// 취향까지 초기화되면 당황스러우니까).
pub fn save_settings(settings: &Settings) {
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write(settings_path(), json);
    }
}

pub fn load_settings() -> Option<Settings> {
    let text = std::fs::read_to_string(settings_path()).ok()?;
    serde_json::from_str(&text).ok()
}


