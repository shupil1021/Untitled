# PalaceOS (CrackHead)

Rust + [miniquad](https://github.com/not-fl3/miniquad)로 만드는 **가짜 Windows 9x 데스크톱 OS**.
Windows 9x / NT 4.0~2000 룩앤필 + CRT 모니터 느낌(화면 곡률·스캔라인·색수차)을 낸다.
지금은 데스크톱 토이 형태지만, 최종 목표는 이 위에 얹을 **ARG/아날로그 호러 게임의 셸**이다 —
그래서 부팅/종료/삭제 같은 "시스템 자체가 살아있는" 연출에 공을 들인다.
스토리 설정은 [STORY.md](STORY.md), 챕터별 개발 순서는 [ROADMAP.md](ROADMAP.md) 참고.

## 실행

```bash
cargo run
```

- Windows 전용(mp4 디코딩에 Media Foundation, 오디오에 WASAPI, Official Site 에 WebView2를
  쓴다) — 이 프로젝트는 Windows에서만 빌드/실행된다.
- 항상 전체화면 4:3 레터박스로 뜬다. 아이콘/버튼은 마우스, 프로그램 안 텍스트 입력·스크롤은
  키보드(방향키 등)도 쓴다.
- `cargo build`(dev 프로파일)는 `Cargo.toml`에 `[profile.dev] opt-level = 2`를 설정해뒀다 —
  기본 `opt-level=0`이면 영상 프레임 변환 같은 픽셀 루프가 매우 느려서다. 최고 성능이
  필요하면 `cargo run --release`.
- `assets/movie.mp4`를 교체하면 재빌드 없이 반영된다(어떤 H.264 프로파일이든 재생 가능).

## 기능

### 부팅 · 로비 · 종료

- **로비(타이틀 화면)**: 로고 뒤로 항상 켜져 있는 정전기 노이즈 + 주기적인 화면 찢김 글리치.
  New Start / Continue(저장 데이터 있을 때만 활성) / Settings / Quit 메뉴. Quit은 화면이
  거의 암전에 가깝게 어두워지며 더 불안정한 글리치가 계속되는 채로 "Really quit PalaceOS?"
  Yes/No를 띄운다.
- **부팅 화면**: BIOS POST(메모리 카운트업 + 장치 인식 로그, 항상 영어) → 로고 → Welcome →
  들쭉날쭉하게 채워지는 로딩 바 → 데스크톱.
- **Erase All Memory**: 저장 파일을 지우고 콘솔 로그 연출(가짜 파일 삭제 목록이 실제 실행
  파일 경로 기준으로 스크롤)을 보여준 뒤 로비로 복귀.
- **블루스크린(BSOD)**: 스토리에서 아직 실제로 트리거되지는 않는 독립 씬 — 클래식 Windows
  9x 블루스크린을 재현. 아무 키/클릭으로 부팅 화면으로 돌아간다.

### 바탕화면 셸

- **아이콘 그리드**: 9×7 고정 격자, 드래그(고스트 미리보기 + 놓으면 가장 가까운 빈 칸으로
  스냅), 빈 화면 드래그로 고무줄 다중 선택, 빈 화면 우클릭 컨텍스트 메뉴.
- **드래그 앤 드롭 규칙**: 폴더/열린 탐색기 창으로 옮기면 그 위치로 이동. My Computer와
  휴지통 자신은 어디로도 옮길 수 없다. 지금 열려있는 파일은 휴지통으로 못 보낸다.
- **작업표시줄**: 시작 버튼(+메뉴: Official Site/Credits/Settings/Shut Down), 창 버튼,
  와이파이 아이콘(실제 연결 상태·SSID·IP, 클릭 시에만 조회), KST 시계.
- **창 관리**(`window_manager.rs`): 같은 파일은 중복으로 안 열리고 기존 창을 앞으로 가져옴,
  이동/크기조절/최소화/최대화, 창마다 `resizable`/`maximizable` 개별 설정, 마지막 위치·크기
  기억, 새 창은 짧은 로딩 스피너 후 열림, 가려진 창은 입력을 안 받음.

### 파일 탐색기 · 휴지통

- **File Explorer**: 주소 표시줄(읽기 전용), 사이드바 폴더 트리(+/- 로 인라인 드릴다운),
  목록형/아이콘 그리드형 보기 전환, 상태 표시줄. My Computer 창은 Downloads/Desktop/
  Videos/Images(종류 기준 가상 카테고리) 고정 탭을 가진다.
  드래그 다중 선택(고무줄), 파일 드래그 앤 드롭으로 이동.
- **휴지통**: 전용 안내 패널(아이콘/설명/"Empty Recycle Bin" 링크) + 아이콘 그리드. 더블
  클릭으로는 안 열리고(실제 Windows처럼) Restore/Delete만 가능. Restore는 원래 있던
  자리(바탕화면/Downloads/폴더)로 정확히 되돌린다.

### 메일

- Outlook Express 스타일: 폴더 트리(Inbox/Sent Items/Write Mail) + 오른쪽 내용 패널,
  하단 상태 표시줄("N Item(s), N Unread").
- **Inbox**: 새 게임을 시작하면 처음엔 비어있다가, 5초(`MAIL_ARRIVAL_DELAY`, `scenes/
  desktop.rs`) 뒤에 메일이 한 통 자동으로 도착한다. 도착하는 순간 화면 우측
  하단(작업표시줄 바로 위)에 5초짜리 "New Mail" 토스트 알림이 뜨는데, 누르면 바로
  Mail 을 연다(`DesktopScene::update_toast`). 보낸 사람은 친구(`FIRST_MAIL_FROM`,
  `apps/mail.rs`)이고, 크랙한 게임을 보낸다는 짧은 본문(`strings.rs::mail::
  GAME_MAIL_SUBJECT/BODY`, 3개 언어)과 함께 **메인 게임 설치 파일 `test
  Setup.exe`가 첨부**돼 있다(STORY.md 7-1절) — 메일로 오는 게임은 항상 이렇게 Setup
  파일로 온다. 첨부는 fs 안의 `FileKind::GameSetup` 노드(`GAME_SETUP_NAME`)로,
  `FileSystem::ensure_game_attachment()`가 Mail 노드의 `attachment`에 걸어둔다 —
  새 게임은 물론 이 첨부가 생기기 전의 예전 저장 파일을 불러와도 그 자리에서 채워
  넣는다(잠깐 있었던 "바로 실행되는 DOORS.exe" 첨부가 걸린 저장 파일이면 Setup 으로
  바꿔주고, 게임 이름이 DOORS → test 로 바뀌기 전의 저장 파일이면 파일 이름도 맞춰준다). "Download"를 누르면 평소처럼 Downloads 에 생기고(한 번 받은 적 있으면
  `ever_downloaded` 기준으로 창을 다시 열어도 "Downloaded" 그대로), 그걸 열면
  **설치 마법사**(`apps/game_installer.rs::GameInstallerApp`, 예전 팩맨 때 마법사를
  되살림)가 뜬다: Welcome → Installing(들쭉날쭉한 진행바, 취소 불가) → Finish.
  진행바가 다 차는 순간 `AppAction::InstallComplete` → desktop.rs 가
  `fs.game_installed`를 켜고 `add_desktop_icon()`으로 바탕화면 빈 칸에 게임 아이콘
  `test.exe`(`FileKind::Game`)를 만든다. 동시에 진짜 컴퓨터의
  **`%APPDATA%\test\`에 게임 파일을 써 넣는다**(`gamefiles.rs`, 아래 "게임 설치 연출" 참고).
  이미 설치된 뒤 Setup 을 다시 열면 곧장
  "이미 설치됨" 페이지만 뜬다(아이콘 중복 방지). 바탕화면의 `test.exe`를 열면
  **PalaceOS 안의 창 하나**로 게임(`apps/doors_game/`, 아래 "메인 게임" 절)이
  뜬다. 아이콘은 새로 그린 `assets/icon_setup.png`(`IconType::Setup`, 상자+디스크+
  화살표)와 `assets/icon_exe.png`(`IconType::Exe`, 프로그램 창 + 문).
- **Write Mail**: To/Subject/Body 실제 텍스트 입력(커서 클릭 이동 포함), Desktop/Downloads
  파일 첨부(다중), Send는 To·Body가 채워져야 활성화. 한글/일본어 IME 조합을 지원(백스페이스로
  조합 중 자모 하나만 지우기 등, `ime.rs` 참고).
- 보낸 메일은 Sent Items에 그대로 쌓인다.

### 그 외 파일별 앱

- **메모장**(`.txt`): 워드랩 + 스크롤 뷰어(읽기 전용).
- **비디오 플레이어**(`.mp4`): Media Foundation 디코딩 + WASAPI 오디오, 재생/일시정지/탐색
  (±5초, 클릭·드래그 스크럽), 볼륨/풍화(Weathering) 열화 효과가 설정 변경에 실시간 반응.
- **이미지 뷰어**: 레터박스 뷰어(읽기 전용) — [STORY.md](STORY.md) 챕터 1용 콘텐츠가 들어갈
  자리, 지금은 표시 기능만 있다.
- **설정**: Graphics(해상도/프레임레이트, CRT 강도/색수차/커서 크기) · Audio(SFX/BGM/Mp4/
  Master 볼륨, Weathering, Mute All) · Interface(부드러운 스크롤, 언어, 배경색) 세 탭.
  언어 변경은 즉시 저장되고 열린 모든 창에 실시간 반영된다.
- **Credits**, **Official Site**(WebView2를 오프스크린 렌더 + 주기적 캡처로 텍스처화해서
  CRT 파이프라인을 그대로 통과시킨다 — 실제 클릭/스크롤/타이핑 가능), **비밀번호 대화상자**
  (`.lock` 파일용).

### 진짜 3D 메쉬 엔진 (`render/mesh3d.rs`) + 메인 게임

진짜 깊이 테스트가 있는 GPU 삼각형 래스터라이저 — 이 게임의 유일한 1인칭 렌더링
엔진이다.

- **`Box3D`(유일한 지오메트리 타입)**: 중심/반너비/회전(yaw·pitch·roll) 하나로
  벽도 바닥도 경사로도 전부 표현한다 — 회전이 없으면 흔한 축정렬 상자, yaw 만
  있으면 "비스듬히 놓인 벽", pitch/roll 이 있으면 "기울어진 벽"이나 "경사로"가
  된다. **경사로는 특수 케이스가 아니라 그냥 회전된 상자의 로컬 윗면**이다 —
  바닥 높이 탐색(`ground_height`)이 상자의 로컬 +Y면(윗면) 두 삼각형에 레이를
  쏴서 걸을 수 있는 높이를 구하는데, 상자가 기울어 있으면 그 삼각형 자체가
  기울어 있으니 교차점 높이가 경사를 자동으로 따라간다.
- **렌더링**: `Box3D::to_triangles()`가 CPU 에서 상자를 월드 공간 삼각형 12개
  (면마다 2개)로 펼치고, 텍스처별로 묶어 miniquad 파이프라인(깊이 테스트 켬,
  `Comparison::LessOrEqual`)으로 오프스크린 컬러+깊이 렌더 타깃에 그린다. 그
  결과 컬러 텍스처를 `gfx::Renderer::sprite_uv`로 보통 창처럼 2D 오버레이(HUD
  등) 위에 합성한다(video.rs 가 디코딩한 영상 프레임을 텍스처로 올리는 것과
  같은 요령) — CRT 파이프라인은 그 합성된 최종 화면 전체에 이미 그대로 걸린다.
- **텍스처**: `BoxTexture` — 가로 4칸×세로 3칸 십자 전개도 한 장으로 상자 6면을
  입힌다. 프래그먼트 셰이더가 텍스처 알파값이 0.5 미만이면 그 프래그먼트를
  `discard`한다(cutout) — 이 3D 파이프라인은 블렌딩 없이 불투명하게만 그리기
  때문에, 알파를 무시하고 그냥 곱하면 완전히 투명해야 할 픽셀 자리에 PNG 에
  박혀있는 RGB 값(보통 검정)이 그대로 불투명하게 칠해진다. 텍스처 없는 단색
  상자는 알파=1 인 1x1 흰 텍스처를 쓰므로 이 컷아웃에 안 걸린다.
- **충돌/바닥 높이**: `resolve_horizontal`은 플레이어를 상자의 로컬 좌표로 옮겨
  AABB 클램프한 뒤 되돌리는 "가장 가까운 점" 방식이라 회전된 상자에도 그대로
  맞는다. 발이 상자의 로컬 윗면 높이 근처거나 그 위(경사로를 오르는 중 포함)면
  수평 충돌 대상에서 제외해서, 경사로를 오르내리는 동안 그 경사로 자체에
  옆으로 밀려나지 않는다. `walkable`(바닥/경사로로 쓰이는지)과 `solid`(수평
  충돌에 끼는지)를 상자마다 따로 켤 수 있어서, 밟고 지나가야 하는 얇은 경사로는
  `solid=false`로, 위로 못 올라가야 하는 장식 벽은 `walkable=false`로 둔다.
- **`src/apps/game3d/` — 창 안 3D 게임 공용 도구**: `View`(창 안에 4:3 으로 맞춰
  3D 장면/조준선/안내를 그림), `MouseLook`(클릭하면 시점 모드), `player.rs`(1인칭 이동·충돌·
  조준 판정), `dialogue.rs`(타자기 대화창 + Y/N 선택지 + 입력 처리). doors_game 과
  maze_game 이 같이 쓴다.
- **`src/apps/maze_game/` — 서브 게임 "B+a"**(`test2.exe`, 시트의 SUB A+a-1 ~ 14): `mod.rs`가
  진행 상태(`Progress`)와 "지금 있는 곳"(`Place::Maze`/`Hub`)을 들고, 조사할 수 있는 물건
  (`Item` — `Box3D` 로 그려지고 충돌도 있다)을 진행 상태에 따라 매번 새로 계산한다(`items()`).
  흐름: ① 미로(`maze.rs` — 열 때마다 5x5 셀 완전 미로를 깊이 우선 탐색으로 새로 만들고, 도착
  칸 = 출발에서 가장 먼 칸)를 걸어 물뿌리개를 줍는다("1라운드 클리어") → ② 대화가 끝나면
  스테이지 1 방(`hub.rs`)으로 올라온다. 화분을 조사하면("이미 누가 꺾어간 것 같다 / 새롭게
  심어야 할 것 같은데...") 그 뒤에 사물함이 나타나고, 사물함을 조사하면("열쇠 3개가 필요할 것
  같다") 북쪽 문 3개가 열린다(어두운 색 → 밝은 색) → ③ 문 i 를 조사하면 새 미로로 들어가고,
  도착 지점의 열쇠 i 를 주우면 밧줄이 내려오고, 밧줄을 조사하면 방으로 돌아온다(이미 다녀온
  문은 안 들어가진다) → ④ 열쇠 3개를 모으면 사물함에서 씨앗을 얻고(모자라면 "열쇠가
  부족하다 (n/3)"), 화분을 조사하면 심어지면서 "시간이 1년은 필요할 것 같다..."가 나온다.
  방/미로 전환은 `pending_move`로 대화가 다 끝난 뒤에 한다. 왼쪽 위에 가진 것(물뿌리개/열쇠
  n/3/씨앗)이 뜬다. 단위 테스트 두 개: 미로는 어떤 시드든 도착까지 길이 있는지, 진행
  전체(`full_progression`)가 시트 순서대로 이어지는지. **다음 이벤트**(CRT 모니터에서 시간
  조작으로 꽃이 핌 → 꽃 획득)는 아직 없다. 진행 상태는 저장하지 않는다(창을 닫으면 처음부터).
- **`src/apps/doors_game/` — 메인 게임 "DOORS"**(`mod.rs` = 창 앱/입력, `world.rs` = 맵·
  물건·조준 대상; 화면/시점/플레이어/대화창은 위 game3d): 메일 첨부 `test Setup.exe`로
  설치해 바탕화면에 생긴 `test.exe`(`FileKind::Game`)를 열면 OS 안의 창 하나로 뜨는 앱(`DoorsGameApp`, 별도 실행
  파일이 아니다). 3D 장면은 `mesh3d.rs`로 640x480 오프스크린에 그려서 창 안에 4:3
  으로 끼워 넣고(남는 곳은 검은 띠 — 그래서 창 크기 조절/최대화 자유), HUD/대화창도
  같은 배율로 따라 커진다. CRT 는 바깥 OS 화면에 이미 걸려 있어 따로 안 입힌다.
  `Mesh3D`는 닫을 때 GPU 자원을 지울 방법이 없어(App 에 Drop 시점 ctx 가 없음)
  스레드 전역에 하나만 만들어 재사용한다.
  **마우스 시점**: 게임 화면을 클릭하면 시점 모드 — 앱이 매 프레임
  `scenes::request_mouse_look()`을 부르는 동안 main.rs 가 실제 커서를 화면
  가운데로 계속 되돌리며 움직인 양만 `Input::look_delta`에 쌓아주고, 가상 커서는
  멈춘 채 안 그린다. `Esc`/다른 창 포커스/창 닫기·최소화로 앱이 요청을 멈추면 다음
  프레임에 저절로 풀린다. 이동 W/S/A/D, 상호작용 E(창이 포커스일 때만).
  **맵**: 바닥/천장/벽 4면으로 사방이 막힌 방(`build_room()`), 플레이어 정면(-Z) 벽
  가운데 닫힌 문 하나(문간 좌우 벽 조각 + 상인방으로 빈틈 없음). 그 문이 도어즈 NPC —
  손잡이를 조준하고 `E`를 누르면 "보리지꽃을 줘." 한마디와 `[Y]/[N]` 선택지만
  나온다. 지금은 수락하면 그냥 "수락됨"(`quest_accepted`)으로 기록만 되고(꽃을 구하는
  미니게임/보상은 아직 없다) 그 뒤로는 더 말을 걸 수 없다. 거부하면 그냥 닫히고 다시
  말을 걸 수 있다.
  **방 물건 조사**(`world.rs::PROPS` — 책상/상자/선반/쓰레기 더미, 각자 충돌 있는
  `Box3D`): 조준선을 가까이 대면 "[E] Examine 이름"이 뜨고(`aimed_target()`이 문
  손잡이와 물건들 중 가장 가까운 것 하나를 고른다), `E`로 그 물건의 한 줄을 보여준다.
  도어즈의 부탁을 수락한 뒤 처음 조사하면 이어서 "방에는 꽃이 없는 것 같다..."가
  나오고 `flower_absence_checked`가 켜진다 — 시트(이벤트 흐름표)의 MAIN A-3 이고,
  두 번째 메일의 조건이다 — 그 대화가 다 끝나면 게임 앱이 `AppAction::
  FlowerAbsenceChecked`를 한 번 돌려주고, desktop.rs 가 `fs.mail2_arrived`를 켜서
  **두 번째 메일**을 도착시킨다(첫 메일 때처럼 토스트 알림 + 열려있는 Mail 새로고침 +
  즉시 저장). 발신자 이름이 깨져 있고(`SECOND_MAIL_FROM` — 폰트에 없는 U+FFFD 를 렌더러가
  마름모로 그린다) 제목/본문은 "내가 꽃을 구할 수 있는 곳을 알고 있어"(3개 언어)다. 받은
  편지함은 `fs.mail_arrived_count()`만큼 도착 순서대로 앞에서부터 보여준다. 첨부/두 번째
  게임은 아래 "서브 게임" 참고.
  **다운로드 완료 + `test2.exe`**(시트의 CRT B-0): 두 번째 메일이 온 뒤 `SUB_GAME_DELAY`(6초)
  뒤에 "다운로드 완료" 알림(`Toast`, 누르면 그냥 닫힘 — 새 메일 알림은 누르면 Mail 을 연다)이
  뜨면서 바탕화면 빈 칸에 서브 게임 아이콘 `test2.exe`(`FileKind::SubGame`,
  `fs.sub_game_ready`로 한 번만)가 생긴다. 이름은 임시다. 두 번째 메일만 오고 종료했어도
  다음 실행에서 이어서 센다.
  창을 닫으면 게임 진행은 처음부터(아직 저장 안 함). 문은 열리지 않는다.

### 게임 설치 연출 (`gamefiles.rs`)

설치 마법사가 끝나는 순간(`InstallComplete`) 진짜 컴퓨터의 `%APPDATA%\test\`(폴더 이름 =
`GAME_FOLDER_NAME`, 게임 이름)에 "정말 설치된 크랙 게임"처럼 보이는 파일들을 실제로 쓴다 —
`README.txt`, 크랙 그룹(R18) 스타일 `test-CRACKED.nfo`, `config/game.ini`, `logs/install.log`,
디컴파일된 듯한 소스 `src/{main,doors,maze}.rs`(도어즈는 꽃을 원한다, 시간이 필요하다 같은
단서가 주석에 숨어 있다), 이미지 `assets/{door,flower_pot,static}.png`, 사운드
`assets/sound/{ambience,door_creak}.wav`, 빈 `saves/`. 이진 에셋은 따로 두지 않고 설치할 때
코드로 만든다(`image` 크레이트로 PNG, 16비트 PCM WAV 는 직접 헤더를 씀). **게임은 이 파일들을
읽지 않는다** — 플레이어가 탐색기로 열어봤을 때를 위한 순수 연출(ARG)용이다. 설치 마법사의 끝
페이지에 설치 위치(`%APPDATA%\test`)가 나온다. 안전장치: 우리가 만든 폴더라는 표식
(`.palaceos`)이 있는 폴더만 덮어쓰고 지운다 — 같은 이름의 남의 폴더가 이미 있으면 아무것도
안 건드린다. "Erase All Memory"(`foundation::delete`)도 이 폴더를 같이 지운다. 단위 테스트가
임시 폴더에서 설치→파일 확인(PNG 디코드/WAV 헤더)→삭제와 남의 폴더 보호를 검사한다.

### 로컬라이제이션

- 영어/한국어/일본어 3개 언어. 모든 UI 문자열은 `strings.rs`에 `S { en, ko, ja }` 상수로
  선언되어 있어 번역 누락이 컴파일 에러가 된다(기능별 하위 모듈로 정리됨).
- 비트맵 폰트 아틀라스는 ASCII + 완성형 한글 + 한글 자모 + 히라가나/가타카나 + 정해둔
  한자 화이트리스트(전체 유니코드 CJK가 아니라 실제 쓰이는 한자만 큐레이션)를 지원한다.
  아틀라스에 없는 글자(자리표시자로 일부러 쓰는 키릴 문자 등)는 "깨진 글자"(마름모+물음표)
  자리표시자로 그려진다.
- 스토리 스포일러는 `secrets.rs`에 따로 분리해뒀다.

### 그래픽 · CRT 효과

- 640×480 가상 해상도에 배칭 2D 렌더러(`gfx.rs`)로 그린 뒤, 오프스크린 텍스처를 CRT
  후처리 셰이더(`crt.rs`)로 화면에 4:3 레터박스로 합성한다.
- 효과: 오버스캔, 배럴 곡률, 색수차, 스캔라인 + 섀도마스크풍 RGB 변조, 비네트 — 강도는
  설정의 CRT Intensity 슬라이더 하나로 같이 조절된다.
- 커스텀 소프트웨어 마우스 커서(스프라이트시트 기반, 방향별 리사이즈 커서 포함).

### 저장 시스템

- 진행 상황은 실행 파일 옆 `palaceos_save.json`(설정/가상 파일 시스템 전체/아이콘 위치/
  창 배치)에, 그래픽·언어 설정은 `palaceos_settings.json`에 따로 저장된다(포터블, 레지스트리
  미사용). 새 필드는 `#[serde(default)]`로 추가해 예전 저장 파일과 호환을 유지한다.
- 5초마다 자동 저장 + 상태가 바뀌는 즉시(다운로드/이동/삭제/발신 등) 추가로 저장한다.

### 개발자 도구 (Director / Director Panel)

플레이어에게는 안 보이는, 트레일러/스크린샷 녹화용 별도 실행 파일 두 개.

- **`director`**: 게임과 같은 씬/렌더러를 공유하지만 조작 UI가 전혀 안 찍히는 독립 창.
  글리치/노이즈 오버레이, 씬 전환, 화면+시스템 오디오(WASAPI 루프백) 녹화 → PNG 시퀀스 +
  wav를 MJPEG-AVI(`output.avi`, 4GB 넘으면 자동 분할)로 자동 합성.
- **`director_panel`**: 그 옆에 따로 뜨는 조작 창 — 씬 전환·글리치/노이즈 강도·녹화 버튼.
- 두 프로세스는 `director_state.json`을 통해서만 통신한다(`director_ipc.rs`) — 실제
  게임(`crackhead.exe`)은 이 모듈을 아예 안 쓴다.

## 구조

```
src/
├── main.rs             # 진입점: Stage(EventHandler) + 프레임 루프
├── lib.rs              # bin/*.rs(director/director_panel)가 공유하는 라이브러리 진입점
├── foundation.rs       # 가짜 파일 시스템(FileSystem/FileId/FileKind) + Settings + 저장/불러오기
├── ui.rs               # 9x 위젯(베벨/버튼/체크박스/아코디언/아이콘) + 커서
├── strings.rs          # 다국어(en/ko/ja) 문자열 테이블
├── gamefiles.rs        # 설치 연출 — %APPDATA%\test\ 에 가짜 게임 파일(소스/이미지/사운드) 쓰기
├── random.rs           # 연출용 xorshift 난수(Rng) + 들쭉날쭉한 로딩 바 곡선(LoadCurve)
├── secrets.rs          # 스토리 스포일러 상수(따로 분리)
├── window_manager.rs   # 창 관리자 (z순서, 드래그, 크기조절, 타이틀바 버튼)
├── director_ipc.rs     # director/director_panel 간 JSON IPC (게임은 안 씀)
├── render/             # 그리기
│   ├── gfx.rs             # 2D 배칭 렌더러 + 비트맵 폰트 아틀라스 + 에셋 로딩
│   ├── crt.rs             # 오프스크린 렌더 타깃 + CRT 셰이더(곡률/스캔라인/색수차) + 4:3 필러박스
│   └── mesh3d.rs          # 진짜 3D 메쉬 렌더러(GPU 깊이테스트) + Box3D/OBJ 메시 + 충돌/바닥높이
├── platform/           # Windows 전용 기능 래퍼
│   ├── video.rs           # mp4 디코딩(Media Foundation) + 오디오 재생(WASAPI) + Weathering
│   ├── webview.rs         # Official Site: WebView2 오프스크린 렌더 + 주기적 캡처 → 텍스처
│   └── ime.rs             # 한/일 IME 조합 타이밍/팝업 위치 우회(Win32 IMM32)
├── apps/               # 파일별 앱 — 새 앱은 파일 하나 + mod.rs 한 줄
│   ├── mod.rs             # App 트레잇 / AppAction / Opened + open() 파일→앱 매칭
│   ├── widgets.rs         # 여러 앱이 같이 쓰는 위젯(아이콘 격자/슬라이더/스크롤바)
│   ├── game3d/            # 창 안 3D 게임 공용 도구(View/MouseLook/Player/Dialogue)
│   ├── doors_game/        # 메인 게임 DOORS — OS 창 안에서 도는 3D 게임(mod/world)
│   ├── maze_game/         # 서브 게임 test2.exe — 미로/스테이지 1 방/열쇠·씨앗(mod/maze/hub)
│   ├── game_installer.rs  # 메일 첨부 test Setup.exe 설치 마법사 → 바탕화면에 test.exe
│   ├── notepad.rs, video_player.rs, image_viewer.rs, mail.rs, explorer.rs,
│   │   recycle_bin.rs, password.rs, credits.rs, settings.rs, official_site.rs
└── scenes/              # 화면 전체를 차지하는 씬 — 새 씬은 파일 하나 + mod.rs 한 줄
    ├── mod.rs              # Scene 트레잇 / Transition / SceneManager / Frame / Input
    ├── lobby.rs, boot.rs, desktop.rs, shutdown.rs, erase.rs, bluescreen.rs
src/bin/
├── director.rs          # 녹화용 게임 화면 창(별도 실행 파일)
└── director_panel.rs    # 그 옆의 조작 창(별도 실행 파일)
```

의존 방향: `render`/`platform`/`foundation`/`strings`/`secrets`는 서로 독립적인 기반
레이어이고, `ui` → `apps` → `window_manager`/`scenes` → `main` 순으로 위 계층이 아래를
참조한다.

**좌표계**: 씬/UI는 가상 해상도 **640×480**에서 절대 좌표로 그리고, 마우스도 그 좌표계로
받는다 — CRT 단계가 이걸 화면에 4:3으로 확대해 보여준다.

## 알려진 제한사항

- 기획이 갈아엎이면서 예전에 있던 "메일로 HexTool 설치 마법사를 받아 ????? 사진을
  검수하는" 라인(자동 도착 메일, HexTool, 사진 검수 도구)을 통째로 걷어냈다. 그 뒤에
  만들었던 "메일로 게임 설치 파일을 받아 팩맨류 게임을 플레이하는" 라인(메일 도착 →
  다운로드 → 설치 마법사 → 바탕화면 설치 → 레이캐스팅 미로/코인)도 미니게임 자체와
  함께 다시 걷어냈다 — 지금 메일에는 그 대신 메인 게임 설치 파일 `test Setup.exe`가 첨부된다
  (위 "메일" 절 참고).
- 게임류 창을 추가할 땐 따로 이야기 없으면 항상 리사이즈/최대화가 꺼진 고정 크기로
  여는 게 기본값이다(`apps/mod.rs::open()`) — 아케이드 게임 화면이 창 크기에 따라
  늘어나 보이는 걸 막기 위함.
- 작성 중인 메일에 첨부한 파일을 보내기 전에 삭제/이동하면, 이미 첨부된 항목은 자동으로
  갱신되지 않는다.
- 이미지 뷰어/일부 스토리 콘텐츠는 [ROADMAP.md](ROADMAP.md)의 챕터 진행에 맞춰 계속
  교체될 예정인 플레이스홀더다.

세부 변경 이력은 이 문서 대신 `git log`를 참고할 것.
