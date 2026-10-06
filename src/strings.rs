//! 화면에 보이는 모든 UI 문자열을 한 곳에 모아둔 파일 — 예전엔 `tr(lang, "Download",
//! "다운로드", "ダウンロード")` 처럼 문구 3개를 호출부에 직접 박아 넣었는데, 그러다
//! 보니 같은 단어("Download" 등)가 여러 파일에 따로따로 타이핑되고, 새 문구를 넣을
//! 때 한자 커버리지 감사(gfx.rs::build_font_atlas)를 매번 코드 전체를 grep 해서
//! 다시 훑어야 했다. 이 파일에 기능별로 나눠 담아두면 "지금 게임에 있는 모든 한국어/
//! 일본어 문구"를 이 파일 하나만 보고 한눈에 훑을 수 있다.
//!
//! 구조: `S`(en/ko/ja 세 문구를 한데 묶은 구조체) 상수 하나가 문구 하나에 대응한다.
//! 인덱스가 어긋나면 조용히 엉뚱한 언어가 나오는 "언어별로 완전히 분리된 배열 3개"
//! 대신, 세 언어를 항상 같은 자리에서 함께 정의해두는 방식을 택했다 — 상수 하나만
//! 봐도 세 언어가 다 있는지 바로 확인되고, 실수로 하나를 빠뜨리면 그 즉시 컴파일
//! 에러가 난다(S 의 세 필드가 전부 필수라서). 호출부는 `t(lang, module::NAME)` 로
//! 언어 타입(Language)에 맞는 문구를 받는다 — 기존 `tr(lang, en, ko, ja)` 를 대체한다.
//! 문구가 코드 곳곳에 흩어져 있던 걸 기능별 하위 모듈(mail/hextool/...)로 나눠서,
//! 원래 어느 화면 문구인지도 모듈 이름으로 바로 알 수 있게 했다.

use crate::foundation::Language;

#[derive(Clone, Copy)]
pub struct S {
    pub en: &'static str,
    pub ko: &'static str,
    pub ja: &'static str,
}

pub fn t(lang: Language, s: S) -> &'static str {
    match lang {
        Language::En => s.en,
        Language::Ko => s.ko,
        Language::Ja => s.ja,
    }
}

// 여러 파일에서 그대로 반복해 쓰는 짧은 UI 용어 — 버튼/공용 라벨.
pub mod common {
    use super::S;
    pub const OK: S = S { en: "OK", ko: "확인", ja: "OK" };
    pub const CANCEL: S = S { en: "Cancel", ko: "취소", ja: "キャンセル" };
    pub const RESTORE: S = S { en: "Restore", ko: "복원", ja: "元に戻す" };
    pub const DELETE: S = S { en: "Delete", ko: "삭제", ja: "削除" };
    pub const NAME: S = S { en: "Name", ko: "이름", ja: "名前" };
    pub const SIZE: S = S { en: "Size", ko: "크기", ja: "サイズ" };
    pub const ADDRESS: S = S { en: "Address:", ko: "주소:", ja: "アドレス:" };
    pub const YES: S = S { en: "Yes", ko: "예", ja: "はい" };
    pub const NO: S = S { en: "No", ko: "아니오", ja: "いいえ" };
    // File Explorer 의 상태바와 휴지통 상태바가 똑같이 쓰는 문구 — "{n}"/"{s}" 는
    // 호출부가 실제 개수로 .replace() 해서 채운다.
    pub const OBJECT_COUNT: S = S { en: "{n} object(s)", ko: "개체 {n}개", ja: "{n}個のオブジェクト" };
    pub const OBJECT_COUNT_SELECTED: S =
        S { en: "{n} object(s) ({s} selected)", ko: "개체 {n}개 (선택 {s}개)", ja: "{n}個のオブジェクト(選択中 {s}個)" };
}

// foundation.rs::category_label()/display_name() — File Explorer 의 고정 카테고리
// 탭 이름과, 로케일과 무관한 내부 식별자(폴더 이름 등)를 화면에 보여줄 때 쓰는
// 표시 이름.
pub mod foundation {
    use super::S;
    pub const CAT_DOWNLOADS: S = S { en: "Downloads", ko: "다운로드", ja: "ダウンロード" };
    pub const CAT_DESKTOP: S = S { en: "Desktop", ko: "바탕화면", ja: "デスクトップ" };
    pub const CAT_VIDEOS: S = S { en: "Videos", ko: "동영상", ja: "ビデオ" };
    pub const CAT_IMAGES: S = S { en: "Images", ko: "사진", ja: "画像" };
    pub const MY_COMPUTER: S = S { en: "My Computer", ko: "내 컴퓨터", ja: "マイコンピュータ" };
    pub const RECYCLE_BIN: S = S { en: "Recycle Bin", ko: "휴지통", ja: "ごみ箱" };
    pub const MAIL: S = S { en: "Mail", ko: "메일", ja: "メール" };
    pub const DELETED: S = S { en: "(deleted)", ko: "(삭제됨)", ja: "(削除済み)" };
}

// apps/password.rs — .lock 비밀번호 대화상자.
pub mod password {
    use super::S;
    pub const ENTER_PASSWORD: S = S { en: "Enter password:", ko: "비밀번호를 입력하세요:", ja: "パスワードを入力してください:" };
    pub const WRONG_PASSWORD: S = S { en: "Wrong password!", ko: "비밀번호가 틀렸습니다!", ja: "パスワードが違います!" };
}

// apps/official_site.rs — WebView2 캡처 창.
pub mod official_site {
    use super::S;
    pub const TITLE: S = S { en: "Official Site", ko: "공식 사이트", ja: "公式サイト" };
    pub const LOAD_FAILED: S = S { en: "Couldn't load site.", ko: "사이트를 불러오지 못했습니다.", ja: "サイトを読み込めませんでした。" };
    pub const RUNTIME_MISSING: S = S { en: "(WebView2 Runtime missing?)", ko: "(WebView2 런타임이 없나요?)", ja: "(WebView2ランタイムがない?)" };
}

// apps/video_player.rs — .mp4 재생 창.
pub mod video_player {
    use super::S;
    pub const NO_VIDEO: S = S { en: "No video.", ko: "동영상이 없습니다.", ja: "動画がありません。" };
    pub const PUT_VIDEO_AT: S = S { en: "Put a video at", ko: "다음 경로에 동영상을 넣으세요:", ja: "動画を次の場所に置いてください:" };
}

// apps/sound_player.rs — .wav 재생 창.
pub mod sound_player {
    use super::S;
    pub const PLAY: S = S { en: "Play", ko: "재생", ja: "再生" };
    pub const PAUSE: S = S { en: "Pause", ko: "일시정지", ja: "一時停止" };
}

// apps/game_installer.rs — 메일 첨부 "test Setup.exe"를 열면 뜨는 설치 마법사.
pub mod game_installer {
    use super::S;
    pub const INSTALL: S = S { en: "Install", ko: "설치", ja: "インストール" };
    pub const FINISH: S = S { en: "Finish", ko: "마침", ja: "完了" };

    pub const PAGE_ALREADY_INSTALLED: S = S { en: "Already Installed", ko: "이미 설치됨", ja: "インストール済み" };
    pub const ALREADY_INSTALLED_MSG: S =
        S { en: "test is already installed on this computer.", ko: "이 컴퓨터에는 test가 이미 설치되어 있습니다.", ja: "このコンピューターにはtestが既にインストールされています。" };

    pub const PAGE_WELCOME: S = S { en: "Welcome", ko: "환영합니다", ja: "ようこそ" };
    pub const WELCOME_MSG: S = S {
        en: "This wizard will install test on your computer.",
        ko: "이 마법사는 이 컴퓨터에 test를 설치합니다.",
        ja: "このウィザードは、このコンピューターにtestをインストールします。",
    };
    pub const CLICK_INSTALL_OR_CANCEL: S =
        S { en: "Click Install to continue, or Cancel to exit.", ko: "계속하려면 설치를, 종료하려면 취소를 누르세요.", ja: "続けるにはインストール、終了するにはキャンセルを押してください。" };

    pub const PAGE_INSTALLING: S = S { en: "Installing", ko: "설치 중", ja: "インストール中" };
    pub const INSTALLING_MSG: S = S { en: "Please wait while test is installed.", ko: "test를 설치하는 동안 잠시 기다려 주세요.", ja: "testのインストール中です。お待ちください。" };
    pub const STEP_COPYING: S = S { en: "Copying files...", ko: "파일 복사 중...", ja: "ファイルをコピー中..." };
    pub const STEP_REGISTERING: S = S { en: "Registering test.dll...", ko: "test.dll 등록 중...", ja: "test.dllを登録中..." };
    pub const STEP_UPDATING: S = S { en: "Updating configuration...", ko: "설정 업데이트 중...", ja: "設定を更新中..." };
    pub const STEP_VERIFYING: S = S { en: "Verifying installation...", ko: "설치 확인 중...", ja: "インストールを確認中..." };
    pub const STEP_FINALIZING: S = S { en: "Finalizing...", ko: "마무리 중...", ja: "仕上げ中..." };
    pub const DONE: S = S { en: "Done.", ko: "완료.", ja: "完了。" };

    pub const PAGE_FINISH: S = S { en: "Setup Complete", ko: "설치 완료", ja: "セットアップ完了" };
    pub const FINISH_MSG: S = S { en: "Setup has finished installing test on your computer.", ko: "test 설치가 완료되었습니다.", ja: "testのインストールが完了しました。" };
    pub const INSTALLED_TO: S = S { en: "Installed to:", ko: "설치 위치:", ja: "インストール先:" };
    pub const CLICK_FINISH_TO_CLOSE: S = S { en: "Click Finish to close this wizard.", ko: "마법사를 닫으려면 마침을 누르세요.", ja: "ウィザードを閉じるには完了を押してください。" };
}

// apps/mail.rs — 메일 앱(Outlook Express 스타일).
pub mod mail {
    use super::S;

    // 첫 메일 — 친구가 크랙 게임 설치 파일(test Setup.exe)을 첨부해서 보낸다(STORY.md 7-1절).
    pub const GAME_MAIL_SUBJECT: S = S { en: "try this lol", ko: "야 이거 해봐 ㅋㅋ", ja: "これやってみて笑" };
    pub const GAME_MAIL_BODY: S = S {
        en: "Got a cracked copy of that game.\nJust run the setup and install it.\n\nIt's kind of weird though.\nTell me what you think after you play it.",
        ko: "그 게임 크랙판 구했다.\nSetup 파일 실행해서 설치하면 됨.\n\n근데 좀 이상한 게임이더라.\n해보고 어땠는지 말해줘.",
        ja: "あのゲームのクラック版、手に入れたよ。\nセットアップを実行してインストールすればいい。\n\nでも、ちょっと変なゲームなんだよね。\nやってみたら感想を教えて。",
    };

    // 두 번째 메일 — 깨진 이름의 발신자가 꽃을 구할 수 있는 곳을 안다고 한다.
    pub const FLOWER_MAIL_SUBJECT: S =
        S { en: "I know where you can get a flower", ko: "내가 꽃을 구할 수 있는 곳을 알고 있어", ja: "花を手に入れられる場所を知っている" };
    pub const FLOWER_MAIL_BODY: S = S {
        en: "I know where you can get a flower.\nI just sent it over. Run it when it arrives.",
        ko: "내가 꽃을 구할 수 있는 곳을 알고 있어.\n방금 보냈어. 받으면 실행해 봐.",
        ja: "花を手に入れられる場所を知っている。\nたった今送ったよ。届いたら起動してみて。",
    };

    // 문(도어즈)의 편지 — 방에서 주운 편지와 같은 내용이 메일로 온다(시트 오른쪽 표).
    pub const LETTER_MAIL_SUBJECT: S = S { en: "A letter", ko: "편지", ja: "手紙" };
    pub const LETTER_MAIL_BODY: S = S {
        en: "Thank you for hearing me out.\nI cannot leave this room.\nI will find out where the flowers are instead.\nPlease wait a little.",
        ko: "부탁을 들어줘서 고마워.\n나는 이 방 밖으로 나갈 수 없어.\n대신 꽃이 있는 곳을 알아볼게.\n조금만 기다려 줘.",
        ja: "頼みを聞いてくれてありがとう。\n私はこの部屋から出られない。\nかわりに花のある場所を調べてみる。\nもう少しだけ待っていて。",
    };

    // 세 번째 메일 — 씨앗을 심은 뒤 오는 힌트(시트의 "메일 가이딩").
    pub const HINT_MAIL_SUBJECT: S = S { en: "hmm...", ko: "음...", ja: "うーん..." };
    pub const HINT_MAIL_BODY: S = S {
        en: "Hmm... maybe I could turn the clock on the desktop forward, physically?",
        ko: "음... 바탕화면에서 물리적으로 시간을 돌려도 괜찮지 않을까?",
        ja: "うーん... デスクトップの時計を、物理的に進めてみてもいいんじゃないかな?",
    };

    // 네 번째 메일 — 꽃을 보낸 뒤 꽃 사진이 첨부돼 온다.
    pub const PHOTO_MAIL_SUBJECT: S = S { en: "a flower", ko: "꽃", ja: "花" };
    pub const PHOTO_MAIL_BODY: S = S { en: "I got the flower.\nThank you.", ko: "꽃을 받았어.\n고마워.", ja: "花を受け取ったよ。\nありがとう。" };

    pub const FOLDER_INBOX: S = S { en: "Inbox", ko: "받은편지함", ja: "受信トレイ" };
    pub const FOLDER_SENT: S = S { en: "Sent Items", ko: "보낸편지함", ja: "送信済みアイテム" };
    pub const FOLDER_COMPOSE: S = S { en: "Write Mail", ko: "메일 쓰기", ja: "メール作成" };

    pub const MAILBOX_LABEL: S = S { en: "Outlook Mailbox", ko: "메일함", ja: "メールボックス" };
    pub const STATUS_ITEMS_UNREAD: S =
        S { en: "{n} Item(s), {u} Unread", ko: "{n}개 항목, 안 읽음 {u}개", ja: "{n}件のアイテム、未読{u}件" };
    pub const STATUS_ITEMS: S = S { en: "{n} Item(s)", ko: "{n}개 항목", ja: "{n}件のアイテム" };
    pub const STATUS_ZERO_ITEMS: S = S { en: "0 Items", ko: "0개 항목", ja: "0件のアイテム" };

    pub const SELECT_FOLDER_MSG: S =
        S { en: "Select a folder to view its contents.", ko: "내용을 보려면 폴더를 선택하세요.", ja: "内容を表示するにはフォルダを選択してください。" };
    pub const NO_NEW_MESSAGES: S = S { en: "No new messages.", ko: "새 메시지가 없습니다.", ja: "新しいメッセージはありません。" };
    pub const NO_SENT_MESSAGES: S =
        S { en: "No sent messages.", ko: "보낸 메일이 없습니다.", ja: "送信済みのメッセージがありません。" };

    pub const COL_FROM: S = S { en: "From", ko: "보낸 사람", ja: "差出人" };
    pub const COL_TO: S = S { en: "To", ko: "받는 사람", ja: "宛先" };
    pub const COL_SUBJECT: S = S { en: "Subject", ko: "제목", ja: "件名" };

    pub const BACK_TO_INBOX: S = S { en: "< Back to Inbox", ko: "< 받은편지함으로", ja: "< 受信トレイに戻る" };
    pub const BACK_TO_SENT: S = S { en: "< Back to Sent Items", ko: "< 보낸편지함으로", ja: "< 送信済みアイテムに戻る" };

    pub const FIELD_FROM: S = S { en: "From:", ko: "보낸 사람:", ja: "差出人:" };
    pub const FIELD_TO: S = S { en: "To:", ko: "받는 사람:", ja: "宛先:" };
    pub const FIELD_CC: S = S { en: "Cc:", ko: "참조:", ja: "CC:" };
    pub const FIELD_SUBJECT: S = S { en: "Subject:", ko: "제목:", ja: "件名:" };

    pub const DOWNLOAD: S = S { en: "Download", ko: "다운로드", ja: "ダウンロード" };
    pub const DOWNLOADING: S = S { en: "Downloading", ko: "다운로드 중", ja: "ダウンロード中" };
    pub const DOWNLOADED: S = S { en: "Downloaded", ko: "다운로드됨", ja: "ダウンロード済み" };

    pub const NO_FILES_TO_ATTACH: S =
        S { en: "No files available to attach.", ko: "첨부할 수 있는 파일이 없습니다.", ja: "添付できるファイルがありません。" };
    pub const REMOVE: S = S { en: "Remove", ko: "제거", ja: "削除" };
    pub const ATTACH_BUTTON: S = S { en: "Attach...", ko: "첨부...", ja: "添付..." };
    pub const SEND_BUTTON: S = S { en: "Send", ko: "보내기", ja: "送信" };
}

// scenes/boot.rs — 부팅 화면.
pub mod boot {
    use super::S;
    pub const WELCOME: S =
        S { en: "Welcome to PalaceOS ver.1.0.0", ko: "PalaceOS ver.1.0.0에 오신 것을 환영합니다", ja: "PalaceOS ver.1.0.0へようこそ" };
}

// scenes/lobby.rs — 시작 화면(New Start/Continue/Settings/Quit 메뉴).
pub mod lobby {
    use super::S;
    pub const NEW_START: S = S { en: "New Start", ko: "새로 시작", ja: "最初から" };
    pub const CONTINUE: S = S { en: "Continue", ko: "이어하기", ja: "続ける" };
    pub const QUIT: S = S { en: "Quit", ko: "종료", ja: "終了" };
    pub const CONFIRM_QUIT: S =
        S { en: "Really quit PalaceOS?", ko: "정말 PalaceOS를 종료하시겠습니까?", ja: "本当にPalaceOSを終了しますか?" };
}

// scenes/desktop.rs — 바탕화면/시작 메뉴/작업표시줄/시스템 메시지 패널.
pub mod desktop {
    use super::S;
    pub const SHUT_DOWN: S = S { en: "Shut Down", ko: "시스템 종료", ja: "シャットダウン" };
    pub const ERASE_LINE1: S = S { en: "Erase all saved progress", ko: "저장된 모든 진행 상태를", ja: "保存された進行状況を" };
    pub const ERASE_LINE2: S =
        S { en: "and restart PalaceOS?", ko: "지우고 PalaceOS를 다시 시작할까요?", ja: "すべて消去してPalaceOSを再起動?" };
    pub const ERASE: S = S { en: "Erase", ko: "지우기", ja: "消去" };
    pub const NETWORK: S = S { en: "Network", ko: "네트워크", ja: "ネットワーク" };
    pub const STATUS: S = S { en: "Status", ko: "상태", ja: "状態" };
    pub const CONNECTED: S = S { en: "Connected", ko: "연결됨", ja: "接続済み" };
    pub const DISCONNECTED: S = S { en: "Disconnected", ko: "연결 안 됨", ja: "未接続" };
    pub const UNKNOWN: S = S { en: "(unknown)", ko: "(알 수 없음)", ja: "(不明)" };
    pub const NEW_MAIL: S = S { en: "New Mail", ko: "새 메일", ja: "新着メール" };
    pub const DATE_TIME: S = S { en: "Date and Time", ko: "날짜 및 시간", ja: "日付と時刻" };
    pub const DOWNLOAD_COMPLETE: S = S { en: "Download Complete", ko: "다운로드 완료", ja: "ダウンロード完了" };
}

// apps/settings.rs — 설정 창.
pub mod settings {
    use super::S;
    pub const TITLE: S = S { en: "Settings", ko: "설정", ja: "設定" };
    pub const DISPLAY: S = S { en: "Display", ko: "디스플레이", ja: "ディスプレイ" };
    pub const RESOLUTION: S = S { en: "Resolution", ko: "해상도", ja: "解像度" };
    pub const FRAME_RATE: S = S { en: "Frame rate", ko: "주사율", ja: "フレームレート" };
    pub const CRT_EFFECTS: S = S { en: "CRT Effects", ko: "CRT 효과", ja: "CRTエフェクト" };
    pub const CRT_INTENSITY: S = S { en: "CRT Intensity", ko: "CRT 강도", ja: "CRT強度" };
    pub const CHROMATIC_ABERRATION: S = S { en: "Chromatic Aberration", ko: "색수차", ja: "色収差" };
    pub const CURSOR_SIZE: S = S { en: "Cursor Size", ko: "커서 크기", ja: "カーソルサイズ" };
    pub const SFX: S = S { en: "SFX", ko: "효과음", ja: "効果音" };
    pub const BGM: S = S { en: "BGM", ko: "배경음", ja: "BGM" };
    pub const MP4_SOUND: S = S { en: "Mp4 Sound", ko: "영상 음량", ja: "動画音量" };
    pub const MASTER: S = S { en: "MASTER", ko: "전체 음량", ja: "マスター" };
    pub const VOLUME: S = S { en: "Volume", ko: "음량", ja: "音量" };
    pub const EFFECTS: S = S { en: "Effects", ko: "효과", ja: "エフェクト" };
    pub const WEATHERING: S = S { en: "Weathering", ko: "레트로 질감", ja: "レトロ感" };
    pub const MUTE_ALL: S = S { en: "Mute All", ko: "전체 음소거", ja: "すべてミュート" };
    pub const APPEARANCE: S = S { en: "Appearance", ko: "화면", ja: "外観" };
    pub const SMOOTH_SCROLL: S = S { en: "Smooth scroll", ko: "부드러운 스크롤", ja: "スムーズスクロール" };
    pub const LANGUAGE: S = S { en: "Language", ko: "언어", ja: "言語" };
    pub const BACKGROUND_COLOR: S = S { en: "Background color", ko: "배경색", ja: "背景色" };
    pub const ERASE_ALL_MEMORY: S = S { en: "Erase All Memory", ko: "모든 기록 지우기", ja: "すべての記憶を消去" };
}

// apps/recycle_bin.rs — 왼쪽 안내 패널 문단.
pub mod recycle_bin {
    use super::S;
    pub const DESC: S = S {
        en: "This folder contains files and folders that you have deleted from your computer.",
        ko: "이 폴더에는 컴퓨터에서 삭제한 파일과 폴더가 들어 있습니다.",
        ja: "このフォルダには、コンピューターから削除したファイルとフォルダが入っています。",
    };
    pub const EMPTY_LINK: S = S {
        en: "To permanently remove all items and reclaim disk space, click Empty Recycle Bin.",
        ko: "모든 항목을 영구히 삭제하고 디스크 공간을 확보하려면 휴지통 비우기를 클릭하세요.",
        ja: "すべての項目を完全に削除してディスク容量を確保するには、ごみ箱を空にするをクリックしてください。",
    };
}

// apps/explorer.rs — File Explorer 창.
pub mod explorer {
    use super::S;
    pub const FOLDERS: S = S { en: "Folders", ko: "폴더", ja: "フォルダ" };
    pub const ITEMS_COUNT: S = S { en: "{n} items", ko: "{n}개 항목", ja: "{n}個の項目" };
}

// apps/credits.rs — 시작 메뉴에서 바로 여는 크레딧 창.
pub mod credits {
    use super::S;
    pub const TITLE: S = S { en: "Credits", ko: "크레딧", ja: "クレジット" };
    pub const DEVELOPERS: S = S { en: "Developers", ko: "개발자", ja: "開発者" };
}
