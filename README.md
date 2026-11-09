# BlackOut

Rust + [miniquad](https://github.com/not-fl3/miniquad)로 만든 **가짜 Windows 9x 데스크톱 OS** 위의
아날로그 호러 / ARG 게임. 화면은 CRT 모니터처럼 곡률·스캔라인·색수차가 입혀져 있다.

## 개요

플레이어는 낡은 PC 앞에 앉아 있다. 데스크톱에는 메일, 파일 탐색기, 메모장, 영상/이미지/사운드
플레이어 같은 프로그램이 있고, 어느 날 친구가 메일로 게임 하나를 보내온다. 설치해서 실행하면
방 안에서 말하는 문(도어즈)을 만나고, 그 부탁을 들어주려고 게임 안과 PC 화면을 오가게 된다 —
게임 속에서 한 일이 메일로 도착하고, 메일의 내용이 다시 게임에 영향을 준다.

- 전체화면 4:3 CRT 연출, 모든 UI 문자열은 영어/한국어/일본어
- 창 안에서 도는 1인칭 3D 게임(자체 3D 렌더러)과 그 안의 미니 게임

## 실행

Windows 전용이다(영상 재생에 Media Foundation, 오디오에 WASAPI, 웹 창에 WebView2를 쓴다).

```bash
cargo run              # 게임
cargo run --release    # 빠른 빌드
cargo test --lib       # 단위 테스트
```

릴리스 빌드는 [Releases](https://github.com/shupil1021/Untitled/releases)에서 받을 수 있다.

## 폴더 구조

한 파일은 되도록 한 가지 일만 하고(대부분 400줄 안쪽), 큰 기능은 폴더로 나눠서 `mod.rs` 가 목차 역할을 한다.
수정할 곳을 찾을 때는 아래 표에서 폴더를 고른 뒤 그 폴더의 `mod.rs` 맨 위 설명을 읽으면 된다.

```
src/
├── main.rs · lib.rs      진입점(창/프레임 루프) · 라이브러리 루트
├── foundation/           기반: fs.rs(가짜 파일 시스템) · progress.rs(게임 진행 상태, 메일 종류) · settings.rs · save.rs
├── scenes/               화면 단위: lobby · boot · desktop/ · shutdown · erase · bluescreen
│   └── desktop/            바탕화면 — mod.rs(프레임 루프) · icons(아이콘/드래그) · popups(작업표시줄/메뉴) ·
│                           actions(창이 요청한 동작) · timers(시간으로 일어나는 일) · network(와이파이) · clock
├── apps/                 창 안에서 도는 프로그램
│   ├── mail/               메일 — inbox · sent · compose · content(메일 종류별 내용)
│   ├── doors_game/         메인 게임 — world(시작방) · floor2(정글) · rooms(미션 방/계단) · interact(E 로 조사한 결과) ·
│   │                       travel(장소 이동/연출) · scene(장면 캐시) · texts(대사) · tests
│   ├── maze_game/          서브 게임(미로 + 화분 방)
│   ├── minigame/           2D/3D 미니게임 — eye(탄막) · pack(포장 리듬) · drive/(3D 카트 레이스) · view(그리기)
│   │   └── drive/            track(코스/갈림길) · kart(물리) · ai · scene(길/가드레일 메시) · mod(레이스 진행)
│   ├── game3d/             창 안 3D 게임이 같이 쓰는 도구: player · dialogue · item · boxes · ena · sprite
│   └── explorer · notepad · image_viewer · sound_player · video_player · settings …
├── render/               gfx(2D) · mesh3d/(3D: math · shapes · obj 파서 · collide · gpu 렌더러/정점 캐시) · crt(CRT 효과)
├── gamefiles/            게임 폴더에 생기는 파일들 — texts(글) · images(코드로 만든 그림) · sounds(WAV)
├── platform/             Windows 전용(영상·오디오, 웹뷰, IME)
├── window_manager.rs     창 관리자 · ui.rs 위젯 · strings.rs 다국어 문자열(en/ko/ja) · signals.rs OS→게임 상태 스냅숏
├── bin/                  녹화용 도구(director)
examples/                 개발용: make_test_build(게임을 다 설치해 둔 저장 파일), race_preview(레이스 장면 PNG)
assets/                   아이콘 · 폰트 · 커서
```

## 개발 메모

- `cargo test --lib` 로 단위 테스트(진행 흐름, 미니게임 규칙, 코스 기하 등)를 돌린다. 게임 로직은 화면과 분리돼 있어서 대부분 테스트로 확인한다.
- 장면을 눈으로 확인하려면 `cargo run --example race_preview -- <폴더>` 처럼 PNG 로 뽑아 본다.
- 문구는 `strings.rs` 에 en/ko/ja 를 같이 적는다. 일본어 한자는 `render/gfx.rs` 의 `KANJI_CHARSET` 에 있어야 보인다.
- 창 안 3D 게임의 대사/조사 안내는 한국어(또는 `[E] ...` 영어)로 코드에 적고, `apps/game3d/i18n.rs` 의 표에 en/ko/ja 를 추가한다. 표에 빠진 문구와 글자 목록에 없는 한자는 `cargo test --lib` 가 잡아 준다.
- 3D 장면은 `Box3D`(상자)와 `MeshObject`(삼각형 메시)로 만들고 `render/mesh3d/` 가 그린다.
- 플레이 기록: 설정의 "플레이 기록"이 켜져 있으면 단계별 소요 시간/미니게임 실패가 저장 파일 옆 `palaceos_playlog.jsonl` 에 JSON 줄로 쌓인다(이 PC 안에만 남고 전송하지 않는다). `cargo run --example playlog_report` 로 요약해서 어느 단계에서 막히는지 본다.
- 가짜 파일시스템의 AppData(게임 폴더)는 설치 뒤에 탐색기에 나타나고, 진행에 따라 `saves/slot0.sav` · `config/game.ini` · `logs/game.log` 가 갱신되며 2층에 나가면 `src/level2.rs` 가 생긴다(`gamefiles::sync_game_files`). 미니게임 폴더의 규칙 숫자는 게임 코드의 상수에서 가져온다.
- 메모장은 모든 .txt 를 고칠 수 있고(커서/IME/Backspace 반복, 드래그·Shift 선택, Ctrl+A/C/X/V, Ctrl+휠로 글자 크기) 쉬는 순간 자동 저장된다. 글자 입력 처리는 메일 작성창과 `apps/text_edit.rs` 를 같이 쓴다. 고친 게임 파일은 `fs.player_edited` 에 기록돼서 게임이 다시 덮어쓰지 않는다.
- 고친 `game.ini` 는 게임이 읽는다 — 문이 한마디 한다(진행은 바뀌지 않는다, `gamefiles::ini_edit`).

