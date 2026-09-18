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
- `cargo run --bin raycaster_test`로 재사용 1인칭 3D 레이캐스팅 엔진(`raycaster.rs`)만
  따로 띄워볼 수 있다 — 실제 게임과 무관한 독립 창(자세한 건 "1인칭 3D 레이캐스팅
  엔진" 절 참고).

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
  Mail 을 연다(`DesktopScene::update_toast`). 제목/본문은 지금 일부러 비워뒀고
  (`(게임 이름) Setup.exe` 첨부만 걸려있다, 예: `Pacman Setup.exe`) — 지금은 "메일 →
  다운로드 → 설치 마법사 → 바탕화면에 설치" 파이프라인만 만드는 단계라 내용은 다음
  기획 확정 때 채운다. 첨부를 다운로드해도(예전과 달리) 자동으로 열리지 않고,
  Downloads 탭에서 직접 더블클릭해야 열린다 — 처음 열 때는(그 게임이 아직
  `fs.installed_games`에 없으면) 옛 HexTool Setup.exe와 같은 요령의 설치 마법사
  (`apps/game_installer.rs::GameInstallerApp` — Welcome → Installing(들쭉날쭉한
  가짜 진행바) → Finish, 약관 페이지는 뺐다)가 뜨고, 진행바가 다 차는 순간 그
  게임을 설치된 것으로 기록하면서 바탕화면에 새 아이콘("(게임 이름).exe", 예:
  `Pacman.exe`)을 만든다. 그 아이콘을 열면 곧장 실제 게임 창(`apps/pacman.rs::
  PacmanApp`)이 뜬다. Setup.exe 자체를 나중에 또 열면(예: Downloads 에 남아있던
  첨부를 다시 눌러본 경우) 마법사를 다시 태우지 않고 "이미 설치됨" 페이지로
  곧장 연다. 새 게임을 추가할 땐 `GameKind`에 variant 하나, `apps/mod.rs::open()`의
  `GameInstalled` 분기에 그 게임 앱을 고르는 한 줄만 더하면 되도록 설계해뒀다.
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
- **팩맨**(`apps/pacman.rs::PacmanApp`, 설치된 `(게임 이름).exe`를 열면 뜬다): `raycaster.rs`
  (아래 참고)로 미로를 그리는 1인칭 버전 — 텍스처/유령은 아직 없다. W/S 로 바라보는
  방향 기준 앞/뒤 이동, A/D 로 좌우 회전(스트레이프 없음). 창이 포커스를 잃으면 키
  입력을 아예 안 읽는다.
  - **라운드 5개**(`ROUND_MAP_SIZES` 상수, 맵 크기 4/5/7/9/14): 1~4라운드는 매번
    `raycaster::generate_maze`(랜덤 Prim)로 미로를 새로 생성해서 플레이할 때마다
    다르게 나오고, 5라운드만 고정 시드(`ROUND5_SEED`)로 생성해 항상 같은 미로가
    나온다. `map_size`는 방 격자 한 변의 방 개수(1~14) — 실제 격자는 (2×map_size+1)
    칸이다. 코인은 시작 칸을 뺀 바닥 칸 전부에 놓는다(`place_coins`) — 그 라운드의
    coins_total 은 맵 크기에 따라 자연히 정해진다. 시작 방향은 시작 칸에서 실제로
    뚫려있는 방향(동→남→서→북 순서로 검사, `Raycaster::face_open_direction`)을
    찾아 그쪽을 보게 한다 — 미로 생성 결과 시작 칸 동쪽이 벽인 경우도 흔해서,
    안 그러면 벽을 마주보고 시작하는 경우가 있었다.
  - 코인 칸을 밟으면 즉시 먹고, 그 라운드에 놓인 코인을 전부 먹으면 화면이 검은
    배경 + "Round Clear"로 2초간 바뀌었다가(`ROUND_CLEAR_HOLD`) 다음 라운드로
    넘어간다(5라운드를 깨면 1라운드로 되돌아간다 — "올 클리어" 화면은 아직 없다).
  - 화면 위쪽 검은 띠에 "(현재 라운드)Round (그 라운드에서 먹은 코인)/(그 라운드
    전체 코인)"이 흰 글씨로 뜬다(코인 자체는 계속 노란 원). 게임 내 텍스트(HUD,
    "Round Clear")는 전부 흰색으로 통일했다.

### 1인칭 3D 레이캐스팅 엔진 (`raycaster.rs`)

팩맨을 만들면서 생긴 "벽 DDA 레이캐스팅 + 충돌 이동 + 미로 생성 + 빌보드(작은
물체) 렌더링"을 게임 고유 규칙(라운드/코인/HUD)과 분리해 재사용 모듈로 뽑아뒀다 —
앞으로 미로/복도 구조의 1인칭 3D 미니게임을 더 추가할 때 이 모듈 하나로 엔진
부분을 공유한다.

- `Raycaster`: 그리드 미로(`walls: Vec<Vec<bool>>`) + 플레이어 위치/각도를 들고,
  `is_wall`/`try_move`/`apply_wasd`(W/S/A/D 네 방향 bool 을 받아 이동/회전에 그대로
  반영)/`face_open_direction`/`render_walls`(천장·바닥·벽 세로띠, 컬럼별 깊이를
  반환)/`render_props`/`render_billboards` 메서드를 제공한다. 입력을 어디서
  읽어오는지(윈도우 매니저 안 앱이든, 독립 실행 창이든)는 신경 쓰지 않고 bool
  네 개만 받으므로 두 상황 모두에 그대로 쓴다.
- `generate_maze` (랜덤 Prim 알고리즘): 재귀 백트래커(한 번 뚫은 방향으로 갈 수
  있는 데까지 쭉 파고들어서 길게 뻗은 복도가 되기 쉽다)보다 짧은 막다른 길/급한
  방향 전환이 훨씬 많이 나와서 미로가 매판 더 꼬여 보인다.
- `Billboard` + `render_billboards`: 코인처럼 완전히 둥근 물체 — 항상 카메라를
  향하는 평면(스프라이트) 하나로 그린다. 둥근 물체는 어느 각도에서 봐도 실루엣이
  원이라 이걸로 충분하다.
- `Prop3D` + `render_props`: 책상/의자처럼 각진 물체는 빌보드로 그리면 옆에서 봤을
  때 납작하게 보이는 게 뻔히 드러나서, 대신 축 정렬 상자(중심/가로·세로·높이/바닥
  에서 띄운 높이)를 **진짜 입체**로 그린다 — 플레이어가 상자 바깥쪽에 있는 면만
  골라(동/서/남/북 중 최대 2면) 벽과 똑같은 컬럼별 레이-평면 교차 방식으로
  그리므로, 실제로 옆으로 돌아가면 옆면이 보인다. 그려진 컬럼만큼 `col_depth`
  를 직접 갱신해서 벽/다른 물체와도 정확히 가려진다 — `render_walls` 직후,
  `render_billboards` 이전에 불러야 한다.
- `src/bin/raycaster_test.rs`: 이 엔진만 따로 띄워서 확인하는 가장 작은 테스트
  창(`cargo run --bin raycaster_test`) — 팩맨의 라운드/코인 규칙 없이 미로 하나를
  생성하고, 바닥 칸 두 곳엔 책상(`Prop3D` 하나)과 의자(좌판 + `base`로 띄운 등받이,
  `Prop3D` 둘)를 진짜 입체로, 나머지 바닥 칸엔 원형 마커(`Billboard`)를 놓은 뒤
  WASD 로 걸어 다니며 벽 충돌/레이캐스팅/입체 물체/빌보드가 서로 잘 가려지는지
  확인한다. R 키로 새 미로를 다시 생성한다.

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
├── gfx.rs              # 2D 배칭 렌더러 + 비트맵 폰트 아틀라스 + 에셋 로딩
├── crt.rs              # 오프스크린 렌더 타깃 + CRT 셰이더(곡률/스캔라인/색수차) + 4:3 필러박스
├── ui.rs               # 9x 위젯(베벨/버튼/체크박스/아코디언/아이콘) + 커서
├── strings.rs          # 다국어(en/ko/ja) 문자열 테이블
├── secrets.rs          # 스토리 스포일러 상수(따로 분리)
├── video.rs            # mp4 디코딩(Media Foundation) + 오디오 재생(WASAPI) + Weathering
├── webview.rs          # Official Site: WebView2 오프스크린 렌더 + 주기적 캡처 → 텍스처
├── ime.rs              # 한/일 IME 조합 타이밍/팝업 위치 우회(Win32 IMM32)
├── window_manager.rs   # 창 관리자 (z순서, 드래그, 크기조절, 타이틀바 버튼)
├── director_ipc.rs     # director/director_panel 간 JSON IPC (게임은 안 씀)
├── raycaster.rs        # 재사용 1인칭 3D 레이캐스팅 엔진(DDA/이동/미로 생성/빌보드) — 팩맨이 씀
├── apps/               # 파일별 앱 — 새 앱은 파일 하나 + mod.rs 한 줄
│   ├── mod.rs             # App 트레잇 / AppAction / Opened + open() 파일→앱 매칭
│   ├── widgets.rs         # 여러 앱이 같이 쓰는 위젯(아이콘 격자/슬라이더/스크롤바)
│   ├── notepad.rs, video_player.rs, image_viewer.rs, mail.rs, explorer.rs,
│   │   recycle_bin.rs, password.rs, credits.rs, settings.rs, official_site.rs,
│   │   game_installer.rs, pacman.rs
└── scenes/              # 화면 전체를 차지하는 씬 — 새 씬은 파일 하나 + mod.rs 한 줄
    ├── mod.rs              # Scene 트레잇 / Transition / SceneManager / Frame / Input
    ├── lobby.rs, boot.rs, desktop.rs, shutdown.rs, erase.rs, bluescreen.rs
src/bin/
├── director.rs          # 녹화용 게임 화면 창(별도 실행 파일)
├── director_panel.rs    # 그 옆의 조작 창(별도 실행 파일)
└── raycaster_test.rs    # raycaster.rs 만 따로 확인하는 최소 테스트 창(별도 실행 파일)
```

의존 방향: `gfx`/`crt`/`foundation`/`video`/`strings`/`secrets`는 서로 독립적인 기반
레이어이고, `ui` → `apps` → `window_manager`/`scenes` → `main` 순으로 위 계층이 아래를
참조한다.

**좌표계**: 씬/UI는 가상 해상도 **640×480**에서 절대 좌표로 그리고, 마우스도 그 좌표계로
받는다 — CRT 단계가 이걸 화면에 4:3으로 확대해 보여준다.

## 알려진 제한사항

- 기획이 갈아엎이면서 예전에 있던 "메일로 HexTool 설치 마법사를 받아 ????? 사진을
  검수하는" 라인(자동 도착 메일, HexTool, 사진 검수 도구)을 통째로 걷어냈다 — 새 기획인
  "메일로 게임 설치 파일을 받아 팩맨류 게임을 플레이하는" 방식은 메일 도착 → 다운로드 →
  설치 마법사 → 바탕화면 설치 → 레이캐스팅 미로/코인까지는 만들어져 있고, 유령/충돌
  판정에 따른 게임오버·점수 저장 같은 나머지 팩맨 규칙은 아직 없다.
- 게임류 창(팩맨 등)은 따로 이야기 없으면 항상 리사이즈/최대화가 꺼진 고정 크기로
  연다(`apps/mod.rs::open()`) — 아케이드 게임 화면이 창 크기에 따라 늘어나 보이는 걸
  막기 위한 기본값.
- 작성 중인 메일에 첨부한 파일을 보내기 전에 삭제/이동하면, 이미 첨부된 항목은 자동으로
  갱신되지 않는다.
- 이미지 뷰어/일부 스토리 콘텐츠는 [ROADMAP.md](ROADMAP.md)의 챕터 진행에 맞춰 계속
  교체될 예정인 플레이스홀더다.

세부 변경 이력은 이 문서 대신 `git log`를 참고할 것.
