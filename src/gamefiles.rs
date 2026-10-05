//! 게임 설치 연출 — 메일로 받은 게임(test)의 설치 마법사가 끝나면 진짜 컴퓨터의
//! `%APPDATA%\<게임 이름>\` 폴더에 게임 파일처럼 보이는 파일들(소스 코드, 이미지, 사운드,
//! 설정, 로그, 크랙 NFO)을 실제로 써 넣는다. 게임은 이 파일들을 읽지 않는다 — 플레이어가
//! 탐색기로 직접 열어봤을 때 "정말 설치된 크랙 게임" 같고, 그 안에 이야기 단서가 숨어
//! 있도록 하는 순전히 연출용 소품이다(ARG). 이미지/사운드는 이진 에셋을 따로 두지 않고 설치할
//! 때 코드로 만들어낸다.
//!
//! 안전: 우리가 만든 폴더라는 표식(`.palaceos`)이 있는 폴더만 덮어쓰고 지운다 — 사용자가
//! 이미 같은 이름의 폴더를 갖고 있으면 아무것도 건드리지 않는다.

use std::io;
use std::path::{Path, PathBuf};

use crate::foundation::GAME_FOLDER_NAME;
use crate::random::Rng;

const MARKER: &str = ".palaceos";

// 설치 위치 — 실제 경로(%APPDATA% = ...\AppData\Roaming) 아래 게임 이름 폴더.
pub fn install_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(GAME_FOLDER_NAME))
}

// 설치 마법사가 끝났을 때 부른다. 실패(권한, 같은 이름의 남의 폴더 등)해도 게임 진행엔
// 영향이 없으니 호출부는 결과를 무시해도 된다.
pub fn install() -> io::Result<PathBuf> {
    let dir = install_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "APPDATA 가 없다"))?;
    install_to(&dir)?;
    Ok(dir)
}

// "Erase All Memory" 때 부른다 — 우리가 만든 폴더일 때만 지운다.
pub fn remove() {
    if let Some(dir) = install_dir() {
        remove_dir(&dir);
    }
}

fn remove_dir(dir: &Path) {
    if dir.join(MARKER).exists() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

fn install_to(dir: &Path) -> io::Result<()> {
    if dir.exists() && !dir.join(MARKER).exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "같은 이름의 폴더가 이미 있다"));
    }
    for sub in ["src", "assets/sound", "config", "logs", "saves"] {
        std::fs::create_dir_all(dir.join(sub))?;
    }
    std::fs::write(dir.join(MARKER), "created by the PalaceOS installer\n")?;

    for (path, text) in TEXT_FILES {
        std::fs::write(dir.join(path), text)?;
    }
    door_image().save(dir.join("assets/door.png")).map_err(io::Error::other)?;
    flower_pot_image().save(dir.join("assets/flower_pot.png")).map_err(io::Error::other)?;
    static_image().save(dir.join("assets/static.png")).map_err(io::Error::other)?;
    std::fs::write(dir.join("assets/sound/ambience.wav"), ambience_wav())?;
    std::fs::write(dir.join("assets/sound/door_creak.wav"), creak_wav())?;
    Ok(())
}

// ================= 텍스트 파일들(소스 코드/설정/로그/NFO) =================

const TEXT_FILES: &[(&str, &str)] = &[
    ("README.txt", README),
    ("test-CRACKED.nfo", NFO),
    ("config/game.ini", GAME_INI),
    ("logs/install.log", INSTALL_LOG),
    ("src/main.rs", SRC_MAIN),
    ("src/doors.rs", SRC_DOORS),
    ("src/maze.rs", SRC_MAZE),
];

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
            image::Rgba([0, 0, 0, 0])
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

// 16비트 모노 PCM WAV 파일 바이트.
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

const RATE: u32 = 22050;

// 낮게 웅웅거리는 방 안 소리 — 55Hz 저음 + 살짝 흔들리는 5도 위 + 느리게 숨 쉬는 노이즈.
fn ambience_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xA11B);
    let secs = 6.0;
    let n = (RATE as f32 * secs) as usize;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let tau = std::f32::consts::TAU;
            let hum = 0.25 * (tau * 55.0 * t).sin() + 0.12 * (tau * 82.5 * t + 0.8 * (tau * 0.2 * t).sin()).sin();
            let breath = 0.5 + 0.5 * (tau * 0.3 * t).sin();
            let noise = (rng.unit() * 2.0 - 1.0) * 0.05 * breath;
            let fade = (t / 0.5).min(1.0).min(((secs - t) / 0.5).max(0.0));
            ((hum + noise) * fade * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, RATE)
}

// 문 삐걱거리는 소리 — 높은 톱니파가 점점 낮아지며 떨린다.
fn creak_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xC0FFEE);
    let secs = 1.4;
    let n = (RATE as f32 * secs) as usize;
    let mut phase = 0.0f32;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let k = t / secs;
            let freq = 200.0 - 110.0 * k + 6.0 * (std::f32::consts::TAU * 14.0 * t).sin();
            phase = (phase + freq / RATE as f32).fract();
            let saw = phase * 2.0 - 1.0;
            let env = (k * 12.0).min(1.0) * (1.0 - k).powf(0.7);
            ((saw * 0.35 + (rng.unit() * 2.0 - 1.0) * 0.1) * env * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, RATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("palaceos_gamefiles_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn install_writes_every_file_and_remove_cleans_up() {
        let dir = temp_dir("ok");
        install_to(&dir).unwrap();
        for (path, _) in TEXT_FILES {
            assert!(dir.join(path).is_file(), "{path}");
        }
        for path in ["assets/door.png", "assets/flower_pot.png", "assets/static.png"] {
            image::open(dir.join(path)).unwrap_or_else(|e| panic!("{path}: {e}"));
        }
        for path in ["assets/sound/ambience.wav", "assets/sound/door_creak.wav"] {
            let bytes = std::fs::read(dir.join(path)).unwrap();
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(&bytes[8..12], b"WAVE");
            let data_len = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
            assert_eq!(bytes.len(), 44 + data_len, "{path}");
        }
        assert!(dir.join("saves").is_dir());
        install_to(&dir).unwrap(); // 다시 설치해도(덮어쓰기) 문제없다
        remove_dir(&dir);
        assert!(!dir.exists());
    }

    // 진짜 %APPDATA%\test 에 설치해서 눈으로 확인하고 싶을 때 손으로 돌린다(기본 테스트에선
    // 빠진다): cargo test --lib install_to_real_appdata -- --ignored --nocapture
    #[test]
    #[ignore]
    fn install_to_real_appdata() {
        let dir = install().expect("설치 실패");
        println!("installed to {}", dir.display());
    }

    #[test]
    fn foreign_folder_is_left_alone() {
        let dir = temp_dir("foreign");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mine.txt"), "important").unwrap();
        assert!(install_to(&dir).is_err());
        remove_dir(&dir); // 표식이 없으니 안 지워진다
        assert!(dir.join("mine.txt").is_file());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
