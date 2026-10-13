// 개발용 도구 — 게임을 이미 다 설치해 둔 저장 파일(palaceos_save.json)을 만든다.
// 사용법: cargo run --example make_test_build -- <저장 파일을 쓸 경로>
// 바탕화면에 test.exe / test2.exe / SHY_EYE.exe / BRAIN_DRIVE.exe / PACK.exe 아이콘이 이미 있고,
// 각 게임의 AppData 폴더도 만들어져 있다. 이야기 진행은 처음 그대로다.

use crackhead::foundation::{FileKind, FileSystem, MailId, MiniKind, SaveData, Settings, GAME_FILE_NAME, SUB_GAME_NAME};

fn main() {
    let out = std::env::args().nth(1).expect("저장 파일 경로가 필요하다");
    let mut fs = FileSystem::new();
    fs.deliver_mail(MailId::Friend);

    // 메인 게임(test.exe) — 설치 마법사를 끝낸 것과 같은 상태.
    fs.game_installed = true;
    crackhead::gamefiles::install_into(&mut fs);
    let id = fs.add(GAME_FILE_NAME, FileKind::Game);
    fs.desktop.push(id);

    // 서브 게임(test2.exe)
    fs.sub_game_ready = true;
    let id = fs.add(SUB_GAME_NAME, FileKind::SubGame);
    fs.desktop.push(id);

    // 미니게임 셋 — 설치 파일도 메일 첨부로 받아 둔 상태.
    for kind in MiniKind::ALL {
        fs.deliver_mail(kind.task_mail());
        fs.mini_installed[kind.index()] = true;
        crackhead::gamefiles::install_mini(&mut fs, kind);
        let id = fs.add(kind.exe_name(), FileKind::MiniGame(kind));
        fs.desktop.push(id);
    }

    // icon_pos 가 비어 있으면 불러올 때 기본 격자 위치로 채워진다.
    let data = SaveData { settings: Settings::default(), fs, icon_pos: Vec::new(), window_geometry: Vec::new() };
    std::fs::write(&out, serde_json::to_string_pretty(&data).unwrap()).unwrap();
    println!("wrote {out}");
}
