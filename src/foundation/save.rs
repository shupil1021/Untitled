//! 저장/불러오기 — JSON 하나(palaceos_save.json)와 설정 전용 파일(palaceos_settings.json). 기본은 exe 옆(폴더째
//! 들고 다니게)이고, exe 폴더에 쓸 수 없으면(Program Files 등) 사용자 데이터 폴더로 옮겨 저장한다. 그것도 실패하면
//! 조용히 넘어가지 않고 `take_save_failure()` 로 화면에 알릴 수 있게 한다.

use serde::{Deserialize, Serialize};

use super::*;

// 설정 + 바탕화면 상태(아이콘 위치/휴지통/잠금 해제 여부) 자동 저장.
// exe 옆에 JSON 하나로 저장하고, 실행할 때 있으면 그대로 복원한다.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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

const SAVE_NAME: &str = "palaceos_save.json";
const SETTINGS_NAME: &str = "palaceos_settings.json";

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf))
}

// exe 폴더에 못 쓸 때 쓰는 사용자 데이터 폴더(Windows: %LOCALAPPDATA%\PalaceOS).
fn fallback_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_DATA_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("PalaceOS")
}

// 이 폴더에 파일을 실제로 만들 수 있는지(읽기 전용 폴더나 권한 없는 폴더를 걸러낸다).
fn dir_is_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".palaceos_write_test_{}", std::process::id()));
    let ok = std::fs::write(&probe, b"x").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

// 저장 폴더 — exe 옆에 쓸 수 있으면 거기(포터블), 아니면 사용자 데이터 폴더. 실행 중에는 한 번만 정한다.
fn data_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = exe_dir().filter(|d| dir_is_writable(d)) {
            return dir;
        }
        let fallback = fallback_dir();
        if std::fs::create_dir_all(&fallback).is_ok() && dir_is_writable(&fallback) {
            return fallback;
        }
        exe_dir().unwrap_or_else(|| PathBuf::from("."))
    })
}

fn save_path() -> PathBuf {
    data_dir().join(SAVE_NAME)
}

// 예전에 exe 옆에 저장해 둔 파일(저장 폴더가 사용자 데이터 폴더로 바뀐 경우에만 따로 읽어 본다).
fn legacy_path(name: &str) -> Option<PathBuf> {
    exe_dir().map(|d| d.join(name)).filter(|p| p.parent() != Some(data_dir()))
}

// 저장 파일이 없거나 읽기/파싱에 실패하면 None (처음 실행이거나 손상된 경우 — 기본값으로 시작).
// 본 파일이 깨져 있으면 직전에 성공한 저장(.bak)으로 한 번 더 시도한다.
pub fn load() -> Option<SaveData> {
    read_json(&save_path()).or_else(|| legacy_path(SAVE_NAME).and_then(|p| read_json(&p)))
}

pub fn save(data: &SaveData) {
    let ok = write_json(&save_path(), data);
    HEALTH.lock().unwrap_or_else(|e| e.into_inner()).record(ok);
}

// 저장이 성공했는지 실패했는지 — "성공 → 실패"로 바뀌는 순간에만 한 번 알리고(매번 알리면 5초마다 뜬다), 다시
// 성공하면 다음 실패 때 또 알린다.
#[derive(Default)]
struct SaveHealth {
    failing: bool,
    notice_pending: bool,
}

impl SaveHealth {
    fn record(&mut self, ok: bool) {
        if ok {
            self.failing = false;
        } else if !self.failing {
            self.failing = true;
            self.notice_pending = true;
        }
    }

    fn take_notice(&mut self) -> bool {
        std::mem::take(&mut self.notice_pending)
    }
}

static HEALTH: Mutex<SaveHealth> = Mutex::new(SaveHealth { failing: false, notice_pending: false });

// 저장에 막 실패했고 아직 알리지 않았으면 true(한 번만). 화면에서 "저장하지 못했어요" 알림을 띄울 때 쓴다.
pub fn take_save_failure() -> bool {
    HEALTH.lock().unwrap_or_else(|e| e.into_inner()).take_notice()
}

// 지금 진행을 저장하는 폴더(알림에서 어디를 확인할지 알려줄 때 쓴다).
pub fn save_dir() -> PathBuf {
    data_dir().to_path_buf()
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".bak");
    PathBuf::from(name)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    for p in [path.to_path_buf(), backup_path(path)] {
        if let Ok(text) = std::fs::read_to_string(&p)
            && let Ok(v) = serde_json::from_str(&text)
        {
            return Some(v);
        }
    }
    None
}

// 쓰는 도중에 꺼져도 본 파일이 반쯤 쓰인 채로 남지 않게 — 임시 파일에 다 쓴 뒤 이름만 바꿔 끼우고,
// 바꾸기 전의 본 파일은 .bak 로 남겨 둔다(읽기에 실패하면 read_json 이 그쪽을 쓴다).
// 성공하면 true.
fn write_json<T: Serialize>(path: &Path, data: &T) -> bool {
    let Ok(json) = serde_json::to_string_pretty(data) else { return false };
    let mut tmp_name = path.as_os_str().to_owned();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    if std::fs::write(&tmp, json).is_err() {
        return false;
    }
    // 새 파일이 제대로 읽히는 경우에만 지금 본 파일을 백업으로 승격한다(이미 깨진 파일로 좋은 백업을 덮지 않게).
    if path.exists() && std::fs::read_to_string(path).ok().is_some_and(|s| serde_json::from_str::<serde_json::Value>(&s).is_ok()) {
        let _ = std::fs::copy(path, backup_path(path));
    }
    if std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    true
}

// "Erase All Memory" / New Start — 진행을 지운다. 파일을 정말 지우지 않고 `.erased` 로 옮겨 두어서, 로비의 "삭제 취소"로
// 되돌릴 수 있다(다음 삭제 때 덮어쓴다). 옮기지 못하면 그냥 지운다. 실패해도 조용히 넘어간다 — 호출부는 지웠다고 가정하고
// 부팅부터 다시 시작한다.
pub fn delete() {
    erase_file(&save_path(), &erased_path());
}

fn erased_path() -> PathBuf {
    let mut name = save_path().into_os_string();
    name.push(".erased");
    PathBuf::from(name)
}

fn erase_file(save: &Path, erased: &Path) {
    if save.exists() && std::fs::rename(save, erased).is_err() {
        let _ = std::fs::remove_file(save);
    }
    let _ = std::fs::remove_file(backup_path(save)); // 백업이 남아 있으면 지운 진행이 되살아난다
}

// 되돌릴 수 있는 삭제된 진행이 있는지(읽을 수 있는 파일만 센다).
pub fn has_erased() -> bool {
    read_json::<SaveData>(&erased_path()).is_some()
}

// 마지막으로 삭제한 진행을 되돌린다. 지금 진행 파일이 이미 있으면(새로 시작해서 쌓인 게 있으면) 덮어쓰지 않고 false.
pub fn restore_erased() -> bool {
    restore_file(&erased_path(), &save_path())
}

fn restore_file(erased: &Path, save: &Path) -> bool {
    !save.exists() && erased.exists() && std::fs::rename(erased, save).is_ok()
}

fn settings_path() -> PathBuf {
    data_dir().join(SETTINGS_NAME)
}

// 언어 등 취향 설정은 게임 진행 저장(palaceos_save.json)과 별개 파일에 즉시
// 저장한다 — 진행 저장은 5초마다(desktop.rs::AUTOSAVE_INTERVAL) 또는 특정
// 이벤트에서만 묶어서 쓰이고, 애초에 아직 게임을 시작하지 않은 로비 화면에서는
// FileSystem 이 없어 SaveData 자체를 만들 수도 없다 — 그래서 설정 변경은 이
// 훨씬 가벼운 전용 파일로 언제 어디서 바뀌든 그 즉시 디스크에 반영한다("Erase
// All Memory" 를 눌러도 이 파일은 안 건드린다 — 진행 상황을 지운다고 언어/그래픽
// 취향까지 초기화되면 당황스러우니까).
pub fn save_settings(settings: &Settings) {
    let ok = write_json(&settings_path(), settings);
    HEALTH.lock().unwrap_or_else(|e| e.into_inner()).record(ok);
}

pub fn load_settings() -> Option<Settings> {
    read_json(&settings_path()).or_else(|| legacy_path(SETTINGS_NAME).and_then(|p| read_json(&p)))
}



#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("palaceos_test_{}_{}.json", name, std::process::id()))
    }

    fn cleanup(p: &Path) {
        for f in [p.to_path_buf(), backup_path(p)] {
            let _ = std::fs::remove_file(f);
        }
    }

    #[test]
    fn write_then_read_roundtrip() {
        let p = temp_path("roundtrip");
        cleanup(&p);
        write_json(&p, &vec![1, 2, 3]);
        assert_eq!(read_json::<Vec<i32>>(&p), Some(vec![1, 2, 3]));
        cleanup(&p);
    }

    #[test]
    fn corrupt_main_falls_back_to_backup() {
        let p = temp_path("backup");
        cleanup(&p);
        write_json(&p, &vec![1]);
        write_json(&p, &vec![2]); // 본 파일 = [2], 백업 = [1]
        std::fs::write(&p, "{ 깨진 json").unwrap();
        assert_eq!(read_json::<Vec<i32>>(&p), Some(vec![1]));
        cleanup(&p);
    }

    #[test]
    fn corrupt_main_does_not_overwrite_good_backup() {
        let p = temp_path("keep_backup");
        cleanup(&p);
        write_json(&p, &vec![1]);
        write_json(&p, &vec![2]);
        std::fs::write(&p, "깨짐").unwrap();
        write_json(&p, &vec![3]); // 깨진 본 파일은 백업으로 승격되지 않는다
        assert_eq!(read_json::<Vec<i32>>(&p), Some(vec![3]));
        std::fs::write(&p, "깨짐").unwrap();
        assert_eq!(read_json::<Vec<i32>>(&p), Some(vec![1]));
        cleanup(&p);
    }

    #[test]
    fn writing_into_a_missing_folder_reports_failure() {
        let dir = std::env::temp_dir().join(format!("palaceos_no_such_dir_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!write_json(&dir.join("x.json"), &vec![1]), "없는 폴더에는 쓸 수 없다");
        let p = temp_path("ok_flag");
        cleanup(&p);
        assert!(write_json(&p, &vec![1]));
        cleanup(&p);
    }

    #[test]
    fn a_read_only_looking_dir_is_not_writable_but_the_temp_dir_is() {
        assert!(dir_is_writable(&std::env::temp_dir()));
        assert!(!dir_is_writable(&std::env::temp_dir().join("palaceos_definitely_missing_dir")));
    }

    #[test]
    fn failure_is_announced_once_until_a_save_succeeds_again() {
        let mut h = SaveHealth::default();
        h.record(true);
        assert!(!h.take_notice(), "성공만 했으면 알릴 게 없다");
        h.record(false);
        h.record(false); // 5초마다 계속 실패해도
        assert!(h.take_notice(), "처음 실패했을 때 한 번 알린다");
        assert!(!h.take_notice());
        h.record(false);
        assert!(!h.take_notice(), "계속 실패하는 동안은 또 알리지 않는다");
        h.record(true);
        h.record(false);
        assert!(h.take_notice(), "회복한 뒤 다시 실패하면 또 알린다");
    }

    #[test]
    fn fallback_dir_lives_under_an_app_folder() {
        assert!(fallback_dir().ends_with("PalaceOS"));
    }

    #[test]
    fn erasing_keeps_the_save_restorable_until_new_progress_exists() {
        let save = temp_path("erase_save");
        let erased = temp_path("erase_erased");
        for f in [&save, &erased] {
            cleanup(f);
        }
        write_json(&save, &vec![1, 2, 3]);
        erase_file(&save, &erased);
        assert!(!save.exists() && erased.exists(), "지운 진행은 .erased 로 옮겨진다");
        // 새 진행이 이미 생겼으면 되돌리기는 덮어쓰지 않는다.
        write_json(&save, &vec![9]);
        assert!(!restore_file(&erased, &save));
        assert_eq!(read_json::<Vec<i32>>(&save), Some(vec![9]));
        // 새 진행이 없으면 되돌아온다.
        std::fs::remove_file(&save).unwrap();
        assert!(restore_file(&erased, &save));
        assert_eq!(read_json::<Vec<i32>>(&save), Some(vec![1, 2, 3]));
        assert!(!erased.exists());
        for f in [&save, &erased] {
            cleanup(f);
        }
    }

    #[test]
    fn erasing_also_drops_the_backup_so_nothing_resurrects() {
        let save = temp_path("erase_bak");
        let erased = temp_path("erase_bak_erased");
        for f in [&save, &erased] {
            cleanup(f);
        }
        write_json(&save, &vec![1]);
        write_json(&save, &vec![2]); // .bak = [1]
        assert!(backup_path(&save).exists());
        erase_file(&save, &erased);
        assert!(!backup_path(&save).exists());
        assert_eq!(read_json::<Vec<i32>>(&save), None);
        for f in [&save, &erased] {
            cleanup(f);
        }
    }

    #[test]
    fn missing_files_give_none() {
        let p = temp_path("missing");
        cleanup(&p);
        assert_eq!(read_json::<Vec<i32>>(&p), None);
    }
}
