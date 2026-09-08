//! `director`(연출용 게임 화면 창)와 `director_panel`(그 옆에 따로 뜨는 조작
//! 창) 두 실행 파일이 서로 다른 창(다른 프로세스)이라 직접 함수를 부를 수
//! 없어서, 작은 JSON 파일 하나를 공유 상태로 써서 통신한다 — panel 이 버튼을
//! 누를 때마다 파일에 원하는 상태를 써두면, director 가 매 프레임 그 파일을
//! 다시 읽어서 반영한다.
//!
//! panel 의 "변수" 탭(스토리 진행 플래그 강제 변경, 개발용 디버그 기능)만은
//! 예외로 실제 게임(crackhead.exe)도 이 파일을 읽는다 — scenes/desktop.rs::
//! DesktopScene::sync_debug_vars() 가 주기적으로 set_* 필드를 확인해서 있으면
//! fs 에 반영하고 그 자리만 지운다(jump_to 를 director 가 한 번 적용한 뒤
//! 지우는 것과 같은 "일회성 명령" 요령). 그 외 필드(glitch/noise/씬 전환/녹화)는
//! 여전히 director 전용이라 crackhead.exe 는 거들떠보지 않는다.
//!
//! 파일은 두 실행 파일이 항상 같은 폴더에 같이 있다는 전제로 "exe 옆"에 둔다
//! (foundation.rs 의 save_path()/settings_path() 와 같은 요령) — 다만 이름을
//! `director_state.json` 으로 분명히 구분해서 게임 저장 파일(palaceos_save.json)
//! 이나 설정 파일(palaceos_settings.json) 과는 절대 안 섞이게 한다.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn state_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("director_state.json")))
        .unwrap_or_else(|| PathBuf::from("director_state.json"))
}

// 연출 도구는 "평소엔 깨끗한 화면, 필요할 때만 켠다"가 기본이라 glitch/noise
// 둘 다 꺼진 채로 시작한다 — 실제 게임의 로비 화면은 이 기본값과 무관하게
// 항상 자동으로 정전기/글리치가 나온다(director 전용 설정). 강도는 켰을 때
// 바로 예전(강도 조절이 생기기 전)과 같은 세기로 보이도록 기본 100% 로 둔다.
#[derive(Serialize, Deserialize, Clone)]
pub struct DirectorState {
    pub glitch: bool,
    pub noise: bool,
    // 0.0~1.0 — 각각 켜져 있을 때만 의미가 있다. 알갱이 개수/찢김 밴드 굵기·
    // 개수·밝기를 이 값에 비례해서 줄인다(director.rs::draw_overlay_effects).
    pub glitch_intensity: f32,
    pub noise_intensity: f32,
    // 0.0(뜸하게)~1.0(잦게) — 글리치가 얼마나 자주 터지는지. director.rs 가
    // 이 값을 실제 대기시간 범위(초)로 변환해서 쓴다. 정전기(noise)는 매
    // 프레임 계속 그리는 효과라 "주기" 개념 자체가 없어서 글리치에만 있다.
    pub glitch_frequency: f32,
    // panel 이 씬 전환 버튼을 누르면 여기 이름("Boot"/"Lobby"/"Desktop"/"Erase"/
    // "Shutdown")을 채워둔다 — director 가 한 번 읽어서 그 씬으로 넘어간 뒤엔
    // 다시 None 으로 써서 지운다(그래야 같은 요청이 매 프레임 반복 적용되지
    // 않는다).
    pub jump_to: Option<String>,
    // panel 의 Record 버튼 상태 — true 가 되는 순간 director 가 새 타임스탬프
    // 폴더를 만들고 매 프레임을 PNG 로 그 안에 저장하기 시작한다(director.rs
    // 의 DirectorStage::apply_director_state 참고). false 로 바뀌면 그냥
    // 저장을 멈출 뿐 폴더는 그대로 남는다.
    #[serde(default)]
    pub recording: bool,
    // panel 의 "변수" 탭 전용 — 스토리 진행 플래그를 강제로 켜거나 꺼달라는
    // 일회성 요청. Some(값) 이면 crackhead.exe(DesktopScene::sync_debug_vars)
    // 가 다음에 이 파일을 읽을 때 실제 fs 에 그 값을 반영한 뒤 다시 None 으로
    // 지운다 — jump_to 와 같은 요령(매 프레임 반복 적용되지 않게).
    #[serde(default)]
    pub set_hex_tool_installed: Option<bool>,
    #[serde(default)]
    pub set_mail_arrived: Option<bool>,
    // 재연구 업무 보고 메일의 정상/비정상 제출 횟수(FileSystem::
    // report_submissions_ok/bad) — 마찬가지로 일회성 "이 값으로 덮어써라"
    // 명령이다(상대적 +1/-1이 아니라 절대값인 이유: panel 은 이 값을 직접
    // 들고 있지 않고 매 프레임 게임 저장 파일을 읽어 보여주기만 하므로,
    // 버튼을 누른 시점의 "화면에 보이는 값" 기준으로 새 절대값을 계산해서
    // 보낸다).
    #[serde(default)]
    pub set_report_submissions_ok: Option<u32>,
    #[serde(default)]
    pub set_report_submissions_bad: Option<u32>,
}

impl Default for DirectorState {
    fn default() -> Self {
        DirectorState {
            glitch: false,
            noise: false,
            glitch_intensity: 1.0,
            noise_intensity: 1.0,
            glitch_frequency: 0.5, // 예전(주기 조절이 생기기 전) 세기와 비슷한 "중간" 정도
            jump_to: None,
            recording: false,
            set_hex_tool_installed: None,
            set_mail_arrived: None,
            set_report_submissions_ok: None,
            set_report_submissions_bad: None,
        }
    }
}

pub fn load() -> DirectorState {
    std::fs::read_to_string(state_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

// panel 이 버튼 하나 누를 때마다(글리치/노이즈/씬 전환 등, Record 와 무관한
// 것들도 전부) state 전체를 다시 저장한다 — director 는 매 프레임 이 파일을
// 다시 읽는데, 그 타이밍이 겹치면 "쓰는 도중"의 반쯤 써진 JSON 을 읽어버릴
// 수 있다. load() 는 파싱 실패 시 조용히 기본값(recording: false 포함)으로
// 대체하는데, 이러면 director 입장에선 "방금 녹화가 꺼졌다가 다시 켜졌다"로
// 보여서 아무 버튼이나 눌러도(글리치 토글이든 씬 전환이든) 녹화 폴더가
// 계속 새로 생기는 문제가 있었다. 임시 파일에 먼저 쓰고 rename 으로
// 바꿔치기하면(같은 드라이브 안에서는 원자적) 읽는 쪽은 항상 완전한 이전
// 내용이거나 완전한 새 내용만 보게 된다.
pub fn save(state: &DirectorState) {
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let path = state_path();
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

// director 가 jump_to 를 한 번 처리한 뒤 지울 때 쓴다 — 매번 전체를 다시 읽고
// 쓰는 대신, 이미 읽어둔 state 를 그대로 받아 jump_to 만 비우고 저장한다.
pub fn clear_jump(mut state: DirectorState) {
    state.jump_to = None;
    save(&state);
}

fn status_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("director_status.json")))
        .unwrap_or_else(|| PathBuf::from("director_status.json"))
}

// DirectorState 와는 반대 방향(director → panel)으로 흐르는 상태 — PNG+wav 를
// output.avi 로 합치는 동안(mux_avi) director 가 여기에 표시해두면, panel 이
// 매 프레임 이 파일을 읽어서 창 아래쪽에 "합치는 중" 표시를 띄운다. 같은
// director_state.json 에 안 넣은 이유는 그 파일은 panel 이 쓰고 director 가
// 읽는 한쪽 방향으로만 정해뒀는데, 여기에 반대 방향 쓰기까지 섞으면 두
// 프로세스가 서로 다른 시점에 같은 파일을 각자 통째로 다시 쓰다가 상대방이
// 막 쓴 내용을 덮어써버릴 수 있어서다(예: panel 이 글리치를 토글하는 사이
// director 가 muxing 상태를 썼다면, 그중 나중에 저장한 쪽이 이긴다).
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DirectorStatus {
    pub muxing: bool,
}

pub fn load_status() -> DirectorStatus {
    std::fs::read_to_string(status_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save_status(status: &DirectorStatus) {
    if let Ok(json) = serde_json::to_string_pretty(status) {
        let path = status_path();
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}
