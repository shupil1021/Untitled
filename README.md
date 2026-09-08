# PalaceOS (CrackHead)

Rust + [miniquad](https://github.com/not-fl3/miniquad) 로 만드는, **가짜 Windows 9x 데스크톱 OS**.
Windows 9x / NT 4.0~2000 룩앤필 + CRT 모니터 느낌(화면 곡률·스캔라인·색수차).
지금은 데스크톱 토이지만, 최종 목표는 이 위에 얹을 ARG/아날로그 호러 게임의 셸이다 —
그래서 부팅/종료/리셋 같은 "시스템 자체가 살아있는" 연출에 공을 들인다.

## 기능

- **로비 화면**: 앱 실행 시 가장 먼저 뜨는 타이틀 화면 — 로고 뒤로 정전기 노이즈 + 이따금
  튀는 글리치, 아래에 New Start/Continue(저장 데이터 있을 때만 활성)/Settings/Quit 메뉴.
  Quit 을 누르면 바로 안 꺼지고 화면이 거의 암전 수준으로 어두워지며 시안/마젠타로
  어긋난 조각 + 검은 틈이 겹친 "찢어지는" 글리치가 계속되는 채로, 창 대화상자
  대신 로비 메뉴와 같은 언어로 "Really quit PalaceOS?" 문구 아래 Yes/No 를 띄운다
- **부팅 화면**: BIOS POST(메모리 테스트 + 장치 인식 줄) → 로고/Welcome/로딩 바 → 데스크톱
- **바탕화면**: 아이콘 드래그(격자 스냅, 다중 선택 고무줄 박스), 빈 화면 우클릭 컨텍스트 메뉴.
  파일 아이콘은 Windows 98 아이콘 팩에서 뽑은 32x32 PNG 텍스처(`assets/icon_*.png`)로 그린다.
  Tar/Installer 는 텍스처 대신 직접 그리는 벡터 도형(`draw_tar_icon`/`draw_installer_icon`)
  이라 s×s 상자 안에서 여백을 두고 작게 그려져 다른 텍스처 아이콘들보다 눈에 띄게
  작아 보였다 — `DesktopScene::draw_one_icon` 에서 이 둘만 기본 크기(`IC_SIZE`)의
  1.3배로 키워서 그린다. 다만 실제로 그려지는 크기가 얼마든 아이콘의 세로 중심과
  그 아래 글자 시작 위치는 항상 고정된 기본값(`ICON_AREA_TOP`/`ICON_BASE_SIZE`/
  `LABEL_GAP`)만 기준으로 잡는다 — 예전엔 텍스트 y 좌표가 실제 그려지는 아이콘
  크기에 딸려 있어서, Tar/Installer 처럼 큰 아이콘을 쓰면 글자가 그만큼 아래로
  밀려 이름이 두 줄로 접혔을 때 바로 아랫줄 아이콘과 겹쳐 보였다. 이 기본값들로
  타일 높이(`TILE_H`)도 실제 필요한 만큼(아이콘 영역 + 글자 최대 2줄 + 여백)
  계산해서 정하는데, 이전(64px)보다 커져서 세로로 들어가는 줄 수가 6줄에서
  5줄로 줄었다(9열×5행=45칸). 세로 겹침을 고친 뒤에도 "Explorer"/"Setup.exe"
  처럼 공백 없이 긴 이름은 옆 칸 글자와 겹쳐 보이는 문제가 남아있었다 —
  `wrap_two_lines` 에 넘기던 최대 너비가 `IC_W+6`(70px)이었는데, 정작 옆
  타일까지의 실제 간격은 `TILE_W`(66px)뿐이라 "다 감싸졌다고 판단된" 글자도
  타일 폭보다 넓게 그려질 수 있었다(70px짜리 글자가 64px 아이콘 상자 중앙에
  그려지면 양옆으로 3px씩 튀어나오는데, 옆 타일까지 겨우 2px밖에 안 남아서
  튀어나온 부분이 옆 칸 글자와 겹쳤다). `TILE_W`보다 확실히 좁은
  `LABEL_MAX_W`(`TILE_W - 8` = 58px) 상수를 새로 만들어 감싸는 기준 너비로
  써봤지만, 그래도 여전히 옆 칸 글자와 살짝 겹쳐 보이는 경우가 있었다 —
  `wrap_two_lines`/`text_width` 는 글자 수 × `ADVANCE`(고정 폭)로 너비를
  "계산"만 할 뿐이고, 실제로 그 계산이 화면에 그려지는 폭(CRT 블룸 등 후처리
  포함)과 완벽히 일치한다는 보장이 없다. 계산에만 기대는 대신, `draw_one_icon`
  에서 라벨을 그리기 직전에 `r.set_clip` 으로 그 아이콘의 타일 폭(`IC_W`)
  밖으로는 물리적으로 아예 못 그리게 잘라버리도록 고쳤다(그리기 끝나면 바깥
  클립을 다시 복원). 이러면 너비 계산이 살짝 틀리더라도 옆 칸으로 글자가
  삐져나가는 일 자체가 원천적으로 불가능해진다. 라벨 글자 크기(`LABEL_TEXT_SCALE`)도
  0.8 → 0.7 로 줄였다 — 감싸는 최대 너비(`LABEL_MAX_W`)는 그대로인데 글자가 작아진
  만큼 같은 폭에 더 많은 글자가 들어가서, 이름이 조금만 길어도 두 줄로 넘어가던
  게 줄었다. `LABEL_LINE_H`(줄 높이)도 `LABEL_TEXT_SCALE` 을 그대로 참조해서 계산하므로
  스케일을 바꾸면 타일 높이(`TILE_H`)도 알아서 다시 계산된다
- **작업표시줄**: 시작 버튼(+시작 메뉴), 실행 중 창 버튼, 시계(KST, 클릭 시 시스템 메시지
  패널 — 첫 메일이 도착하면 아이콘 + 제목 한 줄로 간략하게 뜬다. 메시지가 여러 개
  쌓이면 항목 사이에 구분선이 들어가 나뉘어 보인다), 와이파이 아이콘(실제 연결
  상태/SSID/IP 조회)
- **알림 토스트**: 새 메일이 도착하면 작업표시줄 바로 위 우측 하단에 발신자/제목이
  잠깐(5초) 떴다가 저절로 사라진다. 누르면 바로 Email 을 열고 닫힌다
- **창 관리**: 이동, 크기조절, 최소화/최대화/닫기 — `resizable`(테두리 드래그)과
  `maximizable`(최대화 버튼)이 따로 설정 가능해서 앱마다 다르게 잠글 수 있다.
  `WindowManager::open()` 이 새 창을 작업영역 중앙 근처(살짝 위/왼쪽으로 치우치게,
  `-45`/`-40` 오프셋)에 스폰시키는데, 이 오프셋이 창의 기본 크기가 작업영역 높이에
  가까워지면(예: Email 창이 메뉴바/폴더 트리가 추가되며 커졌을 때) y 를 음수로 만들어
  타이틀바가 화면 위로 잘려나가 보이는 버그가 있었다 — 스폰 위치를 작업영역 안에
  완전히 들어오도록 클램프해서 고쳤다(`open()` 안, x/y 둘 다)
- **휠 스크롤**: 포커스와 무관하게 마우스가 올라간 창이 그대로 받는다
- **파일 확장자별 앱** (`src/apps/`, 파일당 앱 하나):
  - `.txt`: 메모장 (워드랩 + 스크롤)
  - `.mp4`: 영상 플레이어 (재생/일시정지/탐색, WASAPI 오디오 + "풍화" 음질 열화 효과)
  - 이미지(`FileKind::Img(usize)`, `image_viewer.rs`): [STORY.md](STORY.md) 챕터 1(IMAGE) 의
    첫 삽 — 지금은 검수 판정(정상=삭제/이상=체크리스트) 없이 사진을 창 안에 종횡비 유지한 채
    레터박스로 보여주기만 하는 최소 버전이다. 실제 사진 픽셀은 `Assets::photos: Vec<(TextureId,
    u32, u32)>` 에 있고(텍스처만 있는 다른 아이콘들과 달리, 종횡비를 살려 그리려면 원본 크기가
    필요해서 크기도 같이 들고 있다), `FileKind::Img` 의 `usize` 는 그 배열의 인덱스다. 사진은
    사용자가 준 원본(4160×5200 등 고해상도)을 그대로 넣으면 바이너리가 너무 커져서, 긴 변
    900px 로 리사이즈 + JPEG 85 품질로 압축해 `assets/photos/photo01~03.jpg` 로 넣었다(각
    65~85KB 수준). 세 번째 사진은 왼쪽에 RGB 글리치 스캔라인이 있는 버전인데, 원본 우측
    하단에 생성 도구의 워터마크("yeri.ai")가 박혀있어서 그대로 넣으면 우리 게임 에셋에
    제3자 브랜드가 노출되는 문제가 있었다 — 바로 위 여백의 같은 바닥 질감을 잘라 그 자리에
    덮어써서 자연스럽게 지운 뒤 넣었다. 바탕화면이나 Downloads 어디에도 안 두고 그냥
    `fs.add()` 로만 노드를 만들어서 fs 안에만 존재한다 — File Explorer 의 `Videos` 옆에
    위치/폴더와 무관하게 `FileKind::Img` 전부를 모아 보여주는 `Images` 가상 카테고리
    (`explorer_tabs()`, `all_of_kind`)에서 기본으로 보인다. 처음엔 담아두는 폴더("Generated
    Images")를 따로 만들었었는데, 어차피 `Images` 카테고리가 위치 무관하게 다 모아 보여주므로
    담아둘 폴더가 필요 없어서 없앴다.
    사진을 늘릴 땐 `assets/photos/` 에 파일 추가 → `Assets::load()` 에 `load_texture_sized`
    한 줄 → `FileSystem::new()` 에 `fs.add(..., FileKind::Img(다음 인덱스))` 만 더하면
    나머지(아이콘/뷰어/Images 카테고리)는 전부 자동으로 반영된다.
  - `.lock`: 비밀번호 대화상자. 풀면 **폴더로 변해** 안의 파일들이 나옴 — 별도 팝업
    창 없이, My Computer 가 열려있으면 그 창 안에서 원래 있던 카테고리(예:
    Downloads) 바로 아래 하위 폴더로 들어가 보여준다
  - `.tar`: 압축파일. 전용 PNG 아이콘 대신 지퍼 달린 노란 폴더 모양을 직접 그린
    아이콘으로 표시된다. `FileSystem::hex_tool_installed` 가 false 인 동안은 더블클릭해도
    "No extraction utility installed." 안내만 뜨고 안 열린다 — HexTool 설치
    마법사(아래 Installer 항목)를 끝까지 마쳐야 다른 안내로 바뀐다(실제로 내용물을
    풀어 보여주진 않음, 설치 여부만 반영). 안내문은 창 폭 기준으로 자동 줄바꿈돼서
    안 잘린다
  - Installer(`HexTool Setup.exe`): 고전 InstallShield/한컴오피스류 설치 마법사
    느낌 — 창 전체(진짜 타이틀바는 창 관리자가 이미 그려줌)를 그대로 카드로 써서
    맨 위에 굵은 부제 한 줄 + 밑줄로 페이지를 구분하고(안에 또 타이틀바를 얹으면
    "창 속의 창"처럼 보여서 안 그린다), 그 아래 왼쪽 삽화 칸(Installer 아이콘을
    크게) + 오른쪽 안내문(창 폭에 맞춰 자동 줄바꿈), 맨 아래에 Back/Next-or-Finish/
    Cancel 버튼이 있다. 이미 설치돼 있으면(`FileSystem::hex_tool_installed`) 나머지
    페이지 다 건너뛰고 바로 "HexTool is already installed..." + Finish 버튼만
    있는 페이지로 연다. 아니면 Welcome(설명 + Next/Cancel) → **License**(스크롤
    되는 sunken 텍스트 박스에 EULA 문구, 안 다 보이면 오른쪽에 스크롤바도 뜸 +
    "I accept the terms" 체크박스 — 체크 전엔 Next 가 회색으로 비활성, Back 버튼도
    실제로 동작함) → **Installing**(진행바가 ~2.6초에 걸쳐 채워지는데, 부팅 화면
    로딩 바와 같은 방식으로 랜덤 웨이포인트를 만들어 일정한 속도가 아니라 멈칫거리다
    훅 튀는 식으로 들쭉날쭉하게 찬다 — 웨이포인트의 x/y 값은 무작위 구간 길이를
    합으로 나눠 정규화하는데, 부동소수점 오차 때문에 마지막 점이 (1.0, 1.0) 에 정확히
    안 맞고 0.999998 처럼 살짝 모자랄 수 있어서, 진행바가 `done = progress >= 1.0`
    판정을 영영 못 넘겨 99% 근처에서 멈춰버리는 버그가 있었다 — `build_load_waypoints`
    마지막에 마지막 점을 강제로 정확히 (1.0, 1.0) 으로 맞춰 고쳤다(부팅 화면의 로딩
    바도 같은 함수를 복사해 쓰고 있어서 같이 고쳐뒀다). 진행률에 맞춰 "Copying files..." →
    "Registering HexTool.dll..." → ... → "Finalizing..." 로 상태 문구가 바뀌고,
    10% 단위 눈금이 있다. **진행바가 막 100% 를 찍는 그 프레임에 바탕화면에 HexTool
    아이콘이 바로 생긴다**(Finish 버튼을 누르기 전이라도 — 실행 중에 새 아이콘이
    추가되는 첫 사례라 겹치지 않게 빈 격자 칸을 찾아서 놓는다) — 그 뒤 "Done." 을
    0.6초 보여준 뒤 다음 페이지로. 이 페이지에선 취소 불가) → Finish(Finish 버튼 —
    누르면 그냥 창만 닫힘, 아이콘은 이미 생겨있음) 네 페이지를 순서대로 넘어간다.
    `hex_tool_installed` 는 저장 파일에 남아 재시작해도 유지된다
  - Email: Outlook Express(Windows 98) 참고 레이아웃으로 다시 짰다 — 맨 위 장식용
    메뉴바(File/Edit/View/Go/Tools/Compose/Help, 클릭 안 먹음) 하나만 두고, 그 아래
    왼쪽에 고정 폴더 트리(Inbox/Outbox/Sent Items/Deleted
    Items/Drafts, `MailFolder` enum) + 오른쪽 내용물. 예전엔 앱을 열면 곧장 메시지
    목록/내용이 보였는데, 지금은 Outlook Express 처럼 폴더를 하나 골라야 오른쪽에
    뭔가 뜬다(`EmailApp::folder: Option<MailFolder>`, 기본값 None — 참고 이미지의
    시작 화면과 동일). Inbox 를 고르면 그 안에 예전 레이아웃(왼쪽 최신순 메시지
    목록 + 오른쪽 From/Sent/To/Cc/Subject 라벨 필드 + 스크롤되는 본문)이 그대로
    다시 나온다 — Outbox/Sent Items/Deleted Items/Drafts 는 애초에 발신·임시보관·
    삭제 기능 자체가 없어서 지금은 "There are no items in this folder." 안내만
    뜬다. 새로고침(메일 도착 시 `refresh_email_if_open`) 전후로 골라둔 폴더도
    골라둔 메시지처럼 이어간다(`EmailApp::folder_idx`/`set_folder_idx`) — 안 그러면
    `EmailApp::new()` 가 항상 `folder: None` 으로 다시 시작해서, 메일이 막 도착한
    순간 열려있던 창이 "폴더를 선택하세요" 화면으로 도로 튕겨 보인다.

    폴더 트리 배경은 다시 FACE(회색)로 — 한때 참고 이미지처럼 흰 배경으로 바꿔봤지만
    최종적으로는 원래의 회색이 더 낫다는 판단으로 되돌렸다. "이 앱 자체"를 가리키는
    루트 노드(펼침 박스 + 아이콘 + "Email" 라벨)도 없앴다 — 항상 펼쳐져 있고 접을
    수도 없어서 그냥 자리만 차지했던 장식 노드였다. 지금은 다섯 폴더가 트리 맨
    위부터 바로 들여쓰기돼 시작한다.

    **아이콘은 두 번 갈아엎었다.** 처음엔 전용 아이콘이 없는 Outbox/Sent Items/
    Deleted Items/Drafts 를 `icon_folder` 텍스처에 폴더마다 다른 반투명 색을
    덧칠해서(`r.sprite` 의 color 인자 활용) 구분해봤는데, 흰 배경에서 그 반투명
    색 덩어리가 아이콘 위에 뜬 얼룩처럼 또렷하게 보여서 오히려 더 지저분해
    보인다는 피드백을 받았다 — 아이콘 형태 자체가 다른 게 아니라 색만 칠한 거라
    한계가 있었다. 대신 사용자가 이전에 준 Windows 98 아이콘 팩 안에서 실제로 쓸
    수 있는 아이콘을 다시 찾았다: Inbox 는 팩 안에 있던 진짜
    `envelope_closed.ico`(`assets/icon_envelope.png`, `IconType::Envelope`)로
    바꿨다 — 참고 이미지의 Inbox 아이콘과 실제로 같은 모양이다(루트 노드용으로
    같이 가져왔던 `outlook_express.ico`/`IconType::OutlookExpress` 는 루트 노드
    자체를 없애면서 같이 지웠다 — `assets/icon_outlook.png` 도 삭제). 나머지 넷
    (Outbox/Sent Items/Deleted Items/Drafts)은 팩 안에 정확히 맞는 아이콘이 없어서
    평범한 폴더 아이콘을 색 없이 그대로 쓴다 — 다 똑같아 보이는 대신, 적어도
    얼룩덜룩하게 지저분해 보이지는 않는다. 비트맵 폰트엔 볼드 글리치가 따로
    없어서, 안 읽은 메일이 있는 Inbox 라벨은 `draw_bold_text`(1px 오른쪽으로
    겹쳐 두 번 그리는 레트로 "가짜 볼드" 트릭)로 강조했다.

    (이 트리 재작업은 영역 하나만 좁게 바꾼 것이고, 전체 크롬(주소창/상태바 등)을
    My Computer 와 맞추려던 훨씬 이전의 시도와는 다르다 — 그건 Email 과 무관한
    요소까지 너무 많이 바뀌어서 이질감이 든다는 피드백으로 되돌렸었다)

    **내용 없는 화면(폴더 미선택/빈 폴더/받은편지함 비어있음)도 덜 휑하게 고쳤다.**
    예전엔 pane 왼쪽 위 구석에 작은 안내문 한 줄만 덩그러니 있어서, 창을 기본
    크기로만 열어도(하물며 더 키우면) 그 큰 빈 공간이 유난히 휑해 보인다는 피드백을
    받았다. `draw_empty_state` 를 새로 만들어서, 큰 아이콘(48px — "Select a
    folder"는 `IconType::Email`, "There are no items"는 `IconType::Folder`,
    "No new messages"는 Inbox 와 같은 `IconType::Envelope`) 을 pane 세로 40%
    지점에 두고 그 아래 안내문을 붙였다 — 탐색기류가 빈 폴더를 열었을 때 흔히
    쓰는 요령이다.

    **Microsoft Exchange 클라이언트 참고 이미지로 목록/상태바도 더 채웠다.**
    Inbox 목록에 From/Subject 두 칸짜리 헤더(Explorer 의 Name/Size 헤더와 같은
    raised 바)를 추가하고, 각 행도 From 칸과 Subject 칸으로 나눠 보여준다.
    메시지별 읽음 상태(`EmailApp::read: Vec<bool>`, `select()` 로 고르는 순간
    읽음 처리)를 새로 추적해서, 안 읽은 메일은 참고 이미지의 "Welcome!" 행처럼
    From/Subject 둘 다 `draw_bold_text` 로 굵게 보여준다. 폴더 트리의 Inbox
    괄호 숫자도 전체 메시지 수 대신 **안 읽은** 개수로 바꿨다(`unread_count()`).
    맨 아래엔 참고 이미지의 "1 Item, 1 Unread" 그대로 상태바(`draw_status_bar`,
    `STATUS_H`)를 새로 추가했다 — Inbox 가 아니면 셀 데이터가 없어서 그냥
    "0 Items"만 보여준다. 툴바(아이콘 버튼 줄)는 이번에도 안 넣었다 — 아이콘 팩에
    Compose/Reply/Forward/Delete 같은 메일 전용 아이콘이 없어서, 억지로 새로 그리는
    대신 실제 기능이 있는 요소부터 채우는 쪽을 골랐다.

    From 칸 폭은 처음엔 `FROM_COL_W = 110px` 로 고정해뒀는데, 창을 넓게 열면
    Subject 칸만 텅 빈 채로 넓어지고 From 칸은 그대로 좁아서 "PalaceCompany@email.com"
    같은 긴 발신 주소가 "PalaceC..." 로 심하게 잘려 두 칸의 비율이 안 맞아 보인다는
    피드백을 받았다 — `FROM_COL_FRAC`(pane 폭의 40%, `FROM_COL_MIN`~`FROM_COL_MAX`
    로 130~230px 사이만 허용)로 바꿔서, 창 크기와 무관하게 두 칸이 항상 비슷한
    비율로 나뉘고 From 칸도 어지간한 이메일 주소는 안 잘릴 만큼 넓어지게 했다.

    **마무리 다듬기 한 라운드 더.** 기본 창 크기를 480×400 → 500×420 으로 살짝
    키웠다. 폴더 트리 행은 원래 루트 노드 아래 들여쓰려고 뒀던 여백(`CHILD_INDENT`)
    이 루트를 없앤 뒤에도 그대로 남아 왼쪽 가장자리가 쓸데없이 넓어 보였는데,
    그 여백을 없애 트리 항목이 창 가장자리에 더 바짝 붙게 했다. From/Subject
    헤더는 Explorer 의 Name/Size 헤더처럼 raised 바 두 칸으로 만들었더니 두 입체
    테두리가 경계에서 겹쳐 유난히 튀어나와 보인다는 피드백을 받고, 훨씬 차분한
    단일 flat 바 + 얇은 세로 구분선으로 바꿨다. My Computer 의 목록/트리처럼
    헤더+목록 전체를 옅은 회색 `border()` 로 한 번 감싸서 하나의 패널처럼 보이게도
    했다. 마지막으로 메뉴바의 File/Edit/View/Go/Tools/Compose/Help 는 전부
    기능 없는 장식이라 오히려 헷갈린다는 지적을 받고 다 지웠다 — 그 자리엔 이
    창이 뭔지 알려주는 라벨("Outlook Mailbox") 하나만 남겼다.

    **읽음 표시가 기록이 안 되던 버그**: 메시지를 열어 읽음 처리(`select()` 가
    `read[i] = true`)를 해도, 3초마다 도는 주기 새로고침(`refresh_email_if_open`)
    이 매번 `EmailApp::new()` 로 완전히 새 인스턴스를 만들면서 `read` 를 전부
    `false` 로 되돌렸다. 처음엔 `EmailApp` 에 `read_state()`/`set_read_state()`
    를 추가해서 `selected`/`folder` 와 같은 방식으로 새로고침 전후로만 이어받게
    고쳤는데, 그건 **인스턴스 사이**의 문제만 해결할 뿐 게임 자체를 종료했다
    재시작하면(저장 파일에서 다시 불러오면) 여전히 다 안 읽음으로 초기화됐다 —
    `read` 가 애초에 `EmailApp` 안에만 있는 순전히 화면용 상태라 저장 파일(fs)
    에 실릴 방법이 없었기 때문. 그래서 진짜 기록을 fs 쪽으로 옮겼다:
    - `FileSystem` 에 `#[serde(default)] pub email_read: Vec<usize>`(읽은 메시지
      인덱스들)를 추가했다 — `ever_downloaded` 와 같은 요령이고, `#[serde(default)]`
      덕분에 이 필드가 없던 예전 저장 파일도 계속 불러와진다(그냥 다 안 읽은 걸로).
    - `EmailApp::new()` 가 이제 `read_indices: &[usize]` 를 받아서 그걸로 초기
      `read` 를 만든다 — `apps/mod.rs::open()` 이 매번 `&fs.email_read` 를
      넘겨주므로, 새로고침이든 재시작이든 창이 새로 열릴 때마다 항상 fs 의
      진짜 기록으로 다시 맞춰진다.
    - 메시지를 클릭해서 고르면(`draw_pane_content`) `select()` 로 화면(self.read)
      은 바로 갱신하고, 동시에 `AppAction::MarkEmailRead(i)` 를 돌려줘서
      `desktop.rs` 가 `fs.email_read` 에 실제로 기록하고 그 자리에서 즉시
      저장(`write_save`)한다.
    - 이제 `read` 가 매번 fs 로 다시 초기화되므로, 이전에 만들었던
      `read_state()`/`set_read_state()` 인스턴스 간 이어받기 코드는 필요 없어져
      지웠다 — `selected`/`folder` 만 계속 그 방식대로 이어받는다.

(추가 개정 7-9) 복구 관련 버그 세 가지를 한 번에 고쳤다.

**① 복구가 항상 바탕화면으로만 갔던 것을 원래 있던 자리로**: 지금까지 "Restore"
는 무조건 `MoveDest::Desktop` 이었다(원래 어디 있었는지 기억하는 자료구조가
없었으니 어쩔 수 없었다). `FileSystem` 에 `trash_origin: Vec<(FileId, FileOrigin)>`
(`FileOrigin::{Desktop, Downloads, Folder(FileId)}`, `apps::MoveDest` 와 거의 같은
모양이지만 `foundation.rs` 가 `apps` 모듈에 기대면 안 되는 더 기반 레이어라 따로
둔 타입)를 추가해서, `move_ids_to()` 가 대상이 정확히 "Recycle Bin" 이라는 이름의
폴더일 때만 그 직전 위치(`FileSystem::locate()` 로 판정 — desktop/downloads/
폴더 children 순으로 확인)를 기록해둔다. 새 `AppAction::Restore`/`DeskAction::
Restore` 경로가 이 기록을 보고 `restore_from_trash()` 로 정확히 그 자리(폴더면
`add_to_folder`, Downloads 면 `download`, 기록이 없거나 원래 폴더가 이제 폴더가
아니면 바탕화면)로 되돌린다. 휴지통을 나가는 모든 이동(복구든 아니든)에서 그
기록은 지운다 — 다시 휴지통에 들어갈 때 새로 기록되므로.

**② 복구했는데 바탕화면에서 안 보이던 버그**: 원인은 "Restore" 가 새 아이콘을
놓을 자리를 그 순간의 마우스 좌표(`drop_at`)로 계산했다는 것 — 그런데 그 좌표는
당연히 지금 열려있는 휴지통 창의 "Restore" 버튼 위, 즉 그 창 안이다. 창은
바탕화면 아이콘보다 항상 위에 그려지므로, 복구된 아이콘이 정확히 그 창 뒤에
깔려서(진짜로 안 그려진 게 아니라 창에 가려서) 안 보였다. 마우스 좌표에 안
기대는 `add_existing_to_desktop_default()`(= `add_desktop_icon` 과 같은 요령으로
`first_free_tile()` 을 써서 실제로 비어있는 첫 칸에 놓는다)를 새로 만들어
`restore_from_trash()` 의 바탕화면 케이스에 썼다 — 이제 항상 실제로 빈 칸에
놓이고, 격자 왼쪽 위부터 훑으므로 가운데쯤 열려있는 휴지통 창에 가릴 가능성도
낮다.

**③ My Computer 에서 바탕화면 휴지통으로 바로 드래그하면 바탕화면에 먼저
안착하던 문제**: `ExplorerApp` 은 자기 창 밖으로 드롭되면 자기가 아는 게
사이드바뿐이라(바탕화면 아이콘 위치는 모른다) 무조건 `MoveDest::Desktop` 을
요청했다 — 드롭 지점이 실은 바탕화면의 휴지통(또는 다른 폴더) 아이콘 바로 위였어도
그냥 바탕화면에 내려앉고 말았다. `desktop.rs` 의 `DeskAction::MoveFiles` 처리부에서,
요청받은 대상이 `MoveDest::Desktop` 이면 그 드롭 지점(`m`)이 실제로 바탕화면의
어떤 폴더 아이콘 위였는지(`desktop_folder_drop_target_at` — 바탕화면 아이콘끼리
드래그할 때 쓰던 것과 같은 판정 함수를 재사용) 한 번 더 확인해서, 있으면 그
폴더로 목적지를 바꿔준다. 이제 My Computer 안에서 파일을 끌어 바탕화면의
휴지통(또는 다른 폴더) 아이콘 위에 놓으면 곧장 그 안으로 들어간다.

(추가 개정 10) 코드 압축/안정성 점검 — `foundation.rs::delete_permanently()` 와
`detach_from_container()` 가 "Downloads 목록과 모든 폴더의 children 에서 이 id를
뗀다"는 완전히 같은 이중 루프를 각자 갖고 있던 걸 발견해서, `delete_permanently`
가 `detach_from_container` 를 호출하도록 합쳤다(동작은 그대로, 코드만 줄었다).
`cargo build`/`clippy` 를 전체적으로 다시 훑어서 새로 생긴 dead-code/미사용
경고가 없는지, `unwrap()`/`expect()`/배열 인덱싱이 쓰인 자리마다 그 앞뒤 조건이
실제로 패닉을 막아주는지(예: `window_manager.rs` 의 `edges.unwrap()` 은 바로 앞
`edges.is_some()` 로, `video.rs` 의 `pcm_queue.pop_front().unwrap()` 은 반복 횟수를
`need.min(pcm_queue.len())` 로 미리 잘라서 항상 안전함) 확인했다 — 새로 고칠
자리는 못 찾았고, 이번 세션에서 새로 추가된 부분(휴지통/이메일 읽음 기록) 중
겹치던 로직 하나만 정리했다.

    **Inbox 를 고르면 메시지 목록이 pane 전체 폭을 쓴다.** 예전엔 항상 왼쪽에 좁은
    메시지 목록 + 오른쪽에 좁은 읽기 화면을 나란히 보여줬는데, 메시지를 고르면
    목록은 사라지고 읽기 화면이 그 폭을 통째로 넘겨받는 식으로 바꿨다 — 목록/읽기가
    서로 다른 화면이 되어 각자 더 넓게 쓸 수 있다. 읽기 화면 맨 위엔 "< Back to
    Inbox" 링크(호버하면 남색 + 밑줄)가 있어서 눌러서 목록으로 돌아갈 수 있고,
    폴더 트리에서 Inbox 를 다시 눌러도(`draw_folder_tree` 가 클릭마다 `self.selected
    = None` 도 같이 리셋한다) 똑같이 돌아간다. 받은편지함은 처음엔 비어있다가
    데스크톱에 들어오고 6초 뒤 PalaceCompany@email.com 으로부터 메일 한 통(제목
    "Photo QA Request", 사진 검수를 요청하며 첨부파일 비밀번호 4471 을 본문에
    알려줌)이 도착 — 도착 시각은 시계 클릭 시스템 메시지 패널에도 알림으로 뜬다
    (도착 여부는 저장 파일에 남아 재시작해도 다시 6초를 기다리지 않음). 메일이
    있어도 왼쪽 목록에서 직접 발신자를 고르기 전엔 오른쪽에 내용을 강제로 띄우지
    않는다(고른 뒤에 새 메일 도착 등으로 새로고침이 와도 그 선택이 유지됨). 잠금파일
    첨부(`Photos.lock`, 비번 `4471`)가 붙어있고, Download 버튼을 누르면 My
    Computer 의 Downloads 탭에 추가된다(다운로드 여부는 저장 파일에 남아 재시작해도
    유지 — `EmailApp::new` 가 `fs.ever_downloaded` 를 직접 봐서 이미 받은 첨부파일이면
    처음부터 "Downloaded" 로 시작한다. 예전엔 항상 false 로 시작해서, 창을 닫았다
    다시 열거나 다운로드/잠금해제로 새로고침될 때마다 이미 받은 첨부파일도 다시
    Download 버튼이 떠서 몇 번이고 또 누를 수 있었다). 왼쪽 목록의 발신자 줄
    (`wrap_two_lines` 로 최대 두 줄까지 접힘)은 예전엔 그냥 `r.text` 로 그려서,
    "PalaceCompany@email.com" 처럼 공백 없이 긴 주소가 계산상으론 다 감싸졌다고
    판단돼도 실제 렌더링에선 행(sunken 박스) 오른쪽 테두리 밖으로 살짝 삐져나와
    보이는 문제가 있었다(바탕화면 아이콘 라벨에서 겪은 것과 같은 계열의 버그 — 글자
    폭 "계산" 과 실제 렌더링이 완벽히 일치한다는 보장이 없다). 제목 줄은 원래부터
    `text_clipped` 를 썼는데 발신자 줄만 빠져있었던 것 — 똑같이 `text_clipped` 로
    바꿔서 계산이 살짝 틀려도 물리적으로 행 밖으론 못 나가게 막았다.

    **그런데도 여전히 삐져나와 보였다** — 원인은 `text_clipped` 가 아니라
    `wrap_two_lines`(`ui.rs`) 의 글자 단위 폴백 자체에 있는 off-by-one 버그였다.
    "이 접두사까지는 max_w 를 안 넘는지" 를 한 글자씩 늘려가며 검사하다가, **처음으로
    max_w 를 넘어선 그 접두사**(이미 넘친 상태)를 그대로 첫 줄로 써버렸다 — 그래서
    "PalaceCompany@email.com" 은 "PalaceCompan" 로 접혔는데, 이 12글자 자체의 폭이
    이미 max_w 보다 넓었다(계산상 "안 넘음" 판정을 받은 적이 없는 상태). `text_clipped`
    로 그 줄을 그리면 실제로 안 들어가는 마지막 글자를 다시 한번 잘라내면서 "PalaceCompa"
    처럼 원래 접혔어야 할 것보다 한 글자 더 잘려 보였던 것 — 화면엔 "계산은 안 넘친다고
    했는데 실제로는 잘린다" 는 이중 모순으로 나타났다. `ui.rs` 의 `wrap_two_lines` 에서
    "마지막으로 안전하게 들어맞는 접두사" 에서 끊도록 고쳤다 — 이제 wrap 이 만든 줄은
    항상 실제로 max_w 안에 들어오므로, `text_clipped` 는 이미 안 넘치는 줄에 대해선
    아무 것도 자르지 않는 순수한 안전장치로만 남는다.

    **그래도 여전히 글자가 없어져 보였다** — 이번엔 계산 버그가 아니라 애초에
    `wrap_two_lines` 가 "최대 두 줄" 로 설계된 함수였다는 게 문제였다. 사이드바
    (130px)가 좁아서 "PalaceCompany@email.com" 같은 긴 주소는 두 줄로 접어도 다
    안 들어가고("PalaceCompa" / "ny@email.co" 처럼 세 번째 줄로 넘어갈 부분이 통째로
    잘림), 두 줄을 넘는 나머지는 애초에 `wrap_two_lines` 의 설계상 버려지는(두 번째
    줄이 여전히 max_w 를 넘으면 그대로 두고 호출부의 clip 이 잘라내는) 몫이었다.
    `ui.rs` 에 줄 수 제한이 없는 `wrap_lines` 를 새로 추가했다 — 기존 두 줄 접기
    로직(`split_line_once` 로 뽑아낸 공용 함수)을 그대로 재사용하되, 남은 부분이
    다 들어올 때까지 반복해서 접는다. Email 의 발신자 줄(`EmailApp::update`)만 이걸
    쓰도록 바꿔서, 행 높이가 필요한 줄 수만큼 자동으로 늘어나 주소 전체가 글자
    손실 없이 다 보인다(아이콘 라벨/탭처럼 칸 높이가 고정이라 두 줄로 강제해야 하는
    다른 곳은 그대로 `wrap_two_lines` 를 쓴다).

    **결국 왼쪽 목록엔 발신자 대신 제목만 한 줄로 보여주도록 바꿨다** — 어차피
    발신자 주소는 오른쪽 From 필드에서 보이고, 좁은 사이드바에 긴 주소를 억지로
    다 밀어 넣으려니 계속 같은 계열의 버그가 반복됐다. 목록에서는 어떤 메일인지
    한눈에 훑어보는 게 더 중요해서 제목이 더 유용하기도 하다. `ui.rs` 에
    `truncate_ellipsis` 를 새로 추가해서 — 한 줄에 다 안 들어가면 뒤를 잘라내고
    "..." 를 붙인다(`text_clipped` 처럼 그냥 잘리기만 하면 "제목이 원래 더 길다"
    는 사실 자체가 안 보인다) — 이걸로 그린다. 각 행이 이제 항상 한 줄 고정
    높이라 목록도 더 촘촘해졌다. 더 이상 쓰는 곳이 없어진 `wrap_lines` 는 지웠다.

    **오른쪽 안내 문구("Select a message to read." / "No new messages.")도 같은
    계열의 문제였다** — 이건 아예 폭 제한이 없는 `label()`(내부적으로 그냥 `r.text`
    호출, 잘림/줄바꿈 처리가 전혀 없음)로 그리고 있었다. 창을 좁게 리사이즈하면
    (창 크기/위치가 저장/복원되면서 예전에 줄여놨던 크기로 다시 열릴 수도 있다)
    이 문구 폭이 오른쪽 영역보다 넓어질 수 있는데, `label()` 은 폭을 아예 안 보므로
    그대로 창 오른쪽 경계 밖까지 삐져나가 그려졌다. `text_clipped` 로 바꿔서 오른쪽
    영역(`content.w - 20`) 안으로 막았다. 이 잠금파일을 풀면 안에
    `HexTool Setup.exe`(위 Installer) 와 `Photos.tar`(위 .tar) 가 하위 폴더로 들어있다
  - HexTool: Installer 설치를 끝까지 마치면 바탕화면에 생기는 프로그램. 실행하면
    작은 창(260×160)으로 시스템의 `.tar` 파일 목록에서 하나를 고르는 화면(SelectTar)
    이 뜨는데, 안내문 "Select a .tar file to open:" 이 이 좁은 창 폭에선 한 줄로
    오른쪽 밖까지 삐져나가서 `draw_wrapped` 로 접어 그리고, 그 다음 y좌표부터
    목록을 그리도록 고쳤다(목록 높이도 접힌 줄 수에 맞춰 동적으로 계산). 고르는
    순간 `AppAction::Resize` 로 창이 스스로 편집용 크기(600×380)로
    넓어진다(중심은 그대로, 크기만) — 두 화면이 창 하나를 억지로 나눠 쓰던 문제를
    해결한 방식. 최소 360×320 까지 줄일 수 있다(그 아래로는 목록이 버튼과 겹칠 수
    있어서 막아뒀다). 오른쪽 패널(슬라이더+목록)은 폭이 거의 고정(~190px)이라 창을
    넓혀도 그 폭은 그대로고, 늘어난 공간은 전부 왼쪽 미리보기가 가져간다.
    편집 화면: 왼쪽 큰 미리보기 패널(위에 지금 보는 파일명 표시, 실제 사진 자산은
    없어서 항목마다 고유한 색 + 옅은 노이즈로 채운 자리표시자) + 오른쪽에 "N of 5
    remaining"(안 본 사진 수) → Brightness/Saturation 슬라이더 2개(조절하면
    미리보기가 실시간으로 다시 칠해짐 — 원래 색상(Color/hue) 슬라이더도 있었지만
    숨은 형체 판별에 안 쓰이는 데다 UI 만 복잡하게 해서 뺐다. 항목마다 다른 색으로
    보이는 건 슬라이더 없이 고정된 `base_hue(idx)` 값으로만 정해진다) → **이상현상
    종류** 목록(Extra figure /
    Duplicate object / Wrong shadow / Discoloration / None — 파일명이 아니라 지금
    사진에서 실제로 보이는 이상현상 종류를 하나 골라 표시, 다시 누르면 해제) → 하단
    Submit 버튼(하나도 안 골랐으면 비활성). 슬라이더 값 숫자("100" 같은 세 자리
    수)가 패널 오른쪽 밖으로 삐져나가지 않도록 슬라이더 폭 계산에 여유 마진을
    36px → 50px 로 늘려뒀다(`draw_slider` 가 값을 x+w+12 위치에 그리는데, 세 자리
    숫자는 33px 이 필요해서 기존 마진으론 부족했음). 5개 중 `IMG_0144.jpg` 에는
    평소엔 거의 안 보이는 희미한 형체가 숨어있어서, 채도를 낮추고 밝기를 올릴수록
    또렷하게 드러난다(정답은 "Extra figure", 나머진 전부 "None"). Submit 을 누르면
    자동으로 다음 미확인 사진으로 넘어가고, 5장을 다 처리하면 결과 창(Result)에서
    맞은 개수는 안 알려주고 진짜 이상현상(IMG_0144.jpg)을 찾아냈는지만 알려준다 —
    이 화면은 안내문 하나 + 버튼만 있어서 편집 화면 크기(600×380)를 그대로 쓸
    이유가 없으므로, Result 로 넘어가는 순간 `AppAction::Resize` 로 340×150 짜리
    작은 창으로 다시 줄어든다(이 페이지의 Continue 버튼은 텍스트가 88px 인데 버튼
    폭이 70px 밖에 안 돼서 밖으로 삐져나갔던 걸 92px 로 넓혀 고쳤다). 거기서
    Continue 를 누르면 검토가 끝난 `.tar` 를 영구히 지울지 묻는 창(DeleteConfirm)
    으로 넘어가는데, 이 화면 문구("Photos.tar should be permanently
    deleted...")가 Result 문구보다 길어서 줄바꿈하면 3줄까지 나가 340×150 그대로
    쓰면 Keep/Delete 버튼과 겹쳐버린다 — 그래서 DeleteConfirm 진입 시 별도로
    `AppAction::Resize(340, 180)` 를 한 번 더 호출해 세로만 살짝 키운다.
    Keep(그냥 닫기) 또는 Delete(실제로 영구 삭제, `FileSystem::
    delete_permanently()` — 인덱스 기반 FileId 를 다른 곳에서도 계속 참조하고
    있어서 배열에서 물리적으로 빼는 대신 desktop/downloads 목록과 모든 폴더의
    children 에서만 빼고 노드는 `FileKind::Deleted` 로 표시해 어디서도 다시 안
    보이게 한다)
  - 그 외: Settings(Graphics/Audio/Interface 탭), Credits, My Computer(원래 이름은
    "File Explorer" 였는데, 실제 파일이 아니라 바탕화면의 진입점 자체를 가리키는
    아이콘이라는 걸 더 잘 드러내려고 Windows 9x 식 이름인 "My Computer" 로 바꿨다 —
    `FileSystem::new()` 에서 노드 이름만 바꾸면 되고, 창 제목/주소창 경로는 전부 그
    이름을 그대로 읽어서 쓰므로 따로 손댈 곳이 없었다. 전용 아이콘도 새로 만들었다 —
    예전엔 `icon_of()` 가 `FileKind::Explorer` 를 그냥 폴더 아이콘으로 재사용했는데,
    처음엔 `IconType::Computer` 를 만들어 Installer/HexTool 처럼 직접 그리는 벡터
    아이콘(`draw_computer_icon`)으로 시도했는데, 사용자가 예전에 준 Windows 98
    아이콘 팩(`windows98-icons`) 안에 이미 "컴퓨터 + 탐색기 창" 느낌의 정확히 맞는
    아이콘(`computer_explorer`, 32x32)이 있어서 그걸 그대로 쓰기로 하고
    `draw_computer_icon` 은 지웠다 — `assets/icon_computer.png` 로 가져와
    `Assets::icon_computer` 텍스처로 등록하고, 다른 파일 아이콘들과 똑같이
    `draw_icon` 의 텍스처 분기(`r.sprite`)로 그린다)
    (고전 Windows 탐색기 느낌 — 읽기전용 주소창 + 왼쪽 "Folders" 트리(Downloads/Desktop/Videos,
    전부 같은 폴더 아이콘). 모든 행에 +/- 박스가 기본으로 붙어있고, 하위
    폴더가 있는 항목만 안에 +/- 표시가 그려지며 클릭에 반응한다 — 누르면 더블클릭과
    같은 경로로 하위 폴더를 트리에 들여쓰기로 펼치고, 다시 누르면(로컬 상태만 바꾸는
    즉시 처리) 접는다 + Name/Size 컬럼 있는
    목록(기본값)·아이콘 격자 전환(우측 하단 버튼) + 하단 상태바(항목 수/선택 개수).
    메뉴바/툴바는 뺐다. 사이드바 폭은 경계선 드래그로 조절. Desktop 탭엔 My
    Computer 자기 자신은 안 나옴. Videos 는 아직 콘텐츠가 없어 빈 스캐폴드.
    다운로드/잠금해제 시 열려있으면 그 즉시 새로고침(주기적인 폴링은 안 함 — 트리에서
    막 펼친 하위 폴더 같은 로컬 상태가 몇 초마다 조용히 사라지는 문제가 있어서 뺐다)
    — 잠금 풀리거나 폴더를 더블클릭하면 새 창이 아니라 지금 My Computer 창
    "안에서" 원래 있던 카테고리(예: Downloads) 바로 아래에 트리에 들여쓰기돼 붙고,
    주소창도 `My Computer\Downloads\이름` 처럼 경로로 이어 보여줌. 새로고침이
    일어나도 지금 보고 있던 탭/하위 폴더는 그대로 유지되고 첫 탭으로 안 튕김.
    바탕화면에 놓인 폴더(드래그로 옮겨놓은 것)를 더블클릭해도 마찬가지로 My
    Computer 창 안에서 열린다(`DesktopScene::open_folder_in_explorer`) — 예전엔
    탭/사이드바가 없는 단독 탐색기 창이 별도로 떴는데, My Computer 안에서 폴더를
    열 때와 느낌이 달라서 통일했다. My Computer 가 이미 열려있으면 그 창 안에서
    드릴다운 탭으로 보여주고, 안 열려있으면 My Computer 자체를 이 폴더가 활성
    탭인 상태로 새로 연다 — 데스크톱 아이콘 더블클릭과 `DeskAction::Open` 둘 다
    이 경로를 거친다. 이미 열려있던 경우엔 내용만 새로고침(`refresh_app`)하고
    끝났었는데, 그 창이 다른 창에 가려져 있으면 내용만 바뀌고 화면엔 안 보이는
    상태가 됐다 — `WindowManager::focus()` 를 새로 만들어(최소화 상태면 풀고
    z순서 맨 앞으로) `refresh_app` 바로 뒤에 같이 불러서, 바탕화면 폴더를 열면
    항상 My Computer 가 실제로 화면 맨 앞에 포커스된 채로 보이게 했다.
    목록 보기도 아이콘 격자처럼 빈 자리를 눌러 끄는 고무줄 드래그로 여러 항목을
    한 번에 선택할 수 있다(`draw_list_view` 에 icon_grid 와 같은 마퀴 로직을 그대로
    옮겨왔다) — 이미 선택된 항목을 다시 누르면(마퀴가 아닌 단순 클릭) 선택이 풀리지
    않고 유지되는데, 그래야 여러 개를 고른 채로 그 중 하나를 눌러서 통째로 드래그할
    수 있다. 폴더가 비어있어도(항목 0개) 마퀴 드래그 자체는 그대로 동작한다 — 예전엔
    비어있으면 함수가 곧장 반환해서 드래그를 시작할 수조차 없었다. 선택한 항목(들)을
    누른 채로 일정 거리 이상 끌면 파일 옮기기 드래그로 바뀌어, 마우스 포인터를 따라
    옮기는 항목(들)의 흐린 "복사본"(아이콘을 그린 뒤 옅은 색을 덮어써 30% 정도로
    흐리게 흉내낸다 — `draw_icon` 자체엔 알파가 없어서)이 그려진다. 격자 보기는
    칸 크기가 아이콘과 비슷해서 눌렀던 지점(항목 좌상단에서 클릭한 지점까지의
    오프셋)을 그대로 유지한 채 따라오지만, 목록 보기는 행 전체가 창 폭만큼 넓어서
    그 오프셋을 그대로 쓰면(행 오른쪽 끝 근처를 눌렀을 때) 고스트가 커서에서 한참
    떨어져 보인다 — 그래서 목록 보기에서는 대신 고스트 아이콘의 정중앙이 항상
    커서에 오도록 고정 오프셋을 쓴다. 이 고스트는
    `App::drag_ghost(&self) -> Option<DragGhost>` 라는 트레잇 기본 메서드(대부분의
    앱은 None)를 ExplorerApp 이 오버라이드해서 돌려주고, `WindowManager::frame` 이
    매 프레임 각 창의 `drag_ghost()` 를 확인해 `WmFrame::drag_ghost` 로 desktop.rs
    에 넘긴다 — desktop.rs 가 모든 창/작업표시줄을 다 그린 뒤 클립을 전혀 안 건
    상태로 맨 위에 그려서, 마퀴 선택 박스와 달리 창 경계나 화면 밖으로도 자유롭게
    나갈 수 있다(App::update() 안에서 직접 그리면 winman.rs 가 그 창의 client area
    로 걸어둔 클립 때문에 창 밖으로 못 나가서, 일부러 update() 밖으로 빼냈다).
    사이드바의 옮길 수 있는 탭(Desktop/Downloads/실제 폴더 — Videos 는 종류로만 모으는
    가상 카테고리라 대상이 될 수 없음) 위에 놓으면(색으로 따로 강조하진 않고 평소
    hover 하이라이트만 뜬다) `AppAction::MoveFiles` → `FileSystem::
    detach_from_container`/`add_to_folder`/`download()` 로 지금 있던 자리(바탕화면/
    Downloads/폴더 어디든)에서 떼어내 새 자리로 옮긴다. 사이드바가 아니라 아예 이
    창 밖에서(바탕화면이든 다른 창 위든) 손을 놓아도 바로 바탕화면으로 옮겨진다 —
    `area.contains(win.mouse)` 로 지금 커서가 이 창 client area 밖인지만 보고
    판단하므로, 창 밖 어디서 놓든 전부 Desktop 대상으로 취급한다(사이드바 히트가
    없을 때의 기본값). 이렇게 바탕화면으로 옮겨질 때 놓이는 자리는 예전엔 항상
    `first_free_tile()`(왼쪽 위부터 빈 칸을 순서대로 채움)라 어디에 놓든 아이콘이
    엉뚱하게 왼쪽 위 구석에 나타났다 — 이제는 `move_ids_to`/`add_existing_to_desktop`
    이 실제로 손을 놓은 화면 좌표(같은 프레임 안이라 `DeskAction::MoveFiles` 를
    처리하는 시점의 마우스 좌표가 곧 드롭 위치)를 받아서 `nearest_free_tile` 로
    그 지점에서 가장 가까운 빈 칸에 두므로, 바탕화면 안에서 아이콘을 직접
    드래그할 때와 같은 느낌으로 "마우스로 놓은 자리"에 실제로 놓인다. 반대로 바탕화면 아이콘을
    열려있는 My Computer 창 위로 끌어다 놓아도(그 창이 지금 보여주는 폴더/카테고리로)
    옮겨진다(`WindowManager::file_at` 로 그 좌표의 최상단 창이 어떤 FileId 에
    연결됐는지 찾고, 탭이 있는 루트 창이면 `ExplorerApp::current_location()` 을
    downcast 로 물어봐서 대상을 정한다). 열려있는 창이 없으면(진짜 맨 바탕화면),
    놓은 자리에 다른 폴더 아이콘이 있는지도 확인해서(자기 자신은 제외) 있으면 그
    폴더 안으로 옮긴다(`DesktopScene::desktop_folder_drop_target_at`) — 창 확인이
    항상 먼저인 이유는 아이콘 위에 창이 떠 있으면 실제로 보이는 건 창이라 그쪽이
    우선이어야 자연스럽기 때문이다. 이때도 My Computer 와 똑같은 스타일의
    반투명 고스트가 따라다닌다: 예전엔 바탕화면 아이콘을 드래그하는 동안 `icon_pos`
    를 매 프레임 직접 옮겨서 원본 아이콘 자체가 커서를 쫓아다녔는데, 이제는 드래그
    중엔 `icon_pos` 를 전혀 안 건드리고 원래 자리에 그대로 둔 채 고스트만 커서를
    따라가다가, 손을 놓는 순간에야(드래그 시작점부터 놓은 지점까지의 델타를 원래
    좌표에 한 번에 적용해서) 실제 위치가 확정된다. 고스트가 커서를 잡는 오프셋은
    타일(64x60) 왼쪽 위가 아니라 실제로 그려지는 아이콘 글리프 위치(`draw_one_icon`
    이 `x+IC_W/2-IC_SIZE/2, y+3` 에 그리는 것과 같은 계산) 기준으로 잡는다 — 타일
    기준으로 계산했을 땐 드래그를 시작하는 순간 고스트가 실제 아이콘 글리프 자리에서
    옆으로 살짝 어긋나 보이는(잡은 위치가 안 맞는) 문제가 있었다. 바탕화면 쪽만 예외로 취급하는데,
    `DesktopScene::icon_pos` 가 `fs.desktop` 과 같은 인덱스로 짝지어져 있어서
    `retain` 으로 빼면 인덱스가 어긋나므로 DesktopScene 이 `icon_pos` 랑 같이 직접
    떼어내고 붙인다(HexTool 로 검토 끝난 `.tar` 를 영구 삭제할 때도 바탕화면에
    있었을 수 있어 같은 방식으로 고침). 마퀴 선택 박스는 고스트와 달리 여전히
    `gfx.rs` 의 클립 영역(스택이 아니라 통째로 덮어쓰는 값이라, 안에서
    `set_clip(None)` 으로 지워버리면 winman.rs 가 걸어둔 "창 밖으로 안 나가게"
    클립까지 같이 풀려서 창 밖으로 드래그했을 때 사각형이 화면 전체에 삐져나오는
    문제가 있었다) 를 들어올 때의 값으로 복원해서, 창 경계를 절대 넘지 않게 그대로
    묶어둔다 — 선택 범위 표시는 그 목록 안에서만 의미가 있어서다),
    Official Site
    (WebView2 로 실제 웹페이지를 백그라운드에서 캡처해 텍스처로 올림 — CRT 셰이더가
    그대로 먹는다)
- **설정**: 고전 Windows 대화상자 느낌의 그룹박스(레퍼런스: PIF 속성/CD-ROM 속성 창)로
  구획을 나눴다 — Graphics 탭은 "Display"(해상도/프레임레이트 콤보박스 — 필드+▼
  버튼이 분리된 진짜 드롭다운 모양) + "CRT Effects"(강도·색수차·커서 크기 슬라이더),
  Audio 탭은 "Volume"(SFX/BGM/Mp4/Master) + "Effects"(Weathering 음질열화 + Mute All),
  Interface 탭은 "Appearance"(부드러운 스크롤 + 바탕화면 색상)로 묶고 **Erase All
  Memory** 는 위험한 동작이라 그룹박스 밖에 따로 뗀다(저장 데이터 초기화 → 삭제 로그
  연출 → 로비 화면)
- **자동 저장**: 설정 + 바탕화면 상태를 exe 옆 `palaceos_save.json` 에 5초마다 주기 저장
  (`DesktopScene::write_save`). 그것과 별개로, 잠금 해제/다운로드/설치 완료/영구 삭제/
  아이콘 드래그 이동/첫 메일 도착처럼 실제로 상태가 바뀌는 이벤트가 생기면 그 즉시 한 번 더
  저장한다 — 작업표시줄 시작 메뉴의 Shut Down 을 거치지 않고 창을 강제로 닫아도(작업 관리자 등)
  주기 저장 사이의 몇 초가 통째로 날아가는 일이 없도록, 방금 한 행동 단위로 바로바로 남긴다.

  **저장 포맷은 `FileSystem` 전체를 그대로 스냅샷한다.** 예전엔 `SaveData` 가 "바탕화면
  아이콘 이름 목록", "다운로드한 파일 이름 목록", "폴더별 자식 이름 목록" 처럼 이름 기반
  목록만 따로 저장해두고, 불러올 때 `FileSystem::new()` 의 하드코딩된 기본값 위에 그 이름들을
  하나하나 다시 찾아 짜맞추는 식이었다. 이 방식은 구조적으로 계속 같은 종류의 버그를 냈다 —
  기본값에 원래 없던 항목(My Computer 드래그로 새로 바탕화면에 옮긴 Photos 폴더 등)은
  아무리 저장 목록에 이름이 있어도 반영될 자리가 없어 재시작하면 통째로 증발했고, 복원 순서가
  하나라도 어긋나면(예: 잠금 풀기 전/후로 이름이 달라지는데 그 순서를 안 맞추면) 이름을 못 찾아
  조용히 무시됐다. 다운로드한 첨부파일을 바탕화면으로 옮기면 "Downloads 위치 목록"에서 빠지면서
  Email 이 "아직 안 받음" 으로 오판해 재다운로드 버튼이 다시 뜨는 문제도 같은 계열이었다.

  지금은 `foundation::FileSystem`(노드 배열 전체 + desktop/downloads/ever_downloaded/
  email_arrived/hex_tool_installed) 이 통째로 `#[derive(Serialize, Deserialize)]` 로
  직렬화되고, `SaveData` 는 그냥 `{ settings, fs: FileSystem, icon_pos: Vec<(f32,f32)> }`
  세 필드뿐이다(`FileId` 가 그대로 인덱스라서, `FileKind::Folder{children}`/`Lock{children}`
  안의 자식 참조도 별도 처리 없이 그대로 저장/복원된다). 불러올 때는 있었던 상태를 그대로
  되돌려놓을 뿐 이름으로 다시 찾아 재구성하는 과정이 아예 없으므로, "기본값에 없던 항목이라
  반영이 안 됨" 이나 "복원 순서가 어긋나서 이름을 못 찾음" 류의 버그 자체가 구조적으로
  불가능해졌다. 새 파일 형식이라 이 변경 이전에 저장된 옛 파일은 못 읽고(파싱 실패 →
  `foundation::load()` 가 `None` 반환 → 처음 실행처럼 기본값으로 새로 시작) 새 형식으로 다시
  저장된다 — `#[serde(default)]` 로 필드 몇 개만 늘려가던 예전 방식과 달리 구조 자체가
  바뀌어서 부분 호환은 포기했다.

  **창 크기/위치도 파일별로 기억한다.** `DesktopScene::window_geometry:
  HashMap<FileId, (Rect, bool)>`(마지막 복원 크기/위치, 최대화 여부)를 매 프레임
  `WindowManager::window_geometry()` 로 지금 열려있는 모든 창에서 읽어와 갱신해서,
  이동/리사이즈 중에도 실시간으로 최신 값을 들고 있는다 — 창을 닫아도 이 맵에선 안
  지워지므로 마지막으로 뒀던 자리가 그대로 남고, `write_save()` 가 그대로 저장한다.
  다음에 같은 파일을 다시 열 때(`open_file`/`open_folder_in_explorer`/
  `DeskAction::Open` 세 경로 전부) `WindowManager::open()` 이 새 창을 진짜로
  만들었을 때만(이미 열려있어서 앞으로만 가져온 경우는 제외) `apply_saved_geometry()`
  로 기억해둔 자리를 `WindowManager::set_geometry()` 를 통해 덮어씌운다 — 해상도가
  바뀌었거나 화면 밖에 걸쳐 있던 채로 저장됐을 수 있어서, 타이틀바 드래그와 같은
  규칙(60px 여유)으로 작업영역 안에 다시 당겨 넣는다. `Rect`/창 최대화 상태를
  `gfx::Rect`/`window_manager::WinState` 타입 그대로 안 쓰고 `SaveData` 에는
  `(FileId, x, y, w, h, bool)` 원시 튜플로 저장하는 이유는, `foundation` 모듈이
  "자주 안 바뀔 만한 기반 모듈" 이라는 원래 설계를 지키기 위해 더 자주 바뀌는
  `window_manager` 쪽에 의존하지 않게 하려는 것이다(`icon_pos` 도 같은 이유로
  이미 원시 튜플이었다). **처음 구현했을 땐 창을 옮기거나 리사이즈한 뒤 닫았다
  다시 열어도 처음 스폰됐던 자리로 돌아가는 버그가 있었다** — `window_geometry()`
  가 `Window::restore` 필드를 읽었는데, 이 필드는 "최대화하기 직전 크기" 스냅샷일
  뿐이라 `toggle_max()`/`AppAction::Resize` 때만 갱신되고, 정작 타이틀바로 옮기거나
  테두리로 리사이즈하는 동안(`WindowManager::frame` 의 `Drag::Move`/`Drag::Resize`)엔
  `Window::rect` 만 바뀌고 `restore` 는 창을 만들 때 값에서 전혀 안 움직였다. 최대화
  안 된 동안엔 지금 실제로 보이는 `rect` 자체가 곧 "복원 크기" 이므로, 최대화 상태가
  아닐 땐 `rect` 를, 최대화 상태일 땐(그 밑에 깔린 원래 크기를 보존해야 하니) `restore`
  를 쓰도록 고쳤다
- **종료**: 시작 메뉴 Shut Down → BIOS POST 톤의 종료 로그 연출 → 진짜 종료

초기 바탕화면: Recycle Bin, My Computer, Email 순서로 셋이다.

**Recycle Bin(휴지통)**: 새 `FileKind` 변형을 따로 안 만들고, 그냥 이름이 정확히
"Recycle Bin"인 빈 `Folder` 로 만들었다(`FileSystem::new()`). 이러면 드래그해서
파일을 옮기는 동작(`desktop_folder_drop_target_at`), My Computer 에서 더블클릭해
여는 동작(`apps/mod.rs::open()`), 안이 비었을 때 크기 칸을 "--" 로 비우는 동작
(`explorer.rs`) 이 전부 일반 폴더 로직을 그대로 타서 공짜로 된다 — 손대야 하는 건
아이콘뿐이었다. `icon_of()` 를 `FileKind` 대신 `FileNode` 전체를 받게 바꿔서
이름을 볼 수 있게 하고, 이름이 "Recycle Bin"인 폴더는 안에 파일이 있는지에 따라
`IconType::RecycleEmpty`/`RecycleFull` 두 아이콘 중 하나를 고르게 했다(그 외
폴더는 예전처럼 `IconType::Folder`). 아이콘 자체는 사용자가 준 Windows 98 아이콘
팩에서 `recycle_bin_empty-0.png`/`recycle_bin_full-0.png` 를 그대로 가져왔다.

(추가 개정) 처음엔 안이 일반 `Folder` 라는 이유로 열면 그냥 My Computer 창 안에
드릴다운 탭으로 끼워 넣었는데("모든 Folder 는 My Computer 안에서 본다"는 기존
규칙을 그대로 탔다), 사용자가 "별개의 프로그램으로 작동하도록" 요청해서 전용 앱
`RecycleBinApp`(`apps/recycle_bin.rs`)을 새로 만들었다 — 주소창/트리/탭 없이
위에 "Empty Recycle Bin" 버튼 하나짜리 툴바 + 아이콘 격자 + 상태바뿐인, 진짜
Windows 휴지통처럼 훨씬 단순한 창이다. `apps/mod.rs::open()` 의 `Folder` 매칭에
이름이 "Recycle Bin"인 경우를 먼저 매칭시켜 `ExplorerApp` 대신 이 앱을 골라주고,
`desktop.rs::is_drilldown_folder()` 로 "일반 폴더면 My Computer 안에, 휴지통이면
독립 창에" 분기했다. 안의 항목 자체는(드래그로 넣기, 아이콘 표시) 여전히 그냥
`Folder` 라서 다른 폴더와 같은 로직을 그대로 타고, `RecycleBinApp` 은 그 위에
"비우기"(`AppAction::EmptyTrash` → 안의 모든 항목을 `delete_permanently` 로 영구
삭제) 기능만 얹었다.

(추가 개정 2) 실제 Windows 98 휴지통 스크린샷을 참고해서 크롬을 다시 짰다 —
메뉴바(File/Edit/View/Go/Favorites/Help, 장식용 텍스트라 클릭 안 됨) + 툴바
(Back/Forward/Up 은 항상 회색 비활성, 선택된 항목이 있을 때만 켜지는 "Delete") +
주소창(휴지통 아이콘 + "Recycle Bin", 읽기전용) + 왼쪽 안내 패널(큰 휴지통
아이콘 + 제목 + 3색 줄 + 설명 문단 + "Empty Recycle Bin" 링크) + 오른쪽 파일
격자 + 상태바. 여러 줄 문단은 `notepad.rs::wrap_text`(줄바꿈 로직, Email 본문도
같이 씀)를 재사용해서 새로 안 만들고 그대로 가져다 썼다.

(추가 개정 3) 실제 스크린샷을 확대해서 다시 보니 Back/Forward/Up/Delete 툴바
줄이 다 회색으로 죽어있어서(Delete 도 선택된 게 없으면 비활성) 그냥 "안 눌리는
버튼 무더기"로만 보였다 — email.rs 의 툴바를 뺀 것과 같은 이유로, 실제 기능
없이 자리만 차지하는 줄이라 통째로 뺐다. 유일하게 진짜 동작하던 "Delete" 도
왼쪽 패널의 "Empty Recycle Bin" 링크와 기능이 겹쳤어서(둘 다 결국
`AppAction::EmptyTrash`) 아쉬울 게 없었다.

(추가 개정 4) File/Edit/View/Go/Favorites/Help 메뉴바도 같은 이유로 뺐다 — 툴바와
마찬가지로 눌러도 아무 드롭다운도 안 뜨는 장식용 텍스트 줄이라, 사용자가 "Help
같은 게 들어있는 줄도 지워달라"고 확인해줘서 통째로 제거했다. 이제 창은 주소창
+ 왼쪽 안내 패널 + 오른쪽 파일 격자 + 상태바, 넷뿐이다.

(추가 개정 5) 삭제 버튼과 복구 기능을 요청받아서, 오른쪽 파일 격자 아래(상태바
바로 위)에 "Restore"/"Delete" 두 버튼짜리 선택 액션 줄(`draw_sel_row`)을 새로
얹었다(처음엔 격자 위에 뒀는데, 사용자 요청으로 아래로 옮겼다).

(추가 개정 6) "Restore"/"Delete" 글자가 버튼 테두리 밖으로 삐져나오는 문제 —
`ui::button()`/`raw_button()` 은 라벨을 항상 scale 1.0 으로 그리는데, 64px 버튼에
"Restore"(7글자, scale 1.0 이면 77px)가 안 들어갔다. 이 버튼만 공용 button() 을
안 쓰고 직접 그리도록 바꿔서, 이 앱의 다른 글자들과 같은 scale 0.8 로 낮추고
버튼 폭도 66px 로 살짝 넓혔다. 전에
뺐던 Back/Forward/Up 툴바와는 성격이 다르다 — 그건 "이 창엔 애초에 그 기능이
있을 수 없어서" 항상 죽어있는 순수 장식이었지만, 이건 지금 뭔가 선택돼 있는지에
따라 실제로 켜지고 꺼지는 진짜 컨트롤이라(`sel_button`, 선택 없으면 회색 비활성)
같은 "눌리는 척하다 아무 일도 안 함" 문제가 없다. `icon_grid` 가 이미 지원하던
마퀴 드래그/다중 선택(`selected: Vec<usize>`)을 그대로 써서 여러 개를 고르면
"Restore"/"Delete" 둘 다 그 여러 개 전부에 한 번에 적용된다.
- **Restore**: 원래 있던 자리(폴더/Downloads 등)까지는 기억해두는 자료구조가
  없어서, 대신 이미 있던 `AppAction::MoveFiles(ids, MoveDest::Desktop)` 경로를
  그대로 재사용해 바탕화면으로 되돌린다 — 드래그해서 바탕화면에 놓는 것과 완전히
  같은 코드 경로라 새로 만든 게 없다. "정확한 원래 위치"보다 "휴지통 밖으로
  나가서 다시 접근 가능해진다"는 목적 자체가 더 중요하다고 판단했다.
- **Delete**: 선택된 항목들만 골라 기존 `AppAction::EmptyTrash` 를 그대로
  재사용한다(전체 삭제 링크와 같은 액션 타입 — `desktop.rs` 쪽은 "받은 id 목록을
  전부 영구 삭제"라는 같은 로직이라 구분할 필요가 없었다).

같은 김에 두 가지를 더 고쳤다:
- **My Computer 는 절대 지워지지 않게**: `FileKind::Explorer`(My Computer) 도
  다른 바탕화면 아이콘과 똑같은 방식으로 드래그해서 폴더/휴지통 안에 넣을 수
  있었다 — 실수로 넣고 "Empty Recycle Bin" 을 누르면 게임을 진행할 방법 자체가
  사라지는 치명적인 상황이라 이중으로 막았다. (1) `desktop.rs::move_ids_to()`
  가 애초에 `FileKind::Explorer` 를 어떤 폴더로도 못 옮기게 막고(드래그해서
  놓아도 조용히 무시), (2) 혹시 다른 경로로 흘러들어와도 최종 방어선으로
  `foundation.rs::delete_permanently()` 자체가 `Explorer` 노드는 무조건 거부한다
  — 실제 Windows 도 My Computer 를 휴지통으로 못 옮긴다.
- **왼쪽 안내 패널 스크롤**: 창을 세로로 줄이면(또는 기본 크기에서도 글자가
  많으면) 아이콘+제목+색줄+설명 문단+링크가 패널 높이를 넘쳐서 아래쪽이 상태바
  밑으로 그냥 잘려 안 보이는 문제가 있었다. 그리기 전에 콘텐츠 전체 높이를
  먼저 계산해두고(아이콘 높이 + 제목 줄 + 색줄 + 문단 줄 수 × 줄높이 + 링크
  줄 수 × 줄높이), `explorer.rs`/`icon_grid` 와 같은 `ease_scroll`/`scrollbar`
  위젯을 그대로 가져다 세로 스크롤을 붙였다 — 넘칠 때만 오른쪽에 스크롤바가
  나타난다.

**레이어(z-순서) 위에 있는 창에 가려진 창이 커서/호버를 가로채는 문제**: 여러
창이 겹쳐 있을 때, 뒤에 완전히 가려진 창 위로 마우스를 올려도(다른 창이 그
위에 그려져 있어 실제로는 안 보이는데도) 그 뒤쪽 창의 버튼이 리사이즈 커서로
바뀌거나 호버 강조가 뜨는 문제가 있었다. 원인은 `window_manager.rs::frame()`
이 매 프레임 모든 창의 `app.update()` 를 그냥 "지금 실제 마우스 좌표"로 부른다는
점 — 각 앱은 자기 rect 만 알지 자기 위에 다른 창이 덮여 있는지는 모르니, 마우스가
자기 rect 안에 들어오면 무조건 호버로 판정했다. 클릭/드래그(`mouse_down`/
`mouse_clicked`)는 원래도 `focused`(맨 위 창)에만 허용돼 있어서 문제가 없었지만,
클릭 없이 그냥 좌표만 보고 하이라이트하는 호버 스타일링은 그 가드를 안 탔던 것.
이미 휠 스크롤 대상을 찾을 때 쓰던 것과 같은 "이 지점에서 z-순서상 가장 위(=
`wheel_target`)" 계산을 재사용해서, 어떤 창의 rect 안에 마우스가 있어도 그 창이
`wheel_target` 이 아니면(=더 위에 다른 창이 그 지점을 덮고 있으면) 그 창에는
화면 밖 좌표(`(-1e6, -1e6)`)를 대신 넘겨서 그 창의 모든 hit-test 가 실패하게
만들었다. 맨 위(focused) 창은 정의상 항상 `wheel_target` 이므로 이 처리에 걸리지
않는다.

다만 이제 폴더가 `IconType::Folder` 하나로만 안 끝나서, "이 아이콘이 폴더처럼
안을 열어볼 수 있는 대상인가"를 `matches!(icon, IconType::Folder)` 로 직접
검사하던 곳(탐색기 목록의 크기 칸 숨김, 사이드바 트리의 +/- 확장 판정)은 휴지통을
놓칠 뻔했다 — `ui.rs` 에 `is_folder_like()` 를 하나 만들어 그 세 곳이 전부 이걸
쓰도록 바꿨다.

바탕화면 아이콘의 기본 배치 순서는 가장 왼쪽 칸을 1순위로, 그 안에서는 위쪽을
2순위로 격자 전체를 훑어서 "지금 실제로 비어있는" 첫 칸을 찾는다(`DesktopScene::
first_free_tile`). HexTool 설치 완료나 My Computer 드래그로 바탕화면에 새로
놓이는 아이콘이 전부 이 순서를 따른다. 예전엔 `fs.desktop` 의 순번(len()-1)만
보고 자리를 계산해서, 아이콘을 손으로 옮겨 원래 자리가 비어도 그 빈 칸은 다시 안
채워지고 새 아이콘은 항상 "다음 순번" 자리로만 넘어가 격자에 빈 칸이 영영 남는
문제가 있었다 — 매번 실제 점유 상태를 다시 훑는 지금 방식은 그런 빈 칸도 자연스럽게
채운다. (저장 파일이 없는) 첫 부팅의 Recycle Bin/My Computer/Email 기본 배치는 아직 아무
것도 안 놓인 격자라 결과가 같은 `grid_pos` 로 그대로 순서대로 채운다. 이와 별개로,
손으로 드래그해서 놓은 아이콘은(꼭 빈 칸을 찾는 게 아니라) 드롭한 지점에서 가장
가까운 빈 칸에 스냅되는데, 그건 `nearest_free_tile` 이 나선형으로 찾는다.

## 실행

```bash
cargo run
```

## 영상(mp4) 교체

Movie.mp4 앱은 실행 시 `assets/movie.mp4` 를 **읽어서 Windows Media Foundation(OS 내장
디코더)으로 디코딩해 재생**한다. Baseline/Main/High 등 **어떤 H.264 프로파일이든 그대로
재생**되며, 오디오 트랙도 WASAPI 로 함께 재생된다(별도 스레드, 재생 중에도 설정창의
Mp4 Sound/Master/Weathering 슬라이더가 실시간 반영). 파일만 교체하면 재빌드 없이
반영된다. Windows 전용 기능이라 이 프로젝트는 Windows 에서만 빌드/실행된다.

전체화면. 아이콘/버튼은 마우스, 프로그램 안에서는 키보드(방향키 등) 사용.

## 성능 메모

`cargo build`(dev 프로파일)는 기본 `opt-level=0` 이라 영상 프레임 변환 같은 픽셀 루프가
매우 느려질 수 있어, `Cargo.toml` 에 `[profile.dev] opt-level = 2` 를 설정해뒀다.
(그래도 최고 성능이 필요하면 `cargo run --release`.)

## 구조

```
src/
├── main.rs             # 진입점: Stage(EventHandler) + 프레임 루프, mod 선언
├── foundation.rs       # 자주 안 바뀌는 기반: 가짜 파일 시스템 / 공유 설정값 / 저장·불러오기
├── gfx.rs               # 2D 배칭 렌더러 + 비트맵 폰트 + 에셋 로딩
├── crt.rs                # 오프스크린 렌더 타깃 + CRT 셰이더(곡률/스캔라인/색수차) + 4:3 필러박스
├── ui.rs                  # 9x 위젯(베벨/버튼/체크박스/아코디언/아이콘)
├── video.rs                # mp4 디코딩(Media Foundation) + 오디오 재생(WASAPI) + Weathering DSP
├── webview.rs               # Official Site: WebView2 백그라운드 렌더 + 주기적 캡처 → 텍스처
├── window_manager.rs         # 창 관리자 (z순서, 드래그, 크기조절, 타이틀바 버튼)
├── apps/                      # 파일 확장자별 앱 — 새 앱 추가할 땐 파일 하나 + mod.rs 한 줄
│   ├── mod.rs                  # App 트레잇/AppAction/WinInput/Opened + open() 파일→앱 매칭
│   ├── widgets.rs                # 여러 앱이 같이 쓰는 위젯(아이콘 격자/슬라이더/스크롤바/아코디언 리스트)
│   ├── notepad.rs, video_player.rs, email.rs,
│   │   official_site.rs, explorer.rs, password.rs,
│   │   credits.rs, settings.rs, archive.rs, installer.rs,
│   │   hextool.rs                                          # 앱별 구현
└── scenes/                    # 화면 전체를 차지하는 씬들 — 새 씬 추가할 땐 파일 하나 + mod.rs 한 줄
    ├── mod.rs                  # Scene 트레잇/Transition/SceneManager/Frame/Input (씬 상태 관리)
    ├── lobby.rs                 # 로비(타이틀) 화면 — 진입점, 입력 대기 후 BootScene 으로
    ├── boot.rs                  # 부팅 (BIOS POST + 로고/로딩바)
    ├── desktop.rs                 # 데스크톱 (아이콘/작업표시줄/시작메뉴/컨텍스트메뉴/와이파이, 제일 큼)
    ├── shutdown.rs                 # 종료 로그 연출 → Quit
    └── erase.rs                     # "Erase All Memory" 삭제 로그 연출 → LobbyScene
```

의존 방향: `gfx`/`crt`/`foundation`/`video` 는 서로 독립적인 기반 레이어이고,
`ui` → `apps` → `window_manager`/`scenes` → `main` 순으로 위 계층이 아래를 참조한다.

## 좌표계 메모

- 씬/UI 는 가상 해상도 **640×480** 에서 그린다. CRT 단계가 화면에 4:3 으로 확대.
- 창 안 앱은 **절대 가상좌표**로 그리고, 마우스도 절대좌표로 받는다.
- UI 텍스트는 비트맵 폰트라 **ASCII(영문)만** 렌더된다(한글 등은 안 나옴).

## (되돌려진 시도) 나눔고딕 언어 설정

한때 사용자가 준 나눔고딕 TTF 로 폰트 렌더링 자체를 `fontdue` 기반 가변폭
래스터라이즈 아틀라스로 바꾸고 `Settings` 에 English/한국어 토글을 붙였다가,
사용자 요청으로 통째로 되돌렸다. 되돌리는 과정에서 "이제 다 안 쓴다"고 판단해
지웠던 원래 비트맵 폰트 에셋(`assets/font.png`)을 되살리지 못해 한동안 빌드가
깨져있었다 — git 저장소가 아니라 커밋 기록이 없고, 휴지통에도 없고, Windows
File Recovery 로도(`/regular`/`/extensive` 둘 다) 못 찾을 정도로 완전히
사라졌었다. 다행히 사용자가 `Downloads` 폴더에 있던 이 프로젝트의 예전 사본
(`C:\Users\crack\Downloads\crackhead\assets\font.png`)에서 원본을 찾아줘서
그대로 복사해 살렸다. **교훈**: 코드에서 안 쓴다고 확인되더라도, 직접 만든 게
아니라 사용자가 준 원본 에셋(이미지/폰트/데이터 파일 등)은 `rm` 으로 바로
지우지 말고 옆으로 옮겨두는 정도로만 처리할 것 — 특히 git 이 없는 프로젝트는
되돌릴 안전망이 전혀 없다.

## Email 관련 단어 전부 Mail 로 (코드 전체)

처음엔 화면에 보이는 이름(바탕화면 아이콘/창 제목/토스트)만 "Email" → "Mail"
로 바꿨는데, 사용자가 "email 관련 단어들을 전부 다 mail로 바꿔달라"고 요청해서
코드 전체를 훑어 정리했다:
- **파일**: `apps/email.rs` → `apps/mail.rs`, `assets/icon_email.png` →
  `assets/icon_mail.png`.
- **타입/식별자**: `EmailApp`→`MailApp`, `EmailMsg`→`MailMsg`,
  `FileKind::Email`→`FileKind::Mail`, `IconType::Email`→`IconType::Mail`,
  `AppAction::MarkEmailRead`/`DeskAction::MarkEmailRead`→`MarkMailRead`,
  `Assets::icon_email`→`icon_mail`, 그리고 `email_arrived`/`email_read`/
  `email_timer`/`EMAIL_ARRIVAL_DELAY`/`EMAIL_AUTO_ARRIVE`/`refresh_email_if_open`
  /`email_id` 등 함수·필드·지역변수 이름 전부.
- **주석**: 코드 곳곳의 "Email"/"이메일" 표기도 "Mail"/"메일"로 맞췄다.
- **안 건드린 것**: 메일 발신자 주소 `"PalaceCompany@email.com"` — 이건 스토리
  본문 속 실제 이메일 주소 문자열이라 "email" 이라는 단어 자체가 아니라 도메인
  표기라서 그대로 뒀다.

**저장 파일 호환성**: `FileKind::Email`/`email_arrived`/`email_read` 는
`#[derive(Serialize, Deserialize)]` 로 그대로 저장 파일(JSON)에 실리는
필드/태그라서, Rust 쪽 이름만 바꾸고 `#[serde(rename = "email_arrived")]` 같은
어노테이션으로 JSON 상의 키/태그는 예전 이름 그대로 유지했다 — 그래서 이 이름
변경 전에 저장된 파일도 그대로 계속 불러와진다(값 자체가 리셋되거나 로드
실패로 새 게임이 되는 일이 없다).

## Email UI 리디자인 (Outlook Express 웰컴 화면 참고)

실제 Outlook Express 웰컴 화면 스크린샷을 참고해서 다시 짰다 — "내부는 하얗게,
회색으로 약간 감싸는 느낌, 오른쪽엔 고른 탭 정보, 하단엔 현재 탭 아이템 개수"
라는 요청에 맞춰 손댄 부분:
- **왼쪽 폴더 트리 배경**: 회색(FACE) → 흰색. 이전 세션에 "왼쪽을 회색으로"
  요청받아 바꿔뒀던 걸 이번엔 참고 이미지가 흰 트리를 보여줘서 다시 흰색으로
  되돌렸다 — 방향이 명시적으로 바뀐 재설계라 이전 요청과 모순되지 않는다.
- **오른쪽 내용 패널**: `draw_pane_content()` 맨 앞에 흰색 밑칠을 추가했다 —
  이전엔 빈 상태(폴더 미선택/빈 폴더/새 소식 없음 등) 화면에 배경을 아예 안
  그려서 앱 전체 배경색(회색 FACE) 위에 떠 보였다.
- **전체 테두리**: 트리+내용 패널 전체를 My Computer/휴지통과 같은 요령으로
  옅은 회색 `border()` 로 한 번 감싸서 하나의 패널처럼 보이게 했다. 예전엔
  Inbox 목록 화면에서만 헤더+목록 부분에 좁게 테두리를 둘렀는데, 이제 그건
  이중 테두리가 되어 빼고 훨씬 넓은 범위(트리까지 포함)로 한 번만 두른다.
- **상태바("현재 탭 아이템 개수")**: 로직 자체는 이미 있었다 — Inbox 는 실제
  메시지 수(+안 읽음), 나머지 폴더는 아직 실제 데이터가 없어 정직하게
  "0 Items". 스타일만 새 레이아웃에 맞춰 유지했다.

## 기본 메일 즉시 배달 / Inbox 단일 폴더 / 트리 폭 드래그 조절

- **기본 메일**: `fs.mail_arrived` 초기값을 `false` → `true` 로 바꿔서 게임을
  시작하자마자 받은편지함에 메일이 미리 들어있게 했었는데, 다시 요청받아
  `false` 로 되돌렸다 — 받은편지함이 다시 빈 상태로 시작한다.
  `MAIL_AUTO_ARRIVE` 도 여전히 꺼져 있어서 6초 타이머로도 자동으로 안 채워진다
  (스토리 쪽에서 필요한 시점에 직접 켜는 트리거를 만들 계획인 듯).
- **Inbox 만 남기고 나머지 폴더 제거**: Outbox/Sent Items/Deleted Items/Drafts
  는 눌러도 항상 "이 폴더엔 항목이 없습니다" 뿐이라 트리에서 뺐다. `MailFolder`
  열거형/`MAIL_FOLDERS` 배열은 그대로 두고 variant 만 `Inbox` 하나로 줄였다 —
  나중에 스토리 진행상 다른 폴더가 실제로 필요해지면 그때 다시 추가하면 된다
  (구조 자체를 걷어내지 않고 축소만 해서, 되돌리기 쉽게 남겨뒀다).
- **왼쪽 트리 폭 드래그 조절**: `MailApp` 에 `tree_w`/`divider_drag` 필드를
  추가하고, `explorer.rs` 의 사이드바 폭 조절과 똑같은 요령(구분선 위에서
  누르면 드래그 시작, 마우스를 떼면 끝, 최소폭/창폭 비율로 상한 클램프)으로
  트리↔내용 사이 세로줄을 드래그할 수 있게 했다.

## Email UI 미세조정 (스크린샷 피드백)

- **From/Subject 헤더를 다시 튀어나오게**: 예전엔 "이중 입체 테두리가 튀어나와
  보인다"는 피드백으로 flat 바로 바꿨었는데, 이번엔 반대로 My Computer(explorer.rs)
  처럼 `raised()` 두 칸으로 다시 튀어나온 느낌을 달라는 요청 — 그대로 되돌렸다.
- **목록 행을 흰색으로**: 행마다 `sunken()` 으로 회색 박스를 그리던 걸 뺐다 —
  pane 배경이 이미 흰색이라(지난 리디자인 때 추가) 행 자체엔 안 칠해도 된다.
- **읽기 화면의 회색 부분들을 흰색으로**: From/Sent/To/Cc/Subject 필드 값 칸,
  본문 박스, 첨부파일 박스가 전부 `sunken()`(안이 회색으로 칠해짐)이었던 걸
  흰 배경 + 얇은 회색 `border()` 로 바꿨다 — 칸 경계는 그대로 보이되 안은 흰색.

## 첫 메일 자동 도착 꺼둠

스토리 파트 작업을 시작하면서, 데스크톱에 들어오고 6초 뒤 자동으로 도착하던
테스트용 메일(Photo QA Request)이 매번 알림/토스트로 방해가 돼서 껐다.
`desktop.rs::MAIL_AUTO_ARRIVE` 를 `false` 로 두면 `mail_timer` 로직 자체가
안 돈다(`fs.mail_arrived` 가 계속 false 로 남아 Mail 받은편지함도 빈 상태) —
다시 필요해지면 이 상수 하나만 `true` 로 되돌리면 된다.

## 한글 폰트(Terrarum Sans Bitmap) 적용

저작권 걱정 없는 한글 폰트를 찾아 TTF 실시간 래스터라이즈 방식으로 붙이기까지
여러 후보를 거쳤다 — Neo둥근모("너무 딱딱하다") → 갈무리(GalmuriMono9, "일부
흐려 보인다"는 문제를 Nearest 필터/작은 해상도+정수 확대/픽셀 스냅으로
고쳐봤지만 결국 "예전 `font.png` 랑 느낌이 다르다"고 전체를 한 번 되돌림) →
최종적으로 **Terrarum Sans Bitmap**으로 정착했다. 게임 개발자(CuriousTorvald)
가 자기 게임에 쓰려고 직접 만든 폰트라 "비디오 게임을 위한 진짜 멀티링구얼
비트맵 폰트"를 표방하고, `fontdue` 로 실제 래스터라이즈해본 결과(계단식으로
뚝뚝 끊기는 픽셀 곡선)가 참고 이미지의 느낌과 가장 가까웠다. 완성형 한글
11,172자를 초성/중성/종성 조각 조합 방식(옛 도스 한글 비트맵 폰트와 같은
원리)으로 그린다. 라이선스는 SIL OFL 1.1(폰트 자체를 파일 하나만 따로
파는 것만 금지, 게임에 임베드해서 배포/판매하는 건 자유) — `assets/
TerrarumSansBitmap-LICENSE.md` 에 전문을 같이 넣어뒀다.

**"있는 그대로 출력해달라"는 피드백으로 가공을 다 걷어냈다**: 중간에
"흐림을 없애겠다"고 작은 해상도로 그려 정수배 확대하거나 알파를 이진화(0/255
로 딱 자름)하는 시도까지 해봤는데, 그 과정에서 폰트 원래 모양이 오히려
왜곡되고(작은 해상도에서는 라틴 문자 일부가 아예 뭉개졌다) 요청과도 안 맞는
방향이라 전부 되돌렸다. 지금은 `gfx.rs::build_font_atlas` 가 `FONT_PX`(22,
`CELL_H` 와 동일) 로 fontdue 가 그려주는 안티에일리어싱 커버리지를 그대로
텍스처에 담아, 글자 모양 자체는 전혀 안 건드린다.

다만 이걸 화면에 그릴 때 처음엔 Linear 필터를 썼는데, CRT 셰이더(색수차 등)와
겹치니 너무 흐릿해서 "픽셀 느낌이 안 난다"는 피드백을 받고 다른 텍스처(아이콘
등)와 같은 Nearest 로 바꿨다 — 이미 그려둔 텍스처를 "어떻게 표시할지"만
바꾸는 것이라, 글자 모양 자체를 가공하는 것(작게 그려 확대/알파 이진화 등,
전부 되돌린 것들)과는 성격이 다르다.

원본 OTF(9.4MB)에는 fontdue 가 못 읽는 임베디드 비트맵(EBDT/EBLC)과 색상
테이블(COLR/CPAL)이 같이 들어있어서, `fonttools` 로 그 테이블들만 지워
7.2MB 로 줄인 뒤 `assets/TerrarumSansBitmap.otf` 로 임베드했다(실제 쓰는
아웃라인 데이터는 그대로라 렌더링 결과는 동일).

## 로컬라이제이션 (영어/한국어/일본어)

Terrarum Sans Bitmap 이 완성형 한글뿐 아니라 라틴/히라가나/가타카나/한자까지
포함한 "진짜 멀티링구얼" 폰트라는 걸 확인해뒀던 게(위 항목 참고) 여기서
그대로 쓸모가 있었다 — 새 폰트를 안 찾아도 일본어 문자를 그릴 수 있는지부터
먼저 검증했다. `fontdue` 로 히라가나(あいうえおん)/가타카나(アイウエオン)/
한자(日本語設定言) 를 실제로 래스터라이즈해보는 스크래치패드용 테스트 도구를
만들어 확인한 결과, 전부 `glyph_index` 가 0이 아니고(= 폰트 안에 그 글자가
있다는 뜻) 유효한 비트맵으로 그려졌다. `gfx.rs::build_font_atlas` 에는
히라가나(U+3040~U+309F)/가타카나(U+30A0~U+30FF) 전체 범위(각 96자라 통째로
넣어도 부담 없음)와, 실제로 쓰는 문자열에서 뽑은 한자만 담은
`KANJI_CHARSET` 상수를 추가했다 — 이전에 완성형 한글 11,172자를 통째로
넣었던 것과 달리 한자는 한 글자를 그릴 때마다 아틀라스 텍스처 크기가 커지므로,
"쓰는 것만" 담는 방식을 골랐다(로컬라이제이션 문자열이 늘어나 새 한자를 쓰게
되면 이 상수에도 같이 추가해야 함 — 안 그러면 그 글자만 화면에 안 그려짐).

`foundation.rs` 에 `Language` enum(`En`/`Ko`/`Ja`, 기본값 `En`)과
`tr(lang, en, ko, ja) -> &'static str` 헬퍼 함수를 추가했다 — 화면 곳곳의
문자열 리터럴을 `tr(lang, "English string", "한국어 문자열", "日本語文字列")`
로 감싸기만 하면 되는 아주 얇은 함수다. `Settings` 구조체에는
`#[serde(default)] pub language: Language` 필드를 추가해서, 이 필드가 없던
예전 저장 파일도 (기본값 `En` 으로) 문제없이 계속 불러와진다.

지금 이 헬퍼가 실제로 적용된 화면은 **Settings 창**(`src/apps/settings.rs`)
전체다 — 탭 이름(Graphics/그래픽/グラフィック 등), 그룹박스 제목, 슬라이더/
체크박스/버튼 라벨까지 전부 3개 언어로 갈아끼워진다.

**Interface 탭을 로비에서도 볼 수 있게 함**: 처음엔 "로비에는 바탕화면이
없으니 Interface 탭(바탕화면 색상 등)은 뺀다"고 로비의 Settings 패널에서
그 탭 자체를 숨겼었는데, 언어 설정이 그 탭 안에 있다 보니 로비에서는 언어를
못 바꾸는 문제가 생겼다 — `SettingsApp::new_with_tabs` 를 호출하는
`lobby.rs` 쪽 `show_interface` 인자를 `false`→`true` 로 바꿔서 로비에서도
Interface 탭(과 그 안의 언어 아코디언)이 보이도록 했다. 바탕화면 색상
스와치는 로비 맥락에선 좀 안 맞지만, 탭 하나를 통째로 숨기는 것보다는
낫다고 판단했다.

**언어 선택을 아코디언으로 변경**: 처음엔 "English/한국어/日本語" 세
버튼짜리 토글 줄로 만들었는데, Graphics 탭의 Resolution/Frame rate 와
생김새가 다르니 "그래픽 설정처럼 아코디언으로 해달라"는 요청을 받고
`accordion_header`+`accordion_list` 조합(Resolution/Frame rate 와 완전히
같은 컴포넌트)으로 바꿨다. 언어 이름은 지금 UI 언어가 무엇이든 항상 그
언어 자신의 표기(`LANG_NAMES = ["English", "한국어", "日本語"]`)로 보여준다
— 그래야 지금 UI가 읽을 수 없는 언어로 되어 있어도(예: 실수로 일본어를
선택한 한국어 사용자) 목록에서 자기 언어를 알아보고 되돌아올 수 있다.
Interface 탭 전체가 이제 아코디언이 펼쳐질 때 아래 내용(배경색 등)이
밀려 내려가야 해서, Graphics 탭과 똑같이 뷰포트+스크롤바 구조로 다시
짰다(로비의 좁은 Settings 패널에서도 안 잘리도록).

**Erase All Memory 는 지울 데이터가 있을 때만 눌리게 함**: 로비에서
저장된 진행 상태가 아예 없는 상태로 Settings 를 열어도 이 버튼이 눌려서
`crate::foundation::delete()` 가 (지울 파일도 없는데) 실행되는 게
어색했다. `SettingsApp` 에 `has_data: bool` 필드를 추가해서 — 데스크톱
안에서 여는 설정창(`SettingsApp::new`)은 이미 세이브가 로드된 상태이므로
항상 `true`, 로비의 설정창(`new_with_tabs`)은 `Continue` 버튼과 같은
기준인 `self.has_save` 를 그대로 넘긴다. `has_data == false` 면 버튼을
누를 수 있는 `button()` 대신 회색 텍스트의 비활성 모양(raised, 클릭 무시)
으로 그린다.

다른 화면(부팅, 바탕화면, Mail, Recycle Bin 등)은 아직 `tr()` 을 안 붙였고
전부 영어/한국어 혼용 상태 그대로다 — Settings 창을 먼저 전체 번역해서
3언어 배선(폰트 문자셋 포함)이 실제로 화면에 잘 나오는지부터 검증한 것이고,
나머지 화면은 다음 작업에서 이어서 옮기면 된다.

## 로컬라이제이션 2차 작업: 빈 문자 버그 수정 + 전체 화면 번역

**빈 문자 버그의 원인**: "일본어 일부 글자가 비어 보인다"는 제보를 받고,
Settings 창에서 실제로 쓰는 모든 `tr(lang, en, ko, ja)` 호출의 `ja` 문자열을
스크립트로 뽑아 `gfx.rs::KANJI_CHARSET` 과 대조해봤더니 `量`(음량의
"량") 한 글자가 통째로 빠져 있었다 — "Volume"/"Mp4 Sound" 슬라이더 라벨
번역("음량", "동영상 음량")에 이 글자를 쓰는데, 폰트 아틀라스에 없는
문자라 그 자리만 완전히 빈 칸으로 그려졌던 것. 정작 폰트(Terrarum Sans
Bitmap) 자체는 `fonttools` 로 확인해보니 히라가나/가타카나/완성형 한글/
KANJI_CHARSET 문자 전부(량 포함) 갖고 있었다 — 문제는 폰트가 아니라
"이 글자를 아틀라스에 구워 넣으라고 알려주는 목록"이 불완전했던 것뿐이었다.

**전체 화면 번역**: "전체적으로 전부 번역해달라"는 요청을 받고, 지금까지
Settings 창에만 있던 `tr()`/`Language` 배선을 로비/부팅/데스크톱(시작메뉴·
우클릭메뉴·시스템 메시지 패널·와이파이 팝업·새 메일 토스트·Erase 확인창·
바탕화면 아이콘 라벨)·File Explorer·Mail·Recycle Bin·Notepad(내용은 그대로,
UI 없음)·HexTool·HexTool Setup.exe(설치 마법사)·Official Site·.lock 비밀번호
창·Credits·.tar 압축파일 안내·동영상 플레이어까지 전체 화면으로 확장했다.
번역 대상은 진짜 UI 크롬(메뉴/버튼/라벨/대화상자 문구)으로 한정했고, 아래
세 부류는 의도적으로 그대로 뒀다:
- **BIOS POST/종료/삭제 로그**(`boot.rs`/`shutdown.rs`/`erase.rs`) — 실제
  컴퓨터도 BIOS POST 화면은 어느 언어의 Windows에서든 항상 영어로 나오므로,
  "리얼함"을 위해 일부러 그대로 뒀다(단, OS 자체가 사용자에게 건네는
  "Welcome to PalaceOS" 인사말은 진짜 Windows 처럼 언어를 따라가게 번역함).
- **스토리 소품 콘텐츠** — 실제 이메일 본문(Photo QA Request 메일 전체),
  `.txt` 메모장 내용물, HexTool Setup.exe 의 가짜 EULA 전문, 실제 "파일
  이름"(`Photos.tar`, `photo01.jpg` 등)은 실제 OS에서도 파일명이나 수신
  메일 내용이 시스템 언어를 따라 안 바뀌므로 원문 그대로 둔다.
- **브랜드명** — "PalaceOS" 자체는 어느 언어에서도 안 바뀐다.

**시스템 특수 이름의 이중 정체성**: "My Computer"/"Recycle Bin"/"Mail" 처럼
가짜 파일시스템(`FileNode::name`)에 저장된 이름은 `find_by_name("Recycle
Bin")`/드래그앤드롭 목적지 판정/저장 파일 등 코드 전체에서 영어 문자열
그대로를 식별자로 계속 써야 해서, 실제 데이터는 그대로 두고 화면에 보여줄
때만 `foundation::display_name(lang, name)` 을 거치게 했다 — 진짜 Windows
도 "내 컴퓨터" 같은 특수 폴더를 내부적으로는 로케일과 무관한 CLSID 로
식별하고 표시 이름만 번역하는 것과 같은 방식이다. File Explorer 의 고정
카테고리 탭(Downloads/Desktop/Videos/Images)도 마찬가지 이유로
`foundation::category_label(lang, name)` 을 따로 두어, 드래그앤드롭
목적지 매칭(`"Desktop" => MoveDest::Desktop` 등)에 쓰는 탭 이름 자체는
번역하지 않고 화면에 그릴 때만 번역한다.

**로비/데스크톱의 Settings 진입점을 Interface 탭까지 포함하도록 바꿈**:
언어 설정이 Interface 탭 안에 있다 보니, 이 작업 전에 이미 "로비에서도
Interface 탭이 보여야 한다"는 요청으로 로비 쪽 `show_interface` 를
`true` 로 바꿔둔 상태였다(위 항목 참고) — 이번 작업은 그 위에서 나머지
화면들의 문구를 채워나간 것이다.

**KANJI_CHARSET 을 스크립트로 완전 재생성**: 손으로 하나씩 추가하다 보면
이번처럼 글자 하나가 빠지는 실수가 또 나올 수 있어서, 이번엔 저장소 전체
`.rs` 파일을 정규식으로 훑어 모든 `tr(lang, en, ko, ja)` 호출의 `ja` 인자
128자 한자를 전부 뽑아 `KANJI_CHARSET` 을 통째로 다시 생성했다(그 다음
`fonttools` 로 128자 전부 폰트에 실제로 존재하는지 재확인). 앞으로 새
일본어 문자열을 추가할 때도 이 방식(전체 스캔 후 재생성)을 쓰면 이런
누락을 원천적으로 막을 수 있다.

## 로컬라이제이션 3차: 설정 즉시 저장 + 실시간 언어 전환 + 관련 버그 두 건

**언어(취향 설정)를 게임 진행 저장과 분리된 파일로 즉시 저장**: 예전엔
`Settings` 가 `palaceos_save.json`(게임 진행 저장) 안에만 같이 들어있어서,
① 아직 한 번도 게임을 시작하지 않아 저장 파일 자체가 없는 로비 화면에서는
언어를 바꿔도 디스크에 남을 곳이 없었고, ② 게임 중에 바꿔도 5초 주기
자동저장(`desktop.rs::AUTOSAVE_INTERVAL`)을 기다려야 반영됐다.
`foundation.rs` 에 `palaceos_settings.json` 전용 파일과
`save_settings()`/`load_settings()` 를 새로 추가해서, 언어 아코디언에서
항목을 고르는 그 즉시(`settings.rs`) 이 파일에 저장한다. `main.rs` 의
초기 로딩 순서도 이 파일을 최우선으로 바꿨다(`load_settings().or_else(||
load().map(|s| s.settings))`) — 게임 진행 저장이 없어도 언어/그래픽
취향은 계속 남는다. "Erase All Memory"(로비/데스크톱 양쪽)는 이 파일을
안 건드린다 — 진행 상황을 지운다고 언어까지 초기화되면 당황스럽다.

**열려있는 창도 언어를 바꾸는 즉시 반영**: 대부분의 앱은 원래도
`Rc<RefCell<Settings>>` 를 들고 매 프레임 `self.settings.borrow().language`
를 새로 읽으므로 내용은 이미 실시간으로 바뀌었지만, `CreditsApp`/
`ArchiveApp`/`OfficialSiteApp` 세 개는 지난 로컬라이제이션 작업 때 "열
때 언어를 한 번 스냅샷해두는 걸로 충분하다"고 판단해 `Language` 값
하나만 들고 있었다 — 이번 요청("중간에 언어설정을 만지면 바로 그 언어로
변하도록")으로 그 판단을 뒤집고 셋 다 다른 앱들과 똑같이
`Rc<RefCell<Settings>>` 를 들게 바꿨다. 창 **제목**(타이틀바 문구)은
원래 `apps/mod.rs::open()` 이 창을 열 때 한 번만 정해서
`window_manager.rs::Window.title` 에 고정 `String` 으로 박아두는
구조라 내용과 달리 실시간으로 안 바뀌었는데, `App` 트레잇에
`fn title(&self) -> Option<String> { None }` 기본 메서드를 추가하고
`WindowManager::frame()` 이 매 프레임 각 창의 `app.title()` 을 물어봐서
`Some` 이면 타이틀바 문구를 그 자리에서 갈아끼우게 했다. Settings/Mail/
Recycle Bin/Credits/Official Site 창이 이 메서드를 오버라이드한다 —
나머지 창들(Notepad/HexTool/Installer 등)은 제목이 애초에 번역 대상이
아닌 실제 파일 이름이라 그대로 둔다.

**버그: 로비에서 New Start/Erase All Memory 를 누르면 방금 바꾼 설정이
초기화됨**: `lobby.rs`(New Start, Settings 패널의 Erase All Memory)와
`desktop.rs`(Erase All Memory 확인) 세 곳 모두 저장 파일을 지우면서
`*f.settings.borrow_mut() = Settings::new();` 로 지금 켜져있는 설정
객체까지 기본값으로 덮어쓰고 있었다 — 이게 정확히 "로비에서 설정을
바꾸고 게임을 시작하면 설정이 초기화되는" 원인이었다. 위에서 설정을
게임 진행과 분리된 파일로 옮긴 김에, 세 곳 다 이 줄을 지웠다 —
"게임 진행을 지운다"와 "언어/그래픽 취향을 지운다"는 이제 완전히 별개의
동작이라 삭제해도 안전하다(게임 진행 삭제만 남는다).

**버그: My Computer 안에서 더블클릭으로 프로그램을 열면 바탕화면
아이콘이 사라짐**: `apps/explorer.rs` 의 File Explorer 는 항목을 사이드바
(다른 카테고리)로 드래그해서 옮기는 기능이 있는데, "지금 파일을 드래그
중인지"(`item_drag`)를 클릭 시작점 기준 4px 이동으로 판정하고, 마우스를
뗀 프레임에 `!area.contains(win.mouse)` 면 "창 밖으로 드롭했다"로 보고
바탕화면으로 옮겨버린다(`AppAction::MoveFiles`). 문제는 더블클릭으로
항목을 열면 **두 번째 클릭이 처리되는 바로 그 프레임에 새 창이 뜨면서**
File Explorer 창이 그 순간 포커스를 잃는다는 것 — `window_manager.rs`
는 포커스를 잃어 다른 창에 가려진(occluded) 창에는 실제 마우스 좌표
대신 화면 밖 좌표(`-1e6, -1e6`, "이 창에서는 아무것도 히트되면 안
된다"는 신호용 값)를 넘겨준다. 그런데 explorer.rs 의 드래그-드롭 판정
코드는 이 값의 "의미"(히트테스트 무효화용)를 모른 채 그냥 좌표로만
써서, "마우스가 화면 밖 저 멀리(-100만, -100만)로 나갔다" → "창 밖으로
드롭했다" → "선택된 항목(방금 연 프로그램)을 바탕화면으로 옮겨라" 로
잘못 해석했다. 그 결과 방금 연 프로그램이 실제 마우스 커서 위치(=새로
뜬 창에 가려진 자리)로 "이동"되면서 마치 바탕화면에서 삭제된 것처럼
보였다(사실은 새 창 뒤에 숨겨진 채 다른 자리로 옮겨간 것). 고친 방법:
드롭 판정에 `win.focused` 조건을 추가해서, 포커스를 잃은(=좌표를 믿을
수 없는) 프레임에는 드롭 자체를 처리하지 않게 했다 — `item_drag` 정리는
그대로 진행되니 다음 상호작용에는 영향이 없다.

## 버그: 시작메뉴/우클릭메뉴/시스템 메시지 등 오버레이 뒤에 창이 있으면 커서가 그 창 것으로 바뀜

시작메뉴, 바탕화면 우클릭 메뉴, 시계를 눌러 여는 시스템 메시지 패널,
와이파이 팝업, 새 메일 토스트는 전부 `window_manager.rs` 가 관리하는
"창" 목록에 속하지 않고, `desktop.rs` 가 `self.wm.frame(...)` 호출이 다
끝난 뒤에 그 위에 직접 그리는 오버레이다. 반면 마우스 커서 모양(리사이즈
화살표 등)은 `wm.frame()` 안에서 "지금 마우스 아래 가장 위에 보이는
창의 테두리인지"만 보고 그 호출 시점에 미리 계산해버린다 — 그러다 보니
이 오버레이들 중 하나가 화면에 떠 있고, 마침 그 뒤에 있는(가려진) 창의
리사이즈 테두리와 자리가 겹치면, 눈에는 분명 메뉴/패널이 보이는데 커서만
그 아래 창의 리사이즈 화살표로 바뀌는 문제가 있었다 — `wm.frame()` 은
자기 다음에 desktop.rs 가 그 위에 뭘 더 그릴지 전혀 모르기 때문이다.

고친 방법: `update()` 끝의 커서 결정 로직에서, 지금 열려있는 오버레이
(시작메뉴/우클릭메뉴/시스템 메시지 패널/와이파이 팝업/새 메일 토스트/
Erase All Memory 확인창) 위에 마우스가 있는지를 먼저 확인해서, 있으면
`wm_cursor`(창 테두리 기준 계산값)를 무시하고 `Arrow` 부터 다시 시작하게
했다. 그중 실제로 클릭 가능한 항목(시작메뉴/우클릭메뉴의 행)은 바로
다음 줄의 기존 Hand 판정에서 다시 `Hand` 로 바뀐다 — 새 메일 토스트
자리를 이 판정과 그리기 양쪽에서 어긋나지 않게 같이 쓰려고
`DesktopScene::toast_rect()` 헬퍼도 새로 뽑아냈다(messages_panel_rect/
wifi_popup_rect 와 같은 요령).

## 버그: 언어에 따라 문단이 화면/컨테이너 밖으로 넘쳐 잘림 (줄바꿈 알고리즘 전면 교체)

휴지통 왼쪽 안내 패널의 일본어 문구가 오른쪽 끝이 잘려 보인다는 스크린샷
제보를 받았다. 원인은 `recycle_bin.rs`(와 `notepad.rs`/`mail.rs`/
`installer.rs`/`archive.rs`/`hextool.rs`/`erase.rs`까지 코드베이스 전체에
퍼져있던 같은 패턴): 문단을 줄바꿈할 때 "글자 하나당 평균 폭(라틴 문자
'M' 한 글자 폭, 또는 `ADVANCE` 라는 고정 상수)"으로 컨테이너 폭을 나눠
"이 줄에 몇 글자가 들어가는지" 를 어림잡은 뒤, 그 글자 **수**만큼만 잘라
다음 줄로 넘기고 있었다. 이 어림값은 라틴 문자 기준이라, 그보다 실제
렌더링 폭이 훨씬 넓은 한글/한자/가나 글자가 섞이면 "글자 수는 안 넘었지만
실제 픽셀 폭은 이미 컨테이너를 넘어선" 줄이 만들어져 오른쪽 끝이 그대로
잘려 보였다 — 로컬라이제이션으로 한/일 문구가 늘어나면서 이 문제가 실제로
드러난 것.

`ui.rs` 에 이미 `wrap_two_lines`/`split_line_once` 라는, 문자 수 어림값이
아니라 `Renderer::text_width()` 로 **실제 렌더링 폭을 그때그때 재면서**
접는(공백이 있으면 단어 단위로, 단어 자체가 너무 길거나 애초에 공백이
없는 CJK 문장이면 글자 단위로 폴백) 정확한 알고리즘이 있었는데, 딱 두
줄까지만 접는 용도(아이콘 라벨 등)로만 쓰이고 있었다(코드에 "항상 다
보이게 하고 싶으면 대신 wrap_lines 를 쓴다" 는 주석까지 있었지만 정작
`wrap_lines` 자체는 아직 구현돼 있지 않았다). 이번에 그 주석대로
`split_line_once` 를 필요한 줄 수만큼 반복해서 접는 `ui::wrap_lines(r,
text, scale, max_w)` 를 새로 만들고, 코드베이스 전체에 흩어져 있던
"글자 수 어림값" 방식의 줄바꿈(`notepad::wrap_text` 와 각 앱이 저마다
`ADVANCE`/`"M"` 폭으로 `max_chars` 를 구하던 코드)을 전부 이 함수 하나로
바꿔 끼웠다 — Mail 본문, 설치 마법사 안내문/EULA, 휴지통 왼쪽 패널,
압축파일/HexTool 안내문, 메모장, 심지어 "Erase All Memory" 연출의 콘솔
로그(실제 exe 경로를 그대로 출력하는데, 사용자 폴더 이름에 한글이 섞여
있으면 이것도 똑같이 잘릴 수 있었다)까지 전부 대상이었다. 이제 언어가
무엇이든, 텍스트가 실제로 그려질 폭을 기준으로 정확히 접이므로 화면이나
컨테이너 밖으로 넘쳐 잘리는 일이 없다.

## 버그: 사진/동영상을 휴지통에 버려도 Videos/Images 탭에 그대로 남아 보임

사진(`photo01.jpg` 등)을 File Explorer 의 "画像"(Images) 탭에서 휴지통으로
드래그했더니, 휴지통 창에도 들어가 있고 Images 탭에도 그대로 남아서 마치
같은 파일이 두 군데 동시에 있는 것처럼 보인다는 스크린샷 제보를 받았다.

원인은 Downloads/Desktop 탭과 Videos/Images 탭이 서로 다른 방식으로
목록을 만든다는 데 있었다: Downloads/Desktop 은 `fs.downloads`/`fs.desktop`
이라는 진짜 "이 파일들이 여기 들어있다" 는 명단이라 휴지통으로 옮기면
그 명단에서 실제로 빠지지만, Videos/Images 는 그런 명단이 아예 없이
`fs.all_of_kind(FileKind::Mp4/Img)` 로 "어디 있든 이 **종류**인 파일
전부" 를 그때그때 훑어서 보여주는 가상 탭이다. 사진을 휴지통(진짜
`FileKind::Folder`)의 children 에 추가해도, 그 사진 노드 자체의 종류는
여전히 `FileKind::Img` 이므로 Images 탭에서는 계속 걸려서 보였다 —
휴지통에 들어갔다는 사실 자체를 Videos/Images 가 전혀 몰랐던 것.

고친 방법: `foundation.rs` 에 `FileSystem::in_recycle_bin(id)` 를 추가해서
"이 파일이 지금 이름이 정확히 Recycle Bin 인 폴더의 children 안에 있는지"
를 확인하고, `apps/mod.rs::explorer_tabs()` 가 Videos/Images 목록을 만들
때 이 함수로 휴지통에 들어간 항목을 걸러내게 했다.

**복구 위치 기록도 같이 손봤다**: `photo01.jpg` 처럼 애초에 바탕화면/
Downloads/어떤 폴더의 children 에도 안 속하고 순전히 종류만으로
가상 탭에 보이던 파일은, 휴지통에 넣기 직전 위치를 찾는 `FileSystem::
locate()` 가 (그 어떤 명단에도 없으니) `None` 을 돌려준다 — 예전 코드는
이 경우 `trash_origin` 에 아예 기록을 안 남겼는데, 그러면 "기록이 원래
없던(이 기능이 생기기 전에 이미 휴지통에 있던) 파일" 과 구분이 안 돼서
복구 시 엉뚱하게 바탕화면으로 복구돼버렸다. `FileOrigin::Loose` 라는
값을 새로 추가해서 이 경우를 명시적으로 기록하고, 복구할 때는
(`Loose` 면) 아무 데도 새로 안 넣는다 — 휴지통 children 에서 빠지는
순간 `in_recycle_bin` 필터를 다시 통과하니 Images/Videos 탭에 저절로
다시 나타난다(억지로 어딘가에 넣으면 오히려 또 중복으로 보인다).

## 작업표시줄 시계 위치 조정

시계 숫자("12:19" 등)가 sunken 박스 위아래로 살짝 삐져나와 보인다는
스크린샷 제보를 받았다. 원인 두 가지: ① `clock_rect()` 의 높이가
`TASKBAR_H-10`(18px)로, 글자 한 줄이 실제로 차지하는 높이(`CELL_H*0.9`
≈ 20px)보다 낮게 잡혀 있었다. ② 글자를 그릴 때 `ck.y` 를 그대로 y 좌표로
써서(세로 중앙 정렬 없이 위쪽 기준), 그 좁은 박스 안에서도 자리를 제대로
못 잡고 있었다. 높이를 시작 버튼/창 버튼과 같은 `TASKBAR_H-6`(더 넉넉한
값)으로 맞추고, 다른 요소들과 같은 방식(`ck.y + (ck.h - CELL_H*0.9) /
2.0`)으로 세로 중앙 정렬을 넣었다 — 폭도 62→70px 로 살짝 넓혀 "88:88"
같은 폭이 넓은 시각에도 여유가 있게 했다. 와이파이 아이콘 자리(`wifi_rect`)
는 `clock_rect()` 의 높이를 그대로 이어받아 계산되므로 따로 손대지
않아도 같이 커졌다. 이후 "숫자가 왼쪽 테두리에 너무 붙어 보인다"는 추가
피드백을 두 차례 받아 왼쪽 여백을 5→9→13px 로 조금씩 더 넓혔다(오른쪽
clip 폭도 그만큼 같이 줄여서 균형을 맞춤).

## Mail 답장(Reply) 기능 추가

메일을 읽는 화면에서 답장을 작성하고 보내면 게임상으로 그 사람에게 메일이
전달된 것으로 처리되는 기능을 추가했다.

**구조**: 메일 읽기 화면(`draw_pane_content`)의 "◀ Back to Inbox" 줄
오른쪽 끝에 "Reply" 버튼을 추가했다. 누르면 `MailApp` 에 새로 추가한
`compose: Option<ComposeState>` (어떤 메일에 대한 답장인지 `msg_idx`,
지금까지 입력한 본문 `body`, 전송 완료 여부 `sent`) 가 채워지면서 본문/
첨부 화면 대신 작성 화면(`draw_compose`)으로 바뀐다. To/Subject 는
원본 메일에서 그대로 가져와 읽기전용으로 보여주고("Re: " 접두어를 붙인
제목), 본문만 입력할 수 있다.

**입력**: password.rs 의 입력칸과 같은 관례(포커스일 때만 `win.input.typed`
를 받고, Backspace 로 지운다)를 여러 줄로 확장했다 — 다만 줄 중간에
커서를 옮기는 기능은 없이 항상 맨 끝에서만 추가/삭제된다(Enter 는 줄바꿈).
최대 글자 수(`REPLY_MAX_CHARS` = 1000)는 바이트가 아니라 글자(char) 수로
세는데, 한/일 문자는 UTF-8 로 3바이트씩 차지해서 바이트 기준으로 재면
언어에 따라 실제 입력 가능한 분량이 달라져 버리기 때문이다. 본문은
`wrap_lines()`(다국어 대응 줄바꿈, 위 항목 참고)로 접어서 보여주고, 커서
위치를 못 옮기니 스크롤도 그냥 "항상 맨 아래 줄이 보이게" 고정했다 —
깜빡이는 커서는 마지막 줄의 실제 글자 폭(`text_width`)만큼 뒤에 그린다.

**전송**: "Send" 는 본문이 비어있지 않을 때만 눌린다(Submit 버튼 등과
같은 비활성 회색 표시 패턴). 누르면 `ComposeState.sent = true` 로
바뀌어 이 화면이 "메일을 보냈습니다" 확인 화면으로 바뀌고,
`AppAction::SendMail(msg_idx)` 를 돌려준다 — `MarkMailRead`/`Download`
와 완전히 같은 경로(`window_manager.rs` 의 `DeskAction::SendMail` →
`desktop.rs`)로 `fs.mail_replied: Vec<usize>` (새 필드, `mail_read` 와
같은 이유로 `#[serde(default)]`) 에 즉시 기록하고 저장한다 — 그래야
새로고침이나 재시작 후에도 "Reply" 버튼이 회색 "Replied" 표시로 남는다
(첨부파일의 "Downloaded" 와 같은 패턴). 아직 실제로 다시 답장이 오는
스토리 연출은 없다 — `STORY.md` 5장의 "메일 전송 실패" 같은 향후 스토리
훅에서 이 `mail_replied` 기록을 그대로 재사용할 수 있게 기반만 마련해둔
것이다.

새 일본어 문구("返信"/"返信済み" 등)에서 쓰는 한자 `返` 을 KANJI_CHARSET 에
추가하는 것도 잊지 않았다(전체 스캔 스크립트로 재확인).

## Mail 에 "Write Mail" 탭 추가 — 답장이 아니라 새 메일을 직접 작성

Reply 는 이미 온 메일에 대한 답장만 가능해서, 받는 사람/제목을 직접 정해
완전히 새로운 메일을 쓸 수 있는 전용 탭을 왼쪽 폴더 트리에 추가해달라는
요청을 받았다.

**폴더 트리 확장**: `MailFolder` 열거형에 `Compose` 를 추가하고
`MAIL_FOLDERS` 를 `[Inbox, Compose]` 로 늘렸다 — 트리에는 "Write Mail"
(메일 쓰기/メール作成) 로 표시되고 아이콘은 봉투(Inbox 의 `Envelope`)와
구분되도록 일반 `Mail` 아이콘을 쓴다. 안 읽은 메일 개수 배지는 원래
"어떤 folder 든 똑같이 적용"되고 있었는데, 그러면 Write Mail 옆에도
Inbox 의 안 읽음 숫자가 그대로 붙어버려서 `folder == Inbox` 일 때만
계산하도록 고쳤다.

**Reply 와의 차이 — 필드 세 개를 다 직접 입력**: Reply(`ComposeState`,
`draw_compose`) 는 To/Subject 를 원본 메일에서 그대로 가져와 읽기전용으로
보여주고 본문만 입력받으면 됐지만, Write Mail 은 받는 사람/제목도 직접
타이핑해야 한다. 이를 위해 별도의 `NewMailState { to, subject, body,
active: ComposeField, sent }` 상태와, 클릭하면 그 필드가 활성화되는
(테두리가 남색으로 바뀌고 깜빡이는 커서가 붙는) `draw_editable_field()`
헬퍼를 새로 만들었다. 세 필드 중 어디로 타이핑이 들어갈지는
`active: ComposeField(To/Subject/Body)` 하나로 추적한다 — 필드를 클릭하면
그 필드로 전환되고, To/Subject 에서 Enter 를 누르면(줄바꿈이 의미 없는
한 줄짜리 필드라) 다음 필드로 넘어가며, Body 에서만 Enter 가 진짜
줄바꿈이 된다.

**초안은 탭을 오가도 유지, 창을 닫으면 사라짐**: `new_mail: NewMailState`
는 `MailApp` 이 살아있는 동안(Inbox ↔ Write Mail 탭을 오가도) 그대로
남아있지만, `fs` 에는 저장하지 않는다 — 실제 메일 클라이언트도 임시보관함에
따로 저장하지 않는 한 창을 닫으면 작성 중이던 초안이 날아가는 것과 같은
동작이다. "Send" 를 누르면(받는 사람/본문이 둘 다 채워졌을 때만 활성화 —
제목은 진짜 메일처럼 비워도 된다) `AppAction::SendNewMail` 을 돌려주고,
`DeskAction::SendNewMail`(`window_manager.rs`) → `desktop.rs` 경로로
`fs.mail_sent_count: u32`(새 필드) 를 하나 늘려 즉시 저장한다. Reply 는
"어떤 메일에 대한 답장인지" 인덱스가 있어야 해서 `Vec<usize>` 로
기록했지만, Write Mail 은 매번 완전히 새로운 발신이라 그런 구분이 필요
없어서 그냥 누적 카운트만 남긴다 — 아직 이 값을 보고 반응하는 스토리
로직은 없지만, "플레이어가 실제로 메일을 보낸 적이 있는지" 를 나중에
스토리 훅에서 확인할 수 있게 기반만 마련해뒀다. 전송 완료 화면에는
"Write another" 버튼을 둬서 바로 빈 초안으로 되돌아가 새 메일을 또 쓸 수
있게 했다.

이번에 새로 쓴 한자(`作`/`成` — "메일 작성" 류 일본어 문구에서 쓰임)도
KANJI_CHARSET 전체 재스캔으로 확인해 추가했다.

**추가로 테스트하기 편하도록**: `FileSystem::new()` 의 `mail_arrived` 기본값을
`false` → `true` 로 바꿨다 — 예전엔 새 게임을 시작해도 (꺼져있는)
`MAIL_AUTO_ARRIVE` 타이머가 아니면 받은편지함이 계속 비어있어서, Reply/
Write Mail 을 테스트하려면 매번 세이브 파일을 직접 고쳐야 했다(그리고
그 방식은 게임이 켜져있는 동안 고치면 자동저장이 곧바로 되돌려버리는
경쟁 상태가 있어서 신뢰할 수 없었다). 이제 New Start 를 누르면 바로 메일
3통(`seed_messages()`, Reply/목록 UI 테스트용으로 2통 추가)이 도착한
상태로 시작한다.

## Mail 답장(Reply) 기능 제거 + Write Mail 에 첨부파일 + 툴바 추가

Reply 버튼(답장 전용 화면)을 없애고, 대신 "Write Mail" 탭 하나로
답장이든 새 메일이든 다 처리하게 단순화했다. 동시에 Write Mail 에
파일 첨부 기능을 추가하고, 창 위쪽에 Outlook Express "New Message"
창처럼 보이는 툴바를 얹었다. 정확한 참고 자료(그 시절 스크린샷)를
웹 검색으로 찾아봤지만 색인된 결과가 없어서(`Sources` 참고), 대신
알고 있는 그 시절 메일 클라이언트의 공통 관례 — 툴바가 헤더 필드
위에 있고, Send/Attach 가 툴바의 아이콘 버튼이며, 첨부파일은 헤더와
본문 사이에 별도 줄로 보여준다 — 를 그대로 재현했다.

**Reply 관련 코드 전부 제거**: `ComposeState` 구조체, `MailApp::compose`
필드, `draw_compose()` 메서드, 읽기 화면의 Reply 버튼, 메시지별
`replied: Vec<bool>` 상태, `fs.mail_replied: Vec<usize>` 필드,
`AppAction::SendMail`/`DeskAction::SendMail` 을 전부 지웠다 — 어떤
메일에 답장했는지 구분해서 저장할 필요 자체가 없어졌다(예전 저장
파일에 `mail_replied` 키가 남아있어도 `serde_json` 이 알 수 없는
필드는 조용히 무시하므로 문제없이 계속 불러와진다).

**Write Mail 툴바**: 창을 열면 필드 위에 회색 툴바 줄이 있고, 그 안에
"Send"(받는 사람/본문이 채워졌을 때만 활성)와 "Attach..." 버튼이
있다 — 얇은 홈선(sunken 테두리 축소판)으로 두 버튼 그룹을 나눠서
진짜 툴바처럼 보이게 했다. 예전엔 Send 버튼이 화면 맨 아래에 있었는데,
그 자리는 이제 본문 입력 영역이 끝까지 차지한다.

**첨부파일 선택/제거**: "Attach..." 를 누르면(눌려있는 동안은 sunken
으로 눌린 채 고정) 본문 자리에 첨부 가능한 파일 목록이 대신 뜬다 —
목록은 `apps/mod.rs::open()` 이 창을 열 때 `fs.desktop`/`fs.downloads`
에서 폴더류(My Computer/Recycle Bin 포함)와 Mail 자기 자신을 뺀
실제 파일들을 스냅샷으로 만들어 넘겨준 것(`MailApp::attachable`)이다.
하나를 고르면 목록이 닫히고 필드/본문 사이에 아이콘+파일명+"Remove"
버튼으로 된 첨부 칩이 나타난다 — 다시 누르면 첨부가 풀리고 "Attach..."
버튼이 다시 눌릴 수 있는 상태로 돌아간다. 실제로 첨부된 파일이 무엇을
하는 건 아직 없다(게임 진행에 영향 없음) — 첨부 UI 자체를 갖추는 게
이번 요청의 핵심이었다.

Sources:
- 웹 검색으로 Outlook Express 5/6 "New Message" 창의 정확한 툴바 배치를
  보여주는 스크린샷/문서는 찾지 못했다(검색 결과가 전부 최신 Outlook
  관련 문서였음). 그 시절 툴바 구조에 대한 지식은 검색이 아니라 기존
  지식에서 가져왔다.

**바로 이어진 피드백: 버튼을 위쪽 큰 툴바 대신 아래쪽에 작게** — 위
디자인을 적용하자마자 "위쪽 툴바 버튼이 너무 크고 화면을 많이 차지한다,
아래로 옮기고 크기도 줄여달라"는 요청을 받았다. `TOOLBAR_H`(34px) 툴바
전체와 그 안의 큰 버튼(`btn_h = TOOLBAR_H - 8`)을 들어내고, 화면 맨 아래
`BTN_ROW_H`(30px) 한 줄에 작은 버튼(`BTN_H` 20px, 텍스트 스케일도
1.0→0.85) 두 개로 옮겼다 — Send 가 오른쪽 끝(가장 자주 누르는 주 동작),
그 왼쪽에 Attach 를 뒀다. 첨부 목록이 펼쳐진 상태에서도 이 버튼 줄이
그대로 보여야(Attach 를 다시 눌러 닫을 수 있어야) 해서, 첨부 목록
분기와 본문 편집 분기 양쪽 끝에서 공유해 부르는 `draw_new_compose_buttons()`
로 따로 뽑아냈다.

## Mail 쓰기 화면 텍스트 입력 문제 — "이상하게 입력된다" + "한글이 한 글자 늦게"

요청: "메일을 보낼 때 입력란에 텍스트가 요상하게 입력되는 문제를 해결해줘.
그리고 한글쪽에서는 한글이 한 글자 늦게 입력이 돼. 일본어는 테스트가
힘드니까 고질적인 문제들을 찾아보고 전체적으로 수정해줘."

**원인을 두 겹으로 나눠서 봐야 한다.**

**① 진짜 버그(고쳤다) — To/Subject 필드가 한 프레임 늦게 그려짐.**
`draw_new_compose()` 안에서 To/Subject 는 `self.new_mail.to/subject` 를
`.clone()` 해서 화면에 그린 *다음에* 이번 프레임에 들어온 `win.input.typed`
문자를 실제 값에 반영하는 순서였다. 즉 방금 친 문자가 이번 프레임엔
반영만 되고, 화면엔 다음 프레임에야 나타난다 — Body 필드는 우연히 이
순서가 반대라(입력 처리가 그리기보다 먼저) 이 문제가 없었다. 입력 처리
블록을 필드를 그리기 전, 함수 맨 앞으로 옮겨서 세 필드 모두 "이번
프레임에 친 글자가 이번 프레임에 바로 보이는" 동작으로 통일했다
(`src/apps/mail.rs::draw_new_compose`).

**② 한글이 "한 글자 늦게" 보이는 현상(구조적 원인) —**
`main.rs` 의 `char_event` 핸들러와, 그 아래 실제로 메시지를 처리하는
`miniquad` 0.4.11(레지스트리 캐시의 벤더 소스,
`native/windows.rs`)의 Windows IME 처리 코드를 직접 읽어서 확인한 결과:
`WM_CHAR` 은 영문 등 단순 입력에 정상 대응하고, `WM_IME_CHAR` 은 중복
방지를 위해 의도적으로 무시하며, `WM_IME_COMPOSITION` 에서는
`GCS_RESULTSTR` 플래그(= "지금 막 확정된 문자열")가 켜졌을 때만
`ImmGetCompositionStringW` 로 그 확정 문자열을 읽어 `char_event` 를
쏜다 — 이건 Win32 IME 처리의 교과서적으로 올바른 구현이다. 문제는 그
바깥, `miniquad::EventHandler` 가 공개하는 API 자체가 "확정된 문자"만
알려주고 "지금 조합 중이지만 아직 확정 안 된 문자열"은 전혀 노출하지
않는다는 점이다. 그리고 Windows 한글 IME 는 한 음절(예: "안")을 완성해도
그 자체로는 아직 확정 처리를 안 하고, 보통 *다음* 키를 눌러 다음 음절
조합을 시작할 때(또는 스페이스/엔터 등으로 조합을 끝낼 때)에야 이전
음절을 확정한다 — 그래서 우리 앱 입장에서 보면 "방금 완성한 글자가 다음
글자를 치기 전까진 안 보이는" 것처럼 느껴진다. 이건 우리 `char_event`
소비 코드의 버그가 아니라, IME 확정 타이밍 자체의 특성 + miniquad 가
조합 중 미리보기(preedit) 를 노출 안 한다는 두 가지가 겹친 결과다.

**시도했다가 되돌린 것 — Win32 IMM API 직접 폴링으로 라이브 미리보기.**
miniquad 의 이벤트를 우회해서 `ImmGetCompositionStringW` 를 직접 호출해
조합 중(미확정) 문자열을 매 프레임 읽어와 확정된 값 뒤에 이어붙여
미리보기로 그리는 `src/ime.rs::composition_preview()` 를 한 번 추가했다.
그런데 적용해보니 영문만 입력하는 상황(예: "asdasdasd")에서도 마지막
글자가 다음 줄로 내려가 버리는 게 보였다. 이때는 "안 친 글자가 몰래
붙는 IME 버그"라고 잘못 판단해서 이 미리보기 기능을 되돌렸는데
(`src/ime.rs` 삭제, `Cargo.toml` 의 `Win32_UI_Input_Ime`/
`Win32_UI_Input_KeyboardAndMouse` 피처 제거, mail.rs 원상복구), 되돌린
뒤에도 똑같은 증상이 재현돼서(아래 ③) IME 코드는 애초에 무관했다는 게
드러났다 — 다만 검증 못 하는 `unsafe` FFI 코드를 실제 입력 경로에 남겨두지
않기로 한 결정 자체는 여전히 맞는 판단이라 되돌린 채로 둔다.

**③ 진짜 근본 원인(고쳤다) — `wrap_lines()` 자체의 off-by-one 버그.**
IME 를 되돌린 뒤에도 "엔터 안 쳤는데 마지막 글자가 다음 줄에 혼자 써짐"
증상이 그대로 재현된다는 스크린샷을 받고서야, 이게 Mail 이나 IME 문제가
아니라 이 세션 초반에 다국어 줄바꿈을 통일하며 만든 `src/ui.rs::wrap_lines()`
자체의 버그라는 걸 확인했다. 박스 폭을 화면에 직접 찍어서 검증해보니
(`text_area.w=332px`), 잘려나간 문자열 전체("asdasdasd...dasdawd", 31자)는
실제로는 313.5px 밖에 안 돼서 332px 안에 넉넉히 다 들어가는데도 마지막
한 글자가 항상 다음 줄로 밀려났다.

원인은 `wrap_lines` 가 매번 `split_line_once()` 를 무조건 호출한다는
것이었다 — `split_line_once` 의 글자 단위 폴백(공백이 없을 때 쓰는 경로)은
`char_indices()` 로 "각 글자가 시작하는 바이트 위치"들을 훑으면서 "그
위치까지의 접두사가 아직 max_w 를 안 넘는지"를 검사하는데, 이 위치들은
전부 "글자 시작점"이라 마지막 글자의 시작점까지만 검사할 수 있고 "마지막
글자까지 포함한 전체 문자열"은 애초에 검사 대상에 없었다. 그래서
전체 문자열이 이미 통째로 다 들어가는 경우에도 이 함수는 그 사실을 알
방법이 없어 마지막 한 글자를 무조건 다음 줄로 넘겨버렸다 — 두 줄짜리
버전인 `wrap_two_lines()` 에는 애초에 `split_line_once` 를 부르기 전에
"전체가 이미 들어가면 그대로 한 줄로" 확인하는 사전 검사가 있었는데,
나중에 만든 `wrap_lines` 에는 그 사전 검사를 빠뜨렸던 것이다.

`wrap_lines` 의 while 루프 안, `split_line_once` 호출 직전에 같은 사전
검사(`r.text_width(rest, scale) <= max_w` 면 그대로 한 줄로 밀어넣고
종료)를 추가해서 고쳤다(`src/ui.rs::wrap_lines`). 이 함수는 Mail 본문뿐
아니라 Notepad, 설치 마법사 안내문, 압축파일/Hex 도구 미리보기, 휴지통
안내문 등 이 세션에서 다국어 줄바꿈으로 통일한 곳 전부가 공유해서 쓰던
함수라, 지금까지 그 화면들 전부에서 "박스 안에 다 들어가는 문장의 마지막
글자만 이유 없이 다음 줄로 내려가 보이는" 증상이 똑같이 있었을 것이다 —
이번 수정으로 전부 한 번에 고쳐졌다.

## Mail "발송됨" 화면 제거 + 보낸편지함(Sent Items) 탭 추가

입력 버그가 잡힌 걸 확인한 뒤 받은 요청: "이제 입력 잘 되고, 발송됨 씬을
제거해줘. 그리고 보낸 메일 목록 탭도 만들어줘."

**발송 확인 화면 제거.** Send 를 누르면 "메일을 보냈습니다 / 메일이
발송되었습니다" + "새로 쓰기" 버튼이 있는 중간 화면으로 넘어갔었는데
(`NewMailState::sent` 플래그로 분기), 이 화면 자체를 없앴다. 이제 Send를
누르면 그 자리에서 바로: (1) 보낸 내용을 보낸편지함 목록에 즉시 추가하고,
(2) 입력폼을 비워서 바로 다음 메일을 쓸 수 있는 빈 "메일 쓰기" 화면으로
돌아간다 — 확인 화면 없이 실제 메일 클라이언트에서 Send 를 누르면 창이
바로 다음 상태로 넘어가는 것과 같은 느낌.

**보낸편지함(Sent Items) 탭.** 폴더 트리가 Inbox/Compose 둘뿐이었는데
그 사이에 Sent Items 를 추가했다(`MailFolder::Sent`). 지금까지는 보낸
메일이 `fs.mail_sent_count`(그냥 누적 숫자)로만 남아서 실제로 뭘 보냈는지
다시 볼 방법이 없었다 — 이걸 `fs.sent_mail: Vec<SentMail>`(받는 사람/제목/
본문/첨부까지 통째로) 로 바꿔서 저장 파일에도 남게 했다. 창을 열 때
(`apps/mod.rs::open()`)이 값을 스냅샷해서 아이콘까지 미리 구해 넘겨주고,
Send 를 누르는 그 순간에도 `MailApp` 이 로컬 목록에 바로 하나 얹어서
새로고침을 기다리지 않고 즉시 보이게 했다(진짜 저장은 여느 때처럼
`AppAction::SendNewMail → DeskAction::SendNewMail` 을 거쳐 desktop.rs 가
`fs.sent_mail.push(...)` 하고 저장한다).

Sent Items 화면은 Inbox 의 목록/읽기 구조를 그대로 본떴지만(`draw_sent_pane`)
훨씬 단순하다 — 안 읽음 배지/굵은 글씨가 없고(자기가 보낸 걸 "안 읽음"
취급할 이유가 없다), 첨부는 다운로드 버튼 없이 아이콘+이름만 정보로
보여준다(이미 내가 가진 파일이라 "받을" 개념이 없다). 목록 헤더도
From 대신 To 로 바뀐다. 본문 줄바꿈 캐시(`sent_wrapped`/`sent_wrapped_key`)
는 Inbox 것과 일부러 따로 뒀다 — 같은 필드를 같이 쓰면 "Inbox 0번 읽다가
Sent 0번으로 넘어가도 인덱스가 같아서 캐시가 그대로 유효하다고 착각"해
엉뚱한 본문이 잠깐 보일 수 있어서다. 새로고침(메일 도착 타이머)이 선택
상태를 이어받는 로직(`MailApp::set_selected`)도 이제 Inbox/Sent 중 어느
폴더를 보고 있었는지에 따라 clamp 기준 길이를 다르게 잡도록 고쳤다.

## 받은편지함 시드 메일의 제목/본문 다국어화

요청: "발신되는 메일의 제목과 내용은 언어에 따라 번역되도록 만들어줘." —
Mail 앱의 나머지 UI(라벨, 버튼, 폴더 이름 등)는 전부 `tr(lang, en, ko, ja)`
로 세 언어를 다 지원하는데, 정작 받은편지함에 도착하는 스토리용 시드
메일 세 통(`seed_messages()`)의 제목/본문만 영어 문자열이 그대로 박혀
있어서 언어를 바꿔도 안 바뀌는 상태였다 — 그 불일치를 맞췄다.

`from`/`to`/`cc`/`sent`(보낸 사람 이메일 주소, 받는 사람, 참조, 보낸
시간)는 애초에 번역 대상이 아니라(이메일 주소는 언어와 무관) 그대로
두고, `subject`/`body` 만 `tr()` 로 세 언어 문구를 갖게 했다
(`src/apps/mail.rs::seed_messages`). 첫 메일(Photo QA Request/사진 검수
요청/写真確認依頼) 본문엔 새로 등장한 한자 4개(依写真頼)가 있어서
`KANJI_CHARSET`(gfx.rs)에도 추가했다 — 늘 하던 대로 폰트에 실제로 그
글자가 있는지 fontTools 로 먼저 확인한 뒤에 추가했다.

**설정에서 바로 언어를 바꿔도 즉시 반영되도록.** 이 세션 초반에 만든
"창을 새로 안 열어도 언어가 바로 바뀐다" 원칙을 여기도 그대로 지켜야
했다 — 메일 목록은 `MailApp::new()` 때 한 번만 만들어지는 `Vec<MailMsg>`
라 그냥 두면 창을 닫았다 열어야만 새 언어로 보였다. `MailApp` 에
`attachment_seed`/`arrived`(seed_messages 재호출에 필요한 원본 인자)와
`built_lang`(마지막으로 만들 때 쓴 언어)을 저장해두고, `update()` 가 매
프레임 `self.settings` 의 현재 언어와 `built_lang` 을 비교해서 다르면
`self.messages` 를 그 자리에서 다시 만든다 — 메시지 개수 자체는 안
바뀌므로 `read`/`downloaded`/`downloading` 같은 같은-길이 상태 Vec 들은
그대로 둬도 인덱스가 안 어긋난다.

(참고: "발신되는 메일"을 문자 그대로 읽으면 플레이어가 직접 쓴 Sent
Items 쪽을 가리키는 것처럼도 보이는데, 그건 플레이어가 자유롭게 타이핑한
텍스트라 번역 엔진 없이는 애초에 자동 번역이 불가능하다 — 그래서 이미 앱
전체가 다국어인데 유독 영어 고정이던 "받은편지함 스토리 메일"쪽 요청으로
이해하고 작업했다. 혹시 의도가 달랐다면 말씀해주시면 다시 맞추겠다.)

## 한글 IME 실시간 미리보기 재도입

위 작업 도중 "한국어가 여전히 한 글자가 늦게 타이핑 되는 문제가 있어"
라는 재보고를 받았다. 예전에(위쪽 "Mail 쓰기 화면 텍스트 입력 문제"
섹션) 이 문제를 해결하려고 `src/ime.rs::composition_preview()`(Win32
IMM API 로 아직 확정 안 된 조합 중 문자열을 실시간으로 읽어와 미리
보여주는 기능)를 한 번 추가했다가, 영문만 쳐도 마지막 글자가 다음 줄로
밀리는 회귀를 보고 IME 쪽을 의심해 되돌렸었다. 그런데 그 회귀는 나중에
밝혀졌듯 이 모듈과 전혀 무관한 `wrap_lines()` 자체의 버그(전체 문자열이
이미 다 들어가는 경우를 사전 확인 안 해서 항상 마지막 글자를 다음 줄로
넘기던 것)였다 — 그 버그를 이미 고쳤으니, `composition_preview()` 를
다시 추가했다.

구현은 전과 같은 요령이지만 방어 코드를 하나 더 넣었다: 1차 호출로 받은
바이트 길이가 홀수(UTF-16 문자열이라면 있을 수 없는 값)면 그냥 포기하고
None 을 돌려준다 — 버퍼 크기 계산이 어긋나 넘치는 사고를 미리 막기 위한
방어적 처리다. `GetActiveWindow()`(호출 스레드의 메시지 큐가 소유한 창만
돌려줘서 다른 프로세스 창을 잘못 읽을 위험이 없다)로 창 핸들을 얻고,
`ImmGetContext`/`ImmGetCompositionStringW(GCS_COMPSTR)`/`ImmReleaseContext`
로 조합 중 문자열을 읽어 `draw_editable_field`(To/Subject)와 본문
그리기(`draw_new_compose`) 양쪽에서 committed 값 뒤에 이어붙여 화면에만
보여준다 — `self.new_mail.to/subject/body` 자체는 여전히 `char_event` 로
확정된 문자만으로 채워지고 이 미리보기 값의 영향을 전혀 안 받는다.

다만 이 코드는 여전히 `unsafe` Win32 FFI 라 내가 직접 `cargo run` 으로
실행해 검증할 수는 없다(프로젝트 규칙상 안 하기도 함) — 이번엔 이전
회귀의 진짜 원인이 다른 곳(wrap_lines)이었다는 게 확인됐고 코드도 다시
신중히 검토했지만, 확인은 쥬인님이 직접 해주셔야 한다. 한글로 계속
타이핑해봐도 여전히 이상하거나 뭔가 안 맞으면 바로 알려달라.

## Mail 쓰기 본문 스크롤

요청: "메일을 작성할 때 내용이 많아지면 스크롤이 되도록 만들어줘." —
그동안 작성 화면 본문은 항상 "마지막에 화면에 들어가는 줄만" 보여주고
그 위 내용은 아예 볼 방법이 없었다(스크롤 자체가 없었음). Inbox/Sent
읽기 화면과 같은 스크롤 메커니즘(`body_scroll`/`body_scroll_disp` +
휠 + 스크롤바)을 작성 화면 본문에도 그대로 연결했다
(`src/apps/mail.rs::draw_new_compose`) — 읽기 화면과 작성 화면은 폴더가
달라 동시에 화면에 안 보이므로, 같은 필드를 그대로 재사용해도 서로 안
섞인다(Sent 전용 캐시를 따로 둔 것과 같은 이유로 여기는 굳이 따로 안
둬도 된다 — 세 화면 다 동시에 보일 일이 없다).

다만 작성 중인 화면이라 읽기 전용과 달리 하나 더 신경 써야 했다:
**타이핑하면 스크롤이 캐럿을 따라 맨 아래로 돌아가야 한다.** 그래서
매 프레임 "본문 글자 수가 바뀌었는지"(입력/백스페이스/엔터) 또는 "지금
한글/일본어를 조합 중인지"를 확인해서, 둘 중 하나라도 해당하면
`body_scroll` 을 강제로 맨 아래로 당긴다 — 위로 스크롤해서 이전 내용을
읽다가도 타이핑을 계속하면 화면이 캐럿을 따라 자동으로 내려가는, 흔한
텍스트 편집기 동작과 같다. 깜빡이는 입력 캐럿도 지금 스크롤이 맨
아래일 때만 그린다 — 위로 스크롤해서 예전 내용을 보는 동안은 캐럿을
숨겨서 "지금 타이핑이 어디로 들어가는지" 헷갈리지 않게 했다(입력 자체는
항상 본문 끝에만 붙으므로 캐럿을 안 보여줘도 뜻은 정확하다).

## 바탕화면 아이콘 개수 늘리기 (1단계 — 행 하나 추가)

요청: "바탕화면에 아이콘과 아이콘의 텍스트 크기는 유지하면서 배경화면에
넣을 수 있는 아이콘의 개수를 늘려줄 수 있어? 일단 조금씩 순차적으로
늘려보자." — 아이콘/글자 크기(`IC_SIZE`, `LABEL_TEXT_SCALE`)는 그대로
두고, 격자 칸 수(`GRID_MAX_COL`/`GRID_MAX_ROW`, `src/scenes/desktop.rs`)
만 늘리는 방향으로 진행하기로 했다.

이 앱은 실제 창 크기와 무관하게 항상 640x480 가상 해상도로 그린 뒤
CRT 셰이더로 확대해서 보여준다(`main.rs`, `VW`/`VH`) — 바탕화면 격자도
이 640x480 기준으로 계산된다. 지금 칸 하나 크기(`TILE_H`)로 계산해보면
기존 5행(0..=4)은 세로로 355px 만 쓰고 작업표시줄(28px) 위까지 약
452px 가 남아 있어서, 아이콘/글자 크기를 하나도 안 건드리고도 6행
(0..=5, 426px)까지는 여유 있게 들어간다 — 그래서 이번 1단계로
`GRID_MAX_ROW` 를 4→5 로 올려 6행(9열×6행=54칸, 기존 9×5=45칸에서
+9칸)으로 늘렸다. 가로(9열)는 이미 640px 폭을 거의 다 써서(604/640px)
칸 크기를 안 줄이면 더 못 늘리므로 이번엔 손 안 댔다.

`grid_pos`/`nearest_free_tile`/`first_free_tile` 등 격자 관련 함수가
전부 `GRID_MAX_ROW`/`GRID_MAX_COL` 상수만 보고 동작해서(하드코딩된 칸
수가 따로 없음), 상수 하나만 바꿔도 새 아이콘 배치·드래그 스냅·저장된
위치 복원까지 전부 자동으로 6행 기준으로 맞춰진다. 더 늘리고 싶으면
(요청대로 순차적으로) 칸 크기를 줄이거나 세로 여백을 더 줄이는 식으로
다음 단계를 진행하면 된다.

## 바탕화면 아이콘 개수 늘리기 (2단계 — 여백을 줄여 행 하나 더)

"좀 더 늘려줘" 요청으로 2단계 진행. 1단계 이후 6행(452px 중 426px 사용,
여유 26px)까지 늘렸는데, 그 26px 로는 한 행(TILE_H 70.8px)이 더 안
들어가서, 이번엔 아이콘 크기(`IC_SIZE`)와 글자 크기(`LABEL_TEXT_SCALE`)는
그대로 두고 그 사이 여백만 줄였다 — 타일 위쪽 여백(`ICON_AREA_TOP`
3→1px), 아이콘~글자 간격(`LABEL_GAP` 3→1px), 글자~다음 행 사이 여백
(새로 이름 붙인 `TILE_BOTTOM_PAD` 4→1px), 화면 가장자리 여백
(`GRID_ORIGIN` 12→6px). `TILE_H` 가 70.8px→63.8px 로 줄면서 7행
(0..=6)까지 640x480 안에 들어간다(작업표시줄 위까지 448.8px 사용, 여유
약 3px) — 9열×7행 = 63칸(1단계 54칸에서 +9, 최초 45칸에서 +18).

**가로(9열)는 이번에도 손 안 댔다.** 실제로 재보니 "HexTool Setup.exe"
가 두 줄로 접힐 때 둘째 줄 "Setup.exe" 폭이 57.75px 로, 지금 칸 폭 기준
글자 감쌈 한도(`LABEL_MAX_W` = 58px)에 거의 딱 맞게 걸쳐 있다(여유
0.25px) — 칸을 조금이라도 더 좁히면 이 파일명만 글자가 살짝 잘려 보일
것 같아서 위험을 감수하지 않았다. 세로는 반대로 "여백을 줄이는 것"만으로
공간을 벌 수 있어(글자 자체가 잘릴 위험이 없다) 안전하게 더 늘릴 수
있었다.

**다음 단계부터는 여백만 줄이는 방식의 한계다.** 이제 `TILE_H` 의 남는
여백은 위/글자간격/아래 각각 1px 씩뿐이라 더 줄일 데가 없다 — 더
늘리려면 아이콘/글자 크기를 실제로 줄이거나(이번 요청의 전제와 반대),
칸 폭을 넓혀 640x480 세로 여유를 다른 식으로 확보하거나(예: 작업표시줄을
더 얇게), 아니면 가상 해상도(640x480) 자체를 올리는 더 큰 작업이
필요하다 — 마지막 방법은 이 화면뿐 아니라 폰트 아틀라스/CRT 셰이더/다른
모든 창 크기 등 앱 전체에 영향을 주는 훨씬 큰 변경이라 신중히 논의하고
진행해야 한다.

## HexTool 설치 마법사의 License Agreement 텍스트 다국어화

스크린샷(일본어 UI인데 "HEXTOOL LICENSE AGREEMENT" 약관 본문만 영어로
남아있는 것)으로 받은 제보. `src/apps/installer.rs` 의 `LICENSE_TEXT`
상수가 다른 UI 문구와 달리 `tr()` 를 안 거치는 고정 영어 `&str` 였다 —
"발신되는 메일 제목/본문" 때와 같은 종류의 누락이다. 제목("HEXTOOL
LICENSE AGREEMENT")도 이 텍스트 안에 같은 문단으로 들어있어서 통째로
언어별 세 문구를 가진 `license_text(lang) -> &'static str` 함수로 바꿨고,
이 값을 쓰던 `license_lines()` 에 `lang` 인자를 추가해 호출부
(`draw_license_box`, `self.settings.borrow().language` 로 구함)까지
같이 고쳤다. 새로 등장한 한자들은 기존 `KANJI_CHARSET` 안에 이미 다
있어서(우연히 겹침) 폰트 쪽은 손 안 대도 됐다 — 그래도 늘 하던 대로
스캔 스크립트로 재확인했다.

**같이 훑어본 다른 자리들:**
- HexTool 검수 화면의 `ANOMALY_TYPES`("Extra figure" 등)는 겉보기엔
  영어 고정이지만, 실제로 화면에 그릴 땐 `anomaly_label()` 이 이 값을
  언어별 문구로 바꿔서 보여주고 있었다(이 배열 자체는 정답 판정용 내부
  키일 뿐) — 이건 이미 제대로 되어 있어서 안 건드렸다.
- `scenes/erase.rs` 의 "Erase All Memory" 삭제 연출 로그
  ("Deleting C:\...\0001.DAT ... OK" 같은 줄들)는 지금 언어 설정과
  무관하게 항상 영어다. 다만 이건 부팅 화면의 BIOS POST 화면과 같은
  톤으로 "진짜 시스템 콘솔/DOS 로그"처럼 보이게 일부러 꾸민 연출이라
  (실제 BIOS/명령줄 로그들이 보통 영어인 것과 같은 느낌), 사용자가
  읽고 이해해야 하는 문서(라이선스 약관 같은)와는 성격이 달라서 의도적인
  디자인일 수 있다고 판단해 이번엔 손 안 댔다 — 이것도 번역해야 한다면
  말씀해주시면 바로 진행하겠다.

## HexTool 라이선스 약관 일본어 — 글자가 군데군데 빠져 보이는 문제

위 다국어화를 적용한 직후 스크린샷으로 재보고: 일본어로 그려진 약관
본문 곳곳에 글자가 빠져서(예: "以下の規約に同意した" 가 "の規約に同意した"
로, "元の所有者に帰属します" 가 "元の所　者に　します" 로) 보이는 문제.

**원인은 한자 커버리지 스캔 스크립트 자체의 버그였다.** 매 작업 끝에
돌리던 `extract_all_ja2.py` 가 `tr(lang, "...", "...", "...")` 형태만
정규식으로 잡는데, `license_text()` 를 여러 줄로 나눠 쓰면서 마지막
문자열 인자 뒤에 트레일링 콤마(`"...",\n)`)를 붙였더니 — 이 스크립트의
정규식이 "세 번째 문자열 바로 뒤에 `)`" 만 허용하고 콤마는 허용을 안
해서, 이 `tr()` 호출 자체를 통째로 못 찾고 건너뛰었다. 그래서 지난
턴에 "missing: []" 로 나온 결과가 사실은 "이 문자열을 아예 검사 안 해서
빠진 게 없다고 잘못 나온" 거짓 음성이었다 — 실제로는 이 문단에서 쓴
한자 상당수(以/下/他/復/旧/有/権/属/帰/責/負/護/返/逃/間/響 등 36자)가
`KANJI_CHARSET`(폰트 아틀라스에 포함되는 한자 목록)에 없어서, 그
글자들만 그릴 비트맵이 없어 그냥 빈칸으로 보였던 것이다(폰트에 없는
글자는 렌더러가 "펜만 스페이스 하나만큼 움직이고 넘어간다" — gfx.rs
`glyph_advance`/`glyph` 주석 참고).

**스크립트를 고치고 다시 스캔했다.** 정규식 끝을 `"\s*,?\s*\)` 로 바꿔서
트레일링 콤마가 있어도/없어도 다 잡히게 했더니, 실제로 필요한 `tr()`
호출이 132개가 아니라 **170개**였다는 게 드러났다(전에는 상당수를 계속
못 보고 있었다는 뜻). 다시 스캔한 결과 총 172자가 필요했는데
`KANJI_CHARSET` 에는 136자만 있어서 36자가 빠져 있었다 — 전부 폰트에
실제로 있는지 fontTools 로 확인한 뒤 `KANJI_CHARSET` 에 추가했다.

이 스크립트 버그는 이번 한 번의 요청에만 영향을 준 게 아니라, 지난 여러
턴 동안 트레일링 콤마가 있는 여러 줄짜리 `tr()` 호출을 쓸 때마다 계속
조용히 스캔을 건너뛰고 있었을 가능성이 있다 — 이번에 고친 스크립트로
전체를 다시 훑어서 한 번에 다 잡았으니, 지금은 실제로 쓰이는 모든 일본어
한자가 `KANJI_CHARSET` 에 다 들어있는 상태다. 앞으로는 이 고쳐진
스크립트(스크래치패드의 `extract_all_ja2.py`, 트레일링 콤마 허용)로
계속 검사한다.

## 프로그램/창을 열 때 잠깐 프리징

요청: "프로그램을 열거나 창을 열 때 약간의 프리징 타임이 있으면
좋겠어.." — 옛날 느린 컴퓨터에서 프로그램을 실행하면 창만 먼저 뜨고
안쪽 내용이 그려지기까지 잠깐 멈칫하던 그 느낌을 재현해달라는 요청으로
이해했다.

바탕화면 아이콘/폴더/Mail/설정/Credits/Official Site 등 창을 여는 경로가
여러 군데(desktop.rs 안에서만 `self.wm.open(...)` 호출이 6곳)라 각
호출부를 다 고치는 대신, 창이 실제로 새로 생기는 단 한 곳
(`WindowManager::open()`)에만 손을 댔다 — `Window` 구조체에
`spawn_freeze: f32` 필드를 추가해서 새 창을 만들 때 `SPAWN_FREEZE_S`
(0.35초)로 채워두고, `WindowManager::frame()` 의 창 갱신 루프에서 이
값이 0보다 큰 동안은 `app.update()` 자체를 아예 안 부른다(그래서 그
안의 클릭/타이핑도 하나도 안 먹는다) — 대신 회색(FACE) 배경에 이미
Mail 첨부파일 다운로드 연출 등에서 쓰던 `draw_spinner()` 를 가운데
하나 그려서 "로딩 중"처럼 보이게 하고, 매 프레임 `spawn_freeze` 를
`dt` 만큼 줄인다.

창 테두리(제목표시줄 드래그/최소화/최대화/닫기)는 이 프리징과 무관하게
그대로 동작한다 — 로딩 중인 창도 옮기거나 닫을 수 있는 건 실제 OS에서도
흔한 동작이라 굳이 막지 않았다. 이미 열려있는 창의 내용만 새로고침하는
`refresh_app()`(메일 도착/폴더 갱신 등)은 `spawn_freeze` 를 안 건드리므로,
창을 새로 여는 순간에만 이 프리징이 걸리고 이미 뜬 창이 새로고침될 때는
안 걸린다.

## 휴지통을 휴지통 안으로 끌어넣으면 자기 자신이 보이던 버그

스크린샷 제보: 휴지통을 열었더니 그 안에 "휴지통"이라는 항목이 하나
들어있었다("쓰레기통에 쓰레기통"). 원인은 바탕화면 아이콘을 폴더로
드래그해 옮기는 공통 로직(`desktop.rs::move_ids_to`)에 My Computer
(`FileKind::Explorer`)는 폴더/휴지통 안으로 못 옮기게 막는 가드가 이미
있었는데, 휴지통 자기 자신은 그 가드에 안 걸려 있었다는 것 — 그래서
휴지통 아이콘을 휴지통 위로 끌어다 놓으면, 자기 자신을 자기
`children` 목록에 넣어버리는 자기참조 상태가 만들어졌고, 그 뒤로
휴지통을 열 때마다 그 안에 휴지통 자신이 보였다.

My Computer 를 막던 조건에 `self.fs.get(id).name == "Recycle Bin"` 도
같이 걸어서, 휴지통 자기 자신을 포함한 어떤 폴더로도 휴지통을 못 옮기게
막았다(My Computer 를 막은 것과 똑같은 이유 — 얘도 게임 진행에 꼭
필요한 고정 아이콘이라 지워지거나 자기 안에 갇히면 안 된다). **지금
이미 이 상태가 된 저장 파일은 코드만 고친다고 저절로 안 풀린다** —
휴지통을 열어서 그 안의 "휴지통" 항목을 선택하고 "복원"(Restore)을
누르면 원래 있던 자리(바탕화면)로 돌아간다(옮겨질 당시 위치가
`fs.trash_origin` 에 이미 정상적으로 기록돼 있어서 이 경로는 그대로
잘 작동한다).

## Mail UI 를 실제 Outlook Express 스크린샷 느낌으로

받은 참고 스크린샷 두 장(목록 화면 + 읽기 화면)에 맞춰 두 군데를 손봤다.

**읽기 화면 — 상자 테두리 없는 평문 헤더.** 지금까지 From/Sent/To/Cc/
Subject 값을 흰 배경 + 얇은 테두리로 둘러 "입력칸처럼" 그렸었는데,
참고 이미지는 이 블록 전체가 그냥 "라벨: 값" 을 나란히 흘려 쓴 평범한
문단이었다. `draw_field()` 에서 테두리/배경을 다 빼고 텍스트만 남겼고,
참고 이미지처럼 **From 과 Sent 를 한 줄에 절반씩** 나눠 배치했다(To/Cc/
Subject 는 그대로 한 줄씩). 헤더 블록 끝에는 얇은 회색 구분선을 하나
그어서 본문 상자와 분리했다 — 본문 자체는 이미 참고 이미지처럼 흰
바탕 + 테두리 + 스크롤바인 상태라 안 건드렸다. Sent Items 읽기 화면
(To/Subject 두 줄)도 같은 요령으로 맞췄다.

**목록 화면 — "Received" 칸 + 본문 미리보기(AutoPreview).** From/Subject
두 칸이던 목록 헤더에 오른쪽으로 "Received"(받은 시간) 칸을 하나 더
추가했다 — 값은 읽기 화면의 "Sent:" 와 같은 `MailMsg::sent` 필드를
그대로 쓴다(실제 Outlook Express 도 받은 시간/보낸 시간을 같은 헤더
값으로 보여준다). 예전엔 이 필드가 항상 "(unknown)"이었는데, 참고
이미지의 "Tue 1/12/99 9:04 AM" 같은 실제 날짜 느낌을 살리려고 테스트
메일 세 통에 그럴듯한 가짜 날짜/시간을 채워 넣었다(스토리 캐논은 아니고
그냥 테스트 표시용 텍스트라 더미 값으로 넣은 것). 그리고 각 행 헤더
줄 아래에 본문을 최대 두 줄까지 회색으로 미리 보여주는 "AutoPreview"
를 추가했다(`wrap_lines` 로 감싼 뒤 앞 두 줄만 취한다) — 그만큼 한 행의
높이가 늘어났다. 보낸편지함(Sent Items) 목록에도 같은 미리보기를
추가했는데, Sent Items 쪽은 보낸 시각을 저장하는 필드 자체가 없어서
(`SentMailView` 에 타임스탬프가 없다) Received 칸은 그대로 두었다 —
필요하면 `foundation::SentMail` 에 전송 시각 필드를 추가하는 별도
작업으로 진행할 수 있다.

## Mail 창을 크게 열었을 때 글자가 잘리던 문제 + 기본 창 크기 확대

큰 창으로 연 스크린샷 제보: "받은 사람"/"제목" 칸은 그런대로 보이는데
"받은시간" 칸은 창 크기와 상관없이 항상 "Mon 3/..." 로 잘려 있었고,
본문 미리보기도 문장 중간에서 뚝 끊겨 있었다.

**`RECEIVED_COL_W` 자체가 처음부터 너무 좁았다(진짜 버그).** 108px 로
잡았었는데, fontdue 로 실측해보니 "Mon 3/14/05 10:22 AM" 같은 실제
날짜 문자열은 0.85 스케일 기준 약 150.5px 가 필요했다 — 즉 창을 아무리
넓게 열어도(이 칸은 고정폭이라 창 크기와 무관) 절대 다 안 들어가고
항상 잘리게 짜여 있었던 것. 175px 로 다시 잡아서 여유 있게 다 보이도록
고쳤다. From 칸의 최대폭(`FROM_COL_MAX`)도 230→320px 로 올렸다 — 큰
창에서 남는 공간이 Subject 칸으로만 몰려서 From 칸만 계속 좁아 보이던
불균형을 줄였다.

**본문 미리보기 — 문장이 중간에 뚝 끊기지 않게.** 두 줄 안에 본문이 다
안 들어가면, 예전엔 그냥 두 번째 줄 끝에서 아무 표시 없이 잘렸다(스크린샷의
"압축파일은" 처럼) — 이제 `preview_lines()` 헬퍼가 잘린 마지막 줄 + 그
뒤에 남은 내용을 다시 합쳐서 `truncate_ellipsis` 로 한 번 더 잘라 "..."
를 붙인다. Inbox/Sent Items 목록 둘 다 적용했다.

**기본 창 크기도 키웠다.** "창의 기본 사이즈도 저 정도로 바꿔줘" 요청으로
500x420 → 600x430 으로 올렸다 — 이 앱은 항상 640x480 가상 해상도로
그려지고 작업표시줄(28px)을 빼면 세로로 최대 452px 까지 쓸 수 있어서,
600x430 이 화면 가장자리에 거의 붙지만 넘치지는 않는 사실상 최대치에
가까운 크기다.

## Received/Sent 날짜 칸을 다시 뺐다 — 그래도 여전히 잘려서

바로 위 변경을 확인해본 스크린샷 제보: 175px 로 넉넉히 늘렸다고 생각한
"받은시간" 칸이 실제 플레이 화면(CRT 해상도가 위 계산과 다른 조건)에서는
여전히 "Mon 3/14/0..." 로 잘려 있었고, 읽기 화면에서 From 을 Sent 와
반으로 나눠 쓰게 만든 탓에 이번엔 From 자체("PalaceCom...")까지 더 심하게
잘리는 부작용이 생겼다. "받은 날짜는 없어도 되니까 글자 짤리는 걸
중점적으로 고쳐줘" 라는 요청을 받고, 폭 계산을 더 정교하게 다듬는 대신
**날짜 표시 자체를 뺐다** — 그 칸이 쓰던 폭을 From/Subject 가 그대로
돌려받으니 잘릴 걱정이 원천적으로 줄어든다.

- Inbox 목록: Received 칸과 헤더를 빼고 처음의 From/Subject 두 칸 구조로
  되돌렸다(본문 미리보기 두 줄은 그대로 유지).
- 읽기 화면: From 을 Sent 와 반으로 나눠 쓰던 걸 되돌려 다시 한 줄 전체
  폭을 쓰게 했고, Sent 필드 자체를 뺐다.
- 이제 화면 어디에도 안 쓰는 `MailMsg::sent` 필드는 완전히 지웠다(그냥
  냅두면 "field is never read" 경고가 떠서) — 세 테스트 메일에 넣어뒀던
  가짜 날짜 문자열도 같이 정리됐다.

이 시행착오로 배운 것: 폭 계산은 실측(fontdue)으로 정확히 맞춰도, 화면
전체에서 그 칸이 실제로 얼마나 넓게 그려지는지는 다른 여러 레이아웃
변수(트리 폭, 창 크기, 다른 칸들과의 균형)가 겹쳐서 예상과 다르게
나올 수 있다 — 애매하게 여러 번 폭만 조정하기보다, "그 정보가 꼭
필요한가"부터 다시 물어보는 게 더 빠른 해결일 때가 있다.

## 언어를 바꿔도 이미 열린 창의 항목 이름이 안 바뀌던 문제 (전체 점검)

스크린샷 제보: 설정에서 한국어로 바꿨는데 이미 열려있던 My Computer/
휴지통/Mail 창과 작업표시줄 버튼은 여전히 일본어("マイコンピュータ",
"ごみ箱")로 남아있었다. 앱 전체를 훑어서 원인이 되는 패턴을 찾아 한
번에 고쳤다.

**원인.** File Explorer/휴지통/Mail 첨부 목록의 항목 이름(폴더/파일
목록에 그려지는 문자열)이 창을 열 때(`apps/mod.rs::folder_items()`)
`display_name(lang, ...)` 로 미리 번역해서 `String` 으로 구워 넣은
것이었다 — 그 창을 연 순간의 언어로 고정되고, 그 뒤로 설정에서 언어를
바꿔도 이미 만들어진 문자열 자체는 다시 안 바뀐다. 반면 폴더 트리
탭 라벨이나 상태바 같은 건 매 프레임 `tr()`/`category_label()` 을 다시
불러서 이미 실시간으로 잘 반영되고 있었다 — 항목 목록만 예외였던
것. **창 제목**은 그보다 더 근본적으로, `ExplorerApp` 이 아예
`App::title()` 을 구현 안 해서(Mail/휴지통/설정 등은 진작에 구현돼
있었다) 항상 창을 처음 열 때의 제목 문자열 그대로 고정이었다.

**고친 것.**
- `folder_items()` 가 이제 번역 대신 fs 의 원문 이름을 그대로 담아
  돌려준다. 실제 번역은 그릴 때(`explorer.rs::draw_list_view`,
  `widgets.rs::icon_grid` — 이 둘이 File Explorer 목록/격자 보기와
  휴지통 격자 보기를 전부 공유한다)마다 `display_name(lang, ...)` 를
  새로 불러서 반영한다 — 사이드바 탭 라벨과 똑같은 패턴으로 맞춘 것.
- `ExplorerApp` 에 원문 제목(`raw_title`)을 들고 있다가 매 프레임
  다시 번역하는 `title()` 을 새로 구현했다. 드릴다운으로 하위 폴더에
  들어가도 창 제목은 이 창이 원래 대표하는 루트(예: "내 컴퓨터")를
  계속 가리킨다(실제 탐색기도 그렇다).
- 드래그 중 마우스를 따라다니는 고스트 라벨(`ExplorerApp::drag_ghost`)
  도 번역 안 된 원문을 그대로 보여주고 있었어서 같이 고쳤다.
- Mail 의 "첨부 고르기" 목록/이미 고른 첨부 칩도 같은 `folder_items()`
  를 쓰므로 그리는 시점에 번역하도록 맞췄다(실제로는 첨부가 항상 그냥
  파일 이름이라 번역 대상이 되는 경우는 없지만, 나중에라도 어긋나지
  않게 일관되게 처리).
- 작업표시줄 버튼 글자는 원래부터 각 창의 `title` 필드를 그대로 쓰고
  있어서, 창 제목이 실시간으로 바뀌면 자동으로 같이 바뀐다 — 따로 손
  안 대도 됐다.

**점검해서 문제없다고 확인한 것.** Notepad/이미지 뷰어/동영상/비밀번호
창/설치 마법사/HexTool/압축파일 창들은 제목이 전부 실제 "파일 이름"
(예: "Photos.tar", "HexTool Setup.exe")이라 애초에 번역 대상이 아니다
(`display_name()` 이 미리 정해둔 네 개의 특수 키 — My Computer/Recycle
Bin/Mail/(deleted) — 가 아니면 그냥 원문을 그대로 돌려준다) — 그래서
이 창들은 언어를 바꿔도 제목/내용이 안 바뀌는 게 정상이며 버그가 아니다.

## Photos.tar/사진 플레이스홀더 콘텐츠 제거 + 이미지/아이콘/폰트 내장 확인

요청: 이미지 폴더의 사진을 게임에서 빼고 필요 없어진 이미지 파일을
assets 에서 정리, 이미지/아이콘/폰트 파일을 exe 에 합쳐서 빌드. 확인
결과 사진을 완전히 빼면 Mail 의 "Photo QA Request"(비밀번호 4471로
여는 Photos.tar 첨부) 이야기 자체가 사라진다고 미리 알렸고, 그것까지
다 빼는 걸로 확인받았다.

- `foundation.rs::FileSystem::new()` 에서 HexTool Setup.exe/Photos.tar/
  Photos.lock(비밀번호 4471) 시드와 photo01/02.jpg 항목, Mail 의
  `attachment: Some(...)` 를 전부 제거(Mail 은 이제 `attachment: None`).
- Mail 의 "Photo QA Request" 시드 메시지(그 첨부 이야기 전용이던 메일)
  를 통째로 지우고 남은 테스트 메일 두 통을 #1/#2 로 재번호.
- `MailApp`/`seed_messages()` 에서 단일 전역 첨부(attachment_seed) 배선을
  걷어냈다 — 메시지별 첨부 필드(`MailMsg::attachment`)와 다운로드 상태
  추적, Download 버튼 UI 자체는 범용 메커니즘이라 그대로 남겨서, 나중에
  실제 Chapter 1 사진을 붙인 메일이 생기면 그대로 재사용할 수 있다.
- `Assets::photos` 를 빈 배열로 비우고 그 전용 로더(`load_texture_sized`)
  는 지웠다 — `FileKind::Img`/`ImageViewerApp` 자체(범용 사진 뷰어)는
  나중에 실제 사진이 생기면 그대로 쓸 수 있게 남겨뒀다. HexTool/Archive/
  Installer 앱 코드도 마찬가지로 남겨뒀다(Photos.tar 말고 다른 걸 여는
  용도로 재사용 가능 — 지금은 아무 데서도 안 만들어질 뿐).
- 실제 이미지 파일(`assets/photos/photo01.jpg`, `photo02.jpg`)은 코드에서
  참조가 없어져서 안전하게 지울 수 있게 됐지만, **완전히 삭제하지 않고
  `assets/_unused/` 로 옮겨뒀다** — 이 프로젝트는 git 저장소가 아니라
  삭제하면 복구할 방법이 없다(예전에 font.png 를 `rm` 으로 지웠다가
  되돌릴 뻔한 사고가 있었던 것과 같은 이유). 정말 완전히 지워도 된다고
  확인해주시면 그때 마저 지우겠다.

**이미지/아이콘/폰트를 exe 에 합쳐 빌드하는 부분은 — 이미 그렇게 되어
있었다.** 확인해보니 모든 아이콘(`icon_*.png`), 커서(`cursor.png`),
폰트(`TerrarumSansBitmap.otf`)가 전부 `include_bytes!()` 로 컴파일
타임에 실행 파일 안에 그대로 박혀 있다(`gfx.rs`) — 실행할 때 `assets/`
폴더가 옆에 있을 필요가 전혀 없다. 유일하게 런타임에 디스크에서 파일을
읽는 건 `assets/movie.mp4`(video_player.rs)뿐인데, 이건 이미지/아이콘/
폰트가 아니라 영상이라 요청 범위 밖이고, 용량이 커서(Media Foundation
디코더도 보통 실제 파일 경로를 필요로 함) 통째로 실행 파일에 박아
넣는 게 애초에 실용적이지 않다 — 그래서 이 부분은 그대로 뒀다.

## Mail 작성 화면 — 한/일 입력 뭉개짐 + 실제 커서 편집 기능

스크린샷 제보: 스페이스바를 안 눌렀는데도 본문이 숫자 섞인 깨진 한글
조각들로 뭉개져 보임. 같이 요청받은 것: Backspace 꾹 눌러 빠르게 지우기,
글 중간을 클릭해서 그 자리부터 편집, 타이핑 중 휠을 돌리면 강제로 맨
아래로 끌려 내려가는 문제.

**뭉개짐의 원인 — 되돌렸던 IME 미리보기 기능을 다시 완전히 뺐다.**
지난번에 "wrap_lines 버그였다"는 확신으로 IME 조합 중 문자열 실시간
미리보기(`src/ime.rs`, `ImmGetCompositionStringW` 직접 폴링)를 되살렸는데,
이번 스크린샷의 증상(멀쩡한 한글 사이에 숫자와 깨진 글자 조각이 섞여
나옴)은 그 코드가 원인일 가능성이 훨씬 크다고 판단했다 — 검증 못 하는
`unsafe` Win32 FFI 가 매 프레임 낡거나 잘못된 버퍼를 읽어서 화면에 그대로
얹었을 때 나타날 법한 패턴이다. 이번엔 미루지 않고 `src/ime.rs` 삭제,
`Cargo.toml` 의 관련 피처(`Win32_UI_Input_Ime`/`Win32_UI_Input_KeyboardAndMouse`)
제거, mail.rs 의 사용처를 전부 원상복구했다 — 한글 조합이 "한 글자
늦게" 보이는 현상 자체는 여전히 남지만(플랫폼 특성, 이전 분석 참고),
직접 검증할 수 없는 저수준 코드로 화면이 깨지는 것보다는 이쪽이 안전하다.

**실제 커서 편집 기능을 새로 만들었다.** 지금까지 세 입력칸(To/Subject/
본문)은 전부 "끝에만 붙는" 방식이라 커서 개념 자체가 없었다 — 이번에
`NewMailState` 에 `cursor`(글자 인덱스)를 추가해서 진짜 커서 편집이
되게 했다.
- `src/ui.rs` 에 두 헬퍼를 새로 추가: `char_index_at_x()`(한 줄 텍스트에서
  클릭한 x 좌표에 가장 가까운 글자 경계를 찾는다 — 각 글자의 가운데를
  기준으로 앞/뒤 중 더 가까운 쪽), `wrap_with_offsets()`(`wrap_lines` 와
  똑같이 접되 각 줄이 원본 문자열의 몇 번째 글자부터 시작하는지도 같이
  돌려준다 — 여러 줄로 접히는 본문에서 클릭 좌표 ↔ 커서 위치를 서로
  변환하는 데 필요하다).
- To/Subject 는 한 줄이라 `char_index_at_x()` 하나로 충분하고, 본문은
  먼저 어느 줄을 클릭했는지(y 좌표 + 스크롤) 찾은 뒤 그 줄 안에서
  `char_index_at_x()` 로 글자를 찾아 원본 기준 커서 위치로 환산한다.
- 타이핑/Backspace/Enter 가 전부 "끝에 push/pop" 대신 `cursor` 위치에
  삽입/삭제하도록 바꿨다(`insert_char_at`/`backspace_at`, 바이트 인덱스가
  아니라 글자 인덱스로 다뤄서 한글/일본어 같은 멀티바이트 문자를 안전하게
  다룬다). 덤으로 왼쪽/오른쪽 화살표로도 커서를 옮길 수 있게 했다.
- 클릭 판정은 입력 처리보다 먼저 하도록 순서를 바꿨다 — 본문을 클릭해
  커서를 옮기고 그 프레임에 바로 타이핑해도 자연스럽게 그 자리에 들어간다.

**Backspace 꾹 누르면 빠르게 지워지게.** 원래 이 앱은 OS 키보드 자동
반복 이벤트를 전부 걸러내고 있었다(`Input::on_key_down` 이 `repeat=true`
인 이벤트는 무시) — 그래서 Backspace 를 누르고 있어도 딱 한 번만
지워졌다. `Input` 에 `is_down()`/`on_key_up()` 을 새로 추가해서 "지금 이
키가 눌려있는지"를 직접 알 수 있게 하고, Mail 쪽에서 자체적으로
시간을 재서(처음 0.35초까지는 그대로, 그 뒤로는 0.035초마다 하나씩)
반복 삭제를 구현했다 — OS 자동反복 속도에 기대는 대신 직접 속도를
정할 수 있어서 다른 키(Enter 등)의 반복 억제는 그대로 두면서 Backspace
에만 적용할 수 있었다.

**타이핑 중 휠이 강제로 맨 아래로 끌려가던 문제 — 근본 원인은 IME
미리보기 버그였다.** 예전 코드는 "본문이 바뀌었거나 IME 조합 중이면
스크롤을 맨 아래로 당긴다"였는데, IME 미리보기가 조합 중이 아닐 때도
계속 뭔가를 돌려주는 버그가 있었다면 이 조건이 거의 매 프레임 참이
돼서 휠을 돌려도 바로 도로 끌려 내려갔을 것이다 — IME 미리보기를
완전히 뺀 지금은 이 문제가 저절로 없어진다. 그와 별개로 스크롤 로직
자체도 "커서가 있는 줄이 화면 밖으로 나가면 그 줄이 보이게" 로
바꿔서(예전의 "무조건 맨 아래") 타이핑/편집으로 커서가 실제로 움직인
프레임에만 스크롤이 따라가고, 그냥 휠로 옛 내용을 보는 동안은 안
끌려가게 했다.

## 자음/모음 한 글자만 치면 빈칸으로 보이던 문제

제보: 자음이나 모음 하나만 입력하면(다음 글자로 안 넘어가고 그 상태로
끝내면) 빈칸처럼 보임. 원인은 폰트 아틀라스(`gfx.rs::build_font_atlas`)
가 완성형 한글(U+AC00~U+D7A3, "가"~"힣")만 담고 있었던 것 — 자음/모음
하나만 쳤을 때 Windows IME 가 완성형 음절 대신 "호환용 자모"
(U+3131~U+318E, "ㄱ" "ㅏ" 같은 낱자 표시용 문자)로 확정해서 보내는데,
이 블록은 완성형 한글 범위 밖이라 아틀라스에 아예 없었다 — 그래서
그 글자만 그릴 비트맵이 없어 펜만 옮기고 아무것도 안 그려져(빈칸처럼)
보였다. 그 폰트에 이 블록의 글자가 실제로 있는지 fontTools 로 먼저
확인한 뒤(94자 전부 있음) 아틀라스에 추가했다 — 조합 중 내부적으로
쓰이는 자모 블록(U+1100~U+11FF, 256자)도 혹시 몰라 같이 넣었다.

## 한글 "한 글자 늦음" — 문자열은 안 읽는 안전한 절충안

제보: 체감 입력 속도가 느리고, 한글이 다시 한 글자 늦게 보임(IME 조합
중인 문자열 미리보기 기능을 두 번 넣었다 뺐던 그 이슈 — 원인은 Windows
한글 IME 가 다음 글자를 치기 시작해야 이전 글자를 확정하는 플랫폼
자체의 동작이라, miniquad 의 char_event(확정된 문자만 알려줌)만으로는
근본적으로 못 없앤다). "최소한의 대비"를 요청받아, 지난 두 번과는 다른
훨씬 안전한 절충안으로 다시 접근했다.

**조합 중인 문자열의 내용은 이번엔 아예 안 읽는다.** 예전 두 번의 시도는
`ImmGetCompositionStringW` 로 조합 중 문자열 자체를 버퍼에 받아와 화면에
이어붙여 보여주려 했는데, 그중 한 번은 정체를 확신 못 한 채로 되돌렸다
(숫자 섞인 깨진 글자 조각 제보 — 검증할 방법이 없어 원인이 그 코드인지
100% 확신은 없지만, 의심되는 유일한 지점이라 통째로 뺐었다). 이번 새
`src/ime.rs::is_composing()` 은 **"지금 조합 중이냐 아니냐" 딱 하나만**
확인한다 — 버퍼를 넘기지 않고 길이만 묻는 호출 하나뿐이라, 문자열을
읽고 디코드해서 화면에 얹는 과정 자체가 코드에 없다. 지난 사고들의
공통점(버퍼 내용을 읽어와 렌더링하는 지점)이 구조적으로 사라졌다.

**이 정보로 뭘 보여주나 — 캐럿을 깜빡이지 않고 계속 켜둔다.** 조합
중일 땐 To/Subject/본문 캐럿이 평소처럼 깜빡이는 대신 계속 켜져 있고
색도 남색으로 바뀐다(`draw_editable_field`, 본문 캐럿 둘 다) — "방금
누른 키가 이미 인식됐다"를 확정을 기다리지 않고 매 키 입력마다 바로
알려줘서, 실제 글자가 보이는 타이밍 자체는 못 당기더라도 "입력이 씹히고
있나?" 하는 불안한 느낌은 줄어든다. 정직하게 말하면 이건 근본 해결이
아니라 눈속임에 가까운 완화책이다 — 진짜 글자가 화면에 뜨는 시점 자체는
여전히 Windows IME 의 확정 타이밍을 그대로 따른다.

일본어 IME 도 같은 Win32 경로(GCS_COMPSTR)를 타므로 이론상 똑같이
적용되지만, 요청하신 대로 직접 테스트는 못 했다.

## 화면 왼쪽 위에 조합 중인 글자가 따로 떠 보이던 문제

스크린샷 제보로 진짜 원인을 찾았다: 한글로 입력할 때 방금 조합 중인
글자가 화면 왼쪽 위 구석에 잠깐 따로 떠 있다가 다음 글자를 쳐야 우리
입력칸 안으로 들어온 것처럼 보이는 문제. 이건 우리 렌더링 버그가
아니라 — Windows 자신의 IME 조합창(지금 타이핑 중인 글자를 보여주는
작은 팝업)이었다. 이 게임은 그 팝업을 어디에 띄울지 한 번도 Windows 에
알려준 적이 없어서, Windows 가 기본값인 창의 (0,0) — 왼쪽 위 구석 —
에 그냥 띄우고 있었던 것.

**진짜 커서 위치를 정확히 아는 지금(이전 요청에서 클릭-커서-이동 기능을
만들며 화면 좌표를 이미 계산해두고 있었다) Win32 API 로 그 팝업 위치만
옮겨주면 되는 문제였다.** `ImmSetCompositionWindow`/`ImmSetCandidateWindow`
로 "조합창/후보창은 여기(캐럿 옆)에 띄워달라" 고 매 프레임 알려준다
(`src/ime.rs::set_composition_pos`) — 이 함수는 조합 중인 문자열 내용을
전혀 안 읽는다(좌표만 알려줄 뿐), Windows 자신의 이미 검증된 팝업
렌더링을 그대로 쓰기 때문에 지난 두 번의 사고(문자열을 직접 읽어서
그리려다 생긴 것들)와는 위험 성격 자체가 다르다.

가상 해상도(640x480) 좌표를 실제 창 픽셀 좌표로 되돌리는
`crt.rs::virtual_to_screen()` 도 새로 추가했다 — CRT 배럴 왜곡까지
정확히 역산하진 않는다(팝업 위치가 캐럿 근처면 충분하지, 1px 도 안
어긋나야 하는 자리는 아니라서). To/Subject 필드와 본문 둘 다, 캐럿을
그릴 때마다 같이 이 위치를 갱신한다.

## IME 조합창을 캐럿 옆이 아니라 아예 화면 밖으로

바로 위 수정을 캐럿 옆에 세워 확인한 스크린샷 제보: 실제로 캐럿 옆에
세워보니 Windows 의 둥근 모서리/그림자 있는 현대식 팝업이 이 게임의
픽셀아트 CRT 화면과 전혀 안 어울려서 오히려 더 어색한 이물질처럼 튀어
보였다("이렇게 구현한 건 더 짜쳐").

캐럿 위치 계산(`crt.rs::virtual_to_screen`)은 걷어내고, `set_composition_pos`
호출부(mail.rs, To/Subject·본문 둘 다)를 전부 화면 밖 고정 좌표
(`OFFSCREEN = -10000`)로 바꿨다 — 즉 이제 이 함수는 "캐럿 옆에 세워라"
가 아니라 "아예 안 보이게 치워라" 용도로 쓰인다. 위치를 어디로 옮기든
조합/확정(실제 문자 입력) 자체엔 영향이 없으므로 안전하게 그대로 유지된다
— 화면에는 이제 Windows 의 네이티브 팝업이 전혀 안 뜨고, 대신 안 깜빡이는
캐럿(지난 절충안)만으로 "지금 조합 중" 신호를 준다.

## 한 글자 늦음 — 세 번째 시도: 상한선을 둔 조합 미리보기

"한 글자 늦게 보이는 문제, 진짜 해결 못 하냐"는 질문을 받고, 두 번
되돌렸던 `composition_preview()`(조합 중인 문자열 자체를 읽어와
미리보기로 보여주는 기능)를 상한선을 추가해서 세 번째로 다시 넣기로
합의했다.

**코드를 다시 감사했다.** 바인딩 소스(`windows` 크레이트)까지 직접
확인해보니 `lpbuf=None` 이 null 포인터로 정확히 매핑되는 것,
버퍼 크기 계산, 짝수 바이트 검사 등은 원래도 구조적으로 문제가 없어
보였다 — 다만 "조합 중 문자열 길이가 얼마나 보고되든 일단 믿고 그
크기만큼 읽는다" 는 전제 자체가 위험했다. 이번엔 `MAX_COMPOSITION_BYTES`
(128바이트 = UTF-16 64글자) 상한을 추가했다 — 실제 조합 중 문자열은
한글 한 음절/일본어 미확정 단어 수준을 절대 못 넘으므로, 이보다 큰
값이 보고되면(비정상 상태로 의심) 버퍼를 만들지도 읽지도 않고 그냥
포기한다. 정상적인 입력 상황에서는 전혀 안 걸리는 여유 있는 상한이다.

**미리보기는 이제 캐럿 자리에 직접 끼워넣는다.** 예전엔 항상 문자열
끝에 이어붙였는데, 이번 세션에서 클릭-커서-이동 기능이 생기면서 캐럿이
문자열 중간에도 있을 수 있게 됐다 — 그래서 To/Subject/본문 전부, 조합
중 문자열을 지금 커서 위치에 끼워넣어서 보여주고(`effective_cursor`),
화면 캐럿도 그 미리보기 뒤로 같이 옮긴다. `self.new_mail.to/subject/body`
자체(저장/전송에 실제 쓰이는 값)는 여전히 `char_event` 로 확정된 문자만
으로 채워지고 이 미리보기 값의 영향을 전혀 안 받는다.

여전히 `unsafe` Win32 FFI 라 내가 직접 실행해서 검증할 수는 없다 —
다시 문제가 생기면 바로 알려달라.

## 화면 밖으로 치웠는데도 남아있던 IME 팝업 잔상

미리보기가 잘 작동한다는 확인을 받은 뒤, 그런데도 여전히 네이티브
IME 팝업이 보인다는 제보. `CFS_POINT`/`CFS_CANDIDATEPOS` 만으로는 일부
IME 가 그 좌표를 "힌트" 정도로만 받아들이고 자기 나름의 배치 로직으로
화면 안 다른 자리에 도로 띄울 수 있다 — `CFS_FORCE_POSITION` 플래그를
같이 줘서(`src/ime.rs::set_composition_pos`) 힌트가 아니라 반드시 그
좌표를 쓰도록 강제했다(조합창/후보창 둘 다).

## 그래도 왼쪽 위에 남아있던 팝업 — 세 번째 정체는 "상태 창"

`CFS_FORCE_POSITION` 을 넣은 뒤에도 스크린샷에 화면 왼쪽 위 구석에
작은 상자가 여전히 남아있었다. 조합창/후보창 말고 IME 에 세 번째
팝업이 하나 더 있다는 걸 확인했다 — **상태 창(status window)**, 지금
입력 모드가 한글인지 영문인지 보여주는 작은 표시(스크린샷에 보이던
"여" + 초록 아이콘 같은 게 이거였다). 이건 `ImmSetCompositionWindow`/
`ImmSetCandidateWindow` 가 아니라 완전히 별도인 `ImmSetStatusWindowPos`
로만 옮길 수 있어서, 지금까지 두 팝업만 치우고 이건 안 건드리고 있었다.

`set_composition_pos()` 에 이 세 번째 호출을 추가해서 이제 조합창/
후보창/상태창 세 팝업 전부 화면 밖으로 치운다.

## 그래도 남아있던 네 번째 정체 — CiceroUIWndFrame

세 팝업(조합창/후보창/상태창)을 다 치웠는데도 화면 왼쪽 위에 여전히
작은 상자(스크린샷의 "여" + 초록 아이콘)가 남아있었다. 원인은 저
셋과는 아예 다른 계층이었다 — 저 셋은 전부 옛날식 IMM32 API 로 다루는
팝업인데, 최신 Windows 의 IME 는 **TSF(Text Services Framework, 내부
코드명 "Cicero")** 라는 완전히 별도의 서브시스템으로 언어 표시줄/후보
미니 툴바를 띄운다 — `ImmSetCompositionWindow`/`CandidateWindow`/
`StatusWindowPos` 가 아예 관여하지 않는 창이라 그동안 셋 다 안 먹혔던
것.

이 팝업은 실제로 클래스 이름이 `"CiceroUIWndFrame"` 으로 시작하는,
우리 프로세스가 소유한 별도의 최상위 창으로 뜬다(Windows 내부적으로
잘 알려진 이름). 위치를 옮기는 API 자체가 없어서, 대신 매 프레임
`EnumWindows` 로 우리 프로세스 소유의 모든 최상위 창을 훑어 클래스
이름이 이걸로 시작하는 창을 찾으면 통째로 숨긴다(`ShowWindow(SW_HIDE)`,
`src/ime.rs::hide_cicero_windows`) — 순전히 장식용 UI 창이라 숨겨도
실제 조합/확정(문자 입력 자체)에는 전혀 영향이 없다.

## Backspace 짧게 한 번 눌렀는데 여러 글자가 지워지는 문제

제보: Backspace 를 잠깐 눌렀을 뿐인데 여러 글자가 한꺼번에 지워진다.
원인은 Backspace 꾹 누르기 반복 삭제 로직 자체가 아니라, 그 로직이
기준으로 삼는 `dt`(지난 프레임과의 시간 간격)가 어디서도 상한 없이
그대로 쓰이고 있었다는 것 — 창이 포커스를 다시 받거나 첫 프레임처럼
한 번 크게 끊기면 `dt` 가 순간적으로 아주 커질 수 있는데, 반복 삭제
로직은 "지금까지 이 키가 눌려있던 총 시간"을 `dt` 를 매 프레임 누적해서
재기 때문에, 그 한 번의 큰 `dt` 만으로 "한참 눌려있었던 것"처럼 계산돼
버려서 실제로는 짧게 한 번 누른 것뿐인데 반복 삭제 여러 번이 한
프레임 안에서 몰아서 발생했다.

`main.rs::draw()` 에서 `dt` 자체를 100ms 로 상한을 뒀다 — 큰 끊김이
있어도 그 뒤에 `dt` 를 쓰는 모든 로직(반복 삭제뿐 아니라 스크롤
이징/깜빡임 등 프레임 시간 기반 동작 전부)이 "끊긴 그 순간에 갑자기
크게 움직이는" 대신 정상 프레임을 여러 번 받은 것처럼만 동작한다 —
이 프로젝트 전체에 걸리는 근본적인 안전장치라 앞으로 비슷한 종류의
버그를 예방한다.

## dt 상한을 500ms 로

요청으로 전역 dt 상한을 100ms→500ms 로 올렸다. 다만 이 값은 Backspace
꾹 누르기 반복 삭제 로직 기준으론 여전히 너무 크다(반복 간격 35ms보다
훨씬 큼) — 500ms 짜리 dt 하나가 그대로 들어가면 바로 위에서 고쳤던
"짧게 눌렀는데 여러 글자가 지워지는" 버그가 재발할 수 있었다. 그래서
`mail.rs` 의 Backspace hold 누적에만 별도로 더 촘촘한 상한
(`BACKSPACE_MAX_DT_PER_FRAME = 20ms`)을 추가로 뒀다 — 전역 dt 는
요청대로 500ms 를 쓰되, 반복 삭제 판정에 들어가는 값만 한 번 더
눌러서 큰 프레임 끊김이 있어도 이 기능만큼은 안전하게 유지된다.

## 한글 조합 중 Backspace — 글자가 깨지던 진짜 원인

스크린샷 제보: "한국어"를 입력하고 Backspace 를 한 번 눌렀더니 "한"
뒤에 깨진 글자 조각이 남았다. 앞서 고친 dt 상한/반복 삭제 문제와는
전혀 다른, 더 근본적인 원인이었다 — **Windows IME 와 우리 코드가 같은
Backspace 입력을 동시에 각자 처리하고 있었다.**

한글 IME 는 조합 중(음절이 아직 확정 안 됨)일 때 Backspace 를 누르면
"지금 조합 중인 음절 안에서 자모 하나만 지우기"로 자기가 먼저 처리한다
(예: "어" 를 조합하다 Backspace 를 누르면 "ㅇ" 만 남기고 "ㅓ" 를 지운다
— 음절 전체를 지우는 게 아니다). 그런데 우리 코드는 이 사실을 전혀
모른 채 `win.input.is_down(Backspace)` 를 그냥 봐서, 같은 순간에
우리가 이미 확정해서 들고 있는 문자열(`self.new_mail.body` 등)에서도
따로 한 글자를 지워버렸다 — IME 의 조합 상태와 우리 문자열이 서로
어긋나면서, 화면엔 "확정된 문자열(한 글자 이미 지워짐) + 지금 IME 가
따로 들고 있는 반쯤 지워진 조합 중 음절(ㅇ)" 이 뒤섞여 깨진 조각처럼
보였다.

**고친 방법**: 지금 조합 중인지(`composition_preview()` 가 비어있지
않은 문자열을 돌려주는지)를 먼저 확인해서, 조합 중이면 Backspace/
화살표/Enter 를 우리 쪽에서 아예 처리하지 않는다(`composing` 플래그로
게이팅) — 그 키들은 지금 IME 가 자기 조합 상태를 다루는 데 쓰고
있으니 끼어들지 않고 넘겨준다. 확정된 문자(`char_event`/`win.input.typed`)
는 조합 여부와 무관하게 항상 안전하게 그대로 반영되므로, 조합이 끝나고
정말로 확정된 뒤에는(다음 프레임부터) 다시 정상적으로 Backspace 가
먹힌다.

## 백스페이스 두 번 눌렀는데 글자가 또 깨짐 — 같은 문제의 남은 구멍

바로 위 수정을 적용한 뒤에도, "한국어"를 입력하고 Backspace 를 두 번
누르면 여전히 글자가 깨져 보인다는 제보. 원인은 같은 문제의 타이밍
구멍이었다 — `composing` 여부를 매 프레임 새로 확인하는데, **사람이
키를 한 번 누르고 있는 동안에도 여러 프레임이 지나간다.** 조합 중이던
음절을 IME 가 그 짧은 순간 안에 다 처리해버리면(예: 자모를 지우다
조합 자체를 취소), 아직 같은 물리적인 누름이 이어지고 있는데도(키를
안 뗐는데도) 그 다음 프레임엔 이미 `composing` 이 false 로 보인다 —
그러면 그 순간부터 우리 코드가 "지금 막 새로 눌렸다" 고 오인하고
끼어들어서, 같은 한 번의 누름이 (IME 의 조합 취소) + (우리의 삭제)
로 또 두 번 처리됐다.

`backspace_owned_by_ime` 플래그를 추가해서, 조합 중에 시작된 누름은
"이번 누름 전체가 IME 것" 으로 기억해뒀다가 실제로 키를 뗄 때까지는
(도중에 composing 이 false 로 바뀌어도) 계속 우리 쪽에서 손을 안
댄다 — 키를 완전히 떼고 나서 다시 누르는 것부터가 새로운 누름으로
쳐진다.

## Backspace 반복 지연 200ms 로

요청으로 `BACKSPACE_REPEAT_DELAY`(꾹 눌렀을 때 반복 삭제가 시작되기까지
걸리는 시간)를 350ms→200ms 로 줄였다 — 요청하신 "500"은 아마 최근에
만졌던 전역 dt 상한(main.rs, 500ms)과 헷갈리신 것 같은데, 이건 그거랑
다른 값(원래 350ms)이라 그 쪽을 줄였다. 혹시 dt 상한 쪽을 의도하신
거면 말씀해달라.

## 자음/모음 2개 연속 입력 후 백스페이스 — 두 개가 한 번에 지워짐

제보: 자음이나 모음을 2개 연속으로 입력한 뒤 백스페이스를 누르면 한
번에 두 개가 지워진다. 확실한 원인 하나와, 확신은 못 하지만 남아있을
수 있는 원인 하나를 같이 정리한다.

**확실한 원인 — 방금 200ms 로 줄였던 반복 지연이 너무 짧았다.**
`BACKSPACE_REPEAT_DELAY` 를 200ms 로 줄인 직후라, 사람이 그냥 한 번
"툭" 누르는 물리적인 누름 시간 자체가(느긋하게 누르면) 200ms 를
쉽게 넘을 수 있다 — 그러면 아직 같은 한 번의 누름인데도 "꾹 누르고
있다" 로 오인되어 반복 삭제 한 번이 더 끼어들어서, 짧게 한 번 눌렀는데
두 글자가 지워지는 것처럼 보인다. 300ms 로 다시 올렸다 — 처음(350ms)
보다는 여전히 빠르지만, 평범한 한 번의 탭이 실수로 반복 구간에
걸릴 위험은 줄어든다.

**남아있을 수 있는 원인 — IME 자체가 조합 중 여러 자모를 한 번에
취소하는 경우.** 지난 수정으로 조합 중(자모가 아직 확정 안 됨)인
동안은 Backspace 를 IME 에게 완전히 맡기고 있다(우리 쪽에서 손 안 댐
— 안 그러면 더 심하게 깨지는 문제가 있었다). 그런데 Windows 한글 IME
가 (특히 서로 못 합쳐지는 자음 두 개를 연달아 칠 때처럼) 아직 확정 안
된 자모 여러 개를 "임시 대기 버퍼" 하나로 묶어 들고 있다가, Backspace
한 번에 그 버퍼 전체를 한꺼번에 취소하는 경우가 있을 수 있다 — 이건
우리 코드가 아니라 IME 내부 동작이라, 조합 중엔 손을 안 대기로 한
이상 우리 쪽에서 더 손볼 방법이 없다. 300ms 수정 이후에도 자음/모음
2개 조합 직후에서만 계속 재현되면, 앞의 타이밍 문제가 아니라 이
IME 자체의 특성일 가능성이 크다 — 그때는 다시 알려달라.

## 조합 중 Backspace 를 다시 직접 처리 (위험 감수하고)

500ms→200ms→300ms 로 반복 지연을 만졌는데도 "자음/모음 두 개 이상
적고 나서 백스페이스 한 번에 둘 다 지워진다" 는 문제가 반복/타이밍
로직과 무관하다는 걸 재확인받았다 — 위에서 정리한 "결합 자모를 IME
자체가 한 번에 취소한다" 는 설명이 맞았던 것. 지난번엔 이걸 피하려고
조합 중엔 Backspace 를 IME 에게 통째로 맡기기로 했었는데, "위험을
감수하고라도 직접 처리해달라" 는 요청을 받고 다시 되돌렸다.

**이번엔 조합 문자열을 읽거나 그리는 대신, 조합을 우리가 직접
편집한다.** Backspace 를 누른 그 프레임에, `composition_preview()`
로 지금 조합 중인 문자열을 읽고(예: "ㄲ"), Windows 에게 "이 조합은
버려라"(`ImmNotifyIME(..., CPS_CANCEL, ...)`, Esc 를 누른 것과 같은
효과)라고 알린 뒤, 읽어둔 문자열에서 우리가 직접 마지막 한 글자만
뺀 나머지("ㄲ" → "ㄱ")를 우리 텍스트에 그냥 확정된 문자처럼 밀어넣는
다 — "조합 중 Backspace 한 번 = 자모 하나만 지우기" 를 IME 의
판단에 맡기지 않고 우리가 직접 보장한다(`src/ime.rs::cancel_composition`,
`src/apps/mail.rs`). 계속 누르고 있어도 이 처리는 딱 그 첫 프레임
(`pressed`, 매 프레임이 아니라)에만 한 번 일어나고, 취소한 순간부터
"조합 중"이 아니게 되므로 그 다음부터는 이미 있던 꾹 누르기 반복
삭제 로직이 그대로 이어받는다.

**다만 이건 예전보다 더 위험한 시도라는 걸 분명히 해둔다.** 지금까지
만든 IME 관련 코드는 전부 "읽기만" 하거나(`composition_preview`,
`is_composing`→제거됨) "팝업 위치만 옮기는" 수준이었는데, 이번
`cancel_composition()` 은 처음으로 조합 상태 자체를 실제로 바꾸는
(취소하는) 부수효과가 있는 호출이다 — Backspace 를 누른 그 순간의
타이밍(우리 코드가 실제로 IME 가 그 키를 내부적으로 처리하기 *전에*
끼어드는 게 맞는지)을 내가 직접 검증할 방법이 없어서, 의도한 대로
"자모 하나만 지워진다"고 확신할 수 없다 — 어쩌면 IME 가 이미 그
키 입력을 다 처리한 *뒤에* 우리가 읽는 것이라 별 효과가 없을 수도
있고, 반대로 새로운 종류의 어긋남이 생길 수도 있다. 확인 부탁드린다.

## → 위 시도 실패, 되돌림

확인 결과 여전히 두 개가 한꺼번에 지워졌다 — `cancel_composition()`
이 사실상 아무 효과가 없었다. 원인은 우려했던 그대로였다: Windows
는 Backspace 의 `WM_KEYDOWN` 을 `DefWindowProc` 안에서 동기적으로
처리하면서, IME 자신의 "조합 중 Backspace" 로직(결합 자모를 한
번에 취소하는 것 포함)까지 그 안에서 다 끝내버린다 — 이 전체 과정이
우리 `draw()` 루프(여기서 `composition_preview()` 를 읽고
`cancel_composition()` 을 호출한다)가 그 프레임을 처리하기 *전에*
이미 다 끝나 있다. 즉 우리가 조합 문자열을 읽는 시점엔 IME 가 이미
자기 방식대로(자모 전체를 한 번에) 축소해버린 뒤라서, 우리 쪽
취소+재조립 시도는 이미 늦은 개입이었다.

이걸 정말로 고치려면 `WM_KEYDOWN` 자체를 `DefWindowProc` 이 처리하기
전에 가로채야 하는데, 그건 이 프로젝트가 쓰는 miniquad 크레이트의
윈도우 메시지 루프 내부를 직접 고쳐야 하는 훨씬 큰 작업이라 이번엔
손대지 않았다.

`src/ime.rs::cancel_composition()` 은 삭제했고, `src/apps/mail.rs`
는 이전의 안전한 방식(`backspace_owned_by_ime` — 조합 중 시작된
누름은 키를 뗄 때까지 통째로 IME 에게 맡기고 우리는 손 안 댐)으로
되돌렸다. 결합 자모(ㄲ, ㅘ 등)가 Backspace 한 번에 통째로 지워지는
것 자체는 Windows/IME 의 고유 동작이라 앱 레벨에선 더 손볼 수
없다 — 알려진 한계로 남겨둔다.

## → 다시 도전: WndProc 서브클래싱으로 진짜로 먼저 가로채기

위에서 "miniquad 자체를 고쳐야 한다"고 결론 냈던 부분을 다시
파봤다 — 사실 꼭 miniquad 소스를 고칠 필요는 없었다. Windows 창은
누가 만들었든 실행 중에 `SetWindowLongPtrW(hwnd, GWLP_WNDPROC, ...)`
로 "서브클래싱"해서 우리 프로시저를 원래 프로시저 앞에 끼워넣을 수
있는 표준 Win32 기능이 있다 — miniquad 가 이미 만들어둔 창(HWND)에
런타임에 이 API 하나만 호출하면 되고, 벤더 소스는 전혀 안 건드린다.

이걸로 `src/ime.rs::install_backspace_hook()` 을 새로 만들었다.
우리 프로시저는 `WM_KEYDOWN(Backspace)` 이 들어오면 원래 프로시저에
넘기기 *전에* 먼저 가로채서, 지금 조합 중인 문자열을 확인한다:
- 1글자 이하면 → 그냥 원래 프로시저(→ IME 의 기본 처리)에 넘긴다
  (이 경우는 원래도 문제없었다).
- 2글자 이상이면(결합 전 자모 두 개, 예: "ㄲ" 이전의 "ㄱㄱ") → 우리가
  `ImmNotifyIME(CPS_CANCEL)` 로 조합을 취소하고,
  `ImmSetCompositionStringW(SCS_SETSTR)` 로 마지막 한 글자만 뺀
  나머지를 다시 조합 중 상태로 세팅한 뒤, 이 메시지를 원래 프로시저에
  아예 넘기지 않고 여기서 완전히 삼킨다 — IME 의 "결합 자모 전체를
  한 번에 취소" 로직이 실행될 기회 자체가 없어진다.

지난 시도와의 핵심 차이: 지난번엔 우리 draw() 루프(매 프레임, 즉
`WM_KEYDOWN` 이 이미 다 처리되고 한참 뒤)에서 개입하려 해서 늦었지만,
이번엔 `WM_KEYDOWN` 메시지 자체를 원래 프로시저가 받기 *전에* 우리가
먼저 받으므로, 정말로 IME 의 내부 처리보다 앞선 시점에 개입한다.

`install_backspace_hook()` 은 Mail 작성 화면을 그릴 때마다
(`draw_new_compose()` 시작 부분에서) 부르지만 두 번째 호출부터는
플래그로 그냥 무시되는 1회성 설치다.

**주의할 점**: 이번 지점은 지금까지 만든 IME 코드 중 가장 위험한
지점이다 — 창 프로시저 자체를 바꿔치기하는 거라, 만약 잘못되면(예:
원래 프로시저 포인터를 잘못 저장/복원) 텍스트 입력 전체가 안 되거나
창이 완전히 멈출 수도 있다. Backspace 가 아닌 다른 모든 메시지는
손대지 않고 그대로 원래 프로시저에 넘기도록 최대한 좁게 만들었지만,
실행해서 직접 검증할 수 없는 unsafe FFI 라 확인 부탁드린다 — 특히
"타이핑이 아예 안 된다"/"창이 멈춘다" 같은 게 보이면 바로 알려달라,
전체를 되돌릴 수 있다.

## → 이것도 실패, 방향을 완전히 바꿈: "미리 막기" 대신 "사후 보정"

확인 결과 서브클래싱 버전도 여전히 재현됐다 — 우리 창 프로시저가
`WM_KEYDOWN` 을 원래 프로시저보다 먼저 받도록 만들어도 소용없었다는
뜻이다. 원인을 다시 생각해보면: 최신 한글 IME(Microsoft IME)는
TSF(Text Services Framework, `hide_cicero_windows()` 가 다루는 그
"CiceroUIWndFrame" 서브시스템)를 쓰는데, TSF 는 키 입력을 자체
후킹(우리 창의 `WM_KEYDOWN` 디스패치보다도 더 앞단)으로 가로채
처리하는 것으로 보인다 — 그렇다면 우리가 아무리 창 프로시저를
서브클래싱해도, 그 시점엔 이미 TSF 가 조합 상태를 다 바꿔놓은
뒤일 수 있다. 진짜 TSF 수준에서 개입하려면 `ITfKeyEventSink` 같은
TSF 전용 COM 인터페이스를 직접 구현해야 하는데, 이건 사실상 앱을
TSF 클라이언트로 새로 만드는 것에 가까운 훨씬 큰 작업이라 이번에도
손대지 않았다.

그래서 "WM_KEYDOWN 시점에 미리 막기"라는 방향 자체를 접고, 완전히
다른 방향으로 접근했다 — **이미 지워진 뒤에 바로잡기.**

`MailApp` 에 `last_composition: String` 필드를 새로 만들어서 매
프레임 지금 조합 중인 문자열을 기억해둔다. Backspace 가 눌린
프레임에, 이번 조합 문자열이 "직전 프레임 값보다 2글자 이상"
줄어들어 있으면(=IME 가 결합 전 자모를 한 번에 지운 것) — 직전
프레임 값(우리가 스스로 기억해둔, 확실히 정상이었던 값)을 기준으로
"마지막 한 글자만 뺀 나머지"를 계산해서 그걸 다시
`ImmSetCompositionStringW` 로 조합 중 상태에 밀어넣는다
(`src/ime.rs::set_composition_string`, `src/apps/mail.rs`).

이게 첫 번째 시도(`cancel_composition`)와 다른 점: 그때는 "지금 막
읽은(이미 IME 가 지워버린 뒤의) 값"에서 자르려고 해서 애초에 기준이
잘못됐었다(자를 게 이미 없거나 이미 줄어들어 있었다). 이번엔 우리가
"직전 프레임의, 우리가 확실히 아는 정상 값"을 따로 저장해두고
그걸 기준으로 계산하므로, IME 가 그 사이 내부적으로 몇 개를 지웠든
상관없이 항상 "정확히 자모 하나만큼만 줄어든 상태"로 강제로
맞출 수 있다. 그리고 이 보정은 그 프레임의 렌더링(필드 그리기)보다
먼저 일어나므로, 화면상으로는 깜빡임 없이 바로 옳은 값만 보인다.

이 접근은 WM_KEYDOWN 을 가로채거나 창 프로시저를 건드릴 필요가
전혀 없다 — 이미 안전하다고 검증된 "조합 문자열 읽기/쓰기" API 만
평소처럼 매 프레임 폴링하는 방식이라(`composition_preview` 를
그릴 때 쓰는 것과 같은 부류), 그래서 지난 서브클래싱 시도보다
훨씬 안전하다. `install_backspace_hook`/`subclass_wndproc`/
`try_intercept_backspace` 는 전부 삭제했다.

여전히 unsafe FFI 라 실행해서 직접 검증은 못 한다 — 이번에도 안
되면 알려달라, 그땐 TSF 수준까지 내려가는 훨씬 큰 작업이 필요하다는
뜻이 된다.

## → 이것도 실패, 왜 안 됐는지 알아냄 + 완전히 다른 방식으로 해결

역시 재현됐다 — 그런데 이번엔 왜 안 됐는지도 알아냈다. 지난 감지
로직은 "직전 프레임보다 조합 문자열이 2글자 이상 줄었는지"로
판정했는데, 이 전제 자체가 틀렸었다: 결합 모음(ㅘ)이나 겹받침(ㄳ)이
낀 음절은 Windows 가 애초에 자모를 낱개 문자 여러 개로 늘어놓지
않는다 — 완성형 한글 음절 코드포인트 하나(가~힣 범위, U+AC00~
U+D7A3)로 이미 합쳐서 조합 버퍼에 담아둔다. 그래서 "자모 2개가 조합
중"이어도 실제 문자열 길이는 항상 1글자였고, Backspace 를 누르면
그 1글자가 통째로 0글자가 되는 것으로만 보였다 — "2글자 이상
줄었는지" 비교로는 이 경우를 원천적으로 잡아낼 방법이 없었다.

그래서 방향을 또 한 번 바꿨다 — 이번엔 "IME 가 실제로 뭘 지웠는지
감지"하는 시도 자체를 그만뒀다. 대신 Backspace 를 한 번 눌렀을 때
"논리적으로 정확히 무엇이 남아야 하는지"를 유니코드 한글 음절 분해
공식으로 우리가 직접 계산해서, 그 값을 무조건 정답으로 삼고 강제로
덮어쓴다.

완성형 한글 음절은 초성(choseong)·중성(모음)·종성(받침) 세 요소를
하나의 숫자로 인코딩한 것이고, 그 변환 공식은 유니코드 표준에 고정돼
있다. 여기에 겹받침(ㄳ/ㄵ/ㄺ/ㄻ/ㄼ/ㄽ/ㄾ/ㄿ/ㅀ/ㅄ)과 이중모음(ㅘ/ㅙ/ㅚ/
ㅝ/ㅞ/ㅟ/ㅢ)이 각각 어떤 낱자 두 개로 이루어져 있는지도 표준으로
정해져 있다. 이 둘을 합쳐서 `src/ime.rs::hangul_step_back()` 을
만들었다 — 조합 중 문자열의 마지막 글자를 분해해서, 받침이 있으면
받침을 한 단계(겹받침이면 그 절반만) 빼고, 받침이 없으면 모음을
한 단계(이중모음이면 그 절반만) 빼고, 그것도 없으면 초성 낱자 하나만
남긴다 — 즉 "가" → "ㄱ", "각" → "가", "갃"(겹받침 예시) → "갂" 처럼
항상 정확히 한 단계씩만 물러난다. 완성형 음절이 아닌 낱개 자모는
더 쪼갤 게 없으니 그냥 통째로 지운다.

`src/apps/mail.rs` 는 Backspace 를 "짧게 한 번" 눌렀을 때(제보에
따르면 꾹 누르고 있는 경우는 원래도 문제없었으므로 그쪽은 그대로 두고,
`win.input.pressed()` — OS 자동反복을 걸러낸 최초 누름 한 번에만
true — 로만 이 로직을 켠다) 직전 프레임에 기억해둔 조합 문자열을
`hangul_step_back()` 에 넣어 계산한 값을 `set_composition_string()`
으로 조합 버퍼에 강제로 밀어넣는다 — Windows 가 그 사이 내부적으로
뭘 지웠든 전혀 신경 쓰지 않고, 우리가 계산한 값이 항상 최종 결과가
된다. 이건 "IME 가 뭘 했는지 알아내서 맞춰주는" 게 아니라 "IME 가
뭘 했든 무시하고 우리가 정답을 강제로 덮어씌우는" 방식이라, 지난
세 번의 실패(사후 취소/창 프로시저 서브클래싱/길이 비교 감지)와
근본적으로 다르다 — TSF 가 어느 시점에 뭘 하든, 우리 draw() 루프가
그 결과를 매 프레임 다시 덮어써 버리므로 상관없다.

여전히 unsafe FFI 라 실행해서 직접 검증은 못 하지만, 이번 방식은
"IME 내부 동작을 추측"하는 게 아니라 "유니코드 표준 공식으로 정답을
직접 계산"하는 방식이라 앞의 세 시도보다 신뢰도가 훨씬 높다고 본다 —
확인 부탁드린다.

## → 네 번째도 실패, 의심되는 진짜 원인 + "눈속임이라도 좋으니" 접근

역시 재현됐다("여전한데?? 못고쳐???"). 네 번의 시도 중 뒤의 두 번
(길이 비교 감지, 음절 분해 계산)은 계산 로직 자체는 서로 다르지만
공통점이 하나 있다 — 둘 다 "고친 값을 `ImmSetCompositionStringW` 로
조합 버퍼에 다시 써넣어서, 계속 조합 중인 상태를 유지"하려고 했다.
이 공통 지점이 실은 처음부터 안 먹히고 있었던 게 아닌가 의심된다 —
최신 한글 IME(Microsoft IME)는 TSF 기반인데, 레거시 IMM32 의 "조합
취소"(`ImmNotifyIME` CPS_CANCEL)는 이미 여러 번 효과가 확인됐지만
(그 직후 팝업이 사라졌다), 취소 직후 다시 `ImmSetCompositionStringW`
로 "여기서부터 새 조합을 시작해라"라고 밀어넣는 건 TSF 쪽 호환
계층이 그냥 무시하는 것으로 보인다 — 그래서 우리가 계산은 맞게
해놓고도 그 값이 실제 조합 버퍼에 전혀 반영되지 않았을 가능성이
높다.

그래서 이번엔 "다시 조합 중 상태로 되돌리기"라는 목표 자체를
포기했다 — 사용자가 "차라리 눈속임이라도 좋으니 해결해달라"고
요청한 대로, 완벽하게 매끄러운 연속 조합보다 "정확한 삭제 개수"를
우선한다.

`src/ime.rs::cancel_composition()` 은 이제 취소만 한다(재조합 시도
없음 — 이미 확실히 먹히는 부분만 남겼다). `src/apps/mail.rs` 는
Backspace 를 짧게 한 번 눌렀을 때, 직전 프레임 조합 문자열을
`hangul_step_back()` 에 넣어 "정확히 한 단계 물러난 값"을 계산한
뒤 — 조합 버퍼에 다시 밀어넣는 대신 조합을 완전히 취소해버리고, 그
계산값을 그냥 평범한 확정 문자로 우리가 이미 전적으로 신뢰하는
`insert_char_at()` 을 통해 텍스트에 직접 밀어넣는다.

트레이드오프: 이 지점 이후로 이어서 타이핑하면 IME 가 그 자리에서
완전히 새로운 조합을 시작한다 — 예를 들어 "가"까지 입력하고
Backspace 로 "ㄱ"까지 되돌린 뒤 다시 모음을 이어 치면, 원래 IME
안에서 매끄럽게 재조합되는 것과 100% 똑같이 느껴지지 않을 수도
있다(다만 "ㄱ" 자체가 이미 확정 문자로 들어가 있으므로, 그 뒤에 오는
모음이 자연스럽게 결합되는지는 실제로 확인이 필요하다 — Windows
IME 는 보통 확정된 텍스트 바로 뒤에 커서가 있으면 그 자모를 다시
주워서 결합을 이어가는 경우가 많다). 어느 쪽이든 "한 번 눌러서 몇
개가 지워지는지"는 이제 IME 의 내부 동작과 완전히 무관하게 항상
정확하다 — 이 부분이 이번 시도의 핵심이다.

여전히 unsafe FFI 라 실행해서 직접 검증은 못 한다 — 확인 부탁드린다.

## → 다섯 번째: "계산" 대신 "기록"으로 바꿈 (쥬인님 제안)

이것도 재현됐다. 쥬인님이 직접 아이디어를 주셨다 — "차라리 가상
입력까지 배열로 저장했다가 삭제할 때 갑자기 2개가 삭제되면 앞쪽
한 개는 복구되도록 만들면 안 돼??" 정확히 이 방향으로 다시 만들었다.

지난 시도(한글 음절 분해로 "한 단계 전 값"을 유니코드 공식으로
직접 계산)는 계산 자체가 틀렸을 수도 있고, 아니면 그 전전 시도들과
마찬가지로 "계산한 값을 조합 버퍼에 다시 밀어넣기"가 원천적으로
안 먹혀서였을 수도 있다 — 어느 쪽이었는지 확실히 가릴 방법이 없었다.

그래서 이번엔 "계산"을 아예 없앴다. `MailApp` 에
`composition_history: Vec<String>` 를 새로 만들어서, 지금 조합
세션 동안 IME 가 실제로 화면에 보여준 값이 바뀔 때마다("ㅎ" →
"호" → "화") 그 스냅샷을 그대로 순서대로 쌓아둔다. Backspace 를
짧게 한 번 누르면, 이 기록에서 "지금 값 바로 전"에 있던 값을 그대로
꺼내 쓴다 — 유니코드 계산이 전혀 없으니 우리 쪽 산수가 틀릴 여지
자체가 없다(그냥 우리가 이미 화면에 보여줬던 걸 다시 보여주는 것뿐).
그 값을 조합 버퍼에 다시 넣는 대신(이미 안 먹히는 걸로 확인됐다)
`ime::cancel_composition()` 으로 조합을 완전히 취소하고 평범한 확정
문자로 텍스트에 직접 밀어넣는 방식은 그대로 유지했다.

이제 못 쓰게 된 `ime::hangul_step_back()` (한글 음절 분해 계산
함수)과 그 보조 표는 삭제했다.

여전히 unsafe FFI 라 실행해서 직접 검증은 못 한다 — 확인 부탁드린다.

## 휴지통을 트리 확장에서 제외

파일 탐색기 왼쪽 "폴더" 트리에서 데스크톱 항목을 펼치면(+/- 박스)
휴지통이 열려버리는 문제 — 원인은 `src/apps/explorer.rs` 의
`has_expandable_children`/`next_unexpanded_child` 가 "폴더류인가"
판정에 `ui::is_folder_like()` 를 그대로 썼는데, 이 함수는 목록 뷰에서
"크기 칸을 비워둘 항목인가"를 정하는 용도라 휴지통 아이콘
(RecycleEmpty/RecycleFull)도 폴더류로 포함하고 있었다. 그래서
데스크톱 탭을 펼치려고 +/- 를 누르면, 데스크톱 아이콘 목록에서 처음
만나는 "폴더류" 항목이 하필 휴지통이라(휴지통이 데스크톱 아이콘들
중 앞쪽에 있다) 그 항목이 트리 확장 후보로 뽑혔다 — 휴지통은
`desktop.rs::is_drilldown_folder` 에서 이름으로 걸러져서 트리 탭이
아니라 독립 창(RecycleBinApp)으로 열리므로, 결과적으로 "데스크톱을
펼치려고 눌렀는데 휴지통 창이 뜬다"가 됐다. 게다가 그 뒤로도 계속
휴지통만 후보로 잡혀서 데스크톱 밑의 진짜 하위 폴더는 트리에서 영영
못 펼쳤다.

`explorer.rs` 에 `tree_expandable()` 을 새로 만들어(일반
`IconType::Folder` 만 인정, 휴지통 아이콘은 제외) 트리 확장
판정에서만 쓰도록 분리했다 — 목록 뷰의 크기 칸 판정(`is_folder_like`)
은 그대로 둬서 휴지통이 여전히 "크기 없음"으로 보이는 건 안 바뀐다.
이제 휴지통은 트리에서 펼쳐지는 "파일/폴더"가 아니라 항상 더블
클릭으로 여는 "프로그램"으로만 취급된다.

## 드래그로 놓을 자리를 다른 창이 가리고 있을 때

바탕화면 아이콘을 끌어다 놓거나, 열린 폴더 안의 파일을 끌어다 놓을
때 그 자리를 다른 창이 가리고 있으면 그 창을 무시하고 그 자리에
그냥 놓아버리는 문제 — 옮긴 아이콘이 그 창 뒤에 숨어버려서(창을
치우기 전엔 안 보임) 아이콘이 없어진 것처럼 보였다.

두 경로 다 손봤다(`src/scenes/desktop.rs`):

- **바탕화면 아이콘을 바탕화면 빈 곳으로 드래그**: 놓은 자리가 (폴더로
  옮길 수 있는 창이 아닌) 다른 창으로 가려져 있으면 — `wm.file_at(m)`
  으로 그 자리에 어떤 창이든 있는지 먼저 확인해서 — 그 자리에 새로
  놓지 않고 원래 있던 자리 그대로 둔다(드래그를 취소한 것과 같다).
  폴더/휴지통 창 위에 놓은 경우는 그대로 그 폴더 안으로 들어간다(이미
  되던 동작, 안 바뀜).
- **열린 폴더 안의 파일을 그 창 밖으로 드래그**: `ExplorerApp` 은 자기
  창 밖으로 나가면 무조건 "바탕화면으로" 요청하는데(자기가 아는 건
  자기 사이드바뿐이라 바탕화면/다른 창 위치는 모른다), desktop.rs 가
  그 드롭 지점을 다시 확인해서 다른 열린 탐색기 창 위였으면 그 폴더로,
  바탕화면의 다른 폴더 아이콘 위였으면 그 폴더로 바꾼다(바탕화면
  아이콘 드래그와 같은 우선순위 — 이 부분은 desktop_folder_drop_target_at
  만 확인하고 있어서 창은 안 봤었는데 이번에 explorer_drop_target_at
  도 같이 확인하도록 고쳤다). 그래도 여전히 바탕화면행인데 그 지점을
  다른 창이 가리고 있으면, 그 좌표 그대로 내려놓는 대신 확실히 비어
  보이는 자리(`first_free_tile` — 휴지통 Restore 가 이미 같은 이유로
  쓰던 방식)에 놓는다.

## My Computer 사이드바로 바탕화면에 옮기면 자동 정렬

My Computer 안에서 사이드바의 "Desktop" 항목 위로 파일을 끌어다
놓으면(창 밖으로 안 나가고 사이드바 안에서 끝나는 드래그) 자동으로
잘 정돈된 빈 자리에 놓이게 해달라는 요청 — 사실 위 항목에서 이미
"드롭 지점을 창이 가리고 있으면 first_free_tile 로" 로직을 넣어뒀는데,
그게 제대로 작동하려면 먼저 "이 드롭이 정말 Desktop 행인지" 재확인하는
과정을 통과해야 했다. 그런데 그 재확인(`explorer_drop_target_at`) 이
"지금 마우스 위치에 있는 창의 활성 탭이 뭔지"를 물어보는 거라, 사이드바
드롭처럼 마우스가 My Computer 자기 자신 위에 있는 경우엔 (사이드바에서
누른 "Desktop" 이 아니라) 그때 화면에 실제로 보이고 있던 다른 탭(예:
Downloads)의 위치로 잘못 덮어써버리는 버그가 있었다 — 즉 사이드바에서
분명 Desktop 위에 놓았는데 엉뚱한 곳(지금 보고 있던 탭)으로 옮겨질 수
있었다.

`src/scenes/desktop.rs` 의 `DeskAction::MoveFiles` 처리에서, 드롭
지점(`wm.file_at(m)`)이 My Computer 자기 자신이면 이 재확인 자체를
건너뛰도록 고쳤다 — 사이드바 위에서 놓은 경우는 `ExplorerApp` 이 이미
정확히 골라준 목적지(Desktop)를 그대로 믿는다. 그 뒤 "바탕화면행인데
그 자리를 어떤 창이든(자기 자신 포함) 가리고 있으면 first_free_tile
로" 로직은 그대로 이어져서, 이제 사이드바로 Desktop 에 옮기면 항상
빈 자리에 자동으로 잘 정렬돼 놓인다.

## 휴지통: 안의 파일은 안 열리게, 열려있는 파일은 못 버리게

- **휴지통 안의 파일은 더블클릭해도 안 열린다** — `src/apps/recycle_bin.rs`
  에서 더블클릭 시 `AppAction::Open` 을 돌려주던 부분을 제거했다(지운
  걸 계속 열어 쓸 수 있으면 "지웠다"는 의미가 흐려진다는 취지). 그
  판정에만 쓰던 `last_idx`/`last_click` 필드도 이제 안 쓰여서 같이
  지웠다 — 단일 클릭 선택은 `icon_grid` 가 이미 처리해주므로 그대로
  된다.
- **지금 창이 열려있는 파일은 휴지통으로 못 옮긴다** —
  `src/scenes/desktop.rs::move_ids_to()` 의 이동 루프에 `self.wm.is_open(id)`
  검사를 추가해서, 휴지통(`MoveDest::Folder(recycle_bin)`)으로 가는
  경우에만 지금 열려있는 파일을 건너뛰고(원래 있던 자리 그대로) 나머지
  파일들은 평소처럼 옮긴다 — 열어서 보고 있는 파일을 지워버리면(휴지통
  비우기까지 하면 영구 삭제) 그 창이 이제 존재하지 않는 파일을 가리키는
  유령 창이 되는 걸 막는다.

## 코드 정리 (주석 다이어트)

이번 세션 IME/Backspace 씨름 과정에서 코드 안에 남긴 "몇 번째 시도가
왜 실패했는지" 식의 긴 서사형 주석들 — 그 경위는 이미 이 README 에
빠짐없이 남아있으니 코드 안에까지 통째로 반복할 필요가 없다.
`src/ime.rs`(모듈 문서 주석, `cancel_composition` 위), `src/apps/mail.rs`
(Backspace 처리 블록), `src/scenes/desktop.rs`(`DeskAction::MoveFiles`,
바탕화면 드래그 처리)의 주석을 지금 코드가 왜 이렇게 동작하는지만
남기고 "어떤 시도를 몇 번 했었는지" 나열은 걷어냈다 — 동작은 전혀
안 바뀌었다(빌드/clippy 통과, 경고 수 그대로).

`#[allow(dead_code)]`류로 숨겨둔 죽은 코드는 프로젝트 전체에 하나도
없었다 — 그 외 11개 모듈(특히 큰 `mail.rs`/`desktop.rs`)까지 통째로
구조를 다시 짜는 건 자동 테스트가 없는 이 프로젝트에서 한 번에 하기엔
회귀 위험이 커서, 이번엔 이번 세션에서 직접 건드려 주석이 가장
불어나 있던 파일들 위주로만 정리했다.

## 시계 클릭 시스템 메시지 패널 제거

작업표시줄 시계를 누르면 오른쪽에서 슬라이드로 나오던 "System
Messages" 패널(메일 도착 알림 한 줄만 보여주던 것)을 통째로 뺐다 —
`messages_open` 상태, `messages_panel_rect()`, `draw_messages_panel()`
과 그걸 여닫던 클릭 처리, 커서/오버레이 판정에서의 참조까지 전부
제거했다. 시계는 이제 그냥 시각 표시용이고 눌러도 아무 일도 안 한다
(우측 하단에 잠깐 떴다 사라지는 "New Mail" 토스트 알림은 시계와
무관한 별개 기능이라 그대로 남아있다).

## 연출/영상 제작용 컨트롤러 (Director) 추가

트레일러/스크린샷처럼 원하는 장면만 깔끔하게 촬영할 수 있게, 특정
씬을 바로 띄우거나 로비 화면의 정전기 글리치를 실행 중에 껐다 켤 수
있는 **별도 실행 파일**을 만들었다 — 실제 게임(`crackhead.exe`)은
전혀 안 건드린다.

**구조를 먼저 바꿔야 했다**: 지금까지 `src/main.rs` 가 `mod apps; mod
scenes; ...` 로 모든 모듈을 직접 선언하는 단일 바이너리였는데, 이러면
같은 크레이트 안에서만 그 모듈들을 쓸 수 있어서 두 번째 실행 파일이
씬/렌더러 코드를 재사용할 방법이 없었다. 그래서 `src/lib.rs` 를 새로
만들어 그 `mod` 선언들을 전부 `pub mod` 로 옮기고, `src/main.rs` 는
그 라이브러리 크레이트(`crackhead`)를 가져다 쓰는 쪽으로 바꿨다 —
동작은 완전히 그대로다(그냥 선언 위치만 옮겼다).

이 분리의 부수 효과: 지금까지 `cargo clippy --bin crackhead` 로 검증
하던 "정확히 5개 경고" 기준이 이제 `crackhead`(lib)/`crackhead`(bin
"crackhead")/`director`(bin) 세 타깃으로 나뉜다 — lib 쪽에 원래
있던 5개(전부 `too_many_arguments`)가 그대로 남았고, 두 바이너리는
각자 0개다. 또한 라이브러리로 공개되면서 `FileSystem`/`Settings`/
`BootScene`/`DesktopScene`/`EraseScene`/`LobbyScene`/`ShutdownScene`/
`WindowManager` 에 clippy 가 `Default` 구현을 새로 요구해서(`pub fn
new()` 가 인자 없이 만들 수 있는데 `Default` 가 없으면 라이브러리
소비자 입장에서 불편하다는 지적) 전부 `Self::new()` 로 위임하는
`impl Default` 를 추가했다 — 동작에는 아무 영향 없다.

**`src/bin/director.rs`** — Cargo 관례상 `src/bin/` 밑의 파일은
자동으로 별개의 실행 파일이 된다(`cargo build --bin director`).
`main.rs` 의 `Stage`(창 생성 + 렌더 루프)를 그대로 본떠 만들었고,
왼쪽 위에 작은 패널을 하나 더 그린다 — Boot/Lobby/Desktop/Erase/
Shutdown 버튼(누르면 `SceneManager::set()` 으로 그 씬을 바로 띄운다
— 이 메서드도 이번에 새로 추가했다, 평소 게임은 항상 정상 `Transition`
경로만 타서 안 쓴다) 과 "Glitch: ON/OFF" 토글(누르면 `LobbyScene` 이
공유해서 보는 `Rc<Cell<bool>>` 를 뒤집는다 — 이것도 새로 추가한
`LobbyScene::with_glitch_control()` 생성자로만 연결된다, 평소
`LobbyScene::new()` 는 이 handle 을 늘 true 로 자기 혼자 들고
있어서 실제 게임의 글리치 동작은 전혀 안 바뀐다). **F1** 로 패널
자체를 숨길 수 있다 — 녹화 직전에 눌러서 화면에서 완전히 치우면
된다(숨겨진 동안도 조작은 그대로 다 된다).

**파일 위치**: 소스는 게임 본체 소스(`src/apps`, `src/scenes` 등)와
섞이지 않게 `src/bin/director.rs` 한 파일로 따로 뺐고, 빌드한 실행
파일도 `production/director/PalaceOS-Director.exe` 에 따로 둬서
`target/` 빌드 산출물이나 게임 저장 파일(`palaceos_save.json` 등)과
안 섞이게 했다. 소스를 고친 뒤엔 아래 명령으로 다시 빌드해서 그
자리에 새로 복사하면 된다:

```
cargo build --bin director --release
cp target/release/director.exe production/director/PalaceOS-Director.exe
```

## → 컨트롤 UI를 진짜 별개의 창으로 분리

방금 만든 director 는 조작 패널을 게임 화면 위에 겹쳐 그렸다(F1 로
숨기는 방식) — "옆에 별도의 창으로 툴도 같이 뜨면 좋겠다"는 요청을
받고, 진짜 두 번째 OS 창(별개 프로세스)으로 바꿨다. miniquad 는 한
프로세스 안에서 창을 두 개 띄우는 걸 지원하지 않아서, 그냥 실행 파일
자체를 두 개로 나눴다:

- **`director.exe`** — 게임 화면 그 자체(=실제로 녹화할 화면). 이제
  화면 위에 아무 UI 도 안 그린다.
- **`director_panel.exe`** — 그 옆에 따로 뜨는 작은 조작 창. CRT
  곡면 효과 없는 평범한 창이라 버튼 누르기도 더 편하다.

**두 창은 서로 다른 프로세스라 직접 함수를 부를 수 없다** — 그래서
`src/director_ipc.rs` 라는 아주 작은 공유 모듈을 새로 만들어, 두
실행 파일이 같은 폴더에 두는 `director_state.json` 하나로 통신하게
했다. panel 에서 버튼을 누르면 그 파일에 원하는 상태(씬 전환 요청,
글리치/노이즈 on-off)를 써두고, director 가 매 프레임 그 파일을
다시 읽어서 반영한다 — 씬 전환은 한 번 적용한 뒤 파일에서 지워서
반복 적용되지 않게 하고, 글리치/노이즈는 매 프레임 그대로 옮겨 적어서
항상 즉시 반영되게 했다. 파일 이름을 게임 저장 파일(`palaceos_save.json`)
/설정 파일(`palaceos_settings.json`)과 분명히 다르게 지어서 절대
안 섞이게 했다 — 실제 게임(`crackhead.exe`)은 이 모듈을 아예 참조하지
않는다.

**"글리치/노이즈 등등"** 요청에 맞춰 정전기 알갱이(noise)와 찢김
밴드(glitch)를 이제 따로 껐다 켤 수 있다 — `LobbyScene` 이 예전엔
`glitch_enabled` handle 하나만 받았는데, `noise_enabled` handle을
추가해서 `draw_noise()` 안에서 정전기 알갱이 루프와 글리치 밴드
루프를 각각 독립적으로 게이팅한다(생성자도 `with_glitch_control()`
→ `with_director_controls(glitch, noise)` 로 합쳤다). 평소 게임은
`LobbyScene::new()` 가 둘 다 항상 true 인 자기 전용 handle 로 시작해서
전혀 영향 없다.

`director.exe` 를 실행하면 같은 폴더의 `director_panel.exe` 를
자동으로 같이 띄운다(못 찾거나 실행에 실패해도 조용히 넘어간다 —
게임 화면 자체는 panel 없이도 정상 동작한다). 둘 다
`production/director/` 에 같이 둔다:

```
cargo build --bin director --bin director_panel --release
cp target/release/director.exe production/director/PalaceOS-Director.exe
cp target/release/director_panel.exe production/director/PalaceOS-Director-Panel.exe
```

## → 콘솔 창 숨기기

`director.exe`/`director_panel.exe` 둘 다 실행할 때마다 검은 콘솔
창이 게임 창과 같이 떠서 지저분해 보였다 — Rust 는 기본적으로
Windows 에서 콘솔 서브시스템으로 빌드하기 때문이다. 두 파일 맨
위에 `#![windows_subsystem = "windows"]` 를 추가해서 GUI 서브시스템
으로 빌드되게 했다(크레이트 루트에만 붙일 수 있는 속성이라 `src/bin/`
아래 각 파일 맨 위가 정확히 그 자리다). 실제 게임(`crackhead.exe`,
`src/main.rs`)은 요청받지 않았으니 그대로 뒀다 — 필요하면 알려달라.

## → 버그: Glitch/Noise 를 껐다 켜면 대신 예전 씬 전환이 재실행됨

제보("글리치랑 노이즈를 끄고 키면 작동은 안하고 전에 했던 행동이
작동돼") 그대로 재현되는 버그였다. 원인은 `director_panel.rs` 의
`self.state.jump_to` — 씬 버튼을 한 번이라도 누르면 그 이름이 여기
남는데, `director_ipc::save()` 는 매번 `self.state` 통째로(즉 그
남아있는 옛 `jump_to` 까지 같이) 파일에 써버린다. `director.exe`
는 파일에서 읽은 뒤 자기 쪽(파일)의 `jump_to` 는 지우지만, panel
쪽 메모리에 남아있던 값은 그대로라 다음에 Glitch/Noise 버튼처럼
전혀 무관한 걸 눌러도 저장할 때마다 그 옛 씬 전환 요청이 매번 같이
다시 실려서 director.exe 가 또 그 씬으로 튕겼다(그 김에 씬이
새로 만들어지면서 Lobby 였다면 처음부터 다시 시작되니, "글리치를
안 껐다"가 아니라 "장면이 리셋됐다"로 보인 것).

`director_panel.rs` 에서 저장한 직후 `self.state.jump_to` 를 바로
`None` 으로 지워서(씬 전환은 "한 번만 실행할 명령"이니 로컬에도
안 남겨둔다), 그 뒤로는 Glitch/Noise 버튼이 더 이상 옛 씬 전환을
같이 딸려 보내지 않는다.

## → 그래도 Glitch/Noise 가 안 먹힌다는 제보 — 짚이는 데를 방어적으로 고침

`production/director/director_state.json`(실제로 저장된 상태)을
직접 열어보니 `glitch`/`noise` 가 둘 다 기본값(true) 그대로였다 —
즉 꺼짐(false)이 저장된 적 자체가 없어 보인다. 코드를 다시 감사해서
확실한 버그 하나와, 의심되지만 실행해서 확인은 못 하는 것 두 가지를
고쳤다:

1. **자동 실행 이름 불일치(확실한 버그)**: `director.exe` 가 같이
   띄우려던 파일 이름이 cargo 기본값(`director_panel.exe`)이었는데,
   실제로 `production/` 에 배포한 파일 이름은 `PalaceOS-Director-Panel.exe`
   라 못 찾아서 자동 실행 자체가 조용히 실패했을 수 있다 — 지금은
   두 이름을 다 찾아보게 고쳤다.
2. **Glitch/Noise 버튼이 창 아래쪽에 너무 빠듯하게 있었다(의심)**:
   `high_dpi: true` 로 만든 작은 창에서, 실제 클릭 가능한 영역이
   렌더러가 그리는 캔버스보다 살짝 작게 잡히는 경우가 있는데(특히
   아래쪽 요소일수록 영향이 크다), 두 버튼이 하필 그 아래쪽 끝에
   딱 붙어 있었다 — 씬 버튼 5개(위쪽)는 눌리는데 Glitch/Noise(아래
   두 개)만 유독 안 눌리는 것과 정확히 들어맞는 정황이다. 창 높이를
   260→340 으로 넉넉히 늘리고, `high_dpi` 자체를 꺼서(이 창은 CRT
   변환 없이 마우스 좌표를 그대로 쓰는 단순한 창이라 안 켜도 된다)
   애초에 이 불일치 여지를 없앴다.

**확인 부탁**: 혹시 지금 director.exe 화면이 로비(타이틀) 화면이
아니라 Desktop 등 다른 씬에 가 있는 상태에서 테스트했다면, 그것도
원인일 수 있다 — 글리치/정전기는 지금 로비 화면에서만 나오는
연출이라, 다른 씬에서는 켜고 꺼도 화면에 보이는 변화가 아예 없다.
Panel 에서 "Lobby" 버튼을 먼저 눌러 로비 화면으로 돌아간 뒤 다시
테스트해달라.

## → 실제 원인 확인 + Glitch/Noise 를 모든 씬에서 작동하게, 기본값도 OFF 로

스크린샷을 보내주셔서 정확히 확인됐다 — director.exe 화면이 Desktop
이었고, Panel 의 Noise 버튼은 실제로 OFF 로 잘 눌려 있었다(패널 레이아웃
수정이 제대로 먹혔다는 뜻이기도 하다). 즉 버튼도, 통신도 다 정상이었고,
"Desktop 화면엔애초에 글리치/정전기 연출 자체가 없다"가 진짜 원인이었다
— 로비 화면 전용 연출이라 다른 씬에서는 켜고 꺼도 티가 안 났다.

"진짜 제대로 작동하도록"이라는 요청에 맞춰, 로비 화면 안에 있던 연출을
재사용하는 대신 **director 자체가 지금 보고 있는 씬이 뭐든 상관없이
화면 위에 매 프레임 직접 정전기/글리치를 덧그리게** 바꿨다 —
`DirectorStage::draw_overlay_effects()`(`src/bin/director.rs`). 로직 자체는
`scenes/lobby.rs::draw_noise()`/`tick_glitch()` 와 같지만(정전기 알갱이 +
가끔 튀는 가로 찢김 밴드), 씬 하나에 묶여있지 않고 씬을 다 그린 뒤 그
위에 한 번 더 그리는 오버레이라 Boot/Lobby/Desktop/Erase/Shutdown 어느
화면에서도 항상 똑같이 작동한다. 로비 화면 자기 자신의 내장 연출은(이중
으로 겹쳐 나오지 않도록) director 안에서는 항상 꺼둔 채로 만든다 — 실제
게임의 로비 화면은 이 변경과 전혀 무관하게 그대로 자동으로 나온다.

**기본값도 OFF 로 바꿨다** — `director_ipc::DirectorState` 의
`glitch`/`noise` 기본값을 "평소엔 깨끗한 화면, 필요할 때만 켠다"는
연출 도구 취지에 맞게 둘 다 `false` 로 시작하게 했다(이전엔 실수로
둘 다 `true` 였다 — 처음 켜면 이미 정전기가 나오고 있어서 "이게
기본이었나?" 헷갈릴 수 있었다).

## → 로비 화면의 자동 연출은 다시 "설정 불가"로, 오버레이는 그대로

방금 만든 오버레이 방식과 로비 화면 자체의 원래 연출이 director 안에서
같이 섞여 있었던 걸 정리해달라는 요청 — 로비 화면 고유의 자동 정전기/
글리치는 다시 실제 게임과 완전히 똑같은 방식(`LobbyScene::new()`, 항상
자동, panel 로 손댈 수 없음)으로 되돌렸다. 그걸 위해 만들었던
`glitch_enabled`/`noise_enabled` 필드와 `with_director_controls()` 생성자를
`LobbyScene` 에서 통째로 뺐다 — 이제 `scenes/lobby.rs` 는 이 기능이
생기기 전과 완전히 같은 코드다.

Glitch/Noise 토글은 그대로 `DirectorStage::draw_overlay_effects()` 가
전담한다 — 씬을 다 그린 뒤 그 위에 덧그리는 완전히 별개의 레이어라
로비 화면의 자동 연출과는 서로 안 얽힌다. 그래서 지금은:
- **로비 화면**: 항상 자동으로 정전기/글리치가 나온다(설정 불가) +
  Glitch/Noise 를 켜면 그 위에 director 의 오버레이가 추가로 덧그려진다.
- **그 외 모든 씬**(Boot/Desktop/Erase/Shutdown): Glitch/Noise 를 켜야만
  나온다(그 화면엔 원래 이 연출이 아예 없으므로) — 끄면 아무것도 안
  나온다.

## Glitch/Noise 강도 조절 추가

ON/OFF 만으로는 세기가 항상 고정이라 부족하다는 요청 — panel 에
`glitch_intensity`/`noise_intensity`(`director_ipc.rs`, 0.0~1.0)를
새로 추가하고, 25% 단위 5단계(0/25/50/75/100%)로 "-"/"+" 버튼 두
개(`draw_intensity_row`)로 조절하게 했다 — 이 정도 폭에서는 슬라이더
보다 버튼 두 개가 정확히 원하는 값을 짚기 더 쉽다. 켠 채로 시작하면
예전과 같은 세기로 보이도록 기본값은 100% 다.

`DirectorStage::draw_overlay_effects()` 에서 강도를 반영하는 방식:
- **Noise**: 알갱이 개수를 강도에 비례해서 줄이고(220개 → 강도만큼),
  알갱이 밝기에도 강도를 곱해서 더 흐리게 만든다.
- **Glitch**: 찢김 밴드 개수/굵기/좌우 어긋남 폭/불투명도를 전부 강도에
  비례해서 줄인다. 다만 **터지는 빈도(타이밍)는 강도와 무관하게 그대로
  둔다** — 빈도까지 강도에 맞춰 줄이면 "약하게 켰다"가 오히려 "뜸하게
  켰다"로 헷갈릴 수 있어서, 강도는 "한 번 터질 때 얼마나 세게 보이는가"
  로만 쓴다.

`draw_intensity_row()` 는 `PanelStage` 의 메서드가 아니라 자유 함수로
뒀다 — `ui::button()` 이 요구하는 `&WinInput` 을 `self.input` 에서
빌려온 채로 `self.renderer`/`self.state` 를 또 mutably 빌리는 메서드
호출을 하면 안 되기 때문에(빌림 규칙 충돌), 필요한 마우스 값만 값으로
복사해 넘기는 자유 함수로 분리했다.

## → 강도 범위를 "0%=진짜 약하게 ~ 100%=예전의 2~3배"로 재조정

25% 단위 자체는 그대로 두고(0~100%), 그 값이 실제 세기로 변환되는
공식을 바꿨다 — `draw_overlay_effects()` 에 `STRENGTH_MAX = 3.0` 을
두고 `strength = intensity * STRENGTH_MAX` 로 계산해서 이 값을
(예전에 `intensity` 를 직접 곱하던 자리에) 그대로 대신 쓴다. 그래서:

- **0%**: strength=0 → 알갱이/밴드가 거의 없다시피 약하다(사실상
  꺼진 것과 비슷한 수준).
- **100%**: strength=3.0 → 알갱이 개수(220개 기준 최대 660개)/찢김
  밴드 개수(최대 19개)·굵기·좌우 어긋남 폭·밝기/불투명도가 전부
  예전(강도 조절이 생기기 전, 즉 "100%"가 늘 의미하던 세기)의 3배
  까지 나온다.

밝기(`v`)와 불투명도(alpha)는 이제 1.0 을 넘을 수 있어서(예: 알갱이
밝기가 최대 0.35*3=1.05) 둘 다 `.min(1.0)` 으로 잘라서 색이 범위를
벗어나지 않게 했다.

## 글리치를 참고 이미지 느낌으로 — 진짜 화면-내용을 미는 셰이더 기반으로 교체

참고 이미지(반투명 색 밴드가 아니라, 화면 내용 자체가 가로로 밀려서
찢어지고 R/G/B 가 어긋나 번지는 느낌 + 굵은 점선)를 받고, 지금까지의
"반투명 색 사각형을 위에 덧그리는" 방식으로는 이 느낌을 못 낸다는
결론을 냈다 — 색 사각형은 화면 내용을 가리기만 할 뿐, 실제로 그 밑의
글자/아이콘이 옆으로 밀려서 잘려 보이는 것과는 다른 효과다. 진짜로
화면 내용을 미는 건 "이미 그려진 화면을 다시 샘플링해서 다른 위치의
내용을 가져오는" 것이라, 셰이더 단계에서만 가능하다.

**`src/crt.rs` 를 고쳤다** — CRT 셰이더(`CRT_FS`)에 글리치 밴드
유니폼(`glitch_count`, `glitch_bands[6]` — 각 밴드는 `(y0, y1,
x_shift, chroma_boost)`, 전부 화면 UV(0..1) 기준)을 추가했다. 지금
그리는 화면 픽셀의 y 좌표가 어떤 밴드 구간에 들어가면, 그 오프스크린
텍스처를 샘플링하는 x 좌표 자체를 `x_shift` 만큼 밀고 `fract()` 로
반대쪽 가장자리에서 이어 붙게 감아서 — 그 구간만 화면 내용이 옆으로
밀려나 다른 자리 내용이 삐져나온 것처럼 보이게 한다. 그 구간 안에서는
색수차(R/G/B 채널을 살짝 다른 지점에서 샘플링하는 기존 CRT 효과)도
`chroma_boost` 만큼 더 세게 줘서 채널이 확실히 어긋나 보이게 한다
(색수차 설정을 꺼둔 상태여도 글리치 구간만은 최소한의 색번짐이
보장되도록 별도로 더한다).

**실제 게임은 전혀 안 바뀐다** — 새로 만든 `Crt::present_with_glitch()`
는 director 전용이고, 기존 `Crt::present()`(main.rs 가 그대로 쓰는
바로 그 함수)는 내부적으로 `present_with_glitch(..., &[])`(빈 밴드
목록)를 호출하도록만 바꿨다 — 셰이더 쪽도 `glitch_count` 가 0이면
루프가 첫 반복에서 바로 끝나 샘플링 좌표가 원래(`warped`)와 완전히
같아진다. `src/main.rs` 는 단 한 줄도 안 고쳤다.

**`src/bin/director.rs`**: 반투명 밴드를 직접 그리던 코드를 걷어내고,
대신 밴드 정보(y0/y1/x_shift/chroma_boost)를 계산해서
`self.pending_glitch_bands` 에 채워뒀다가 `crt.present_with_glitch()`
호출 시 그대로 넘긴다. 밴드 경계에는(참고 이미지의 굵은 점선처럼)
흰색/검은색이 번갈아 나오는 점선을 덧그려서(`draw_dashed_line`) "이
자리에서 화면이 잘렸다"는 느낌을 더 뚜렷하게 했다. Noise(정전기
알갱이)는 화면 내용과 무관한 효과라 지금처럼 그냥 반투명 사각형을
덧그리는 방식 그대로 둔다.

**주의할 점 — 이번 세션 중 가장 위험한 변경이다.** 셰이더(GLSL)
코드는 컴파일 시점이 아니라 프로그램을 실제로 켰을 때(`ctx.new_shader()`
호출 시점, `.expect("CRT 셰이더 컴파일 실패")`) 컴파일된다 — 그
시점에 GPU 드라이버가 이 셰이더를 못 받아들이면 **director 뿐 아니라
실제 게임(crackhead.exe)도 시작하자마자 패닉으로 죽는다** (CRT 는 두
쪽이 완전히 같은 셰이더 코드를 공유한다). 새로 넣은 유니폼 배열
인덱싱(`glitch_bands[i]`, `i` 가 반복문 변수)은 GLSL ES 1.00 에서
표준적으로 지원되는 패턴(반복 횟수 상한이 상수인 for 문 + 그 안에서
유니폼 값으로 break)이라 문제없을 것으로 보이지만, 실제로 셰이더를
컴파일해서 확인하는 건 이 환경에서 직접 못 한다 — **꼭 director 뿐
아니라 평소 쓰던 진짜 게임(crackhead.exe)도 한 번 켜서 정상적으로
뜨는지 확인해달라.** 혹시 둘 중 하나라도 안 뜨면 바로 알려달라, 이
변경 전체를 되돌릴 수 있다.

## 블루스크린(BSOD) 씬 추가

참고 이미지(Windows 9x 식 "A fatal exception 0E has occurred") 그대로
`src/scenes/bluescreen.rs::BlueScreenScene` 을 새로 만들었다 — 파란
배경 + 위쪽 가운데 "Windows" 라벨 상자 + 본문(고정폭 정렬) + 아래쪽
깜빡이는 "Press any key to continue" 커서. 본문 텍스트는 각본 그대로의
영어 원문이라(실제 이메일 원문을 언어와 무관하게 그대로 두는 것과
같은 이유) `tr()` 로 번역하지 않는다.

지금은 게임 진행 중 **어디서도 자동으로 안 뜬다** — 스토리 트리거가
아직 없어서, 화면 자체와 "아무 키나 누르면(또는 클릭하면) BootScene
으로 돌아간다"는 동작만 먼저 만들어뒀다(실제 Win9x 블루스크린도
결국 어떤 키를 누르든 재부팅으로 이어진다는 데서 착안). "아무 키나"
판정을 위해 `scenes/mod.rs::Input` 에 `any_key_pressed()` 를 새로
추가했다. 나중에 실제 스토리 트리거가 생기면 그 시점에 다른 씬에서
`Transition::Switch(Box::new(BlueScreenScene::new()))` 로 연결하면
된다.

director 툴에도 씬 버튼으로 추가했다 — panel 의 SCENE_BUTTONS 에
"BlueScreen" 을 넣고(그래서 패널 창 높이도 6개 버튼이 들어가게
420→450 으로 늘렸다), `director.rs::make_scene()` 에도 매칭 분기를
추가해서 미리보기/촬영이 바로 가능하다.

## 강도 조절을 슬라이더로, 글리치 발동 주기도 조절 가능하게

- **-/+ 버튼 → 드래그 슬라이더**: `draw_intensity_row`(버튼 두 개)를
  없애고 `draw_slider_row`(라벨+퍼센트 글자 한 줄 + 그 아래 드래그
  막대)로 바꿨다 — 트랙 아무 데나 누른 채로 좌우로 끌면 그 지점 값으로
  바로 따라온다(손잡이를 정확히 잡을 필요 없이 클릭 한 번으로도 그
  지점 값으로 즉시 이동). 25% 단위 제한도 없어져서 1% 단위로 원하는
  값을 정확히 짚을 수 있다.
- **글리치 발동 주기(빈도) 조절 추가**: `director_ipc::DirectorState`
  에 `glitch_frequency`(0.0=뜸하게~1.0=잦게, 기본 0.5) 를 새로 추가하고,
  `director.rs::glitch_gap_range()` 가 이 값을 실제 "다음 글리치까지
  대기 시간" 범위(초)로 변환한다 — 0% 는 한참(4~8초) 기다렸다 한 번씩,
  100% 는 거의 쉴 새 없이(0.25~0.9초 간격) 터진다. 강도(Intensity)와는
  별개 슬라이더로 뒀다 — 강도는 "한 번 터질 때 얼마나 세게", 주기는
  "얼마나 자주"라 서로 다른 축이다. 정전기(Noise)는 매 프레임 계속
  그리는 효과라 "주기" 개념 자체가 없어서 주기 슬라이더는 글리치에만
  있다.

패널에 슬라이더 두 줄(Glitch Intensity/Frequency)이 늘어난 만큼 창
높이도 450→470 으로 늘렸다.

## 글리치를 "쫙 찢어지는" 느낌으로, 주기 100% = 상시 작동

**밴드를 매 프레임 다시 뽑지 않고 버스트 한 번(0.14초) 동안 고정한다**
— 예전엔 활성 상태인 동안 매 프레임 밴드를 새로 뽑아서, 짧은 시간
안에 밴드가 계속 어른거려 "화면이 찢어졌다"보다는 "지지직거린다"에
더 가까워 보였다. `overlay_glitch_bands` 필드를 새로 둬서, 버스트가
시작되는 순간(`roll_glitch_bands()`) 딱 한 번만 뽑고 그 버스트가
끝날 때까지 그대로 유지한다 — 한 번 쫙 찢어진 스냅샷처럼 또렷하게
보인다.

**밴드를 더 크고 적게, 대신 확실하게 어긋나도록 바꿨다** — 예전엔
밴드 개수가 최대 19개까지 늘어나면서 잘게 쪼개진 조각들이 화면을
덮었는데, 참고 이미지는 그보다 훨씬 큼직하고 뚜렷한 몇 개의 찢김에
가까웠다. `roll_glitch_bands()` 를 밴드 개수는 적게(최대 6개), 밴드
하나의 두께(28~90px, 강도에 비례해 더 두꺼워짐)와 좌우 어긋남 폭
(28~90px, 강도에 비례해 최대 270px 까지)은 크게 잡도록 다시 짰다.
어긋나는 방향도 자잘하게 흔들리지 않고 각 밴드마다 왼쪽/오른쪽
한 방향으로만 크게 밀어서, 참고 이미지처럼 "그 자리에서 옆으로 쫙
밀려난" 느낌이 나게 했다.

**Frequency 를 100% 로 두면 상시 작동한다** — 예전엔 100% 여도 여전히
0.25~0.9초의 대기 간격이 있었는데, `glitch_frequency >= 0.999` 일 때는
버스트가 끝나자마자(대기 없이) 바로 다음 버스트를 시작하도록 특수
처리했다 — 실질적으로 화면이 계속 찢어진 상태로 유지된다(정확히는
0.14초 버스트가 끊김 없이 계속 이어지는 것이라, 밴드 모양 자체는
버스트마다 새로 뽑혀 조금씩 바뀐다).

## 글리치를 "몇 개의 큰 사각 밴드" 대신 "절차적 가로 줄무늬"로 다시 설계

"줄무늬 같이 생기는 걸 좀 더 레퍼런스를 찾아보고 시각적인 개선을
해달라"는 요청을 받고, 실제 글리치 아트/셰이더 튜토리얼들을 검색해서
찾아봤다 — datamoshing/VHS 글리치 셰이더들이 공통으로 쓰는 기법은
"화면을 아주 가는 가로줄 여러 개로 쪼갠 뒤, 줄마다 해시 노이즈로
독립적으로 옆으로 어긋나 있는지/얼마나 어긋나 있는지를 정하는" 절차적
줄무늬 변위(stripe displacement)였다 — 지금까지 만들었던 "손으로 뽑은
큼직한 사각 밴드 몇 개 + 그 경계에 굵은 점선"보다 훨씬 화면 신호가
실제로 깨진 것처럼 보이고, 이게 바로 참고 이미지들에서 보이는
"줄무늬" 텍스처의 정체였다.

**`src/crt.rs` 를 다시 고쳤다** — `glitch_count`/`glitch_bands[6]`
배열 유니폼을 걷어내고, 대신 `glitch_on`/`glitch_seed`/`glitch_freq`
(줄 개수)/`glitch_fill`(그중 어긋난 줄의 비율)/`glitch_shift`(어긋남
폭)/`glitch_chroma`(색번짐 보정) 스칼라 유니폼 6개로 바꿨다. 셰이더
안에서 `ghash()`(sin 기반 해시 의사난수)로 각 가로줄(`floor(y *
glitch_freq)`)마다 "이 줄이 어긋나 있는지"(`glitch_fill` 확률)와
"얼마나/어느 방향으로"를 독립적으로 계산한다 — 줄 하나하나가 전부
제각각이라 앞서 만든 "몇 개의 균일한 사각 밴드"보다 훨씬 자연스럽고
빽빽한 줄무늬 텍스처가 나온다. `Crt::present_with_glitch()` 의
시그니처도 `&[[f32;4]]`(밴드 배열) 대신 `Option<GlitchParams>`(seed/
freq/fill/shift/chroma 다섯 필드짜리 구조체) 하나로 훨씬 단순해졌다.

`src/bin/director.rs` 의 밴드 굴리기/점선 그리기 코드(`roll_glitch_bands`,
`draw_dashed_line`)는 이제 다 필요 없어져서 지웠다 — 버스트가 시작될
때 `seed` 하나만 새로 뽑아서 그대로 유지하고(예전과 같은 이유 — 매
프레임 다시 뽑으면 패턴이 너무 빨리 어른거려 "찢어짐"보다
"지지직거림"에 가까워 보인다), freq/fill/shift/chroma 는 강도
슬라이더 값에서 매 프레임 그대로 계산한다.

**이번에도 CRT 셰이더(`crt.rs`)를 다시 고쳤다는 뜻이라, 이전과 같은
위험이 그대로 있다** — 셰이더는 프로그램을 실제로 켰을 때 컴파일되고,
실제 게임(`crackhead.exe`)이 director 와 완전히 같은 셰이더 코드를
공유한다. 이번 변경은 오히려 유니폼 구조가 배열에서 평범한 스칼라
6개로 단순해져서 이전 버전보다 컴파일 위험은 더 낮다고 보지만, **역시
실제로 컴파일해서 확인하는 건 못 한다 — 꼭 게임도 한 번 켜서 정상적으로
뜨는지 확인해달라.**

## → 실제로 사고 발생: 실행하면 바로 꺼짐 — CRT 셰이더 긴급 원복

우려했던 그대로였다("실행하면 바로 꺼지는데??" — director 뿐 아니라
실제 게임 crackhead.exe 도 같이). 정확히 어느 GLSL 구문이 문제였는지는
(컴파일 에러 로그를 직접 볼 방법이 없어서) 못 밝혔지만, 원인을 더
파고드는 대신 **`src/crt.rs` 를 이번 세션에서 글리치 기능을 만들기
전 상태로 완전히 되돌렸다** — `CrtUniform`/`CRT_FS`/`present()` 전부
이 세션 훨씬 전부터 실제로 잘 작동하던 원래 코드 그대로다. 지금
`crackhead.exe`(실제 게임)는 이 세션 시작 시점과 완전히 동일한
렌더링 코드를 쓴다.

이유: 셰이더가 실행 시점에야 컴파일되는 코드라 나로서는 고친 뒤
"이번엔 될 것 같다"는 추측만 할 수 있었는데, 그 추측이 두 번 다
틀렸다 — 그런데 이 코드는 director 만의 것이 아니라 **실제 게임과
공유하는 코드**라, 잘못되면 실제 게임까지 못 켜지는 상황을 계속
반복할 순 없었다. 그래서 이번엔 "고쳐서 다시 시도"가 아니라 "아예
공유 지점 자체를 건드리지 않는 방식으로 후퇴"를 택했다.

`src/bin/director.rs` 의 Glitch 효과는 지금 **셰이더 없이 순수
사각형만으로** 다시 만들었다 — 화면을 2px 짜리 가는 줄 수십 개로
스캔하면서 줄마다 독립적으로 나타날지/얼마나 진하게 보일지를 정하고,
옅은 시안색 반투명 줄로 겹쳐서 색번짐 힌트만 준다. **화면 내용
자체가 옆으로 밀려 찢어져 보이는 진짜 효과는 이제 안 나온다** —
그러려면 셰이더가 필요한데, 셰이더는 이제 실제 게임과 공유하지 않는
방향으로 확실히 굳혔기 때문이다. 참고 이미지 수준의 "화면-내용이
실제로 찢어지는" 효과를 원한다면, 다음 두 방법 중 하나가 필요하다:
1. `crt.rs` 를 다시 고치되, 이번엔 아주 작은 변경 하나씩 나눠서 매번
   실제로 켜서 확인해가며(직접 GPU 에서 셰이더가 컴파일되는지 눈으로
   보면서) 진행하는 방법 — 느리지만 확실하다.
2. Director 전용의 완전히 별도인 셰이더/파이프라인을 새로 만들어서,
   crt.rs 자체(그리고 실제 게임)는 절대 안 건드리는 방법 — 손은 더
   가지만 실제 게임 쪽 위험은 원천적으로 없앨 수 있다.

지금은 우선 안전한 상태로 되돌리는 것 자체가 급해서 둘 다 진행하지
않았다 — 어느 쪽으로 갈지는 실제 게임이 다시 잘 뜨는 걸 확인해준
뒤에 상의해서 정하고 싶다.

## → 화면-내용 글리치 복구: 이번엔 진짜로 완전히 분리된 셰이더로

위에서 말한 "2번(director 전용의 완전히 별도인 셰이더)"으로 다시
만들었다 — "옆으로 찢어지는 느낌이 사라졌다"는 제보를 받고, 이번엔
`crt.rs`(실제 게임과 공유하는 파일)는 손끝 하나 안 대고, 그 안의
`Crt` 구조체/셰이더를 통째로 복사해서 `src/bin/director.rs` **파일
안에만** `DirectorCrt` 라는 이름으로 새로 만들었다 — 구조체, 파이프라인,
셰이더 소스(`DIRECTOR_CRT_VS`/`DIRECTOR_CRT_FS`) 전부 이 파일 안에서
끝난다. `crackhead`(실제 게임)를 빌드하는 코드는 이 새 코드가 존재하는지
조차 알 방법이 없다(별도 실행 파일이라 컴파일 자체가 안 묶인다) —
그래서 이번에 또 셰이더가 컴파일 실패하더라도, 그 즉시 실제 게임이
아니라 director.exe 하나만 영향을 받는다.

셰이더 내용 자체도 지난번보다 더 보수적으로 다시 짰다 — 조건부
연산자(`? :`)를 GLSL 내장 함수인 `step()`/`mix()` 조합으로 바꿔서
분기 없이 계산하게 했다(정확한 크래시 원인은 못 밝혔지만, 혹시
삼항 연산자 관련 문제였을 가능성도 배제하려고 더 안전한 관용구를
썼다). 로직 자체(화면을 가로줄로 쪼개서 줄마다 해시로 밀지 말지/
얼마나 밀지 정하는 절차적 줄무늬)는 지난번과 같다.

이번에도 실행해서 직접 컴파일을 확인할 순 없지만, **이번엔 최악의
경우에도 director.exe 만 안 뜨고 실제 게임은 100% 안전하다** — 확인
부탁한다.

## → 그런데도 director 만 여전히 안 뜸 — 추측 대신 실제 로그를 남기게 함

"director만 작동 안해..." 제보 그대로였다. 세 번 연속 "이번엔 될 것
같다"고 추측만 하다 틀린 셈이라, 이번엔 추측을 그만두고 **실제
컴파일러 에러 메시지를 직접 볼 수 있게** 코드를 바꿨다.

`DirectorCrt::new()` 가 이제 글리치 셰이더 컴파일을 `.expect()` 로
바로 패닉내는 대신 결과를 확인한다 — 실패하면:
1. 진짜 에러 메시지(`ShaderError` 의 `Display` — GPU 드라이버가 준
   컴파일러 로그 그대로)를 exe 옆의 `director_shader_error.log` 파일에
   그대로 적어둔다.
2. 글리치 없이 원래 CRT 효과만 있는 **기본 셰이더**(`DIRECTOR_CRT_FS_SAFE`
   — src/crt.rs 의 이미 여러 번 실제로 잘 작동한 걸로 확인된 원본 코드와
   완전히 같다)로 대신 컴파일해서 계속 진행한다.

그래서 이제는 글리치 셰이더가 또 실패해도 director.exe 자체는 뜬다
(대신 그 실행에서는 Glitch 를 켜도 화면에 아무 효과가 없다 — 화면은
평소 CRT 그대로 나오고, `director_shader_error.log` 파일만 새로
생긴다). 그 로그 파일 내용을 보내주면 이번엔 추측이 아니라 실제
원인을 보고 정확히 고칠 수 있다.

`present()` 도 두 경우(글리치 유니폼 10개 / 기본 유니폼 4개)에 맞는
유니폼 구조체를 각각 보내도록 나눴다 — 셰이더가 기대하는 유니폼
개수와 실제로 보내는 값의 개수가 안 맞으면 그 자체로 또 다른 문제가
생길 수 있어서, 실패 시엔 확실히 4개짜리로만 보낸다.

## → 드디어 진짜 원인 찾음: `active` 는 GLSL 예약어였다

로그 파일 덕분에 세 번의 실패가 전부 같은 이유였다는 게 바로
드러났다:

```
Fragment shader error:
ERROR: 0:35: 'active' : Reserved word.
ERROR: 0:35: '' : compilation terminated
ERROR: 2 compilation errors.  No code generated.
```

글리치 로직 안에서 `float active = ghash(...)` 라는 변수를 썼는데,
`active` 가 GLSL 의 예약어였다(향후 키워드로 쓰려고 언어 스펙에 미리
막아둔 이름 — 문법적으로는 멀쩡해 보이지만 컴파일러가 그냥 거부한다).
이 변수명은 첫 시도(밴드 배열 버전)부터 세 번 내내 그대로 복사해서
썼던 거라, 지금까지의 모든 실패가 사실 **딱 이 한 줄** 때문이었다 —
로직 자체(절차적 줄무늬 변위)는 처음부터 문제가 없었다.

`director.rs` 의 `DIRECTOR_CRT_FS` 에서 그 변수 이름을 `roll` 로
바꿨다. 이제 컴파일이 되니 `glitch_supported` 는 true 가 되고, 지난
메시지에서 만든 안전한 대체 셰이더(`DIRECTOR_CRT_FS_SAFE`)로
넘어가는 경로는 안 타게 된다 — 진짜 화면-내용 찢김 글리치가 다시
나온다.

여러 번 헤맨 근본 원인은 GLSL 컴파일러 로그를 직접 볼 방법이 없어서
매번 코드를 다시 짜며 추측만 했던 것 — 지난 메시지에서 실패 시
`director_shader_error.log` 를 남기도록 미리 바꿔둔 게 이번에 바로
효과를 봤다.

## 글리치를 더 뭉탱이지게, 세로로도 살짝 밀리게

찢김 효과가 이제 실제로 나오긴 하는데 줄무늬가 너무 얇고(가로 줄이
50~140개나 돼서 3~10px 짜리), 가로로만 밀릴 뿐 세로 흔들림이 전혀
없었다. 두 가지를 손봤다 — 전부 `director.rs` 안에서만, 여전히
`crt.rs`(진짜 게임)는 손대지 않는다:

- **뭉탱이지게**: `draw_overlay_effects()` 에서 밴드 개수를 정하는
  `glitch_freq` 공식을 `50.0 + gi*30.0`(50~140줄) 에서
  `8.0 + gi*14.0`(8~22줄) 로 줄였다 — 줄 수가 줄면 한 줄의 세로
  두께가 그만큼 굵어져서, 참고 이미지처럼 큼직큼직하게 찢어지는
  느낌이 난다.
- **세로로도 살짝 밀리게**: `GlitchParams`/`DirectorCrtUniform` 에
  `vshift` 필드를 새로 추가하고, `DIRECTOR_CRT_FS` 셰이더 안에서
  가로 변위(`shiftAmt`)를 계산하던 바로 옆에 세로 변위도 계산해
  `sampleUv.y` 에 더한다. 가로 변위와 같은 해시(`row`, `glitch_seed`)
  를 쓰되, 다른 매직넘버(`91.345`)로 오프셋을 줘서 가로/세로 방향이
  서로 상관없이 따로 흔들리게 했다. 가로는 화면을 넘어가면
  `fract()` 로 반대편에서 다시 나오게(래핑) 하지만, 세로는
  `clamp()` 로 막아뒀다 — 위아래로 wrap 시키면 베젤 안쪽의 검은
  여백을 끌어와 이상하게 잘려 보이기 때문에, 살짝 밀리다가
  가장자리에서 멈추는 쪽이 더 자연스럽다.

## Director 는 이제 테스트/제작용 작업 공간 — 창 크기도 키움

Director 는 더 이상 단순 미리보기 툴이 아니라, 실제 게임(`crackhead`)
을 직접 건드리기 전에 새 기능을 먼저 만들고 검증하는 작업 공간으로도
쓰기로 했다 — 이미 셰이더/씬 코드가 `src/bin/director.rs` 안에 완전히
격리돼 있어서(공유하는 건 `lib.rs` 의 씬/렌더러 코드뿐, CRT 셰이더도
따로 있음) 이 방향과 잘 맞는다. 앞으로 새 기능은 원본 게임을 바로
건드리지 않고 Director 쪽에서 먼저 다뤄본다.

작업 화면으로 쓰기엔 창이 좀 작아서(miniquad 기본값 800x600) 조금
키웠다 — `director.rs` main() 의 `conf::Conf` 에 `window_width: 960,
window_height: 720` 을 명시(4:3 비율은 그대로 유지, 필러박스 계산과
안 어긋나게).

## Director 에 녹화 기능 추가

Director 가 이제 제작용 작업 공간으로도 쓰이니, 원하는 장면을 화면
그대로(CRT 곡률/스캔라인/글리치 다 입혀진 최종 화면) PNG 프레임
시퀀스로 저장하는 기능을 넣었다. 외부 인코더(ffmpeg 등) 없이 순수
Rust 로 구현했고, 이미 의존성에 있던 `image` 크레이트(PNG 인코딩용)
를 그대로 재사용했다 — 새 의존성 추가 없음.

**어떻게 켜고 끄나**: panel(`PalaceOS-Director-Panel.exe`) 맨 아래에
`● Record` 버튼이 새로 생겼다. 누르면 `■ Stop Recording` 으로 바뀌고
그때부터 매 프레임을 저장한다. 다른 Glitch/Noise/씬전환 버튼과 똑같이
`director_ipc::DirectorState` 에 `recording: bool` 필드 하나로
공유한다.

**저장 위치**: director.exe 옆에 `recordings/rec_<유닉스초>_<밀리초>/`
폴더를 새로 만들고 그 안에 `frame_000000.png`, `frame_000001.png`, ...
순서로 저장한다. Record 버튼을 껐다 다시 켜면 그때마다 새 타임스탬프
폴더가 생겨서 이전 촬영분을 덮어쓰지 않는다.

**구현 방식 — 왜 화면과 별개로 한 번 더 그리나**: 실제 창(필러박스
때문에 화면 크기에 따라 여백/크기가 계속 바뀜)을 그대로 캡처하면
녹화 결과물 해상도가 창 크기에 따라 들쭉날쭉해진다. 그래서
`DirectorCrt` 에 화면용 `pass`/`color_tex` 와는 별개로 고정
1280x960(4:3) 크기의 `record_pass`/`record_tex` 오프스크린 렌더
타깃을 하나 더 만들어뒀다 — 녹화 중엔 매 프레임 똑같은 CRT 셰이더로
그 텍스처에도 한 번 더 그리고(`present_to_record()`), miniquad 의
`texture_read_pixels()` 로 그 텍스처의 픽셀을 CPU 로 읽어와서
(`capture_record_pixels()`) PNG 로 저장한다. `present()`/
`present_to_record()` 둘 다 실제로는 같은 `draw_pass()` 헬퍼를
공유한다(타깃 렌더패스와 뷰포트만 다름) — 예전엔 `present()` 하나에
로직이 다 들어 있던 걸 이번에 이렇게 분리했다.

**세로 뒤집힘 처리**: OpenGL 은 텍스처/프레임버퍼 원점이 왼쪽
아래라, `texture_read_pixels()` 로 받은 픽셀의 첫 줄이 이미지의
맨 아래 줄이다. 그대로 PNG 로 저장하면 상하가 뒤집혀 나오므로,
`save_frame_png()` 에서 행(row) 순서를 뒤집은 뒤에 저장한다.

**함수 인자가 너무 많아지는 문제**: `present()` 에 화면용/녹화용
분기까지 얹으니 인자가 8개까지 늘어나 clippy 의
`too_many_arguments` 경고가 새로 떴다(지금까지 지켜온 "경고 5개
고정" 기준에 안 맞음). `time`/`ca_amount`/`intensity`/`glitch` 를
`PresentParams` 라는 작은 구조체 하나로 묶어서 인자 개수를 줄이고
경고를 없앴다 — 의미상 특별한 그룹핑은 아니고 순수하게 인자 개수를
줄이기 위한 것.

이번에도 실제 게임(`crackhead.exe`)과 공유 `crt.rs`/`lib.rs` 는 전혀
손대지 않았다 — `DirectorCrt`/녹화 로직 전부 `src/bin/director.rs`
안에만 있고, `director_ipc.rs` 에 필드 하나(`recording`) 추가한 게
lib 쪽 변경의 전부다(director_panel.exe 도 이 모듈을 통해서만
데이터를 주고받으므로 안전).

## Record 에 소리도 같이 — WASAPI 루프백 캡처

Record 버튼을 켜면 화면(PNG 프레임)뿐 아니라 그 순간 스피커로 나가고
있는 소리도 같이 녹음해서 같은 폴더에 `audio.wav` 로 저장한다.
마이크를 쓰는 게 아니라 "지금 출력 장치로 나가는 소리를 그대로
가로채는" WASAPI 루프백 캡처 방식이다 — 이미 영상 재생(video.rs)에
WASAPI 를 쓰고 있어서 같은 크레이트/초기화 패턴을 그대로 재사용했다.
재생 스레드가 `IAudioRenderClient` 로 소리를 밀어 넣는 것과 반대로,
여기선 `IAudioCaptureClient` 로 받아서 그대로 파일에 흘려 쓴다는
점만 다르다. Record 버튼을 끄면 캡처 스레드가 멈추면서 wav 헤더의
데이터 길이를 실제로 받은 만큼으로 되돌아가 고쳐 쓴다(wav 저장 로직
자체는 director_ipc 의 지난 패턴과 동일 — 처음엔 0으로 써두고 끝나면
패치).

## → PNG + wav 를 하나의 재생 가능한 영상(output.avi)으로 자동 합치기

"이미지도 묶어서 영상으로 재생 가능하면 좋겠다"는 요청으로, Record
를 끄는 순간 이미 저장된 `frame_NNNNNN.png` 들과 `audio.wav` 를
합쳐서 같은 폴더에 `output.avi` 를 자동으로 만들어준다.

**왜 mp4/H.264 가 아니라 옛날 AVI 인가**: mp4 로 만들려면 Media
Foundation 의 H.264 인코더 MFT 를 직접 붙여야 하는데(인코더 협상,
샘플 타임스탬프, 키프레임 주기 등 손댈 게 많고 실패하면 원인 파악도
까다롭다 — 이번 세션 초반 CRT 셰이더 컴파일 실패로 director 가 아예
안 뜨던 사고를 겪은 뒤라 더 조심스러웠다), 대신 압축을 아예 안 하는
BI_RGB(비압축 24비트 BGR) 프레임을 그대로 담는 옛날식 AVI(Video for
Windows) 컨테이너를 순수 Rust 로 직접 조립했다. 인코더가 없으니
"셰이더가 컴파일이 안 된다"류의 실패 지점 자체가 없고, 어떤
플레이어에서도 별도 코덱 설치 없이 바로 재생된다 — 대신 압축이
없어서 파일이 크다(1280x960, 30fps, 10초 ≈ 1GB 안팎). 연출 확인용
도구라 "가볍고 안전하게 재생 가능"이 "작고 빠르게"보다 중요하다고
판단했다.

**구현 요점**(`src/bin/director.rs::mux_avi`):
- 프레임 크기가 고정(`RECORD_W x RECORD_H`)이라 파일 전체 크기를 쓰기
  전에 정확히 계산할 수 있다 — wav 헤더처럼 "0으로 써두고 나중에
  되돌아가 고치는" 대신, 처음부터 정확한 크기로 한 번에 순서대로
  쓴다(중간에 seek 가 전혀 없음).
- PNG 를 다시 디코드해 Windows DIB 가 기대하는 BGR24 + 4바이트 행
  정렬로 변환한다(`rgba_to_bgr24_dib`). `biHeight` 를 음수로 써서
  "위에서 아래로" 순서를 그대로 쓰게 해, PNG 저장 때 이미 한 번
  뒤집어둔 순서를 또 뒤집을 필요가 없게 했다.
  - 혹시 프레임 하나가 손상돼 디코드에 실패해도 그 자리를 그냥
    건너뛰지 않고 검은 프레임(0으로 채움)으로 채운다 — 그래야 미리
    계산해둔 전체 파일 크기/인덱스 오프셋이 프레임 개수와 어긋나지
    않는다.
- `audio.wav` 는 자체 제작한 파일이라 포맷을 안다는 걸 알면서도,
  나중에 wav 저장 방식이 바뀌어도 안 깨지게 `read_wav()` 에서
  RIFF 청크를 하나씩 걸어가며 `fmt `/`data` 를 찾는 방식으로 읽는다.
- 오디오/비디오 각 청크의 위치를 `idx1` 인덱스 청크에 기록해야
  플레이어가 곧바로 탐색(seek)할 수 있다 — movi 리스트를 쓰면서 각
  청크의 상대 오프셋을 같이 누적해뒀다가 마지막에 한 번에 쓴다.
- PNG 디코드 + 큰 파일 쓰기라 시간이 좀 걸릴 수 있어서, Record 버튼을
  끈 그 프레임에 바로 처리하지 않고 별도 스레드에서 돌린다(director
  화면이 멎지 않게). 오디오 캡처 스레드를 먼저 join 해서 wav 파일이
  완전히 닫힌 뒤에 합치기를 시작한다.

## → 버그: 파란 화면(BSOD)을 녹화했더니 output.avi 가 초록색으로 나옴

24bpp(BGR24, 행마다 4바이트 정렬) 로 만들었던 게 문제였다 — 비압축
24bpp RGB 는 실무에서 잘 안 쓰이는 조합이라 Windows 쪽 AVI 파서가
픽셀 포맷을 잘못 짐작해서 색이 완전히 틀어져 나온 것으로 보인다.
32bpp(BGRA) 로 바꿨다 — 비압축 RGB 로는 사실상 표준처럼 널리
지원되는 포맷이고, 폭×4 는 항상 4의 배수라 24bpp 에서 신경 써야 했던
행(row) 정렬 계산도 아예 필요 없어져서 코드도 더 단순해졌다
(`rgba_to_bgr24_dib` → `rgba_to_bgra32`, `biBitCount` 24→32).

## 녹음이 시스템 전체가 아니라 director 프로그램 소리만 잡히도록

지금까지는 "지금 스피커로 나가는 소리를 전부" 받는 WASAPI 엔드포인트
루프백이라, 옆에서 다른 소리(알림음, 다른 앱 소리 등)가 나면 그것도
같이 녹음됐다. Windows 10 2004+ 부터 있는 **프로세스별 루프백 캡처**
로 바꿔서, director.exe(와 자식 프로세스) 가 내는 소리만 받도록
했다 — 진짜 게임(crackhead.exe)의 사운드는 여전히 안 건드린다(이
기능은 director.exe 자기 자신을 대상 PID 로 지정해서 잡는 것이라
애초에 다른 프로세스와는 무관).

**왜 복잡한가**: 이건 실제 오디오 장치가 아니라 "가상 장치"라, 여느
WASAPI 코드처럼 `IMMDeviceEnumerator` 로 장치를 열 수 없다. 대신
`ActivateAudioInterfaceAsync()` 라는 비동기 API 로 "이 PID(와 그
자식들)가 내는 소리만" 이라는 파라미터(`AUDIOCLIENT_ACTIVATION_PARAMS`)
를 넘겨 활성화해야 하고, 그 결과를 받으려면 콜백 인터페이스
(`IActivateAudioInterfaceCompletionHandler`) 를 직접 구현해서 넘겨야
한다 — Rust 로는 `windows` 크레이트의 `#[implement]` 매크로로
COM 인터페이스를 직접 구현하는 방식이다(이 프로젝트에서 커스텀 COM
인터페이스를 구현한 첫 사례). 콜백은 별도 스레드에서 불릴 수 있어
`mpsc` 채널로 결과(`IAudioClient`)를 캡처 스레드로 돌려받는다.

파라미터 전달 방식도 특이하다 — `ActivateAudioInterfaceAsync` 는 그
파라미터를 `PROPVARIANT`(그것도 `VT_BLOB` 타입) 에 담아서 넘기라고
요구하는데, `windows-core` 의 안전한 `PROPVARIANT` 타입엔 blob 을
만드는 생성자가 없다. 대신 그 타입이 공개해둔 raw union 필드
(`windows::core::imp::PROPVARIANT`) 를 문서화된 레이아웃 그대로 직접
채워서 만들었다 — 이름 없는(anonymous) 내부 필드에 값을 대입하는 것
뿐이라 위험한 트랜스뮤트(transmute) 는 아니지만, 꼭 필요한 만큼만
`unsafe` 로 다뤘다.

**안전장치**: 프로세스 루프백 활성화가 실패하면(오래된 Windows 버전
등) 예전처럼 시스템 전체 소리를 받는 엔드포인트 루프백으로 자동
대체한다 — "녹음이 아예 안 되는 것"보다 "그래도 뭔가는 녹음되는
것"이 낫다고 판단했다. 이번에도 `audio_capture_loop` 자체가
`director.rs` 안에서만 도는 별도 스레드라, 여기서 무슨 일이 있어도
director 화면이나 진짜 게임에는 영향이 없다.

**Cargo.toml 변경**: `windows` 크레이트에 `implement` 피처를 켰고
(COM 인터페이스를 직접 구현하려면 필요), `#[implement]` 매크로가
생성하는 코드가 `windows_core::` 경로를 직접 참조해서 `windows-core`
를 별도 의존성으로도 추가해야 했다(`windows::core` 간접 재노출만으론
매크로 생성 코드가 못 찾는다 — windows-rs 에 알려진 요구사항).

## → 사고: director 창이 몇 초 뒤에 저절로 꺼지고 패널도 안 먹힘 — 프로세스 루프백 원복

"도구의 컨트롤이 안되고, 게임창은 몇초뒤에 스스로 꺼져" 라는 제보를
받고 확인해보니 실제로 director.exe 가 실행 몇 초 뒤에 프로세스 자체가
죽어 있었다(패널은 살아있었지만 상대가 없어져 아무 반응이 없었던 것).
원인은 두 가지가 겹쳐 있었다.

1. **재발 트리거**: 이전 세션에서 director 를 강제 종료하는 바람에
   `director_state.json` 에 `recording: true` 가 그대로 남아있었다.
   director 는 켤 때 자기 상태를 `false` 로 시작하니, 다음 실행 때
   `apply_director_state()` 가 "방금 사용자가 Record 를 눌렀다" 로
   착각해서 아무도 안 눌렀는데 매번 저절로 녹화(+ 오디오 캡처)를
   다시 시작해버렸다 — 실제로 확인해보니 실행할 때마다
   `recordings/rec_.../` 폴더가 새로 계속 쌓이고 있었다.
2. **진짜 크래시 원인으로 추정**: 그렇게 매번 자동으로 켜지던 오디오
   캡처가 바로 지난 변경에서 넣은 "프로세스별 루프백"
   (`ActivateAudioInterfaceAsync` + 직접 구현한 COM 콜백
   `IActivateAudioInterfaceCompletionHandler`) 경로였다. 이 콜백은
   OS 오디오 서비스가 별도 스레드에서 비동기로 불러주는 구조라,
   Rust 에서 `#[implement]` 매크로로 완전히 새로 만든 COM 객체의
   수명/스레드 안전성을 이 프로젝트에서 직접 검증할 방법이 마땅치
   않았다 — 이번 세션에선 실제로 소리를 내며 눌러볼 수 있는 환경이
   없어 컴파일만 확인하고 넘겼던 게 화근이었다.

이 조합(자동 재시작 + 검증 안 된 COM 콜백)이 겹치면서 크래시가 났을
가능성이 높다고 보고, **프로세스별 루프백 기능 자체를 완전히
되돌렸다** — `LoopbackActivateHandler`/`activate_process_loopback_client`
와 관련 `Cargo.toml` 변경(`implement` 피처, `windows-core` 의존성)을
전부 제거하고, 예전에 이미 안정적으로 동작을 확인했던 "시스템 전체
소리를 받는 엔드포인트 루프백" 으로 완전히 복귀했다. 커스텀 COM
콜백처럼 실제 실행 환경에서 검증할 수 없는 코드는, 컴파일이 되더라도
당장 쓰지 않는 게 낫다는 판단이다.

재발 방지로 `DirectorStage::new()` 시작 시 `director_state.json` 의
`recording` 이 `true` 로 남아있으면 무조건 한 번 `false` 로 되돌려
저장해두는 코드도 같이 추가했다 — 이제 director 가 녹화 도중
비정상 종료돼도, 다음 실행 때 저절로 녹화가 다시 시작되는 일은 없다
(이 재발-방지 코드는 프로세스 루프백과 무관하게 계속 남겨뒀다).

## → 버그: 10초 녹화했는데 output.avi 가 1초짜리로 나옴

원인은 AVI 에 적어넣는 fps 를 "설정의 목표 fps"(기본 60) 로
그대로 썼던 것 — 그런데 실제로 한 프레임을 녹화하는 데는
`present_to_record()`(셰이더 한 번 더 그리기) + `texture_read_pixels()`
(GPU→CPU 픽셀 읽기, 동기 호출이라 그 자리에서 기다림) + PNG 인코딩이
전부 매 프레임 순서대로 들어가서, 목표 fps 만큼 프레임을 못 채우고
실제로는 훨씬 느리게 저장되고 있었다. 즉 10초 동안 실제로는 30장
정도만 찍혔는데, "60fps 로 재생해라" 라고 헤더에 적어버리니
30장/60fps = 0.5초짜리 영상이 나온 것.

목표 fps 대신, 녹화 시작~끝까지 실제 걸린 시간과 실제로 저장된
프레임 수로 **그 자리에서 직접 계산한 fps** 를 쓰도록 고쳤다
(`record_frame / (date::now() - record_start_time)`) — 이러면 한
프레임 저장이 아무리 느려도(심지어 도중에 느려지거나 빨라져도) 항상
"실제로 녹화한 시간 그대로" 재생되는 영상이 나온다. 프레임 저장
자체를 더 빠르게 만드는 건 아니라서(그러려면 PNG 인코딩/픽셀 읽기를
비동기·병렬화해야 하는데 지금은 그렇게까지는 안 함) 결과 영상의
초당 프레임 수 자체는 낮게 나올 수 있지만(뚝뚝 끊기는 느낌), 최소한
"재생 길이가 실제로 녹화한 길이와 다르다"는 문제는 사라진다.

## → 진짜 원인: PNG 저장이 메인 스레드를 막아서 앞부분 1초만 찍히고 있었다

fps 계산을 고쳤는데도 "10초 녹화했는데 진짜 앞부분 1초만 영상으로
나온다" 는 제보가 다시 왔다. 재현해보니 실제로 문제였다 — 원인은 fps
숫자가 아니라, **한 프레임을 녹화할 때마다 하는 일**(추가 렌더패스 →
`texture_read_pixels` 로 GPU→CPU 픽셀 읽기(동기, 파이프라인 전체를
멈춰 세움) → PNG 인코딩 → 디스크 쓰기) 을 전부 `draw()` 가 도는
**메인 스레드에서 그대로** 했던 것. 1280x960 이미지, 그것도 글리치/
스캔라인처럼 고주파 노이즈가 많아 압축이 잘 안 되는 화면을 매번
인코딩하다 보니 프레임 하나 저장하는 데 시간이 걸렸고, 그동안
`draw()` 자체가 멈춰서 게임도 같이 멎어 있었다. 그래서 "10초"라고
버튼을 누르고 있던 실제 시간 동안, 실제로 처리(=저장)된 프레임은
초반 1초 안팎에 몰려 있는 몇 장뿐이었다 — 나머지 9초는 그 몇 안 되는
프레임을 저장하느라 멈춰있었던 셈.

지난번 "실측 fps" 수정은 이 증상을 감추기만 했다 — 프레임이 적으면
적은 대로 fps 를 낮게 계산해서 재생 시간 자체는 맞춰줬지만, 그 적은
프레임 수 자체(=녹화가 사실상 멈춰있던 문제)는 그대로였다.

**진짜 수정**: 픽셀을 GPU 에서 읽어오는 것(`texture_read_pixels`)
까지는 메인 스레드에서 하되, 그 다음(PNG 인코딩 + 디스크 저장)은
전용 스레드(`FrameWriter`, `AudioCapture` 와 같은 구조)로 넘겨서
비동기로 처리한다. `draw()` 는 픽셀을 채널로 던지자마자 바로 다음
프레임으로 넘어가니 더 이상 인코딩 속도에 발목 잡히지 않는다. 겸사로
PNG 압축도 기본(Balanced) 대신 `CompressionType::Fast` +
`FilterType::NoFilter` 로 바꿔 인코딩 자체도 더 빠르게 했다(파일은
좀 커지지만 이 도구엔 속도가 더 중요하다). Record 를 끌 때는 밀려
있는 프레임을 이 전용 스레드가 다 디스크에 쓸 때까지 기다린 뒤에야
(`FrameWriter::finish()`) mux_avi 를 돌린다 — 그래야 아직 안 써진
프레임이 빠진 채로 영상이 만들어지는 걸 막을 수 있다.

로컬에서 5초 정도 녹화를 재현해봤더니, 수정 전엔 프레임이 초반에
몰려 있었는데 수정 후엔 265장이 녹화 시작부터 끝까지 고르게
찍혔다(초당 약 50장) — 실제로 문제가 해결된 것을 확인했다.

## → 버그: 녹화 도중 다른 기능(글리치/씬 전환 등)을 쓰면 녹화 폴더가 계속 바뀜

"툴로 기능을 사용할 때마다 녹화 폴더를 변경하는거 같다" 는 제보.
원인은 `director_ipc::save()` 가 `director_state.json` 을 그냥
`std::fs::write()` 로 통째로 덮어써서, "쓰는 도중"에 그 파일을 읽으면
반쯤 써진 JSON 을 볼 수 있었던 것 — panel 은 Record 버튼뿐 아니라
글리치/노이즈 토글, 슬라이더 드래그, 씬 전환 버튼 등 **아무 조작이든**
할 때마다 state 전체를 다시 저장하는데, 그 순간 director 가 매 프레임
같은 파일을 다시 읽다가(`apply_director_state`) 타이밍이 겹치면
파싱에 실패한다. `load()` 는 파싱 실패 시 조용히 기본값
(`recording: false` 포함)으로 대체하도록 돼 있어서, director 입장에선
"방금 녹화가 꺼졌다가 그다음 프레임에 다시 켜졌다"로 보였다 — 그래서
녹화 중에 글리치를 켜거나 씬을 바꾸는 등 **아무 버튼이나 누르면**
그때마다 새 `recordings/rec_.../` 폴더가 또 만들어졌다.

`save()` 를 원자적으로 고쳤다 — 바로 그 자리에 덮어쓰는 대신
`director_state.json.tmp` 라는 임시 파일에 먼저 다 쓴 뒤,
`std::fs::rename()` 으로 진짜 파일 이름으로 바꿔치기한다. 같은
드라이브 안에서 rename 은 원자적 연산이라, 읽는 쪽(director)은 항상
완전히 다 써진 이전 내용이거나 완전히 다 써진 새 내용만 보게 되고,
반쯤 써진 파일을 읽는 경우 자체가 없어진다. 이제 녹화 중에 다른 조작을
아무리 해도 녹화 폴더/파일은 하나로 유지된다.

## panel 창 아래에 "합치는 중" 표시 추가

Record 를 끄면 화면상으로는 바로 꺼진 것처럼 보이지만, 실제로는
PNG+wav 를 output.avi 로 합치는 작업(mux_avi)이 별도 스레드에서 몇
초~몇십 초 더 걸린다. 그동안 진행 상황을 알 수 있게 panel 창 맨
아래에 파란 바 + "Encoding video..." 표시를 띄우도록 했다.

두 실행 파일이 서로 다른 프로세스라 여기서도 파일 하나로 통신해야
하는데, 기존 `director_state.json` 은 이미 "panel 이 쓰고 director 가
읽는" 방향으로 정해져 있어서 여기에 반대 방향(director → panel)
쓰기까지 섞으면 두 프로세스가 같은 파일을 서로 다른 시점에 통째로
다시 쓰다가 상대방이 막 쓴 내용을 지워버릴 위험이 있다(예: panel 이
글리치를 토글하는 바로 그 순간 director 가 muxing 상태를 저장하면,
둘 중 나중 저장이 이긴다). 그래서 방향이 반대인 상태는
`director_status.json` 이라는 별도 파일로 분리했다 —
`director_ipc::DirectorStatus { muxing: bool }`, `load_status()`/
`save_status()` (`director_state.json` 과 같은 임시파일+rename 방식).
director 는 mux 스레드를 시작하기 직전에 `muxing: true` 를, 다 끝난
뒤에 `muxing: false` 를 써두고, panel 은 매 프레임 이 파일을 읽어서
`muxing` 이 true 인 동안만 아래쪽 바를 그린다.

실제로 5초짜리 녹화를 끝내고 확인해보니, 정지 직후엔 `muxing: true`
로 몇 초간 유지되다가 합치기가 끝나자 `false` 로 정확히 바뀌는 걸
로컬에서 반복 확인했다.

## → 버그: 게임 스타트 버튼을 누른 시점에서 영상이 끊김

"영상이 게임 스타트버튼을 누르는 시점에서 끝난다" 는 제보. 프레임
저장을 전용 스레드(`FrameWriter`)로 넘긴 게 지난번 수정이었는데,
그 스레드 안 `for (idx, pixels) in rx { save_frame_png(...); }` 루프가
**단 한 프레임이라도 panic 하면 스레드 자체가 그대로 죽는다**는 걸
놓치고 있었다. 스레드가 죽으면 `rx`(채널 수신 쪽)도 같이 사라지는데,
메인 스레드의 `fw.send(...)` 는 실패를 그냥 무시하도록(`let _ = ...`)
해뒀던 터라 — 겉보기엔 녹화가 계속되는 것처럼 보이지만(record_frame
카운터는 계속 올라가고 앱도 안 멎는다) 실제로는 그 이후 프레임이 전부
디스크에 안 쓰이고 조용히 버려지고 있었다. 나중에 mux_avi 가 실제로
디스크에 남아있는 파일만 모아서 영상을 만드니, "특정 시점 이후가
통째로 빠진" 영상이 나온 것 — 정확히 어느 프레임에서 무슨 이유로
실패했는지는 알 수 없지만(재현 환경이 없어 직접 확인은 못 함), 씬
전환처럼 화면 내용이 크게 바뀌는 시점과 겹친 정황상 그 부근에서 뭔가
어긋난 것으로 보인다.

**고친 내용**: `save_frame_png` 가 이제 `Result` 를 돌려주고(예전엔
실패를 그냥 조용히 삼켰다), `FrameWriter` 의 루프는 프레임 하나 처리를
`std::panic::catch_unwind` 로 감싼다 — 어떤 이유로 실패하든(디스크
꽉 참, 인코딩 오류, 예상 못한 panic 등) 그 프레임만 건너뛰고
`recordings/.../frame_writer_error.log` 에 사유를 남긴 뒤, **다음
프레임부터는 계속 정상적으로 저장을 이어간다**. 이제 프레임 하나가
잘못돼도 그 이후 녹화 전체가 사라지는 일은 없고, 다음에 비슷한 문제가
또 생기면 이 로그로 정확한 원인을 알 수 있다.

## → 진짜 원인 확정: AVI 컨테이너 자체의 4GB 한계 — output.avi 를 여러 파트로 쪼갬

앞의 catch_unwind 수정으로도 "영상이 중간에 끝난다"는 제보가 다시
왔다. 이번엔 실제 녹화 폴더(`recordings/rec_1787656716_459/`)를 직접
열어봤다 — `frame_writer_error.log` 는 아예 없었고, PNG 는
frame_000000 부터 frame_001470 까지 **빠짐없이 1471장 전부** 정상
저장돼 있었다(파일 크기도 200~4400KB 로 계속 바뀌면서 실제 화면
내용이 끝까지 진행되고 있었다는 것도 확인). 문제는 프레임 저장이
아니라 **그 다음 단계, output.avi 자체**에 있었다 — 파일 크기가
7.2GB 였다.

AVI1.0(RIFF) 포맷은 청크 크기·`idx1` 인덱스의 오프셋 필드가 전부
**32비트**다 — 그런데 우리 프레임은 1280x960 32bpp 비압축이라 한 장에
약 4.9MB, 900장이 채 안 돼서 벌써 4GB 를 넘어간다. 그 지점을 넘는
프레임들의 idx1 오프셋 값은 32비트 범위를 넘겨서 **조용히
넘쳐버리는데**(release 빌드는 오버플로 검사가 꺼져 있어서 panic 도
안 나고 그냥 잘못된 값이 됨), 그러면 플레이어가 그 오프셋을 보고
완전히 엉뚱한 위치를 읽으려다 재생을 멈추거나 실패한다 — 정확히
"영상이 중간에 끝난다"는 증상 그대로다. 우리 쪽 오프셋 계산 자체는
처음부터 맞았다(로컬 재현 테스트에서도 짧은 녹화는 항상 멀쩡했다) —
파일이 4GB 를 넘을 만큼 길게 녹화했을 때만 드러나는, 컨테이너
포맷 자체의 한계였다.

**고친 내용**: `mux_avi()` 가 이제 프레임을 전부 한 파일에 몰아넣는
대신, 영상 데이터가 `MAX_AVI_VIDEO_BYTES`(~1.8GiB, 실제 한계인 4GB
보다 넉넉히 낮게 잡아 오래된/부실한 플레이어의 호환성 여유도 뒀다)를
넘지 않을 만큼씩 잘라서 `output_001.avi`, `output_002.avi` ... 로
따로따로 만든다(짧은 녹화라 한 파일로 충분하면 예전처럼 그냥
`output.avi` 하나만 나온다). 오디오도 각 파트가 담당하는 시간
구간(초 단위)에 맞는 만큼만 block_align 배수로 잘라서 같이
넣는다 — 파트별로 영상·소리가 서로 밀리지 않고 맞게 나온다.
파일 쓰기 로직 자체는 `write_avi_part()` 라는 함수로 그대로
분리했을 뿐, 어떻게 쓰는지는 이전과 동일하다.

## → 비압축 대신 MJPEG 로 — "그냥 압축해서 만들면 되지 않아?"

파일을 여러 파트로 쪼개는 걸로 4GB 한계는 피했지만, 근본적으로
"프레임 하나에 4.9MB(1280x960, 32bpp 비압축)"부터가 과했다 —
쪼개기는 증상 회피지 원인 해결이 아니었다. "압축해서 만들면 되지
않냐"는 지적을 받고, PNG 로 저장하던 프레임을 **JPEG(품질 85)**
로 바꾸고 AVI 도 그 JPEG 를 그대로 담는 **MJPEG(모션 JPEG)** 로
바꿨다.

- **프레임 저장**(`save_frame_jpg`, 예전 `save_frame_png`): PNG 는
  무손실이라 프레임당 1.6~4.4MB 였는데, JPEG(85) 는 프레임당 대략
  80~90KB — 약 20배 작다. 알파 채널을 버리고(JPEG 은 알파가 없음)
  RGB 로 변환하면서 동시에 상하 반전도 같이 처리해 한 번에 끝낸다.
- **AVI 조립**(`mux_avi`/`write_avi_part`): 이제 이미 저장된 JPEG
  바이트를 **다시 디코드하지 않고 그대로** `00dc` 청크에 옮겨 쓴다
  (예전엔 PNG 를 열어 RGBA 로 디코드한 뒤 BGRA32 로 다시 변환해서
  썼는데, 그 과정 자체가 없어져 mux 속도도 빨라졌다). 프레임마다
  압축률이 달라 크기가 들쭉날쭉해지므로, 파일 하나를 쓰기 전에 각
  프레임의 실제 파일 크기를 먼저 다 읽어둬서(디코드 없이 메타데이터만)
  정확한 헤더를 계산한다 — "고정 크기라 미리 계산 가능"이라는 예전
  전제가 깨져서 이 부분만 다시 짰다. `biCompression`/`fccHandler`
  를 `MJPG` 로 선언한다(전엔 `BI_RGB`).
- 파트 나누기도 "고정 프레임 수로 나누기"에서 "누적 크기가 한계
  직전까지 프레임을 묶기"로 바꿨다(프레임 크기가 더 이상 고정이
  아니므로).

로컬에서 5초짜리 녹화로 확인해보니 275프레임에 프레임당 약 83KB,
output.avi 전체가 22.9MB 였다 — 같은 길이를 예전 비압축 방식으로
찍으면 대략 1.3GB 였을 걸 감안하면 대략 1/60 크기. 이제 어지간히 긴
녹화가 아니면 애초에 파트 분할(4GB 한계)에 걸릴 일도 거의 없다.

## → "영상이 재생이 안돼" — 파일이 아니라 코덱 문제였다(확인 완료)

MJPEG 로 바꾼 뒤 "24초는 찍히는데 영상이 재생이 안 된다"는 제보가
왔다. 실제 output.avi 를 받아서 청크 단위로 끝까지 직접 걸어가며
검증했다 — `movi` 안의 1303개 프레임 청크 전부 ID/크기가 정확히
맞물리고, 그 뒤 `idx1`(크기까지 정확)로 딱 이어지는 것까지 확인했다.
JPEG 데이터 자체도 표준 baseline JFIF(SOI/APP0/SOF0 마커 정상)였다 —
즉 **파일은 100% 정상**이었다.

원인은 재생 프로그램이었다 — Windows 기본 "영화 및 TV" 앱은 MJPEG
코덱이 기본으로 없는 경우가 흔하다. 실제로 VLC 로는 정상 재생되는 걸
확인했다. 사용자에게 "MJPEG 유지(파일 작음, VLC 등 별도 플레이어
필요)" 와 "비압축 복귀(코덱 불필요, 파일 커짐)" 중 선택하게 했고,
**MJPEG 유지**를 골라서 지금 상태 그대로 둔다 — 코드 변경 없음, 녹화
결과물은 VLC 같은 일반적인 무료 플레이어로 재생하면 된다.

## 실제 게임에 Photos 앱 추가

`assets/photo/` 에 폐 건물 사진(중복 제거 후 394장)을 채워두고, 바탕화면
Mail 바로 아래(fs.desktop 순서상 mail 다음 = 같은 열의 바로 아랫칸)에
"Photos" 아이콘을 추가했다. 열면 썸네일 피드가 뜨고, 사진을 클릭하면
자세히 보기로 넘어가서 원본을 보거나 "Download" 로 저장할 수 있다.

**왜 기존 `FileKind::Img`/`Assets::photos` 를 안 썼나**: 예전 Photos.tar
콘텐츠용으로 만들어뒀던 이 경로는 게임 시작할 때(`Assets::load`) 사진을
전부 미리 텍스처로 올려두는 구조다. 사진 2~3장짜리 소품일 땐 문제없지만
394장을 그렇게 올리면(1080x~1600 사진 하나가 RGBA 로 3~7MB) 텍스처 메모리
가 1~1.5GB 대로 치솟고, 게임을 켤 때마다 그 디코딩 시간만큼 부팅이
느려진다 — Photos 앱을 한 번도 안 여는 플레이어까지 그 비용을 치르게
된다. 그래서 `PhotosApp` 은 완전히 자기 안에서 도는 지연 로딩 캐시를
따로 둔다 — 앱을 열 때는 파일 이름 목록만 훑고(`assets/photo` 를
video_player.rs 의 movie.mp4 처럼 실행 파일 옆에서 런타임에 찾는다,
재빌드 없이 사진만 바꿔도 됨), 실제 디코드+텍스처 업로드는 화면에 보일
때(피드는 96px 축소판, 자세히 보기는 열람 중인 한 장만 원본 해상도)
그때그때 한다. 자세히 보기 텍스처는 다른 사진으로 넘어갈 때 이전 것을
`delete_texture` 로 바로 반납해서 여러 장을 옮겨봐도 안 쌓인다.

**"다운로드"는 가상 Downloads 탭 대신 진짜 파일 복사**: 메일 첨부파일의
다운로드는 가상 파일 시스템(`fs.downloads`) 에 기존 FileId 를 추가하는
방식인데, 이 사진들은 애초에 `fs.nodes` 에 등록된 파일이 아니라 실제
디스크에 있는 파일이다. 그래서 director.rs 의 `recordings/` 폴더와 같은
요령으로, exe 옆에 실제 `downloads/` 폴더를 만들어 원본 jpg 를 그대로
복사한다 — "다운로드 받는다"는 말 그대로의 동작이기도 하고, 가상 FS 쪽
코드(Explorer/Downloads 탭 렌더링 등)를 전혀 안 건드려도 된다.

새 게임에서만 바로 보인다(기존 저장 파일은 `FileSystem::new()` 가 다시
안 불려서 desktop 목록이 그대로 유지됨 — 이 프로젝트의 다른 콘텐츠
추가도 다 이런 식이라 특별히 마이그레이션은 안 넣었다). 디스플레이가
없는 환경이라 실제로 클릭해서 열어보는 눈으로 하는 확인은 못 했고,
컴파일/클리피(5개 유지)와 새 게임 기동이 안 죽는 것까지만 확인했다 —
직접 열어보고 이상 있으면 알려달라.

## → "빌드해도 프로그램이 딱히 생기지는 않는다" — build_director.ps1 추가

지금까지 director/director_panel 을 빌드해서 `production/director/`
안의 `PalaceOS-Director.exe`/`PalaceOS-Director-Panel.exe` 로 바꿔치기
하는 건 매번 내가(Claude) 대화 중에 수동으로 `cargo build --release`
+ 복사 + 이름 바꾸기 + 이전 상태 파일 정리까지 손으로 해주던 과정이라,
직접 `cargo build` 만 돌리면 `target/release/` 안에 원래 이름
(director.exe)으로만 생길 뿐 `production/director/` 쪽은 전혀 안
바뀐다 — "빌드했는데 프로그램이 안 생긴다"는 게 바로 이 얘기였다.

이 과정을 그대로 스크립트로 옮긴 `build_director.ps1` 을 프로젝트
루트에 추가했다. 사용법:
```
powershell -ExecutionPolicy Bypass -File build_director.ps1
```
release 빌드 → `production/director/` 에 두 exe 복사+이름 변경 →
`director_state.json`/`director_status.json`/각종 `.log` 같은 이전
실행의 남은 상태 파일 정리까지 한 번에 한다. 실제로 로컬에서 실행해서
새 타임스탬프로 두 exe 가 갱신되는 것까지 확인했다.

## → Photos: 다운로드를 게임 내 Downloads 탭으로, 썸네일은 여백 없이 꽉 채우게

두 가지 피드백을 반영했다.

1. **"다운로드는 게임 내 다운로드를 말한 거야"** — 처음엔 exe 옆 실제
   `downloads/` 폴더에 파일을 복사하는 식으로 만들었는데, 원한 건 메일
   첨부파일처럼 File Explorer 의 Downloads 탭에 들어가는 것이었다.
   `FileKind::Photo(String)`(사진 파일명을 담는 새 종류)와
   `AppAction::DownloadPhoto(String)` → `DeskAction::DownloadPhoto` →
   `FileSystem::find_or_add_photo()` 경로를 새로 만들어서, 메일 첨부
   다운로드와 완전히 같은 처리(`fs.download()`, Explorer 새로고침, 즉시
   저장)를 탄다. 같은 사진을 두 번 다운로드해도 `find_or_add_photo` 가
   이미 만든 노드를 재사용해서 Downloads 에 중복으로 안 쌓인다.
   Downloads 탭에서 그 사진을 다시 열면(`apps/mod.rs::open()`) 새로 만든
   `PhotoViewerApp` 이 파일명으로 원본을 다시 디코드해서 보여준다
   (`assets.photos`/`ImageViewerApp` 은 여전히 안 씀 — 이유는 위 Photos
   앱 추가 항목 참고).
2. **"마진 없이 서로 채우는 방식"** — 사진마다 종횡비가 달라서 정사각형
   칸 안에 맞추면(letterbox) 검은 여백이 생겼다. 썸네일 전용
   `load_thumb_texture()` 를 새로 만들어, 원본 가운데를 정사각형으로
   잘라낸(`image::imageops::crop_imm`) 뒤 칸 크기로 리사이즈하게 했다 —
   사진 가장자리가 살짝씩 잘려나가는 대신 칸을 항상 꽉 채운다(cover
   방식). 자세히 보기 화면은 사진 전체를 보여줘야 하니 원래대로
   letterbox(비율 유지, 레터박스)를 그대로 뒀다 — 썸네일 피드에서만
   바뀐 것.

## → Photos 창 크기 고정

썸네일 3x3(스크롤 없이 딱 맞는) 크기로 창을 고정해달라는 요청 —
`apps/mod.rs::open()` 의 `FileKind::PhotoGallery` 항목에서 `size` 를
(350, 350) 으로, `resizable`/`maximizable` 을 둘 다 false 로 바꿨다
(Tar/Installer 같은 다른 고정 크기 창과 같은 패턴). 이제 창 테두리를
끌어도 안 늘어나고 최대화 버튼도 안 먹는다.

## Director 글리치: 뭉탱이 크기도 버스트마다 랜덤

지금까지 밴드 굵기(`GlitchParams::freq`, 줄 수가 적을수록 뭉탱이짐)는
강도 슬라이더 값만으로 매 프레임 고정 계산됐다 — 같은 강도로 계속
터지면 항상 똑같은 굵기로만 찢어졌다. seed 와 똑같은 요령으로, 새
버스트가 시작될 때 `overlay_glitch_freq` 를 강도에 비례한 범위
(`6.0 ~ 8.0 + 강도*14.0`) 안에서 한 번 랜덤으로 뽑아 그 버스트 내내
고정해 쓰도록 바꿨다 — 이제 강도는 그대로여도 터질 때마다 밴드가
얇을 때도 굵을 때도 있다.

## Photos 창 배경을 검은색으로

썸네일 격자 뒤 배경이 밝은 회색(0.93)이었는데, 검은색으로 바꿨다 —
격자가 350x350 고정창을 정확히 안 채우는 자리(마지막 줄 등)가 밝게
남아 눈에 띄던 것을 없앴다.

## Photos 피드를 정사각형 크롭 대신 매이슨리(Pinterest 식) 레이아웃으로

"원본 비율 유지하면서 깔끔하게 정렬해달라"는 요청 — 예전엔 사진마다
가운데를 정사각형으로 잘라서(cover) 96x96 칸에 욱여넣었는데, 이번엔
그 크롭을 없애고 각 사진이 원본 종횡비 그대로 보이는 매이슨리(핀터레스트
피드 같은, 여러 열에 높이가 제각각인 채로 빈틈없이 쌓이는) 레이아웃으로
바꿨다.

**동작 방식**(`layout_masonry()`): 열 개수를 창 너비 기준으로 정하고,
사진을 순서대로 "지금까지 쌓인 높이가 가장 낮은 열"에 배정한다 — 그
사진의 칸 너비는 열 너비로 고정, 높이는 `열 너비 / 원본 종횡비` 로
계산해서 원본 비율이 그대로 유지된다. 그러면 열마다 높이가 자연스럽게
비슷하게 맞춰지면서도, 자르거나 여백을 채우는 일 없이 사진들이 서로
빈틈없이 이어 붙는다.

**종횡비를 미리 알아야 하는 문제**: 이 레이아웃을 짜려면 배치하기 전에
모든 사진의 가로/세로 비율을 알아야 하는데, 394장을 전부 디코드해서
확인하면 느리다 — 대신 `image::image_dimensions()` 로 파일 헤더만 읽어
(픽셀 디코드 없이) 크기만 빠르게 알아낸다. `PhotosApp::new()` 에서 한
번만 하고, 실제 픽셀 디코드+텍스처 업로드는 여전히 화면에 보일 때만
그때그때 한다(지난 변경에서 만든 지연 로딩 그대로).

## 사진 클릭 시 같은 창 안 자세히 보기 → 별개의 창(PhotoViewerApp)

"이미지를 누르면 별개의 창이 나오게" 요청 — 지금까지는 Photos 창
안에서 `self.detail` 로 피드/자세히보기 화면을 토글했는데, 이제
썸네일을 클릭하면 `AppAction::OpenPhoto(파일명)` 을 돌려주고, Mail/
Explorer 항목을 열 때와 완전히 같은 경로(desktop.rs 의
`DeskAction::OpenPhoto`)로 별개의 창을 연다. 그 창에서 쓰는 앱은
Explorer 의 Downloads 탭에서 이미 받은 사진을 다시 열 때 쓰던
`PhotoViewerApp` 을 그대로 재사용한다 — 같은 사진을 피드에서 또
클릭해도(`wm.open` 이 FileId 로 중복 체크) 창이 여러 개 안 생기고
기존 창이 앞으로 나온다.

이 경로를 타려면 그 사진이 `fs.nodes` 에 FileId 로 등록돼 있어야 하는데
(그래야 `wm.open` 의 중복 열기 체크·창 위치 기억이 작동한다), 클릭한
시점엔 아직 다운로드 전일 수 있다 — 그래서 `FileSystem::find_or_add_photo()`
를 "다운로드할 때"(`fs.download()` 도 같이 부름) 뿐 아니라 "열기만 할
때"(다운로드는 안 부름)에도 쓰도록 했다. 그냥 열어보기만 해서는 Downloads
탭에 안 뜨고, 그 창 안에서 "Download" 를 눌러야 진짜로 다운로드된다.

`PhotosApp` 자체는 이제 피드 화면만 그리는 훨씬 단순한 구조가 됐다
(자세히보기/원본 텍스처 캐시/헤더 버튼이 전부 `PhotoViewerApp` 쪽으로
옮겨감).

## PhotoViewerApp 하단에 회색 "Download" 글자

버튼 테두리 없이 사진 아래에 회색 글자만 놓고, 누르면 다운로드되게
했다(호버 시 밝은 회색으로, 다운로드 직후엔 "Downloaded" 로 잠깐
바뀜) — `ui::button` 의 3D 버튼 대신 `r.text()` 로 직접 그리고
클릭 판정만 좌표로 직접 계산한, 이 프로젝트에서 흔한 "글자만 있는
링크" 스타일이다.

## → Download 글자를 사진 위에 겹치도록

따로 검은 띠(footer)를 만들어 그 안에 두는 대신, 사진이 창을 꽉
채우게(letterbox 계산에 FOOTER_H 를 안 빼고) 그린 뒤 그 오른쪽 아래
모서리 위에 글자를 그대로 얹었다 — 밝은 사진 위에서도 글자가 묻히지
않게, 살짝 어긋난 검은 그림자를 한 번 먼저 그리고 그 위에 회색 글자를
덧그린다(옅은 드롭섀도 효과).

## Photos 앱 이름을 깨진 글자로, 아이콘도 전용 모양으로

**이름**: 바탕화면 아이콘/창 제목에 쓰이던 "Photos"(한국어는 "사진")
대신 "사찢진" 으로 바꿨다 — "사진" 사이에 낯선 음절(찢, "찢다"의 그
글자)을 끼워 넣어 이름 자체가 오염된 느낌을 내면서도, "찢어진 사진"
이라는 실제 컨셉(폐 건물 사진들)과도 은근히 맞아떨어지게 골랐다.
`foundation.rs::display_name()` 에 이 이름을 위한 번역 항목을 일부러
안 둬서, 어떤 언어 설정이든 이 원문 그대로 나온다(글자 자체는 폰트
아틀라스가 완성형 한글을 전부 담고 있어 정상적으로 렌더링된다 —
"깨져 보인다"는 인상은 글자가 실제로 깨진 게 아니라 낯선 조합에서
오는 것).

**아이콘**: 예전엔 낱장 사진 파일과 똑같은 `IconType::Img` 를 그대로
썼는데, 여러 장을 모아보는 앱이라는 걸 구분하기 위해 새 아이콘
(`IconType::PhotosApp`)을 만들었다 — Tar/Installer/HexTool 처럼 PNG
텍스처 대신 도형을 직접 그리는 방식으로, 살짝 어긋나게 겹쳐 쌓인
사진 두 장(뒤) 위에 하늘/해/산이 그려진 사진 한 장(맨 앞)을 놓아
"사진첩" 느낌을 냈다. 낱장 사진(다운로드한 개별 파일)의 아이콘은
그대로 `IconType::Img` 를 쓴다 — 앱 자체와 그 안의 사진 하나하나가
아이콘으로 구분된다.

## → "진짜 깨진 폰트"로, 아이콘도 더 훼손된 느낌으로

"사찢진"은 결국 완성형 음절이라 진짜 깨진 것처럼은 안 보인다는
피드백 — 이번엔 완성형 한글(음절) 대신 **호환용 자모**(U+3131~318E,
자음/모음 낱개)를 그대로 써서 이름을 "ㅅㅏㅈㅣㄴ"으로 바꿨다. 자음과
모음이 하나의 글자로 합쳐지지 않고 낱개로 흩어져 나오는데, 이게 실제
한글 인코딩이 깨졌을 때 정확히 나오는 모양이라 훨씬 그럴듯하게
"깨진 글자"로 읽힌다(폰트 자체엔 이 자모들이 다 들어있어서 렌더링은
정상이다 — 그냥 조합이 안 될 뿐).

아이콘도 평온한 하늘/해/산 사진 대신, 프레임 안을 TV 지지직 같은
가로 노이즈 줄무늬로 채우고 그 위에 Director 글리치 특유의 좌우로
갈라진 빨강/청록 색분리 줄무늬를 얹은 뒤, 오른쪽 아래 모서리를
반투명 그림자로 파먹어 "뜯겨나간" 자국까지 낸 걸로 다시 그렸다 —
이름("ㅅㅏㅈㅣㄴ")과 이 앱이 보여주는 폐 건물 사진들의 훼손된
느낌에 맞춘 것.

## → 진짜 "네모+물음표"(tofu) 깨진 글자 — 렌더러에 자리표시자 추가

스크린샷(옛날 프로그램의 잘못된 코드페이지 — 검은 네모에 물음표가
찍힌 글자들과 멀쩡한 라틴 글자가 뒤섞인 모습)까지 주면서 "진짜 깨진
폰트"를 요청 — 자모를 흩어놓는 것만으로는(이전 시도) 여전히 다
읽히는 글자라 부족했다.

문제는 우리 렌더러(`gfx.rs`)가 애초에 아틀라스에 없는 글자를 만나면
**그냥 그리는 걸 건너뛰었다**는 것 — Windows 가 보여주는 그 특유의
"네모 안에 물음표" 자리표시자(흔히 두부, tofu 라고 부름)가 아예
없었다. `Renderer::glyph()` 에 그 자리표시자를 그리는 `draw_tofu()`
를 새로 추가했다 — 어두운 사각형을 그린 뒤 그 위에 흰 물음표를
겹쳐 그려서, 실제 지원 안 되는 코드페이지 글자를 만났을 때의 모양을
흉내낸다. 아틀라스에 있는 글자들의 렌더링은 전혀 안 바뀐다 —
지금까지 조용히 안 그려지던 자리에만 이 표시가 새로 나타난다.

Photos 앱 이름도 흩어놓은 자모("ㅅㅏㅈㅣㄴ") 사이사이에 아틀라스에
아예 없는 문자(키릴 문자, 물음표 등)를 섞어 `"ㅅЫㅏЩㅈ?ㅣЁㄴ"` 로
바꿨다 — 읽을 수 있는 자모 조각과 진짜 깨진 두부가 뒤섞여 나와서,
스크린샷 속 그 잘못된 코드페이지 화면과 훨씬 가까워졌다.

## → tofu 모양을 네모 대신 진짜 마름모로

"?가 들어있는 마름모는 안 돼?" — 처음 만든 tofu 자리표시자는 그냥
사각형이었다. 렌더러에 회전된 사각형(마름모)을 그리는 기능이 없어서,
`ui.rs::fill_circle` 이 원을 가로줄들로 라스터화하는 것과 같은
요령으로 `draw_tofu()` 를 다시 짰다 — 1px 높이 가로줄을 여러 개
그리되, 마름모 중심에서 위/아래로 멀어질수록 그 줄의 폭을 좁혀서
쌓으면 마름모 윤곽이 나온다. 그 위에 흰 물음표를 얹는 건 그대로라,
이제 스크린샷 속 그 "마름모+물음표" 모양과 실제로 같은 형태가 됐다.

## → 자모 섞지 말고 전부 깨진 글자로

"ㅅㅏㅈㅣㄴ 같은 거 하지 말고 그냥 깨진 글자만" — 자모+키릴 문자를
섞었던 이름(`"ㅅЫㅏЩㅈ?ㅣЁㄴ"`)은 여전히 읽히는 자모 조각이 남아있어
부족했다. 읽을 수 있는 글자를 하나도 안 남기고 전부 아틀라스에 없는
키릴 문자로만 채운 `"ЫЩЁЪЭЮ"` 로 바꿨다 — 이제 이름 전체가 마름모+
물음표(tofu)로만 나온다.

(참고: `target/debug/palaceos_save.json` 같은 예전 저장 파일이 있으면
`FileSystem::new()` 가 다시 안 불려서 이름이 안 바뀐 채로 보인다 —
이번에도 테스트용 저장 파일을 지워서 다음 실행이 새 게임으로
시작하게 해뒀다.)

## Photos 아이콘을 사용자가 준 이미지로 교체

직접 그린 "지지직 노이즈+색분리 줄무늬" 아이콘 대신, 사용자가 준
32x32 아이콘(흰 창 틀 안에 물음표 — Windows 의 "연결 프로그램 없는
파일" 아이콘과 같은 느낌)을 그대로 쓰기로 했다. `assets/icon_photos.png`
로 저장해두고, 다른 아이콘들(`icon_mail.png` 등)과 똑같은 방식으로
`Assets::load()` 에서 `include_bytes!` 로 읽어 텍스처로 올린다.
`IconType::PhotosApp` 을 Tar/Installer/HexTool 같은 "직접 그리는"
특수 케이스에서 빼고, 다른 파일 아이콘들과 같은 일반 텍스처 방식으로
옮겼다 — 이제 안 쓰는 `draw_photos_icon()` 손그림 함수는 지웠다.

## Photos 피드를 10장으로 제한(진행도 연동 잠금 해제의 임시 자리)

"게임 진행에 따라 이미지가 계속 추가되게 만들 건데, 일단 10개 정도만"
— `PhotosApp::new()` 에서 파일 목록을 정렬한 뒤
`UNLOCKED_PHOTO_COUNT`(10) 만큼만 잘라 쓰도록 했다. `assets/photo`
안의 394장은 그대로 다 남아있고 폴더에서 지운 게 아니다 — 지금은
그중 앞쪽 10장만 보여줄 뿐, 나중에 진행 상황에 따라 하나씩 풀리는
시스템이 생기면 이 고정 개수 대신 그 진행값을 받아쓰도록 바뀔
자리로 남겨뒀다(그 시스템 자체는 아직 안 만들었다).

## Photos 바탕화면 아이콘에 랜덤한 색수차 글리치

"?????" (일부러 깨뜨린 이름) 프로그램 아이콘에 4~11초 간격으로 랜덤하게
0.18초짜리 짧은 글리치가 스친다 — lobby.rs 의 화면 전체 글리치 타이밍
로직(`tick_glitch`)과 같은 요령을 `DesktopScene` 에 작게 옮겨왔다
(`tick_icon_glitch`, 이 프로젝트 관례대로 desktop.rs 전용 xorshift64
`Rng` 를 새로 둠).

`ui::draw_icon()` 은 항상 흰색으로만 그리는 공용 API라 색을 못 입히므로,
글리치가 진행 중일 땐 그 대신 `assets.icon_photos` 텍스처를 직접
세 번(빨강은 왼쪽으로, 초록은 그대로, 파랑은 오른쪽으로 살짝씩 어긋나게)
겹쳐 그려서 Director 글리치와 같은 색수차 느낌을 아이콘 크기로 흉내낸다.
어긋나는 폭은 버스트가 시작되는 순간에 한 번만 랜덤으로 뽑아 그 버스트
내내 고정한다(매 프레임 다시 뽑으면 떨리기만 하고 "한 번 찢어진" 느낌이
안 난다) — lobby.rs 의 로고 흔들림과 같은 패턴.

## → 버그: 일본어 문장부호가 tofu 로 깨짐(회귀) — Photos 는 삭제 불가 + "Are You idiot?" 모달

**일본어 회귀 원인**: 휴지통 안내문의 일본어 텍스트에서 쉼표(、)/
마침표(。) 자리마다 ◆ 가 찍히는 제보. 원래 이 문장부호들(U+3000~303F
CJK 기호·구두점 블록)은 폰트 아틀라스에 애초부터 없었는데, 예전엔
`Renderer::glyph()` 가 없는 글자를 그냥 조용히 건너뛰어서(빈 칸으로만
보임) 안 티가 났다 — 이번에 Photos 이름을 "진짜 깨진 글자"로 만들려고
없는 글자에 대해 항상 마름모+물음표(tofu) 를 그리게 바꾸면서, 원래도
없었던 이 문장부호들까지 전부 깨져 보이는 회귀가 생겼다. `gfx.rs::
build_font_atlas()` 에 CJK 기호·구두점 블록을 통째로 추가해서 근본
원인부터 고쳤다 — 히라가나/가타카나를 통째로 넣는 것과 같은 요령.

**Photos 삭제 방지**: `desktop.rs::move_ids_to()` 에 이미 있던 My
Computer/Recycle Bin 자기 자신의 "휴지통을 포함해 어떤 폴더로도 못
옮긴다" 가드와 같은 자리에, `FileKind::PhotoGallery` 도 추가했다 —
Photos 아이콘을 휴지통으로 끌어다 놓으면 실제로 옮겨지지 않는다.

**"Are You idiot?" 모달**: My Computer/Recycle Bin 은 조용히 막던 것과
달리, Photos 를 휴지통에 넣으려 하면 `idiot_confirm` 플래그를 세워서
화면 전체를 덮는 조롱 모달을 띄운다. `erase_confirm`("Erase All
Memory" 확인창)과 완전히 같은 방식으로 구현했다 — 진짜
WindowManager 창이 아니라 desktop.rs 가 직접 그리는 오버레이라서,
"최소화/최대화/크기조절/닫기 버튼이 없어야 한다"는 요구조건이
저절로 만족된다(애초에 그런 버튼을 그릴 코드 자체가 없다). "Yes"
버튼 하나만 눌러야 닫히고, 그 전까진 다른 모든 클릭(시작메뉴, 다른
아이콘, 창 등)이 다 막힌다 — `erase_confirm` 이 열려있을 때와 같은
입력 차단 로직을 공유한다.

## 전체 문자열 전수 조사 — 깨진 글자 더 있었음

메일 테스트 메시지("첨부파일이 없는 짧은 테스트 메시지입니다 ◆ 안
읽음 목록과...")에서 ◆ 가 또 나온다는 제보 — CJK 문장부호를 고친 뒤로도
남아있던 다른 깨진 글자들이 있었다. Python 스크립트로 `src/` 안의 모든
`.rs` 파일에서 문자열 리터럴만 뽑아(라인 주석은 제외) 폰트 아틀라스가
커버하는 범위(ASCII+한글+가나+한자+CJK구두점) 밖의 문자를 쓰는 곳을
전수 조사했다. 찾아낸 것:

- `apps/mail.rs`: 테스트 메시지 3개(영/한/일) 전부 문장 중간에 **em
  dash(—, U+2014)** 를 구분자로 쓰고 있었다 — 아틀라스에 없어서 tofu 로
  깨짐. 전부 마침표/쉼표로 자연스럽게 바꿨다.
- `apps/explorer.rs`, `apps/recycle_bin.rs`: 상태바의 일본어 문구
  ("...オブジェクト（選択中 {m}個）")가 **전각 괄호（）**(U+FF08/FF09)
  를 썼다 — 한국어/영어 버전은 반각 괄호를 쓰는데 일본어만 전각이라
  일관성도 없었다. 반각 괄호로 통일했다.
- `bin/director_panel.rs`: Record 버튼 라벨의 **■/●** 기호(U+25A0/25CF)
  — 게임 폰트와 같은 렌더러를 쓰는 Director 패널이라 마찬가지로 tofu 로
  깨져 있었다. `[REC]`/`[STOP]` 같은 ASCII 표기로 바꿨다.

`▼`(콤보박스 화살표)나 `✓`(체크박스) 처럼 코드에 등장하는 다른 특수
기호들은 확인해보니 전부 `r.rect` 로 직접 그리는 손그림 도형이었지
실제 폰트 글자가 아니어서 문제없었다 — 스크립트가 initally 이런 것도
"의심 문자"로 걸러냈지만(정규식이 원시 문자열 리터럴/여러 줄 문자열을
완벽히 못 걸러내서), 하나하나 실제 문자열 리터럴인지 직접 확인해서
가려냈다.

## 이미 다운로드한 사진은 "Download" 글자 자체를 안 보여줌

Photos 피드에서 처음 보는 사진이든, Explorer 의 Downloads 탭에서
이미 받은 걸 다시 열어본 사진이든 `PhotoViewerApp` 하나를 그대로
같이 썼는데, 후자의 경우에도 계속 "Download" 글자가 떠 있었다 — 이미
받은 걸 또 받으라는 셈이라 안 맞는 동작이었다.

`PhotoViewerApp::new()` 에 `show_download: bool` 인자를 추가해서,
그 사진을 아예 안 그리는 게 아니라 하단의 "Download" 글자 자체를
조건부로만 그리게 했다. 이 값은 `apps/mod.rs::open()` 에서
`FileKind::Photo` 를 열 때 `fs.ever_downloaded.contains(&id)` 를
보고 정한다 — 메일 첨부파일의 "재다운로드 버튼을 보여줄지" 판단에
쓰던 것과 똑같은 필드를 그대로 재사용했다(foundation.rs 의 기존
주석에 이미 "Mail 이 재다운로드 버튼을 보여줄지 판단할 때 downloads
대신 이걸 봐야 한다" 고 설명돼 있던 그 필드). 한 번이라도 받은 적
있으면 `show_download=false` 로 열려서 글자 자체가 안 보이고, 아직
한 번도 안 받았으면(Photos 피드에서 처음 클릭) 그대로 보인다.

## → 버그 수정: Photos 피드에서 열어도 Download 가 안 보였음

`fs.ever_downloaded` 기준으로 바꾼 뒤 "이제는 ?????에서 이미지를
열었을 때 다운로드가 안 나온다"는 제보 — 테스트하면서 이미 여러 장을
한 번씩 받아봤을 테니, 그 사진들은 실제로 `ever_downloaded` 가 true
가 맞았다(그 자체는 버그가 아니었다). 문제는 판단 기준을 잘못 잡은
것 — "이 사진을 예전에 받은 적 있는가" 가 아니라 "**지금 어느
화면에서 열었는가**" 로 봤어야 했다. Photos 피드에서 클릭했으면
(그 사진을 전에 받았든 안 받았든) Download 를 보여줘야 다시 받고
싶을 때 받을 수 있고, Explorer/Downloads 탭에서 열었을 때만
"이미 여기 있는 파일" 이니 안 보여주는 게 맞다.

그래서 두 진입점을 완전히 분리했다:
- `apps/mod.rs::open()` 의 `FileKind::Photo` 분기(Explorer 에서 더블
  클릭) — 이제 조건 없이 항상 `show_download: false`.
- `desktop.rs::DeskAction::OpenPhoto`(Photos 피드에서 썸네일 클릭) —
  더 이상 `apps::open()` 을 거치지 않고 `Opened` 를 직접 만들어서
  항상 `show_download: true`.

같은 사진을 두 경로 다로 열어본 적 있으면(피드에서 봤다가 나중에
Explorer 로도 열어보는 등) 먼저 열린 창이 그대로 앞으로 나오는
기존 중복-열기 방지 동작은 그대로 유지된다 — 창을 한 번 닫고 다시
열면 그 경로에 맞는 화면으로 새로 뜬다.

## → 완전히 별개의 창으로: FileId 공유 자체를 없앰

"?????에서 여는 이미지 파일과 my computer 에서 여는 이미지파일을
별개로 구분하라니까" — 지난 수정은 코드 경로(show_download 값)만
갈랐을 뿐, 창 중복-열기 방지가 여전히 같은 FileId 를 기준으로 했다.
그래서 예를 들어 My Computer 로 먼저 열어(Download 버튼 없음) 창이
떠있는 상태에서 Photos 피드로 같은 사진을 클릭하면, 새 창이 뜨는 게
아니라 그 기존 창(버튼 없는 쪽)이 그냥 앞으로만 나왔다 — "별개"가
아니었다.

`DeskAction::OpenPhoto`(Photos 피드) 에서 `fs.find_or_add_photo()`
호출 자체를 없애고, `wm.open()` 에 파일 식별자를 아예 안 넘긴다
(`Some(id)` 대신 `None`) — 이러면 My Computer 쪽 창과 겹쳐 앞으로
당겨지는 일 없이 매번 독립된 새 창(Download 버튼 있는 미리보기)이
뜬다. `find_or_add_photo()` 로 `FileKind::Photo` 노드를 만드는 건
이제 실제로 "다운로드" 버튼을 눌렀을 때(`DeskAction::DownloadPhoto`)
뿐이다 — 그냥 미리보기만 열어보는 건 fs 에 아무 흔적도 안 남긴다.
대신 창 위치를 기억해뒀다 복원하는 기능은(FileId 기반이라) Photos
피드 미리보기 창에는 더 이상 적용 안 된다 — 매번 기본 위치에 뜬다.

## 크레딧에 gxng_m1n 추가

`foundation.rs::CREDITS` 목록에 개발자 한 명("gxng_m1n")을 더 추가했다
(3명 → 4명). 이름이 하나 늘어난 만큼 Credits 창이 OK 버튼과 안 겹치게
`desktop.rs` 에서 여는 창 높이도 180→200 으로 같이 늘렸다.

## → 버그 수정: 크래딧 창 위에 파일을 놓으면 게임이 터짐

**원인**: Credits/Official Site 창은 desktop.rs 가 `CREDITS_WIN =
usize::MAX - 2` 같은 **가짜 FileId**(진짜 fs.nodes 항목이 아니라 창
중복-열기 방지용으로만 쓰는 값)로 연다. 바탕화면 아이콘을 어떤 창
위로 드래그해서 놓으면 `explorer_drop_target_at()` 이
`wm.file_at(m)` 로 "그 자리에 있는 창의 FileId" 를 얻어와서 곧바로
`self.fs.get(win_file)` 로 넘기는데, 그 창이 Credits/Official Site면
이 가짜 값이 그대로 `fs.nodes` 배열 인덱스로 쓰여 **범위를 한참
벗어나 그 자리에서 panic** — 게임 전체가 죽었다.

**고친 내용**: `FileSystem::contains(id)` 를 새로 추가해서(`id <
nodes.len()`) `explorer_drop_target_at()` 맨 앞에서 그 창이 진짜
fs.nodes 항목에 연결된 창인지부터 확인하고, 아니면(Credits/Official
Site 같은 가짜-FileId 창이면) 그냥 "옮길 대상 아님"(None)으로 조용히
처리한다 — 실제 동작도 이게 맞다(파일 탐색기가 아닌 창 위에 놓으면
어차피 옮겨지면 안 된다). 데스크톱 아이콘끼리의 드롭 판정
(`desktop_folder_drop_target_at`)은 애초에 `fs.desktop` 에서만 id 를
가져와서 이 문제와 무관하다 — 코드 전체에서 `wm.file_at()` 결과를
`fs.get()` 에 넘기는 지점이 이 한 곳뿐이라는 것도 확인했다.

## → 버그 수정: ?????(Photos) 에서 같은 사진을 여러 번 클릭하면 창이 계속 쌓임

**원인**: 바로 위에서 "Photos 피드로 연 창과 My Computer(Explorer/
Downloads 탭)로 연 창은 완전히 별개로 취급해야 한다"는 요청을
반영하면서, Photos 피드 쪽 `DeskAction::OpenPhoto` 핸들러가
`wm.open(op, None, work)` 처럼 dedup 키를 아예 `None` 으로 넘기게
고쳤었다. 그런데 `WindowManager::open()` 의 중복-창 방지 로직은
`file: Option<FileId>` 가 `Some(...)` 일 때만 작동하므로, `None` 을
넘기면 My Computer 쪽과는 안 섞이지만 **Photos 피드 안에서 같은
사진을 반복 클릭하는 것도 매번 전혀 새 창**으로 열려버렸다 —
사용자가 스크린샷으로 보여준 것처럼 같은 파일명의 창이 계단식으로
계속 쌓이는 원인이었다.

**고친 내용**: "My Computer 와는 분리하되, Photos 피드 안에서는
dedup 되어야 한다"를 동시에 만족시키기 위해, 파일명에서 결정적으로
뽑아낸 **가짜 FileId** 를 dedup 키로 쓰도록 바꿨다.
`PHOTO_PREVIEW_WIN_BASE = usize::MAX / 2` 를 시작점으로 잡고,
`photo_preview_win_id(filename)` 이 파일명을 해시해서 그 근처
10,000,000 칸짜리 대역 안의 한 값으로 매핑한다 — 같은 파일명은 항상
같은 값이 나오므로 같은 사진을 여러 번 클릭해도
`wm.open(op, Some(photo_preview_win_id(&filename)), work)` 가 기존
창을 앞으로 당길 뿐 새로 열지 않는다. 이 대역은 진짜 `fs.nodes`
인덱스(0부터 시작, 지금 최대 수백 개)와도, `CREDITS_WIN`/
`OFFICIAL_SITE_WIN`(`usize::MAX` 바로 밑 두 개)과도 절대 안 겹치는
자리라서 My Computer 쪽 진짜 FileId 로 여는 창과는 여전히 완전히
별개로 남는다. 혹시라도 이 가짜 id 로 뭔가를 드롭하려 해도, 바로 위
버그에서 추가한 `FileSystem::contains()` 검사가 "진짜 파일 아님"으로
안전하게 걸러준다.

## → 테스트 이미지 교체 + Photos 4 폴더 랜덤 노출

기존에 Photos(?????) 피드에서 쓰던 "폐건물" 사진 394장을 전부
지우고, TestImageMaker 로 만든 자리표시 테스트 이미지 102장(각기
크기가 다른 단색 배경에 라벨 텍스트만 있는 jpg)으로 교체했다.
`assets/photo/` 밑에 `corpseImage/`(12장), `crackImage/`(12장),
`hintImage/`(8장), `normalImage/`(70장) 네 개의 하위 폴더로 나눠
넣었다 — 앞으로 시체/균열/힌트/일반 같은 종류별로 다르게 취급하는
기능이 생길 걸 대비해 폴더 자체를 분류 단위로 잡아뒀다.

**Photos 앱 쪽 변경**: 원래 `PhotosApp::new()` 는 `assets/photo` 
바로 밑의 파일만(하위 폴더는 무시하고) 훑어서 파일명 알파벳 순으로
정렬한 뒤 앞쪽 `UNLOCKED_PHOTO_COUNT`(10)장만 잘라 보여줬다. 이
방식 그대로 두면 알파벳순 정렬 특성상 `corpseImage1~9` 같은 것들이
`crackImage`/`hintImage`/`normalImage` 보다 항상 먼저 잘려서, 사실상
corpseImage 폴더 사진만 계속 보이는 문제가 생긴다.

그래서 `scan_photos()` 를 새로 추가해 `assets/photo` 바로 밑 파일뿐
아니라 한 단계 아래 하위 폴더(corpseImage 등) 안의 사진까지 전부
모으고, 정렬 대신 파일마다 만든 xorshift64 PRNG(`photos.rs` 전용 —
desktop.rs 의 아이콘 글리치용 PRNG 와 같은 관례로 파일마다 따로 둠)로
Fisher-Yates 셔플을 한 번 한 뒤에 앞쪽 `UNLOCKED_PHOTO_COUNT` 장만
자른다 — 그러면 네 폴더 사진이 골고루 섞여서 뽑히고, Photos 앱을 새로
열 때마다(새로 게임을 시작할 때마다) 조합이 달라진다.

하위 폴더 사진의 "파일명" 식별자는 `"corpseImage/corpseImage1.jpg"`
처럼 `폴더명/파일명` 형태로 만들어서 `AppAction::OpenPhoto`/
`DownloadPhoto`/`FileKind::Photo` 에 그대로 쓴다 — 서로 다른 폴더에
같은 이름의 파일이 있어도 안 겹치고, `PhotoViewerApp` 이 원본을 다시
찾을 때도 이 식별자를 그대로 `assets/photo` 밑에 붙이면(`/` 구분자는
Windows 에서도 그대로 동작) 정확한 하위 폴더 파일을 찾는다. 다만
이 식별자를 그대로 창 제목이나 Explorer/Downloads 탭 이름으로 쓰면
"corpseImage/corpseImage1.jpg" 처럼 폴더 경로가 그대로 노출돼
지저분해 보이므로, `FileSystem::find_or_add_photo()` 와
`DeskAction::OpenPhoto` 핸들러 양쪽에서 마지막 `/` 뒤쪽(진짜
파일명)만 잘라내 화면에 보여주는 이름으로 쓰도록 고쳤다 — 내부
식별자와 화면 표시 이름을 분리한 것.

## → 받은편지함 초기화 + STORY.md 프롤로그 "입사 안내 메일" 도입(HexTool 첨부)

지금까지 있던 더미 테스트 메일 두 통(Test Message #1/#2, 첨부 없음)을
걷어내고, `STORY.md` 프롤로그에 있던 진짜 스토리 메일 — 새로 입사한
연구원에게 PALLAS OS 재연구(이미지 검수) 업무를 배정한다는 내용 —
을 넣었다. 사용자가 준 원문에서 마크다운 헤더(`###`)와 오타
("뽑아주십시요"/"보내주십시요" → "뽑아주십시오"/"보내주십시오")만
정리했고, 나머지 문장은 그대로 살렸다. 다른 UI 문구와 마찬가지로
`tr()` 로 영어/한국어/일본어 세 버전을 다 채웠다(발신자
`hr@pallascorp.local`/수신자 주소는 언어와 무관해서 고정).

**도착 타이밍**: 예전에 만들어뒀지만 꺼둔 채로 있던
`MAIL_AUTO_ARRIVE`/`MAIL_ARRIVAL_DELAY` 타이머를 다시 켰다(desktop.rs) —
`MAIL_AUTO_ARRIVE = true`, `MAIL_ARRIVAL_DELAY = 5.0` 로 맞춰서 새
게임을 시작하고 5초 뒤에 이 메일 한 통이 도착한다. `FileSystem::new()`
의 `mail_arrived` 기본값도 `true`(테스트 편의용으로 즉시 도착하게 해뒀던
값)에서 `false` 로 되돌렸다 — 이제 진짜로 "메일이 아직 안 왔다가 잠시 뒤
도착"하는 연출이 살아난다. 도착 토스트(우측 하단 알림)의 발신자/제목
문구도 예전 테스트용 문구("PalaceCompany@email.com" / "Photo QA
Request")에서 새 메일 내용에 맞게 바꾸고, 하드코딩된 한국어 대신
`tr()` 로 설정 언어를 따라가게 고쳤다.

**HexTool 첨부**: 이 메일에 HexTool 을 바로 첨부해서 보낸다 —
원래 스토리대로라면 Photos.tar → Installer 마법사를 거쳐야 HexTool 이
생기지만, 지금은 그 중간 단계 없이 이 메일이 곧장 쥐여주는 쪽으로
단순화했다(다음에 만들 "이미지 검수" 체크리스트 기능을 바로 테스트할
수 있게). 다만 메일 본문(`apps/mail.rs::seed_messages()`)은 `fs` 를
들고 있지 않아서 첨부에 쓸 `FileId` 를 스스로 만들 수 없는 문제가
있었다 — `FileSystem::new()` 가 미리 `FileKind::HexTool` 노드를 하나
만들어서 새 필드 `fs.mail_hextool_attachment` 에 박아두고,
`apps/mod.rs::open()` 의 `FileKind::Mail` 분기가 이 값을 그대로
`MailApp::new()` 에 넘겨주는 식으로 풀었다(`seed_messages()` 도
`hextool_id: FileId` 파라미터를 새로 받는다 — 언어가 바뀌어 메시지를
다시 만들 때도 `MailApp` 이 이 값을 필드로 들고 있다가 다시 넘겨준다).
이 노드는 바탕화면/Downloads 어디에도 안 걸려있다가, 메일 첨부칸의
"Download" 버튼을 눌러야 비로소 `fs.download()` 를 통해 Downloads
탭에 나타난다(다른 첨부파일 다운로드와 완전히 같은 경로라 별도
분기가 필요 없었다).

**세이브 파일 정리**: 이 구조 변경으로 `mail_arrived: true` 가 이미
저장된 기존 세이브 3개(`production/director/`, `target/debug/`,
`target/release/` 의 `palaceos_save.json`)를 불러오면 새 메일이
처음부터 도착해있는 것처럼 보이고, `mail_hextool_attachment` 필드가
없어서(`#[serde(default)]` 로 0 채워짐) 첨부가 엉뚱한 노드(id 0 =
My Computer)를 가리키게 되는 문제가 있었다 — 크래시는 안 나지만
분명히 잘못된 상태라, 사용자 확인을 받고 세 파일 다 지웠다.

## → 메일 첨부파일이 본문 텍스트랑 같이 스크롤되도록 수정

첨부파일이 있는 메일(Inbox 든 Sent Items 든)을 읽을 때, 첨부 박스가
본문 스크롤 영역 밖에 따로 잘려나가 창 맨 아래에 항상 고정돼 있었다
(`attach_h` 만큼 `body_area` 높이를 미리 빼두고, 그 자리에 박스를
`content.y + content.h - attach_h + 4.0` 로 고정 배치하던 방식). 본문이
길어서 스크롤해야 하는 메일에서는 첨부 박스가 화면 하단에 늘 떠 있는
게 오히려 "본문의 일부"가 아니라 "창의 UI 장식"처럼 보이는 어색함이
있었다.

첨부 박스를 body_area 전체 높이(더 이상 attach_h 를 안 뺀다) 안에서,
본문 텍스트의 마지막 줄 바로 다음에 오는 "가상의 줄"로 취급하도록
고쳤다. 스크롤 단위 자체는 그대로 "줄 수"(line 개수)를 쓰지만,
첨부가 있으면 `(ATTACH_BOX_H + ATTACH_GAP) / LINE_H` 만큼을 소수
줄 수로 더해서 `max_body_scroll`/스크롤바 비율(`frac`) 계산에
반영한다 — 그래야 마지막 줄까지 다 스크롤해야 첨부 박스도 끝까지
보인다. 박스의 실제 화면 y 좌표는 텍스트 한 줄 그릴 때 쓰는 것과
똑같은 변환식(`text_area.y + i*LINE_H - line_off`, i=줄 번호)을 그대로
써서 `i = lines.len()`(마지막 줄 바로 다음)에 대입해 구했고, 텍스트를
그리던 `r.set_clip(Some(text_area))` 구간 안에서 같이 그리도록
옮겨서 스크롤에 따라 자연스럽게 잘려 보이게(clip) 했다.

Inbox 쪽엔 다운로드 버튼과 진행 타이머(`self.downloading[msg_idx]`)가
있는데, 박스가 화면 밖으로 스크롤됐다고 타이머가 멈추면 "눌러놓고
다른 부분을 보러 스크롤했더니 다운로드가 멈췄다"는 이상한 동작이
되므로, 타이머 갱신 로직은 박스가 화면에 보이는지와 무관하게 항상
실행하고 실제 그리기(아이콘/버튼/스피너)만 `box_y`가 text_area 범위
안에 들어올 때만 하도록 분리했다.

## → 첨부파일 Download 버튼이 스크롤바랑 겹치는 문제 수정

바로 위 변경으로 첨부 박스가 스크롤 영역 안으로 들어오면서, 박스
폭을 예전 그대로 `content.w - 16.0`(거의 본문 상자 전체 폭)으로
두고 있었다는 게 스크린샷으로 드러났다 — 본문 텍스트는 오른쪽에
스크롤바(SB_W=8px) 자리를 미리 비워두고 그리는데(`text_w`가
`body_area.w - 20.0 - SB_W`), 첨부 박스는 그 자리를 안 비워서 폭이
더 넓었고, 그 결과 오른쪽 끝의 Download 버튼이 스크롤바와 겹쳐
보였다. Inbox/Sent Items 둘 다 `box_w` 계산에 `- SB_W - 4.0` 을
더해서 스크롤바 자리(+여유 4px)를 항상 비워두도록 고쳤다 — 스크롤이
필요 없어서 스크롤바가 안 보일 때도 폭을 똑같이 좁혀서, 스크롤
가능 여부에 따라 버튼 위치가 오락가락하지 않게 했다.

## → 일본어 메일 본문 깨짐 + 최소화된 창의 작업표시줄 언어 고정 버그 수정

**1) PALLAS OS 메일 일본어 본문의 두부(tofu) 깨짐**: 새로 넣은
"PALLAS OS 재연구 업무 배정 안내" 메일의 일본어 본문/제목에 실제
쓰인 한자 중 82자(事会似体例修傷処判別割加務勤危可合含告員問困囲在
報安実害審対念提損撃攻料断映期査検業様歓殿活無物現生的皆直研社祈移
究等範級結維覧討証該説調識象貴資迎近遂部閲険難題類 등)가
`gfx.rs::build_font_atlas()` 의 `KANJI_CHARSET`(그때그때 실제로 쓰는
한자만 수동으로 골라 담는 방식)에 없어서 전부 두부(마름모+물음표)로
보였다 — 한자를 몇 개 더 추가하는 게 아니라 통째로 새 문단(체감상
메일 하나 분량)이 늘어난 거라 빠진 게 특히 많았다. 코드 전체를
훑어 `tr()` 세 번째 인자(일본어)에 실제로 쓰인 한자를
`0x4E00~0x9FFF`(CJK 통합 한자) 범위에서 뽑아 `KANJI_CHARSET`에 이미
있는 것과 비교하는 파이썬 스크립트로 빠진 글자를 전부 찾아내
`KANJI_CHARSET`에 이어붙였다 — 지금은 이 메일에서만 쓰인 한자들이지만
스크립트 자체는 앞으로도 재사용할 수 있게 남겨둔다(scratchpad).

**2) 최소화한 창을 최소화한 채로 언어를 바꾸면 작업표시줄 글자가 안
바뀜**: `window_manager.rs`의 창 갱신 루프가 `WinState::Minimized`
인 창은 `continue`로 건너뛰는데, 언어가 바뀔 때마다 창 제목을
다시 읽어오는 `if let Some(t) = self.windows[i].app.title() { ... }`
갱신 코드가 하필 그 `continue`보다 아래(그리기/업데이트 블록 다음)에
있었다 — 그래서 최소화된 창은 제목 갱신 자체가 통째로 스킵됐다.
최소화된 창도 화면엔 안 보이지만 작업표시줄 버튼에 그 제목 글자가
그대로 남아있으므로, 최소화 상태로 언어를 바꾸면(스크린샷 제보:
"내 컴퓨터"/"휴지통" 창을 최소화해둔 채 설정에서 일본어 → 한국어로
바꿨더니 작업표시줄 버튼만 "マイコンピュータ"/"ごみ箱"로 그대로 남아
있었음) 그 창을 다시 열기 전까진 작업표시줄 글자만 예전 언어로 멈춰
있었다. 제목 갱신 코드를 `Minimized` 여부를 검사하는 `continue`
바로 위로 옮겨서, 창을 실제로 그리든 안 그리든 제목만은 항상 매
프레임 갱신되도록 고쳤다.

## → HexTool: 메일 첨부를 설치 마법사로, 검토 대상을 이미지/mp4 파일로, 체크리스트 제거

세 가지를 한꺼번에 손봤다.

**1) 메일 첨부가 HexTool 자체 대신 설치 마법사**: 지금까지 "PALLAS OS
재연구 업무 배정 안내" 메일은 완성된 `FileKind::HexTool` 을 곧장
쥐여줬는데, 이제 원래 있던 설치 마법사 흐름을 그대로 태운다.
`foundation.rs::FileSystem::new()` 가 만들어두는
`fs.mail_hextool_attachment` 노드를 `FileKind::HexTool` 대신
`FileKind::Installer`("HexTool Setup.exe")로 바꿨다 — 메일 첨부를
다운로드해서 실행하면 `installer.rs` 의 설치 마법사가 뜨고, Finish
까지 마쳐야(`AppAction::InstallComplete`) 바탕화면에 진짜 HexTool
아이콘이 생긴다. 이 마법사/`AppAction::InstallComplete` 처리 자체는
Chapter 1 콘텐츠였다가 걷어냈던 걸 그대로 재사용한 것이라 새로 만든
코드는 없다.

**2) HexTool 검토 대상이 .tar 대신 실제 이미지/mp4 파일**: `apps/
hextool.rs` 의 파일 선택 화면(`Page::SelectTar` → `Page::SelectFile`
로 이름도 바꿈)이 이제 `fs.all_of_kind()` 로 `FileKind::Photo`(Photos
앱에서 다운로드한 사진)/`FileKind::Img`/`FileKind::Mp4` 인 파일을
전부 훑어서 목록에 올린다(휴지통에 있는 건 Explorer 의 Videos/Images
가상 탭과 같은 요령으로 뺀다) — `apps/mod.rs`의 `FileKind::HexTool`
분기에서 `fs.all_of_kind(...).map(|id| (id, name, icon_of(...)))` 로
만들어 넘긴다. 목록 각 줄의 아이콘도 예전엔 전부 `IconType::Tar`
고정이었는데, 이제 실제 파일 종류에 맞는 아이콘(`icon_of()`)을
그대로 쓴다.

**3) 하단 이상현상 체크리스트 제거**: `ANOMALY_TYPES` 체크박스
목록(사진마다 이상현상 종류를 하나 골라 제출하던 채점 UI)과 그걸
지탱하던 `ITEMS`/`FLAGGED_INDEX`/`correct_answer`/`found_flagged`/
`Page::Result` 를 전부 걷어냈다 — 재연구 업무 메일이 말하는 "체크리스트를
작성해 파일로 뽑는" 절차는 나중에 진짜 콘텐츠로 다시 만들 예정이라,
지금은 그 자리를 비워둔다. 대신 예전엔 "여러 장(5장 고정)을 순서대로
넘기며 매번 답을 고르던" 구조였던 걸 "선택한 파일 하나를 밝기/채도
슬라이더로 들여다보다가 Done 누르면 바로 삭제 확인 창으로" 가는 훨씬
단순한 구조로 바꿨다. 밝기/채도로 반전시키면 흐릿한 형체가 드러나는
연출은 그대로 남겨뒀는데, 이제 "정답이 있는 특정 인덱스"가 아니라
파일 ID 를 시드로 한 결정적 의사난수로 "3개 중 1개 꼴"로 나오게 바꿔서
— 채점 없이 그냥 분위기(뭔가 있을 수도 있다는 긴장감)만 남겼다.
DeleteConfirm 화면 아이콘도 `IconType::Tar` 고정 대신 실제로 고른
파일의 아이콘(`self.loaded_icon`)을 보여주도록 고쳤다.

## → HexTool: 파일 선택 화면 없애고 곧장 편집 화면으로, 빈 미리보기에서 바로 선택

바로 위에서 만든 `Page::SelectFile`(작은 목록 창) → `Page::Editor`
(큰 편집 창) 2단계 구조를 다시 걷어내고, 이제 HexTool 을 열면 곧장
편집 화면이 뜬다. `Page::SelectFile` 자체를 지워서 `Page` 는
`Editor`/`DeleteConfirm` 둘만 남았고, `apps/mod.rs`도 창을 처음부터
편집 화면이 다 들어가는 크기(420×320)로 연다 — 예전처럼 작은 목록
창에서 골라야 `AppAction::Resize` 로 커지는 중간 단계가 없어졌다.

처음 열었을 때는 `loaded_id`가 아직 `None`이라 미리보기 자리가
빈 sunken 박스 + "클릭해서 파일 선택" 안내 문구로 뜬다. 그 자리를
클릭하거나, 위에 새로 넣은 "Select..." 버튼을 누르면 새 필드
`picker_open`이 켜지면서 같은 자리에 파일 목록(예전 SelectFile
페이지의 그 목록 그대로, `draw_picker()`로 옮겨 재사용)이 펼쳐진다.
하나를 고르면 그 자리에서 바로 미리보기로 바뀌고(`picker_open =
false`), 이미 파일을 보고 있는 도중에도 미리보기를 다시 클릭하거나
"Select..."를 다시 누르면 언제든 다른 파일로 바꿔 고를 수 있다.
목록이 애초에 비어있으면(검토할 이미지/mp4 파일이 하나도 없음)
빈 미리보기 대신 처음부터 picker 를 펼쳐서 "파일이 없습니다" 안내가
바로 보이게 했다. 아직 파일을 안 골랐을 땐 Done 버튼도(지울 대상이
없으니) Submit 이 비활성일 때와 같은 회색 텍스트로 눌러도 반응 없게
비활성 처리했다.

## → HexTool: Done 버튼 제거 + 미리보기/슬라이더를 좌우 배치로

바로 위 Done 버튼이 사실 하루도 못 갔다 — 이번 요청으로 아예 없앴다.
Done 이 하던 일(다 봤으면 삭제 확인 창 `Page::DeleteConfirm` 으로
넘어가기)은 Done 이 없어지면서 부를 방법이 통째로 사라지므로, 도달할
수 없는 코드를 그대로 남겨두는 대신 `Page` enum(Editor/DeleteConfirm
구분 자체)과 `DeleteConfirm` 처리, 그거에만 쓰이던 `loaded_icon`
필드/`CONFIRM_SIZE`/`draw_wrapped()` 까지 전부 같이 걷어냈다 —
HexTool 은 이제 "골라서 밝기·채도로 들여다보는" 순수 뷰어만 남았고,
검토를 마친 파일을 정리(삭제)하는 절차는 나중에 체크리스트 기능과
함께 다시 설계할 몫으로 남겨뒀다.

레이아웃도 위(파일명+Select 버튼)/미리보기/슬라이더 순서로 세로로
쌓던 것에서, 미리보기는 왼쪽에 크게, 밝기/채도 슬라이더는 오른쪽의
좁은 세로 패널(`PANEL_W=130`, 창 폭의 40% 넘게는 안 커지게 클램프)에
위아래로 쌓는 좌우 2단 구성으로 바꿨다 — Done 버튼이 먹던 하단 자리가
없어진 김에, 미리보기가 세로로 훨씬 커져서 자리표시자를 들여다보기
편해졌다.

## → HexTool 미리보기가 가짜 색 자리표시자 대신 원본 이미지를 그대로 로딩

지금까지 미리보기는 파일 ID 를 시드로 한 의사난수 색 + 노이즈 알갱이
+ (3개 중 1개꼴로) 밝기·채도로 드러나는 흐릿한 형체까지, 진짜 이미지
없이 전부 합성으로 만든 자리표시자였다. 이제 `photos.rs` 가 Photos
피드/PhotoViewerApp 에서 쓰는 것과 같은 지연 디코드 방식(고른 파일이
바뀔 때만 한 번, `load_scaled_texture()` 로 원본 그대로 텍스처에
올림)을 그대로 재사용해서 실제 이미지를 보여준다 — `load_scaled_texture`
를 `pub(crate)` 로 열어서 `hextool.rs` 에서도 가져다 쓸 수 있게 했다.

원본을 찾으려면 assets/photo/ 하위 폴더까지 포함한 전체 식별자
(`corpseImage/corpseImage1.jpg` 같은 `FileKind::Photo` 안의 문자열)가
필요한데, HexTool 의 파일 목록에는 지금까지 Explorer 에 보이는 표시용
이름(하위 폴더 없는 파일명만)만 담겨 있었다 — `apps/mod.rs`의
`FileKind::HexTool` 분기에서 목록 튜플에 4번째 값으로 이 식별자를
같이 담아 넘기도록 고쳤다(Img/Mp4 는 아직 실제 파일이 없어서 `None`,
그런 항목을 고르면 "이 파일 형식은 미리볼 수 없습니다" 안내만 뜬다).

밝기/채도는 이 렌더러에 셰이더 유니폼이 없어서 진짜 픽셀 단위 보정은
못 한다 — 대신 밝기는 스프라이트를 그릴 때 곱연산 틴트(`[b,b,b,1]`,
1.0 을 넘는 값은 렌더러가 흰색 쪽으로 알아서 잘라내므로 "밝게"도
어느 정도는 먹힌다)로, 채도는 채도가 낮을수록 그 위에 회색 반투명을
덧씌우는 방식으로 "빛바랜" 느낌만 흉내낸다. 진짜 이미지를 쓰게 되면서
"가끔 형체가 숨어있는" 합성 연출은 근거(진짜 이미지엔 그런 게 없다)가
없어져서 같이 걷어냈다 — `base_hue`/`has_hidden_shape`/`hsv_to_rgb`/
`fill_circle_approx` 전부 삭제.

## → HexTool: Select 버튼 제거, 슬라이더 마진 축소, 빈 자리 검은색, 휠 확대/축소 + 미니맵

스크린샷으로 지적받은 것들을 한 번에 손봤다.

**Select 버튼 제거**: 위쪽 "Select..." 버튼을 지웠다 — 원래도 미리보기
자체를 클릭하면 같은 동작(파일 목록 펼치기)을 하던 코드가 이미
있어서, 버튼 없이도 기능은 그대로 남는다. 버튼이 있던 자리를 파일명
표시줄이 전체 폭을 쓰도록 넓혔다.

**슬라이더 사이 마진 축소**: 밝기/채도 슬라이더 사이 간격
`SLIDER_ROW_H` 를 48 → 40 으로 줄였다.

**빈 자리 검은색**: 미리보기에서 원본 종횡비 때문에 안 채워지는
레터박스 여백이 예전엔 `sunken()` 배경(회색 계열)이 그대로 비쳐
보였는데, 이제 이미지를 그리기 전에 미리보기 안쪽 전체를 검은색으로
먼저 칠한다. 아래 확대 기능과 맞물려서, 확대해서 이미지가 미리보기
자리보다 커진 경우에도(잘려나가는 바깥쪽 없이) 검은 배경 위에 이미지만
보이게 된다.

**호버 위치 기준 휠 확대/축소**: 새 필드 `zoom`(1.0=전체가 다 보이는
배율, 최대 8.0)과 `center`(지금 뷰포트 중심의 이미지 내 정규화 좌표
0..1)로 뷰포트 상태를 들고 있는다. 미리보기 위에 마우스가 있을 때
휠이 들어오면, 휠 굴리기 전 마우스 위치에 대응하는 이미지 좌표를 먼저
구해두고 zoom 을 바꾼 뒤, 그 좌표가 새 zoom 에서도 마우스 위치와 같은
화면 자리에 오도록 `center` 를 역산한다 — 그래서 확대/축소가 항상 커서
지점을 기준으로 일어난다. 실제로 그릴 땐 계산된 위치/크기로 스프라이트를
그리되(zoom 이 크면 미리보기 영역보다 커질 수 있다) `r.set_clip()` 으로
미리보기 안쪽만 잘라 보여준다 — 텍스처 UV 를 직접 잘라내는 대신 클리핑만
으로 확대/이동을 구현한 것.

**미니맵**: 슬라이더 밑 남는 공간에 전체 이미지 축소판을 그리고, 그
위에 지금 보고 있는 영역(뷰포트)을 노란 테두리 상자로 표시한다. 상자
크기/위치는 `view_frac`(뷰포트에 이미지의 몇 %가 보이는지, `draw_preview`
가 매 프레임 계산해서 저장)과 `center` 를 그대로 재사용해서 구한다 —
zoom=1.0(전체 보임)일 땐 상자가 미니맵 전체를 덮고, 확대할수록 작아지며
`center` 를 따라 움직인다.

## → HexTool: 미니맵 정사각형 고정, 좌클릭 드래그 이동, "새로 선택" 링크로 재선택 분리

스크린샷으로 미니맵이 위아래로 길쭉하게(패널 폭 x 남는 세로 공간을
그대로 다 썼었다) 보인다는 지적을 받아 세 가지를 더 손봤다.

**미니맵 1:1 고정**: 패널 폭과(밑에 새로 생긴 "새로 선택" 링크 한 줄
높이를 뺀) 남는 세로 공간 중 더 좁은 쪽에 맞춰 정사각형 한 변을 정하고,
패널 안에서 가로로 가운데 정렬한다.

**좌클릭 드래그로 이동**: 새 필드 `drag_last: Option<(f32,f32)>` 로
지난 프레임 마우스 위치를 들고 있다가, 미리보기 위에서 마우스 왼쪽
버튼을 누른 채 움직이면 그 이동량(화면 픽셀)만큼 반대 방향으로
`center` 를 옮긴다 — 손으로 이미지를 끌어당기는 느낌. 확대(zoom>1)해서
잘려나간 부분을 보고 싶을 때 미니맵 상자를 직접 클릭하는 대신 이미지
위에서 바로 드래그해서 움직일 수 있다.

**"새로 선택" 텍스트 링크로만 재선택**: 지금까지는 이미지를 이미 고른
뒤에도 미리보기 자체를 클릭하면 파일 목록이 다시 펼쳐졌는데, 이제 그
자리(클릭)는 드래그(이동)가 대신 차지한다 — 클릭과 드래그를 픽셀
단위로 구분하기보다, 아예 "이미지를 고른 뒤에는 클릭이 항상 이동
제스처의 시작"이라고 단순하게 정리했다. 대신 미니맵 바로 밑에
"새로 선택"(mail.rs 의 "< 받은편지함으로" 링크와 같은 스타일 — 평소엔
회색, 마우스를 올리면 남색+밑줄) 글자를 새로 넣어서, 이미지를 이미
고른 상태에서 파일 목록을 다시 펼치는 유일한 방법이 되게 했다. 아직
아무 파일도 안 골랐을 때(처음 여는 화면)는 이 링크 대신 예전처럼 빈
미리보기 자체를 클릭하면 바로 목록이 뜬다 — "처음에만" 이라는 요청
그대로.

## → 메일 쓰기: 첨부파일 여러 개 + Inbox 처럼 본문과 같이 스크롤

지금까지 "Write Mail" 은 첨부를 하나만 붙일 수 있었고(`NewMailState.
attachment: Option<...>`), 그 하나도 필드와 본문 사이에 따로 자리를
뺀 고정 "칩" 하나로만 보여줬다. 이번 요청으로 여러 개를 붙일 수 있게
하고, Inbox/Sent Items 읽기 화면에서 이미 만든 "첨부가 본문 마지막
줄 다음에 이어붙어 같이 스크롤되는" 방식을 그대로 재사용했다.

**데이터 구조**: `Option<(FileId,String,IconType)>` 였던 첨부 관련
타입들을 전부 `Vec<...>` 로 바꿨다 — `NewMailState::attachments`,
`SentMailView::attachments`, `foundation.rs::SentMail::attachments`
(세이브 파일에 저장되는 쪽이라 필드 이름이 바뀌어도 예전 세이브가
깨지지 않게 `#[serde(default)]` 를 붙였다 — 예전 필드는 그냥 무시되고
빈 배열로 시작한다), `AppAction::SendNewMail`/`DeskAction::SendNewMail`
의 `attachment: Option<(FileId,String)>` 도 `attachments: Vec<...>`
로. Inbox 쪽 시드 메일(`MailMsg`)은 지금도 첨부가 최대 하나뿐이라
그대로 뒀다 — 이번 요청은 "메일 쓰기" 한정이라 범위를 안 넓혔다.

**여러 개 첨부**: "Attach..." 목록에서 파일을 고르면 예전처럼 목록이
바로 닫히는 대신 계속 열려 있어서 여러 개를 이어서 고를 수 있다. 이미
붙인 파일은 목록에서 옅은 초록 배경 + 초록 글자로 표시되고, 다시
누르면 그 자리에서 뗀다(토글) — 같은 파일을 두 번 붙이는 중복도
자연히 막힌다.

**본문과 같이 스크롤**: 필드와 본문 사이에 있던 고정 첨부 "칩" UI를
완전히 없애고, 대신 본문 텍스트 마지막 줄 바로 다음부터 첨부 개수만큼
박스를 이어 붙인다 — Inbox 읽기 화면과 똑같은 수식(`text_area.y +
(lines.len() + attach_unit_lines*i)*LINE_H + ATTACH_GAP - line_off`)
을 인덱스 `i` 로 일반화해서 여러 개를 순서대로 쌓았다. 각 박스에는
그 자리에서 바로 뗄 수 있는 "Remove" 버튼이 있다(Inbox 의 Download
버튼이 있던 자리와 같은 감각). 스크롤 가능 범위(`max_body_scroll`)와
스크롤바 비율도 첨부 개수만큼 늘어난 가상 줄 수(`total_lines`)를
반영하도록 고쳤다.

## → 메일 쓰기 첨부 박스 사이 마진 축소

스크린샷으로 첨부를 여러 개 붙였을 때 박스끼리(그리고 본문 마지막
줄과 첫 박스 사이) 간격이 너무 넓어 보인다는 지적을 받았다. 원인은
compose 전용 `ATTACH_GAP` 상수가 8.0 이었던 것 — Inbox/Sent Items
읽기 화면 쪽 `ATTACH_GAP`(10.0, 박스 높이도 더 큰 38.0)과 맞춰뒀던
값을 그대로 가져다 썼는데, compose 쪽은 박스 높이가 더 작아서(26.0)
그 비율로는 상대적으로 더 헐렁해 보였다. `draw_new_compose()`의
`ATTACH_GAP` 만 2.0 으로 줄여서 첨부 박스들이 서로 거의 붙어 보이게
했다 — Inbox/Sent Items 쪽 간격은 이번 지적 대상이 아니라 그대로
뒀다.

## → HexTool 사용성 개선 (기능은 그대로, 다루기 편하게)

기능 자체를 바꾸지 않는 선에서 실제로 써보면 불편했을 부분 세 곳을
손봤다.

**파일 목록에 스크롤 추가**: `draw_picker()`(빈 미리보기를 클릭하거나
"새로 선택"을 눌렀을 때 뜨는 파일 목록)가 지금까지 넘치는 항목을
그냥 잘라버리고 스크롤할 방법이 아예 없었다 — 목록 높이보다 파일이
많으면(Photos 로 사진을 여러 장 받아둔 상태라면 충분히 있을 수 있는
일) 뒤쪽 파일은 영영 못 골랐다. `picker_scroll`/`picker_scroll_disp`/
`picker_sb_drag` 필드를 추가해서 휠 스크롤 + 오른쪽 스크롤바
(`widgets::scrollbar`, 설정의 "부드러운 스크롤" 값도 그대로 반영)로
넘겨볼 수 있게 했다.

**확대 배율 표시 + 원클릭 초기화**: 지금까지 확대/이동한 뷰를 원래대로
되돌리는 유일한 방법이 "새로 선택" → 같은 파일을 다시 클릭(그러면
zoom/center 가 초기화되는 부수효과를 이용)하는 우회로였다. 패널
맨 위에 "확대: 140%" 처럼 지금 배율을 보여주는 줄을 새로 넣고, 기본
배율(zoom=1.0, center=중앙)이 아닐 때 그 글자를 누르면 바로 zoom/center
를 되돌리게 했다 — mail.rs 의 "< 받은편지함으로" 링크와 같은 hover
스타일(기본은 회색, 원래대로 되돌릴 게 있을 때만 진한 회색+hover 시
남색·밑줄)이라 지금 뭔가 바뀐 상태인지 아닌지도 한눈에 보인다.

**조작법 힌트**: 처음 이미지를 열면 휠로 확대/축소하고 드래그로
이동할 수 있다는 게 화면 어디에도 안 쓰여 있어서, 시도해보지 않고는
알 방법이 없었다. 미리보기 왼쪽 아래에 "휠로 확대/축소, 드래그로
이동" 문구를 작게 넣었는데, 아직 한 번도 확대/이동을 안 써본 동안
(`zoom == MIN_ZOOM`)만 보이고 한 번이라도 조작하면 사라진다 — 계속
떠 있으면 오히려 이미지를 가려서 방해가 되니, 배운 뒤엔 필요 없는
문구다. 사진 배경이 밝든 어둡든 읽히도록 그림자를 한 번 더 깔았다
(photos.rs 의 Download 글자와 같은 요령).

## → ????? 사진을 조건부로 갱신 + 보고 메일 발송 시스템 + Mail/HexTool 선택 목록 라이브 새로고침

세 가지를 함께 손봤다 — 서로 얽혀 있어서 한 번에 정리한다.

**1) ????? 가 더는 완전 랜덤이 아니다**: 지금까지 `PhotosApp::new()`
는 열 때마다 assets/photo 전체를 다시 훑어 매번 새로 셔플했다. 이제
`foundation.rs::FileSystem` 에 `photos_current`(지금 피드에 떠 있는
사진들의 식별자)와 `photos_seen`(지금까지 한 번이라도 나왔던 식별자
전부)을 추가해서, 한 번 정해진 `photos_current` 는 명시적으로 갱신을
부르기 전까진 절대 안 바뀐다 — 같은 사진을 몇 번을 열어봐도 항상 같다.
`photos.rs` 의 랜덤 셔플 로직은 `pick_new_photos(exclude)` 로 뽑아내서
`ensure_photos_selected(fs)`(비어있을 때만 처음 한 번 뽑음 — 새 게임/
예전 저장 파일 둘 다 대응)와 `refresh_photos_feed(fs)`(무조건 새로
뽑되 `photos_seen` 은 전부 제외) 양쪽에서 재사용한다.
`PhotosApp::new()` 자체는 이제 셔플을 안 하고 `fs.photos_current` 를
그대로 받아서 디스크 경로만 다시 구한다. `ensure_photos_selected` 는
`DesktopScene::new()` 에서 창이 열리기 전에 미리 호출해둔다(값을 만드는
데 `&mut FileSystem` 이 필요한데, `apps::open()` 은 `&FileSystem` 만
받아서 여는 시점엔 못 하기 때문).

**2) 보고 메일을 보내야 갱신된다**: "특정 조건"이 정확히 뭔지는
재연구 업무 메일 본문에 이미 있었다 — 이상 현상이 있는 사진을
회사 이메일로 보내는 것. `desktop.rs` 에 `REPORT_EMAIL =
"hr@pallascorp.local"`(입사 안내 메일을 보낸 그 주소) 상수를 두고,
`DeskAction::SendNewMail` 처리에서 받는 사람이 그 주소이고 첨부 중에
`FileKind::Photo` 이면서 식별자가 `"normalImage/"` 로 시작하지 않는
것(=이상 현상이 있다고 분류된 corpse/crack/hint 사진)이 하나라도
있으면 `refresh_photos_feed()` 를 부른다 — normalImage 만 보내면
조건을 안 채운 것으로 친다(요청 그대로). 이 흐름을 타면 ????? 가 지금
열려있어도 바로 반영되도록 `refresh_photos_if_open()` 도 새로 만들어
같이 불렀다(스크롤 위치는 콘텐츠가 통째로 바뀌니 초기화돼도 자연스럽다
고 보고 전체 재구성 방식을 그대로 썼다 — Recycle Bin 새로고침과 같은
요령).

**3) Mail/HexTool 의 파일 선택 목록이 이제 라이브로 갱신된다**: 지금까지
`MailApp.attachable`/`HexToolApp.review_files` 는 그 창을 여는 시점의
`fs` 스냅샷으로 딱 한 번만 만들어지고 그 뒤로 절대 안 바뀌었다 —
Mail 이나 HexTool 을 이미 열어둔 채로 다른 창(Photos, 다른 Mail
첨부 다운로드 등)에서 파일을 받으면, 그 파일은 이미 열려있는 선택
목록에 영원히 안 나타나고 창을 닫았다 다시 열어야만 보였다. 목록을
만드는 계산 자체를 `apps/mod.rs::mail_attachable_files()`/
`hextool_review_files()` 로 함수화해서 `open()` 과 새로고침 양쪽에서
재사용하게 하고, `MailApp::refresh_attachable()`/`HexToolApp::
refresh_review_files()` 를 새로 추가해서 desktop.rs 가 목록만
그 자리에서 바꿔치기할 수 있게 했다(`refresh_mail_if_open()` 처럼
앱을 통째로 새로 만드는 방식은 안 썼다 — Mail 은 "Write Mail" 에
작성 중이던 초안이, HexTool 은 지금 보고 있는 미리보기/확대/슬라이더
상태가 통째로 날아가 버리기 때문). `refresh_mail_attachable_if_open()`
/`refresh_hextool_if_open()` 를 다운로드(`Download`/`DownloadPhoto`)
뿐 아니라 삭제(`DeletePermanently`/`EmptyTrash`)·이동(`MoveFiles`)·
복구(`Restore`) 등 `refresh_explorer_if_open()` 이 이미 불리던 모든
자리에 같이 넣어서, Explorer 뿐 아니라 이 두 선택 목록도 항상 최신
상태를 보장하도록 맞췄다.

## → 버그 수정: 일부 컴퓨터에서 CRT 셰이더가 격자 무늬로 깨지는 문제

**증상**: 스크린샷 제보 — 화면 전체에 미세한 격자/무아레 잡음이 낀
것처럼 보이고, 설정에서 CRT 강도(intensity)를 낮추면 괜찮아짐. 특정
컴퓨터에서만 재현됨.

**원인**: `crt.rs` 의 CRT 프래그먼트 셰이더가 스캔라인/새도우 마스크를
`sin(warped.y * tex_size.y * 3.14159 - time * 2.0)` 처럼, 화면
해상도(`tex_size`, 보통 수백 단위) 를 그대로 곱한 큰 값을 삼각함수에
넣어서 계산한다. 그런데 셰이더 맨 위에 `precision mediump float;`
가 박혀 있었고, `varying` 로 넘어오는 `uv` 도 `lowp` 로 선언돼
있었다 — GLSL ES 스펙상 mediump 는 **상대** 정밀도만 보장하고(대략
2^-10, 1024분의 1) 그 이상은 GPU/드라이버 재량이라, 이렇게 몇 백~몇
천 단위까지 커지는 값을 mediump 로 계산하면 `sin()` 결과가 GPU에
따라 완전히 어긋난다 — 그 결과가 바로 스캔라인 대신 보이는 불규칙한
격자 잡음이다. intensity 를 낮추면 이 깨진 항에 곱해지는
`mix(1.0, ..., intensity)` 비율이 줄어들어서 증상이 옅어지는 것도
정확히 들어맞는다. 데스크톱 GPU 자체(주사율/LED 구조)의 문제가
아니라, 그래픽 드라이버가 `mediump` 를 실제로 몇 비트로 구현했는지
차이에서 오는 셰이더 정밀도 버그였다.

**고친 내용**: 프래그먼트 셰이더의 기본 정밀도를 `mediump` →
`highp` 로 올렸다. 데스크톱 GPU 에서 highp 는 사실상 항상 완전한
32비트 float 라 이런 정밀도 문제에서 안전하다. `varying vec2 uv` 도
`lowp` → `highp` 로 같이 올렸다 — 정점/프래그먼트 셰이더 양쪽의
varying precision 은 반드시 일치해야 하는 GLSL ES 규칙이라, 정점
셰이더(`CRT_VS`) 의 선언도 함께 고쳤다(안 그러면 uv 보간 자체가
이미 낮은 정밀도로 시작해서, sin() 인자를 아무리 highp 로 계산해도
소용없다). 셰이더는 런타임에 GPU 드라이버가 컴파일하는 GLSL 문자열
이라 `cargo build`/clippy 로는 검증이 안 되고, "게임 실행 파일을
띄워서 확인하지 말라"는 지침 때문에 실제 렌더링 결과로 재확인은 못
했다 — 문법은 유효한 GLSL ES 1.00 이고 원인-증상(intensity 를
낮추면 나아짐)이 정확히 들어맞아 이 진단에 확신은 있지만, 실제
화면에서 격자 잡음이 사라졌는지는 사용자 쪽에서 확인이 필요하다.

## → 메일 본문의 "PALLAS OS" 를 "PALACE OS" 로

`src/apps/mail.rs` 의 재연구 업무 배정 메일에 나오는 "PALLAS OS"
10곳(제목/본문, 영어·한국어·일본어 세 언어 전부)을 전부 "PALACE OS"
로 바꿨다 — 요청 범위가 "메일에 적힌 것"이라 딱 이 파일만 고쳤다.
`STORY.md`/`ROADMAP.md` 에는 여전히 "PALLAS OS" 로 남아있다 — 스토리
전체에서 개체명을 통일하고 싶으면 그쪽도 같이 바꿔야 한다는 점은
알아두면 좋을 것 같다.

## → 버그 수정: 일본어 문장 중간의 라틴 단어 사이 공백이 강제 줄바꿈처럼 작동

**증상**: 스크린샷 제보 — 메일 본문(일본어)에서 "PALACE OS" 가
"PALACE" 는 한 줄 끝에, "OS" 는 다음 줄 맨 앞에 따로 떨어져 보임.
사용자가 정확히 "스페이스바가 엔터로 작동하는 것 같다"고 원인을
짚었다.

**원인**: `ui.rs::split_line_once()`(모든 여러 줄 안내문 줄바꿈이
거치는 `wrap_lines`/`wrap_with_offsets` 의 핵심)가 지금까지 "문자열에
공백이 하나라도 있으면" 무조건 공백 기준으로 먼저 접었다 — 영어
문장처럼 공백이 여러 개 고르게 있는 텍스트에서는 이게 정상적인
단어 단위 줄바꿈이지만, 일본어는 원래 띄어쓰기가 없는 언어라 문장에
공백이 딱 하나(예: "PALACE OS" 사이)뿐인 경우가 있다. 이 경우 그
공백 하나를 기준으로 "PALACE" 앞부분 전체를 "첫 번째 단어"로
묶어버려서, 실제로는 그 줄이 아직 폭에 여유가 있어 줄이 안 넘칠
자리인데도 그 공백에서 무조건 끊어버렸다 — 정확히 "공백이 엔터처럼
작동"하는 것으로 보이는 증상.

**고친 내용**: 순서를 뒤집었다 — 먼저 글자 단위로(폭 기준) 실제로
줄이 넘치는 지점(`split_byte`)을 찾고, 그 안에 공백이 있으면(그리고
그 공백이 접두사의 앞쪽 절반보다 뒤쪽에 있어서 끊어도 첫 줄이 너무
짧아지지 않으면) 라틴 단어가 중간에 안 잘리도록 그 공백에서 끊는다.
공백이 실제로 그 줄이 넘치는 지점 근처의 "평범한 단어 경계"일 때만
쓰이므로, 영어 문장의 정상적인 단어 단위 줄바꿈은 그대로 유지되고,
일본어 문장 안의 어쩌다 하나 있는 공백이 엉뚱한 자리에서 강제로
줄을 끊는 일은 없어진다. `wrap_lines`/`wrap_with_offsets` 를 쓰는
Mail 본문/설치 마법사 안내문/휴지통·압축파일 안내문 등 여러 줄
안내문 전체에 공통으로 적용되는 수정이라, 스크린샷에 같이 있던
"일부 글자가 삐져나가 보이는" 증상(강제로 잘못 끊긴 줄이 다음
줄로 밀리면서 생기던 부수 현상일 가능성이 높다)도 같이 없어질
것으로 보인다.

## → 버그 수정: 버튼 글자가 언어별로 폭 밖으로 삐져나가는 문제 (진짜 "글자 잘림" 원인)

**재확인**: 같은 스크린샷으로 다시 제보를 받았는데, 코드를 다시
추적해보니 "PALACE"/"OS" 줄바꿈 지점 자체는 지금 본문 폭 기준으로
실제로 거기서 넘치는 게 맞았다(위에서 고친 `split_line_once()` 가
글자 폭을 정확히 계산해서 낸 결과) — 즉 그 특정 지점은 버그가
아니라 지금 Mail 창 폭에서 그 문장이 실제로 다 안 들어가는 것이었다.
다만 "다운로드" 스크린샷 쪽을 다시 파보니 그건 확실한 별개의
진짜 버그였다.

**원인**: `ui.rs::raw_button()`(모든 `button()` 호출의 실제 구현)과
`recycle_bin.rs::sel_button()` 가 라벨 텍스트를 항상 가운데 정렬로
그리기만 하고 **한 번도 폭으로 잘라내지(clip) 않았다**. 호출부들이
버튼 폭을 영어 라벨 기준으로 대충 고정값(예: Mail 첨부 박스의 Download
버튼은 `90.0`)으로 잡아뒀는데, 한국어("다운로드"/"다운로드 중")나
일본어("ダウンロード中") 번역이 그보다 넓으면 가운데 정렬된 글자가
버튼 테두리 밖으로 그대로 삐져나와 옆의 스크롤바/다른 UI와 겹쳐
보였다 — 스크린샷의 "다운로드" 뒤에 붙어있던 알록달록한 잔상이 바로
그 삐져나온 글자+CRT 색수차였다.

**고친 내용**: 두 겹으로 고쳤다.
1. Mail 첨부 박스의 Download/Downloading/Downloaded 버튼 폭을 고정
   `90.0` 대신, 세 상태 각각 그 언어에서 실제로 필요한 폭(`r.text_width()`
   로 직접 계산)중 가장 넓은 값에 맞추도록 바꿨다 — 이제 언어가 뭐든,
   상태가 어떻든 버튼이 항상 자기 글자를 다 담을 만큼 넓다.
2. 그래도 다른 곳에서 또 비슷하게 폭을 안 넉넉히 잡는 실수가 있을 수
   있으니, `raw_button()`/`sel_button()` 자체에 안전장치를 넣었다 —
   글자가 원래 폭에 다 들어가면 그대로 가운데 정렬하고, 안 들어가면
   왼쪽 여백까지 당긴 뒤 오른쪽 끝에서 잘라(`text_clipped`) 최소한
   버튼 밖으로 새지는 않게 막는다. `button()` 을 쓰는 앱 전체(HexTool/
   Installer/Archive/Credits/Password/Recycle Bin 등)에 한 번에
   적용되는 마지막 방어선이라, 개별 호출부의 폭 계산을 일일이 다
   찾아 고치지 않아도 최소한 "화면 밖으로 새는" 것만은 항상 막힌다.

## → 재연구 업무 배정 메일 본문을 새 문구로 교체

사용자가 준 한국어 문구로 `apps/mail.rs::seed_messages()` 의 메일
본문을 통째로 갈아끼웠다 — 영어/일본어는 그 한국어 문구를 기준으로
같이 번역해서 3개 언어 전부 내용이 맞도록 채웠다(원문 마크다운
굵게(`**...**`) 표시는 이 렌더러가 굵은 글씨를 지원하지 않아서
표시 문자만 지우고 나머지 문단 구성(빈 줄로 구분되는 절)은 그대로
살렸다).

본문 중 "(깨진글자 5개)"는 바탕화면의 Photos 앱과 완전히 같은 키릴
문자열 `ЫЩЁЪЭ`(foundation.rs::FileSystem::new() 가 만드는 그 이름 그대로)
를 그대로 썼다 — 메일이 가리키는 게 실제로 그 앱이라는 걸 분명히
하려고. "(깨진글자 7개)"는 아직 게임에 실제로 등장하지 않는 무언가를
가리키는 새 자리표시자라, 기존 것과 안 겹치는 7자짜리 키릴 문자열
`ЖЦЧШЮЯФ` 를 새로 만들어 넣었다 — 둘 다 폰트 아틀라스에 없는 키릴
문자라 항상 두부(마름모+물음표)로 보인다는 점은 기존 Photos 이름과
같다. 지금 이 두 자리표시자에 실제 기능을 붙이거나 새 FileKind 를
만들지는 않았다 — 이번 요청은 메일 문구 교체뿐이라, 새 문자열은
어디까지나 메일 텍스트 안의 복선일 뿐이다.

**한자 커버리지 재점검**: 새 일본어 본문에 처음 쓰인 한자 12자
(刻/大/小/弊/拡/深/縮/致/被/覚/際/非)가 `gfx.rs::build_font_atlas()`
의 `KANJI_CHARSET` 에 없어서 두부로 깨질 뻔했다 — scratchpad 의
한자 커버리지 검사 스크립트를 다시 돌려서 미리 찾아내 추가했다. 이
중 大/小/拡/縮 은 사실 이번 메일 교체와 무관하게, **바로 이전 턴에서
HexTool 사용성을 개선하며 추가한 일본어 문구("拡大縮小"/"拡大")가
그때 감사를 안 거쳐 이미 깨져 있던 것**을 이번에 스크립트를 다시
돌리다 우연히 함께 잡아낸 것이다 — 그때 놓쳤던 회귀를 이번에 같이
고쳤다.

## → 재연구 업무 메일의 보낸/받는 사람 주소 교체

`apps/mail.rs::seed_messages()` 의 `from`/`to` 를 각각
`hr@pallascorp.local` → `test@mail.com`, `user@pallascorp.local` →
`toast@mail.com` 으로 바꿨다. 이 주소를 참조하던 다른 두 자리도 같이
맞췄다 — `desktop.rs::REPORT_EMAIL`(재연구 업무 보고를 보내면 ?????
피드를 갱신시키는, "메일을 보낸 그 주소로 회신"을 판정하는 상수)과
첫 메일 도착 토스트 알림에 찍히는 발신자 주소. 토스트 쪽 코드를
보다가 제목 문구가 아직 "PALLAS OS"로 남아있던 걸 발견해서(예전
PALLAS→PALACE 개명 때 놓친 자리) 같이 "PALACE OS"로 고쳤고, 혹시
다른 데도 더 있을까 싶어 전체를 다시 훑었지만 이제 남은 곳은 없다.

## → 리팩터: 언어별 UI 문자열을 `src/strings.rs` 로 전부 이전

지금까지 화면 문구는 `tr(lang, "Download", "다운로드", "ダウンロード")`
처럼 en/ko/ja 세 개를 호출부에 직접 박아 넣는 식이었다. 15개 파일에
`tr(lang, ...)` 호출이 146곳 넘게 흩어져 있었는데(직접 세어보기
전까진 몰랐다), 이번 요청으로 전부 `src/strings.rs` 한 파일로
옮기고, `Language` 타입(foundation.rs, 기존에 이미 있었다)에 따라
그 파일에서 문구를 골라오는 구조로 바꿨다.

**설계 — "언어별 배열 3개" 대신 "문구 하나당 상수 하나"**: 요청대로
"언어별 배열 + 언어 타입으로 조회"를 그대로 구현하면(`EN: [&str; N]`,
`KO: [&str; N]`, `JA: [&str; N]` 을 인덱스나 enum variant 순서로
맞추는 방식) 세 배열의 순서가 하나라도 어긋나면 컴파일은 되는데
엉뚱한 언어의 엉뚱한 문구가 나오는 사고가 난다 — 이 프로젝트는 지금
게임을 직접 띄워서 눈으로 검증하지 않고 작업하고 있어서, 그런 조용한
오작동은 특히 위험하다고 판단했다. 그래서 대신 다음 구조를 썼다:

```rust
pub struct S { pub en: &'static str, pub ko: &'static str, pub ja: &'static str }
pub fn t(lang: Language, s: S) -> &'static str { ... }

pub mod mail {
    pub const DOWNLOAD: S = S { en: "Download", ko: "다운로드", ja: "ダウンロード" };
    ...
}
```
호출부는 `tr(lang, "Download", "다운로드", "ダウンロード")` 대신
`t(lang, mail::DOWNLOAD)` 를 쓴다. 세 언어가 항상 상수 하나(`S`)
안에 나란히 있어서 — 배열 인덱스가 어긋날 여지 자체가 없고, 하나만
빠뜨리면(`S` 의 필드 세 개가 전부 필수라) 그 즉시 컴파일 에러가
난다. "화면별로 묶은 배열"이라는 요청의 취지는 파일 이름이 아니라
`mail`/`hextool`/`settings`/`explorer`/`recycle_bin`/`archive`/
`credits`/`installer`/`official_site`/`password`/`video_player`/
`foundation`/`boot`/`lobby`/`desktop` 하위 모듈로 살렸다 — 어느 화면
문구인지 모듈 이름만 보고 바로 알 수 있고, `common` 모듈엔 OK/Cancel/
Download/Restore/Delete/Name/Size/Address/Yes/No 처럼 여러 파일에서
그대로 반복되던 짧은 UI 용어를 모아 하나로 합쳤다(예: File Explorer
와 휴지통이 각자 따로 타이핑하던 "{n} object(s)" 상태바 문구가
완전히 똑같았다 — 이번에 옮기다가 발견).

**작업 방식**: 파일 하나씩 `tr(lang, ...)` 호출을 찾아 `strings.rs`
에 상수로 옮기고 호출부를 `t(lang, ...)` 로 바꾼 뒤, 매번
`cargo build`/clippy 로 그 파일이 깨끗이 컴파일되는지 확인하며
진행했다 — 한 파일이 끝날 때마다 확인해서 실수가 나면 그 파일
범위 안에서 바로 잡을 수 있게 했다. 중간에 `grep "tr(lang,"` 만으로
찾은 15개 파일 목록에 여러 줄에 걸친 `tr(\n lang, ...)` 호출이
있는 파일(`boot.rs` 등)이 빠져있는 걸 발견해서, `\btr\(` 전체
단어 기준으로 다시 훑어 최종적으로 16개 파일을 전부 처리했다.
마지막엔 `foundation::tr()` 함수 자체(이제 아무도 안 씀)를 지우고,
그 존재를 언급하던 각 파일의 주석들도 `t()`/`strings.rs` 기준으로
업데이트했다. 끝나고 한자 커버리지 감사 스크립트를 다시 돌려서
이전 과정에서 문구가 깨지지 않았는지도 확인했다(0건).

**대안이 있냐는 질문에**: 처음엔 "공용 용어만 몇십 개 뽑아서
중앙화하고, 나머지(특히 메일 본문 같은 긴 문단)는 지금처럼 호출부에
그대로 두는" 더 작은 범위를 대안으로 제안했었다 — 146곳을 게임
실행 검증 없이 한 번에 옮기는 작업 자체의 위험성 때문이었다. 요청을
받고 전체 이전으로 진행했고, 결과적으로 파일마다 끊어서 매번
빌드로 확인하는 방식으로 위험을 관리했다. 그래도 남은 위험은 있다
— 옮기는 과정에서 문구 자체(오타, 줄바꿈 등)는 원본을 그대로
복사했지만, 실제 화면에 예전과 똑같이 보이는지는 게임을 직접 띄워
확인해야 확실하다(지침에 따라 이번에도 실행 검증은 안 했다).

## → 안티 리버싱 대비: 스토리 스포일러 상수를 `src/secrets.rs` 로 분리

앞으로 안티 디버깅/안티 리버싱 작업을 외부 개발자에게 맡길 계획이
있어서, "실제로 암호화·난독화해야 할 값"만 따로 격리해달라는 요청을
받았다. 지금 코드를 다 훑어봤을 때 실행 파일에 평문으로 남아있으면
`strings.exe` 한 번만 돌려도 스토리 스포일러가 새어나가는 값은
두 종류뿐이었다:

- `strings.rs::mail::PALACE_MAIL_SUBJECT`/`PALACE_MAIL_BODY` — 재연구
  업무 배정 메일의 제목/본문 전체(스토리 핵심 내용 포함).
- 메일 본문과 `foundation.rs`의 Photos 앱 이름이 공유하는 키릴
  placeholder 문자열 `"ЫЩЁЪЭ"`(5자)와, 메일 본문에만 등장하는 복선용
  placeholder `"ЖЦЧШЮЯФ"`(7자).

`password.rs`의 `.lock` 비밀번호는 검증 로직(`PasswordApp`, `FileKind::Lock`)
만 있고 실제 정답 문자열은 아직 어디에도 하드코딩돼 있지 않다(LOCKED.lock
콘텐츠 자체가 아직 미구현) — 나중에 실제로 만들 때 처음부터
`secrets.rs`에 넣으면 된다. 숨은 번호(hidden numbers) 계열도 아직
코드에 없다.

이 두 종류를 새 파일 `src/secrets.rs`로 옮기고, `strings.rs::mail`에는
"옮겨졌다"는 주석만 남겼다. `foundation.rs::FileSystem::new()`(Photos
앱 이름 생성)와 `scenes/desktop.rs::refresh_photos_if_open()`(같은
이름으로 재검색)는 이제 `secrets::PHOTOS_APP_NAME` 상수를 공유해서
쓴다 — 두 곳이 항상 같은 문자열이어야 하는 제약이 코드로 강제된다.
`apps/mail.rs::seed_messages()`와 `scenes/desktop.rs`의 메일 도착
토스트도 `secrets::PALACE_MAIL_SUBJECT`/`PALACE_MAIL_BODY`를 직접
참조하도록 고쳤다.

`strings.rs`(버튼/라벨 등 일반 UI 문구)와 `secrets.rs`(스토리
스포일러)를 나눈 이유는, 나중에 안티 리버싱 작업자가 "이 파일 하나
안의 상수만 암호화하면 된다"는 걸 코드 구조만 보고 바로 알 수
있게 하기 위해서다 — 400줄 넘는 `strings.rs` 전체를 뒤지며 뭐가
진짜 비밀이고 뭐가 그냥 버튼 텍스트인지 매번 판단할 필요가 없다.
`cargo build`/`cargo clippy` 모두 기존 5개 경고 그대로 통과했고,
`grep`으로 두 placeholder 문자열이 `secrets.rs` 밖에 더 이상
남아있지 않은 것도 확인했다.

## → 리팩터: 하드코딩 제거 1차 작업 (git 저장소 신설 + 색상/매직 문자열/화면 해상도)

"장기적으로 유지보수하기 쉽게 하드코딩을 제거해달라"는 요청을 받았다.
이 프로젝트는 지금까지 git 저장소가 아예 없었어서(이전부터 알려진
사실) 실수해도 되돌릴 안전망이 없었다 — 큰 범위 리팩터를 시작하기
전에 먼저 `git init` 하고 현재 상태를 첫 커밋으로 남겼다(`target/`,
`production/` 은 빌드 산출물이라 `.gitignore` 로 제외). 이후 작업은
전부 별도 커밋으로 나눠서, 중간에 뭔가 잘못돼도 그 직전 커밋으로
돌아갈 수 있게 했다.

"하드코딩"이 뭘 가리키는지 범위가 넓어서, 코드 전체를 grep 으로
훑어 후보를 추려 사용자에게 확인받았다. 이번 1차 작업에서 처리한
세 갈래:

**① 색상 팔레트 중복.** `ui.rs` 에 이미 `BLACK`/`WHITE`/`FACE`/`NAVY`
같은 Win9x 팔레트 상수가 있었는데도, `hextool.rs`/`official_site.rs`/
`video_player.rs`/`photos.rs` 4개 파일이 검은색을 `[0.0, 0.0, 0.0, 1.0]`
리터럴로 따로 타이핑하고 있었다 — 상수를 쓰도록 고쳤다. 또
`foundation.rs::BG_COLORS`(설정 화면의 바탕화면 색 선택지)의 "Teal"/
"Navy" 항목이 `ui.rs::TEAL`/`NAVY` 와 값은 같지만 완전히 별개로 타이핑된
리터럴이었다 — `desktop.rs` 가 배경색 조회 실패 시 `TEAL` 을 기본값으로
쓰기 때문에 이 둘이 어긋나면 조용히 버그가 되는 구조라, `BG_COLORS` 가
`ui::TEAL`/`ui::NAVY` 를 직접 참조하도록 바꿨다.

**② 내부적으로 "이름 자체가 타입 마커"인 fs 노드들.** `FileKind::Folder`
중 이름이 정확히 `"Recycle Bin"` 인 것만 휴지통 취급, `"My Computer"`
인 것만 탐색기 취급, 메일 첨부 파일명이 실제 fs 노드 이름과 정확히
`"HexTool Setup.exe"` 로 일치해야 하는 식으로, 리터럴 문자열 자체가
암묵적인 식별자로 쓰이고 있었다. 이 세 문자열이 `foundation.rs`,
`ui.rs`, `apps/mod.rs`, `apps/explorer.rs`, `apps/recycle_bin.rs`,
`apps/mail.rs`, `scenes/desktop.rs` 등 7개 파일에 걸쳐 총 15곳 넘게
따로 타이핑돼 있었다 — 오타 하나로 매칭이 조용히 깨질 수 있는 위험한
패턴이라, `foundation.rs` 에 `MY_COMPUTER_NAME`/`RECYCLE_BIN_NAME`/
`HEXTOOL_SETUP_EXE_NAME` 상수 세 개를 새로 두고 모든 자리에서 이걸
참조하도록 바꿨다.

**③ 화면 해상도(640×480) 중복.** `main.rs`(실제 게임)와
`src/bin/director.rs`(연출용 별개 실행 파일)가 각자 `const VW: u32 = 640;
const VH: u32 = 480;` 을 독립적으로 들고 있었고, `scenes/desktop.rs`/
`boot.rs`/`lobby.rs`/`bluescreen.rs`/`erase.rs`/`shutdown.rs` 6개 씬
파일에서 화면 전체를 채우거나 가운데 정렬을 계산하는 자리마다
`640.0`/`480.0` 리터럴이 약 35곳 흩어져 있었다. `gfx.rs` 에 정수형
`VIRTUAL_W`/`VIRTUAL_H` 와, 계산에 바로 쓰기 좋은 f32 형
`SCREEN_W`/`SCREEN_H` 를 추가하고 전부 이걸 참조하도록 바꿨다 — 나중에
해상도를 바꿀 일이 생기면 이 두 상수만 고치면 된다. 다만 `desktop.rs`
의 Official Site 창 기본 크기(480.0×380.0)처럼 우연히 같은 숫자일
뿐 화면 크기와 무관한 값들은 그대로 남겨뒀다(잘못 엮으면 오히려
의미가 왜곡된다).

파일마다 고칠 때마다 `cargo build`/`cargo clippy` 로 확인했고(기존
5개 경고 그대로 유지, 새 경고 없음), 별도 커밋 3개(baseline, 색상/
매직문자열, 화면해상도)로 나눠 남겼다. 남은 여지: 앱별로 흩어진
개별 레이아웃 좌표(예: 버튼 위치 90.0, 24.0 등)는 대부분 그 화면
하나에서만 쓰이는 진짜 "그 화면 고유의 값"이라 지금은 안 건드렸다 —
여러 곳에서 반복되며 어긋날 위험이 있는 것들 위주로 먼저 처리했다.

## → HexTool 재작업: "My Computer" 스타일 선택창 + 검수 저장/내보내기 흐름

HexTool의 이미지 선택 방식과 검수 흐름을 다시 설계해달라는 요청을 받았다.
이전 세션에서 시도했던 몇 가지 버전(순차 자동 진행, 설치 마법사풍 로딩
게이지, 3종 체크박스 등)은 전부 되돌리고(`git revert` 세 번), 이번엔
아래 요구사항 그대로 새로 만들었다:

1. HexTool에서 "이미지 선택"을 누르면(또는 빈 미리보기를 클릭하면)
   `apps/hex_picker.rs`의 새 창이 뜬다 — File Explorer("My Computer")와
   똑같은 아이콘 그리드(`widgets::icon_grid` 그대로 재사용)로 지금
   ?????에 떠 있는 사진 전체를 보여주고, 클릭하면 그 자리에서 바로
   골라지고 창이 닫힌다(파일 열기 대화상자처럼).
2. HexTool 오른쪽 패널은 위에서부터: **"N개의 이미지 중 M개 검수됨"**
   상태 → 밝기/채도 슬라이더 → 미니맵 → **"이상현상 있음"** 체크박스 →
   버튼(검수 저장/압축파일 내보내기) 순서로 다시 배치했다.
3. "검수 저장"을 누르면 지금 보고 있는 사진 하나의 체크 여부만
   `fs.photo_reviews`(새 필드, `HashMap<String, bool>`, 저장 파일에
   같이 실림)에 기록한다 — 사진마다 자유롭게 순서 없이 골라 검수할 수
   있고, 게임을 껐다 켜도 진행 상황이 유지된다.
4. ?????의 모든 사진이 검수되면(photo_reviews 와 photos_current 의
   교집합이 photos_current 전체를 덮으면) 버튼이 자동으로
   **"압축파일 내보내기"**로 바뀐다 — 누르면 이상현상으로 체크된
   사진들만 모아 `FileKind::PhotoReport` 압축파일(`Report.zip`)을
   바탕화면에 만든다(재검수해도 새 아이콘이 안 쌓이고 내용만 갱신).
5. 이 압축파일을 재연구 업무 보고 메일(test@mail.com)에 첨부해서
   보내면 ????? 피드가 갱신된다 — 트리거 조건이 "PhotoReport 압축파일이
   첨부됐는지"로 바뀌었다(예전의 "이상현상 사진 직접 첨부" 조건은
   폐기).

구현 메모:
- `icon_grid` 위젯은 항목의 `FileId` 필드를 내부적으로 전혀 안 쓰고
  화면에 보여줄 이름/아이콘, 그리고 클릭된 "인덱스"만 다룬다는 걸
  확인하고, `HexPickerApp` 이 그 인덱스 자리에 실제 FileId 대신 그냥
  배열 인덱스를 채워 넣어 재사용했다 — 새 그리드 위젯을 따로 만들
  필요가 없었다.
- 선택창은 실제 fs 파일이 아니라서 여는 데 `FileId` 가 없다 — Settings/
  Credits 창과 같은 요령으로 `usize::MAX` 근처의 가짜 id(`HEX_PICKER_WIN`)
  를 씀. 사진을 고르면 desktop.rs 가 `WindowManager::close_file()`(이번에
  새로 추가한 메서드)으로 직접 닫아준다 — 그 앱 자신은 `AppAction::Close`
  대신 `SelectPhotoForHexTool` 만 반환하기 때문.
- `AppAction`/`DeskAction` 에 `OpenHexPicker`/`SelectPhotoForHexTool`/
  `SavePhotoReview`/`ExportPhotoReport` 4개를 새로 추가.
- 예전에 HexTool 이 "다운로드된 파일 목록"에서 검토 대상을 고르던
  `apps/mod.rs::hextool_review_files()` 와, fs 변경마다 그 목록을
  실시간 갱신하던 `desktop.rs::refresh_hextool_if_open()` 는 이제
  완전히 불필요해져서 삭제했다 — HexTool 이 더는 다운로드 개념과
  무관하게 ?????의 사진만 직접 다룬다.
- `cargo build`/`cargo clippy` 로 세 실행 파일 모두 확인, 경고는 기존
  5개 그대로. 새 문구(検収を保存/圧縮ファイルを書き出す/画像を選択 등)
  감사에서 "圧"/"書" 두 한자가 빠진 걸 발견해 `gfx.rs::KANJI_CHARSET`
  에 추가했다.

## → HexTool 패널 재구성: 링크로 바뀐 선택 버튼, 체크박스 3종, 스크롤

방금 만든 HexTool 화면에 대한 피드백을 반영해 오른쪽 패널을 다시 다듬었다:

- 창 위쪽에 있던 "이미지 선택" 버튼을 없앴다 — 창을 좁히면 타이틀바
  버튼과 겹쳐 글자가 잘려 보이는 문제가 있었다. 같은 기능을 패널 맨
  위의 "이미지 선택..." 텍스트 링크로 옮겼다.
- 이상현상 체크박스를 하나("이상현상 있음")에서 **시체 / 글리치 /
  이상현상 없음** 세 개로 늘렸다. 서로 배타적으로 동작한다 — 체크하면
  나머지 둘이 자동으로 꺼지고, 이미 골라진 걸 다시 눌러 끄면 "아무것도
  안 고른" 상태(None)로 돌아간다.
- **저장 버튼은 셋 중 하나를 반드시 골라야만 활성화**된다 — 아직 아무
  것도 안 고르면(카테고리가 None) 버튼이 회색으로 눌러도 반응 없는
  상태로 보인다.
- 미니맵을 체크박스 위(밝기/채도 슬라이더 바로 아래)로 옮겼다 — 순서는
  이제 링크 → 검수 현황 → 슬라이더 → 미니맵 → 체크박스 → 버튼.
- 패널 내용이 창 높이보다 길어지면 마우스 휠/스크롤바로 볼 수 있게
  했다 — 각 행의 y 좌표를 상수(LINK_Y/STATUS_Y/SLIDERS_Y/MINIMAP_Y/
  CHECKS_Y/BTN_Y)로 미리 계산해두고, 스크롤 오프셋만큼 통째로 밀어서
  그린다. 스크롤로 가려진 행은 그리지도 입력을 받지도 않는다(안 보이는
  체크박스가 마우스 좌표만 우연히 겹쳐서 몰래 눌리는 걸 막으려고
  visible() 판정을 넣었다 — File Explorer의 파일 목록 스크롤과 같은
  요령).

구현 메모:
- `foundation.rs::AnomalyCategory`(Corpse/Glitch/NoAnomaly) enum을
  새로 만들어 `fs.photo_reviews` 의 값 타입을 `bool` 에서 이걸로
  바꿨다 — "이상현상 없음"도 "아직 검수 안 함"과 구분되는 명시적인
  선택지라는 걸 타입으로 표현했다(전자는 `Some(NoAnomaly)`, 후자는
  `None`). 압축파일에는 `Corpse`/`Glitch` 로 체크된 사진만 담긴다.
- `cargo build`/`cargo clippy` 로 세 실행 파일 모두 확인, 경고는
  기존 5개 그대로. 새 문구(死体 등) 감사에서 "死" 한자가 빠진 걸
  발견해 `gfx.rs::KANJI_CHARSET` 에 추가했다.

## → HexTool 패널 세부 조정: 선택 링크 완전 제거, 그룹박스, 저장 딜레이, 버튼 정렬

바로 전 패널 개편에 대한 추가 피드백을 반영했다:

- 패널 맨 위에 있던 "이미지 선택..." 링크를 완전히 없앴다 — 대신
  "검수 저장"을 누르면 짧게(0.5초) "저장 중..." 표시가 뜬 뒤 미리보기가
  자동으로 빈 자리로 돌아간다. 빈 자리는 원래부터 클릭하면 선택 창이
  뜨는 자리였으니, 저장할 때마다 자연스럽게 다음 사진을 고를 수 있는
  상태로 돌아가는 셈이라 별도 링크가 필요 없어졌다.
- 시체/글리치/이상현상 없음 체크박스 3개를 `ui::group_box`(설정 화면의
  "Display"/"CRT Effects" 같은 그룹박스와 같은 위젯)로 둘러싸서 "이
  셋이 한 세트"라는 걸 시각적으로 묶었다.
- "검수 저장" 버튼이 비활성 상태일 때 글자가 버튼 세로 중앙에 안
  맞고 위로 치우쳐 보이던 문제를 고쳤다 — 고정값(+5.0)으로 대충
  잡았던 y 좌표를, 정상 버튼(`ui.rs::raw_button`)이 쓰는 것과 똑같은
  공식(`y + (h - CELL_H) / 2.0`)으로 바꿨다.

구현 메모: `HexToolApp::saving: Option<f32>` 필드로 "저장 중" 표시
경과 시간을 들고 있다가 `SAVE_DELAY`(0.5초)를 넘으면 `loaded_photo_id`
등을 비워 빈 미리보기 상태로 되돌린다. `cargo build`/`cargo clippy`
확인, 경고는 기존 5개 그대로. 이제 안 쓰는 `SELECT_IMAGE` 문구도
`strings.rs` 에서 지웠다.

## → HexTool 체크박스 줄 간격 버그 수정 + 이미지 선택창 실사진 썸네일

**체크박스 줄이 겹쳐 보이던 진짜 원인을 찾아 고쳤다.** `ui::checkbox()`
는 라벨을 `y-3`에, 16×16 체크박스 사각형을 `y..y+16`에 그리는데, 지금까지
각 줄을 `row_y + 10.0` 에 그리고 있어서 체크박스 사각형이 `row_y+10`부터
`row_y+26`까지 차지했다 — 그런데 한 줄 폭(`CHECK_ROW_H`)은 20이었으니,
다음 줄이 시작되는 자리(`row_y+20`)보다 6px 더 내려와 다음 줄 영역을
침범하고 있었다. 그래서 줄 사이가 다닥다닥 붙어 보이고, 그룹 박스
아래쪽 여백도 위쪽(26px)보다 훨씬 좁게(2px) 찌그러져 보였던 것 — "높이가
안 맞는 느낌"의 정체였다. `CHECK_Y_OFFSET`(4.0)/`CHECK_ROW_H`(22.0)로
바꿔서 한 줄이 실제로 쓰는 폭(4+16=20)이 줄 폭(22) 안에 넉넉히 들어가게
하고, 그룹 박스 위/아래 여백(`BOX_TOP_INSET`=14, `BOX_BOTTOM_PAD`=16)도
서로 비슷하게 맞춰 위아래가 대칭으로 보이게 했다.

**이미지 선택 창(`apps/hex_picker.rs`)은 아이콘 대신 실제 사진 축소판을
보여주도록 다시 만들었다.** `widgets::icon_grid`(파일 종류별 고정
아이콘만 그리는 범용 위젯)를 쓰는 대신, photos.rs 의 피드와 같은 지연
디코드 방식(`load_scaled_texture`)으로 셀마다 실제 이미지를 그때그때
읽어 축소판으로 그리는 전용 그리드를 새로 짰다 — 화면에 한 번이라도
보인 셀만 디코드해서 텍스처로 올리고, 스크롤로 가려진 셀은 건드리지도
않는다(desktop.rs 의 다른 스크롤 목록들과 같은 요령).

구현 메모: 다중 선택/마퀴 드래그가 필요 없는 "고르면 곧장 닫히는" 창
이라 `icon_grid` 의 `selected`/`marquee_start` 상태를 아예 들고 있을
필요가 없어져, 그 부분 코드가 오히려 더 단순해졌다. `cargo build`/
`cargo clippy` 확인, 경고는 기존 5개 그대로.
