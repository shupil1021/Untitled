//! 데스크톱 씬 (아이콘/작업표시줄/시작메뉴/컨텍스트메뉴/와이파이/시스템메시지패널).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::apps::{
    explorer_app_for_folder, explorer_app_refreshed, mail_attachable_files, open, CreditsApp, ExplorerApp, ExplorerLocation, MailApp,
    MoveDest, OfficialSiteApp, Opened, SettingsApp, mail_from, mail_subject,
};
use crate::foundation::{
    display_name, FileId, FileKind, FileOrigin, FileSystem, Language, SentMail, MailId, Settings, GAME_FILE_NAME, MY_COMPUTER_NAME, SUB_GAME_NAME,
    OFFICIAL_SITE_URL, RECYCLE_BIN_NAME,
};
use crate::render::gfx::{Assets, Rect, Renderer, CELL_H, SCREEN_H, SCREEN_W};
use crate::strings::{common, credits, desktop as s, explorer, official_site, settings, t, S};
use crate::ui::*;
use crate::window_manager::{DeskAction, Gui, WindowManager};

use super::{EraseScene, Frame, Scene, ShutdownScene, Transition};

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

// 설정/크레딧/공식 사이트 창을 구분하는 특수 FileId (실제 파일 아님).
const SETTINGS_WIN: FileId = usize::MAX - 1;
const CREDITS_WIN: FileId = usize::MAX - 2;
const OFFICIAL_SITE_WIN: FileId = usize::MAX - 3;

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
    // 게임을 새로 시작하고 MAIL_ARRIVAL_DELAY 초가 지나면 메일이 한 번 도착한다
    // (fs.mail_arrived 를 true 로) — 이미 도착했으면(불러온 저장에서 이미
    // true 였거나 이번 세션에서 이미 울렸으면) 더 안 잰다.
    mail_timer: f32,
    toast: Option<Toast>, // 우측 하단에 잠깐 뜨는 알림 — 없으면 안 보임
    hint_timer: f32,      // 씨앗을 심은 뒤 흐른 시간 — HINT_MAIL_DELAY 가 지나면 시간 힌트 메일이 온다
    clock_popup: bool,    // 시계를 눌러서 연 날짜 팝업
    sub_game_timer: f32,  // 두 번째 메일이 온 뒤 흐른 시간 — SUB_GAME_DELAY 가 지나면 서브 게임 아이콘이 생긴다
    toast_timer: f32,                // 위 알림이 사라지기까지 남은 시간
    erase_confirm: bool,   // "Erase All Memory" 확인창 — 화면 전체(다른 창 포함)를 덮는 진짜 모달
    // 창을 열었다 옮기거나 크기를 바꾼 적 있으면 마지막 자리를 여기 기억해둔다(파일
    // ID 로 키) — 지금 열려있는 창은 매 프레임 wm 에서 값을 다시 읽어와 갱신하고,
    // 닫힌 파일의 항목은 다음에 다시 열 때까지 그대로 남아있는다. write_save() 가
    // 이걸 그대로 저장한다.
    window_geometry: HashMap<FileId, (Rect, bool)>,
}

const AUTOSAVE_INTERVAL: f32 = 5.0; // 이 주기(초)마다 설정/바탕화면 상태를 자동 저장한다.
const MAIL_ARRIVAL_DELAY: f32 = 5.0; // 게임 시작 후 이만큼(초) 지나면 메일이 도착한다.
const TOAST_DURATION: f32 = 5.0; // 우측 하단 알림이 떠있는 시간(초)
const HINT_MAIL_DELAY: f32 = 12.0; // 씨앗을 심은 뒤 이만큼(초) 지나면 "시간을 돌려보라"는 힌트 메일이 온다
const SUB_GAME_DELAY: f32 = 6.0; // 두 번째 메일이 도착한 뒤 이만큼(초) 지나면 "다운로드 완료" + 서브 게임 아이콘

// 우측 하단 알림 한 개 — 제목줄/두 줄 내용. opens_mail 이면 누를 때 Mail 을 연다(새 메일
// 알림), 아니면 그냥 닫힌다(다운로드 완료 알림).
#[derive(Clone)]
struct Toast {
    title: S,
    line1: String,
    line2: String,
    opens_mail: bool,
}

impl Toast {
    // 새 메일 알림 — 보낸 사람 + 제목.
    fn new_mail(from: &str, subject: &str) -> Toast {
        Toast { title: s::NEW_MAIL, line1: from.to_string(), line2: subject.to_string(), opens_mail: true }
    }
}

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
        // icon_pos 는 항상 fs.desktop 과 같은 길이여야 한다 — 수동으로 손댄 저장
        // 파일 등으로 길이가 어긋나 있으면, 모자란 만큼 기본 격자 위치로 채운다.
        while icon_pos.len() < fs.desktop.len() {
            icon_pos.push(Self::grid_pos(icon_pos.len()));
        }
        icon_pos.truncate(fs.desktop.len());
        // 게임 파일 첨부가 생기기 전의 예전 저장 파일이면 지금 채워 넣는다.
        fs.ensure_game_attachment();
        fs.migrate_mail_log();
        fs.ensure_appdata();
        if fs.game_installed {
            // 게임 폴더가 생기기 전에 설치한 예전 저장 파일에도 파일들을 채워 넣는다.
            crate::gamefiles::install_into(&mut fs);
        }
        // Photos.lock 이 풀려서 폴더로 바뀌었는지는 이제 별도 플래그 없이 fs 스냅샷
        // 자체(이름이 이미 "Photos" 로 바뀌어 있는지)로 판단한다.
        let unlocked = fs.find_by_name("Photos").is_some();

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
            mail_timer: 0.0,
            toast: None,
            sub_game_timer: 0.0,
            hint_timer: 0.0,
            clock_popup: false,
            toast_timer: 0.0,
            erase_confirm: false,
            window_geometry,
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

        // "Erase All Memory" 확인창 — 이 모달이 떠 있는 동안은 시작메뉴/아이콘/다른
        // 창 등 화면의 무엇도 클릭에 반응하면 안 되므로, 아래에서 나머지 로직이 보는
        // click 을 아예 꺼버린다. 모달 자체의 버튼 판정은 꺼지기 전의(raw) 클릭으로
        // 여기서 먼저 처리한다.
        let modal_was_open = self.erase_confirm;
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
            self.clock_popup = false;
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
            } else if self.clock_rect().contains(m.0, m.1) {
                // 시계를 누르면 날짜 팝업 — 시스템 시간을 돌렸을 때 날짜가 실제로 바뀌었는지 확인할 수 있다.
                self.clock_popup = !self.clock_popup;
                self.wifi_info = None;
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

        // 2.5) 날짜 팝업: 팝업 위 클릭은 흡수, 바깥을 클릭하면 닫는다(시계를 누른 그 클릭은 위에서 이미 처리).
        if self.clock_popup && click && !consumed {
            if self.clock_popup_rect().contains(m.0, m.1) {
                consumed = true;
            } else {
                self.clock_popup = false;
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
            self.apply_desk_action(a, &f.settings, work, m);
        }

        // 5) 바탕화면 아이콘 클릭 / 드래그 / 놓기 (최하위) — icons.rs
        self.handle_icon_mouse(f, m, click, consumed, work);

        // 시간이 지나면 일어나는 일들(와이파이 갱신, 메일 도착, 서브 게임 다운로드 알림 등) — timers.rs.
        self.update_timers(f);

        // File Explorer/Mail 은 주기적으로 무조건 새로고침하지 않는다 — 그렇게 하면
        // 트리에서 방금 펼친 하위 폴더처럼 새로고침이 다시 만들어낼 수 없는 로컬
        // 상태(현재 활성 탭이 아닌 다른 카테고리를 펼쳐둔 상태 등)가 몇 초마다 조용히
        // 사라져 보이는 문제가 있었다. 대신 다운로드/잠금해제처럼 fs 내용이 실제로
        // 바뀌는 이벤트가 생길 때 그 즉시(refresh_explorer_if_open 직접 호출)
        // 새로고침한다.

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
        if self.clock_popup {
            self.draw_clock_popup(f.r, lang);
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
            || (self.start_open && self.start_menu_rect(f.r, lang).contains(m.0, m.1))
            || self.context_menu.is_some_and(|pos| Self::context_menu_rect(f.r, lang, pos).contains(m.0, m.1))
            || (self.wifi_info.is_some() && self.wifi_popup_rect().contains(m.0, m.1))
            || (self.clock_popup && self.clock_popup_rect().contains(m.0, m.1))
            || (self.toast.is_some() && Self::toast_rect().contains(m.0, m.1));
        f.cursor = if over_overlay { CursorKind::Arrow } else { wm_cursor };
        if f.cursor == CursorKind::Arrow {
            let dragging_icon = self.drag.as_ref().is_some_and(|d| d.moved);
            let over_clickable = self.start_button_rect().contains(m.0, m.1)
                || self.wifi_rect().contains(m.0, m.1)
                || self.clock_rect().contains(m.0, m.1)
                || (self.start_open && self.start_menu_rect(f.r, lang).contains(m.0, m.1))
                || self.context_menu.is_some_and(|pos| Self::context_menu_rect(f.r, lang, pos).contains(m.0, m.1))
                || taskbar_buttons.iter().any(|(_, _, r, ..)| r.contains(m.0, m.1))
                || (!over_window && !consumed && self.icon_hit(m).is_some())
                || (self.toast.is_some() && Self::toast_rect().contains(m.0, m.1));
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

mod actions;
mod clock;
mod icons;
mod network;
mod popups;
mod timers;

use network::{network_connected, query_wifi_info, WifiInfo};
