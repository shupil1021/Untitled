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
    Lock { password: String, children: Vec<FileId> },  // 잠금(풀리면 폴더로)
    Folder { children: Vec<FileId> },                  // 일반 폴더 (잠금 풀리면 이걸로 변함)
    // #[serde(rename)] 로 저장 파일의 JSON 태그는 예전 이름("Email") 그대로 유지한다
    // — Rust 쪽 이름만 Mail 로 바꿔서 예전 저장 파일도 계속 불러와진다.
    #[serde(rename = "Email")]
    Mail { attachment: Option<FileId> },                // 메일 앱 (첨부파일 하나까지)
    Explorer,                                           // 바탕화면의 File Explorer (탭 있는 탐색기)
    Tar,                                                 // .tar 압축파일 — FileSystem::hex_tool_installed 가 true 여야 실제로 열어볼 수 있다
    Installer,                                           // Tar 를 열 수 있게 해주는 프로그램의 설치 마법사(.exe)
    HexTool,                                             // Installer 를 끝까지 마치면 바탕화면에 생기는 설치된 프로그램 아이콘
    PhotoGallery,                                        // 바탕화면의 Photos 앱 — assets/photo/ 사진들을 피드로 훑어보고 다운로드
    Photo(String),                                       // Photos 앱에서 다운로드한 사진 한 장 — assets/photo/ 안의 파일명
    Deleted,                                             // FileSystem::delete_permanently() 로 지워진 자리 — 그 무엇에서도 더는 참조되지 않는다
}

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
    // #[serde(rename)] 로 저장 파일의 JSON 키는 예전 이름 그대로 유지한다 — Rust
    // 쪽 필드 이름만 email_* 에서 mail_* 로 바꿔서 예전 저장 파일도 계속 불러와진다.
    #[serde(rename = "email_arrived")]
    pub mail_arrived: bool,     // 첫 메일이 도착했는지 — 도착 전엔 받은편지함이 빈 상태
    pub hex_tool_installed: bool, // HexTool Setup.exe 설치 마법사를 끝까지 마쳤는지 — 이게 true 여야 .tar 를 열 수 있다
    // 읽은 메일의 인덱스(MailApp::seed_messages 순번) — MailApp 자체는 창을 닫거나
    // 3초 주기 새로고침으로 새로 만들어질 때마다 통째로 새 인스턴스가 되므로, 읽음
    // 여부를 여기(저장 파일에 실리는 fs)에 둬야 새로고침은 물론 게임을 종료했다
    // 재시작해도 유지된다. #[serde(default)] 는 이 필드가 없던 예전 저장 파일도
    // (그냥 다 안 읽은 것으로) 계속 불러올 수 있게 해준다.
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
    // 입사 안내 메일(seed_messages)이 첨부로 거는 실제 FileKind::HexTool 노드의
    // id — 메일 쪽(apps/mail.rs::seed_messages)은 fs 를 직접 들고 있지 않아서
    // 첨부에 쓸 FileId 를 스스로 만들 수 없다. 그래서 FileSystem::new() 가 미리
    // 하나 만들어서 이 필드에 박아두고, apps/mod.rs::open() 의 FileKind::Mail
    // 분기가 매번 이 값을 MailApp::new() 로 그대로 넘겨준다.
    #[serde(default)]
    pub mail_hextool_attachment: FileId,
    // ?????(Photos) 피드에 지금 떠 있는 사진들의 식별자(assets/photo/ 기준
    // "폴더명/파일명") — 예전엔 앱을 열 때마다 통째로 다시 랜덤 셔플했는데,
    // 이제 한 번 정해지면 photos.rs::refresh_photos_feed() 가 불릴 때까지
    // (재연구 업무 보고 메일을 보내야 한다) 그대로 유지된다. #[serde(default)]
    // 라 이 필드가 없던 예전 저장 파일을 불러오면 빈 상태로 시작하고,
    // DesktopScene::new() 가 즉시 ensure_photos_selected() 로 채워준다.
    #[serde(default)]
    pub photos_current: Vec<String>,
    // photos_current 에 지금까지 한 번이라도 들어갔던 식별자 전부 — 다음 갱신 때
    // 이미 봤던 사진이 또 나오지 않도록 제외하는 데 쓴다.
    #[serde(default)]
    pub photos_seen: Vec<String>,
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

impl FileSystem {
    pub fn new() -> FileSystem {
        let mut fs = FileSystem {
            nodes: Vec::new(),
            desktop: Vec::new(),
            downloads: Vec::new(),
            ever_downloaded: Vec::new(),
            // STORY.md 프롤로그대로 새 게임을 시작하면 받은편지함이 빈 상태로
            // 시작하고, desktop.rs 의 MAIL_AUTO_ARRIVE 타이머가 MAIL_ARRIVAL_DELAY
            // 초 뒤에 입사 안내 메일을 도착시킨다.
            mail_arrived: false,
            hex_tool_installed: false,
            mail_read: Vec::new(),
            sent_mail: Vec::new(),
            trash_origin: Vec::new(),
            mail_hextool_attachment: 0, // 아래에서 실제 노드를 만들고 바로 채운다
            photos_current: Vec::new(), // DesktopScene::new() 가 ensure_photos_selected() 로 채운다
            photos_seen: Vec::new(),
        };

        // 바탕화면엔 고정 아이콘 두 개만 둔다 — 나머지 예제 파일들은 다 치웠다.
        let explorer = fs.add("My Computer", FileKind::Explorer);
        // Photos.tar/Photos.lock/HexTool Setup.exe 로 이어지던 첫 챕터용 플레이스홀더
        // 사진 콘텐츠(photo01/02.jpg)를 걷어냈다 — 실제 검수 로직 없이 그냥 열어볼
        // 수 있는 사진 두 장뿐이던 임시 내용이라, 진짜 Chapter 1 콘텐츠로 다시 채울
        // 예정. 이 콘텐츠를 그리던 앱(installer.rs/archive.rs/hextool.rs/
        // image_viewer.rs)과 관련 FileKind(Installer/Tar/Lock/Img) 자체는 나중에
        // 다른 콘텐츠로 재사용할 수 있게 그대로 남겨뒀다 — 지금은 그냥 아무 데서도
        // 안 만들어질 뿐이다.
        let mail = fs.add("Mail", FileKind::Mail { attachment: None });
        // Mail 바로 아래(fs.desktop 에서 mail 다음 순번 = 같은 열의 바로 아랫칸,
        // desktop.rs::grid_pos 가 열 우선으로 채운다) 사진 피드 앱. 이름은 읽을
        // 수 있는 글자를 하나도 안 섞고 전부 폰트 아틀라스에 없는 문자(키릴
        // 문자)로만 채웠다 — gfx.rs::Renderer::draw_tofu() 가 전부
        // "마름모+물음표"(두부, tofu) 자리표시자로 그려준다. 자모를 섞어봤던
        // 이전 시도는 여전히 다 읽히는 글자라 부족하다는 피드백이 왔었다.
        // display_name() 에 이 이름을 위한 번역 항목이 없어서 어떤 언어
        // 설정이든 이 원문 그대로 나온다.
        let photos = fs.add(crate::secrets::PHOTOS_APP_NAME, FileKind::PhotoGallery);

        // 휴지통도 그냥 이름이 "Recycle Bin"인 빈 Folder — 드래그로 파일을 옮기면
        // desktop_folder_drop_target_at 이 다른 폴더와 똑같이 인식하고, 더블클릭하면
        // apps/mod.rs 의 Explorer 열기 경로도 그대로 탄다. 아이콘만 icon_of() 에서
        // 이름으로 특수 취급(비었으면 RecycleEmpty, 아니면 RecycleFull).
        let recycle_bin = fs.add("Recycle Bin", FileKind::Folder { children: vec![] });

        fs.desktop = vec![recycle_bin, explorer, mail, photos];

        // 입사 안내 메일이 첨부로 거는 HexTool 설치 마법사 — 바탕화면/Downloads
        // 어디에도 아직 안 걸려있는, 오직 메일 첨부용으로만 미리 만들어두는 실제
        // 노드. 다운로드해야 비로소 Downloads 탭에 나타나고, 그걸 열면(FileKind::
        // Installer) installer.rs 의 마법사가 뜬다 — Finish 까지 마쳐야 바탕화면에
        // 진짜 HexTool 이 생긴다(완성된 프로그램을 곧장 쥐여주던 이전 방식 대신,
        // 이제 원래 있던 설치 마법사 흐름을 그대로 탄다).
        fs.mail_hextool_attachment = fs.add("HexTool Setup.exe", FileKind::Installer);
        fs
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

    // Photos 앱에서 사진을 "다운로드"하면 부른다 — 같은 파일명으로 이미 만들어둔
    // FileKind::Photo 노드가 있으면 그걸 그대로 재사용하고(같은 사진을 두 번
    // 다운로드해도 Downloads 탭에 중복으로 안 쌓임), 없으면 새로 만든다.
    pub fn find_or_add_photo(&mut self, filename: &str) -> FileId {
        if let Some(id) = (0..self.nodes.len()).find(|&i| matches!(&self.nodes[i].kind, FileKind::Photo(f) if f == filename)) {
            return id;
        }
        // filename 은 assets/photo 하위 폴더(corpseImage 등)까지 포함한 식별자라
        // "corpseImage/corpseImage1.jpg" 형태일 수 있다 — Explorer/Downloads
        // 탭에 보여줄 이름(name)은 그 마지막 조각(파일명)만 쓴다.
        let display = filename.rsplit('/').next().unwrap_or(filename);
        self.add(display, FileKind::Photo(filename.to_string()))
    }

    // 폴더 위치와 상관없이 전체에서 조건에 맞는 파일들을 찾는다 — File Explorer 의
    // Videos/Images 탭처럼 "어디 있든 이 종류인 파일 전부" 를 보여줄 때 쓴다.
    pub fn all_of_kind(&self, pred: impl Fn(&FileKind) -> bool) -> Vec<FileId> {
        (0..self.nodes.len()).filter(|&i| pred(&self.nodes[i].kind)).collect()
    }

    // 메일 첨부파일 등을 "다운로드" — Downloads 탭에 추가한다(이미 있으면 무시).
    pub fn download(&mut self, id: FileId) {
        if !self.downloads.contains(&id) {
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

    // 파일을 영구히 지운다 — HexTool 로 검토를 끝낸 .tar 를 없앨 때 쓴다. 인덱스
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
            FileKind::Folder { children } => n.name == "Recycle Bin" && children.contains(&id),
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
pub const BG_COLORS: [(&str, [f32; 4]); 5] = [
    ("Teal", [0.0, 0.5, 0.5, 1.0]),
    ("Navy", [0.0, 0.0, 0.5, 1.0]),
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
        "My Computer" => std::borrow::Cow::Borrowed(t(lang, s::MY_COMPUTER)),
        "Recycle Bin" => std::borrow::Cow::Borrowed(t(lang, s::RECYCLE_BIN)),
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

// fs 필드가 FileSystem 전체(노드 배열/desktop/downloads/ever_downloaded/mail_arrived/
// hex_tool_installed 전부)를 그대로 담으므로, 이름으로 다시 찾아 재구성해야 했던
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

