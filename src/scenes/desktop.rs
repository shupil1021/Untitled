//! 데스크톱 씬 (아이콘/작업표시줄/시작메뉴/컨텍스트메뉴/와이파이/시스템메시지패널).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::apps::{
    ensure_photos_selected, explorer_app_for_folder, explorer_app_refreshed, mail_attachable_files, open, refresh_photos_feed, CreditsApp,
    ExplorerApp, ExplorerLocation, MailApp, MoveDest, OfficialSiteApp, Opened, PhotoViewerApp, SettingsApp,
};
use crate::foundation::{display_name, FileId, FileKind, FileOrigin, FileSystem, Language, SentMail, Settings, MY_COMPUTER_NAME, OFFICIAL_SITE_URL, RECYCLE_BIN_NAME};
use crate::gfx::{Assets, Rect, Renderer, CELL_H, SCREEN_H, SCREEN_W};
use crate::secrets;
use crate::strings::{common, credits, desktop as s, explorer, official_site, settings, t};
use crate::ui::*;
use crate::window_manager::{DeskAction, Gui, WindowManager};

use super::{EraseScene, Frame, Scene, ShutdownScene, Transition};

// 아주 단순한 xorshift64 의사난수 — lobby.rs/boot.rs 등과 같은 용도지만, 씬마다
// 쓰는 자리가 달라서 공유 모듈로 안 뽑고 각자 작게 둔다(이 프로젝트 관례).
// Photos 아이콘의 랜덤 글리치 타이밍에 쓴다.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (self.next_u32() % 1_000_000) as f32 / 1_000_000.0 * (max - min)
    }
}

// Photos("?????"로 보이는, 일부러 깨뜨린 이름) 아이콘에 이따금 짧게 스치는
// 색수차 글리치 — 이 앱 자체가 훼손된 폐 건물 사진들을 다루는 컨셉이라, 그
// 아이콘도 가끔 화면이 잠깐 지지직거리는 것처럼 보이게 했다.
const ICON_GLITCH_BURST: f32 = 0.18; // 글리치가 지속되는 시간(초)
const ICON_GLITCH_GAP_MIN: f32 = 4.0; // 글리치 사이 최소 대기(초)
const ICON_GLITCH_GAP_MAX: f32 = 11.0; // 글리치 사이 최대 대기(초)

const TASKBAR_H: f32 = 28.0;
const IC_SIZE: f32 = 26.0; // 실제 그려지는 아이콘 텍스처 크기
const IC_W: f32 = 64.0;
const IC_H: f32 = 60.0;
const TILE_W: f32 = 66.0;
// draw_one_icon 이 아이콘/글자 위치를 잡는 데 쓰는 기본값들 — 실제 그려지는 아이콘
// 크기(Tar/Installer 는 IC_SIZE 보다 크다)와 무관하게 항상 이 값들을 기준으로
// 렌더링해서, 아이콘 종류가 뭐든 글자 시작 위치가 흔들리지 않는다.
//
// 2단계(아이콘 개수 더 늘리기): 아이콘 크기(IC_SIZE)와 글자 크기(LABEL_TEXT_SCALE)는
// 그대로 두고, 이 여백들(위/글자간격/아래)만 3~4px 에서 1px 로 바짝 줄여서 한 행을
// 더 욱여넣었다 — 세로 칸 수는 이미 640x480 안에서 거의 다 쓰고 있어서(1단계에서
// 6행까지 늘린 뒤엔 여유가 3px 정도만 남음), 아이콘/글자 자체가 아니라 그 사이의
// "빈 틈"을 줄이는 것만으로 한 행을 더 만들었다. 가로(9열)는 이미 "HexTool
// Setup.exe" 같은 긴 이름이 칸 폭(LABEL_MAX_W)에 거의 딱 맞게 걸쳐 있어서(1px도
// 안 남음) 더 줄이면 그 이름만 글자가 잘려 보일 위험이 있어 이번엔 손 안 댔다.
const ICON_AREA_TOP: f32 = 1.0;  // 타일 위쪽에서 아이콘 영역 시작까지 여백
const ICON_BASE_SIZE: f32 = IC_SIZE; // 아이콘 영역의 기준 크기 — 실제 아이콘은 이 중심에 맞춰 그려짐
const LABEL_GAP: f32 = 1.0;      // 아이콘 영역 바로 아래 ~ 글자 시작까지 간격
const LABEL_TEXT_SCALE: f32 = 0.7;
const LABEL_LINE_H: f32 = CELL_H * LABEL_TEXT_SCALE + 2.0; // wrap_two_lines 가 최대 2줄까지 접는 글자 한 줄 높이
const LABEL_MAX_LINES: f32 = 2.0;
// 글자를 감쌀 때 기준으로 삼는 최대 너비 — 옆 타일까지의 간격(TILE_W)보다 확실히
// 좁게 잡아서 옆 칸과 최소한의 여백이 항상 남도록 한다.
const LABEL_MAX_W: f32 = TILE_W - 8.0;
const TILE_BOTTOM_PAD: f32 = 1.0; // 이 타일의 마지막 글자 줄 ~ 다음 행 위쪽 여백까지 사이 여백
// 타일 높이는 위 기본값들로 실제로 필요한 만큼(아이콘 영역 + 글자 최대 2줄 + 아래
// 여백)을 계산해서 정한다 — 손으로 맞춘 숫자 대신 항상 실제 레이아웃과 일치한다.
const TILE_H: f32 = ICON_AREA_TOP + ICON_BASE_SIZE + LABEL_GAP + LABEL_LINE_H * LABEL_MAX_LINES + TILE_BOTTOM_PAD;
const GRID_ORIGIN: f32 = 6.0; // 1단계엔 12px 였다 — 화면 가장자리 여백도 조금 줄여서 한 행분을 더 벌었다.
const GRID_MAX_COL: i32 = 8; // 열 0..=8 (9열)
// 1단계에서 6행까지 늘렸는데, 위 여백들을 바짝 줄여서(TILE_H 63.8px) 7행까지
// 640x480 안에 들어간다(작업표시줄 위까지 448.8px 사용, 약 3px 여유) — 더 늘리고
// 싶으면 이제부턴 여백을 더 줄이는 정도로는 안 되고 아이콘/글자 크기 자체를
// 건드려야 한다.
const GRID_MAX_ROW: i32 = 6; // 행 0..=6 (7행)
// 시작메뉴/우클릭메뉴 항목은 인덱스로만 분기하므로(run_menu_action) 이 배열은
// 순전히 폭 계산용 영어 원문이고, 실제로 그릴 땐 menu_item_label() 로 언어별
// 문구를 따로 골라 쓴다.
const MENU_ITEMS: [&str; 4] = ["Official Site", "Credits", "Settings", "Shut Down"];
// 빈 바탕화면 우클릭 메뉴 — 시작 메뉴와 항목이 같지만 Shut Down 은 뺐다.
const CONTEXT_MENU_ITEMS: [&str; 3] = ["Official Site", "Credits", "Settings"];

fn menu_item_label(lang: Language, key: &str) -> &'static str {
    match key {
        "Official Site" => t(lang, official_site::TITLE),
        "Credits" => t(lang, credits::TITLE),
        "Settings" => t(lang, settings::TITLE),
        "Shut Down" => t(lang, s::SHUT_DOWN),
        _ => "",
    }
}
// 텍스트(CELL_H*스케일)가 줄 안에 딱 맞아서 위/아래 여백이 같아지도록 맞춘 값 —
// 이게 안 맞으면 구분선이 위/아래 글자와 간격이 다르게 보인다.
const CTX_TEXT_SCALE: f32 = 0.75;
const CTX_ROW_H: f32 = CELL_H * CTX_TEXT_SCALE + 4.0;

// 설정/크레딧 창을 구분하는 특수 FileId (실제 파일 아님).
const SETTINGS_WIN: FileId = usize::MAX - 1;
const CREDITS_WIN: FileId = usize::MAX - 2;
const OFFICIAL_SITE_WIN: FileId = usize::MAX - 3;

// Photos 피드에서 미리보기로 연 사진 창을 파일명별로 구분하는 가짜 FileId 대역의 시작점.
// 진짜 fs.nodes 인덱스(0부터 시작, 지금 최대 수백 개)와도, CREDITS_WIN/OFFICIAL_SITE_WIN
// (usize::MAX 바로 밑)과도 절대 안 겹치는 자리에 잡아서 안전하다. 같은 파일명은 항상
// 같은 가짜 id 로 해시되므로 wm.open() 의 기존 dedup(같은 id 면 새로 안 열고 기존 창을
// 앞으로 당김) 이 "Photos 피드 안에서" 만 그대로 작동한다 — My Computer 쪽 진짜 FileId
// 와는 절대 안 겹치니 서로 독립된 창이라는 원래 설계는 그대로 유지된다.
const PHOTO_PREVIEW_WIN_BASE: FileId = usize::MAX / 2;

fn photo_preview_win_id(filename: &str) -> FileId {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    filename.hash(&mut h);
    PHOTO_PREVIEW_WIN_BASE + (h.finish() as usize % 10_000_000)
}

// 작업표시줄 와이파이 아이콘용 — 실제 이 PC 의 인터넷 연결 상태를 물어본다.
// (와이파이인지 유선인지까지는 구분 안 하고, 그냥 "연결돼 있는지"만 확인)
fn network_connected() -> bool {
    use windows::Win32::Networking::WinInet::{InternetGetConnectedState, INTERNET_CONNECTION};
    let mut flags = INTERNET_CONNECTION(0);
    unsafe { InternetGetConnectedState(&mut flags, 0) }.is_ok()
}

// 와이파이 아이콘을 눌렀을 때 보여줄 상세 정보. 아이콘을 열 때 한 번만 조회한다
// (WLAN 조회는 매 프레임 부를 만큼 가볍지 않다).
struct WifiInfo {
    ssid: Option<String>,
    ip: Option<String>,
}

fn query_wifi_info(connected: bool) -> WifiInfo {
    WifiInfo { ssid: if connected { current_ssid() } else { None }, ip: local_ip() }
}

// 소켓을 직접 열어서 IP 를 알아내면(UDP connect 트릭 등) 처음 실행할 때 Windows
// 방화벽이 "이 앱의 네트워크 접근을 허용할까요?" 팝업을 띄운다 — 소켓/실제 통신을
// 아예 안 하고 OS 가 이미 들고 있는 어댑터 설정을 순수 조회만 하는 IP Helper API
// (GetAdaptersAddresses) 를 쓰면 그 팝업 자체가 안 뜬다.
fn local_ip() -> Option<String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
        IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};
    unsafe {
        let family = AF_INET.0 as u32;
        let flags = GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size = 0u32;
        // 첫 호출은 필요한 버퍼 크기만 알아내려는 것 — 항상 실패(버퍼 부족)한다.
        let _ = GetAdaptersAddresses(family, flags, None, None, &mut size);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let list = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;
        if GetAdaptersAddresses(family, flags, None, Some(list), &mut size) != 0 {
            return None;
        }
        let mut cur = list;
        while !cur.is_null() {
            let adapter = &*cur;
            if adapter.OperStatus == IfOperStatusUp {
                let mut ua = adapter.FirstUnicastAddress;
                while !ua.is_null() {
                    let sa = (*ua).Address.lpSockaddr;
                    if !sa.is_null() && (*sa).sa_family == AF_INET {
                        let sin = sa as *const SOCKADDR_IN;
                        let b = (*sin).sin_addr.S_un.S_un_b;
                        return Some(format!("{}.{}.{}.{}", b.s_b1, b.s_b2, b.s_b3, b.s_b4));
                    }
                    ua = (*ua).Next;
                }
            }
            cur = adapter.Next;
        }
        None
    }
}

// 현재 연결된 와이파이 인터페이스의 SSID 를 WLAN API 로 조회. 실패하거나 와이파이가
// 아니면(유선 등) None.
fn current_ssid() -> Option<String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::WiFi::*;
    unsafe {
        let mut handle = HANDLE::default();
        let mut negotiated = 0u32;
        if WlanOpenHandle(2, None, &mut negotiated, &mut handle) != 0 {
            return None;
        }
        let mut list_ptr: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        if WlanEnumInterfaces(handle, None, &mut list_ptr) != 0 || list_ptr.is_null() {
            let _ = WlanCloseHandle(handle, None);
            return None;
        }
        let n = (*list_ptr).dwNumberOfItems as usize;
        let infos = std::slice::from_raw_parts((*list_ptr).InterfaceInfo.as_ptr(), n);
        let mut ssid = None;
        for info in infos {
            let mut data_size = 0u32;
            let mut data_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let ok = WlanQueryInterface(
                handle,
                &info.InterfaceGuid,
                wlan_intf_opcode_current_connection,
                None,
                &mut data_size,
                &mut data_ptr,
                None,
            );
            if ok == 0 && !data_ptr.is_null() {
                let attrs = &*(data_ptr as *const WLAN_CONNECTION_ATTRIBUTES);
                let raw = attrs.wlanAssociationAttributes.dot11Ssid;
                let len = (raw.uSSIDLength as usize).min(32);
                if len > 0 {
                    ssid = String::from_utf8(raw.ucSSID[..len].to_vec()).ok();
                }
                WlanFreeMemory(data_ptr);
                if ssid.is_some() {
                    break;
                }
            }
        }
        WlanFreeMemory(list_ptr as *const _);
        let _ = WlanCloseHandle(handle, None);
        ssid
    }
}

struct IconDrag {
    start: (f32, f32),                // 드래그 시작 시점 마우스 좌표
    group_start: Vec<(usize, (f32, f32))>, // 함께 움직일 선택된 아이콘들의 (인덱스, 시작 위치).
    // 눌렀던 아이콘의 원래 좌상단에서 클릭 지점까지의 오프셋 — 고스트 미리보기가
    // "잡은 지점" 그대로 커서를 따라오게 하는 데 쓴다.
    offset: (f32, f32),
    moved: bool,
}

pub struct DesktopScene {
    fs: FileSystem,
    wm: WindowManager,
    start_open: bool,
    selected: Vec<usize>,
    last_idx: i32,
    last_click: f32,
    icon_pos: Vec<(f32, f32)>, // fs.desktop 과 같은 인덱스
    drag: Option<IconDrag>,
    marquee_start: Option<(f32, f32)>, // 빈 바탕화면 드래그로 여러 아이콘을 선택하는 고무줄 박스 시작점
    context_menu: Option<(f32, f32)>,  // 빈 바탕화면 우클릭 메뉴가 열려있으면 그 위치
    wifi_connected: bool,     // 실제 인터넷 연결 상태 (주기적으로 다시 검사)
    wifi_check_timer: f32,
    wifi_info: Option<WifiInfo>, // 와이파이 아이콘을 눌러서 연 정보 패널 (열 때 한 번만 조회)
    prev_down: bool,
    secret_unlocked: bool, // Photos.lock 이 풀렸는지 (저장/복원용)
    save_timer: f32,       // 자동 저장 주기 타이머
    erase_confirm: bool,   // "Erase All Memory" 확인창 — 화면 전체(다른 창 포함)를 덮는 진짜 모달
    idiot_confirm: bool, // Photos("?????") 아이콘을 휴지통에 넣으려 하면 뜨는 "Are You idiot?" 모달
    mail_timer: f32,      // 첫 메일이 도착할 때까지 세는 타이머 (도착하면 더 안 씀)
    toast: Option<(String, String)>, // 우측 하단에 잠깐 뜨는 알림(발신자, 제목) — 없으면 안 보임
    toast_timer: f32,                // 위 알림이 사라지기까지 남은 시간
    // 창을 열었다 옮기거나 크기를 바꾼 적 있으면 마지막 자리를 여기 기억해둔다(파일
    // ID 로 키) — 지금 열려있는 창은 매 프레임 wm 에서 값을 다시 읽어와 갱신하고,
    // 닫힌 파일의 항목은 다음에 다시 열 때까지 그대로 남아있는다. write_save() 가
    // 이걸 그대로 저장한다.
    window_geometry: HashMap<FileId, (Rect, bool)>,
    icon_glitch_rng: Rng,
    icon_glitch_timer: f32,  // 다음 아이콘 글리치까지 남은 시간
    icon_glitch_active: f32, // 지금 글리치가 진행 중이면 남은 지속시간(> 0)
    icon_glitch_offset: f32, // 이번 버스트의 색 채널 어긋남 폭(버스트 시작 때 한 번만 뽑음)
}

const AUTOSAVE_INTERVAL: f32 = 5.0; // 이 주기(초)마다 설정/바탕화면 상태를 자동 저장한다.
const MAIL_ARRIVAL_DELAY: f32 = 5.0; // 데스크톱에 들어오고 이만큼 지나면 첫 메일(입사 안내)이 도착한다.
const MAIL_AUTO_ARRIVE: bool = true;
const TOAST_DURATION: f32 = 5.0; // 우측 하단 알림이 떠있는 시간(초)
// 재연구 업무 메일이 말하는 "회사 이메일" — 입사 안내 메일을 보낸 그 주소로
// 그대로 보고를 보내는 것으로 취급한다. 여기로 normalImage 가 아닌 사진을
// 첨부해 보내면 ????? 피드가 새로 갱신된다(DeskAction::SendNewMail 참고).
const REPORT_EMAIL: &str = "test@mail.com";

impl Default for DesktopScene {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopScene {
    pub fn new() -> DesktopScene {
        // 저장 파일엔 FileSystem 전체가 있었던 그대로 스냅샷돼 있다 — 이름으로 하나하나
        // 다시 찾아 재구성하지 않고 그대로 복원하므로 복원 순서를 신경 쓸 필요가 없다.
        let save = crate::foundation::load();
        let (mut fs, mut icon_pos, window_geometry) = match save {
            Some(save) => {
                let wg = save
                    .window_geometry
                    .iter()
                    .map(|&(fid, x, y, w, h, maximized)| (fid, (Rect::new(x, y, w, h), maximized)))
                    .collect();
                (save.fs, save.icon_pos, wg)
            }
            None => {
                let fs = FileSystem::new();
                let icon_pos = (0..fs.desktop.len()).map(Self::grid_pos).collect();
                (fs, icon_pos, HashMap::new())
            }
        };
        // ?????(Photos) 이 처음 열려도 곧장 랜덤 셔플 없이 보여줄 수 있도록, 새
        // 게임이든 예전 저장 파일이든 여기서 미리 한 번 뽑아둔다(이미 있으면
        // 그대로 둔다 — ensure_photos_selected() 자체가 멱등).
        ensure_photos_selected(&mut fs);
        // icon_pos 는 항상 fs.desktop 과 같은 길이여야 한다 — 수동으로 손댄 저장
        // 파일 등으로 길이가 어긋나 있으면, 모자란 만큼 기본 격자 위치로 채운다.
        while icon_pos.len() < fs.desktop.len() {
            icon_pos.push(Self::grid_pos(icon_pos.len()));
        }
        icon_pos.truncate(fs.desktop.len());
        // Photos.lock 이 풀려서 폴더로 바뀌었는지는 이제 별도 플래그 없이 fs 스냅샷
        // 자체(이름이 이미 "Photos" 로 바뀌어 있는지)로 판단한다.
        let unlocked = fs.find_by_name("Photos").is_some();
        let mut icon_glitch_rng = Rng::new((miniquad::date::now() * 1e6) as u64);
        let icon_glitch_timer = icon_glitch_rng.range_f32(ICON_GLITCH_GAP_MIN, ICON_GLITCH_GAP_MAX);

        DesktopScene {
            fs,
            wm: WindowManager::new(),
            start_open: false,
            selected: Vec::new(),
            last_idx: -1,
            last_click: -1.0,
            icon_pos,
            drag: None,
            marquee_start: None,
            context_menu: None,
            wifi_connected: network_connected(), // 시작하자마자 한 번은 실제 상태로 초기화
            wifi_check_timer: 0.0,
            wifi_info: None,
            prev_down: false,
            secret_unlocked: unlocked,
            save_timer: 0.0,
            erase_confirm: false,
            idiot_confirm: false,
            mail_timer: 0.0,
            toast: None,
            toast_timer: 0.0,
            window_geometry,
            icon_glitch_rng,
            icon_glitch_timer,
            icon_glitch_active: 0.0,
            icon_glitch_offset: 0.0,
        }
    }

    // File Explorer 가 지금 열려있으면 내용을 새로고침한다 — 다운로드/잠금해제처럼
    // fs 내용이 바뀌었는데 이미 열려있는 창은 스냅샷이 그대로라 반영 안 되는 경우에
    // 쓴다. 3초마다 도는 주기 새로고침에서도 그대로 쓰이므로, 지금 보고 있던 탭/
    // 하위 폴더 위치를 먼저 downcast 로 읽어뒀다가 새로고침 뒤에도 그 자리 그대로
    // 있도록 explorer_app_refreshed 로 되돌려준다 — 안 그러면 항상 첫 탭(Downloads)
    // 으로 튕겨서, 다른 탭을 보던 중에 새로고침이 일어날 때마다 화면이 튀어보인다.
    fn refresh_explorer_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME)
            && self.wm.is_open(explorer_id)
        {
            let loc = self
                .wm
                .app_mut(explorer_id)
                .and_then(|app| app.as_any_mut().downcast_mut::<ExplorerApp>())
                .and_then(|app| app.current_location());
            let app = explorer_app_refreshed(&self.fs, explorer_id, loc, settings);
            self.wm.refresh_app(explorer_id, app);
        }
    }

    // 휴지통이 지금 열려있으면(별개의 RecycleBinApp 창) 내용을 새로고침한다 — Empty
    // Recycle Bin 을 눌러 fs 가 바뀐 뒤에도 그 창은 여는 시점의 스냅샷 그대로라
    // 반영이 안 되는 걸 막는다. RecycleBinApp 은 ExplorerApp 과 달리 탭/위치 상태가
    // 없어서(그냥 목록 하나) open() 으로 통째로 다시 만들면 그만이다.
    fn refresh_recycle_bin_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(bin_id) = self.fs.find_by_name(RECYCLE_BIN_NAME)
            && self.wm.is_open(bin_id)
        {
            let op = open(&self.fs, bin_id, settings);
            self.wm.refresh_app(bin_id, op.app);
        }
    }

    // Mail 이 지금 열려있으면 내용을 새로고침한다 — 첫 메일이 막 도착했는데 이미
    // 열려있던 (그래서 빈 편지함 스냅샷인) 창엔 반영이 안 되는 경우에 쓴다. 골라둔
    // 메시지가 있었으면(주기 새로고침 도중에도) 그 선택을 그대로 이어간다.
    fn refresh_mail_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(mail_id) = self.fs.find_by_name("Mail")
            && self.wm.is_open(mail_id)
        {
            let app_ref = self.wm.app_mut(mail_id).and_then(|app| app.as_any_mut().downcast_mut::<MailApp>());
            let sel = app_ref.as_ref().and_then(|app| app.selected());
            // 골라둔 메시지뿐 아니라 폴더 트리 선택(Inbox 등)도 이어가야 한다 — 안 그러면
            // MailApp::new() 가 항상 folder=None 으로 다시 시작해서, 메일 도착 새로고침
            // 때마다 "폴더를 선택하세요" 화면으로 도로 튕겨 보인다. 읽음 여부는 따로
            // 이어받을 필요가 없다 — open() 이 매번 fs.mail_read(진짜 기록)로 다시
            // 초기화해주므로 새 인스턴스에도 그대로 반영된다.
            let folder = app_ref.and_then(|app| app.folder_idx());
            let op = open(&self.fs, mail_id, settings);
            self.wm.refresh_app(mail_id, op.app);
            if let Some(app) = self.wm.app_mut(mail_id).and_then(|app| app.as_any_mut().downcast_mut::<MailApp>()) {
                // 폴더를 먼저 되돌려야 한다 — set_selected() 가 Inbox/Sent 중 어느
                // 목록 길이에 맞춰 clamp 할지 지금 self.folder 를 보고 정하기 때문이다.
                app.set_folder_idx(folder);
                if let Some(sel) = sel {
                    app.set_selected(Some(sel));
                }
            }
        }
    }

    // Mail 의 "Write Mail" 첨부 목록(attachable)이 지금 열려있는 창에도 최신
    // 상태로 반영되도록 — 다운로드/이동/삭제로 fs.desktop/downloads 가 바뀌었는데
    // Mail 을 이미 열어둔 채였다면(예: HexTool 로 파일을 검토하는 동안 다른 창에서
    // Mail 을 열어놨다가, 그사이 Photos 에서 새 사진을 받는 경우) 창을 닫았다 다시
    // 열기 전까진 목록에 새 파일이 안 보이던 문제 — refresh_mail_if_open() 처럼
    // 통째로 새 MailApp 을 만드는 대신(그러면 "Write Mail" 에 작성 중이던 초안이
    // 통째로 날아간다) attachable 목록만 그 자리에서 바꿔치기한다.
    fn refresh_mail_attachable_if_open(&mut self) {
        if let Some(mail_id) = self.fs.find_by_name("Mail")
            && self.wm.is_open(mail_id)
            && let Some(app) = self.wm.app_mut(mail_id).and_then(|app| app.as_any_mut().downcast_mut::<MailApp>())
        {
            app.refresh_attachable(mail_attachable_files(&self.fs));
        }
    }

    // ?????(Photos) 이 지금 열려있으면 새로 뽑힌 fs.photos_current 로 통째로
    // 다시 연다 — 재연구 업무 보고 메일을 보내는 순간(refresh_photos_feed 가 막
    // 불린 직후)에만 호출하므로, 스크롤 위치가 초기화되는 정도는(콘텐츠 자체가
    // 통째로 바뀌니) 자연스럽다.
    fn refresh_photos_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(photos_id) = self.fs.find_by_name(secrets::PHOTOS_APP_NAME)
            && self.wm.is_open(photos_id)
        {
            let op = open(&self.fs, photos_id, settings);
            self.wm.refresh_app(photos_id, op.app);
        }
    }

    // 현재 설정/바탕화면 상태를 그대로 파일에 저장한다 — fs 전체를 그대로 스냅샷하므로
    // 이름 목록을 따로 뽑아 모을 필요가 없다.
    fn write_save(&self, settings: &Rc<RefCell<Settings>>) {
        let window_geometry =
            self.window_geometry.iter().map(|(&fid, &(rect, maximized))| (fid, rect.x, rect.y, rect.w, rect.h, maximized)).collect();
        crate::foundation::save(&crate::foundation::SaveData {
            settings: settings.borrow().clone(),
            fs: self.fs.clone(),
            icon_pos: self.icon_pos.clone(),
            window_geometry,
        });
    }

    // 두 점으로부터 정규화된(음수 없는) 사각형을 만든다.
    fn marquee_rect(a: (f32, f32), b: (f32, f32)) -> Rect {
        Rect::new(a.0.min(b.0), a.1.min(b.1), (a.0 - b.0).abs(), (a.1 - b.1).abs())
    }

    // mr 과 겹치는 아이콘들을 선택 상태로 갱신한다. 드래그 중에도 매 프레임 불러서
    // 마우스를 떼기 전에도 실시간으로 선택 표시가 되게 한다.
    fn update_marquee_selection(&mut self, mr: Rect) {
        self.selected = (0..self.fs.desktop.len())
            .filter(|&i| Rect::new(self.icon_pos[i].0, self.icon_pos[i].1, IC_W, IC_H).intersects(&mr))
            .collect();
    }

    // 새 아이콘이 놓일 기본 자리 — 가장 왼쪽 칸을 1순위로, 그 안에서는 위쪽을 2순위로
    // 채운다(왼쪽 열이 다 차면(6줄) 그 오른쪽 칸으로 한 칸 넘어간다).
    fn grid_pos(i: usize) -> (f32, f32) {
        let rows = (GRID_MAX_ROW + 1) as usize;
        let c = (i / rows) as i32;
        let r = (i % rows) as i32;
        (GRID_ORIGIN + c as f32 * TILE_W, GRID_ORIGIN + r as f32 * TILE_H)
    }

    // ---- 타일맵 (아이콘이 스냅되는 격자) ----
    fn tile_of(pos: (f32, f32)) -> (i32, i32) {
        (((pos.0 - GRID_ORIGIN) / TILE_W).round() as i32, ((pos.1 - GRID_ORIGIN) / TILE_H).round() as i32)
    }
    fn tile_to_pos(c: i32, r: i32) -> (f32, f32) {
        (GRID_ORIGIN + c as f32 * TILE_W, GRID_ORIGIN + r as f32 * TILE_H)
    }
    fn tile_occupied(&self, c: i32, r: i32, ignore: usize) -> bool {
        self.icon_pos
            .iter()
            .enumerate()
            .any(|(i, p)| i != ignore && Self::tile_of(*p) == (c, r))
    }
    // (c,r) 에서 가장 가까운 빈 타일을 나선형으로 찾는다 — 손으로 드래그한 아이콘을
    // 놓았을 때 그 근처 빈 칸에 스냅시키는 용도(드롭 지점 근처를 우선한다).
    fn nearest_free_tile(&self, c: i32, r: i32, ignore: usize) -> (i32, i32) {
        for radius in 0i32..12 {
            for dr in -radius..=radius {
                for dc in -radius..=radius {
                    if radius > 0 && dc.abs().max(dr.abs()) != radius {
                        continue;
                    }
                    let (nc, nr) = (c + dc, r + dr);
                    if (0..=GRID_MAX_COL).contains(&nc)
                        && (0..=GRID_MAX_ROW).contains(&nr)
                        && !self.tile_occupied(nc, nr, ignore)
                    {
                        return (nc, nr);
                    }
                }
            }
        }
        (c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW))
    }

    // 지금 비어있는 칸 중 왼쪽을 1순위, 위쪽을 2순위로 격자 전체를 훑어 첫 번째로
    // 찾은 자리 — 새로 생기거나 바탕화면으로 옮겨오는 아이콘 자리를 정할 때 쓴다.
    // 매번 실제로 비어있는 칸을 다시 훑어서, 손으로 옮겨 비운 자리도 자연스럽게 채운다.
    fn first_free_tile(&self) -> (i32, i32) {
        for c in 0..=GRID_MAX_COL {
            for r in 0..=GRID_MAX_ROW {
                if !self.tile_occupied(c, r, usize::MAX) {
                    return (c, r);
                }
            }
        }
        (0, 0) // 격자가 완전히 꽉 찼으면 어쩔 수 없이 겹쳐서라도 첫 칸에 둔다.
    }

    // fs 에 새 파일을 만들어 바탕화면에 아이콘으로 추가한다 — 지금까지는 바탕화면
    // 아이콘이 전부 시작할 때 한 번만 고정으로 깔렸는데(fs.desktop 이 이후로 안 바뀜),
    // 설치 마법사를 끝내면 "설치된 프로그램" 아이콘이 실행 중에 새로 생겨야 해서
    // 처음으로 런타임에 fs.desktop/icon_pos 를 늘리는 경로가 생겼다.
    fn add_desktop_icon(&mut self, name: &str, kind: FileKind) {
        let id = self.fs.add(name, kind);
        self.fs.desktop.push(id);
        let (fc, fr) = self.first_free_tile();
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // add_desktop_icon 과 같은 자리 배치 요령이지만, 새 파일을 만드는 대신 File
    // Explorer 에서 드래그해온 기존 파일(id)을 바탕화면에 놓는다 — Downloads/폴더에서
    // 옮겨오는 경우. drop_at 은 실제로 마우스를 놓은 화면 좌표 — 그 위치에서 가장
    // 가까운 빈 칸(nearest_free_tile)에 둬서 "마우스로 놓은 자리"에 실제로 놓인다.
    fn add_existing_to_desktop(&mut self, id: FileId, drop_at: (f32, f32)) {
        if self.fs.desktop.contains(&id) {
            return;
        }
        self.fs.desktop.push(id);
        let (c, r) = Self::tile_of(drop_at);
        let (fc, fr) = self.nearest_free_tile(c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW), usize::MAX);
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // add_existing_to_desktop 과 달리 마우스 좌표에 안 기댄다 — 휴지통 "Restore"
    // 버튼처럼 놓은 지점이라는 게 아예 없는(버튼을 누른 시점의 마우스 좌표는 그
    // 버튼 위, 즉 지금 열려있는 휴지통 창 안이라 그 좌표를 드롭 지점으로 쓰면
    // 복구된 아이콘이 바로 그 창 뒤에 깔려 안 보이는 문제가 있었다) 프로그램적
    // 복구/생성엔 add_desktop_icon 처럼 항상 실제로 비어있는 첫 칸(first_free_tile)
    // 에 놓는 게 맞다.
    fn add_existing_to_desktop_default(&mut self, id: FileId) {
        if self.fs.desktop.contains(&id) {
            return;
        }
        self.fs.desktop.push(id);
        let (fc, fr) = self.first_free_tile();
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // File Explorer 사이드바 드래그(DeskAction::MoveFiles)와 바탕화면 아이콘을 열린
    // 탐색기 창으로 끌어다 놓는 경로가 둘 다 "지금 있던 자리에서 떼어 새 자리로"
    // 라는 같은 로직을 쓰므로 하나로 묶어둔다. drop_at 은 MoveDest::Desktop 일 때만
    // 쓰인다(어디에 놓였는지).
    fn move_ids_to(&mut self, ids: &[FileId], dest: MoveDest, drop_at: (f32, f32)) {
        // 휴지통(이름이 "Recycle Bin"인 Folder) 으로 들어가는 경우에만 "들어가기
        // 직전에 어디 있었는지" 를 기록해서, 나중에 Restore 가 정확히 그 자리로
        // 되돌릴 수 있게 한다. 그 외의 이동(휴지통에서 나가는 것 포함)은 더 이상
        // 그 기록이 필요 없으니 지운다.
        let into_recycle_bin = matches!(dest, MoveDest::Folder(fid) if self.fs.get(fid).name == RECYCLE_BIN_NAME);
        for &id in ids {
            // My Computer(FileKind::Explorer)는 실제 Windows 도 그렇듯 폴더/휴지통 안으로
            // 옮길 수 없다 — 휴지통에 들어가면 "Empty Recycle Bin" 한 번으로 영구히
            // 사라져서 게임을 진행할 방법 자체가 없어지는 치명적인 경우라, 옮기는 시도
            // 자체를 여기서 막는다(그 무엇으로도 못 지운다는 사실을 애초에 보장). 휴지통
            // 자기 자신도 같은 이유로 막는다 — 이 가드가 없었을 땐 휴지통 아이콘을
            // 휴지통 안으로 끌어놓으면 자기 자신을 자기 children 에 넣어버려서(제보받은
            // "쓰레기통에 쓰레기통" 스크린샷) 휴지통을 열면 그 안에 휴지통이 보이는
            // 자기참조 상태가 됐었다.
            if matches!(dest, MoveDest::Folder(_))
                && (matches!(self.fs.get(id).kind, FileKind::Explorer) || self.fs.get(id).name == RECYCLE_BIN_NAME)
            {
                continue;
            }
            // Photos 앱("?????"로 보이는 아이콘)도 같은 이유로 어떤 폴더로도 못
            // 옮긴다 — 다만 휴지통으로 넣으려는 시도(=지우려는 시도)일 땐 조용히
            // 막는 대신 "Are You idiot?" 모달을 띄워서 대놓고 못 지운다는 걸
            // 알려준다.
            if matches!(dest, MoveDest::Folder(_)) && matches!(self.fs.get(id).kind, FileKind::PhotoGallery) {
                if into_recycle_bin {
                    self.idiot_confirm = true;
                }
                continue;
            }
            // 지금 창이 열려있는 파일은 휴지통으로 못 보낸다 — 열어서 보고 있는 걸
            // 그대로 지워버리면(휴지통 비우기까지 한 번이면 영구 삭제) 그 창은 이제
            // 존재하지 않는 파일을 가리키는 유령 창이 되어버린다. 실제 Windows 도
            // 열려있는 파일은 삭제를 막거나 경고하는 것과 같은 맥락 — 여기선 그냥
            // 조용히 옮기지 않고 원래 있던 자리에 그대로 둔다.
            if into_recycle_bin && self.wm.is_open(id) {
                continue;
            }
            self.fs.trash_origin.retain(|&(tid, _)| tid != id);
            if into_recycle_bin {
                // locate() 가 못 찾으면(예: 어떤 컨테이너에도 안 속하고 Videos/Images
                // 가상 탭에만 종류로 보이던 사진/동영상) Loose 로 기록한다 — None 으로
                // 그냥 건너뛰면 나중에 복구할 때 이 파일이 "기록이 아예 없던 것"과
                // 구분이 안 돼서 엉뚱하게 바탕화면으로 복구돼버린다.
                let origin = self.fs.locate(id).unwrap_or(FileOrigin::Loose);
                self.fs.trash_origin.push((id, origin));
            }
            if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                self.fs.desktop.remove(pos);
                self.icon_pos.remove(pos);
            }
            self.fs.detach_from_container(id);
            match dest {
                MoveDest::Desktop => self.add_existing_to_desktop(id, drop_at),
                MoveDest::Downloads => self.fs.download(id),
                MoveDest::Folder(fid) => self.fs.add_to_folder(fid, id),
            }
        }
    }

    // 휴지통의 "Restore" — fs.trash_origin 에 기록해둔 원래 위치로 정확히 되돌린다
    // (기록이 없으면, 예: 이 기능이 생기기 전에 이미 휴지통에 들어가 있던 항목이면
    // 바탕화면으로). 원래 있던 폴더가 그 사이 사라졌을 리는 없지만(폴더 자체를
    // 영구 삭제하는 기능은 없다) 혹시 몰라 여전히 Folder 인지 확인하고 아니면
    // 역시 바탕화면으로 대신 보낸다.
    fn restore_from_trash(&mut self, ids: &[FileId]) {
        for &id in ids {
            let origin =
                self.fs.trash_origin.iter().position(|&(tid, _)| tid == id).map(|pos| self.fs.trash_origin.remove(pos).1);
            self.fs.detach_from_container(id);
            match origin {
                Some(FileOrigin::Downloads) => self.fs.download(id),
                Some(FileOrigin::Folder(fid)) if matches!(self.fs.get(fid).kind, FileKind::Folder { .. }) => {
                    self.fs.add_to_folder(fid, id);
                }
                // 애초에 어떤 컨테이너에도 안 속하고 종류(Img/Mp4)만으로 Videos/Images
                // 가상 탭에 보이던 파일 — 방금 detach_from_container 로 휴지통 children
                // 에서 빠졌으니, 여기서 아무 데도 새로 안 넣어도 그 가상 탭에 저절로
                // 다시 나타난다(어딘가에 억지로 넣으면 오히려 중복으로 보인다).
                Some(FileOrigin::Loose) => {}
                _ => self.add_existing_to_desktop_default(id),
            }
        }
    }

    // 바탕화면 아이콘을 드래그해서 놓은 자리(m)가 열려있는 File Explorer 창 위라면,
    // 그 창이 지금 보여주고 있는 폴더/카테고리를 옮길 대상으로 돌려준다 — 창이
    // 없거나 파일 탐색기가 아니거나(Mail/Settings 등), 대상이 될 수 없는 카테고리
    // (Videos 등)면 None.
    fn explorer_drop_target_at(&mut self, m: (f32, f32)) -> Option<MoveDest> {
        let win_file = self.wm.file_at(m)?;
        // Credits/Official Site 같은 창은 실제 fs.nodes 항목이 아니라 창
        // 중복-열기 방지용 가짜 FileId(CREDITS_WIN 등, usize::MAX 근처)를 쓴다 —
        // 그 값을 아래 fs.get() 에 그대로 넘기면 배열 범위를 벗어나 패닉한다
        // (제보받은 "크래딧 창 위에 파일을 놓으면 게임이 터진다" 버그의 원인).
        // 진짜 fs.nodes 안의 id 가 아니면 그냥 "옮길 대상 아님"으로 취급한다.
        if !self.fs.contains(win_file) {
            return None;
        }
        // 창 자체가 실제 폴더에 연결돼 있으면(더블클릭으로 드릴다운한 단일 폴더
        // 탐색기 창) 그 폴더가 바로 대상 — 탭이 없어 downcast 로 물어볼 필요가 없다.
        if matches!(self.fs.get(win_file).kind, FileKind::Folder { .. }) {
            return Some(MoveDest::Folder(win_file));
        }
        // 그 외엔 탭이 있는 루트 File Explorer 창일 수 있다 — 지금 활성 탭을 물어본다.
        let explorer = self.wm.app_mut(win_file)?.as_any_mut().downcast_mut::<ExplorerApp>()?;
        match explorer.current_location()? {
            ExplorerLocation::Folder(fid) => Some(MoveDest::Folder(fid)),
            ExplorerLocation::Category(name) => match name.as_str() {
                "Desktop" => Some(MoveDest::Desktop),
                "Downloads" => Some(MoveDest::Downloads),
                _ => None, // Videos/Images 등 — 종류로만 모으는 가상 카테고리라 대상이 될 수 없다
            },
        }
    }

    // 바탕화면 위의 다른 폴더 아이콘 위에 놓았는지 — 있으면 그 폴더 안으로 옮긴다.
    // exclude 는 지금 드래그 중인 아이콘들 자신의 인덱스(자기 자신 위에 놓은 걸
    // "드롭"으로 치면 안 되니 제외).
    fn desktop_folder_drop_target_at(&self, m: (f32, f32), exclude: &[usize]) -> Option<FileId> {
        for i in 0..self.fs.desktop.len() {
            if exclude.contains(&i) {
                continue;
            }
            let (x, y) = self.icon_pos[i];
            if Rect::new(x, y, IC_W, IC_H).contains(m.0, m.1) {
                let fid = self.fs.desktop[i];
                if matches!(self.fs.get(fid).kind, FileKind::Folder { .. }) {
                    return Some(fid);
                }
            }
        }
        None
    }

    fn start_button_rect(&self) -> Rect {
        Rect::new(2.0, SCREEN_H - TASKBAR_H + 3.0, 42.0, TASKBAR_H - 6.0)
    }

    fn clock_rect(&self) -> Rect {
        // 왼쪽에 와이파이 아이콘이 들어갈 자리(24px)를 남겨두고 그만큼 왼쪽으로 옮긴 뒤,
        // 화면 오른쪽 끝에 너무 붙지 않도록 10px 만큼 다시 오른쪽으로 뺐다. 높이는
        // 시작 버튼/창 버튼과 같은 TASKBAR_H-6 으로 맞춘다 — 예전엔 TASKBAR_H-10(18px)
        // 이었는데 글자 한 줄 높이(CELL_H*0.9 ≈ 20px)보다 낮아서 시계 숫자가 sunken
        // 박스 위아래로 살짝 삐져나와 보였다.
        const W: f32 = 70.0;
        const RIGHT_MARGIN: f32 = 10.0;
        Rect::new(SCREEN_W - RIGHT_MARGIN - W, SCREEN_H - TASKBAR_H + 3.0, W, TASKBAR_H - 6.0)
    }

    fn wifi_rect(&self) -> Rect {
        let ck = self.clock_rect();
        Rect::new(ck.x - 22.0, ck.y, 18.0, ck.h)
    }

    // 와이파이 아이콘을 누르면 그 위에 뜨는 작은 연결 정보 팝업.
    fn wifi_popup_rect(&self) -> Rect {
        let (w, h) = (220.0, 78.0); // "Status: Disconnected" 같은 긴 줄이 안 삐져나오게 넉넉히
        let wr = self.wifi_rect();
        let x = (wr.x + wr.w - w).max(4.0);
        Rect::new(x, SCREEN_H - TASKBAR_H - h, w, h)
    }

    fn start_menu_rect(&self, r: &Renderer, lang: Language) -> Rect {
        let h = MENU_ITEMS.len() as f32 * 24.0 + 8.0;
        // 가장 긴 번역 문구 폭에 맞춰 너비를 정해서 텍스트가 회색 영역을 벗어나지
        // 않게 한다 — ADVANCE(고정폭) 기반 글자수 계산 대신 실제 text_width() 를
        // 쓴다(한글/일본어는 라틴 문자와 폭이 다른 글자가 섞여 있어 글자수만으론
        // 안 맞는다).
        let max_w = MENU_ITEMS.iter().map(|&k| r.text_width(menu_item_label(lang, k), 1.0)).fold(0.0f32, f32::max);
        let w = max_w + 40.0;
        Rect::new(2.0, SCREEN_H - TASKBAR_H - h, w, h)
    }

    // 우클릭한 지점 pos 에 메뉴를 띄우되, 화면(작업표시줄 위쪽) 밖으로 안 나가게 clamp.
    fn context_menu_rect(r: &Renderer, lang: Language, pos: (f32, f32)) -> Rect {
        let h = CONTEXT_MENU_ITEMS.len() as f32 * CTX_ROW_H + 6.0;
        let max_w = CONTEXT_MENU_ITEMS.iter().map(|&k| r.text_width(menu_item_label(lang, k), CTX_TEXT_SCALE)).fold(0.0f32, f32::max);
        let w = max_w + 24.0;
        let x = pos.0.min(SCREEN_W - w).max(0.0);
        let y = pos.1.min(SCREEN_H - TASKBAR_H - h).max(0.0);
        Rect::new(x, y, w, h)
    }

    // "Erase All Memory" 확인창의 다이얼로그 박스 + Erase/Cancel 버튼 자리. 화면
    // 전체(640x480) 기준 절대좌표라 어떤 창이 위에 떠 있어도 항상 화면 정중앙에
    // 뜬다 — 입력 판정(update())과 그리기(draw_erase_confirm()) 양쪽에서 같이 써서
    // 자리가 어긋나지 않게 한다.
    fn erase_confirm_layout() -> (Rect, Rect, Rect) {
        let dw = 220.0;
        let dh = 110.0;
        let dr = Rect::new((SCREEN_W - dw) / 2.0, (SCREEN_H - dh) / 2.0, dw, dh);
        let bw = 80.0;
        let by = dr.y + dr.h - 34.0;
        let erase_btn = Rect::new(dr.x + 12.0, by, bw, 22.0);
        let cancel_btn = Rect::new(dr.x + dr.w - 12.0 - bw, by, bw, 22.0);
        (dr, erase_btn, cancel_btn)
    }

    // 화면 전체(다른 창 포함)를 반투명 흰색 워시로 덮어 "뿌옇게 흐려진" 느낌을 낸
    // 다음, 그 위에 확인 다이얼로그를 그린다. 진짜 가우시안 블러는 렌더러가 프레임
    // 끝에 한 번에만 그리는 배치 방식이라(그리는 도중엔 이미 그려진 화면을 다시
    // 읽어 블러할 수가 없다) 안 되고, 이 정도가 엔진을 안 뜯어고치고 낼 수 있는
    // 현실적인 타협점이다.
    fn draw_erase_confirm(&self, r: &mut Renderer, _time: f32, mouse: (f32, f32), lang: Language) {
        r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, [1.0, 1.0, 1.0, 0.4]);

        let (dr, erase_btn, cancel_btn) = Self::erase_confirm_layout();
        raised(r, dr.x, dr.y, dr.w, dr.h);
        r.text_clipped(dr.x + 12.0, dr.y + 12.0, t(lang, s::ERASE_LINE1), 0.8, BLACK, dr.w - 24.0);
        r.text_clipped(dr.x + 12.0, dr.y + 32.0, t(lang, s::ERASE_LINE2), 0.8, BLACK, dr.w - 24.0);

        // 다른 창들과 같은 raised/sunken 버튼 스타일이 아니라 여기서는 그냥 살짝
        // 눌림 효과만 흉내내는 간단한 hover 표시 — button() 헬퍼는 WinInput 이
        // 필요한데 여기는 화면 전체 모달이라 안 맞아서 직접 그린다. 버튼 자리
        // 자체는 고정 Rect(erase_confirm_layout)라 번역 문구 길이가 달라져도
        // 클릭 판정은 안 흔들린다.
        let erase_label = t(lang, s::ERASE);
        let cancel_label = t(lang, common::CANCEL);
        for (btn, label) in [(erase_btn, erase_label), (cancel_btn, cancel_label)] {
            let hover = btn.contains(mouse.0, mouse.1);
            if hover {
                sunken(r, btn.x, btn.y, btn.w, btn.h);
            } else {
                raised(r, btn.x, btn.y, btn.w, btn.h);
            }
            let tw = r.text_width(label, 1.0);
            r.text(btn.x + (btn.w - tw) / 2.0, btn.y + 1.0, label, 1.0, BLACK);
        }
    }

    fn idiot_confirm_layout() -> Rect {
        let dw = 200.0;
        let dh = 90.0;
        let dr = Rect::new((SCREEN_W - dw) / 2.0, (SCREEN_H - dh) / 2.0, dw, dh);
        let bw = 70.0;
        Rect::new(dr.x + (dr.w - bw) / 2.0, dr.y + dr.h - 32.0, bw, 22.0)
    }

    // Photos("?????") 아이콘을 휴지통으로 끌어다 놓으려 하면 뜨는 놀림 모달 —
    // erase_confirm 과 같은 요령으로 진짜 WindowManager 창이 아니라 화면 전체를
    // 덮는 오버레이로 직접 그린다. 최소화/최대화/크기조절/닫기 버튼 자체가 없는
    // 것도 그래서다(진짜 창이 아니니 애초에 그런 버튼을 그릴 일이 없다) — 오직
    // "Yes" 버튼 하나만 눌러야 닫힌다.
    fn draw_idiot_confirm(&self, r: &mut Renderer, mouse: (f32, f32)) {
        r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, [0.0, 0.0, 0.0, 0.55]);

        let dw = 200.0;
        let dh = 90.0;
        let dr = Rect::new((SCREEN_W - dw) / 2.0, (SCREEN_H - dh) / 2.0, dw, dh);
        raised(r, dr.x, dr.y, dr.w, dr.h);

        // 진짜 창처럼 보이는 파란 타이틀바 — 다만 버튼은 하나도 안 그린다(장식만).
        let tb = Rect::new(dr.x, dr.y, dr.w, 18.0);
        r.rect(tb.x, tb.y, tb.w, tb.h, NAVY);
        r.text(tb.x + 4.0, tb.y, "System", 0.8, WHITE);

        let msg = "Are You idiot?";
        let tw = r.text_width(msg, 0.9);
        r.text(dr.x + (dr.w - tw) / 2.0, dr.y + 36.0, msg, 0.9, BLACK);

        let yes_btn = Self::idiot_confirm_layout();
        let hover = yes_btn.contains(mouse.0, mouse.1);
        if hover {
            sunken(r, yes_btn.x, yes_btn.y, yes_btn.w, yes_btn.h);
        } else {
            raised(r, yes_btn.x, yes_btn.y, yes_btn.w, yes_btn.h);
        }
        let yw = r.text_width("Yes", 1.0);
        r.text(yes_btn.x + (yes_btn.w - yw) / 2.0, yes_btn.y + 1.0, "Yes", 1.0, BLACK);
    }

    fn taskbar_buttons(&self) -> Vec<(u32, String, Rect, bool, bool)> {
        let ty = SCREEN_H - TASKBAR_H;
        let items = self.wm.taskbar_items();
        let start_x = 50.0;
        // 와이파이 아이콘이 시계보다 왼쪽에 따로 자리를 차지하고 있어서, 시계 x 좌표까지
        // 남는 공간을 다 쓰면 그 아이콘이랑 창 버튼 글자가 겹친다 — 와이파이 아이콘의
        // x 좌표를 오른쪽 경계로 써야 한다.
        let right_bound = self.wifi_rect().x;
        let avail = right_bound - start_x - 4.0;
        let n = items.len().max(1) as f32;
        let bw = (avail / n).min(140.0);
        items
            .into_iter()
            .enumerate()
            .map(|(i, (id, title, foc, min))| {
                let x = start_x + i as f32 * bw;
                (id, title, Rect::new(x, ty + 3.0, bw - 3.0, TASKBAR_H - 6.0), foc, min)
            })
            .collect()
    }

    fn icon_hit(&self, m: (f32, f32)) -> Option<usize> {
        for i in 0..self.fs.desktop.len() {
            let (x, y) = self.icon_pos[i];
            if Rect::new(x, y, IC_W, IC_H).contains(m.0, m.1) {
                return Some(i);
            }
        }
        None
    }

    // 글리치 버스트를 랜덤한 간격으로 발생/진행시킨다 — lobby.rs::tick_glitch 와
    // 같은 요령. 버스트가 막 시작되는 순간에만 이번 버스트의 색 채널 어긋남 폭을
    // 새로 뽑아서 그 버스트 내내 고정해 쓴다(매 프레임 다시 뽑으면 떨리기만 하고
    // "한 번 찢어진" 느낌이 안 난다).
    fn tick_icon_glitch(&mut self, dt: f32) {
        if self.icon_glitch_active > 0.0 {
            self.icon_glitch_active -= dt;
            return;
        }
        self.icon_glitch_timer -= dt;
        if self.icon_glitch_timer <= 0.0 {
            self.icon_glitch_active = ICON_GLITCH_BURST;
            self.icon_glitch_offset = self.icon_glitch_rng.range_f32(1.5, 3.5);
            self.icon_glitch_timer = self.icon_glitch_rng.range_f32(ICON_GLITCH_GAP_MIN, ICON_GLITCH_GAP_MAX);
        }
    }

    // Photos 아이콘 전용 — 살짝 어긋난 색 채널 세 겹(빨강은 왼쪽으로, 파랑은
    // 오른쪽으로)을 겹쳐 그려서 Director 글리치와 같은 색수차 느낌을 미니어처로
    // 낸다. draw_icon() 은 흰색으로만 그리게 돼 있어(공용 API) 여기선 그 대신
    // assets.icon_photos 텍스처를 직접 색을 입혀 그린다.
    fn draw_photos_icon_glitched(r: &mut Renderer, assets: &Assets, x: f32, y: f32, s: f32, off: f32) {
        r.sprite(assets.icon_photos, x - off, y, s, s, [1.0, 0.35, 0.35, 0.85]);
        r.sprite(assets.icon_photos, x, y, s, s, [0.35, 1.0, 0.35, 0.7]);
        r.sprite(assets.icon_photos, x + off, y, s, s, [0.35, 0.35, 1.0, 0.85]);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_one_icon(&self, r: &mut Renderer, assets: &Assets, x: f32, y: f32, icon: &IconType, name: &str, selected: bool) {
        // Tar/Installer 는 텍스처가 아니라 직접 그리는 벡터 도형이라(draw_tar_icon/
        // draw_installer_icon), 실제로 칠해지는 면적이 s×s 상자 안에서 꽤 여백을
        // 두고 작게 그려져 다른(텍스처 기반) 아이콘들보다 눈에 띄게 작아 보였다 —
        // 이 둘만 조금 더 키운다. 다만 실제로 그려지는 크기가 얼마든 아이콘의 세로
        // 중심은 항상 ICON_AREA_TOP+ICON_BASE_SIZE/2(고정값) 에 맞추고, 그 아래
        // 글자 시작 위치(ty0)도 항상 그 고정값 기준으로만 잡는다 — 아이콘이 커져도
        // 글자가 덩달아 밀려 내려가 다음 줄 아이콘과 겹치는 일이 없다.
        let icon_s = match icon {
            IconType::Tar | IconType::Installer => IC_SIZE * 1.3,
            _ => IC_SIZE,
        };
        let icon_center_y = y + ICON_AREA_TOP + ICON_BASE_SIZE / 2.0;
        let (icon_x, icon_y) = (x + IC_W / 2.0 - icon_s / 2.0, icon_center_y - icon_s / 2.0);
        if matches!(icon, IconType::PhotosApp) && self.icon_glitch_active > 0.0 {
            Self::draw_photos_icon_glitched(r, assets, icon_x, icon_y, icon_s, self.icon_glitch_offset);
        } else {
            draw_icon(r, assets, icon, icon_x, icon_y, icon_s);
        }
        let ls = LABEL_TEXT_SCALE;
        let lines = wrap_two_lines(r, name, ls, LABEL_MAX_W);
        let line_h = LABEL_LINE_H;
        let ty0 = y + ICON_AREA_TOP + ICON_BASE_SIZE + LABEL_GAP;
        // LABEL_MAX_W 로 감싸도, 글자폭 계산(ADVANCE 기준)이 실제 렌더링(CRT
        // 블룸 등)과 완벽히 안 맞으면 여전히 옆 칸 글자와 살짝 겹쳐 보일 수 있다 —
        // 계산에 기대는 대신 타일 폭(IC_W) 밖으로는 물리적으로 아예 못 그리게
        // 클립을 걸어서 확실히 막는다.
        let outer_clip = r.clip();
        let label_rect = Rect::new(x - 1.0, ty0 - 1.0, IC_W + 2.0, line_h * LABEL_MAX_LINES + 2.0);
        r.set_clip(Some(outer_clip.map_or(label_rect, |c| c.intersect(&label_rect))));
        for (i, line) in lines.iter().enumerate() {
            let tw = r.text_width(line, ls);
            let tx = x + (IC_W - tw) / 2.0;
            let ty = ty0 + i as f32 * line_h;
            if selected {
                r.rect(tx - 2.0, ty - 1.0, tw + 4.0, line_h, NAVY);
                r.text(tx, ty, line, ls, WHITE);
            } else {
                r.text(tx, ty, line, ls, WHITE);
            }
        }
        r.set_clip(outer_clip);
    }

    // 드래그 중에도 icon_pos 는 원래 자리 그대로라(더는 실시간으로 안 움직인다 —
    // 대신 update() 가 반투명 고스트를 커서 쪽에 따로 그린다), 예전처럼 드래그 중인
    // 아이콘을 맨 위에 다시 그려줄 필요가 없어져서 한 번에 순서대로만 그리면 된다.
    fn draw_icons(&self, r: &mut Renderer, assets: &Assets, lang: Language) {
        for (i, &fid) in self.fs.desktop.iter().enumerate() {
            let node = self.fs.get(fid);
            let (x, y) = self.icon_pos[i];
            let name = display_name(lang, &node.name);
            self.draw_one_icon(r, assets, x, y, &icon_of(node), &name, self.selected.contains(&i));
        }
    }

    fn draw_taskbar(&self, r: &mut Renderer, buttons: &[(u32, String, Rect, bool, bool)], time: f32) {
        let ty = SCREEN_H - TASKBAR_H;
        raised(r, 0.0, ty, SCREEN_W, TASKBAR_H);

        // 시작 버튼 (저울 아이콘, 문구 없음)
        let sb = self.start_button_rect();
        if self.start_open {
            sunken(r, sb.x, sb.y, sb.w, sb.h);
        } else {
            raised(r, sb.x, sb.y, sb.w, sb.h);
        }
        draw_scale(r, sb.x + sb.w / 2.0 - 9.0, sb.y + 2.0, 18.0, BLACK);

        // 창 버튼들
        for (_id, title, rect, foc, _min) in buttons {
            let (rect, foc) = (*rect, *foc);
            if foc {
                sunken(r, rect.x, rect.y, rect.w, rect.h);
            } else {
                raised(r, rect.x, rect.y, rect.w, rect.h);
            }
            r.text_clipped(rect.x + 5.0, rect.y + 2.0, title, 0.9, BLACK, rect.w - 8.0);
        }

        // 와이파이 표시 — 연결돼 있으면 평소대로, 끊겨 있으면 점+X만 깜빡이며 표시.
        let wr = self.wifi_rect();
        let wifi_color = if self.wifi_connected { BLACK } else { GRAY };
        let blink_visible: bool = (time * 2.0) as i32 % 2 == 0; // 0.5초 간격으로 깜빡
        draw_wifi(r, wr.x + 2.0, wr.y + wr.h * 0.5 - 7.0, 14.0, wifi_color, self.wifi_connected, blink_visible);

        // 시계 — 그냥 표시용(클릭해도 아무 일 없음).
        let ck = self.clock_rect();
        sunken(r, ck.x, ck.y, ck.w, ck.h);
        let unix = miniquad::date::now() as u64 + 9 * 3600; // KST
        let hh = (unix / 3600) % 24;
        let mm = (unix / 60) % 60;
        let clock = format!("{:02}:{:02}", hh, mm);
        // 세로 중앙 정렬 — 예전엔 ck.y 를 그대로 써서(위쪽 정렬) 글자가 박스 위아래로
        // 삐져나와 보였다. 다른 타임바 요소들(창 버튼 제목 등)과 같은 요령으로 맞춘다.
        let ty = ck.y + (ck.h - CELL_H * 0.9) / 2.0;
        // 왼쪽 여백을 5→9px 로 늘렸다 — sunken() 테두리 바로 옆에 숫자가 거의
        // 붙어 보인다는 피드백을 받았다(오른쪽 clip 폭도 그만큼 같이 줄여 균형을 맞춘다).
        r.text_clipped(ck.x + 13.0, ty, &clock, 0.9, BLACK, ck.w - 16.0);
    }

    fn draw_start_menu(&self, r: &mut Renderer, m: (f32, f32), lang: Language) {
        let mr = self.start_menu_rect(r, lang);
        raised(r, mr.x, mr.y, mr.w, mr.h);
        r.rect(mr.x + 3.0, mr.y + 3.0, 22.0, mr.h - 6.0, NAVY);
        for (i, &key) in MENU_ITEMS.iter().enumerate() {
            let name = menu_item_label(lang, key);
            let iy = mr.y + 4.0 + i as f32 * 24.0;
            let row = Rect::new(mr.x + 28.0, iy, mr.w - 32.0, 22.0);
            let hover = row.contains(m.0, m.1);
            if hover {
                r.rect(row.x, row.y, row.w, row.h, NAVY);
                r.text(row.x + 4.0, iy + 2.0, name, 1.0, WHITE);
            } else {
                r.text(row.x + 4.0, iy + 2.0, name, 1.0, BLACK);
            }
        }
    }

    // 시작 메뉴와 바탕화면 우클릭 메뉴가 공유하는 항목(Official Site/Credits/Settings)
    // 처리. Shut Down 은 시작 메뉴에만 있어서 호출부에서 따로 처리한다.
    fn run_menu_action(&mut self, idx: i32, settings: Rc<RefCell<Settings>>, work: Rect) {
        let lang = settings.borrow().language;
        match idx {
            0 => {
                // Official Site: 실제 사이트를 창 안에 띄운다. WebView2 로 백그라운드에서
                // 렌더링해 주기적으로 캡처한 이미지를 텍스처로 올리는 방식이라(OfficialSiteApp
                // 참고) CRT 셰이더도 다른 화면들과 똑같이 먹는다.
                let op = Opened {
                    // 캡처 해상도를 창 표시 크기보다 훨씬 크게 잡는다 — 뷰포트가
                    // 너무 좁으면(예: 460x320) 사이트가 모바일/확대된 레이아웃으로
                    // 렌더링돼서 답답해 보인다. 넓게 그린 다음 축소해서 보여주면
                    // 일반 데스크톱 화면처럼 자연스럽게 나온다.
                    // JPEG 캡처로 바꿔서 여유가 좀 생겼지만, 그래도 해상도가 너무 크면
                    // 캡처 한 장당 걸리는 시간이 늘어나 프레임레이트가 떨어진다 — 화질과
                    // 속도 사이에서 적당한 지점으로 살짝 낮췄다.
                    app: Box::new(OfficialSiteApp::new(OFFICIAL_SITE_URL, 1000, 750, settings.clone())),
                    title: menu_item_label(lang, "Official Site").to_string(),
                    size: (480.0, 380.0),
                    maximized: false,
                    resizable: false,
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                // 이미 열려있어서 새로 스폰된 게 아니면(그냥 앞으로 가져온 거면) 위치를
                // 또 밀면 안 된다 — 안 그러면 클릭할 때마다 창이 계속 아래로 내려간다.
                if self.wm.open(op, Some(OFFICIAL_SITE_WIN), work) {
                    self.wm.nudge_last(30.0); // 다른 창들보다 살짝 아래에서 스폰
                }
            }
            1 => {
                let op = Opened {
                    app: Box::new(CreditsApp(settings.clone())),
                    title: menu_item_label(lang, "Credits").to_string(),
                    // 개발자 이름이 늘어날 때마다(지금 4명) 목록 줄 수만큼 여유가
                    // 있어야 OK 버튼과 안 겹친다 — 이름 하나 늘 때 20px 정도 더 잡는다.
                    size: (260.0, 200.0),
                    maximized: false,
                    resizable: false,
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                self.wm.open(op, Some(CREDITS_WIN), work);
            }
            2 => {
                let op = Opened {
                    app: Box::new(SettingsApp::new(settings)),
                    title: menu_item_label(lang, "Settings").to_string(),
                    size: (360.0, 260.0), // 클리핑이 제대로 고쳐져서 넘치는 내용은 스크롤로 알아서 잘리니 더 줄여도 안전하다
                    maximized: false,
                    resizable: false,   // 설정창 크기 고정
                    maximizable: false,
                    movable: true,
                    min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
                };
                self.wm.open(op, Some(SETTINGS_WIN), work); // 이미 열려있으면 새로 안 열고 앞으로
            }
            _ => {}
        }
    }

    // "New Mail" 토스트 자리 — update_toast() 와 커서 판정(update()) 양쪽에서
    // 같이 써서 자리가 어긋나지 않게 한다(wifi_popup_rect 와 같은 요령).
    fn toast_rect() -> Rect {
        const W: f32 = 190.0;
        const H: f32 = 72.0;
        let ty = SCREEN_H - TASKBAR_H;
        let x = SCREEN_W - W - 8.0;
        let y = ty - H - 8.0;
        Rect::new(x, y, W, H)
    }

    // 시스템 메시지가 새로 생기면(지금은 메일 도착) 우측 하단(작업표시줄 바로 위,
    // 와이파이/시계 트레이 근처)에 잠깐 떴다가 TOAST_DURATION 뒤 저절로 사라지는
    // 알림. 누르면 바로 Mail 을 열고 닫힌다.
    fn update_toast(&mut self, f: &mut Frame, work: Rect, lang: Language) {
        let Some((from, subject)) = self.toast.clone() else { return };
        let Rect { x, y, w, h } = Self::toast_rect();
        raised(f.r, x, y, w, h);
        f.r.rect(x + 2.0, y + 2.0, w - 4.0, 16.0, DARK_GRAY);
        f.r.text(x + 6.0, y + 3.0, t(lang, s::NEW_MAIL), 0.8, WHITE);
        f.r.text_clipped(x + 6.0, y + 26.0, &from, 0.8, BLACK, w - 12.0);
        f.r.text_clipped(x + 6.0, y + 46.0, &subject, 0.8, GRAY, w - 12.0);

        if Rect::new(x, y, w, h).contains(f.input.mouse.0, f.input.mouse.1) && f.input.mouse_clicked {
            self.toast = None;
            self.toast_timer = 0.0;
            if let Some(mail_id) = self.fs.find_by_name("Mail") {
                let op = open(&self.fs, mail_id, &f.settings);
                self.wm.open(op, Some(mail_id), work);
            }
        }
    }

    // 와이파이 아이콘을 누르면 뜨는 연결 정보(상태/SSID/IP) 팝업.
    fn draw_wifi_popup(&self, r: &mut Renderer, lang: Language) {
        let pr = self.wifi_popup_rect();
        raised(r, pr.x, pr.y, pr.w, pr.h);
        r.rect(pr.x, pr.y, pr.w, 20.0, DARK_GRAY);
        r.text(pr.x + 6.0, pr.y + 2.0, t(lang, s::NETWORK), 0.85, WHITE);

        let status_word = t(lang, s::STATUS);
        let state_word = if self.wifi_connected { t(lang, s::CONNECTED) } else { t(lang, s::DISCONNECTED) };
        let status = format!("{status_word}: {state_word}");
        r.text_clipped(pr.x + 8.0, pr.y + 26.0, &status, 0.8, if self.wifi_connected { BLACK } else { GRAY }, pr.w - 16.0);

        let unknown = t(lang, s::UNKNOWN);
        let info = self.wifi_info.as_ref();
        let ssid_line = match info.and_then(|i| i.ssid.as_deref()) {
            Some(ssid) => format!("SSID: {ssid}"),
            None => format!("SSID: {unknown}"),
        };
        r.text_clipped(pr.x + 8.0, pr.y + 42.0, &ssid_line, 0.8, GRAY, pr.w - 16.0);

        let ip_line = match info.and_then(|i| i.ip.as_deref()) {
            Some(ip) => format!("IP: {ip}"),
            None => format!("IP: {unknown}"),
        };
        r.text_clipped(pr.x + 8.0, pr.y + 58.0, &ip_line, 0.8, GRAY, pr.w - 16.0);
    }

    // 아이콘 왼쪽 세로 띠가 없다는 점 빼면 시작 메뉴랑 같은 모양 — 컨텍스트 메뉴답게
    // 더 단순하게.
    fn draw_context_menu(&self, r: &mut Renderer, pos: (f32, f32), m: (f32, f32), lang: Language) {
        let mr = Self::context_menu_rect(r, lang, pos);
        raised(r, mr.x, mr.y, mr.w, mr.h);
        for (i, &key) in CONTEXT_MENU_ITEMS.iter().enumerate() {
            let name = menu_item_label(lang, key);
            let iy = mr.y + 3.0 + i as f32 * CTX_ROW_H;
            let row = Rect::new(mr.x + 3.0, iy, mr.w - 6.0, CTX_ROW_H);
            let hover = row.contains(m.0, m.1);
            // 텍스트 셀 높이(CELL_H*스케일)를 줄 안에서 세로로 가운데 맞춰야 구분선이
            // 위/아래 글자와 같은 간격으로 보인다.
            let ty = row.y + (row.h - CELL_H * CTX_TEXT_SCALE) / 2.0;
            if hover {
                r.rect(row.x, row.y, row.w, row.h, NAVY);
                r.text(row.x + 5.0, ty, name, CTX_TEXT_SCALE, WHITE);
            } else {
                r.text(row.x + 5.0, ty, name, CTX_TEXT_SCALE, BLACK);
            }
            // 마지막 항목 빼고 아래에 구분선을 그어 버튼들을 나눈다.
            if i + 1 < CONTEXT_MENU_ITEMS.len() {
                r.rect(mr.x + 2.0, row.y + row.h, mr.w - 4.0, 1.0, GRAY);
            }
        }
    }

    // 새로 연 창(open() 이 true 를 돌려줬을 때만 — 이미 열려있어서 앞으로만 가져온
    // 경우는 지금 자리를 그대로 둬야 한다)에 기억해둔 마지막 크기/위치가 있으면 적용한다.
    fn apply_saved_geometry(&mut self, file: FileId, work: Rect) {
        if let Some(&(rect, maximized)) = self.window_geometry.get(&file) {
            self.wm.set_geometry(file, rect, maximized, work);
        }
    }

    fn open_file(&mut self, idx: usize, settings: &Rc<RefCell<Settings>>, work: Rect) {
        let fid = self.fs.desktop[idx];
        if self.is_drilldown_folder(fid) {
            self.open_folder_in_explorer(fid, settings, work);
            return;
        }
        let op = open(&self.fs, fid, settings);
        if self.wm.open(op, Some(fid), work) {
            self.apply_saved_geometry(fid, work);
        }
    }

    // 일반 폴더는 항상 My Computer 창 안에서 드릴다운 탭으로 보여준다. 휴지통(이름이
    // 정확히 "Recycle Bin"인 폴더)만 예외로 별개의 독립 창(RecycleBinApp)에 연다 —
    // apps/mod.rs::open() 이 그 이름을 보고 ExplorerApp 대신 RecycleBinApp 을 골라준다.
    fn is_drilldown_folder(&self, id: FileId) -> bool {
        let node = self.fs.get(id);
        matches!(node.kind, FileKind::Folder { .. }) && node.name != RECYCLE_BIN_NAME
    }

    // 폴더를(바탕화면 아이콘이든 다른 경로로든) 열 때 항상 이걸 거친다 — My Computer
    // 가 열려있으면 그 창 "안에서" 드릴다운 탭으로 보여주고, 안 열려있으면 My Computer
    // 자체를 이 폴더가 활성 탭인 상태로 새로 연다.
    fn open_folder_in_explorer(&mut self, folder_id: FileId, settings: &Rc<RefCell<Settings>>, work: Rect) {
        let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME) else { return };
        let app = explorer_app_for_folder(&self.fs, explorer_id, folder_id, settings);
        if self.wm.is_open(explorer_id) {
            self.wm.refresh_app(explorer_id, app);
            self.wm.focus(explorer_id);
        } else {
            let lang = settings.borrow().language;
            let op = Opened {
                app,
                title: display_name(lang, &self.fs.get(explorer_id).name).into_owned(),
                size: (500.0, 340.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                min_size: (340.0, 260.0),
            };
            if self.wm.open(op, Some(explorer_id), work) {
                self.apply_saved_geometry(explorer_id, work);
            }
        }
    }

}

impl Scene for DesktopScene {
    fn update(&mut self, f: &mut Frame) -> Transition {
        let m = f.input.mouse;
        let click = f.input.mouse_clicked;
        let ty = SCREEN_H - TASKBAR_H;
        let work = Rect::new(0.0, 0.0, SCREEN_W, ty);
        let mut consumed = false;
        let mut shutdown = false;
        let mut erase = false;
        let lang = f.settings.borrow().language;
        self.tick_icon_glitch(f.dt);

        // "Erase All Memory" 확인창 — 이 모달이 떠 있는 동안은 시작메뉴/아이콘/다른
        // 창 등 화면의 무엇도 클릭에 반응하면 안 되므로, 아래에서 나머지 로직이 보는
        // click 을 아예 꺼버린다. 모달 자체의 버튼 판정은 꺼지기 전의(raw) 클릭으로
        // 여기서 먼저 처리한다.
        let modal_was_open = self.erase_confirm || self.idiot_confirm;
        if self.erase_confirm {
            let (_, erase_btn, cancel_btn) = Self::erase_confirm_layout();
            if click {
                if erase_btn.contains(m.0, m.1) {
                    self.erase_confirm = false;
                    erase = true;
                } else if cancel_btn.contains(m.0, m.1) {
                    self.erase_confirm = false;
                }
            }
        }
        if self.idiot_confirm {
            // "Yes" 하나뿐이라 다른 선택지로 빠져나갈 방법이 없다 — 최소화/최대화/
            // 크기조절/닫기 버튼 자체가 없는 화면 전체 모달이라(진짜 창이 아니다)
            // 이 버튼 말고는 어디를 눌러도 안 닫힌다.
            let yes_btn = Self::idiot_confirm_layout();
            if click && yes_btn.contains(m.0, m.1) {
                self.idiot_confirm = false;
            }
        }
        let click = click && !modal_was_open;
        let input_blocked = modal_was_open;

        let bg_idx = f.settings.borrow().bg_color_idx;
        let bg_color = crate::foundation::BG_COLORS.get(bg_idx).map_or(TEAL, |(_, c)| *c);
        f.r.rect(0.0, 0.0, SCREEN_W, SCREEN_H, bg_color);

        // 1) 시작 메뉴 입력
        if self.start_open && click {
            let mr = self.start_menu_rect(f.r, lang);
            if mr.contains(m.0, m.1) {
                consumed = true;
                let idx = ((m.1 - (mr.y + 4.0)) / 24.0) as i32;
                if idx == 3 {
                    shutdown = true;
                } else {
                    self.run_menu_action(idx, f.settings.clone(), work);
                }
                self.start_open = false;
            } else if !self.start_button_rect().contains(m.0, m.1) {
                self.start_open = false;
            }
        }

        // 1.5) 빈 바탕화면 우클릭 컨텍스트 메뉴 (Official Site / Credits / Settings 로 바로 이동)
        // 메뉴가 열려있는 상태에서 다른 빈 자리를 다시 우클릭하면, 그 자리에 메뉴를
        // 새로 띄운다(먼저 닫혔다가 다시 열리는 게 아니라 바로 옮겨가는 느낌).
        if f.input.right_clicked
            && m.1 < ty
            && self.icon_hit(m).is_none()
            && !self.wm.any_window_at(m)
        {
            self.context_menu = Some(m);
            // 컨텍스트 메뉴를 띄울 때 다른 떠있는 패널/팝업들은 다 닫는다
            // (와이파이 정보 팝업, 시작 메뉴).
            self.wifi_info = None;
            self.start_open = false;
        } else if let Some(pos) = self.context_menu
            && click
        {
            let mr = Self::context_menu_rect(f.r, lang, pos);
            if mr.contains(m.0, m.1) {
                consumed = true;
                let idx = ((m.1 - (mr.y + 3.0)) / CTX_ROW_H) as i32;
                self.run_menu_action(idx, f.settings.clone(), work);
            }
            self.context_menu = None;
        }

        // 2) 작업표시줄 입력
        if click && m.1 >= ty && !consumed {
            consumed = true;
            if self.start_button_rect().contains(m.0, m.1) {
                self.start_open = !self.start_open;
            } else if self.wifi_rect().contains(m.0, m.1) {
                // 열려있으면 닫고, 닫혀있으면 그제서야 조회한다(SSID/IP 조회가 가벼운 건
                // 아니라서 매 프레임/아이콘 그릴 때마다 하지 않고 누를 때만 한다).
                self.wifi_info =
                    if self.wifi_info.is_some() { None } else { Some(query_wifi_info(self.wifi_connected)) };
            } else {
                for (id, _title, rect, _f, _mn) in self.taskbar_buttons() {
                    if rect.contains(m.0, m.1) {
                        self.wm.on_taskbar_click(id);
                        break;
                    }
                }
            }
        }

        // 2.6) 와이파이 정보 팝업: 팝업 위 클릭은 흡수, 바깥을 클릭하면 닫는다.
        if self.wifi_info.is_some() && click && !consumed {
            if self.wifi_popup_rect().contains(m.0, m.1) {
                consumed = true;
            } else {
                self.wifi_info = None;
            }
        }

        // 3) 바탕화면 아이콘 (창 뒤에 먼저 그림)
        self.draw_icons(f.r, f.assets, lang);

        // 고무줄 선택 박스 진행 중이면 매 프레임 선택을 갱신하고 그린다 — 창들보다 먼저
        // 그려서 항상 창 아래(뒤)에 깔리게 한다 (마우스를 떼기 전에도 실시간으로 갱신).
        if let Some(ms) = self.marquee_start
            && f.input.mouse_down
            && !input_blocked
        {
            let mr = Self::marquee_rect(ms, m);
            self.update_marquee_selection(mr);
            f.r.rect(mr.x, mr.y, mr.w, mr.h, [0.2, 0.4, 0.9, 0.25]);
            border(f.r, mr.x, mr.y, mr.w.max(1.0), mr.h.max(1.0), [0.2, 0.4, 0.9, 0.9]);
        }

        // 4) 창들 (모달/크래시 중이면 드래그/슬라이더 조작도 안 먹게 down 도 같이 꺼둔다)
        let gui = Gui {
            mouse: m,
            down: f.input.mouse_down && !input_blocked,
            clicked: click && !consumed,
            wheel: f.input.wheel,
            dt: f.dt,
            time: f.time,
            input: f.input,
        };
        let res = self.wm.frame(&mut *f.ctx, f.r, f.assets, &gui, work);
        if res.consumed {
            consumed = true;
        }
        let wm_cursor = res.cursor;
        let over_window = res.over_window;
        // 지금 열려있는 창들의 크기/위치를 매 프레임 기억해둔다 — 이동/리사이즈 중에도
        // 실시간으로 갱신되고, 창을 닫아도 이 맵엔 마지막 값이 그대로 남아있어서 다음에
        // 다시 열 때(또는 종료 전 자동저장 때) 그 자리로 복원된다.
        for (fid, rect, maximized) in self.wm.window_geometry() {
            self.window_geometry.insert(fid, (rect, maximized));
        }
        // File Explorer 안에서 창 밖으로도 나가는 드래그 미리보기가 있으면 받아둔다 —
        // 창을 다 그린 뒤(이 시점 이후) 맨 위에 클립 없이 그려서 창 경계를 넘나든다.
        let explorer_drag_ghost = res.drag_ghost;
        for a in res.actions {
            match a {
                DeskAction::Unlock(id) => {
                    self.fs.unlock(id);
                    self.secret_unlocked = true;
                    // 잠금 풀린 폴더는 별도 창을 새로 안 띄운다 — File Explorer 가
                    // 열려있으면 그 창 안에서 바로 (원래 있던 카테고리의 하위 폴더로)
                    // 들어가 보여주고, 안 열려있으면 다음에 열 때 자연히 폴더 아이콘으로 보인다.
                    if let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME)
                        && self.wm.is_open(explorer_id)
                    {
                        let app = explorer_app_for_folder(&self.fs, explorer_id, id, &f.settings);
                        self.wm.refresh_app(explorer_id, app);
                    }
                    // 5초 주기 자동저장을 기다리다 그 사이에 창이 갑자기 닫히면(작업
                    // 관리자로 강제 종료 등, ShutdownScene 을 안 거치는 경우) 방금 한
                    // 행동이 통째로 날아간다 — 그래서 실제로 상태가 바뀌는 이벤트마다
                    // 그 즉시 한 번 더 저장해둔다(항목별 즉시 저장).
                    self.write_save(&f.settings);
                }
                DeskAction::Open(id) => {
                    // 폴더는(잠금 풀린 폴더 포함) 항상 File Explorer 창 안에서 드릴다운
                    // 탭으로 보여준다 — 열려있으면 그 창 안에서, 안 열려있으면 새로 연다.
                    // 휴지통만 예외 — 별개의 프로그램(RecycleBinApp)으로 독립된 창에 연다.
                    if self.is_drilldown_folder(id) {
                        self.open_folder_in_explorer(id, &f.settings, work);
                    } else {
                        let op = open(&self.fs, id, &f.settings);
                        if self.wm.open(op, Some(id), work) {
                            self.apply_saved_geometry(id, work);
                        }
                    }
                }
                DeskAction::OpenPhoto(filename) => {
                    // Photos 피드에서 보는 창과 My Computer(Explorer/Downloads 탭)에서
                    // 보는 창은 완전히 별개로 취급한다 — 같은 사진이어도 fs.nodes 에
                    // 등록된 FileId 를 아예 안 쓴다(그건 "다운로드해서 실제로 갖고
                    // 있는 파일"에만 붙는 정체성이라, apps::open() 의 FileKind::Photo
                    // 분기에서만 만든다). 여기선 wm.open() 에 file: None 을 넘겨서,
                    // My Computer 쪽에 그 사진 창이 이미 열려있어도 서로 겹쳐 앞으로
                    // 당겨지는 일 없이 완전히 독립된 새 창(Download 버튼 있는 미리보기)
                    // 이 뜬다 — 그래서 "이미 다운로드했으니 다시 열면 Download 버튼이
                    // 안 보인다"가 Photos 피드 쪽에는 절대 영향을 안 준다.
                    //
                    // 다만 같은 사진을 피드 안에서 여러 번 클릭했을 때도 매번 새 창이
                    // 뜨는 건 원치 않으므로, 파일명에서 결정적으로 뽑아낸 가짜 FileId
                    // (photo_preview_win_id) 를 dedup 키로 넘긴다 — 진짜 FileId 대역과
                    // 안 겹치니 My Computer 와의 분리는 유지하면서, 피드 안에서의 중복
                    // 클릭만 기존 창을 앞으로 당기도록 만든다.
                    // filename 은 assets/photo 하위 폴더까지 포함한 식별자("corpseImage/
                    // corpseImage1.jpg")일 수 있어서, 창 제목에는 마지막 조각(파일명)만
                    // 보여준다.
                    let lang = f.settings.borrow().language;
                    let title_name = filename.rsplit('/').next().unwrap_or(&filename);
                    let op = Opened {
                        app: Box::new(PhotoViewerApp::new(filename.clone(), true)),
                        title: display_name(lang, title_name).into_owned(),
                        size: (420.0, 320.0),
                        maximized: false,
                        resizable: true,
                        maximizable: true,
                        movable: true,
                        min_size: (150.0, 90.0),
                    };
                    self.wm.open(op, Some(photo_preview_win_id(&filename)), work);
                }
                DeskAction::RequestErase => self.erase_confirm = true,
                DeskAction::Download(id) => {
                    self.fs.download(id);
                    // 지금 File Explorer 가 열려있으면 Downloads 탭에 바로 반영되도록
                    // 새로고침한다 — 안 그러면 창을 닫았다 다시 열어야만 보인다. Mail/
                    // HexTool 의 파일 선택 목록도 같은 이유로 같이 새로고침한다 — 안
                    // 그러면 방금 받은 파일을 그 창들에서 곧장 첨부/검토할 수가 없었다
                    // (다시 열어야만 보이던 문제).
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    // 다운로드 직후 그 즉시 저장 — 5초 자동저장을 기다리는 사이 창이
                    // 닫히면 방금 다운로드한 기록이 통째로 사라지는 문제가 있었다.
                    self.write_save(&f.settings);
                }
                DeskAction::DownloadPhoto(filename) => {
                    let id = self.fs.find_or_add_photo(&filename);
                    self.fs.download(id);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    self.write_save(&f.settings);
                }
                DeskAction::ExportPhotoReport(photos) => {
                    // HexTool 검수를 마쳤다 — 이상현상을 하나도 못 찾았으면 photos 가
                    // 비어있는데, 그래도 "검수는 끝났다" 자체는 기록해야 다음에 HexTool
                    // 을 열 때 로딩 게이지를 또 안 보여준다.
                    self.fs.photos_pending_review = false;
                    if !photos.is_empty() {
                        // 처음이면 바탕화면에 새 아이콘 자리를 잡아주고(add_desktop_icon
                        // 과 같은 요령), 이미 압축파일이 있었으면 set_photo_report() 가
                        // 내용만 갈아끼우므로 자리는 그대로 둔다.
                        let (id, is_new) = self.fs.set_photo_report(photos);
                        if is_new {
                            self.fs.desktop.push(id);
                            let (fc, fr) = self.first_free_tile();
                            self.icon_pos.push(Self::tile_to_pos(fc, fr));
                        }
                        self.refresh_explorer_if_open(&f.settings);
                        self.refresh_mail_attachable_if_open();
                    }
                    self.write_save(&f.settings);
                }
                DeskAction::InstallComplete => {
                    // HexTool Setup.exe 마법사를 Finish 까지 끝냈다 — 이제부터 .tar 를
                    // 열면 archive.rs 가 "설치 안 됨" 대신 다른 안내를 보여주고, 실제
                    // 프로그램을 설치한 것처럼 바탕화면에 아이콘도 생긴다.
                    self.fs.hex_tool_installed = true;
                    self.add_desktop_icon("HexTool", FileKind::HexTool);
                    self.write_save(&f.settings);
                }
                DeskAction::DeletePermanently(id) => {
                    // HexTool 로 검토를 끝낸 .tar 를 실제로 지운다 — 어디 있었든(바탕화면/
                    // Downloads/폴더 안) 다 사라지므로, 열려있던 File Explorer 가 있으면
                    // 바로 반영되도록 새로고침한다. 바탕화면에 있었을 수도 있으니(드래그로
                    // 옮겨왔을 경우) icon_pos 와 짝을 맞춰 먼저 떼어낸다.
                    if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                        self.fs.desktop.remove(pos);
                        self.icon_pos.remove(pos);
                    }
                    self.fs.trash_origin.retain(|&(tid, _)| tid != id);
                    self.fs.delete_permanently(id);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    self.write_save(&f.settings);
                }
                DeskAction::MoveFiles(ids, dest) => {
                    // ExplorerApp 은 자기 사이드바만 알아서, 창 밖으로 드롭되면 일단
                    // Desktop 이라고 요청한다 — 그 지점이 실은 다른 탐색기 창이나
                    // 바탕화면의 다른 폴더/휴지통 아이콘 위였을 수도 있어 여기서
                    // 재확인한다. 단, 드롭 지점이 My Computer 자기 자신이면(=사이드바
                    // "Desktop" 항목 위에 놓은 정상 경우) 재확인을 건너뛴다 —
                    // current_location() 은 "지금 보이는 탭"이라, 재확인하면 사이드바에서
                    // 고른 게 아니라 마침 보고 있던 다른 탭 위치로 잘못 덮어써버린다.
                    let my_computer = self.fs.find_by_name(MY_COMPUTER_NAME);
                    let window_at_drop = self.wm.file_at(m);
                    let dest = if matches!(dest, MoveDest::Desktop) && window_at_drop != my_computer {
                        self.explorer_drop_target_at(m)
                            .or_else(|| self.desktop_folder_drop_target_at(m, &[]).map(MoveDest::Folder))
                            .unwrap_or(dest)
                    } else {
                        dest
                    };
                    // 바탕화면행인데 그 드롭 지점을 어떤 창이든(My Computer 사이드바
                    // 자신 포함 — 그 좌표는 창 안의 좌표라 화면상 빈 자리와 무관하다)
                    // 가리고 있으면, 그대로 내려놓지 않고 first_free_tile 로 확실히
                    // 비어 보이는 자리에 자동 정렬해서 놓는다(휴지통 Restore 와 같은
                    // 이유 — 창 뒤에 숨으면 옮긴 게 아니라 없어진 것처럼 보인다).
                    let drop_at = if matches!(dest, MoveDest::Desktop) && window_at_drop.is_some() {
                        let (fc, fr) = self.first_free_tile();
                        Self::tile_to_pos(fc, fr)
                    } else {
                        m
                    };
                    self.move_ids_to(&ids, dest, drop_at);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_recycle_bin_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    self.write_save(&f.settings);
                }
                DeskAction::Restore(ids) => {
                    self.restore_from_trash(&ids);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_recycle_bin_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    self.write_save(&f.settings);
                }
                DeskAction::MarkMailRead(i) => {
                    // fs 에 기록해야 3초 주기 새로고침은 물론 게임 재시작 후에도 읽음
                    // 표시가 유지된다 — MailApp 자체의 read 는 창이 새로 만들어질
                    // 때마다(apps/mod.rs::open() 이 매번 fs.mail_read 로 다시 초기화)
                    // 사라지는 순전히 화면용 상태라 여기(fs)가 진짜 기록이다.
                    if !self.fs.mail_read.contains(&i) {
                        self.fs.mail_read.push(i);
                        self.write_save(&f.settings);
                    }
                }
                DeskAction::SendNewMail { to, subject, body, attachments } => {
                    // 재연구 업무 메일이 시킨 "이상현상 검수 보고"를 실제로 해내면
                    // (REPORT_EMAIL 앞으로, HexTool 검수 결과 압축파일(FileKind::
                    // PhotoReport)을 첨부해 보내면) ????? 피드를 새로 갱신한다 — 이게
                    // "?????가 완전 랜덤이 아니라 특정 조건을 만족해야 바뀐다"의 그
                    // 조건이다. 예전엔 검수 없이 이상현상 사진을 개별로 바로 첨부해도
                    // 인정됐는데, 이제 HexTool 로 순차 검수해 압축파일로 묶어 보내는
                    // 흐름으로 바뀌었다. 첨부 목록을 SentMail 로 옮기기(move) 전에 먼저
                    // 확인해야 한다.
                    let sent_report = to.trim().eq_ignore_ascii_case(REPORT_EMAIL)
                        && attachments.iter().any(|&(id, _)| matches!(&self.fs.get(id).kind, FileKind::PhotoReport(_)));
                    // Mail 의 "Write Mail" 탭에서 완전히 새로 작성해 보낸 메일 — 내용째
                    // fs.sent_mail 에 쌓아서 Mail 앱의 "Sent Items" 탭에 그대로 보여준다.
                    self.fs.sent_mail.push(SentMail { to, subject, body, attachments });
                    if sent_report {
                        refresh_photos_feed(&mut self.fs);
                        self.refresh_photos_if_open(&f.settings);
                    }
                    self.write_save(&f.settings);
                }
                DeskAction::EmptyTrash(ids) => {
                    // 휴지통의 "Empty Recycle Bin" — DeletePermanently 와 같은 요령으로
                    // 하나씩 영구히 지운다(바탕화면에 있었을 리는 없지만 혹시 몰라 같은
                    // 안전장치를 그대로 둔다).
                    for id in ids {
                        if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                            self.fs.desktop.remove(pos);
                            self.icon_pos.remove(pos);
                        }
                        self.fs.trash_origin.retain(|&(tid, _)| tid != id);
                        self.fs.delete_permanently(id);
                    }
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_recycle_bin_if_open(&f.settings);
                    self.refresh_mail_attachable_if_open();
                    self.write_save(&f.settings);
                }
            }
        }

        // 5) 바탕화면 아이콘 클릭 / 드래그 (최하위)
        if !consumed && click {
            if let Some(i) = self.icon_hit(m) {
                if self.last_idx == i as i32 && f.time - self.last_click < 0.35 {
                    self.open_file(i, &f.settings, work);
                    self.drag = None;
                    self.last_idx = -1;
                } else {
                    // 이미 여러 개가 선택된 상태에서 그 중 하나를 누르면 선택을 유지한 채
                    // 다같이 옮길 수 있게 하고, 선택 안 된 아이콘을 누르면 그것만 새로 선택한다.
                    if !self.selected.contains(&i) {
                        self.selected = vec![i];
                    }
                    let group_start = self.selected.iter().map(|&gi| (gi, self.icon_pos[gi])).collect();
                    // 오프셋은 타일(icon_pos, 64x60) 왼쪽 위가 아니라 실제로 그려지는
                    // 아이콘 글리프 위치(draw_one_icon 의 x+IC_W/2-IC_SIZE/2, y+3 와
                    // 똑같은 계산) 기준으로 잡아야 한다 — 안 그러면 고스트가 처음
                    // 그려지는 순간 실제 아이콘 글리프 자리에서 옆으로 살짝 어긋나
                    // "잡은 위치가 안 맞는" 느낌이 났다.
                    let glyph = (self.icon_pos[i].0 + IC_W / 2.0 - IC_SIZE / 2.0, self.icon_pos[i].1 + 3.0);
                    let offset = (m.0 - glyph.0, m.1 - glyph.1);
                    self.drag = Some(IconDrag { start: m, group_start, offset, moved: false });
                    self.last_idx = i as i32;
                }
                self.last_click = f.time;
            } else {
                // 빈 바탕화면을 누르면 고무줄(마퀴) 선택 시작.
                self.selected.clear();
                self.last_idx = -1;
                self.marquee_start = Some(m);
            }
        }

        // 드래그 이동(버튼 누른 채) — 실제 아이콘 위치(icon_pos)는 여기서 안 바꾼다.
        // 원래 자리에 그대로 둔 채로, 아래에서 반투명 복사본(고스트)만 커서를
        // 따라다니게 그린다(File Explorer 드래그와 같은 느낌) — 실제 이동은 손을
        // 놓는 순간에 한 번만 계산해서 적용한다.
        if !consumed
            && f.input.mouse_down
            && let Some(d) = &mut self.drag
        {
            let dx = m.0 - d.start.0;
            let dy = m.1 - d.start.1;
            d.moved = d.moved || dx * dx + dy * dy > 16.0;
        }

        // 놓기: 드래그 종료
        let released = self.prev_down && !f.input.mouse_down;
        if released {
            if let Some(ms) = self.marquee_start.take() {
                let mr = Self::marquee_rect(ms, m);
                self.update_marquee_selection(mr);
            }
            if let Some(d) = self.drag.take()
                && d.moved
            {
                // 놓은 자리가 열려있는 File Explorer 창 위면 바탕화면이 아니라 그
                // 창이 보여주는 폴더/카테고리로 옮긴다 — id 는 인덱스가 밀리기 전에
                // 먼저 모아둬야 한다(move_ids_to 가 fs.desktop 에서 실제로 빼버린다).
                // 창이 없으면(바탕화면 자체), 놓은 자리에 다른 폴더 아이콘이 있는지도
                // 확인해서(자기 자신은 제외) 있으면 그 폴더 안으로 넣는다 — 창 쪽을
                // 먼저 확인하는 건 창이 아이콘보다 위에 그려져서 실제로 보이는 게
                // 창이면 그쪽이 우선이어야 자연스럽기 때문이다.
                let dragged: Vec<usize> = d.group_start.iter().map(|&(gi, _)| gi).collect();
                let explorer_dest = self
                    .explorer_drop_target_at(m)
                    .or_else(|| self.desktop_folder_drop_target_at(m, &dragged).map(MoveDest::Folder));
                if let Some(dest) = explorer_dest {
                    let ids: Vec<FileId> = d.group_start.iter().filter_map(|&(gi, _)| self.fs.desktop.get(gi).copied()).collect();
                    self.move_ids_to(&ids, dest, m);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_recycle_bin_if_open(&f.settings);
                } else if self.wm.file_at(m).is_some() {
                    // 옮길 대상은 못 되는 다른 창(폴더로 못 옮기는 Notepad/Mail 등)이
                    // 그 자리를 가리고 있다 — 그대로 놓으면 아이콘이 창 뒤에 숨어버리니
                    // icon_pos 를 안 건드려 드래그를 취소한 것처럼 둔다(폴더 창으로 옮기는
                    // 경우는 위 explorer_dest 에서 이미 처리돼 여기까지 안 온다).
                } else {
                    // 실제로 안 움직였던 icon_pos 에 이제야 델타(드래그 시작점부터 놓은
                    // 지점까지)를 한 번에 적용해서 목표 칸을 정한다.
                    let dx = m.0 - d.start.0;
                    let dy = m.1 - d.start.1;
                    for &(gi, gp) in &d.group_start {
                        if gi < self.icon_pos.len() {
                            let (c, r) = Self::tile_of((gp.0 + dx, gp.1 + dy));
                            let (fc, fr) = self.nearest_free_tile(c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW), gi);
                            self.icon_pos[gi] = Self::tile_to_pos(fc, fr);
                        }
                    }
                }
                // 아이콘을 옮기거나 옮겨놓은 자리도 그 즉시 저장 — 5초 자동저장을
                // 기다리다 그 사이에 닫으면 방금 한 일이 반영 안 된 채로 남는다.
                self.write_save(&f.settings);
            }
        }
        self.prev_down = f.input.mouse_down;

        // 와이파이 상태는 매 프레임 물어보기엔 아까우니 몇 초마다 한 번만 갱신한다.
        const WIFI_CHECK_INTERVAL: f32 = 3.0;
        self.wifi_check_timer += f.dt;
        if self.wifi_check_timer >= WIFI_CHECK_INTERVAL {
            self.wifi_check_timer = 0.0;
            self.wifi_connected = network_connected();
        }

        // File Explorer/Mail 은 주기적으로 무조건 새로고침하지 않는다 — 그렇게 하면
        // 트리에서 방금 펼친 하위 폴더처럼 새로고침이 다시 만들어낼 수 없는 로컬
        // 상태(현재 활성 탭이 아닌 다른 카테고리를 펼쳐둔 상태 등)가 몇 초마다 조용히
        // 사라져 보이는 문제가 있었다. 대신 다운로드/잠금해제/메일도착처럼 fs 내용이
        // 실제로 바뀌는 이벤트가 생길 때 그 즉시(refresh_explorer_if_open/
        // refresh_mail_if_open 직접 호출) 새로고침한다.

        // 첫 메일 도착 — 데스크톱에 들어오고 MAIL_ARRIVAL_DELAY 초 뒤에 한 번만 발생.
        if MAIL_AUTO_ARRIVE && !self.fs.mail_arrived {
            self.mail_timer += f.dt;
            if self.mail_timer >= MAIL_ARRIVAL_DELAY {
                self.fs.mail_arrived = true;
                self.refresh_mail_if_open(&f.settings);
                let lang = f.settings.borrow().language;
                let subject = t(lang, secrets::PALACE_MAIL_SUBJECT);
                self.toast = Some(("test@mail.com".to_string(), subject.to_string()));
                self.toast_timer = TOAST_DURATION;
                self.write_save(&f.settings);
            }
        }
        if self.toast_timer > 0.0 {
            self.toast_timer -= f.dt;
            if self.toast_timer <= 0.0 {
                self.toast = None;
            }
        }

        // 6) 작업표시줄 + 시작 메뉴 (맨 위)
        // 한 번만 계산해서 그리기와 아래 커서 판정에 같이 쓴다 (제목 문자열 복제+정렬
        // 비용이 있어서 프레임당 여러 번 부르지 않는 게 좋다).
        let taskbar_buttons = self.taskbar_buttons();
        self.draw_taskbar(f.r, &taskbar_buttons, f.time);
        if self.start_open {
            self.draw_start_menu(f.r, m, lang);
        }
        if let Some(pos) = self.context_menu {
            self.draw_context_menu(f.r, pos, m, lang);
        }
        if self.wifi_info.is_some() {
            self.draw_wifi_popup(f.r, lang);
        }
        if !self.erase_confirm {
            self.update_toast(f, work, lang);
        }

        // 파일을 드래그로 옮기는 중이면(바탕화면 아이콘이든 File Explorer 안이든)
        // 반투명 복사본이 커서를 따라다니게 그린다 — 여기는 창 클립도, 바탕화면
        // 클립도 안 걸린 채로 그려서 창 경계나 화면 밖으로도 자유롭게 나갈 수 있다.
        if let Some(ghost) = &explorer_drag_ghost {
            draw_drag_ghost(f.r, f.assets, &ghost.icon, &ghost.label, ghost.pos);
        }
        if let Some(d) = &self.drag
            && d.moved
            && let Some(&(gi, _)) = d.group_start.first()
            && gi < self.fs.desktop.len()
        {
            let fid = self.fs.desktop[gi];
            let node = self.fs.get(fid);
            let icon = icon_of(node);
            let label = if d.group_start.len() == 1 {
                display_name(lang, &node.name).into_owned()
            } else {
                t(lang, explorer::ITEMS_COUNT).replace("{n}", &d.group_start.len().to_string())
            };
            let pos = (m.0 - d.offset.0, m.1 - d.offset.1);
            draw_drag_ghost(f.r, f.assets, &icon, &label, pos);
        }

        // "Erase All Memory" 확인창 — 진짜 모달이라 화면의 다른 모든 것(다른 창, 시작
        // 메뉴, 컨텍스트 메뉴까지) 을 덮도록 맨 마지막(가장 위)에 그린다.
        if self.erase_confirm {
            self.draw_erase_confirm(f.r, f.time, m, lang);
        }
        if self.idiot_confirm {
            self.draw_idiot_confirm(f.r, m);
        }

        // 자동 저장: 주기적으로, 그리고 종료할 때 한 번 더 확실히 저장한다.
        // (Erase All Memory 로 지우는 도중이면 절대 다시 써서 되살리면 안 된다.)
        self.save_timer += f.dt;
        if self.save_timer >= AUTOSAVE_INTERVAL && !erase {
            self.save_timer = 0.0;
            self.write_save(&f.settings);
        }
        if shutdown {
            self.write_save(&f.settings);
        }

        // 커서: 창 관리자가 이미 뭔가(리사이즈/이동) 정했으면 그게 우선이고, 아니면
        // 클릭 가능한 데스크톱 요소 위나 아이콘 드래그 중엔 Hand 로.
        //
        // wm_cursor 는 wm.frame() 이 "지금 마우스 아래 가장 위에 보이는 창의 테두리
        // 인지"만 보고 계산한 값이라, 시작메뉴/우클릭메뉴/와이파이 팝업/새 메일
        // 토스트처럼 창들보다 나중에(desktop.rs 가 직접) 그 위에 그리는 오버레이는
        // wm.frame() 입장에선 존재조차 모른다 — 그래서 이런 오버레이가 마침 그
        // 뒤에 있는 창의 리사이즈 테두리 위에 겹쳐 있으면, 화면엔 분명 오버레이가
        // 보이는데 커서만 그 아래 창의 리사이즈 화살표로 바뀌는 문제가 있었다. 지금
        // 열려있는 오버레이 위에 마우스가 있으면 wm_cursor 를 무시하고 Arrow 부터
        // 다시 시작한다 — 그중 실제로 클릭 가능한 항목(시작메뉴/우클릭메뉴 행)은
        // 바로 아래 Hand 판정에서 다시 Hand 로 바뀐다.
        let over_overlay = self.erase_confirm
            || self.idiot_confirm
            || (self.start_open && self.start_menu_rect(f.r, lang).contains(m.0, m.1))
            || self.context_menu.is_some_and(|pos| Self::context_menu_rect(f.r, lang, pos).contains(m.0, m.1))
            || (self.wifi_info.is_some() && self.wifi_popup_rect().contains(m.0, m.1))
            || (self.toast.is_some() && Self::toast_rect().contains(m.0, m.1));
        f.cursor = if over_overlay { CursorKind::Arrow } else { wm_cursor };
        if f.cursor == CursorKind::Arrow {
            let dragging_icon = self.drag.as_ref().is_some_and(|d| d.moved);
            let over_clickable = self.start_button_rect().contains(m.0, m.1)
                || self.wifi_rect().contains(m.0, m.1)
                || (self.start_open && self.start_menu_rect(f.r, lang).contains(m.0, m.1))
                || self.context_menu.is_some_and(|pos| Self::context_menu_rect(f.r, lang, pos).contains(m.0, m.1))
                || taskbar_buttons.iter().any(|(_, _, r, ..)| r.contains(m.0, m.1))
                || (!over_window && !consumed && self.icon_hit(m).is_some());
            if dragging_icon || over_clickable {
                f.cursor = CursorKind::Hand;
            }
        }

        if erase {
            // 저장 파일(게임 진행 상태)만 지우고, 그냥 바로 로비 화면으로 넘기지
            // 않고 지지직거리는 "메모리 삭제 중" 연출부터 보여준다 — 그 연출이
            // 끝나면 EraseScene 이 알아서 LobbyScene 으로 넘어간다. 언어/그래픽
            // 취향(Settings)은 게임 진행과 무관한 별도 파일에 저장되므로 여기서
            // 기본값으로 되돌리지 않는다 — Erase All Memory 를 눌렀다고 언어까지
            // 초기화되면 당황스럽다.
            crate::foundation::delete();
            Transition::Switch(Box::new(EraseScene::new()))
        } else if shutdown {
            // 바로 Transition::Quit 하지 않고 "종료 중" 연출부터 보여준다 — 그 연출이
            // 끝나면 ShutdownScene 이 알아서 Transition::Quit 을 반환한다.
            Transition::Switch(Box::new(ShutdownScene::new()))
        } else {
            Transition::None
        }
    }
}
