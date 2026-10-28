//! 게임 설치 연출 — 메일로 받은 게임(test)의 설치 마법사가 끝나면, **게임 안 가짜 컴퓨터**의
//! File Explorer 에 `AppData\<게임 이름>\` 폴더가 생기고 그 안에 게임 파일처럼 보이는
//! 파일들(소스 코드, 이미지, 사운드, 설정, 로그, 크랙 NFO)이 들어간다. 진짜 컴퓨터의 디스크에는
//! 아무것도 쓰지 않는다 — 전부 `FileSystem`(저장 파일에 실리는 가짜 파일시스템) 안의 노드다.
//! 플레이어가 게임 안에서 열어볼 수 있다: 글 파일은 Notepad, 이미지는 이미지 뷰어,
//! 사운드는 사운드 플레이어(apps/sound_player.rs). 게임 자체는 이 파일들을 읽지 않는다 —
//! 이야기 단서를 숨겨둔 연출용 소품이다(ARG). 이미지/사운드는 이진 에셋을 따로 두지 않고
//! 코드로 만들어낸다(이미지는 시작할 때 Assets 가, 사운드는 열 때 플레이어가 만든다).

use crate::foundation::{FileId, FileKind, FileSystem, MiniKind, GAME_FOLDER_NAME};

mod images;
mod sounds;
mod texts;

pub use images::{generated_images, FLOWER_IMAGE, FLOWER_IMAGE_NAME, REWARD_IMAGE_BASE};
pub use sounds::{sound_wav, wav_duration, SOUND_RATE, flush_wav, quest_start_wav};
use texts::{GAME_INI, INSTALL_LOG, NFO, README, SRC_DOORS, SRC_MAIN, SRC_MAZE};

// 파일 하나의 내용 종류. Image/Sound 의 숫자는 IMAGE_NAMES/SOUND_NAMES 의 순서와 같다.
enum Entry {
    Text(&'static str),
    Image(usize),
    Sound(usize),
}

pub const IMAGE_NAMES: [&str; 3] = ["door.png", "flower_pot.png", "static.png"];
pub const SOUND_NAMES: [&str; 2] = ["ambience.wav", "door_creak.wav"];

// 게임 폴더 안의 모든 파일 — (폴더 경로/파일 이름, 내용).
const FILES: &[(&str, Entry)] = &[
    ("README.txt", Entry::Text(README)),
    ("test-CRACKED.nfo", Entry::Text(NFO)),
    ("config/game.ini", Entry::Text(GAME_INI)),
    ("logs/install.log", Entry::Text(INSTALL_LOG)),
    ("src/main.rs", Entry::Text(SRC_MAIN)),
    ("src/doors.rs", Entry::Text(SRC_DOORS)),
    ("src/maze.rs", Entry::Text(SRC_MAZE)),
    ("assets/door.png", Entry::Image(0)),
    ("assets/flower_pot.png", Entry::Image(1)),
    ("assets/static.png", Entry::Image(2)),
    ("assets/sound/ambience.wav", Entry::Sound(0)),
    ("assets/sound/door_creak.wav", Entry::Sound(1)),
];
// 파일이 하나도 없는 빈 폴더.
const EMPTY_DIRS: &[&str] = &["saves"];

// 설치 마법사가 끝났을 때 부른다 — AppData 폴더 밑에 게임 폴더와 파일들을 만든다. 이미
// 만들어져 있으면(다시 불러온 저장 등) 아무것도 안 한다.
pub fn install_into(fs: &mut FileSystem) {
    let appdata = fs.ensure_appdata();
    if child_named(fs, appdata, GAME_FOLDER_NAME).is_some() {
        return;
    }
    let root = new_folder(fs, appdata, GAME_FOLDER_NAME);
    for dir in EMPTY_DIRS {
        folder_at(fs, root, dir);
    }
    for (path, entry) in FILES {
        let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
        let parent = folder_at(fs, root, dir);
        let kind = match entry {
            // 소스에는 \r\n 으로 적어뒀지만(윈도 메모장 느낌) 게임 안 Notepad 는 \n 만 쓴다.
            Entry::Text(text) => FileKind::Txt(text.replace("\r\n", "\n")),
            Entry::Image(i) => FileKind::Img(*i),
            Entry::Sound(i) => FileKind::Sound(*i),
        };
        let id = fs.add(name, kind);
        fs.add_to_folder(parent, id);
    }
}

fn child_named(fs: &FileSystem, folder: FileId, name: &str) -> Option<FileId> {
    match &fs.get(folder).kind {
        FileKind::Folder { children } => children.iter().copied().find(|&c| fs.get(c).name == name),
        _ => None,
    }
}

fn new_folder(fs: &mut FileSystem, parent: FileId, name: &str) -> FileId {
    let id = fs.add(name, FileKind::Folder { children: Vec::new() });
    fs.add_to_folder(parent, id);
    id
}

// root 아래 "a/b/c" 경로의 폴더를 (없는 마디는 만들면서) 찾아 돌려준다. 빈 경로면 root.
fn folder_at(fs: &mut FileSystem, root: FileId, path: &str) -> FileId {
    path.split('/').filter(|s| !s.is_empty()).fold(root, |cur, seg| match child_named(fs, cur, seg) {
        Some(id) => id,
        None => new_folder(fs, cur, seg),
    })
}

// 설치한 미니게임의 폴더 — AppData\<게임 이름>\ 에 설정 파일과 안내문이 들어간다(연출용).
pub fn install_mini(fs: &mut FileSystem, kind: MiniKind) {
    let appdata = fs.ensure_appdata();
    if child_named(fs, appdata, kind.app_name()).is_some() {
        return;
    }
    let root = new_folder(fs, appdata, kind.app_name());
    let (readme, cfg): (&str, &str) = match kind {
        MiniKind::Eye => (
            "SHY_EYE\n\n- WASD / arrow keys: move\n- 3 lives, survive 30 seconds\n- No attacks. Just don't be seen.\n",
            "[shy_eye]\nlives=3\nduration=30\ninvuln_after_hit=1.0\nbackground_eye=closed_when_cleared\n",
        ),
        MiniKind::Brain => (
            "BRAIN_DRIVE\n\n- A / D / arrows: change lane\n- S / down: brake\n- 1000m in 60 seconds, 3 crashes and you stop\n",
            "[brain_drive]\nlanes=3\ngoal_m=1000\nlimit_s=60\nspeed=20\nbrake_speed=10\nsign_destination=home\n",
        ),
        MiniKind::Finger => (
            "PACK\n\n- Space / button: pack the box when it touches the line\n- 12 boxes, 8 or more to pass\n- Holding the button counts once.\n",
            "[pack]\nboxes=12\npass=8\nwindow_s=0.25\npractice_rounds=3\n",
        ),
    };
    for (name, text) in [("README.txt", readme), ("config.ini", cfg)] {
        let id = fs.add(name, FileKind::Txt(text.to_string()));
        fs.add_to_folder(root, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // path("a/b/c") 로 파일을 찾는다.
    fn find(fs: &FileSystem, root: FileId, path: &str) -> Option<FileId> {
        path.split('/').try_fold(root, |cur, seg| child_named(fs, cur, seg))
    }

    #[test]
    fn install_builds_the_file_tree_inside_the_fake_fs() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        let appdata = fs.find_by_name("AppData").expect("AppData 폴더");
        let root = child_named(&fs, appdata, GAME_FOLDER_NAME).expect("게임 폴더");

        for (path, entry) in FILES {
            let id = find(&fs, root, path).unwrap_or_else(|| panic!("{path} 가 없다"));
            match (entry, &fs.get(id).kind) {
                (Entry::Text(_), FileKind::Txt(t)) => assert!(!t.contains('\r'), "{path}"),
                (Entry::Image(i), FileKind::Img(j)) | (Entry::Sound(i), FileKind::Sound(j)) => assert_eq!(i, j, "{path}"),
                _ => panic!("{path}: 종류가 안 맞는다"),
            }
        }
        assert!(matches!(&fs.get(find(&fs, root, "saves").unwrap()).kind, FileKind::Folder { children } if children.is_empty()));

        // 다시 설치해도(저장을 불러온 뒤 등) 중복으로 안 늘어난다.
        let nodes_before = fs.all_of_kind(|_| true).len();
        install_into(&mut fs);
        assert_eq!(fs.all_of_kind(|_| true).len(), nodes_before);
    }

    #[test]
    fn file_names_match_image_and_sound_tables() {
        for (path, entry) in FILES {
            let name = path.rsplit('/').next().unwrap();
            match entry {
                Entry::Image(i) => assert_eq!(IMAGE_NAMES[*i], name),
                Entry::Sound(i) => assert_eq!(SOUND_NAMES[*i], name),
                Entry::Text(_) => {}
            }
        }
    }

    #[test]
    fn generated_sounds_are_valid_wav() {
        for i in 0..SOUND_NAMES.len() {
            let bytes = sound_wav(i);
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");
            let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
            assert_eq!(bytes.len(), 44 + data_len);
            assert!(wav_duration(&bytes) > 1.0);
        }
    }

    // 미니게임 보상 사진(충혈된 눈 / 짓무른 뇌 / 절단된 손가락)은 생성 이미지 목록 안의 올바른 자리에 있다.
    #[test]
    fn reward_images_are_in_the_generated_table() {
        let images = generated_images();
        for kind in MiniKind::ALL {
            assert!(kind.image_index() < images.len(), "{kind:?}");
            assert!(images[kind.image_index()].pixels().any(|p| p.0 != [12, 12, 14, 255]), "{kind:?}: 배경만 있는 빈 그림이다");
        }
    }

    // 미니게임을 설치하면 AppData 아래 그 게임 폴더가 생기고 두 번 설치해도 늘어나지 않는다.
    #[test]
    fn installing_a_minigame_creates_its_folder_once() {
        let mut fs = FileSystem::new();
        install_mini(&mut fs, MiniKind::Brain);
        let appdata = fs.find_by_name("AppData").unwrap();
        let root = child_named(&fs, appdata, "BRAIN_DRIVE").expect("게임 폴더");
        assert!(child_named(&fs, root, "README.txt").is_some() && child_named(&fs, root, "config.ini").is_some());
        let nodes = fs.all_of_kind(|_| true).len();
        install_mini(&mut fs, MiniKind::Brain);
        assert_eq!(fs.all_of_kind(|_| true).len(), nodes);
    }
}
