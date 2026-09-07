//! 창 안에 들어가는 앱들의 공통 틀(App 트레잇/AppAction/WinInput/Opened) + 파일을
//! 열 때 어떤 앱을 띄울지 정하는 open(). 앱 하나하나는 파일별로 나뉘어 있다 —
//! 새 앱을 추가할 땐 이 디렉터리에 파일 하나 만들고, 아래 `mod` 목록에 추가하고,
//! FileKind 에 해당하는 경우라면 open() 의 match 에 한 줄만 더하면 된다.

mod archive;
mod credits;
mod explorer;
mod image_viewer;
mod installer;
mod mail;
mod notepad;
mod official_site;
mod password;
mod photos;
mod recycle_bin;
mod settings;
mod hextool;
mod video_player;
mod widgets;

pub use archive::ArchiveApp;
pub use credits::CreditsApp;
pub use explorer::{ExplorerApp, ExplorerLocation};
pub use image_viewer::ImageViewerApp;
pub use installer::InstallerApp;
pub use mail::{MailApp, SentMailView};
pub use notepad::NotepadApp;
pub use official_site::OfficialSiteApp;
pub use password::PasswordApp;
pub(crate) use photos::{ensure_photos_selected, refresh_photos_feed};
pub use photos::{PhotoViewerApp, PhotosApp};
pub use recycle_bin::RecycleBinApp;
pub use settings::SettingsApp;
pub use hextool::HexToolApp;
pub use video_player::VideoApp;

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{display_name, FileId, FileKind, FileSystem, Settings};
use crate::gfx::{Assets, Rect, Renderer};
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
    OpenPhoto(String),       // Photos 피드에서 썸네일 클릭 → assets/photo 파일명으로
                             // FileKind::Photo 를 새로(또는 재사용해) 만들어 별개의 창으로 연다
    RequestErase,            // 설정의 "Erase All Memory" → 화면 전체를 덮는 확인창을 띄워달라는 요청
    Download(FileId),        // 메일 첨부파일 "Download" → File Explorer 의 Downloads 탭에 추가
    DownloadPhoto(String),   // Photos 앱의 "Download" → assets/photo 파일명으로 FileKind::Photo 를
                             // 새로(또는 이미 있으면 그걸 재사용해) 만들어 Downloads 탭에 추가
    InstallComplete,         // HexTool Setup.exe 마법사를 Finish 까지 끝냄 → hex_tool_installed 를 true 로
    Resize(f32, f32),        // 이 창의 크기를 (너비,높이)로 바꿔달라는 요청 — 중심은 그대로 두고 크기만
    DeletePermanently(FileId), // HexTool 검토를 마친 .tar 를 영구히 지워달라는 요청
    MoveFiles(Vec<FileId>, MoveDest), // File Explorer 에서 사이드바로 드래그해 옮긴 파일들
    EmptyTrash(Vec<FileId>),   // 휴지통의 "Empty Recycle Bin" — 안의 항목들을 전부 영구히 지운다
    MarkMailRead(usize),       // Mail 에서 메시지(인덱스)를 읽었다 — fs.mail_read 에 기록해야 재시작 후에도 유지된다
    Restore(Vec<FileId>),      // 휴지통의 "Restore" — fs.trash_origin 에 기록된 원래 위치로 되돌린다
    // Mail 의 "Write Mail" 탭에서 새 메일을 작성해 보냄 — fs.sent_mail 에 내용째 쌓는다.
    // 첨부는 여러 개를 붙일 수 있어서 Vec(순서대로 붙인 순서).
    SendNewMail { to: String, subject: String, body: String, attachments: Vec<(FileId, String)> },
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

// HexTool 에서 검토 대상으로 고를 수 있는 파일 — mail_attachable_files() 와 같은
// 이유로 open() 과 desktop.rs 의 새로고침 양쪽에서 재사용한다.
pub(crate) fn hextool_review_files(fs: &FileSystem) -> Vec<(FileId, String, IconType, Option<String>)> {
    fs.all_of_kind(|k| matches!(k, FileKind::Photo(_) | FileKind::Img(_) | FileKind::Mp4))
        .into_iter()
        .filter(|&id| !fs.in_recycle_bin(id))
        .map(|id| {
            let node = fs.get(id);
            let photo_id = match &node.kind {
                FileKind::Photo(filename) => Some(filename.clone()),
                _ => None,
            };
            (id, node.name.clone(), icon_of(node), photo_id)
        })
        .collect()
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
            Opened {
                app: Box::new(MailApp::new(fs.mail_arrived, &fs.mail_read, attachable, sent, fs.mail_hextool_attachment, settings.clone())),
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
        FileKind::Tar => Opened {
            app: Box::new(ArchiveApp::new(fs.hex_tool_installed, settings.clone())),
            title: name,
            // 안내문이 줄바꿈되더라도 너무 잘게 쪼개지지 않을 정도로 폭을 좀 넓혔다.
            size: (340.0, 160.0),
            maximized: false,
            resizable: false,
            maximizable: false,
            movable: true,
            min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
        },
        FileKind::Installer => Opened {
            app: Box::new(InstallerApp::new(settings.clone(), fs.hex_tool_installed)),
            title: name,
            size: (380.0, 260.0),
            maximized: false,
            resizable: false,
            maximizable: false,
            movable: true,
            min_size: (150.0, 90.0), // resizable 이 꺼져있어 실제로는 안 쓰임
        },
        FileKind::HexTool => {
            let review_files = hextool_review_files(fs);
            Opened {
                app: Box::new(HexToolApp::new(review_files, settings.clone())),
                title: name,
                // 이제 파일 선택용 별도 작은 창 없이 곧장 편집 화면(빈 미리보기 +
                // 슬라이더)으로 여니까, 처음부터 그 화면이 다 들어가는 크기로 연다.
                size: (420.0, 320.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                min_size: (360.0, 260.0),
            }
        }
        FileKind::Photo(filename) => Opened {
            // 이 경로(open())는 Explorer/Downloads 탭에서 더블클릭해서 여는
            // 경우에만 탄다 — Photos 피드에서 썸네일을 클릭하는 경로는
            // desktop.rs::DeskAction::OpenPhoto 가 이 함수를 거치지 않고 따로
            // PhotoViewerApp 을 만든다(show_download: true). 그래서 여기선
            // "이미 다운로드된 파일을 Explorer 로 다시 열었다"는 뜻이니 항상
            // false — Download 글자 자체를 안 보여준다.
            app: Box::new(PhotoViewerApp::new(filename.clone(), false)),
            title: name,
            size: (420.0, 320.0),
            maximized: false,
            resizable: true,
            maximizable: true,
            movable: true,
            min_size: (150.0, 90.0),
        },
        FileKind::PhotoGallery => Opened {
            // fs.photos_current 를 그대로 받는다 — 여기서 새로 뽑지 않는다(랜덤으로
            // 매번 바뀌지 않게 하려고 desktop.rs::DesktopScene::new() 가 미리
            // ensure_photos_selected() 로 채워둔 걸 그대로 쓴다).
            app: Box::new(PhotosApp::new(fs.photos_current.clone())),
            title: name,
            // 썸네일 3열x3행이 스크롤 없이 딱 맞게 보이는 고정 크기 — 사용자가
            // 준 스크린샷 크기 그대로. 크기 조절(드래그/최대화) 둘 다 막는다.
            size: (350.0, 350.0),
            maximized: false,
            resizable: false,
            maximizable: false,
            movable: true,
            min_size: (350.0, 350.0), // resizable 이 꺼져있어 실제로는 안 쓰임
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
    ids.iter().map(|&cid| { let c = fs.get(cid); (cid, c.name.clone(), icon_of(c)) }).collect()
}

// File Explorer 의 고정 카테고리 4개(Downloads/Desktop/Videos/Images). Videos/Images 는
// 위치와 무관하게 종류로 찾고, Downloads 는 메일 등에서 실제로 "다운로드"한 파일만,
// Desktop 은 바탕화면 그대로(단, self_id — 지금 보고 있는 File Explorer 자기 자신
// — 는 목록에 안 나오게 뺀다. 안에서 자길 또 열 이유가 없다). 탭 이름 자체("Downloads"
// 등)는 트리 위치 찾기/드래그앤드롭 목적지 매칭에 쓰는 언어 불변 키라 번역하지
// 않고, explorer.rs 가 화면에 그릴 때만 category_label() 로 번역한다.
fn explorer_tabs(fs: &FileSystem, self_id: FileId) -> ExplorerTabs {
    let downloads = folder_items(fs, &fs.downloads);
    let desktop = folder_items(fs, &fs.desktop.iter().copied().filter(|&fid| fid != self_id).collect::<Vec<_>>());
    // 휴지통에 들어간 항목은 종류가 여전히 Mp4/Img 라도 이 가상 탭에서 뺀다 — 안 그러면
    // 휴지통 안에도 있고 Videos/Images 탭에도 그대로 남아 두 군데에 동시에 보인다.
    let videos = folder_items(fs, &fs.all_of_kind(|k| matches!(k, FileKind::Mp4)).into_iter().filter(|&id| !fs.in_recycle_bin(id)).collect::<Vec<_>>());
    let images = folder_items(fs, &fs.all_of_kind(|k| matches!(k, FileKind::Img(_))).into_iter().filter(|&id| !fs.in_recycle_bin(id)).collect::<Vec<_>>());
    vec![
        ("Downloads".to_string(), downloads, None, None),
        ("Desktop".to_string(), desktop, None, None),
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
    let parent_idx = tabs.iter().position(|(_, items, ..)| items.iter().any(|(fid, ..)| *fid == folder_id));
    let parent_name = parent_idx.map(|i| tabs[i].0.clone());
    let node = fs.get(folder_id);
    let items = match &node.kind {
        FileKind::Folder { children } => folder_items(fs, children),
        _ => Vec::new(),
    };
    // 부모 카테고리를 찾았으면 바로 그 아래(트리에서 바로 다음 줄)에 끼워넣고, 못
    // 찾았으면(이론상 안 생기지만 방어적으로) 맨 뒤에 붙인다.
    let insert_at = parent_idx.map(|i| i + 1).unwrap_or(tabs.len());
    tabs.insert(insert_at, (node.name.clone(), items, parent_name, Some(folder_id)));
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
