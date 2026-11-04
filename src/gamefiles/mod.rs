//! 게임 설치 연출 — 메일로 받은 게임(test)의 설치 마법사가 끝나면, **게임 안 가짜 컴퓨터**의
//! File Explorer 에 `AppData\<게임 이름>\` 폴더가 생기고 그 안에 게임 파일처럼 보이는
//! 파일들(소스 코드, 이미지, 사운드, 설정, 로그, 크랙 NFO)이 들어간다. 진짜 컴퓨터의 디스크에는
//! 아무것도 쓰지 않는다 — 전부 `FileSystem`(저장 파일에 실리는 가짜 파일시스템) 안의 노드다.
//! 플레이어가 게임 안에서 열어볼 수 있다: 글 파일은 Notepad, 이미지는 이미지 뷰어,
//! 사운드는 사운드 플레이어(apps/sound_player.rs). 게임 자체는 이 파일들을 읽지 않는다 —
//! 이야기 단서를 숨겨둔 연출용 소품이다(ARG). 이미지/사운드는 이진 에셋을 따로 두지 않고
//! 코드로 만들어낸다(이미지는 시작할 때 Assets 가, 사운드는 열 때 플레이어가 만든다).
//! 파일은 설치할 때 한 번 만들고 끝이 아니라, 진행에 따라 게임이 다시 쓴다(sync_game_files — 아래 "살아 있는 게임 파일").

use crate::foundation::{DoorsProgress, FileId, FileKind, FileSystem, MainPlace, MazeProgress, MiniKind, MiniProgress, GAME_FOLDER_NAME};

mod images;
mod sounds;
mod texts;

pub use images::{generated_images, FLOWER_IMAGE, FLOWER_IMAGE_NAME, REWARD_IMAGE_BASE};
pub use sounds::{sound_wav, wav_duration, SOUND_RATE, flush_wav, quest_start_wav};
use texts::{GAME_INI, INSTALL_LOG, NFO, README, SRC_DOORS, SRC_LEVEL2, SRC_MAIN, SRC_MAZE};

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
    // 규칙 숫자는 게임 코드의 상수에서 가져온다(apps::minigame::rules_text).
    let (readme, cfg) = crate::apps::minigame::rules_text(kind);
    for (name, text) in [("README.txt", readme), ("config.ini", cfg)] {
        let id = fs.add(name, FileKind::Txt(text));
        fs.add_to_folder(root, id);
    }
}

// ---- 살아 있는 게임 파일 ----
// 설치한 게임 폴더(AppData\test)의 파일들은 얼려 둔 소품이 아니다 — 플레이어가 진행하는 만큼 게임이 자기 파일을 다시 쓴다:
// `saves/slot0.sav` 는 지금 진행 상태를, `config/game.ini` 의 door_mood/flowers_in_room 은 문의 기분과 방의 꽃을,
// `logs/game.log` 는 일어난 일을 시각과 함께, 2층에 나가면 `src/level2.rs` 가 새로 생긴다. 게임 자체는 이 파일을 읽지
// 않는다(연출용 단서). 플레이어가 파일이나 saves 폴더를 지워도 다음에 게임이 조용히 다시 만들고, 폴더 전체(test)를 지우면
// 되살리지 않는다.

const LOG_HEADER: &str = "# game.log\n";
const LOG_MAX_LINES: usize = 80; // 로그가 저장 파일을 불리지 않게 오래된 줄부터 버린다

// 플레이어가 메모장에서 고친 `config/game.ini` 가 게임에게 어떻게 읽히는지 — 문이 이걸 보고 한마디 한다(apps/doors_game). 게임의 진행이나
// 규칙은 절대 바꾸지 않는다(가짜 꽃으로 퍼즐을 건너뛸 수 없다) — 문은 속지 않고 말만 한다.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum IniEdit {
    #[default]
    None, // 안 고쳤거나, 고쳤어도 실제와 같다
    Mood(IniMood), // door_mood 에 실제와 다른 (아는) 기분을 적었다
    UnknownMood,   // door_mood 에 모르는 말을 적었다
    FakeFlowers,   // flowers_in_room 을 실제보다 크게 적었다
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniMood {
    Lonely,
    Waiting,
    Hopeful,
    Satisfied,
}

impl IniMood {
    fn parse(word: &str) -> Option<IniMood> {
        match word {
            "lonely" => Some(IniMood::Lonely),
            "waiting" => Some(IniMood::Waiting),
            "hopeful" => Some(IniMood::Hopeful),
            "satisfied" => Some(IniMood::Satisfied),
            _ => None,
        }
    }

    // 문의 대사에 들어가는 한국어 낱말(번역은 doors_game/i18n 표에서).
    pub fn word(self) -> &'static str {
        match self {
            IniMood::Lonely => "쓸쓸하다",
            IniMood::Waiting => "기다리는 중이다",
            IniMood::Hopeful => "기대하고 있다",
            IniMood::Satisfied => "만족했다",
        }
    }
}

// 문의 기분 — 진행에 따라 lonely → waiting → hopeful → satisfied.
fn door_mood(d: &DoorsProgress) -> &'static str {
    if d.flower_given {
        "satisfied"
    } else if d.door_changed {
        "hopeful"
    } else if d.quest_accepted {
        "waiting"
    } else {
        "lonely"
    }
}

// "key=value" 줄들에서 key 의 값(공백/CRLF 무시).
fn ini_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| l.trim().strip_prefix(key)?.trim_start().strip_prefix('=').map(str::trim))
}

// 게임 폴더의 config/game.ini 를 플레이어가 고쳤다면, 그게 실제 진행과 어떻게 다른지.
pub fn ini_edit(fs: &FileSystem) -> IniEdit {
    let Some(appdata) = fs.find_by_name(crate::foundation::APPDATA_NAME) else { return IniEdit::None };
    let Some(root) = child_named(fs, appdata, GAME_FOLDER_NAME) else { return IniEdit::None };
    let Some(config) = child_named(fs, root, "config") else { return IniEdit::None };
    let Some(ini) = child_named(fs, config, "game.ini") else { return IniEdit::None };
    if !fs.was_edited_by_player(ini) {
        return IniEdit::None;
    }
    let FileKind::Txt(text) = &fs.get(ini).kind else { return IniEdit::None };
    let d = &fs.doors_progress;
    // 가짜 꽃이 먼저 — 방에 꽃이 있다고 적었지만 실제로는 없다.
    if let Some(n) = ini_value(text, "flowers_in_room").and_then(|v| v.parse::<u32>().ok())
        && n > flowers_in_room(d)
    {
        return IniEdit::FakeFlowers;
    }
    match ini_value(text, "door_mood") {
        Some(word) if !word.is_empty() && word != door_mood(d) => match IniMood::parse(word) {
            Some(m) => IniEdit::Mood(m),
            None => IniEdit::UnknownMood,
        },
        _ => IniEdit::None,
    }
}

// 방 안에 지금 꽃이 몇 송이 있는지(손에 쥔 꽃도 방 안에 있는 것으로 센다).
fn flowers_in_room(d: &DoorsProgress) -> u32 {
    u32::from(d.flower_received && !d.flower_given)
}

fn flag(b: bool) -> u8 {
    u8::from(b)
}

fn render_slot(d: &DoorsProgress, m: &MazeProgress, minis: &[MiniProgress; 3]) -> String {
    let mission = |k: MiniKind| format!("{:?}", d.missions[k.index()]);
    let cleared = |k: MiniKind| flag(minis[k.index()].cleared);
    let place = match d.place {
        MainPlace::StartRoom => "start_room".to_string(),
        MainPlace::Jungle => "jungle".to_string(),
        MainPlace::MissionRoom(k) => format!("mission_room:{}", k.app_name()),
        MainPlace::StairHall => "stair_hall".to_string(),
    };
    format!(
        "[slot0]\nplace={place}\ndoor_mood={}\nflowers_in_room={}\nquest_accepted={}\nletter_sent={}\nflower_given={}\ntutorial_done={}\n\n[sub]\nwatering_can={}\nkeys={}\nseed_planted={}\nbloomed={}\nflower_sent={}\n\n[level2]\nerrand={}\neye={}\nbrain={}\nfinger={}\ncleared={}{}{}\nexchanged={}\nfed={}\ndone={}\n",
        door_mood(d),
        flowers_in_room(d),
        flag(d.quest_accepted),
        flag(d.letter_sent),
        flag(d.flower_given),
        flag(d.tutorial_done),
        flag(m.has_can),
        m.keys.iter().filter(|&&k| k).count(),
        flag(m.planted),
        flag(m.bloomed),
        flag(m.flower_sent),
        flag(d.errand_accepted),
        mission(MiniKind::Eye),
        mission(MiniKind::Brain),
        mission(MiniKind::Finger),
        cleared(MiniKind::Eye),
        cleared(MiniKind::Brain),
        cleared(MiniKind::Finger),
        flag(d.exchanged),
        flag(d.chip_fed),
        flag(d.floor2_done),
    )
}

// 설치 로그의 마지막 줄에 설치한 시각을 붙인다(한 번만).
fn stamp_install_log(text: &str, when: &str) -> String {
    if text.contains("install complete (") {
        return text.to_string();
    }
    text.replace("[ok]   install complete", &format!("[ok]   install complete ({when})"))
}

// 일어난 일 — (한 번만 적는 표식, 로그 줄, 조건). 표식은 줄 안에 그대로 들어 있어서 같은 일을 두 번 적지 않는다.
fn log_events(d: &DoorsProgress, m: &MazeProgress, minis: &[MiniProgress; 3]) -> Vec<(&'static str, bool)> {
    vec![
        ("[info] door: request accepted", d.quest_accepted),
        ("[info] mail_hook: outgoing letter relayed", d.letter_sent),
        ("[info] door: mood changed", d.door_changed),
        ("[warn] room 0: flower count is 0", d.flower_absence_checked),
        ("[info] maze: watering can acquired", m.has_can),
        ("[info] maze: seed planted", m.planted),
        ("[info] maze: pot state changed, clock moved forward", m.bloomed),
        ("[info] mail_hook: parcel relayed (flower)", m.flower_sent),
        ("[ok]   door: request satisfied", d.flower_given),
        ("[info] room 0: exit unlocked", d.tutorial_done),
        ("[warn] level 2: unexpected exterior (jungle)", d.jungle_visited),
        ("[info] stairs: hungry", d.stair_met),
        ("[info] sofa: errand registered", d.errand_accepted),
        ("[ok]   SHY_EYE: cleared", minis[MiniKind::Eye.index()].cleared),
        ("[ok]   BRAIN_DRIVE: cleared", minis[MiniKind::Brain.index()].cleared),
        ("[ok]   PACK: cleared", minis[MiniKind::Finger.index()].cleared),
        ("[info] sofa: trade completed", d.exchanged),
        ("[ok]   stairs: fed", d.chip_fed),
        ("[ok]   level 2: complete", d.floor2_done),
    ]
}

// 새로 일어난 일을 로그에 덧붙이고(시각과 함께), 너무 길면 오래된 줄부터 버린다. 바뀌었으면 Some(새 내용).
fn updated_log(old: &str, events: &[(&'static str, bool)], when: &str) -> String {
    let mut lines: Vec<String> = if old.is_empty() { vec![LOG_HEADER.trim_end().to_string()] } else { old.lines().map(str::to_string).collect() };
    for (what, happened) in events {
        if *happened && !lines.iter().any(|l| l.ends_with(what)) {
            lines.push(format!("[{when}] {what}"));
        }
    }
    let header = lines.first().cloned().filter(|l| l.starts_with('#'));
    let keep_from = lines.len().saturating_sub(LOG_MAX_LINES);
    let mut kept: Vec<String> = lines.split_off(keep_from);
    if let Some(h) = header
        && kept.first() != Some(&h)
    {
        kept.insert(0, h);
    }
    let mut out = kept.join("\n");
    out.push('\n');
    out
}

// 이 폴더 안의 텍스트 파일 내용을 text 로 맞춘다 — 없으면(플레이어가 지웠으면) 새로 만든다. 바뀌었으면 true.
fn set_text(fs: &mut FileSystem, parent: FileId, name: &str, text: &str) -> bool {
    match child_named(fs, parent, name) {
        // 플레이어가 고친 파일은 그대로 둔다(게임이 원래 내용으로 되돌리지 않는다).
        Some(id) if fs.was_edited_by_player(id) => false,
        Some(id) => match &mut fs.get_mut(id).kind {
            FileKind::Txt(old) if old.as_str() == text => false,
            FileKind::Txt(old) => {
                *old = text.to_string();
                true
            }
            _ => false,
        },
        None => {
            let id = fs.add(name, FileKind::Txt(text.to_string()));
            fs.add_to_folder(parent, id);
            true
        }
    }
}

fn text_of(fs: &FileSystem, parent: FileId, name: &str) -> String {
    child_named(fs, parent, name).map(|id| match &fs.get(id).kind {
        FileKind::Txt(t) => t.clone(),
        _ => String::new(),
    }).unwrap_or_default()
}

// 진행 상황을 게임 폴더의 파일들에 반영한다. 게임 폴더가 없으면(설치 전이거나 플레이어가 폴더째 지웠으면) 아무것도 안
// 한다. now_local 은 이 PC 의 지금(로컬 유닉스 초 + 가상 시계) — 로그에 적히는 시각이다. 바뀐 게 있으면 true.
pub fn sync_game_files(fs: &mut FileSystem, now_local: i64) -> bool {
    let appdata = fs.ensure_appdata();
    let Some(root) = child_named(fs, appdata, GAME_FOLDER_NAME) else { return false };
    let (d, m, minis) = (fs.doors_progress.clone(), fs.maze_progress.clone(), fs.minis);
    let when = format_when(now_local);
    let mut changed = false;

    // saves/slot0.sav — 지금 진행 상태. saves 폴더를 지웠으면 다시 만들고 그 사실을 로그에 남긴다.
    let had_saves = child_named(fs, root, "saves").is_some(); // (install_into 가 빈 saves 폴더를 만들어 두므로 처음엔 항상 있다)
    let saves = folder_at(fs, root, "saves");
    changed |= set_text(fs, saves, "slot0.sav", &render_slot(&d, &m, &minis));

    // config/game.ini — 문의 기분과 방 안의 꽃.
    let config = folder_at(fs, root, "config");
    let ini = GAME_INI
        .replace("\r\n", "\n")
        .replace("door_mood=lonely", &format!("door_mood={}", door_mood(&d)))
        .replace("flowers_in_room=0", &format!("flowers_in_room={}", flowers_in_room(&d)));
    changed |= set_text(fs, config, "game.ini", &ini);

    // logs/install.log 에 설치 시각, logs/game.log 에 일어난 일.
    let logs = folder_at(fs, root, "logs");
    let install = text_of(fs, logs, "install.log");
    if !install.is_empty() {
        changed |= set_text(fs, logs, "install.log", &stamp_install_log(&install, &when));
    }
    let mut events = log_events(&d, &m, &minis);
    events.push(("[warn] saves folder was missing, recreated", !had_saves));
    events.push(("[warn] config: game.ini was modified outside the game", ini_edit(fs) != IniEdit::None));
    let old_log = text_of(fs, logs, "game.log");
    changed |= set_text(fs, logs, "game.log", &updated_log(&old_log, &events, &when));

    // src/level2.rs — 2층에 나가 보면 그 소스가 새로 생긴다.
    if d.jungle_visited {
        let src = folder_at(fs, root, "src");
        changed |= set_text(fs, src, "level2.rs", &SRC_LEVEL2.replace("\r\n", "\n"));
    }
    changed
}

// 로그에 적는 시각 "YYYY-MM-DD HH:MM".
fn format_when(local_unix: i64) -> String {
    use crate::scenes::desktop::clock::{civil_date, hour_minute};
    let (y, mo, d) = civil_date(local_unix);
    let (hh, mm) = hour_minute(local_unix);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}")
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

    // ---- 살아 있는 게임 파일 ----
    const NOW: i64 = 1_759_708_800 + 13 * 3600 + 5 * 60; // 2025-10-06 13:05

    fn game_root(fs: &FileSystem) -> FileId {
        let appdata = fs.find_by_name("AppData").unwrap();
        child_named(fs, appdata, GAME_FOLDER_NAME).unwrap()
    }

    fn read(fs: &FileSystem, path: &str) -> String {
        let id = find(fs, game_root(fs), path).unwrap_or_else(|| panic!("{path} 가 없다"));
        match &fs.get(id).kind {
            FileKind::Txt(t) => t.clone(),
            _ => panic!("{path}: 글 파일이 아니다"),
        }
    }

    #[test]
    fn nothing_happens_before_the_game_is_installed() {
        let mut fs = FileSystem::new();
        assert!(!sync_game_files(&mut fs, NOW));
        assert!(fs.find_by_name("AppData").is_some_and(|a| child_named(&fs, a, GAME_FOLDER_NAME).is_none()));
    }

    #[test]
    fn the_save_and_ini_follow_the_progress() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        assert!(read(&fs, "saves/slot0.sav").contains("door_mood=lonely") && read(&fs, "config/game.ini").contains("door_mood=lonely"));
        fs.doors_progress.quest_accepted = true;
        sync_game_files(&mut fs, NOW);
        assert!(read(&fs, "config/game.ini").contains("door_mood=waiting"));
        fs.doors_progress.door_changed = true;
        fs.doors_progress.flower_received = true;
        sync_game_files(&mut fs, NOW);
        let ini = read(&fs, "config/game.ini");
        assert!(ini.contains("door_mood=hopeful") && ini.contains("flowers_in_room=1"), "{ini}");
        fs.doors_progress.flower_given = true;
        sync_game_files(&mut fs, NOW);
        let ini = read(&fs, "config/game.ini");
        assert!(ini.contains("door_mood=satisfied") && ini.contains("flowers_in_room=0"), "{ini}");
        assert!(read(&fs, "saves/slot0.sav").contains("flower_given=1"));
        assert!(!read(&fs, "config/game.ini").contains('\r'));
    }

    #[test]
    fn the_log_records_each_event_once_with_the_time_it_was_first_seen() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        assert!(sync_game_files(&mut fs, NOW));
        assert!(!sync_game_files(&mut fs, NOW + 60), "바뀐 게 없으면 파일을 건드리지 않는다");
        fs.doors_progress.quest_accepted = true;
        sync_game_files(&mut fs, NOW + 3600);
        sync_game_files(&mut fs, NOW + 7200); // 같은 일을 또 적지 않는다
        let log = read(&fs, "logs/game.log");
        assert_eq!(log.matches("door: request accepted").count(), 1, "{log}");
        assert!(log.contains("[2025-10-06 14:05] [info] door: request accepted"), "{log}");
        assert!(log.starts_with("# game.log"));
        assert!(read(&fs, "logs/install.log").contains("install complete (2025-10-06 13:05)"), "설치 시각이 한 번 붙는다");
        sync_game_files(&mut fs, NOW + 9999);
        assert_eq!(read(&fs, "logs/install.log").matches("install complete (").count(), 1);
    }

    #[test]
    fn the_jungle_source_appears_only_after_leaving_the_room() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        assert!(find(&fs, game_root(&fs), "src/level2.rs").is_none());
        fs.doors_progress.jungle_visited = true;
        sync_game_files(&mut fs, NOW);
        assert!(read(&fs, "src/level2.rs").contains("manners matter"));
    }

    #[test]
    fn deleted_files_come_back_quietly_but_a_deleted_game_folder_stays_gone() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        // 플레이어가 saves 폴더를 지웠다 — 게임이 다시 만들고 로그에 남긴다.
        let root = game_root(&fs);
        let saves = child_named(&fs, root, "saves").unwrap();
        if let FileKind::Folder { children } = &mut fs.get_mut(root).kind {
            children.retain(|&c| c != saves);
        }
        assert!(child_named(&fs, root, "saves").is_none());
        assert!(sync_game_files(&mut fs, NOW + 60));
        assert!(child_named(&fs, root, "saves").is_some() && read(&fs, "saves/slot0.sav").contains("[slot0]"));
        assert!(read(&fs, "logs/game.log").contains("saves folder was missing, recreated"));
        // 게임 폴더(test)째 지우면 되살리지 않는다.
        let appdata = fs.find_by_name("AppData").unwrap();
        if let FileKind::Folder { children } = &mut fs.get_mut(appdata).kind {
            children.retain(|&c| c != root);
        }
        assert!(!sync_game_files(&mut fs, NOW + 120));
        assert!(child_named(&fs, appdata, GAME_FOLDER_NAME).is_none());
    }

    #[test]
    fn the_log_is_capped_and_keeps_its_header() {
        let mut old = String::from("# game.log\n");
        for i in 0..200 {
            old.push_str(&format!("[2025-01-01 00:00] line {i}\n"));
        }
        let out = updated_log(&old, &[("[ok]   something new", true)], "2025-10-06 13:05");
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines.len() <= LOG_MAX_LINES + 1, "{}", lines.len());
        assert_eq!(lines[0], "# game.log");
        assert!(lines.last().unwrap().ends_with("something new"));
        assert!(!out.contains("line 0\n"), "오래된 줄은 버려진다");
    }

    #[test]
    fn a_minigame_folder_quotes_the_real_rules() {
        let mut fs = FileSystem::new();
        install_mini(&mut fs, MiniKind::Finger);
        let appdata = fs.find_by_name("AppData").unwrap();
        let root = child_named(&fs, appdata, "PACK").unwrap();
        let cfg = match &fs.get(child_named(&fs, root, "config.ini").unwrap()).kind {
            FileKind::Txt(t) => t.clone(),
            _ => panic!(),
        };
        assert_eq!(cfg, crate::apps::minigame::rules_text(MiniKind::Finger).1);
    }

    // 메모장에서 고친 게임 파일은 게임이 다시 덮어쓰지 않는다(고치지 않은 파일은 계속 갱신된다).
    #[test]
    fn files_the_player_edited_are_left_alone() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        let root = game_root(&fs);
        let ini = find(&fs, root, "config/game.ini").unwrap();
        assert!(fs.save_text(ini, "door_mood=angry\n".to_string()));
        fs.doors_progress.quest_accepted = true;
        sync_game_files(&mut fs, NOW + 60);
        assert_eq!(read(&fs, "config/game.ini"), "door_mood=angry\n", "고친 설정 파일은 그대로");
        assert!(read(&fs, "saves/slot0.sav").contains("quest_accepted=1"), "고치지 않은 파일은 계속 갱신된다");
        assert!(read(&fs, "logs/game.log").contains("request accepted"));
    }

    // ---- 고친 game.ini 에 대한 읽기 ----
    fn edit_ini(fs: &mut FileSystem, text: &str) {
        let ini = find(fs, game_root(fs), "config/game.ini").unwrap();
        assert!(fs.save_text(ini, text.to_string()));
    }

    #[test]
    fn only_a_player_edited_ini_that_differs_from_reality_counts() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        assert_eq!(ini_edit(&fs), IniEdit::None, "안 고쳤으면 None");
        // 실제 기분과 같게 적으면(또는 기분을 지우면) 반응할 게 없다.
        edit_ini(&mut fs, "[game]\ndoor_mood=lonely\n");
        assert_eq!(ini_edit(&fs), IniEdit::None);
        edit_ini(&mut fs, "[game]\nroom=0\n");
        assert_eq!(ini_edit(&fs), IniEdit::None);
        // 다른 (아는) 기분.
        edit_ini(&mut fs, "[game]\r\ndoor_mood = hopeful \r\n");
        assert_eq!(ini_edit(&fs), IniEdit::Mood(IniMood::Hopeful), "공백과 CRLF 는 무시한다");
        // 모르는 말.
        edit_ini(&mut fs, "door_mood=furious\n");
        assert_eq!(ini_edit(&fs), IniEdit::UnknownMood);
        // 진행이 그 기분에 도달하면 더는 어긋나지 않는다.
        fs.doors_progress.quest_accepted = true;
        edit_ini(&mut fs, "door_mood=waiting\n");
        assert_eq!(ini_edit(&fs), IniEdit::None);
    }

    #[test]
    fn fake_flowers_win_over_a_mood_and_never_change_progress() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        edit_ini(&mut fs, "door_mood=hopeful\nflowers_in_room=3\n");
        assert_eq!(ini_edit(&fs), IniEdit::FakeFlowers);
        // 정말 꽃이 한 송이 있으면 1 이라고 적는 건 거짓이 아니다.
        fs.doors_progress.flower_received = true;
        edit_ini(&mut fs, "flowers_in_room=1\n");
        assert_eq!(ini_edit(&fs), IniEdit::None);
        assert!(!fs.doors_progress.flower_given && !fs.doors_progress.tutorial_done, "진행은 그대로다");
    }

    #[test]
    fn the_log_notes_a_modified_config_once() {
        let mut fs = FileSystem::new();
        install_into(&mut fs);
        sync_game_files(&mut fs, NOW);
        assert!(!read(&fs, "logs/game.log").contains("modified outside the game"));
        edit_ini(&mut fs, "door_mood=hopeful\n");
        sync_game_files(&mut fs, NOW + 60);
        sync_game_files(&mut fs, NOW + 120);
        assert_eq!(read(&fs, "logs/game.log").matches("game.ini was modified outside the game").count(), 1);
    }
}

