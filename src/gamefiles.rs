//! 게임 설치 연출 — 메일로 받은 게임(test)의 설치 마법사가 끝나면, **게임 안 가짜 컴퓨터**의
//! File Explorer 에 `AppData\<게임 이름>\` 폴더가 생기고 그 안에 게임 파일처럼 보이는
//! 파일들(소스 코드, 이미지, 사운드, 설정, 로그, 크랙 NFO)이 들어간다. 진짜 컴퓨터의 디스크에는
//! 아무것도 쓰지 않는다 — 전부 `FileSystem`(저장 파일에 실리는 가짜 파일시스템) 안의 노드다.
//! 플레이어가 게임 안에서 열어볼 수 있다: 글 파일은 Notepad, 이미지는 이미지 뷰어,
//! 사운드는 사운드 플레이어(apps/sound_player.rs). 게임 자체는 이 파일들을 읽지 않는다 —
//! 이야기 단서를 숨겨둔 연출용 소품이다(ARG). 이미지/사운드는 이진 에셋을 따로 두지 않고
//! 코드로 만들어낸다(이미지는 시작할 때 Assets 가, 사운드는 열 때 플레이어가 만든다).

use crate::foundation::{FileId, FileKind, FileSystem, GAME_FOLDER_NAME};
use crate::random::Rng;

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

// ================= 글 파일들(소스 코드/설정/로그/NFO) =================

const README: &str = "test - cracked edition\r\n\
\r\n\
1. Run test.exe.\r\n\
2. Do not update the game. The updater phones home.\r\n\
3. If a door talks to you, that is normal. Answer it.\r\n\
\r\n\
Known issues\r\n\
  - The room has no flowers. We checked. Twice.\r\n\
  - Some mail comes from senders whose names do not display correctly.\r\n\
\r\n\
Do not delete the saves folder.\r\n";

const NFO: &str = "\
 ________________________________________________\r\n\
|                                                |\r\n\
|   R 1 8   p r e s e n t s                      |\r\n\
|                                                |\r\n\
|   test  (cracked)                              |\r\n\
|   ----------------------------------------     |\r\n\
|   protection ...... removed                    |\r\n\
|   release type .... cracked                    |\r\n\
|   size ............ small, for what it does    |\r\n\
|                                                |\r\n\
|   notes:                                       |\r\n\
|     the original had a license check on the    |\r\n\
|     door. we cut it out. the door did not      |\r\n\
|     like that. we do not know why it minds.    |\r\n\
|________________________________________________|\r\n";

const GAME_INI: &str = "\
[video]\r\n\
width=640\r\n\
height=480\r\n\
fov=56\r\n\
\r\n\
[game]\r\n\
room=0\r\n\
mouse_sens=0.0032\r\n\
door_mood=lonely\r\n\
flowers_in_room=0\r\n\
\r\n\
[network]\r\n\
mail_hook=enabled\r\n\
sender_name=<corrupted>\r\n";

const INSTALL_LOG: &str = "\
[ok]   copied src/main.rs\r\n\
[ok]   copied src/doors.rs\r\n\
[ok]   copied src/maze.rs\r\n\
[ok]   copied assets/door.png\r\n\
[ok]   copied assets/flower_pot.png\r\n\
[ok]   copied assets/static.png\r\n\
[ok]   copied assets/sound/ambience.wav\r\n\
[ok]   copied assets/sound/door_creak.wav\r\n\
[ok]   wrote config/game.ini\r\n\
[warn] license check skipped (cracked)\r\n\
[warn] door: unexpected listener on room 0\r\n\
[ok]   install complete\r\n";

const SRC_MAIN: &str = "\
// test -- main loop (decompiled, names recovered where possible)\r\n\
mod doors;\r\n\
mod maze;\r\n\
\r\n\
fn main() {\r\n\
    let mut room = Room::load(0);\r\n\
    loop {\r\n\
        room.update();\r\n\
        if room.player_leaves() {\r\n\
            // never reached in the retail build.\r\n\
            break;\r\n\
        }\r\n\
    }\r\n\
}\r\n";

const SRC_DOORS: &str = "\
// doors.rs -- every door has its own personality.\r\n\
// the game is cleared when ALL doors are satisfied.\r\n\
\r\n\
pub struct Door {\r\n\
    pub request: &'static str,\r\n\
    pub accepted: bool,\r\n\
    pub satisfied: bool,\r\n\
}\r\n\
\r\n\
pub fn room_zero_door() -> Door {\r\n\
    Door {\r\n\
        request: \"give me a borage flower\",\r\n\
        accepted: false,\r\n\
        satisfied: false,\r\n\
    }\r\n\
}\r\n\
\r\n\
impl Door {\r\n\
    pub fn talk(&mut self, accept: bool) {\r\n\
        // TODO: declining should do something. it does not. yet.\r\n\
        self.accepted |= accept;\r\n\
    }\r\n\
\r\n\
    pub fn give_flower(&mut self) {\r\n\
        // the flower is not in this room.\r\n\
        // see maze.rs for where to look.\r\n\
        self.satisfied = true;\r\n\
    }\r\n\
}\r\n";

const SRC_MAZE: &str = "\
// maze.rs -- the other game, hidden inside this one.\r\n\
// 3 keys. 3 doors. a rope comes down once you have the key.\r\n\
\r\n\
const KEYS_NEEDED: usize = 3;\r\n\
\r\n\
// a seed grows if you give it time.\r\n\
// the clock on the desktop is not a suggestion.\r\n\
pub fn plant(seed: Seed, pot: &mut Pot) {\r\n\
    pot.seed = Some(seed);\r\n\
    pot.needs_years = 1;\r\n\
}\r\n";

// ================= 코드로 만드는 이미지/사운드 =================

// IMAGE_NAMES 순서대로 — Assets::load 가 이 순서로 photos 에 올리므로 FileKind::Img(i) 의
// i 가 그대로 이 목록의 인덱스다.
pub fn generated_images() -> [image::RgbaImage; 3] {
    [door_image(), flower_pot_image(), static_image()]
}

fn door_image() -> image::RgbaImage {
    let (w, h) = (64u32, 96u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        let frame = x < 3 || x >= w - 3 || y < 3 || y >= h - 3;
        let panel = (10..w - 10).contains(&x) && ((10..40).contains(&y) || (50..86).contains(&y));
        let handle = (46..52).contains(&x) && (46..52).contains(&y);
        let c = if handle {
            [204, 184, 115]
        } else if frame {
            [60, 38, 24]
        } else if panel {
            [104, 70, 45]
        } else {
            [89, 61, 41]
        };
        image::Rgba([c[0], c[1], c[2], 255])
    })
}

fn flower_pot_image() -> image::RgbaImage {
    let (w, h) = (48u32, 48u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        // 위가 넓은 사다리꼴 화분 + 맨 위 흙. 꽃도 줄기도 없다.
        let (left, right) = (10 + (y.saturating_sub(18)) / 3, 38 - (y.saturating_sub(18)) / 3);
        let in_pot = (18..44).contains(&y) && (left..right).contains(&x);
        let soil = (16..20).contains(&y) && (11..37).contains(&x);
        if soil {
            image::Rgba([48, 34, 24, 255])
        } else if in_pot {
            image::Rgba([154, 89, 51, 255])
        } else {
            image::Rgba([12, 12, 14, 255])
        }
    })
}

fn static_image() -> image::RgbaImage {
    let mut rng = Rng::new(0x5EED);
    image::RgbaImage::from_fn(64, 64, |_, _| {
        let v = (rng.unit() * 255.0) as u8;
        image::Rgba([v, v, v, 255])
    })
}

pub const SOUND_RATE: u32 = 22050;

// SOUND_NAMES[i] 의 WAV 파일 바이트(16비트 모노 PCM) — 사운드 플레이어가 열 때 만든다.
pub fn sound_wav(i: usize) -> Vec<u8> {
    match i {
        0 => ambience_wav(),
        _ => creak_wav(),
    }
}

// WAV 바이트의 재생 시간(초).
pub fn wav_duration(wav: &[u8]) -> f32 {
    (wav.len().saturating_sub(44) / 2) as f32 / SOUND_RATE as f32
}

fn wav_bytes(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt 청크 크기
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // 모노
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // 초당 바이트
    out.extend_from_slice(&2u16.to_le_bytes()); // 블록 크기
    out.extend_from_slice(&16u16.to_le_bytes()); // 비트 수
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

// 낮게 웅웅거리는 방 안 소리 — 55Hz 저음 + 살짝 흔들리는 5도 위 + 느리게 숨 쉬는 노이즈.
fn ambience_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xA11B);
    let secs = 6.0;
    let n = (SOUND_RATE as f32 * secs) as usize;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / SOUND_RATE as f32;
            let tau = std::f32::consts::TAU;
            let hum = 0.25 * (tau * 55.0 * t).sin() + 0.12 * (tau * 82.5 * t + 0.8 * (tau * 0.2 * t).sin()).sin();
            let breath = 0.5 + 0.5 * (tau * 0.3 * t).sin();
            let noise = (rng.unit() * 2.0 - 1.0) * 0.05 * breath;
            let fade = (t / 0.5).min(1.0).min(((secs - t) / 0.5).max(0.0));
            ((hum + noise) * fade * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, SOUND_RATE)
}

// 문 삐걱거리는 소리 — 높은 톱니파가 점점 낮아지며 떨린다.
fn creak_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xC0FFEE);
    let secs = 1.4;
    let n = (SOUND_RATE as f32 * secs) as usize;
    let mut phase = 0.0f32;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / SOUND_RATE as f32;
            let k = t / secs;
            let freq = 200.0 - 110.0 * k + 6.0 * (std::f32::consts::TAU * 14.0 * t).sin();
            phase = (phase + freq / SOUND_RATE as f32).fract();
            let saw = phase * 2.0 - 1.0;
            let env = (k * 12.0).min(1.0) * (1.0 - k).powf(0.7);
            ((saw * 0.35 + (rng.unit() * 2.0 - 1.0) * 0.1) * env * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, SOUND_RATE)
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
}
