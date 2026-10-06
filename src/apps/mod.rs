//! 창 안에 들어가는 앱들의 공통 틀(App 트레잇/AppAction/WinInput/Opened) + 파일을
//! 열 때 어떤 앱을 띄울지 정하는 open(). 앱 하나하나는 파일별로 나뉘어 있다 —
//! 새 앱을 추가할 땐 이 디렉터리에 파일 하나 만들고, 아래 `mod` 목록에 추가하고,
//! FileKind 에 해당하는 경우라면 open() 의 match 에 한 줄만 더하면 된다.

mod credits;
mod doors_game;
mod game3d;
mod game_installer;
mod explorer;
mod image_viewer;
mod mail;
mod maze_game;
mod notepad;
mod official_site;
mod password;
mod recycle_bin;
mod settings;
mod sound_player;
mod video_player;
mod widgets;

pub use credits::CreditsApp;
pub use doors_game::DoorsGameApp;
pub use game_installer::GameInstallerApp;
pub use explorer::{ExplorerApp, ExplorerLocation};
pub use image_viewer::ImageViewerApp;
pub use maze_game::MazeGameApp;
pub use mail::{mail_from, mail_subject, MailApp, MailAttachment, SentMailView};
pub use notepad::NotepadApp;
pub use official_site::OfficialSiteApp;
pub use password::PasswordApp;
pub use recycle_bin::RecycleBinApp;
pub use settings::SettingsApp;
pub use sound_player::SoundApp;
pub use video_player::VideoApp;

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{display_name, FileId, FileKind, FileOrigin, FileSystem, Settings, APPDATA_NAME};
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::scenes::Input;
use crate::ui::{icon_of, IconType};

// 창 안의 앱이 받는 입력. 마우스는 절대 가상좌표(앱도 절대좌표로 그린다).
pub struct WinInput<'a> {
    pub mouse: (f32, f32),
    pub mouse_down: bool,
    pub mouse_clicked: bool,
    pub focused: bool,
    pub wheel: f32,
    pub dt: f32,
    pub time: f32,
    pub input: &'a Input,
}

// 앱이 데스크톱에 요청하는 동작.
pub enum AppAction {
    None,
    Close,
    Unlock(FileId),          // 비밀번호 성공 → 잠금파일을 폴더로
    Open(FileId),            // 탐색기에서 자식 열기
    RequestErase,            // 설정의 "Erase All Memory" → 화면 전체를 덮는 확인창을 띄워달라는 요청
    Download(FileId),        // 메일 첨부파일 "Download" → File Explorer 의 Downloads 탭에 추가
    Resize(f32, f32),        // 이 창의 크기를 (너비,높이)로 바꿔달라는 요청 — 중심은 그대로 두고 크기만
    DeletePermanently(FileId), // 파일을 영구히 지워달라는 요청
    MoveFiles(Vec<FileId>, MoveDest), // File Explorer 에서 사이드바로 드래그해 옮긴 파일들
    EmptyTrash(Vec<FileId>),   // 휴지통의 "Empty Recycle Bin" — 안의 항목들을 전부 영구히 지운다
    MarkMailRead(usize),       // Mail 에서 메시지(인덱스)를 읽었다 — fs.mail_read 에 기록해야 재시작 후에도 유지된다
    Restore(Vec<FileId>),      // 휴지통의 "Restore" — fs.trash_origin 에 기록된 원래 위치로 되돌린다
    // Mail 의 "Write Mail" 탭에서 새 메일을 작성해 보냄 — fs.sent_mail 에 내용째 쌓는다.
    // 첨부는 여러 개를 붙일 수 있어서 Vec(순서대로 붙인 순서).
    SendNewMail { to: String, subject: String, body: String, attachments: Vec<(FileId, String)> },
    // 설치 마법사의 진행바가 다 참 — fs.game_installed 를 켜고 바탕화면에 게임
    // 아이콘(test.exe)을 만들어달라는 요청. 마법사 창은 그대로 둔다(Finish 로 닫음).
    InstallComplete,
    // 게임 안에서 방에 꽃이 없다는 걸 확인함 — 두 번째 메일(꽃을 구할 수 있는 곳을 안다는)을
    // 도착시켜달라는 요청. 여러 번 와도 한 번만 도착한다.
    FlowerAbsenceChecked,
    // 문 게임에서 편지를 우편함에 넣음 — 같은 내용의 편지 메일이 도착하게 해달라는 요청.
    LetterSent,
    // 서브 게임에서 씨앗을 심음 — 잠시 뒤 시간 힌트 메일이 오게 해달라는 요청.
    SeedPlanted,
    // 서브 게임에서 꽃을 우체통에 넣음 — 꽃 사진이 첨부된 메일이 오게 해달라는 요청.
    FlowerSent,
}

// File Explorer 사이드바 드래그로 파일을 옮길 수 있는 대상 — Desktop/Downloads 는
// FileId 가 없는 특수 위치라 이름으로 구분하고, 실제 폴더는 FileId 로 가리킨다.
// Videos/Images 는 종류(Mp4/Img)로만 모아 보여주는 가상 카테고리라 옮길 대상이 될 수 없다.
#[derive(Clone, Copy)]
pub enum MoveDest {
    Desktop,
    Downloads,
    Folder(FileId),
}

pub trait App {
    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction;
    // 새로고침 시 앱별 상태(예: File Explorer 의 현재 탭 위치)를 되살리려고 downcast
    // 하는 데 쓴다. 트레잇 기본 메서드로는(Self: Sized 필요) dyn App 을 통해 못 불러서
    // 앱마다 직접 구현해야 한다 — 본문은 항상 `{ self }` 하나뿐.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    // 지금 파일을 드래그해 옮기는 중이면 그 미리보기(고스트) 정보를 돌려준다 — 대부분의
    // 앱은 해당 없어서 기본값은 None. File Explorer 처럼 창 밖으로도 삐져나가야 하는
    // 드래그가 있는 앱만 오버라이드한다. update() 안에서 그리지 않고 굳이 이렇게 밖으로
    // 빼내는 이유는, update() 는 window_manager 가 그 창의 client area 로 클립을 걸어둔
    // 채로 부르기 때문에(gfx.rs::push_quad) 그 안에서 그리면 창 경계 밖으로 못 나가기
    // 때문이다 — desktop.rs 가 모든 창을 다 그린 뒤 이 값을 받아 클립 없이 맨 위에 그린다.
    fn drag_ghost(&self) -> Option<DragGhost> {
        None
    }
    // 창 제목이 언어 설정처럼 매 프레임 바뀔 수 있는 값에서 나오는 앱만 오버라이드
    // 한다 — window_manager.rs::frame() 이 매 프레임 이 값을 물어봐서 Some 이면 그
    // 창의 타이틀바 문구를 갈아끼운다. 대부분의 앱은 제목이 스토리 파일 이름 같은
    // 고정값이라(언어와 무관) 기본값 None 그대로 두면 처음 열 때 준 title 이 계속
    // 쓰인다 — 그래야 창이 열려있는 도중에 언어를 바꿔도(Settings 탭에서든 어디서든)
    // 그 즉시 이미 열려있는 창의 타이틀바까지 같이 바뀐다.
    fn title(&self) -> Option<String> {
        None
    }
}

// App::drag_ghost 가 돌려주는 드래그 미리보기 — 이미 클릭 오프셋이 반영된 위치.
pub struct DragGhost {
    pub icon: IconType,
    pub label: String,
    pub pos: (f32, f32),
}

// 파일을 열어 (앱, 창 제목, 초기 크기) 를 만든다.
pub struct Opened {
    pub app: Box<dyn App>,
    pub title: String,
    pub size: (f32, f32),
    pub maximized: bool,
    pub resizable: bool,   // 테두리를 끌어서 자유롭게 크기 조절 가능한지
    pub maximizable: bool, // 타이틀바의 최대화/복원 버튼이 먹는지 (resizable 과 별개인 창이 있을 수 있다)
    pub movable: bool,     // 타이틀바 드래그로 위치를 옮길 수 있는지
    pub min_size: (f32, f32), // 리사이즈로 줄일 수 있는 최소 크기 — 레이아웃이 겹치지 않는 선
}

// "Write Mail" 에서 첨부로 고를 수 있는 파일 — 바탕화면/Downloads 에 있는 실제
// 파일들 중 폴더류(My Computer/Recycle Bin 포함)와 Mail 자기 자신은 뺀다(폴더를
// "첨부"한다는 개념 자체가 없고, 메일함 자체를 첨부할 수도 없다). open() 과
// desktop.rs 의 새로고침(다운로드 등으로 fs 가 바뀐 뒤 이미 열린 Mail 창에
// 최신 목록을 다시 넣어주는 것) 양쪽에서 똑같은 계산이 필요해서 함수로 뺐다.
pub(crate) fn mail_attachable_files(fs: &FileSystem) -> Vec<(FileId, String, IconType)> {
    let attachable_ids: Vec<FileId> = {
        let mut ids: Vec<FileId> = fs.desktop.iter().chain(fs.downloads.iter()).copied().collect();
        ids.sort_unstable();
        ids.dedup();
        ids.retain(|&fid| !matches!(fs.get(fid).kind, FileKind::Folder { .. } | FileKind::Explorer | FileKind::Mail { .. }));
        ids
    };
    folder_items(fs, &attachable_ids)
}

pub fn open(fs: &FileSystem, id: FileId, settings: &Rc<RefCell<Settings>>) -> Opened {
    let node = fs.get(id);
    let lang = settings.borrow().language;
    // raw_name 은 "Recycle Bin" 같은 문자열 매칭(아래 FileKind::Folder 가드) 등
    // 로직에 쓰는 언어 불변 식별자, name 은 그걸 화면에 실제로 보여줄 창 제목으로
    // 바꾼 것 — My Computer/Recycle Bin/Mail 같은 시스템 특수 항목만 display_name()
    // 으로 언어별 표시 이름을 쓰고, Photos.tar/Setup.exe 처럼 이야기 소품으로 등장하는
    // 실제 "파일" 이름은 실제 OS에서도 언어에 따라 안 바뀌므로 원문 그대로 둔다.
    let raw_name = node.name.clone();
    let name = display_name(lang, &raw_name).into_owned();
    match &node.kind {
        FileKind::Txt(text) => Opened {
            app: Box::new(NotepadApp::new(text.clone(), settings.clone())),
            title: name,
            size: (320.0, 240.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (150.0, 90.0),
        },
        FileKind::Mp4 => Opened {
            app: Box::new(VideoApp::new(settings.clone())),
            title: name,
            size: (340.0, 280.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (150.0, 90.0),
        },
        &FileKind::Img(idx) => Opened {
            app: Box::new(ImageViewerApp::new(idx)),
            title: name,
            size: (420.0, 320.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (150.0, 90.0),
        },
        &FileKind::Sound(idx) => Opened {
            app: Box::new(SoundApp::new(idx, settings.clone())),
            title: name,
            size: (300.0, 130.0),
            maximized: false,
            resizable: false,   // 작은 플레이어라 크기 고정
            maximizable: false,
            movable: true,
            min_size: (300.0, 130.0),
        },
        FileKind::Lock { password, .. } => Opened {
            app: Box::new(PasswordApp::new(id, password.clone(), settings.clone())),
            title: name,
            size: (260.0, 140.0),
            maximized: false,
            resizable: false,   // 대화상자는 크기 고정
            maximizable: false, // 최대화도 의미 없음
            movable: true,
            min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
        },
        FileKind::Folder { children } if raw_name == crate::foundation::RECYCLE_BIN_NAME => {
            let items = folder_items(fs, children);
            Opened {
                app: Box::new(RecycleBinApp::new(items, settings.clone())),
                title: name,
                // 메뉴바+툴바+주소창+상태바(=100) 를 두르고도 왼쪽 안내 패널의 아이콘+
                // 제목+색줄+설명 문단+링크가 다 들어갈 높이가 필요해서 기존 폴더뷰보다
                // 좀 더 세로로 넉넉하게 잡았다.
                size: (400.0, 340.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                min_size: (300.0, 260.0),
            }
        }
        FileKind::Folder { children } => {
            let items = folder_items(fs, children);
            Opened {
                app: Box::new(ExplorerApp::new(items, raw_name.clone(), settings.clone())),
                title: name,
                // 탭이 없어도 메뉴바/툴바/주소창/상태바 크롬은 그대로 두르므로 여유를 둔다.
                size: (420.0, 320.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                min_size: (300.0, 220.0),
            }
        }
        FileKind::Mail { .. } => {
            let attachable = mail_attachable_files(fs);
            // 보낸 메일함(Sent Items) — fs.sent_mail 을 그대로 스냅샷으로 넘긴다.
            // 첨부파일 아이콘은 attachable 과 같은 요령으로 여기서 미리 구해둔다.
            let sent = fs
                .sent_mail
                .iter()
                .map(|m| {
                    let attachments = m.attachments.iter().map(|(aid, name)| (*aid, name.clone(), icon_of(fs.get(*aid)))).collect();
                    SentMailView { to: m.to.clone(), subject: m.subject.clone(), body: m.body.clone(), attachments }
                })
                .collect();
            // 첫 메일의 첨부(게임 파일) — 아이콘까지 미리 구해서 넘긴다. 이미 한 번
            // 받은 적 있으면(ever_downloaded) 다시 "Download" 버튼이 안 뜨게 표시.
            let attachments: Vec<Option<MailAttachment>> = fs
                .mail_attachments()
                .into_iter()
                .map(|a| {
                    a.map(|aid| MailAttachment {
                        id: aid,
                        name: fs.get(aid).name.clone(),
                        icon: icon_of(fs.get(aid)),
                        downloaded: fs.ever_downloaded.contains(&aid),
                    })
                })
                .collect();
            Opened {
                app: Box::new(MailApp::new(&fs.mail_log, &fs.mail_read, attachments, attachable, sent, settings.clone())),
                title: name,
                // Outlook Express/Exchange 참고 레이아웃 — 메뉴바 + 폴더 트리(150) +
                // 상태바(20)까지 들어가야 해서 기존보다 좌우/위아래로 넉넉해야 한다.
                // 툴바는 만들었다가 다시 뺐다(기능 없는 버튼이 오히려 헷갈려서).
                // 스크린샷으로 받은 참고 창 크기에 맞춰 조정.
                size: (470.0, 340.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                // 폴더 트리(150) + 내용 최소 폭(필드 라벨 76 + 필드 칸 + 여백) + 메뉴바
                // (20)/상태바(20) 높이 + 뒤로가기 줄/필드 블록/본문/첨부파일 박스가
                // 겹치지 않는 선.
                min_size: (400.0, 320.0),
            }
        }
        FileKind::Explorer => Opened {
            app: Box::new(ExplorerApp::new_tabbed(explorer_tabs(fs, id), raw_name.clone(), settings.clone())),
            title: name,
            // 주소창+트리+상태바까지 두른 고전 탐색기 크롬이 다 들어가게 넉넉히.
            size: (500.0, 340.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (340.0, 260.0),
        },
        // 메일로 받은 "test Setup.exe" — 설치 마법사. 이미 설치됐으면 곧장 "이미
        // 설치됨" 페이지로 연다. 대화상자라 크기 고정.
        FileKind::GameSetup => Opened {
            app: Box::new(GameInstallerApp::new(fs.game_installed, settings.clone())),
            title: name,
            size: (360.0, 220.0),
            maximized: false,
            resizable: false,
            maximizable: false,
            movable: true,
            min_size: (360.0, 220.0),
        },
        // 서브 게임(test2.exe) — 미로 게임. 창 안 3D 게임이라 test.exe 와 같은 창 설정.
        FileKind::SubGame => Opened {
            app: Box::new(MazeGameApp::new()),
            title: name,
            size: (486.0, 386.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (246.0, 206.0),
        },
        // 설치가 끝나 바탕화면에 생긴 게임(test.exe) — 별도 실행 파일이 아니라 이 OS 안의 창 하나로
        // 돈다. 게임 화면은 창 안에 4:3 으로 맞춰 넣으므로(남는 곳은 검은 띠) 크기
        // 조절/최대화도 그냥 허용한다. 기본 크기는 클라이언트 영역이 480x360(4:3)이
        // 되게 테두리(3*2)/타이틀바(20)만큼 더했다.
        FileKind::Game => Opened {
            app: Box::new(DoorsGameApp::new()),
            title: name,
            size: (486.0, 386.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (246.0, 206.0),
        },
        FileKind::Deleted => unreachable!("삭제된 파일은 그 무엇에서도 더는 참조되지 않아 열릴 일이 없다"),
    }
}

type ExplorerItems = Vec<(FileId, String, crate::ui::IconType)>;
// (탭 이름, 안의 항목들, 부모 카테고리 이름, 자기 자신의 FileId) — 부모가 있으면
// 그 카테고리의 하위 폴더로 취급해서 트리에서 들여쓰기하고 주소창에도 경로로 이어
// 보여준다. FileId 는 드릴다운 탭(폴더 자신)일 때만 Some — 새로고침 뒤에도 같은
// 폴더로 돌아가려고 ExplorerApp::current_location() 이 이걸로 식별한다.
type ExplorerTabs = Vec<(String, ExplorerItems, Option<String>, Option<FileId>)>;

// 항목 이름은 원문(raw fs 이름) 그대로 담아둔다 — 예전엔 여기서 display_name()
// 으로 미리 번역해서 넣었는데, 그러면 창을 이미 연 채로 언어를 바꿔도 이 문자열
// 자체가 그때 언어로 굳어있어서 안 바뀌었다(창 제목이나 트리 탭 라벨은 매 프레임
// 다시 번역해서 반영되는데, 이 목록/격자 항목 이름만 그러지 않았던 것). 이제
// explorer.rs 의 draw_list_view/icon_grid 가 그릴 때마다 display_name() 을 다시
// 불러서, 창이 열려있는 동안 언어를 바꿔도 그 자리에서 바로 반영된다.
fn folder_items(fs: &FileSystem, ids: &[FileId]) -> ExplorerItems {
    ids.iter().map(|&cid| (cid, fs.get(cid).name.clone(), icon_of(fs.get(cid)))).collect()
}

// File Explorer 의 고정 카테고리 4개(Downloads/Desktop/Videos/Images). Videos/Images 는
// 위치와 무관하게 종류로 찾고, Downloads 는 메일 등에서 실제로 "다운로드"한 파일만,
// Desktop 은 바탕화면 그대로(단, self_id — 지금 보고 있는 File Explorer 자기 자신
// — 는 목록에 안 나오게 뺀다. 안에서 자길 또 열 이유가 없다). 탭 이름 자체("Downloads"
// 등)는 트리 위치 찾기/드래그앤드롭 목적지 매칭에 쓰는 언어 불변 키라 번역하지
// 않고, explorer.rs 가 화면에 그릴 때만 category_label() 로 번역한다.
fn explorer_tabs(fs: &FileSystem, self_id: FileId) -> ExplorerTabs {
    let downloads = folder_items(fs, &fs.downloads);
    let appdata = fs.find_by_name(APPDATA_NAME).map_or_else(Vec::new, |id| match &fs.get(id).kind {
        FileKind::Folder { children } => folder_items(fs, children),
        _ => Vec::new(),
    });
    let desktop = folder_items(fs, &fs.desktop.iter().copied().filter(|&fid| fid != self_id).collect::<Vec<_>>());
    // 휴지통에 들어간 항목은 종류가 여전히 Mp4/Img 라도 이 가상 탭에서 뺀다 — 안 그러면
    // 휴지통 안에도 있고 Videos/Images 탭에도 그대로 남아 두 군데에 동시에 보인다.
    let videos = folder_items(fs, &fs.all_of_kind(|k| matches!(k, FileKind::Mp4)).into_iter().filter(|&id| !fs.in_recycle_bin(id)).collect::<Vec<_>>());
    let images = folder_items(fs, &fs.all_of_kind(|k| matches!(k, FileKind::Img(_))).into_iter().filter(|&id| !fs.in_recycle_bin(id)).collect::<Vec<_>>());
    vec![
        ("Downloads".to_string(), downloads, None, None),
        ("Desktop".to_string(), desktop, None, None),
        // AppData 폴더 노드의 내용 — 설치한 게임의 폴더가 여기 생긴다(gamefiles.rs).
        (APPDATA_NAME.to_string(), appdata, None, None),
        ("Videos".to_string(), videos, None, None),
        ("Images".to_string(), images, None, None),
    ]
}

// 폴더(folder_id)를 새 창 대신 이미 열려있는 File Explorer(explorer_id) 창 "안에서"
// 보여주는 ExplorerApp 을 만든다 — 어느 카테고리 안에 있던 폴더인지 찾아서 그 바로
// 아래(트리에서 들여쓰기된 하위 항목)에 끼워넣고 그걸 바로 활성 탭으로 삼는다
// (사이드바에서 다른 카테고리를 누르면 평소처럼 빠져나간다). 잠금 해제 직후나,
// DesktopScene 이 DeskAction::Open 처리 중 폴더를 발견하면 이걸로 wm.refresh_app()
// 을 호출한다 — 그래서 별도 팝업 창 없이 File Explorer 안에서 바로 이어서 보인다.
pub fn explorer_app_for_folder(
    fs: &FileSystem,
    explorer_id: FileId,
    folder_id: FileId,
    settings: &Rc<RefCell<Settings>>,
) -> Box<dyn App> {
    let mut tabs = explorer_tabs(fs, explorer_id);
    let in_category = |tabs: &ExplorerTabs, id: FileId| tabs.iter().position(|(_, items, ..)| items.iter().any(|(fid, ..)| *fid == id));
    // 폴더 안의 폴더(AppData	estssets 처럼)면 고정 카테고리에 바로 나오는 맨 위 조상까지
    // 거슬러 올라가서, 조상들을 전부 트리에 끼워 넣는다 — 안 그러면 한 칸 들어갈 때마다
    // 방금 지나온 폴더 탭이 사라져서 "밖으로 튕겨나간" 것처럼 보인다. chain 은 깊은 쪽부터.
    let mut chain = vec![folder_id];
    while in_category(&tabs, *chain.last().unwrap()).is_none() {
        match fs.locate(*chain.last().unwrap()) {
            Some(FileOrigin::Folder(p)) if !chain.contains(&p) => chain.push(p),
            _ => break,
        }
    }
    let parent_idx = in_category(&tabs, *chain.last().unwrap());
    // 맨 위 조상은 카테고리 탭 바로 아래, 그 아래 조상들은 차례로 이어서 끼워넣는다. 카테고리를
    // 못 찾았으면(이론상 안 생기지만 방어적으로) 맨 뒤에 붙인다.
    let first_at = parent_idx.map(|i| i + 1).unwrap_or(tabs.len());
    let mut parent_name = parent_idx.map(|i| tabs[i].0.clone());
    let mut insert_at = first_at;
    for (k, &fid) in chain.iter().rev().enumerate() {
        let node = fs.get(fid);
        let items = match &node.kind {
            FileKind::Folder { children } => folder_items(fs, children),
            _ => Vec::new(),
        };
        insert_at = first_at + k;
        tabs.insert(insert_at, (node.name.clone(), items, parent_name.clone(), Some(fid)));
        parent_name = Some(node.name.clone());
    }
    // 창 제목은 드릴다운으로 들어간 하위 폴더가 아니라 이 창이 원래 대표하는
    // 루트(explorer_id)를 계속 가리켜야 한다 — 실제 탐색기도 My Computer 창 안에서
    // 바탕화면 폴더로 들어가도 제목이 "바탕화면"으로 안 바뀌고 "내 컴퓨터"인 채다.
    let raw_title = fs.get(explorer_id).name.clone();
    Box::new(ExplorerApp::new_tabbed_active(tabs, insert_at, raw_title, settings.clone()))
}

// refresh_explorer_if_open 이 새로고침 직전에 저장해둔 위치(current_location())로
// 되돌아가는 버전의 새로고침 — Category 면 그 이름의 고정 탭을 다시 찾고, Folder
// 면 explorer_app_for_folder 로 그 폴더 하위 탭을 다시 끼워넣는다. 저장된 위치를
// 못 찾으면(예: 폴더가 사라짐) 그냥 첫 탭으로 돌아간다.
pub fn explorer_app_refreshed(
    fs: &FileSystem,
    explorer_id: FileId,
    loc: Option<ExplorerLocation>,
    settings: &Rc<RefCell<Settings>>,
) -> Box<dyn App> {
    let raw_title = fs.get(explorer_id).name.clone();
    match loc {
        Some(ExplorerLocation::Folder(folder_id)) => explorer_app_for_folder(fs, explorer_id, folder_id, settings),
        Some(ExplorerLocation::Category(name)) => {
            let tabs = explorer_tabs(fs, explorer_id);
            let active = tabs.iter().position(|(n, ..)| *n == name).unwrap_or(0);
            Box::new(ExplorerApp::new_tabbed_active(tabs, active, raw_title, settings.clone()))
        }
        None => Box::new(ExplorerApp::new_tabbed(explorer_tabs(fs, explorer_id), raw_title, settings.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::GAME_FOLDER_NAME;

    fn child(fs: &FileSystem, folder: FileId, name: &str) -> FileId {
        match &fs.get(folder).kind {
            FileKind::Folder { children } => children.iter().copied().find(|&c| fs.get(c).name == name).unwrap_or_else(|| panic!("{name}")),
            _ => panic!("폴더가 아니다"),
        }
    }

    // AppData\test 안의 assets\sound 처럼 폴더 안의 폴더로 들어가도, 지나온 폴더들(AppData → test → assets)
    // 이 트리에 그대로 남아야 한다 — 예전엔 한 칸 들어갈 때마다 부모 탭이 사라져 "밖으로 튕긴" 것처럼 보였다.
    #[test]
    fn nested_folder_keeps_its_ancestors_in_the_tree() {
        let mut fs = FileSystem::new();
        crate::gamefiles::install_into(&mut fs);
        let explorer = fs.find_by_name(crate::foundation::MY_COMPUTER_NAME).unwrap();
        let appdata = fs.find_by_name(APPDATA_NAME).unwrap();
        let test = child(&fs, appdata, GAME_FOLDER_NAME);
        let assets = child(&fs, test, "assets");
        let sound = child(&fs, assets, "sound");
        let settings = Rc::new(RefCell::new(Settings::default()));

        for (target, expected) in [
            (test, vec!["AppData", "test"]),
            (assets, vec!["AppData", "test", "assets"]),
            (sound, vec!["AppData", "test", "assets", "sound"]),
        ] {
            let mut app = explorer_app_for_folder(&fs, explorer, target, &settings);
            let app = app.as_any_mut().downcast_mut::<ExplorerApp>().unwrap();
            assert!(matches!(app.current_location(), Some(ExplorerLocation::Folder(id)) if id == target));
            let trail = app.tab_trail();
            let pos = trail.iter().position(|(n, _)| n == expected[0]).unwrap();
            // 조상 체인이 카테고리 바로 아래에 차례로 이어서 있고, 각 탭의 부모는 바로 위 탭이다.
            for (k, name) in expected.iter().enumerate().skip(1) {
                assert_eq!(trail[pos + k], (name.to_string(), Some(expected[k - 1].to_string())), "{expected:?}");
            }
            // 들여쓰기 깊이도 한 단계씩 늘어난다(카테고리 0, test 1, assets 2 ...).
            for (k, name) in expected.iter().enumerate() {
                let i = trail.iter().position(|(n, _)| n == name).unwrap();
                assert_eq!(app.tab_depth(i), k, "{name}");
            }
        }
    }
}
