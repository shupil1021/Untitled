//! 공유 설정값 — 해상도/FPS 선택지, 언어, 바탕화면 색 등. 설정창이 바꾸고 main 이 읽어서 반영한다.

use serde::{Deserialize, Serialize};

use super::*;

// 설정창이 바꾸고, main 이 읽어서 CRT 해상도에 반영.

// 픽셀 수 오름차순. 오프스크린 렌더 해상도만 바꾸는 값이라 4:3 이 아니어도 화면에는
// 항상 4:3 필러박스로 올바르게 나온다 (crt.rs 의 스케일링이 상쇄시켜줌) — 그래서
// 16:9/16:10 같은 실사용 모니터 해상도를 그대로 옵션에 넣어도 무방하다.
// 색수차/스캔라인만으론 부족하다는 피드백에 따라 오프스크린 렌더 해상도 자체를 낮춰
// 90년대 CRT 특유의 거친 도트 느낌을 더 강하게 살렸다. 기본값은 이 목록의 최대치(화질
// 최우선 요청에 따라).
pub const RES_OPTS: [(&str, u32, u32); 6] = [
    ("640x480", 640, 480),
    ("720x540", 720, 540),
    ("800x600", 800, 600),
    ("960x720", 960, 720),
    ("1200x900", 1200, 900),
    ("1440x1080", 1440, 1080),
];
// (표시용 라벨, 실제 목표 fps) — main.rs 의 프레임 제한 루프가 뒤 숫자를 그대로 쓴다.
// fps 값이 0 이면 "Unlimited" — 프레임 제한을 아예 걸지 않는다.
pub const FPS_OPTS: [(&str, u32); 6] =
    [("30 fps", 30), ("48 fps", 48), ("60 fps", 60), ("120 fps", 120), ("165 fps", 165), ("Unlimited", 0)];
pub const CREDITS: [&str; 4] = ["CrackHead", "KK!n0M@1o", "TIM.rsa", "gxng_m1n"];
// Graphics=화면/렌더링, Audio=사운드, Interface=조작감/UI — 탭 이름이 내용이랑
// 맞게 정리했다(이전엔 "Video" 탭인데 사운드 슬라이더만 있어서 헷갈렸다).
pub const TABS: [&str; 3] = ["Graphics", "Audio", "Interface"];
pub const OFFICIAL_SITE_URL: &str = "https://kkinomalo.com/?category=Notes";

// Interface 탭의 바탕화면 색상 선택지. (표시용 라벨, 색상)
// "Teal"/"Navy" 는 ui.rs::TEAL/NAVY(기본 바탕화면색/타이틀바색으로도 쓰이는
// 상수)와 같은 값이어야 한다 — desktop.rs 가 BG_COLORS 조회 실패 시 TEAL 을
// 기본값으로 쓰기 때문에, 리터럴을 따로 두면 둘이 몰래 어긋날 수 있어 상수를
// 그대로 참조한다.
pub const BG_COLORS: [(&str, [f32; 4]); 5] = [
    ("Teal", crate::ui::TEAL),
    ("Navy", crate::ui::NAVY),
    ("Forest", [0.0, 0.35, 0.15, 1.0]),
    ("Plum", [0.35, 0.0, 0.35, 1.0]),
    ("Charcoal", [0.15, 0.15, 0.15, 1.0]),
];

// UI 표시 언어 — strings.rs::t() 이 이걸 보고 S{en,ko,ja} 세 필드 중 하나를
// 고른다. 기본은 English(#[derive(Default)] 로 첫 variant 가 기본값이 되게 한다).
#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    En,
    Ko,
    Ja,
}

// 가짜 파일시스템의 "이름"(FileNode::name)은 find_by_name() 매칭/저장 파일 등
// 코드 전반에서 영어 문자열 그대로를 식별자로 쓰므로, 실제 데이터를 바꾸는 대신
// 화면에 보여줄 때만 이 함수를 거쳐 언어별 표시 이름으로 바꿔치기한다 — 진짜
// Windows 도 "내 컴퓨터"/"휴지통" 같은 특수 폴더는 내부적으로 로케일과 무관한
// CLSID 로 식별하고 표시 이름만 언어별로 다르게 보여주는 것과 같은 방식이다.
// Photos.tar/Setup.exe 처럼 이야기 소품으로 등장하는 실제 "파일" 이름은 실제
// OS에서도 파일명이 언어에 따라 안 바뀌므로 여기서 옮기지 않고 원문 그대로 둔다.
// File Explorer 의 고정 카테고리 탭 이름(Downloads/Desktop/Videos/Images)도
// display_name() 과 같은 이유로 원문 문자열을 그대로 드래그앤드롭 목적지 매칭
// (explorer.rs/desktop.rs 의 "Desktop"/"Downloads" 문자열 비교) 등 내부 식별자로
// 계속 쓰고, 화면에 보여줄 때만 이 함수를 거친다.
pub fn category_label<'a>(lang: Language, name: &'a str) -> std::borrow::Cow<'a, str> {
    use crate::strings::{foundation as s, t};
    match name {
        "Downloads" => std::borrow::Cow::Borrowed(t(lang, s::CAT_DOWNLOADS)),
        "Desktop" => std::borrow::Cow::Borrowed(t(lang, s::CAT_DESKTOP)),
        _ => std::borrow::Cow::Borrowed(name),
    }
}

pub fn display_name<'a>(lang: Language, name: &'a str) -> std::borrow::Cow<'a, str> {
    use crate::strings::{foundation as s, t};
    match name {
        MY_COMPUTER_NAME => std::borrow::Cow::Borrowed(t(lang, s::MY_COMPUTER)),
        RECYCLE_BIN_NAME => std::borrow::Cow::Borrowed(t(lang, s::RECYCLE_BIN)),
        "Mail" => std::borrow::Cow::Borrowed(t(lang, s::MAIL)),
        "(deleted)" => std::borrow::Cow::Borrowed(t(lang, s::DELETED)),
        _ => std::borrow::Cow::Borrowed(name),
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub res_idx: usize,
    pub fps_idx: usize,
    pub sfx: f32,
    pub bgm: f32,
    pub mp4_sound: f32, // .mp4 재생 음량 — 실제 비디오 오디오(video.rs) 볼륨과 연동됨
    pub master: f32,
    pub mute_all: bool, // 켜면 개별 슬라이더 값은 그대로 두고 전체 소리만 죽인다
    pub smooth_scroll: bool, // 휠 스크롤을 딱딱하게 즉시 이동 대신 부드럽게 따라가게
    pub chromatic_aberration: f32, // CRT 색수차(가장자리 RGB 번짐) 강도 (0=끔, 1=기본)
    pub crt_intensity: f32,         // CRT 스캔라인/새도우마스크/비네팅 전체 강도 (0=끔, 1=기본)
    pub cursor_scale: f32,          // 마우스 커서 크기
    pub bg_color_idx: usize,        // 바탕화면 색상 (BG_COLORS 인덱스)
    pub weathering: f32,            // .mp4 오디오에 입히는 "낡은 소리" 정도(로우패스+히스+크래클) — 0=원본, 1=많이 낡음
    // #[serde(default)] 로 이 필드가 없던 예전 저장 파일도 (Language::En 기본값으로)
    // 계속 불러와진다.
    #[serde(default)]
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn new() -> Settings {
        Settings {
            res_idx: 3, // 960x720 기본값 (RES_OPTS 목록의 최대치는 아니지만 화질/속도 절충점)
            fps_idx: 2, // 60 fps 기본값
            sfx: 0.8,
            bgm: 0.6,
            mp4_sound: 0.8,
            master: 1.0,
            mute_all: false,
            smooth_scroll: true,
            chromatic_aberration: 0.5, // 슬라이더 최대치까지 여유를 두려고 중간값을 기본으로
            crt_intensity: 1.0,
            cursor_scale: 0.5,
            bg_color_idx: 0,
            weathering: 0.5, // 기본값 50 — 레트로/아날로그 호러 톤에 맞춰 낡은 느낌이 확실히 배어있게
            language: Language::En, // 기본은 영어로 작동
        }
    }
}

