//! 가짜 파일 시스템 — 파일/폴더 노드, 바탕화면/Downloads 위치, 메일함(도착 기록, 첨부), 휴지통.
//! 저장 파일이 이 구조체의 스냅샷이다(save.rs). 게임 진행 상태(DoorsProgress 등)는 progress.rs.

use serde::{Deserialize, Serialize};

use super::*;

// 모든 파일/폴더를 하나의 Vec(아레나)에 담고 FileId(인덱스)로 참조한다.
// 폴더/잠금파일은 자식들을 FileId 로 가리킨다.

pub type FileId = usize;


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
    // 미니게임(눈/뇌/손가락)의 설치 파일 — 메일 첨부로 오고, 열면 설치 마법사가 뜬다.
    MiniSetup(MiniKind),
    // 설치가 끝나 바탕화면에 생긴 미니게임 아이콘 — 열면 2D 미니게임 창이 뜬다.
    MiniGame(MiniKind),
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
    // 받은편지함에 도착한 메일들 — 도착한 순서대로(앞이 먼저). 메일 앱은 이 순서 그대로
    // 보여주고, mail_read 의 인덱스도 이 순번이다. 첫 메일은 desktop.rs 의 타이머가, 나머지는
    // 게임 안 이벤트(AppAction)가 deliver_mail 로 도착시킨다.
    #[serde(default)]
    pub mail_log: Vec<MailId>,
    // 예전 저장 파일 호환용 — mail_log 가 생기기 전엔 "첫 메일 도착"/"두 번째 메일 도착"/"그 뒤
    // 몇 통 더"를 이 값들로 기록했다. 불러온 직후 migrate_mail_log 가 mail_log 로 옮긴다.
    #[serde(default, rename = "mail_arrived")]
    legacy_mail_arrived: bool,
    // 읽은 메일의 인덱스(MailApp::seed_messages 순번) — MailApp 자체는 창을 닫거나
    // 3초 주기 새로고침으로 새로 만들어질 때마다 통째로 새 인스턴스가 되므로, 읽음
    // 여부를 여기(저장 파일에 실리는 fs)에 둬야 새로고침은 물론 게임을 종료했다
    // 재시작해도 유지된다. #[serde(default)] 는 이 필드가 없던 예전 저장 파일도
    // (그냥 다 안 읽은 것으로) 계속 불러올 수 있게 해준다.
    #[serde(default, rename = "mail2_arrived")]
    legacy_mail2_arrived: bool,
    #[serde(default, rename = "extra_mails")]
    legacy_extra_mails: usize,
    // 창 안 게임들의 진행 상황 — 위 DoorsProgress/MazeProgress 참고. 이 필드가 없던 예전 저장
    // 파일은 처음부터 시작으로 불러온다.
    #[serde(default)]
    pub doors_progress: DoorsProgress,
    #[serde(default)]
    pub maze_progress: MazeProgress,
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
    // 미니게임 셋(눈/뇌/손가락)의 진행, 메일로 오는 설치 파일 노드, 설치 여부, 보상 사진 노드 — 인덱스는 MiniKind::index().
    #[serde(default)]
    pub minis: [MiniProgress; 3],
    #[serde(default)]
    pub mini_setup: [Option<FileId>; 3],
    #[serde(default)]
    pub mini_installed: [bool; 3],
    #[serde(default)]
    pub reward_image: [Option<FileId>; 3],
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
            mail_log: Vec::new(),
            legacy_mail_arrived: false,
            legacy_mail2_arrived: false,
            legacy_extra_mails: 0,
            doors_progress: DoorsProgress::default(),
            maze_progress: MazeProgress::default(),
            seed_planted: false,
            flower_image: None,
            mail_read: Vec::new(),
            sent_mail: Vec::new(),
            trash_origin: Vec::new(),
            game_installed: false,
            sub_game_ready: false,
            minis: [MiniProgress::default(); 3],
            mini_setup: [None; 3],
            mini_installed: [false; 3],
            reward_image: [None; 3],
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

    // 메일을 받은편지함에 도착시킨다 — 이미 도착했으면 아무것도 안 하고 false(중복 도착 방지).
    pub fn deliver_mail(&mut self, id: MailId) -> bool {
        if self.mail_log.contains(&id) {
            return false;
        }
        self.prepare_mail_assets(id);
        self.mail_log.push(id);
        true
    }

    // 메일에 붙어 올 첨부 노드(설치 파일 / 보상 사진)를 (없으면) 만들어 둔다.
    fn prepare_mail_assets(&mut self, id: MailId) {
        for kind in MiniKind::ALL {
            let i = kind.index();
            if id == kind.task_mail() && self.mini_setup[i].is_none() {
                self.mini_setup[i] = Some(self.add(kind.setup_name(), FileKind::MiniSetup(kind)));
            }
            if id == kind.reward_mail() && self.reward_image[i].is_none() {
                self.reward_image[i] = Some(self.add(kind.image_name(), FileKind::Img(kind.image_index())));
            }
        }
        if id == MailId::Photo {
            self.ensure_flower_image();
        }
    }

    // 그 미니게임의 보상 사진을 메일에서 내려받았는지 — 메인 게임이 재료를 받는 조건이다.
    pub fn reward_downloaded(&self, kind: MiniKind) -> bool {
        self.reward_image[kind.index()].is_some_and(|id| self.ever_downloaded.contains(&id))
    }

    pub fn has_mail(&self, id: MailId) -> bool {
        self.mail_log.contains(&id)
    }

    // 그 메일을 읽었는지(받은편지함에서 눌러 mail_read 에 기록됐는지).
    pub fn is_mail_read(&self, id: MailId) -> bool {
        self.mail_log.iter().position(|&m| m == id).is_some_and(|i| self.mail_read.contains(&i))
    }

    // 예전 저장 파일(mail_log 가 없던 때)의 도착 기록을 mail_log 로 옮긴다 — 불러온 직후 한 번.
    // 예전 순서는 항상 친구 메일 → 꽃 위치 메일 → 힌트 → 꽃 사진이라 읽음 인덱스도 그대로 맞는다.
    pub fn migrate_mail_log(&mut self) {
        if !self.mail_log.is_empty() {
            return;
        }
        if self.legacy_mail_arrived {
            self.mail_log.push(MailId::Friend);
        }
        if self.legacy_mail2_arrived {
            self.mail_log.push(MailId::Flower);
        }
        if self.legacy_extra_mails >= 1 {
            self.mail_log.push(MailId::Hint);
        }
        if self.legacy_extra_mails >= 2 {
            self.mail_log.push(MailId::Photo);
        }
        self.legacy_mail_arrived = false;
        self.legacy_mail2_arrived = false;
        self.legacy_extra_mails = 0;
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

    // 받은편지함 메시지(mail_log 순서)마다의 첨부 노드 — 친구 메일엔 게임 설치 파일, 꽃 사진
    // 메일엔 flower.png, 나머지는 없다.
    pub fn mail_attachments(&self) -> Vec<Option<FileId>> {
        self.mail_log
            .iter()
            .map(|id| match id {
                MailId::Friend => self.mail_attachment(),
                MailId::Photo => self.flower_image,
                MailId::EyeTask => self.mini_setup[0],
                MailId::BrainTask => self.mini_setup[1],
                MailId::FingerTask => self.mini_setup[2],
                MailId::EyeReward => self.reward_image[0],
                MailId::BrainReward => self.reward_image[1],
                MailId::FingerReward => self.reward_image[2],
                _ => None,
            })
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    // mail_log 가 생기기 전 저장 파일(도착 여부를 bool/개수로 기록)이 같은 순서의 mail_log 로 옮겨지고,
    // 읽음 인덱스도 그대로 맞는다.
    #[test]
    fn legacy_mail_flags_migrate_to_the_log() {
        let mut fs = FileSystem::new();
        fs.legacy_mail_arrived = true;
        fs.legacy_mail2_arrived = true;
        fs.legacy_extra_mails = 2;
        fs.mail_read = vec![1];
        fs.migrate_mail_log();
        assert_eq!(fs.mail_log, [MailId::Friend, MailId::Flower, MailId::Hint, MailId::Photo]);
        assert!(fs.is_mail_read(MailId::Flower) && !fs.is_mail_read(MailId::Friend));
        fs.migrate_mail_log(); // 한 번 옮긴 뒤엔 다시 안 건드린다
        assert_eq!(fs.mail_log.len(), 4);
    }

    // 게임 진행 상황이 저장 파일(JSON)을 거쳐도 그대로 돌아오고, 이 필드가 없던 예전 저장 파일은
    // 처음부터 시작으로 불러와진다.
    #[test]
    fn game_progress_survives_save_roundtrip_and_old_saves_load() {
        let mut fs = FileSystem::new();
        fs.doors_progress.quest_accepted = true;
        fs.doors_progress.has_letter = true;
        fs.maze_progress.has_can = true;
        fs.maze_progress.keys = [true, false, true];
        fs.maze_progress.planted_at = Some(1_759_708_800.5);
        let json = serde_json::to_string(&fs).unwrap();
        let back: FileSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(back.doors_progress, fs.doors_progress);
        assert_eq!(back.maze_progress, fs.maze_progress);

        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = value.as_object_mut().unwrap();
        obj.remove("doors_progress");
        obj.remove("maze_progress");
        let old: FileSystem = serde_json::from_value(value).unwrap();
        assert_eq!(old.doors_progress, DoorsProgress::default());
        assert_eq!(old.maze_progress, MazeProgress::default());
    }

    #[test]
    fn deliver_mail_is_idempotent_and_keeps_arrival_order() {
        let mut fs = FileSystem::new();
        assert!(fs.deliver_mail(MailId::Friend));
        assert!(fs.deliver_mail(MailId::Letter));
        assert!(!fs.deliver_mail(MailId::Letter));
        assert!(fs.deliver_mail(MailId::Flower));
        assert_eq!(fs.mail_log, [MailId::Friend, MailId::Letter, MailId::Flower]);
        assert_eq!(fs.mail_attachments().len(), 3);
    }

    // 미니게임 메일은 도착할 때 설치 파일 / 보상 사진 노드를 달고 오고, 사진을 내려받으면 그 게임의 보상이 "받아진" 것으로 친다.
    #[test]
    fn minigame_mails_carry_their_files() {
        let mut fs = FileSystem::new();
        fs.deliver_mail(MailId::EyeTask);
        let setup = fs.mini_setup[0].expect("설치 파일 노드");
        assert!(matches!(fs.get(setup).kind, FileKind::MiniSetup(MiniKind::Eye)));
        assert_eq!(fs.get(setup).name, "SHY_EYE Setup.exe");
        assert_eq!(fs.mail_attachments(), [Some(setup)]);
        assert!(!fs.reward_downloaded(MiniKind::Eye));

        fs.deliver_mail(MailId::EyeReward);
        let image = fs.reward_image[0].expect("보상 사진 노드");
        assert!(matches!(fs.get(image).kind, FileKind::Img(i) if i == MiniKind::Eye.image_index()));
        assert_eq!(fs.get(image).name, "bloodshot_eye.png");
        assert!(!fs.reward_downloaded(MiniKind::Eye), "메일이 와도 내려받기 전엔 안 받은 것");
        fs.download(image);
        assert!(fs.reward_downloaded(MiniKind::Eye) && !fs.reward_downloaded(MiniKind::Brain));

        // 중복 도착은 노드를 또 만들지 않는다.
        let nodes = fs.all_of_kind(|_| true).len();
        assert!(!fs.deliver_mail(MailId::EyeReward));
        assert_eq!(fs.all_of_kind(|_| true).len(), nodes);
    }
}
