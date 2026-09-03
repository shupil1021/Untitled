//! 메일 앱 — Outlook Express 스타일: 위에 메뉴바(장식용, 클릭 안 먹음), 그 아래
//! 왼쪽에 폴더 트리(Inbox + Write Mail — Sent Items/Deleted Items/Drafts 는 임시
//! 보관/삭제 기능 자체가 아직 없어서 트리에서 뺐다), 오른쪽엔 목록/읽기/작성 화면.
//! 왼쪽 트리와 오른쪽 내용을 나누는 세로줄은 드래그로 폭을 조절할 수 있다
//! (explorer.rs 의 사이드바와 같은 요령).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{display_name, FileId, Language, Settings};
use crate::gfx::{Assets, Color, Rect, Renderer, CELL_H};
use crate::secrets;
use crate::strings::{mail as s, t};
use crate::ui::*;

use super::widgets::{ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

struct MailMsg {
    from: &'static str,
    to: &'static str,
    cc: &'static str,
    subject: &'static str,
    body: &'static str,
    attachment: Option<(FileId, String)>,
}

// 보낸 메일함(Sent Items)에 보여줄 메일 한 통 — fs.sent_mail(SentMail, 순수
// 데이터)을 창을 열 때 아이콘까지 미리 구해서 스냅샷으로 만든 것(apps/mod.rs::open()).
// MailMsg 와 달리 사용자가 직접 입력한 값이라 &'static str 대신 String 을 쓴다.
pub struct SentMailView {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<(FileId, String, IconType)>,
}

// STORY.md 프롤로그의 "입사 안내 메일" — 새 게임을 시작하고 MAIL_ARRIVAL_DELAY 초
// 뒤에 도착하는 첫(그리고 지금은 유일한) 메일. arrived 가 false 면(아직 도착 전)
// 받은편지함이 비어있다 — DesktopScene 이 타이머로 도착시킨다. hextool_id 는
// FileSystem::new() 가 미리 만들어둔 실제 FileKind::Installer 노드(fs.mail_hextool_attachment,
// "HexTool Setup.exe")를 그대로 받아 첨부로 건다 — 다운로드해서 실행하면
// installer.rs 의 설치 마법사가 뜨고, Finish 까지 마쳐야 바탕화면에 진짜 HexTool
// 이 생긴다.
// from/to/cc 는 이메일 주소라 언어와 무관하게 그대로 두고, subject/body 만 t() 로
// 언어별 문구를 고른다(MailApp::update 가 설정 언어가 바뀔 때마다 다시 불러준다).
//
// 메일 제목/본문(스토리 스포일러)은 secrets.rs 에 따로 모아둔다 — 본문 안의
// secrets::PHOTOS_APP_NAME(5자)은 바탕화면 Photos 앱(foundation.rs::FileSystem::new())
// 과 같은 개체를 가리키는 상수를 공유한다. secrets::UNNAMED_ENTITY(7자)는 아직
// 게임에 실제로 등장하지 않는 무언가를 가리키는 복선용 자리표시자다 — 둘 다 폰트
// 아틀라스에 없는 키릴 문자라 항상 두부(마름모+물음표)로 보인다.
fn seed_messages(arrived: bool, lang: Language, hextool_id: FileId) -> Vec<MailMsg> {
    if !arrived {
        return Vec::new();
    }
    vec![MailMsg {
        from: "test@mail.com",
        to: "toast@mail.com",
        cc: "",
        subject: t(lang, secrets::PALACE_MAIL_SUBJECT),
        body: t(lang, secrets::PALACE_MAIL_BODY),
        attachment: Some((hextool_id, "HexTool Setup.exe".to_string())),
    }]
}

// 왼쪽 폴더 트리 항목 — Deleted Items/Drafts 는 삭제/임시보관 기능 자체가 아직
// 없어서(눌러도 항상 빈 상태 안내뿐이라 의미가 없었다) 트리에서 뺐다. Sent
// Items 는 "Write Mail" 에서 실제로 보낸 메일이 쌓이는 진짜 목록이라 있고,
// Compose("Write Mail")는 새 메일을 작성해서 보낼 수 있는 전용 탭이다.
#[derive(Clone, Copy, PartialEq)]
enum MailFolder {
    Inbox,
    Sent,
    Compose,
}

const MAIL_FOLDERS: [MailFolder; 3] = [MailFolder::Inbox, MailFolder::Sent, MailFolder::Compose];

impl MailFolder {
    fn label(self, lang: Language) -> &'static str {
        match self {
            MailFolder::Inbox => t(lang, s::FOLDER_INBOX),
            MailFolder::Sent => t(lang, s::FOLDER_SENT),
            MailFolder::Compose => t(lang, s::FOLDER_COMPOSE),
        }
    }
}

const ROW_PAD: f32 = 4.0;
const ROW_GAP: f32 = 3.0;
const TEXT_SCALE: f32 = 0.85;
// Download 버튼을 누른 뒤 실제로 다운로드가 끝나기까지 보여주는 가짜 로딩 시간(초).
const DOWNLOAD_DELAY: f32 = 1.4;

// 메뉴바/폴더 트리 — Outlook Express 참고 레이아웃. File/Edit/View/Go/Tools/Compose/
// Help 메뉴 항목들은 다 눌러도 아무 기능이 없는 장식이라 오히려 헷갈린다는 피드백을
// 받고 다 뺐다 — 대신 그 자리에 이 창이 뭔지 알려주는 라벨 하나만 남긴다.
const MENU_H: f32 = 20.0;
// 왼쪽 트리 폭의 기본값/최소값 — explorer.rs 의 사이드바와 같은 요령으로 드래그해서
// 조절할 수 있다(MailApp::tree_w). 최대값은 고정 px 대신 창 폭의 비율로 잡아서
// 창을 아주 좁게 줄여도 오른쪽 내용 칸이 완전히 안 없어지게 한다.
const FOLDER_TREE_W_DEFAULT: f32 = 150.0;
const FOLDER_TREE_W_MIN: f32 = 70.0;
const FOLDER_TREE_W_MAX_FRAC: f32 = 0.6;
const DIVIDER_W: f32 = 5.0; // 드래그 판정 폭(실제로 그려지는 선은 1px, 잡기 쉽게 여유를 둔다)
const FOLDER_ROW_H: f32 = 20.0;
// 읽는 화면 위의 "◀ Back to Inbox" 링크 줄 높이.
const BACK_ROW_H: f32 = 22.0;
// 맨 아래 상태바("N Item(s), N Unread") 높이.
const STATUS_H: f32 = 20.0;
// 목록 헤더("From"/"Subject") 높이 + From 칸 폭 비율.
const LIST_HEADER_H: f32 = 20.0;
// From 칸 폭 — 예전엔 110px 고정이라, 창을 넓게 열면 Subject 칸만 텅 비게 넓어지고
// From 칸은 그대로 좁아서 "PalaceCompany@email.com" 같은 긴 주소가 "PalaceC..." 로
// 심하게 잘려 두 칸의 비율이 안 맞아 보였다 — pane 폭의 비율로 잡아서 창 크기와
// 무관하게 두 칸이 항상 비슷한 느낌으로 나뉘게 하고, 그래도 너무 좁거나 넓어지지
// 않게 최소/최대값으로 막는다.
const FROM_COL_FRAC: f32 = 0.4;
const FROM_COL_MIN: f32 = 130.0;
// 창을 크게 열어도(사용자가 최대화하거나 기본 크기 자체를 키운 경우) From 칸이
// 230px 에서 멈춰 있으면 나머지 공간은 Subject 칸으로만 몰려서 균형이 안 맞아
// 보였다 — 최대값을 넉넉히 올려서 넓은 창에서는 From 도 같이 넓어지게 했다.
const FROM_COL_MAX: f32 = 320.0;
// 목록 각 행 아래에 본문 미리보기를 두 줄까지 곁들인다("자동 미리보기" 기능).
// 받은 시간 칸(Received)은 한 번 추가했다가 뺐다 — 그 칸에 넣을 폭을
// From/Subject 가 그대로 되돌려받아야 "글자가 안 잘린다"는 요청에 더 맞다고
// 판단했다(날짜 자체도 "없어도 된다"는 요청).
const PREVIEW_SCALE: f32 = 0.8;
const PREVIEW_LINES: usize = 2;
// Windows 자체의 IME 조합창(현대식 팝업 스타일이라 우리 CRT 픽셀아트 화면과
// 안 어울린다)을 아예 안 보이게 화면 밖으로 치워두는 좌표.
const OFFSCREEN: i32 = -10000;
// 메일 본문에 입력할 수 있는 최대 글자 수 — 다국어 입력을 감안해 글자(char) 단위로
// 세지, 바이트 단위로 세지 않는다(한/일 문자는 UTF-8 상 3바이트라 바이트 기준으로
// 재면 실제로 입력 가능한 분량이 언어마다 달라져 버린다).
const COMPOSE_MAX_CHARS: usize = 1000;

// "Write Mail" 탭에서 직접 입력하는 세 필드 중 지금 타이핑이 어디로 들어가는지.
#[derive(Clone, Copy, PartialEq)]
enum ComposeField {
    To,
    Subject,
    Body,
}

// "Write Mail" 탭 — 받는 사람/제목/본문을 전부 직접 입력하는 새 메일 작성 상태.
// MailApp 이 살아있는 동안(창을 닫기 전까지)만 유지되는 초안이라 fs 에 저장하지는
// 않는다 — 실제 메일 클라이언트도 임시보관함에 따로 옮기지 않는 한 창을 닫으면
// 초안이 사라지는 것과 같다. "보냈다"는 사실 자체(mail_sent_count)만 fs 에 남긴다.
struct NewMailState {
    to: String,
    subject: String,
    body: String,
    active: ComposeField,
    // 지금 활성 필드(active) 문자열 안에서 커서 위치(char 인덱스, 0..=len) —
    // 필드를 클릭하면 그 자리로, 타이핑/백스페이스/엔터를 치면 그 위치 기준으로
    // 삽입/삭제된다(끝에만 붙던 예전 방식과 달리 문장 중간을 클릭해서 그 자리부터
    // 고칠 수 있다).
    cursor: usize,
    // 첨부 가능한 파일 중 고른 것들 — 여러 개를 붙일 수 있다(붙인 순서 그대로).
    // 보낼 때 특별한 효과는 없지만(게임 진행에 영향 없음) 첨부 UI 자체를 Outlook
    // Express 스타일로 갖추기 위한 것.
    attachments: Vec<(FileId, String, IconType)>,
    picker_open: bool, // "Attach..." 를 눌러서 첨부할 파일을 고르는 목록이 펼쳐져 있는지
}

impl NewMailState {
    fn new() -> NewMailState {
        NewMailState { to: String::new(), subject: String::new(), body: String::new(), active: ComposeField::To, cursor: 0, attachments: Vec::new(), picker_open: false }
    }
}

pub struct MailApp {
    messages: Vec<MailMsg>,
    // seed_messages() 를 다시 부를 때 필요한 원본 인자 — messages 의 subject/body
    // 는 언어별로 번역되므로, 설정 언어가 바뀌면 이 값들로 다시 만들어야 한다
    // (update() 가 매 프레임 lang 을 확인해서 바뀌었으면 다시 만든다).
    arrived: bool,
    hextool_id: FileId, // seed_messages() 를 다시 부를 때(언어가 바뀔 때) 첨부에 또 넘겨줘야 한다
    built_lang: Language, // messages 를 마지막으로 만들 때 쓴 언어 — 캐시 무효화 키.
    // 왼쪽 폴더 트리에서 고른 폴더 — 처음엔 아무것도 안 골라서(Outlook Express 를
    // 막 열었을 때처럼) 오른쪽이 빈 안내 상태로 시작한다.
    folder: Option<MailFolder>,
    // 발신자를 아직 안 골랐으면 None — 메시지가 있어도 오른쪽에 내용을 강제로
    // 띄우지 않고 빈 상태로 둔다(사용자가 목록에서 직접 눌러야 나온다).
    selected: Option<usize>,
    // 메시지별 읽음 상태(messages 와 같은 길이) — 참고 이미지의 "N Item(s), N Unread"
    // 상태바처럼 안 읽은 개수를 세는 데 쓴다. select() 로 고르면 그 순간 읽음 처리한다.
    read: Vec<bool>,
    new_mail: NewMailState, // "Write Mail" 탭의 입력 상태 — 탭을 오갈 때도 그대로 유지된다
    // "Write Mail" 에서 첨부로 고를 수 있는 파일 목록 — 창을 열 때 desktop.rs 가
    // 스냅샷으로 넘겨준다(실시간으로 바뀌는 fs 를 MailApp 이 직접 들고 있지 않아서).
    attachable: Vec<(FileId, String, IconType)>,
    // 보낸편지함(Sent Items) 목록 — 창을 열 때 fs.sent_mail 스냅샷으로 시작하고,
    // "Write Mail" 에서 실제로 보내면 그 자리에서 바로 하나 덧붙인다(발송 즉시
    // 목록에 보이게 하려고 — fs 쪽은 SendNewMail 액션을 통해 desktop.rs 가 따로 채운다).
    sent: Vec<SentMailView>,
    downloaded: Vec<bool>, // 메시지별 다운로드 로컬 표시 상태 (messages 와 같은 길이)
    // 다운로드 버튼을 누른 뒤 바로 완료 처리하지 않고 잠깐 "로딩 중" 연출을 보여주려고
    // 쓴다 — Some(elapsed) 면 그 메시지가 지금 다운로드 진행 중, elapsed 는 지난 시간(초).
    downloading: Vec<Option<f32>>,
    list_scroll: f32,
    list_scroll_disp: f32,
    list_sb_drag: bool,
    body_scroll: f32,
    body_scroll_disp: f32,
    body_sb_drag: bool,
    body_wrapped: Vec<String>,
    body_wrapped_key: (usize, i32), // (선택된 메시지, 반올림한 폭 픽셀) — 캐시 무효화 키
    // Sent Items 읽기 화면도 같은 요령으로 줄바꿈을 캐싱하지만, Inbox 캐시와 필드를
    // 같이 쓰면 "Inbox 0번 읽다가 Sent 0번으로 넘어가도 인덱스가 같아 캐시가 그대로
    // 유효하다고 착각"해서 엉뚱한 본문이 잠깐 보일 수 있어 따로 둔다.
    sent_wrapped: Vec<String>,
    sent_wrapped_key: (usize, i32),
    tree_w: f32,       // 왼쪽 폴더 트리 폭 — 구분선을 드래그하면 바뀐다
    divider_drag: bool, // 지금 그 구분선을 드래그하는 중인지
    // Backspace 를 꾹 누르고 있으면 빠르게 반복 삭제되게 하는 데 쓴다 — 계속
    // 눌려있는 시간(초)을 직접 재서(OS 자동反복 이벤트는 안 씀) 처음 REPEAT_DELAY
    // 초까지는 한 번만, 그 뒤로는 REPEAT_INTERVAL 마다 하나씩 더 지운다.
    backspace_hold: f32,
    backspace_fired: u32, // 지금 이 hold 동안 이미 처리한 삭제 횟수(중복 처리 방지)
    // 지금 이어지는 한 번의 누름(키를 떼기 전까지)이 IME 조합 중에 시작됐으면
    // true — 사람이 키를 누르고 있는 몇 프레임 사이에 IME 가 조합을 끝내버려도
    // (그래서 그 뒤 프레임엔 composing 이 false 로 보여도), 같은 한 번의 누름
    // 안에서는 계속 IME 가 이미 처리한 것으로 치고 우리가 또 지우지 않는다.
    backspace_owned_by_ime: bool,
    // 지금 이어지는 조합(composition) 세션 동안, IME 가 화면에 보여준
    // 조합 문자열이 바뀔 때마다(글자가 늘어나거나 다른 모양으로 바뀔 때마다)
    // 그 스냅샷을 순서대로 쌓아둔다 — "ㅎ" → "호" → "화" 처럼. Backspace 를
    // 눌렀을 때 "한 단계 전엔 뭐였는지"를 (IME 내부 동작이 그 사이 뭘 했든
    // 상관없이) 이 기록에서 그대로 꺼내 쓴다(draw_new_compose 참고) — 조합이
    // 끝나면(커밋되거나 완전히 취소되면) 비운다.
    composition_history: Vec<String>,
    settings: Rc<RefCell<Settings>>,
}

impl MailApp {
    pub(super) fn new(
        arrived: bool, read_indices: &[usize], attachable: Vec<(FileId, String, IconType)>, sent: Vec<SentMailView>,
        hextool_id: FileId, settings: Rc<RefCell<Settings>>,
    ) -> MailApp {
        let lang = settings.borrow().language;
        let messages = seed_messages(arrived, lang, hextool_id);
        let read = (0..messages.len()).map(|i| read_indices.contains(&i)).collect();
        let downloaded = vec![false; messages.len()];
        let downloading = vec![None; messages.len()];
        MailApp {
            messages,
            arrived,
            hextool_id,
            built_lang: lang,
            folder: None,
            selected: None,
            read,
            new_mail: NewMailState::new(),
            attachable,
            sent,
            downloaded,
            downloading,
            list_scroll: 0.0,
            list_scroll_disp: 0.0,
            list_sb_drag: false,
            body_scroll: 0.0,
            body_scroll_disp: 0.0,
            body_sb_drag: false,
            body_wrapped: Vec::new(),
            body_wrapped_key: (usize::MAX, i32::MIN),
            sent_wrapped: Vec::new(),
            sent_wrapped_key: (usize::MAX, i32::MIN),
            tree_w: FOLDER_TREE_W_DEFAULT,
            divider_drag: false,
            backspace_hold: 0.0,
            backspace_fired: 0,
            backspace_owned_by_ime: false,
            composition_history: Vec::new(),
            settings,
        }
    }

    // 새로고침(refresh_mail_if_open) 전후로 선택 상태를 옮겨 담으려고 쓴다 — 안
    // 그러면 주기적 새로고침 때마다 골라둔 메시지가 도로 풀린다.
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn set_selected(&mut self, sel: Option<usize>) {
        // Inbox 는 self.messages, Sent Items 는 self.sent 를 본다 — 서로 길이가
        // 다를 수 있어서 지금 골라둔 폴더 기준으로 clamp 해야 한다.
        let len = if self.folder == Some(MailFolder::Sent) { self.sent.len() } else { self.messages.len() };
        self.selected = sel.filter(|&i| i < len);
    }

    // selected()/set_selected() 와 같은 이유 — 새로고침 전후로 골라둔 폴더(Inbox 등)도
    // 그대로 이어가야 한다. 안 그러면 메일 도착 새로고침 때마다 폴더 트리 선택이
    // 풀려서 "폴더를 선택하세요" 화면으로 도로 튕겨 보인다.
    pub fn folder_idx(&self) -> Option<usize> {
        self.folder.map(|f| MAIL_FOLDERS.iter().position(|&mf| mf == f).unwrap())
    }

    pub fn set_folder_idx(&mut self, idx: Option<usize>) {
        self.folder = idx.and_then(|i| MAIL_FOLDERS.get(i).copied());
    }

    // 다운로드/이동/삭제 등으로 fs 가 바뀐 뒤 desktop.rs 가 불러준다 — "Write Mail"
    // 에 작성 중이던 초안(NewMailState)은 그대로 두고 첨부로 고를 수 있는 목록만
    // 최신화한다(그래서 Mail 을 열어둔 채로 다른 창에서 파일을 받아도 다시 열지
    // 않고 바로 "Attach..." 목록에 나타난다).
    pub fn refresh_attachable(&mut self, attachable: Vec<(FileId, String, IconType)>) {
        self.attachable = attachable;
    }

    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        if let Some(read) = self.read.get_mut(i) {
            *read = true;
        }
        self.body_scroll = 0.0;
        self.body_scroll_disp = 0.0;
    }

    fn unread_count(&self) -> usize {
        self.read.iter().filter(|&&r| !r).count()
    }

    // From/Sent/To/Cc/Subject 라벨 한 줄 — 참고로 받은 실제 Outlook Express 읽기
    // 창처럼 상자 테두리 없이 "라벨: 값" 을 그냥 나란히 흘려 쓴다(예전엔 값 쪽에
    // 흰 배경 + 얇은 테두리를 둘러 입력칸처럼 보이게 했었는데, 참고 이미지는 이
    // 블록 전체가 그냥 평범한 텍스트 문단이라 그 쪽으로 다시 맞췄다).
    fn draw_field(&self, r: &mut Renderer, x: f32, y: f32, w: f32, label: &str, value: &str) {
        // "Subject:" (8글자) 기준으로 잡아야 라벨 뒤에 값이 겹치지 않는다.
        const LABEL_W: f32 = 76.0;
        r.text(x, y, label, 0.8, BLACK);
        let max_w = (w - LABEL_W).max(10.0);
        r.text_clipped(x + LABEL_W, y, value, 0.8, BLACK, max_w);
    }

    // 메뉴바 — 장식용. 실제 드롭다운 메뉴는 없고, Win9x 메뉴바 느낌만 낸다.
    fn draw_menu_bar(&self, r: &mut Renderer, area: Rect, lang: Language) {
        r.rect(area.x, area.y, area.w, MENU_H, FACE);
        let label = t(lang, s::MAILBOX_LABEL);
        r.text(area.x + 6.0, area.y + (MENU_H - CELL_H) / 2.0, label, 1.0, BLACK);
        r.rect(area.x, area.y + MENU_H - 1.0, area.w, 1.0, GRAY);
    }

    // 상태바 — 참고 이미지의 "N Item(s), N Unread" 그대로. Sent Items 는 안 읽음
    // 개념이 없어서 개수만. 나머지(Compose/미선택)는 셀 데이터가 없어 "0 Items"만.
    fn draw_status_bar(&self, r: &mut Renderer, area: Rect, lang: Language) {
        let text = match self.folder {
            Some(MailFolder::Inbox) if !self.messages.is_empty() => {
                let unread = self.unread_count();
                let n = self.messages.len();
                if unread > 0 {
                    t(lang, s::STATUS_ITEMS_UNREAD).replace("{n}", &n.to_string()).replace("{u}", &unread.to_string())
                } else {
                    t(lang, s::STATUS_ITEMS).replace("{n}", &n.to_string())
                }
            }
            Some(MailFolder::Sent) if !self.sent.is_empty() => {
                t(lang, s::STATUS_ITEMS).replace("{n}", &self.sent.len().to_string())
            }
            _ => t(lang, s::STATUS_ZERO_ITEMS).to_string(),
        };
        sunken(r, area.x, area.y, area.w, STATUS_H);
        let ty = area.y + (STATUS_H - CELL_H * 0.8) / 2.0;
        r.text_clipped(area.x + 8.0, ty, &text, 0.8, BLACK, area.w - 16.0);
    }

    // 참고 이미지(Outlook Express 웰컴 화면)의 폴더 트리 느낌을 살리되, 루트
    // 노드는 뺐다 — 이 트리는 애초에 "이 앱 자체"를 가리키는 노드가 하나 더
    // 있을 필요가 없어서(항상 펼쳐져 있고 접을 수도 없으니 그냥 자리만
    // 차지했다), 다섯 폴더가 곧바로 트리 맨 위부터 시작한다. 배경은 참고
    // 이미지처럼 흰색 — 이전엔 회색이었는데, 이번 리디자인에서 오른쪽 내용
    // 패널과 통일된 흰 바탕 위에 회색 테두리로 전체를 한 번 감싸는 식으로 바꿨다.
    fn draw_folder_tree(&mut self, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput, lang: Language) {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        let mut fy = area.y + 4.0;
        for folder in MAIL_FOLDERS {
            let row = Rect::new(area.x + 2.0, fy, area.w - 4.0, FOLDER_ROW_H);
            let hover = row.contains(win.mouse.0, win.mouse.1);
            let active = self.folder == Some(folder);
            if active {
                r.rect(row.x, row.y, row.w, row.h, [0.78, 0.88, 1.0, 1.0]);
                border(r, row.x, row.y, row.w, row.h, NAVY);
            } else if hover {
                r.rect(row.x, row.y, row.w, row.h, [0.0, 0.0, 0.4, 0.06]);
            }
            draw_icon(r, assets, &folder_icon(folder), row.x + 3.0, row.y + 2.0, 16.0);
            // 안 읽은 개수 배지는 Inbox 에만 의미가 있다 — Compose("Write Mail") 옆에도
            // 같은 숫자가 붙어버리는 걸 막는다.
            let unread = if matches!(folder, MailFolder::Inbox) { self.unread_count() } else { 0 };
            let has_unread = unread > 0;
            let label = if has_unread { format!("{} ({unread})", folder.label(lang)) } else { folder.label(lang).to_string() };
            let ty = row.y + (row.h - CELL_H * TEXT_SCALE) / 2.0;
            if has_unread {
                draw_bold_text(r, row.x + 22.0, ty, &label, TEXT_SCALE, BLACK);
            } else {
                r.text_clipped(row.x + 22.0, ty, &label, TEXT_SCALE, BLACK, row.w - 26.0);
            }
            if win.mouse_clicked && hover {
                self.folder = Some(folder);
                // 폴더를 (다시) 고르면 읽던 메시지에서 목록으로 돌아간다 — Inbox 를
                // 또 눌러서 "뒤로 가기" 대신으로 쓸 수 있게.
                self.selected = None;
            }
            fy += FOLDER_ROW_H;
        }
    }
}

// Inbox 는 실제 편지함 아이콘, 나머지 넷은 전용 아이콘이 없어서 평범한 폴더 아이콘을
// 그대로 쓴다 — 예전엔 icon_folder 에 폴더마다 다른 반투명 색을 덧칠해서 구분해봤는데,
// 흰 배경 위에서 색이 뭉개진 얼룩처럼 보여서 오히려 더 지저분해 보였다. 아이콘 형태
// 자체가 다른 게 아니면 색만 칠해서는 안 나아 보인다는 걸 확인하고 그만뒀다.
fn folder_icon(folder: MailFolder) -> IconType {
    match folder {
        MailFolder::Inbox | MailFolder::Sent => IconType::Envelope,
        MailFolder::Compose => IconType::Mail,
    }
}

// 비트맵 폰트에 굵은 글씨체가 따로 없어서, 1px 오른쪽으로 겹쳐 두 번 그리는 걸로
// 흉내낸다(레트로 UI에서 흔한 "가짜 볼드" 트릭) — Inbox(안 읽음 있음)/루트 라벨처럼
// 강조하고 싶은 한 줄에서만 쓴다.
fn draw_bold_text(r: &mut Renderer, x: f32, y: f32, text: &str, scale: f32, color: Color) {
    r.text(x, y, text, scale, color);
    r.text(x + 1.0, y, text, scale, color);
}

// 목록 행의 본문 미리보기(AutoPreview) — 최대 PREVIEW_LINES 줄까지 접되, 그
// 안에 본문이 다 안 들어가면 그냥 문장 중간에서 뚝 끊긴 것처럼 안 보이게
// 마지막 줄 끝에 "..." 를 붙인다. wrap_lines 로 접은 줄이 PREVIEW_LINES 를
// 넘으면, 마지막 줄부터 그 뒤에 남은 내용을 전부 다시 이어붙여서 그 문자열을
// truncate_ellipsis 로 한 번 더 잘라 넣는다 — 그래야 정확히 그 줄의 실제
// 렌더링 폭 기준으로 "..." 가 붙는다.
fn preview_lines(r: &Renderer, body: &str, max_w: f32) -> Vec<String> {
    let mut lines = wrap_lines(r, body, PREVIEW_SCALE, max_w);
    if lines.len() > PREVIEW_LINES {
        let last = PREVIEW_LINES - 1;
        let overflow = lines[last..].join(" ");
        lines.truncate(PREVIEW_LINES);
        lines[last] = truncate_ellipsis(r, &overflow, PREVIEW_SCALE, max_w);
    }
    lines
}

// "Write Mail" 필드 텍스트 편집 — cursor(글자 인덱스)가 가리키는 자리에 끼워
// 넣거나 그 앞 글자를 지운다. 문자열은 바이트 인덱스라 char_indices 로 몇 번째
// 글자가 몇 번째 바이트에서 시작하는지 먼저 찾는다(한/일 문자는 여러 바이트라
// 그냥 cursor 를 바이트 인덱스로 쓰면 글자 중간을 끊어버릴 수 있다).
fn insert_char_at(s: &mut String, cursor: &mut usize, c: char, max_chars: usize) {
    if s.chars().count() >= max_chars {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor).map(|(b, _)| b).unwrap_or(s.len());
    s.insert(byte_idx, c);
    *cursor += 1;
}

fn backspace_at(s: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let byte_idx = s.char_indices().nth(*cursor - 1).map(|(b, _)| b).unwrap_or(0);
    s.remove(byte_idx);
    *cursor -= 1;
}

// Backspace 를 꾹 누르고 있을 때 지금까지 몇 번 지워졌어야 하는지 — 누른 순간
// 한 번(즉시), REPEAT_DELAY 초까지 그대로 있으면 그 뒤로 REPEAT_INTERVAL 마다
// 하나씩 더. hold 는 지금 이 키가 계속 눌려있던 시간(초).
const BACKSPACE_REPEAT_DELAY: f32 = 0.3;
const BACKSPACE_REPEAT_INTERVAL: f32 = 0.035;
// 한 프레임에 hold 누적치를 이보다 많이는 안 늘린다 — main.rs 의 전역 dt
// 상한(500ms)만으론 이 반복 판정 기준으로 여전히 너무 커서, 큰 프레임 끊김이
// 한 번 있으면 그 한 프레임에 반복 삭제가 몰아서 발생할 수 있었다.
const BACKSPACE_MAX_DT_PER_FRAME: f32 = 0.02;
fn backspace_repeats_due(hold: f32) -> u32 {
    if hold <= 0.0 {
        return 0;
    }
    if hold < BACKSPACE_REPEAT_DELAY {
        return 1;
    }
    2 + ((hold - BACKSPACE_REPEAT_DELAY) / BACKSPACE_REPEAT_INTERVAL) as u32
}

// 내용이 없는 상태(폴더 미선택/빈 폴더/받은편지함 비어있음) — 예전엔 왼쪽 위 구석에
// 작은 글자 한 줄만 덩그러니 있어서, pane 이 넓으면(창을 키우거나 아예 기본 크기만
// 돼도) 그 큰 빈 공간이 유난히 휑해 보였다. 큰 아이콘 하나를 pane 가운데 즈음에
// 놓고 그 아래 안내문을 붙여서 "여기가 원래 이런 화면이다" 라는 느낌이 나게 한다
// (탐색기류가 빈 폴더 열었을 때 흔히 쓰는 요령).
fn draw_empty_state(r: &mut Renderer, assets: &Assets, pane: Rect, icon: IconType, text: &str) {
    const ICON_S: f32 = 48.0;
    let icon_x = pane.x + (pane.w - ICON_S) / 2.0;
    let icon_y = pane.y + (pane.h * 0.4 - ICON_S / 2.0).max(10.0);
    draw_icon(r, assets, &icon, icon_x, icon_y, ICON_S);
    let scale = 0.9;
    let max_w = (pane.w - 40.0).max(10.0);
    let tw = r.text_width(text, scale).min(max_w);
    let tx = pane.x + (pane.w - tw) / 2.0;
    let ty = icon_y + ICON_S + 14.0;
    r.text_clipped(tx, ty, text, scale, GRAY, max_w);
}

impl App for MailApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(crate::foundation::display_name(lang, "Mail").into_owned())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let smooth = self.settings.borrow().smooth_scroll;
        let lang = self.settings.borrow().language;
        // Inbox 시드 메일의 제목/본문은 언어별 문구라, 창을 새로 안 열어도(설정에서
        // 바로) 언어를 바꾸면 즉시 반영되게 여기서 다시 만든다 — 나머지 라벨들이
        // 이미 다 이런 식으로 즉시 반응하는 것과 맞춘다. read/downloaded/downloading
        // 은 메시지 개수 자체가 안 바뀌므로 그대로 유지해도 인덱스가 안 어긋난다.
        if lang != self.built_lang {
            self.messages = seed_messages(self.arrived, lang, self.hextool_id);
            self.built_lang = lang;
            self.body_wrapped_key = (usize::MAX, i32::MIN);
        }

        self.draw_menu_bar(r, area, lang);

        let body_top = area.y + MENU_H;
        let body_h = (area.h - MENU_H - STATUS_H).max(0.0);
        self.tree_w = self.tree_w.clamp(FOLDER_TREE_W_MIN, area.w * FOLDER_TREE_W_MAX_FRAC);
        let tree_area = Rect::new(area.x, body_top, self.tree_w, body_h);
        self.draw_folder_tree(r, assets, tree_area, win, lang);
        r.rect(area.x + self.tree_w - 1.0, body_top, 1.0, body_h, [0.6, 0.6, 0.62, 1.0]);

        // 트리↔내용 구분선 드래그 — explorer.rs 의 사이드바 폭 조절과 같은 요령.
        let divider = Rect::new(area.x + self.tree_w - DIVIDER_W / 2.0, body_top, DIVIDER_W, body_h);
        if win.mouse_clicked && divider.contains(win.mouse.0, win.mouse.1) {
            self.divider_drag = true;
        }
        if self.divider_drag && win.mouse_down {
            self.tree_w = (win.mouse.0 - area.x).clamp(FOLDER_TREE_W_MIN, area.w * FOLDER_TREE_W_MAX_FRAC);
        }
        if !win.mouse_down {
            self.divider_drag = false;
        }

        let pane = Rect::new(area.x + self.tree_w, body_top, area.w - self.tree_w, body_h);
        let action = self.draw_pane_content(r, assets, pane, win, smooth, lang);
        // My Computer/휴지통처럼 트리+내용 전체를 옅은 회색 테두리로 한 번 감싸서
        // 하나의 패널처럼 보이게 한다(참고 이미지의 전체 베벨 테두리 느낌).
        border(r, area.x, body_top, area.w, body_h, [0.6, 0.6, 0.62, 1.0]);
        self.draw_status_bar(r, Rect::new(area.x, area.y + area.h - STATUS_H, area.w, STATUS_H), lang);
        action
    }
}

impl MailApp {
    // pane 폭 안에 폴더 내용을 그린다 — update() 에서 분리한 이유는 이 함수 안에서
    // 여러 곳에 `return` 을 자유롭게 쓰기 위해서다(update() 자체에 흩어두면 그 뒤에
    // 그려야 하는 상태바를 매번 같이 호출해줘야 해서 번거롭다).
    fn draw_pane_content(&mut self, r: &mut Renderer, assets: &Assets, pane: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        // 참고 이미지처럼 오른쪽 내용 패널은 항상 흰 배경 — 이전엔 이 밑칠이 없어서
        // 빈 상태 화면들이 회색(FACE) 바탕 위에 떠 있었다.
        r.rect(pane.x, pane.y, pane.w, pane.h, WHITE);
        let Some(folder) = self.folder else {
            let msg = t(lang, s::SELECT_FOLDER_MSG);
            draw_empty_state(r, assets, pane, IconType::Mail, msg);
            return AppAction::None;
        };
        if folder == MailFolder::Compose {
            return self.draw_new_compose(r, assets, pane, win, smooth, lang);
        }
        if folder == MailFolder::Sent {
            return self.draw_sent_pane(r, assets, pane, win, smooth, lang);
        }

        // ---------------- Inbox 선택됨 ----------------
        if self.messages.is_empty() {
            // 아직 첫 메일이 도착하기 전 — 받은편지함이 빈 상태.
            draw_empty_state(r, assets, pane, IconType::Envelope, t(lang, s::NO_NEW_MESSAGES));
            return AppAction::None;
        }

        // 메시지를 아직 안 골랐으면 목록이 pane 전체 폭을 쓰고, 하나를 고르면 그
        // 내용이 pane 전체 폭을 쓴다 — 목록/읽기가 서로 다른 화면이라 각자 넓게 쓴다.
        let Some(msg_idx) = self.selected else {
            // 헤더 — 고전 메일 클라이언트의 From/Subject 컬럼 헤더를 흉내낸다.
            // My Computer(explorer.rs) 의 Name/Size 헤더처럼 raised 바 두 칸으로
            // 튀어나온 느낌을 살렸다(전에는 이 느낌이 과하다는 피드백에 flat 바로
            // 바꿨었는데, 이번엔 반대로 My Computer 처럼 튀어나오게 해달라는 요청).
            // "받은 날짜는 없어도 되니까 글자 짤리는 걸 중점적으로 고쳐줘" 요청으로
            // Received 칸을 뺐다 — 그 칸이 차지하던 폭을 From/Subject 가 그대로
            // 돌려받아서 더 안 잘리게 된다.
            let from_col_w = (pane.w * FROM_COL_FRAC).clamp(FROM_COL_MIN, FROM_COL_MAX).min(pane.w * 0.6);
            let subject_col_x = pane.x + from_col_w;
            let subject_col_w = (pane.w - from_col_w).max(0.0);
            raised(r, pane.x, pane.y, from_col_w, LIST_HEADER_H);
            raised(r, subject_col_x, pane.y, subject_col_w, LIST_HEADER_H);
            let header_ty = pane.y + (LIST_HEADER_H - CELL_H * 0.85) / 2.0;
            r.text(pane.x + 6.0, header_ty, t(lang, s::COL_FROM), 0.85, BLACK);
            r.text(subject_col_x + 6.0, header_ty, t(lang, s::COL_SUBJECT), 0.85, BLACK);

            // 행 안쪽에서 글자가 시작/끝나는 좌우 여백.
            const TEXT_INSET: f32 = 6.0;
            let list_area = Rect::new(pane.x, pane.y + LIST_HEADER_H, pane.w, pane.h - LIST_HEADER_H);
            let line_h = CELL_H * TEXT_SCALE + 2.0;
            let preview_line_h = CELL_H * PREVIEW_SCALE + 2.0;
            let row_w = pane.w - ROW_PAD * 2.0;
            let from_max_w = from_col_w - TEXT_INSET * 2.0;
            let subject_max_w = subject_col_w - TEXT_INSET * 2.0;
            // 헤더 한 줄(From/Subject) 아래에 본문 미리보기 최대 두 줄 — 참고
            // 이미지의 "AutoPreview" 느낌. 미리보기는 Subject 칸과 같은 x 에서
            // 시작해서 행 끝까지 넓게 쓴다(From 칸 밑으로는 안 내려온다).
            let preview_max_w = (row_w - from_col_w - TEXT_INSET).max(10.0);
            let row_h = line_h + preview_line_h * PREVIEW_LINES as f32 + ROW_PAD * 2.0;
            let total_h = (row_h + ROW_GAP) * self.messages.len() as f32;
            let max_list_scroll = (total_h - list_area.h).max(0.0);
            if list_area.contains(win.mouse.0, win.mouse.1) {
                self.list_scroll -= win.wheel / 120.0 * (line_h * 2.0);
            }
            self.list_scroll = self.list_scroll.clamp(0.0, max_list_scroll);
            ease_scroll(&mut self.list_scroll_disp, self.list_scroll, win.dt, smooth);

            r.set_clip(Some(list_area));
            let mut clicked_row = None;
            let mut ry = list_area.y - self.list_scroll_disp;
            for (i, msg) in self.messages.iter().enumerate() {
                if ry + row_h >= list_area.y && ry <= list_area.y + list_area.h {
                    let row = Rect::new(pane.x + ROW_PAD, ry, row_w, row_h - ROW_GAP);
                    let hover = row.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
                    // 이 목록은 selected == None 일 때만 그려지므로(그 아래 `let Some
                    // (msg_idx) = self.selected else` 분기) 여기 자체엔 "선택된" 행이
                    // 있을 수 없다 — hover 만 표시한다. 예전엔 행마다 sunken() 으로
                    // 회색 박스를 그렸는데, 목록 전체를 흰색으로 해달라는 요청으로
                    // 뺐다 — pane 배경이 이미 흰색이라 행 자체엔 따로 안 칠해도 된다.
                    if hover {
                        r.rect(row.x + 2.0, row.y + 2.0, row.w - 4.0, row.h - 4.0, [0.0, 0.0, 0.4, 0.08]);
                    }
                    // 안 읽은 메일은 굵게 — 참고 이미지의 "Welcome!" 행처럼. 한 줄로
                    // 강제 — 다 안 들어가면 잘라내고 "..." 를 붙인다.
                    let unread = !self.read.get(i).copied().unwrap_or(true);
                    let from = truncate_ellipsis(r, msg.from, TEXT_SCALE, from_max_w);
                    let subject = truncate_ellipsis(r, msg.subject, TEXT_SCALE, subject_max_w);
                    let ty = row.y + ROW_PAD;
                    if unread {
                        draw_bold_text(r, row.x + TEXT_INSET, ty, &from, TEXT_SCALE, BLACK);
                        draw_bold_text(r, subject_col_x + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    } else {
                        r.text(row.x + TEXT_INSET, ty, &from, TEXT_SCALE, BLACK);
                        r.text(subject_col_x + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    }
                    r.rect(row.x + from_col_w, row.y, 1.0, row.h, [0.85, 0.85, 0.85, 1.0]); // 컬럼 구분선

                    // 본문 미리보기 — 회색, 굵지 않게(안 읽음이어도). 두 줄 안에 다
                    // 못 들어가면 마지막 줄 끝에 "..." 를 붙여서, 문장 중간에서
                    // 그냥 뚝 잘린 것처럼 안 보이게 한다.
                    let preview = preview_lines(r, msg.body, preview_max_w);
                    let mut pty = ty + line_h;
                    for pline in &preview {
                        r.text(subject_col_x + TEXT_INSET, pty, pline, PREVIEW_SCALE, GRAY);
                        pty += preview_line_h;
                    }

                    if win.mouse_clicked && hover {
                        clicked_row = Some(i);
                    }
                }
                ry += row_h + ROW_GAP;
            }
            r.set_clip(None);
            if let Some(i) = clicked_row {
                self.select(i);
            }
            if max_list_scroll > 0.0 {
                let sb_x = pane.x + pane.w - 8.0;
                let frac = (list_area.h / total_h).min(1.0);
                scrollbar(r, win, sb_x, list_area.y, 8.0, list_area.h, frac, self.list_scroll_disp, &mut self.list_scroll, max_list_scroll, &mut self.list_sb_drag);
            }
            // 예전엔 여기서 헤더+목록만 따로 한 번 더 회색 테두리로 감쌌는데, 이제
            // update() 가 트리+내용 전체를 이미 한 번 감싸므로 이중 테두리가 되어
            // 뺐다.
            // 방금 고른 메시지가 있으면 desktop.rs 에게 fs.mail_read 에 기록해달라고
            // 요청한다 — 화면(self.read)은 select() 가 이미 바로 갱신했지만, 그건
            // 새로고침 때마다 사라지는 인스턴스 상태라 진짜 기록은 fs 쪽에 남겨야 한다.
            return match clicked_row {
                Some(i) => AppAction::MarkMailRead(i),
                None => AppAction::None,
            };
        };

        // ---------------- 메시지 하나를 골랐음: pane 전체 폭을 다 쓰는 읽기 화면.
        // 맨 위에 목록으로 돌아가는 링크를 붙인다(폴더 트리에서 Inbox 를 다시 눌러도
        // 똑같이 목록으로 돌아가지만, 이쪽이 더 눈에 잘 띈다). ----------------
        let back_row = Rect::new(pane.x, pane.y, pane.w, BACK_ROW_H);
        let back_hover = back_row.contains(win.mouse.0, win.mouse.1);
        let back_color = if back_hover { NAVY } else { BLACK };
        let back_label = t(lang, s::BACK_TO_INBOX);
        r.text(back_row.x + 6.0, back_row.y + (BACK_ROW_H - CELL_H) / 2.0, back_label, 1.0, back_color);
        if back_hover {
            let tw = r.text_width(back_label, 1.0);
            r.rect(back_row.x + 6.0, back_row.y + BACK_ROW_H - 4.0, tw, 1.0, back_color);
        }
        if win.mouse_clicked && back_hover {
            self.selected = None;
        }

        r.rect(pane.x, pane.y + BACK_ROW_H - 1.0, pane.w, 1.0, GRAY);
        let content = Rect::new(pane.x, pane.y + BACK_ROW_H, pane.w, pane.h - BACK_ROW_H);

        let has_attachment = self.messages[msg_idx].attachment.is_some();
        let cx = content.x + 8.0;
        let field_w = content.w - 16.0;

        // From/To/Cc/Subject 라벨 필드 블록 — 한 줄씩, 값 전체 폭을 다 쓴다("받은
        // 날짜는 없어도 되니까 글자 짤리는 걸 고쳐줘" 요청으로 Sent 필드를 빼고
        // From 을 다시 한 줄 전체 폭으로 되돌렸다 — 반으로 나눠 쓰던 때는
        // "PalaceCompany@email.com" 같은 긴 주소가 훨씬 더 심하게 잘렸다). 맨
        // 끝에 얇은 구분선을 하나 그어서 본문 상자와 분리한다.
        const FIELD_ROW_H: f32 = 20.0;
        let mut fy = content.y + 6.0;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_FROM), self.messages[msg_idx].from);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_TO), self.messages[msg_idx].to);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_CC), self.messages[msg_idx].cc);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_SUBJECT), self.messages[msg_idx].subject);
        fy += FIELD_ROW_H + 6.0;
        r.rect(content.x + 4.0, fy, content.w - 8.0, 1.0, GRAY);
        fy += 6.0;

        // 첨부파일이 있어도 이제 따로 자리를 안 빼고(예전엔 attach_h 만큼 밑을
        // 잘라 항상 하단에 고정했었다) 본문 상자 전체를 텍스트 스크롤 영역으로
        // 쓴다 — 첨부 박스는 본문 마지막 줄 바로 아래에 이어지는 "콘텐츠의 일부"로
        // 취급해서 텍스트와 같이 스크롤된다.
        let body_area = Rect::new(content.x, fy, content.w, content.y + content.h - fy);
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        const ATTACH_BOX_H: f32 = 38.0;
        const ATTACH_GAP: f32 = 10.0; // 본문 마지막 줄과 첨부 박스 사이 여백
        // draw_field() 와 같은 이유로 sunken() 대신 흰 배경 + 얇은 회색 테두리.
        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, [0.6, 0.6, 0.62, 1.0]);
        let text_w = body_area.w - 20.0 - SB_W;
        // 캐시 무효화 키에 폭을 그대로(f32) 쓰지 않고 정수 픽셀로 반올림해 담는다 —
        // 실수를 직접 비교하면 부동소수점 오차로 같은 폭인데도 매 프레임 다시
        // 계산해버릴 위험이 있다.
        let key = (msg_idx, text_w.round() as i32);
        if key != self.body_wrapped_key {
            self.body_wrapped = wrap_lines(r, self.messages[msg_idx].body, 1.0, text_w);
            self.body_wrapped_key = key;
        }
        let lines = &self.body_wrapped;
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 2.0, body_area.w - 12.0, body_area.h - 4.0);
        let visible = (text_area.h / LINE_H).floor() as usize;
        // 첨부 박스 몫(있으면 여백+박스 높이)을 "줄 수"로 환산해서 스크롤 가능
        // 범위에 더한다 — 그래야 마지막 줄까지 다 내려도 첨부 박스가 화면에
        // 다 들어올 때까지 더 스크롤된다.
        let attach_extra_lines = if has_attachment { (ATTACH_BOX_H + ATTACH_GAP) / LINE_H } else { 0.0 };
        let total_lines = lines.len() as f32 + attach_extra_lines;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);
        // 마우스가 본문 박스 위에 있을 때만 휠을 먹는다(목록 쪽과 대칭되는 이유).
        if body_area.contains(win.mouse.0, win.mouse.1) {
            self.body_scroll -= win.wheel / 120.0 * 3.0;
        }
        self.body_scroll = self.body_scroll.clamp(0.0, max_body_scroll);
        ease_scroll(&mut self.body_scroll_disp, self.body_scroll, win.dt, smooth);

        let line_off = self.body_scroll_disp * LINE_H;
        let first = self.body_scroll_disp as usize;
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - (line_off - first as f32 * LINE_H);
        for line in lines.iter().skip(first) {
            if ty > text_area.y + text_area.h {
                break;
            }
            r.text(text_area.x, ty, line, 1.0, BLACK);
            ty += LINE_H;
        }

        // 첨부파일 — 본문 텍스트와 같은 좌표계(스크롤 오프셋 line_off) 위에서
        // 마지막 줄 바로 다음 "가상의 줄" 자리에 그린다. 다운로드 진행 타이머는
        // 화면 밖으로 스크롤돼도 멈추면 안 되니(눌러놓고 스크롤해서 딴 걸 보는
        // 동안에도 계속 받아지고 있어야 자연스럽다) 그리기 여부와 상관없이 항상
        // 갱신하고, 실제 UI(아이콘/버튼)만 보이는 범위일 때만 그린다.
        let mut finished_download = None;
        if let Some((id, name)) = self.messages[msg_idx].attachment.clone() {
            if let Some(elapsed) = self.downloading[msg_idx] {
                let elapsed = elapsed + win.dt;
                if elapsed >= DOWNLOAD_DELAY {
                    self.downloading[msg_idx] = None;
                    self.downloaded[msg_idx] = true;
                    finished_download = Some(id);
                } else {
                    self.downloading[msg_idx] = Some(elapsed);
                }
            }

            let box_y = text_area.y + lines.len() as f32 * LINE_H + ATTACH_GAP - line_off;
            // 스크롤바(SB_W)가 뜨는 오른쪽 끝 자리는 항상 비워둔다 — 안 그러면
            // 스크롤이 필요한 긴 메일에서 첨부 박스의 다운로드 버튼이 스크롤바와
            // 겹쳐 보인다.
            let box_w = content.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, &IconType::Lock, cx + 5.0, box_y + 3.0, 26.0);

                let dl_label = t(lang, s::DOWNLOAD);
                let dl_ing_label = t(lang, s::DOWNLOADING);
                let dl_done_label = t(lang, s::DOWNLOADED);
                // 세 상태(Download/Downloading/Downloaded)가 언어마다 폭이 다른데
                // (한국어/일본어가 영어보다 김) 예전엔 90px 로 고정해뒀다 — 한국어
                // "다운로드"/"다운로드 중" 같은 문구가 버튼 밖으로 삐져나와 스크롤바
                // 쪽과 겹쳐 보이는 문제로 이어졌다(제보받음). 세 상태 각각 실제로
                // 필요한 폭을 계산해 가장 넓은 쪽에 맞춘다 — 그러면 상태가 바뀔 때
                // 버튼이 다시 커지거나 하지 않고, 어떤 언어에서도 항상 다 들어간다.
                let btn_w = (r.text_width(dl_label, 1.0) + 16.0)
                    .max(22.0 + r.text_width(dl_ing_label, 0.75) + 6.0)
                    .max(6.0 + r.text_width(dl_done_label, 0.8) + 6.0)
                    .max(70.0);
                let btn_h = 22.0;
                let btn_x = cx + box_w - btn_w - 6.0;
                let btn_y = box_y + (box_h - btn_h) / 2.0;
                let name_max_w = (btn_x - (cx + 38.0) - 6.0).max(10.0); // 버튼과 안 겹치게 그 앞까지만
                r.text_clipped(cx + 38.0, box_y + 11.0, &name, 0.85, BLACK, name_max_w);
                // 다운로드 버튼을 누르면 바로 완료 처리하지 않고 DOWNLOAD_DELAY 초 동안
                // 스피너 + "Downloading..." 를 보여주는 연출을 먼저 거친다 — 그래야 클릭
                // 즉시 파일이 나타나는 대신 뭔가 "받아오는 중"이라는 느낌이 든다.
                if self.downloaded[msg_idx] {
                    r.text(btn_x + 6.0, btn_y + 3.0, dl_done_label, 0.8, GRAY);
                } else if self.downloading[msg_idx].is_some() {
                    sunken(r, btn_x, btn_y, btn_w, btn_h);
                    draw_spinner(r, btn_x + 12.0, btn_y + btn_h / 2.0, 6.0, win.time);
                    r.text_clipped(btn_x + 22.0, btn_y + 3.0, dl_ing_label, 0.75, GRAY, btn_w - 26.0);
                } else if button(r, btn_x, btn_y, btn_w, btn_h, dl_label, win) {
                    self.downloading[msg_idx] = Some(0.0);
                }
            }
        }
        r.set_clip(None);
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 8.0;
            let frac = visible as f32 / total_lines;
            scrollbar(r, win, sb_x, body_area.y + 2.0, SB_W, body_area.h - 4.0, frac, self.body_scroll_disp, &mut self.body_scroll, max_body_scroll, &mut self.body_sb_drag);
        }

        if let Some(id) = finished_download {
            return AppAction::Download(id);
        }

        AppAction::None
    }

    // 보낸편지함(Sent Items) — Inbox 의 목록/읽기 구조를 그대로 본떴지만 훨씬
    // 단순하다: 안 읽음 배지/굵은 글씨가 없고(자기가 보낸 걸 "안 읽음" 취급할
    // 이유가 없다), 첨부는 다운로드 버튼 없이 그냥 정보로만 보여준다(이미 내가
    // 가지고 있던 파일이라 "받을" 게 없다). 목록 헤더도 From 대신 To.
    #[allow(clippy::too_many_arguments)]
    fn draw_sent_pane(&mut self, r: &mut Renderer, assets: &Assets, pane: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        if self.sent.is_empty() {
            draw_empty_state(r, assets, pane, IconType::Envelope, t(lang, s::NO_SENT_MESSAGES));
            return AppAction::None;
        }

        let Some(msg_idx) = self.selected else {
            let from_col_w = (pane.w * FROM_COL_FRAC).clamp(FROM_COL_MIN, FROM_COL_MAX).min(pane.w * 0.6);
            raised(r, pane.x, pane.y, from_col_w, LIST_HEADER_H);
            raised(r, pane.x + from_col_w, pane.y, pane.w - from_col_w, LIST_HEADER_H);
            let header_ty = pane.y + (LIST_HEADER_H - CELL_H * 0.85) / 2.0;
            r.text(pane.x + 6.0, header_ty, t(lang, s::COL_TO), 0.85, BLACK);
            r.text(pane.x + from_col_w + 6.0, header_ty, t(lang, s::COL_SUBJECT), 0.85, BLACK);

            const TEXT_INSET: f32 = 6.0;
            let list_area = Rect::new(pane.x, pane.y + LIST_HEADER_H, pane.w, pane.h - LIST_HEADER_H);
            let line_h = CELL_H * TEXT_SCALE + 2.0;
            let preview_line_h = CELL_H * PREVIEW_SCALE + 2.0;
            let row_w = pane.w - ROW_PAD * 2.0;
            let from_max_w = from_col_w - TEXT_INSET * 2.0;
            let subject_max_w = row_w - from_col_w - TEXT_INSET * 2.0;
            let preview_max_w = (row_w - from_col_w - TEXT_INSET).max(10.0);
            let row_h = line_h + preview_line_h * PREVIEW_LINES as f32 + ROW_PAD * 2.0;
            let total_h = (row_h + ROW_GAP) * self.sent.len() as f32;
            let max_list_scroll = (total_h - list_area.h).max(0.0);
            if list_area.contains(win.mouse.0, win.mouse.1) {
                self.list_scroll -= win.wheel / 120.0 * (line_h * 2.0);
            }
            self.list_scroll = self.list_scroll.clamp(0.0, max_list_scroll);
            ease_scroll(&mut self.list_scroll_disp, self.list_scroll, win.dt, smooth);

            r.set_clip(Some(list_area));
            let mut clicked_row = None;
            let mut ry = list_area.y - self.list_scroll_disp;
            for (i, msg) in self.sent.iter().enumerate() {
                if ry + row_h >= list_area.y && ry <= list_area.y + list_area.h {
                    let row = Rect::new(pane.x + ROW_PAD, ry, row_w, row_h - ROW_GAP);
                    let hover = row.intersect(&list_area).contains(win.mouse.0, win.mouse.1);
                    if hover {
                        r.rect(row.x + 2.0, row.y + 2.0, row.w - 4.0, row.h - 4.0, [0.0, 0.0, 0.4, 0.08]);
                    }
                    let to = truncate_ellipsis(r, &msg.to, TEXT_SCALE, from_max_w);
                    let subject = truncate_ellipsis(r, &msg.subject, TEXT_SCALE, subject_max_w);
                    let ty = row.y + ROW_PAD;
                    r.text(row.x + TEXT_INSET, ty, &to, TEXT_SCALE, BLACK);
                    r.text(row.x + from_col_w + TEXT_INSET, ty, &subject, TEXT_SCALE, BLACK);
                    r.rect(row.x + from_col_w, row.y, 1.0, row.h, [0.85, 0.85, 0.85, 1.0]);

                    let preview = preview_lines(r, &msg.body, preview_max_w);
                    let mut pty = ty + line_h;
                    for pline in &preview {
                        r.text(row.x + from_col_w + TEXT_INSET, pty, pline, PREVIEW_SCALE, GRAY);
                        pty += preview_line_h;
                    }

                    if win.mouse_clicked && hover {
                        clicked_row = Some(i);
                    }
                }
                ry += row_h + ROW_GAP;
            }
            r.set_clip(None);
            if let Some(i) = clicked_row {
                self.selected = Some(i);
                self.body_scroll = 0.0;
                self.body_scroll_disp = 0.0;
            }
            if max_list_scroll > 0.0 {
                let sb_x = pane.x + pane.w - 8.0;
                let frac = (list_area.h / total_h).min(1.0);
                scrollbar(r, win, sb_x, list_area.y, 8.0, list_area.h, frac, self.list_scroll_disp, &mut self.list_scroll, max_list_scroll, &mut self.list_sb_drag);
            }
            return AppAction::None;
        };

        // ---------------- 보낸 메일 하나를 골랐음: 읽기 화면 ----------------
        let back_row = Rect::new(pane.x, pane.y, pane.w, BACK_ROW_H);
        let back_hover = back_row.contains(win.mouse.0, win.mouse.1);
        let back_color = if back_hover { NAVY } else { BLACK };
        let back_label = t(lang, s::BACK_TO_SENT);
        r.text(back_row.x + 6.0, back_row.y + (BACK_ROW_H - CELL_H) / 2.0, back_label, 1.0, back_color);
        if back_hover {
            let tw = r.text_width(back_label, 1.0);
            r.rect(back_row.x + 6.0, back_row.y + BACK_ROW_H - 4.0, tw, 1.0, back_color);
        }
        if win.mouse_clicked && back_hover {
            self.selected = None;
        }

        r.rect(pane.x, pane.y + BACK_ROW_H - 1.0, pane.w, 1.0, GRAY);
        let content = Rect::new(pane.x, pane.y + BACK_ROW_H, pane.w, pane.h - BACK_ROW_H);

        let attach_count = self.sent[msg_idx].attachments.len();
        let cx = content.x + 8.0;
        let field_w = content.w - 16.0;

        const FIELD_ROW_H: f32 = 20.0;
        let mut fy = content.y + 6.0;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_TO), &self.sent[msg_idx].to);
        fy += FIELD_ROW_H;
        self.draw_field(r, cx, fy, field_w, t(lang, s::FIELD_SUBJECT), &self.sent[msg_idx].subject);
        fy += FIELD_ROW_H + 6.0;
        r.rect(content.x + 4.0, fy, content.w - 8.0, 1.0, GRAY);
        fy += 6.0;

        // Inbox 읽기 화면과 같은 요령 — 첨부 박스도 따로 자리를 안 빼고 본문
        // 텍스트 마지막 줄 바로 다음 "가상의 줄" 로 취급해서 같이 스크롤된다.
        let body_area = Rect::new(content.x, fy, content.w, content.y + content.h - fy);
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        const ATTACH_BOX_H: f32 = 38.0;
        const ATTACH_GAP: f32 = 10.0;
        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, [0.6, 0.6, 0.62, 1.0]);
        let text_w = body_area.w - 20.0 - SB_W;
        let key = (msg_idx, text_w.round() as i32);
        if key != self.sent_wrapped_key {
            self.sent_wrapped = wrap_lines(r, &self.sent[msg_idx].body, 1.0, text_w);
            self.sent_wrapped_key = key;
        }
        let lines = &self.sent_wrapped;
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 2.0, body_area.w - 12.0, body_area.h - 4.0);
        let visible = (text_area.h / LINE_H).floor() as usize;
        let attach_unit_lines = (ATTACH_BOX_H + ATTACH_GAP) / LINE_H;
        let total_lines = lines.len() as f32 + attach_unit_lines * attach_count as f32;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);
        if body_area.contains(win.mouse.0, win.mouse.1) {
            self.body_scroll -= win.wheel / 120.0 * 3.0;
        }
        self.body_scroll = self.body_scroll.clamp(0.0, max_body_scroll);
        ease_scroll(&mut self.body_scroll_disp, self.body_scroll, win.dt, smooth);

        let line_off = self.body_scroll_disp * LINE_H;
        let first = self.body_scroll_disp as usize;
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - (line_off - first as f32 * LINE_H);
        for line in lines.iter().skip(first) {
            if ty > text_area.y + text_area.h {
                break;
            }
            r.text(text_area.x, ty, line, 1.0, BLACK);
            ty += LINE_H;
        }

        // 첨부파일들 — 다운로드 버튼 없이 그냥 아이콘+이름만(이미 내가 가진
        // 파일이라 "받을" 개념이 없다). 본문 마지막 줄 다음부터 하나씩 이어
        // 붙여서 같이 스크롤된다.
        for (i, (_, name, icon)) in self.sent[msg_idx].attachments.iter().enumerate() {
            let box_y = text_area.y + (lines.len() as f32 + attach_unit_lines * i as f32) * LINE_H + ATTACH_GAP - line_off;
            // 스크롤바(SB_W)가 뜨는 오른쪽 끝 자리는 항상 비워둔다 — 안 그러면
            // 스크롤이 필요한 긴 메일에서 첨부 박스가 스크롤바와 겹쳐 보인다.
            let box_w = content.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, icon, cx + 5.0, box_y + 3.0, 26.0);
                r.text_clipped(cx + 38.0, box_y + 11.0, name, 0.85, BLACK, box_w - 44.0);
            }
        }
        r.set_clip(None);
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 8.0;
            let frac = visible as f32 / total_lines;
            scrollbar(r, win, sb_x, body_area.y + 2.0, SB_W, body_area.h - 4.0, frac, self.body_scroll_disp, &mut self.body_scroll, max_body_scroll, &mut self.body_sb_drag);
        }

        AppAction::None
    }

    // draw_field() 의 입력 가능한 버전 — 클릭하면 그 필드가 활성화되고(테두리가
    // 남색으로 바뀌고 깜빡이는 커서가 붙는다). 실제 타이핑 처리는 여기서 안 하고
    // (한 프레임에 여러 필드가 동시에 안 그려지므로, 타이핑은 호출부가 active_field
    // 기준으로 한 번에 처리하는 게 더 간단하다) 그리기 + 클릭 감지만 담당한다.
    // 커서는 항상 값의 맨 끝이 아니라 cursor(글자 인덱스)가 가리키는 자리에
    // 그린다 — 클릭했으면(Some(idx) 반환) 그 클릭 위치에 가장 가까운 글자
    // 경계를 돌려줘서, 호출부가 active_field 와 커서 위치를 같이 옮길 수 있다.
    #[allow(clippy::too_many_arguments)]
    fn draw_editable_field(
        &self, r: &mut Renderer, x: f32, y: f32, w: f32, label: &str, value: &str, active: bool, cursor: usize, win: &WinInput,
    ) -> Option<usize> {
        const LABEL_W: f32 = 76.0;
        const FIELD_H: f32 = 22.0;
        r.text(x, y + (FIELD_H - CELL_H * 0.8) / 2.0, label, 0.8, BLACK);
        let field = Rect::new(x + LABEL_W, y, w - LABEL_W, FIELD_H);
        r.rect(field.x, field.y, field.w, field.h, WHITE);
        border(r, field.x, field.y, field.w, field.h, if active { NAVY } else { [0.6, 0.6, 0.62, 1.0] });
        let ty = field.y + (field.h - CELL_H * 0.8) / 2.0;
        let max_w = field.w - 12.0;
        let byte_idx = value.char_indices().nth(cursor).map(|(b, _)| b).unwrap_or(value.len());
        // 조합 중인(아직 확정 안 된) 문자열을 캐럿 위치에 끼워넣어서만 화면에
        // 보여준다 — self.new_mail.* 자체(저장/전송에 실제 쓰이는 값)는 여전히
        // char_event 로 확정된 문자만으로 채워지고 이 값의 영향을 전혀 안 받는다.
        let preview = if active && win.focused { crate::ime::composition_preview() } else { None };
        let display = match &preview {
            Some(p) if !p.is_empty() => format!("{}{}{}", &value[..byte_idx], p, &value[byte_idx..]),
            _ => value.to_string(),
        };
        r.text_clipped(field.x + 6.0, ty, &display, 0.8, BLACK, max_w);
        if active {
            let tw = r.text_width(&value[..byte_idx], 0.8).min(max_w);
            let cursor_vx = field.x + 6.0 + tw;
            if win.focused {
                // Windows 자체의 IME 조합창/후보창은 화면 어디에 띄우든 우리가 그린
                // 픽셀아트 CRT 화면이랑 스타일이 안 맞아서(둥근 모서리/그림자 있는
                // 현대식 팝업) 캐럿 옆으로 옮겨놔도 어색한 상자로 튀어 보였다 —
                // 그래서 아예 화면 밖으로 치워서 안 보이게 한다. 조합/확정 자체는
                // 이 팝업 위치와 무관하게 그대로 잘 되고, 조합 중인 내용은 위에서
                // 이미 우리가 직접 그려서 보여준다.
                crate::ime::set_composition_pos(OFFSCREEN, OFFSCREEN);
                // 그래도 남는 네 번째 팝업(최신 IME 의 CiceroUIWndFrame 언어
                // 표시줄)은 저 API 들로 못 옮기므로 통째로 숨긴다.
                crate::ime::hide_cicero_windows();
            }
            if (win.time % 1.0) < 0.5 {
                r.rect(cursor_vx + 1.0, ty, 2.0, CELL_H * 0.8, BLACK);
            }
        }
        if win.mouse_clicked && field.contains(win.mouse.0, win.mouse.1) {
            Some(char_index_at_x(r, value, 0.8, win.mouse.0 - (field.x + 6.0)))
        } else {
            None
        }
    }

    // "Write Mail" 탭 — 받는 사람/제목도 직접 입력해야 한다. 세 필드(To/Subject/Body)
    // 중 지금 타이핑이 어디로 들어가는지는 self.new_mail.active 로 추적하고, 필드를
    // 클릭하면 그 필드로 옮겨간다. 위쪽에 진짜 Outlook Express "New Message" 창처럼
    // Send/Attach 아이콘 버튼이 늘어선 툴바를 두었다(참고 자료를 못 찾아 정확한
    // 버튼 순서/아이콘까지는 재현 못 했지만, "툴바가 필드 위에 있고 첨부는 별도
    // 버튼으로 고른다" 는 구조 자체는 그 시절 메일 클라이언트의 공통된 관례다).
    fn draw_new_compose(&mut self, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput, smooth: bool, lang: Language) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, WHITE);

        // 필드 배치는 실제 글자 길이와 무관하게 항상 같은 자리라(라벨/줄 높이가
        // 고정값), 그리기 전에 미리 다 계산해둘 수 있다 — 그래야 본문 클릭 판정을
        // (지금까지 그려진 걸 기준으로 뒤늦게 하는 대신) 입력 처리보다 먼저 해서,
        // "본문을 클릭해 커서를 옮기고 그 프레임에 바로 타이핑" 도 자연스럽게 된다.
        const FIELD_ROW_H: f32 = 20.0;
        const ATTACH_BOX_H: f32 = 26.0;
        const ATTACH_GAP: f32 = 2.0; // 첨부 박스끼리(그리고 본문 마지막 줄과) 거의 붙어 보이게
        const BTN_ROW_H: f32 = 30.0;
        const LINE_H: f32 = 20.0;
        const SB_W: f32 = 8.0;
        let cx = area.x + 8.0;
        let field_w = area.w - 16.0;
        let to_y = area.y + 6.0;
        let subject_y = to_y + FIELD_ROW_H;
        let fy = subject_y + FIELD_ROW_H + 4.0;
        // 첨부는 예전처럼 필드와 본문 사이에 따로 자리를 빼지 않는다 — Inbox/Sent
        // 읽기 화면과 같은 요령으로 본문 스크롤 영역 안, 마지막 줄 다음에 이어
        // 붙여서 같이 스크롤된다(여러 개를 붙일 수 있으니 한 줄로 고정해두면
        // 금방 좁아진다).
        let body_area = Rect::new(area.x, fy, area.w, (area.y + area.h - BTN_ROW_H - fy).max(20.0));
        let text_area = Rect::new(body_area.x + 6.0, body_area.y + 4.0, body_area.w - 14.0 - SB_W, body_area.h - 8.0);

        // 본문 클릭으로 커서를 옮기는 건 이 프레임에서 아직 편집 전(지금 화면에
        // 보이는 그대로)의 텍스트 기준으로 판정해야 한다 — 방금 그려졌던 자리를
        // 그대로 누른 것이니까. 편집이 끝난 뒤에는 줄이 다시 바뀔 수 있어서
        // 렌더링용으로 한 번 더 계산한다(아래).
        if win.mouse_clicked && win.focused && !self.new_mail.picker_open && body_area.contains(win.mouse.0, win.mouse.1) {
            let pre_lines = wrap_with_offsets(r, &self.new_mail.body, 1.0, text_area.w.max(10.0));
            let visible = ((text_area.h / LINE_H).floor() as usize).max(1);
            let max_scroll = (pre_lines.len() as f32 - visible as f32).max(0.0);
            // body_scroll(목표값) 대신 body_scroll_disp(부드럽게 따라가는 화면표시용
            // 값)을 써야 한다 — 스크롤 애니메이션이 아직 다 안 끝난 순간에 클릭하면
            // 실제로 화면에 보이는 자리와 안 맞을 수 있어서.
            let scroll = self.body_scroll_disp.clamp(0.0, max_scroll);
            let row = ((win.mouse.1 - text_area.y) / LINE_H + scroll).floor().max(0.0) as usize;
            let row = row.min(pre_lines.len().saturating_sub(1));
            if let Some((line, start)) = pre_lines.get(row) {
                let col = char_index_at_x(r, line, 1.0, win.mouse.0 - text_area.x);
                self.new_mail.active = ComposeField::Body;
                self.new_mail.cursor = start + col;
            }
        }

        // 첨부 목록이 펼쳐진 동안은 필드 자체가 안 보이니 입력을 건너뛴다.
        // 본문이 이 프레임에 편집(타이핑/백스페이스/엔터/화살표)됐으면 캐럿이
        // 보이도록 스크롤도 같이 맞춰야 하니 표시해둔다.
        let mut body_cursor_moved = false;
        if win.mouse_clicked && win.focused && !self.new_mail.picker_open && body_area.contains(win.mouse.0, win.mouse.1) {
            body_cursor_moved = true;
        }
        if win.focused && !self.new_mail.picker_open {
            let NewMailState { to, subject, body, cursor, active, .. } = &mut self.new_mail;
            let max_len_of = |f: ComposeField| if f == ComposeField::Body { COMPOSE_MAX_CHARS } else { 200 };
            for &c in &win.input.typed {
                if c.is_control() {
                    continue;
                }
                let field: &mut String = match *active {
                    ComposeField::To => &mut *to,
                    ComposeField::Subject => &mut *subject,
                    ComposeField::Body => &mut *body,
                };
                insert_char_at(field, cursor, c, max_len_of(*active));
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            // 결합 전 자모 여러 개(예: "ㄲ" 이전의 "ㄱㄱ")를 조합 중일 때 Backspace
            // 한 번 = Windows/한글 IME 가 자모 전체를 한 번에 지워버리는 문제 — IME
            // 내부 동작을 계산/가로채서 맞추려던 시도들은 전부 실패했다(경위는
            // README). 대신 이 조합 세션 동안 IME 가 실제로 보여준 화면을 우리가
            // 직접 기록해뒀다가(composition_history) "한 단계 전 기록"을 그대로
            // 꺼내 쓴다 — 계산이 아니라 조회라 틀릴 여지가 없다. 조합 버퍼에 다시
            // 넣는 건 안 먹히므로, 조합을 취소하고(`ime::cancel_composition`) 그
            // 값을 평범한 확정 문자로 직접 밀어넣는다. 짧게 한 번 누른 경우만
            // 처리한다(`pressed()` — 꾹 누르는 건 원래도 문제없었다).
            let cur_composition = crate::ime::composition_preview().unwrap_or_default();
            if win.input.pressed(KeyCode::Backspace) && !self.composition_history.is_empty() {
                let target =
                    if self.composition_history.len() >= 2 { self.composition_history[self.composition_history.len() - 2].clone() } else { String::new() };
                crate::ime::cancel_composition();
                if !target.is_empty() {
                    let field: &mut String = match *active {
                        ComposeField::To => &mut *to,
                        ComposeField::Subject => &mut *subject,
                        ComposeField::Body => &mut *body,
                    };
                    for ch in target.chars() {
                        insert_char_at(field, cursor, ch, max_len_of(*active));
                    }
                    if *active == ComposeField::Body {
                        body_cursor_moved = true;
                    }
                }
                // 취소했으니 이 조합 세션은 끝났다 — 기록도 같이 비운다.
                self.composition_history.clear();
            } else if cur_composition.is_empty() {
                self.composition_history.clear();
            } else if self.composition_history.last() != Some(&cur_composition) {
                self.composition_history.push(cur_composition.clone());
            }
            let composing = crate::ime::composition_preview().is_some_and(|p| !p.is_empty());
            let key_down = win.input.is_down(KeyCode::Backspace);
            if !key_down {
                self.backspace_hold = 0.0;
                self.backspace_fired = 0;
                self.backspace_owned_by_ime = false;
            } else if composing {
                self.backspace_owned_by_ime = true;
                self.backspace_hold = 0.0;
                self.backspace_fired = 0;
            } else if self.backspace_owned_by_ime {
                // 조합 중에 시작된 같은 누름이 이어지는 중 — 아직 키를 안 뗐으니
                // 계속 IME 몫으로 치고 우리는 손 안 댄다.
            } else {
                // win.dt 자체엔 500ms 상한(main.rs) 이 있지만, 그건 여기 기준으론
                // 여전히 너무 크다 — "누적된 hold 시간 → 반복 횟수" 로 바로
                // 환산하는 로직이라, 큰 dt 하나가 그대로 "그동안 계속 눌려있던
                // 것"으로 오인되면 한 프레임에 반복 삭제가 몰아서 발생한다(짧게
                // 한 번 눌렀는데 여러 글자가 지워지던 버그). 그래서 여기 한
                // 프레임에 반영하는 dt 는 반복 간격(BACKSPACE_REPEAT_INTERVAL)
                // 보다도 작게 별도로 한 번 더 눌러서, 큰 프레임 끊김이 있어도
                // 절대 한 프레임에 여러 번 반복 판정이 몰리지 않게 한다.
                self.backspace_hold += win.dt.min(BACKSPACE_MAX_DT_PER_FRAME);
            }
            let due = backspace_repeats_due(self.backspace_hold);
            let fire_count = due.saturating_sub(self.backspace_fired).min(6);
            if fire_count > 0 {
                self.backspace_fired = due;
                let field: &mut String = match *active {
                    ComposeField::To => &mut *to,
                    ComposeField::Subject => &mut *subject,
                    ComposeField::Body => &mut *body,
                };
                for _ in 0..fire_count {
                    backspace_at(field, cursor);
                }
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Left) {
                *cursor = cursor.saturating_sub(1);
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Right) {
                let len = match *active {
                    ComposeField::To => to.chars().count(),
                    ComposeField::Subject => subject.chars().count(),
                    ComposeField::Body => body.chars().count(),
                };
                *cursor = (*cursor + 1).min(len);
                if *active == ComposeField::Body {
                    body_cursor_moved = true;
                }
            }
            if !composing && win.input.pressed(KeyCode::Enter) {
                match *active {
                    ComposeField::To => {
                        *active = ComposeField::Subject;
                        *cursor = subject.chars().count();
                    }
                    ComposeField::Subject => {
                        *active = ComposeField::Body;
                        *cursor = body.chars().count();
                        body_cursor_moved = true;
                    }
                    ComposeField::Body => {
                        insert_char_at(body, cursor, '\n', COMPOSE_MAX_CHARS);
                        body_cursor_moved = true;
                    }
                }
            }
        }

        let to = self.new_mail.to.clone();
        if let Some(idx) =
            self.draw_editable_field(r, cx, to_y, field_w, t(lang, s::FIELD_TO), &to, self.new_mail.active == ComposeField::To, self.new_mail.cursor, win)
        {
            self.new_mail.active = ComposeField::To;
            self.new_mail.cursor = idx;
        }
        let subject = self.new_mail.subject.clone();
        if let Some(idx) = self.draw_editable_field(
            r, cx, subject_y, field_w, t(lang, s::FIELD_SUBJECT), &subject, self.new_mail.active == ComposeField::Subject,
            self.new_mail.cursor, win,
        ) {
            self.new_mail.active = ComposeField::Subject;
            self.new_mail.cursor = idx;
        }

        r.rect(body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h, WHITE);
        border(
            r, body_area.x + 4.0, body_area.y, body_area.w - 8.0, body_area.h,
            if self.new_mail.active == ComposeField::Body && !self.new_mail.picker_open { NAVY } else { [0.6, 0.6, 0.62, 1.0] },
        );

        // 첨부 목록이 펼쳐져 있으면 본문 대신 그 목록을 이 자리에 보여준다 —
        // 진짜 Outlook Express 도 첨부 파일 선택 창이 뜨면 그동안 본문 입력이
        // 잠깐 가려지는 것과 비슷한 느낌. 아래쪽 Send/Attach 버튼 줄은 두 경우
        // 모두에서 그려야 하므로(첨부 목록을 보는 중에도 Attach 를 다시 눌러 닫을
        // 수 있어야 한다) 여기서 조기 반환하지 않고 아래로 이어서 그린다. 여러
        // 개를 붙일 수 있으니 하나 고른다고 바로 닫지 않는다 — 이미 붙인 파일은
        // 체크 표시로 보여주고, 다시 누르면 그 자리에서 뗄 수 있다(토글).
        if self.new_mail.picker_open {
            let list_area = Rect::new(body_area.x + 6.0, body_area.y + 4.0, body_area.w - 12.0, body_area.h - 8.0);
            if self.attachable.is_empty() {
                label(
                    r, list_area.x + 4.0, list_area.y + 4.0,
                    t(lang, s::NO_FILES_TO_ATTACH),
                    GRAY,
                );
            } else {
                const ROW_H: f32 = 22.0;
                r.set_clip(Some(list_area));
                for (i, (id, name, icon)) in self.attachable.iter().enumerate() {
                    let ry = list_area.y + i as f32 * ROW_H;
                    if ry + ROW_H > list_area.y + list_area.h {
                        break;
                    }
                    let row = Rect::new(list_area.x, ry, list_area.w, ROW_H);
                    let attached = self.new_mail.attachments.iter().any(|(aid, ..)| aid == id);
                    let hover = row.contains(win.mouse.0, win.mouse.1);
                    if attached {
                        r.rect(row.x, row.y, row.w, row.h, [0.85, 0.93, 0.85, 1.0]);
                    } else if hover {
                        r.rect(row.x, row.y, row.w, row.h, [0.82, 0.88, 0.98, 1.0]);
                    }
                    draw_icon(r, assets, icon, row.x + 2.0, row.y + 2.0, 18.0);
                    let name_color = if attached { [0.15, 0.45, 0.15, 1.0] } else { BLACK };
                    r.text_clipped(row.x + 24.0, row.y + 3.0, &display_name(lang, name), 0.85, name_color, row.w - 28.0);
                    if win.mouse_clicked && hover {
                        if attached {
                            self.new_mail.attachments.retain(|(aid, ..)| aid != id);
                        } else {
                            self.new_mail.attachments.push((*id, name.clone(), *icon));
                        }
                    }
                }
                r.set_clip(None);
            }
            return self.draw_new_compose_buttons(r, area, win, lang);
        }

        // 조합 중인(아직 확정 안 된) 문자열을 캐럿 위치에 끼워넣어서만 화면에
        // 보여준다 — self.new_mail.body 자체는 여전히 char_event 로 확정된
        // 문자만으로 채워지고 이 값의 영향을 전혀 안 받는다. 화면상 캐럿도
        // 이 미리보기 뒤(effective_cursor)로 같이 옮겨서, 지금 막 조합 중인
        // 글자 바로 뒤에 캐럿이 있는 것처럼 보이게 한다.
        let body_preview =
            if self.new_mail.active == ComposeField::Body && win.focused { crate::ime::composition_preview() } else { None };
        let (display_body, effective_cursor) = match &body_preview {
            Some(p) if !p.is_empty() => {
                let byte_idx =
                    self.new_mail.body.char_indices().nth(self.new_mail.cursor).map(|(b, _)| b).unwrap_or(self.new_mail.body.len());
                let mut s = String::with_capacity(self.new_mail.body.len() + p.len());
                s.push_str(&self.new_mail.body[..byte_idx]);
                s.push_str(p);
                s.push_str(&self.new_mail.body[byte_idx..]);
                (s, self.new_mail.cursor + p.chars().count())
            }
            _ => (self.new_mail.body.clone(), self.new_mail.cursor),
        };
        // 렌더링용으로 (편집이 끝난) 최종 본문을 다시 접는다 — 위에서 클릭 판정에
        // 썼던 pre_lines 는 편집 전 스냅샷이라 여기서 재사용하면 안 된다.
        let lines = wrap_with_offsets(r, &display_body, 1.0, text_area.w.max(10.0));
        let visible = ((text_area.h / LINE_H).floor() as usize).max(1);
        let attach_unit_lines = (ATTACH_BOX_H + ATTACH_GAP) / LINE_H;
        let total_lines = lines.len() as f32 + attach_unit_lines * self.new_mail.attachments.len() as f32;
        let max_body_scroll = (total_lines - visible as f32).max(0.0);

        // 캐럿이 있는 줄을 찾는다 — effective_cursor 이하로 시작하는 줄들 중
        // 가장 마지막 것.
        let cursor_line = lines.iter().rposition(|&(_, start)| start <= effective_cursor).unwrap_or(0);

        // 이번 프레임에 커서가 움직였으면(클릭/타이핑/백스페이스/화살표/엔터)
        // 그 줄이 화면 안에 들어오도록 스크롤을 당긴다 — 그 외엔 휠로 스크롤한
        // 값을 그대로 둔다(편집 중이 아닌데 계속 맨 아래로 끌려 내려가던 문제).
        if body_cursor_moved {
            if (cursor_line as f32) < self.body_scroll {
                self.body_scroll = cursor_line as f32;
            } else if (cursor_line as f32) > self.body_scroll + (visible as f32 - 1.0) {
                self.body_scroll = (cursor_line as f32 - (visible as f32 - 1.0)).max(0.0);
            }
        }
        if body_area.contains(win.mouse.0, win.mouse.1) {
            self.body_scroll -= win.wheel / 120.0 * 3.0;
        }
        self.body_scroll = self.body_scroll.clamp(0.0, max_body_scroll);
        ease_scroll(&mut self.body_scroll_disp, self.body_scroll, win.dt, smooth);

        let line_off = self.body_scroll_disp * LINE_H;
        let first = self.body_scroll_disp as usize;
        r.set_clip(Some(text_area));
        let mut ty = text_area.y - (line_off - first as f32 * LINE_H);
        for (i, (line, _)) in lines.iter().enumerate().skip(first) {
            if ty > text_area.y + text_area.h {
                break;
            }
            r.text(text_area.x, ty, line, 1.0, BLACK);
            // 캐럿은 실제로 그 줄이 이번에 화면에 그려졌을 때만(스크롤 때문에
            // 화면 밖으로 밀려났으면 그리지 않는다) 그린다 — 조합 중인 글자는
            // 위에서 이미 텍스트에 직접 끼워넣어 보여줬으므로, 여기 캐럿은 그냥
            // 평소처럼 깜빡이기만 한다.
            if self.new_mail.active == ComposeField::Body && win.focused && i == cursor_line {
                let (line_text, line_start) = &lines[i];
                let col = effective_cursor.saturating_sub(*line_start).min(line_text.chars().count());
                let byte_idx = line_text.char_indices().nth(col).map(|(b, _)| b).unwrap_or(line_text.len());
                let cursor_x = text_area.x + r.text_width(&line_text[..byte_idx], 1.0);
                // IME 팝업들은 화면 밖으로 치운다 — To/Subject 필드와 같은 이유
                // (draw_editable_field 참고).
                crate::ime::set_composition_pos(OFFSCREEN, OFFSCREEN);
                crate::ime::hide_cicero_windows();
                if (win.time % 1.0) < 0.5 {
                    r.rect(cursor_x + 1.0, ty + 1.0, 2.0, CELL_H - 4.0, BLACK);
                }
            }
            ty += LINE_H;
        }

        // 첨부파일들 — 본문 마지막 줄 다음부터 하나씩 이어 붙여서 같이
        // 스크롤된다(Inbox/Sent 읽기 화면과 같은 요령). 각 박스에 Remove 버튼이
        // 있어서 그 자리에서 바로 뗄 수 있다.
        let mut removed = None;
        for (i, (id, name, icon)) in self.new_mail.attachments.iter().enumerate() {
            let box_y = text_area.y + (lines.len() as f32 + attach_unit_lines * i as f32) * LINE_H + ATTACH_GAP - line_off;
            let box_w = body_area.w - 16.0 - SB_W - 4.0;
            let box_h = ATTACH_BOX_H;
            if box_y + box_h > text_area.y && box_y < text_area.y + text_area.h {
                r.rect(cx, box_y, box_w, box_h, WHITE);
                border(r, cx, box_y, box_w, box_h, [0.6, 0.6, 0.62, 1.0]);
                draw_icon(r, assets, icon, cx + 4.0, box_y + 3.0, 18.0);
                let remove_label = t(lang, s::REMOVE);
                let remove_w = r.text_width(remove_label, 0.8) + 16.0;
                let remove_x = cx + box_w - remove_w - 4.0;
                let name_max_w = (remove_x - (cx + 28.0) - 6.0).max(10.0);
                r.text_clipped(cx + 28.0, box_y + 5.0, &display_name(lang, name), 0.85, BLACK, name_max_w);
                if button(r, remove_x, box_y + 2.0, remove_w, box_h - 4.0, remove_label, win) {
                    removed = Some(*id);
                }
            }
        }
        r.set_clip(None);
        if let Some(id) = removed {
            self.new_mail.attachments.retain(|(aid, ..)| *aid != id);
        }
        if max_body_scroll > 0.0 {
            let sb_x = body_area.x + body_area.w - SB_W - 6.0;
            let frac = (visible as f32 / total_lines).min(1.0);
            scrollbar(r, win, sb_x, body_area.y + 4.0, SB_W, body_area.h - 8.0, frac, self.body_scroll_disp, &mut self.body_scroll, max_body_scroll, &mut self.body_sb_drag);
        }

        self.draw_new_compose_buttons(r, area, win, lang)
    }

    // "Write Mail" 하단의 Send/Attach 버튼 줄 — 원래 위쪽에 큼직한 툴바로 뒀었는데
    // ("보내기랑 첨부를 아래쪽으로 옮겨줘. 그리고 버튼 사이즈도 좀 줄여줘") 요청을
    // 받고 화면 맨 아래 작은 버튼 두 개로 옮겼다. 첨부 목록이 펼쳐져 있는 중에도
    // 그려야 해서(Attach 를 다시 눌러 닫을 수 있게) draw_new_compose 끝과 첨부 목록
    // 분기 양쪽에서 공유해 부른다.
    fn draw_new_compose_buttons(&mut self, r: &mut Renderer, area: Rect, win: &WinInput, lang: Language) -> AppAction {
        const BTN_ROW_H: f32 = 30.0;
        const BTN_H: f32 = 20.0;
        let by = area.y + area.h - BTN_ROW_H + (BTN_ROW_H - BTN_H) / 2.0;
        r.rect(area.x, area.y + area.h - BTN_ROW_H, area.w, 1.0, GRAY);

        let attach_label = t(lang, s::ATTACH_BUTTON);
        let attach_w = r.text_width(attach_label, 0.85) + 16.0;
        let send_label = t(lang, s::SEND_BUTTON);
        let send_w = r.text_width(send_label, 0.85) + 16.0;
        // 오른쪽 정렬 — Send 가 가장 오른쪽(가장 눌릴 확률이 높은 주 동작).
        let send_x = area.x + area.w - send_w - 8.0;
        let attach_x = send_x - attach_w - 6.0;

        let attach_rect = Rect::new(attach_x, by, attach_w, BTN_H);
        if self.new_mail.picker_open {
            sunken(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            let tw = r.text_width(attach_label, 0.85);
            r.text(attach_rect.x + (attach_w - tw) / 2.0 + 1.0, attach_rect.y + (BTN_H - CELL_H * 0.85) / 2.0 + 1.0, attach_label, 0.85, BLACK);
            if win.mouse_clicked && attach_rect.contains(win.mouse.0, win.mouse.1) {
                self.new_mail.picker_open = false;
            }
        } else {
            let hover = attach_rect.contains(win.mouse.0, win.mouse.1);
            let pressed = hover && win.mouse_down;
            if pressed {
                sunken(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            } else {
                raised(r, attach_rect.x, attach_rect.y, attach_rect.w, attach_rect.h);
            }
            let tw = r.text_width(attach_label, 0.85);
            let off = if pressed { 1.0 } else { 0.0 };
            r.text(attach_rect.x + (attach_w - tw) / 2.0 + off, attach_rect.y + (BTN_H - CELL_H * 0.85) / 2.0 + off, attach_label, 0.85, BLACK);
            if hover && win.mouse_clicked {
                self.new_mail.picker_open = true;
            }
        }

        let can_send = !self.new_mail.to.trim().is_empty() && !self.new_mail.body.trim().is_empty();
        if can_send {
            let hover = Rect::new(send_x, by, send_w, BTN_H).contains(win.mouse.0, win.mouse.1);
            let pressed = hover && win.mouse_down;
            if pressed {
                sunken(r, send_x, by, send_w, BTN_H);
            } else {
                raised(r, send_x, by, send_w, BTN_H);
            }
            let tw = r.text_width(send_label, 0.85);
            let off = if pressed { 1.0 } else { 0.0 };
            r.text(send_x + (send_w - tw) / 2.0 + off, by + (BTN_H - CELL_H * 0.85) / 2.0 + off, send_label, 0.85, BLACK);
            if hover && win.mouse_clicked {
                // 발송 확인 화면 없이 곧바로 처리한다 — Sent Items 목록에 바로
                // 하나 덧붙이고(이 자리에서 즉시 보이도록), 폼은 다음 메일을 쓸 수
                // 있게 비운다. fs.sent_mail 쪽 진짜 기록은 desktop.rs 가 아래
                // AppAction 을 받아서 남긴다.
                let to = self.new_mail.to.clone();
                let subject = self.new_mail.subject.clone();
                let body = self.new_mail.body.clone();
                let attachments_view = self.new_mail.attachments.clone();
                let attachments = attachments_view.iter().map(|(id, name, _)| (*id, name.clone())).collect();
                self.sent.push(SentMailView { to: to.clone(), subject: subject.clone(), body: body.clone(), attachments: attachments_view });
                self.new_mail = NewMailState::new();
                return AppAction::SendNewMail { to, subject, body, attachments };
            }
        } else {
            raised(r, send_x, by, send_w, BTN_H);
            let tw = r.text_width(send_label, 0.85);
            r.text(send_x + (send_w - tw) / 2.0, by + (BTN_H - CELL_H * 0.85) / 2.0, send_label, 0.85, [0.55, 0.55, 0.55, 1.0]);
        }

        AppAction::None
    }
}
