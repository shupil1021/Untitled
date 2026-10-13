//! 메일 앱 — Outlook Express 스타일: 위에 메뉴바(장식용, 클릭 안 먹음), 그 아래
//! 왼쪽에 폴더 트리(Inbox + Write Mail — Sent Items/Deleted Items/Drafts 는 임시
//! 보관/삭제 기능 자체가 아직 없어서 트리에서 뺐다), 오른쪽엔 목록/읽기/작성 화면.
//! 왼쪽 트리와 오른쪽 내용을 나누는 세로줄은 드래그로 폭을 조절할 수 있다
//! (explorer.rs 의 사이드바와 같은 요령).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{display_name, FileId, Language, MailId, Settings};
use crate::render::gfx::{Assets, Color, Rect, Renderer, CELL_H};
use crate::strings::{mail as s, t};
use crate::ui::*;

use super::widgets::{ease_scroll, scrollbar, Scrollbar};
use super::{App, AppAction, WinInput};

struct MailMsg {
    from: &'static str,
    to: &'static str,
    cc: &'static str,
    subject: &'static str,
    body: &'static str,
    attachment: Option<(FileId, String, IconType)>,
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

// 첫 메일에 붙는 첨부(게임 설치 파일) — apps/mod.rs::open() 이 fs 에서 구해 넘긴다.
// downloaded 는 "이미 한 번 받은 적 있는지"(fs.ever_downloaded) — 창을 새로 열어도
// 다시 "Download" 버튼이 뜨지 않게 초기 표시 상태로 쓴다.
pub struct MailAttachment {
    pub id: FileId,
    pub name: String,
    pub icon: IconType,
    pub downloaded: bool,
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

pub struct MailApp {
    messages: Vec<MailMsg>,
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
        log: &[MailId],
        read_indices: &[usize],
        attachments: Vec<Option<MailAttachment>>,
        attachable: Vec<(FileId, String, IconType)>,
        sent: Vec<SentMailView>,
        settings: Rc<RefCell<Settings>>,
    ) -> MailApp {
        let lang = settings.borrow().language;
        // 메시지마다 "이미 받은 적 있는지"(fs.ever_downloaded) — 창을 새로 열어도 다시 "Download"
        // 버튼이 안 뜨게 초기 표시 상태로 쓴다.
        let already: Vec<bool> = log.iter().enumerate().map(|(i, _)| attachments.get(i).and_then(|a| a.as_ref()).is_some_and(|a| a.downloaded)).collect();
        let messages = seed_messages(log, lang, attachments.into_iter().map(|a| a.map(|a| (a.id, a.name, a.icon))).collect());
        let read = (0..messages.len()).map(|i| read_indices.contains(&i)).collect();
        let downloaded = messages.iter().enumerate().map(|(i, m)| m.attachment.is_some() && already[i]).collect();
        let downloading = vec![None; messages.len()];
        MailApp {
            messages,
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

mod compose;
mod content;
mod inbox;
mod sent;

pub use content::{mail_from, mail_subject};
use compose::NewMailState;
use content::seed_messages;
