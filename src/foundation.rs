//! 자주 안 바뀔 만한 기반 모듈들을 한 파일로 묶어뒀다 — 가짜 파일 시스템(fs),
//! 공유 설정값(settings), 저장/불러오기(save). 각각 원래 독립된 파일이었지만 다들
//! 작고 안정적이라(UI/앱 쪽처럼 자주 손댈 일이 없음) 파일 개수를 줄이려고 여기로
//! 합쳤다. 아래 섹션 구분선 기준으로 원래 파일 경계가 어디였는지 알 수 있다. 씬
//! 프레임워크(Scene/Frame/Transition/SceneManager)는 씬들이 이제 각자 파일로
//! 나뉘어서(scenes/ 디렉터리) 그쪽 mod.rs 로 옮겨갔다.

// ================= 가짜 파일 시스템 (구 fs.rs) =================
// 모든 파일/폴더를 하나의 Vec(아레나)에 담고 FileId(인덱스)로 참조한다.
// 폴더/잠금파일은 자식들을 FileId 로 가리킨다.

pub type FileId = usize;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum FileKind {
    Txt(String),                                       // 메모장 텍스트
    Mp4,                                                // 영상
    Img(usize),                                         // 이미지 — Assets::photos 의 인덱스
    Sound(usize),                                       // 사운드(.wav) — gamefiles::SOUND_NAMES 의 인덱스
    Lock { password: String, children: Vec<FileId> },  // 잠금(풀리면 폴더로)
    Folder { children: Vec<FileId> },                  // 일반 폴더 (잠금 풀리면 이걸로 변함)
    // #[serde(rename)] 로 저장 파일의 JSON 태그는 예전 이름("Email") 그대로 유지한다
    // — Rust 쪽 이름만 Mail 로 바꿔서 예전 저장 파일도 계속 불러와진다.
    #[serde(rename = "Email")]
    Mail { attachment: Option<FileId> },                // 메일 앱 (첨부파일 하나까지)
    Explorer,                                           // 바탕화면의 File Explorer (탭 있는 탐색기)
    // 메일로 받는 게임 설치 파일("test Setup.exe") — 열면 설치 마법사
    // (apps/game_installer.rs)가 뜬다. 메일로 오는 게임은 항상 이 Setup 파일로 온다.
    GameSetup,
    // 두 번째 메일 뒤 "다운로드 완료" 알림과 함께 바탕화면에 생기는 서브 게임 아이콘
    // ("test2.exe") — 열면 미로 게임(apps/maze_game)이 OS 창 안에서 뜬다.
    SubGame,
    // 설치 마법사가 끝나면 바탕화면에 생기는 게임 아이콘("test.exe") — 친구가 메일로
    // 보낸 크랙 게임(STORY.md 7-1절). 열면 OS 안의 창 하나로 3D 게임
    // (apps/doors_game.rs)이 돈다.
    Game,
    Deleted,                                            // FileSystem::delete_permanently() 로 지워진 자리 — 그 무엇에서도 더는 참조되지 않는다
}

// 일부 fs 노드는 이름 자체가 "이건 특수 노드다"라는 표식으로 쓰인다(전용
// FileKind 대신 이름 문자열로 구분) — 예: Folder 중에서 이름이 정확히
// RECYCLE_BIN_NAME 인 것만 휴지통 취급. 화면엔 display_name()/strings.rs::t()
// 로 언어별 문구가 나가지만, fs 안에 실제로 저장되는 원문은 항상 이 영어
// 상수 그대로다. 리터럴 "My Computer"/"Recycle Bin" 을 여러 파일에 따로
// 타이핑하면 오타 하나로 매칭이 조용히 깨질 수 있어 상수로 모아뒀다.
pub const MY_COMPUTER_NAME: &str = "My Computer";
pub const RECYCLE_BIN_NAME: &str = "Recycle Bin";
// File Explorer 의 "AppData" 탭 = 이 이름의 폴더 노드의 내용(탭 이름도 같은 문자열).
pub const APPDATA_NAME: &str = "AppData";
// 첫 메일에 첨부돼 오는 게임 설치 파일(FileKind::GameSetup)과, 설치가 끝나면
// 바탕화면에 생기는 게임 아이콘(FileKind::Game)의 fs 이름.
pub const GAME_SETUP_NAME: &str = "test Setup.exe";
pub const GAME_FILE_NAME: &str = "test.exe";
// 설치할 때 게임 안 AppData 폴더 밑에 만들어지는 게임 폴더 이름(gamefiles.rs) — 게임 이름(.exe 뗀 것).
pub const GAME_FOLDER_NAME: &str = "test";
pub const SUB_GAME_NAME: &str = "test2.exe";

#[derive(Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub kind: FileKind,
}

// FileSystem 자체를 통째로 Serialize/Deserialize 한다 — 저장 파일이 곧 이 구조체의
// 스냅샷이라, 이름으로 다시 찾아 재구성하는 대신 있었던 그대로 복원된다.
#[derive(Clone, Serialize, Deserialize)]
pub struct FileSystem {
    nodes: Vec<FileNode>,
    pub desktop: Vec<FileId>,   // 바탕화면에 놓인 파일들
    pub downloads: Vec<FileId>, // 지금 실제로 Downloads 탭 안에 있는 파일들(위치) — 바탕화면/폴더로
                                 // 옮기면 여기서 빠진다.
    // "한 번이라도 다운로드한 적 있는지" — downloads 와 달리 나중에 바탕화면/폴더로
    // 옮겨도 절대 안 빠진다. Mail 이 재다운로드 버튼을 보여줄지 판단할 때 downloads
    // 대신 이걸 봐야 한다 — downloads 를 그대로 썼더니, 다운로드한 첨부파일(예: Photos
    // 폴더)을 바탕화면으로 옮기면 downloads 에서 빠지면서 "아직 안 받음" 취급돼 다시
    // 다운로드 버튼이 나타나는 문제가 있었다.
    pub ever_downloaded: Vec<FileId>,
    // 첫 메일(게임 다운로드 파일 첨부)이 도착했는지 — 도착 전엔 받은편지함이 빈
    // 상태. desktop.rs 의 타이머가 새 게임을 시작하고 일정 시간 뒤 true 로 바꾼다.
    #[serde(default)]
    pub mail_arrived: bool,
    // 읽은 메일의 인덱스(MailApp::seed_messages 순번) — MailApp 자체는 창을 닫거나
    // 3초 주기 새로고침으로 새로 만들어질 때마다 통째로 새 인스턴스가 되므로, 읽음
    // 여부를 여기(저장 파일에 실리는 fs)에 둬야 새로고침은 물론 게임을 종료했다
    // 재시작해도 유지된다. #[serde(default)] 는 이 필드가 없던 예전 저장 파일도
    // (그냥 다 안 읽은 것으로) 계속 불러올 수 있게 해준다.
    // 두 번째 메일(방에 꽃이 없다는 걸 확인하면 오는, 꽃을 구할 수 있는 곳을 안다는 메일)이
    // 도착했는지 — 첫 메일과 달리 타이머가 아니라 게임 안 이벤트(AppAction::
    // FlowerAbsenceChecked)로 도착한다.
    #[serde(default)]
    pub mail2_arrived: bool,
    // 두 번째 메일 뒤에 더 도착한 메일 수 — (1) 씨앗을 심은 뒤 오는 시간 힌트 메일, (2) 꽃을
    // 우체통에 넣은 뒤 꽃 사진이 첨부돼 돌아오는 메일. 모두 게임 안 이벤트로 도착한다.
    #[serde(default)]
    pub extra_mails: usize,
    // 서브 게임에서 씨앗을 심었는지 — 힌트 메일이 올 차례인지 판단하는 데 쓴다.
    #[serde(default)]
    pub seed_planted: bool,
    // 꽃 사진 첨부 노드(flower.png) — 꽃이 발송될 때 처음 만들어진다. 바탕화면/Downloads 어디에도
    // 안 놓이고 메일에서 "Download" 해야 Downloads 에 생긴다.
    #[serde(default)]
    pub flower_image: Option<FileId>,
    #[serde(default, rename = "email_read")]
    pub mail_read: Vec<usize>,
    // Mail 의 "Write Mail" 탭에서 실제로 보낸 메일들 — Mail 앱의 "Sent Items" 탭에
    // 그대로 보여주려고 내용째(받는 사람/제목/본문/첨부) 저장한다. 최신 메일이
    // 뒤에 붙는다(보낸 순서 그대로).
    #[serde(default)]
    pub sent_mail: Vec<SentMail>,
    // 휴지통에 들어가기 직전에 어디 있었는지 — 복구("Restore")할 때 무조건
    // 바탕화면이 아니라 원래 있던 자리로 돌려놓는 데 쓴다. 휴지통 밖으로 나가면
    // (복구되든, 다른 곳으로 다시 옮겨지든) desktop.rs 가 이 기록을 지운다.
    #[serde(default)]
    pub trash_origin: Vec<(FileId, FileOrigin)>,
    // 설치 마법사(GameSetup)를 끝까지 간 적 있는지 — 켜져 있으면 Setup.exe 를 다시
    // 열어도 마법사 없이 "이미 설치됨" 페이지로 연다(바탕화면 아이콘 중복 방지).
    #[serde(default)]
    pub game_installed: bool,
    // 서브 게임(test2.exe)이 바탕화면에 이미 생겼는지 — 두 번째 메일이 온 뒤 잠깐 있다가
    // "다운로드 완료" 알림과 함께 한 번만 생긴다.
    #[serde(default)]
    pub sub_game_ready: bool,
}

// Mail 의 "Write Mail" 탭에서 보낸 메일 한 통 — fs.sent_mail 에 쌓인다. 첨부는
// 여러 개를 붙일 수 있어서 Vec(붙인 순서 그대로). #[serde(default)] 는 이
// 필드가 단수 attachment(Option) 였던 예전 저장 파일도(그냥 첨부 없는 것으로)
// 계속 불러올 수 있게 해준다 — 필드 이름 자체가 바뀌어서 예전 값은 어차피 못
// 읽지만, 최소한 그 필드가 아예 없다는 이유로 로드 전체가 깨지지는 않는다.
#[derive(Clone, Serialize, Deserialize)]
pub struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<(FileId, String)>,
}

// FileSystem::trash_origin 이 기억해두는 "휴지통에 들어가기 전 위치" — MoveDest
// (apps/mod.rs) 와 거의 같은 모양이지만, foundation.rs 는 apps 모듈에 의존하면 안
// 되는 더 기반 레이어라 똑같은 뜻의 타입을 여기 따로 둔다.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum FileOrigin {
    Desktop,
    Downloads,
    Folder(FileId),
    // photo01.jpg 처럼 바탕화면/Downloads/어떤 폴더의 children 에도 안 들어있고, 그냥
    // FileKind::Img/Mp4 라는 이유만으로 File Explorer 의 Images/Videos 가상 탭에
    // 보이던 파일 — locate() 가 아무 컨테이너도 못 찾으면(= 원래 어디에도 "속해"
    // 있지 않았으면) 이 값으로 기록한다. 복구할 때는 어디에도 다시 안 넣는다 —
    // 휴지통 children 에서만 빠지면(detach_from_container) all_of_kind() 기반인
    // 그 가상 탭에 저절로 다시 나타난다.
    Loose,
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

// FileSystem::download() 이 쓴다 — existing(지금 Downloads 탭에 이미 있는
// 파일들의 이름) 안에 name 과 완전히 같은 게 있으면, 실제 Windows 탐색기가
// 중복 다운로드를 처리하듯 확장자 앞에 "(1)", "(2)"... 를 붙여 안 겹치는
// 이름을 찾아 돌려준다. 겹치는 게 없으면 원래 이름 그대로.
fn dedupe_download_name(existing: &[String], name: &str) -> String {
    if !existing.iter().any(|n| n == name) {
        return name.to_string();
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s, format!(".{e}")),
        None => (name, String::new()),
    };
    let mut n = 1u32;
    loop {
        let candidate = format!("{stem}({n}){ext}");
        if !existing.iter().any(|x| x == &candidate) {
            return candidate;
        }
        n += 1;
    }
}

impl FileSystem {
    pub fn new() -> FileSystem {
        let mut fs = FileSystem {
            nodes: Vec::new(),
            desktop: Vec::new(),
            downloads: Vec::new(),
            ever_downloaded: Vec::new(),
            mail_arrived: false,
            mail2_arrived: false,
            extra_mails: 0,
            seed_planted: false,
            flower_image: None,
            mail_read: Vec::new(),
            sent_mail: Vec::new(),
            trash_origin: Vec::new(),
            game_installed: false,
            sub_game_ready: false,
        };

        // 바탕화면엔 고정 아이콘들만 둔다 — 나머지 예제 파일들은 다 치웠다.
        let explorer = fs.add(MY_COMPUTER_NAME, FileKind::Explorer);
        let mail = fs.add("Mail", FileKind::Mail { attachment: None });
        fs.ensure_game_attachment();
        fs.ensure_appdata();

        // 휴지통도 그냥 이름이 "Recycle Bin"인 빈 Folder — 드래그로 파일을 옮기면
        // desktop_folder_drop_target_at 이 다른 폴더와 똑같이 인식하고, 더블클릭하면
        // apps/mod.rs 의 Explorer 열기 경로도 그대로 탄다. 아이콘만 icon_of() 에서
        // 이름으로 특수 취급(비었으면 RecycleEmpty, 아니면 RecycleFull).
        let recycle_bin = fs.add(RECYCLE_BIN_NAME, FileKind::Folder { children: vec![] });

        fs.desktop = vec![recycle_bin, explorer, mail];

        fs
    }

    // File Explorer 의 "AppData" 탭이 보여주는 폴더 노드 — 바탕화면에는 안 놓이고 탭으로만
    // 보인다. 없으면 만든다(예전 저장 파일에도 그대로 채워 넣는다). 설치된 게임 폴더가 이
    // 안에 들어간다(gamefiles.rs).
    pub fn ensure_appdata(&mut self) -> FileId {
        match self.find_by_name(APPDATA_NAME) {
            Some(id) => id,
            None => self.add(APPDATA_NAME, FileKind::Folder { children: Vec::new() }),
        }
    }

    // 지금까지 도착한 메일 개수(도착 순서대로 앞에서부터) — MailApp 이 그만큼만 보여준다.
    pub fn mail_arrived_count(&self) -> usize {
        self.mail_arrived as usize + self.mail2_arrived as usize + self.extra_mails
    }

    // 꽃 사진 첨부 노드를 (없으면) 만든다.
    pub fn ensure_flower_image(&mut self) -> FileId {
        match self.flower_image {
            Some(id) => id,
            None => {
                let id = self.add(crate::gamefiles::FLOWER_IMAGE_NAME, FileKind::Img(crate::gamefiles::FLOWER_IMAGE));
                self.flower_image = Some(id);
                id
            }
        }
    }

    // 받은편지함 메시지 순서대로의 첨부 노드 — (1) 게임 설치 파일, (2) 없음, (3) 없음, (4) 꽃 사진.
    pub fn mail_attachments(&self) -> Vec<Option<FileId>> {
        vec![self.mail_attachment(), None, None, self.flower_image]
    }

    // Mail 노드의 첨부(첫 메일에 붙어오는 게임 설치 파일)가 아직 없으면 만들어
    // 붙인다 — 어디에도(바탕화면/Downloads/폴더) 안 넣고 노드만 만들어둔다(메일에서
    // "다운로드"해야 Downloads 에 생긴다). 새 게임은 물론, 이 첨부가 생기기 전에
    // 저장된 예전 저장 파일을 불러왔을 때도(desktop.rs 가 불러온 직후 호출) 그대로
    // 채워 넣는다. 잠깐 있었던 "설치 없이 바로 실행되는 DOORS.exe"(FileKind::Game)
    // 가 첨부로 걸린 저장 파일이면 그 노드를 Setup 파일로 바꿔준다.
    pub fn ensure_game_attachment(&mut self) {
        let Some(mail) = self.find_by_name("Mail") else { return };
        match self.nodes[mail].kind {
            FileKind::Mail { attachment: Some(id) } => {
                if matches!(self.nodes[id].kind, FileKind::Game) {
                    self.nodes[id].kind = FileKind::GameSetup;
                }
            }
            _ => {
                let setup = self.add(GAME_SETUP_NAME, FileKind::GameSetup);
                self.nodes[mail].kind = FileKind::Mail { attachment: Some(setup) };
            }
        }
        // 게임 이름이 바뀐 적이 있어서(DOORS → test) 예전 저장 파일에 남은 설치
        // 파일/게임 아이콘 이름도 지금 이름으로 맞춘다.
        for node in self.nodes.iter_mut() {
            match node.kind {
                FileKind::GameSetup => node.name = GAME_SETUP_NAME.to_string(),
                FileKind::Game => node.name = GAME_FILE_NAME.to_string(),
                _ => {}
            }
        }
    }

    // Mail 에 붙은 첨부 파일 — 영구 삭제된 뒤엔(Deleted) 없는 것으로 친다.
    pub fn mail_attachment(&self) -> Option<FileId> {
        let mail = self.find_by_name("Mail")?;
        match self.nodes[mail].kind {
            FileKind::Mail { attachment: Some(id) } if !matches!(self.nodes[id].kind, FileKind::Deleted) => Some(id),
            _ => None,
        }
    }

    pub fn add(&mut self, name: &str, kind: FileKind) -> FileId {
        self.nodes.push(FileNode { name: name.to_string(), kind });
        self.nodes.len() - 1
    }

    pub fn get(&self, id: FileId) -> &FileNode {
        &self.nodes[id]
    }

    // desktop.rs 가 Credits/Official Site 처럼 "실제 파일은 아니지만 중복으로
    // 못 열리게 창 중복-열기 판정에만 쓰는" 가짜 FileId(usize::MAX 근처 값)를
    // WindowManager::file_at() 이 그대로 돌려줄 수 있다 — 그 값을 실수로 get()
    // 에 넘기면 배열 인덱스 범위를 한참 벗어나 그 자리에서 패닉(게임 전체가
    // 죽음)한다. 진짜 fs.nodes 안의 id 인지 먼저 확인할 때 쓴다.
    pub fn contains(&self, id: FileId) -> bool {
        id < self.nodes.len()
    }

    pub fn find_by_name(&self, name: &str) -> Option<FileId> {
        (0..self.nodes.len()).find(|&i| self.nodes[i].name == name)
    }

    // 폴더 위치와 상관없이 전체에서 조건에 맞는 파일들을 찾는다 — File Explorer 의
    // Videos/Images 탭처럼 "어디 있든 이 종류인 파일 전부" 를 보여줄 때 쓴다.
    pub fn all_of_kind(&self, pred: impl Fn(&FileKind) -> bool) -> Vec<FileId> {
        (0..self.nodes.len()).filter(|&i| pred(&self.nodes[i].kind)).collect()
    }

    // 메일 첨부파일 등을 "다운로드" — Downloads 탭에 추가한다(이미 있으면 무시).
    // Downloads 탭에 이미 같은 이름의 파일이 있으면(실제 Windows 탐색기가
    // 그러듯) "이름(1).확장자", "이름(2).확장자" 식으로 번호를 붙여 구분한다.
    pub fn download(&mut self, id: FileId) {
        if !self.downloads.contains(&id) {
            let existing_names: Vec<String> = self.downloads.iter().map(|&i| self.nodes[i].name.clone()).collect();
            let name = self.nodes[id].name.clone();
            self.nodes[id].name = dedupe_download_name(&existing_names, &name);
            self.downloads.push(id);
        }
        if !self.ever_downloaded.contains(&id) {
            self.ever_downloaded.push(id);
        }
    }

    // 잠금 파일을 폴더로 변환(비밀번호가 풀렸을 때).
    pub fn unlock(&mut self, id: FileId) {
        if let FileKind::Lock { children, .. } = &self.nodes[id].kind {
            let children = children.clone();
            let base = self.nodes[id].name.trim_end_matches(".lock").to_string();
            self.nodes[id].name = base;
            self.nodes[id].kind = FileKind::Folder { children };
        }
    }

    // 파일을 영구히 지운다 — 휴지통 비우기 등에 쓴다. 인덱스
    // 기반 FileId 를 그대로 다른 곳(폴더 children, 저장 파일 등)에서 계속 쓰고
    // 있어서 아레나에서 물리적으로 빼버리면(Vec::remove) 그 뒤 인덱스가 전부
    // 밀려 다른 참조가 깨진다 — 그 대신 downloads 목록과 모든 폴더의 children 에서만
    // 이 id 를 빼고, 노드 자체는 FileKind::Deleted 로 표시해 그 무엇에서도 다시 안
    // 보이게 한다(all_of_kind 같은 전체 스캔에도 안 걸림). 바탕화면(desktop)은 여기서
    // 안 건드린다 — icon_pos 와 인덱스를 맞춰야 해서 DesktopScene 이 호출 전에 직접
    // 뗀다(detach_from_container 와 같은 이유).
    pub fn delete_permanently(&mut self, id: FileId) {
        // My Computer(FileKind::Explorer)는 어떤 경로로 이 함수가 불려도 절대 지워지지
        // 않게 이중으로 막는다 — desktop.rs::move_ids_to 가 애초에 휴지통/폴더로 못
        // 옮기게 막아뒀지만, 혹시 다른 경로로 id 가 흘러들어와도 여기서 최종 방어선.
        if matches!(self.nodes[id].kind, FileKind::Explorer) {
            return;
        }
        self.detach_from_container(id);
        self.nodes[id].kind = FileKind::Deleted;
        self.nodes[id].name = "(deleted)".to_string();
    }

    // File Explorer 드래그로 파일을 옮길 때, 그리고 delete_permanently 가 지우기
    // 전에 흔적을 지울 때 둘 다 쓴다 — Downloads 목록과 모든 폴더의 children 에서만
    // 뗀다. 바탕화면(desktop)은 일부러 안 건드린다: DesktopScene 의 icon_pos 가
    // fs.desktop 과 같은 인덱스로 짝지어져 있어서, 여기서 retain 으로 빼버리면
    // icon_pos 만 안 줄어들어 그 뒤로 인덱스가 다 어긋난다 — 그래서 바탕화면 쪽
    // 제거/추가는 항상 DesktopScene 이 icon_pos 와 함께 직접 처리한다.
    pub fn detach_from_container(&mut self, id: FileId) {
        self.downloads.retain(|&d| d != id);
        for node in self.nodes.iter_mut() {
            if let FileKind::Folder { children } = &mut node.kind {
                children.retain(|&c| c != id);
            }
        }
    }

    // 파일을 실제 폴더(잠금 풀린 Photos 등) 안으로 옮긴다 — 이미 그 폴더 안에 있으면 무시.
    pub fn add_to_folder(&mut self, folder_id: FileId, id: FileId) {
        // 폴더를 자기 자신 안으로 옮기면(예: 열려서 드릴다운 탭으로 보이고 있는
        // 폴더를 그 탭 안으로 다시 드래그) 자기 자신을 자기 children 에 넣는
        // 자기참조 상태가 된다 — 휴지통 자기참조를 막던 것과 같은 종류의 버그라,
        // 여기 공용 함수에서 한 번에 막는다(호출부마다 따로 검사할 필요 없이).
        if folder_id == id {
            return;
        }
        if let FileKind::Folder { children } = &mut self.nodes[folder_id].kind
            && !children.contains(&id)
        {
            children.push(id);
        }
    }

    // 지금 이 파일이 어디 있는지 — 휴지통으로 들어가기 직전에 trash_origin 에
    // 기록해둘 위치를 찾는 데 쓴다. 바탕화면(desktop)도 여기서 함께 봐야 하므로
    // (아이콘 위치 배열은 DesktopScene 이 따로 들고 있지만 "바탕화면에 있다"는
    // 사실 자체는 fs.desktop 만으로 충분히 알 수 있다) FileSystem 안에 둔다.
    pub fn locate(&self, id: FileId) -> Option<FileOrigin> {
        if self.desktop.contains(&id) {
            return Some(FileOrigin::Desktop);
        }
        if self.downloads.contains(&id) {
            return Some(FileOrigin::Downloads);
        }
        for (i, node) in self.nodes.iter().enumerate() {
            if let FileKind::Folder { children } = &node.kind
                && children.contains(&id)
            {
                return Some(FileOrigin::Folder(i));
            }
        }
        None
    }

    // 이 파일이 지금 휴지통(이름이 정확히 "Recycle Bin"인 Folder) 의 children 안에
    // 있는지 — File Explorer 의 Videos/Images 탭은 실제 컨테이너 소속과 무관하게
    // "이 종류(FileKind::Img/Mp4)인 파일 전부" 를 훑어서(all_of_kind) 보여주는
    // 가상 탭이라, 사진/동영상을 휴지통에 버려도 그 자체로는 목록에서 안 빠진다
    // (휴지통 폴더의 children 에 추가될 뿐, FileKind 는 그대로 Img/Mp4). explorer_tabs()
    // 가 Videos/Images 목록을 만들 때 이 함수로 휴지통에 들어간 항목을 걸러내서,
    // "휴지통 안에도 있고 Images 탭에도 그대로 보이는" 것처럼 보이는 문제를 막는다.
    pub fn in_recycle_bin(&self, id: FileId) -> bool {
        self.nodes.iter().any(|n| match &n.kind {
            FileKind::Folder { children } => n.name == RECYCLE_BIN_NAME && children.contains(&id),
            _ => false,
        })
    }
}

// ================= 공유 설정값 (구 settings.rs) =================
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
        "Videos" => std::borrow::Cow::Borrowed(t(lang, s::CAT_VIDEOS)),
        "Images" => std::borrow::Cow::Borrowed(t(lang, s::CAT_IMAGES)),
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

// ================= 저장/불러오기 (구 save.rs) =================
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

