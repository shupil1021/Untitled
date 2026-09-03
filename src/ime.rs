//! 한글/일본어 IME 로 조합 중인(아직 확정 안 된) 문자열을 미리 읽어오고
//! (composition_preview), 그 조합을 취소하는(cancel_composition) 작은 모듈 —
//! 전부 IMM32 unsafe FFI 라 직접 실행해 검증할 수는 없다(문제 생기면 알려달라).
//!
//! composition_preview() 의 MAX_COMPOSITION_BYTES 상한: 정상적인 조합 중
//! 문자열은 몇 글자를 절대 안 넘으므로, 그보다 훨씬 큰 값이 보고되면 버퍼
//! 크기가 잘못됐다고 보고 안전하게 포기한다(읽지도 그리지도 않는다).

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Input::Ime::{
    CANDIDATEFORM, CFS_CANDIDATEPOS, CFS_FORCE_POSITION, CFS_POINT, COMPOSITIONFORM, CPS_CANCEL, GCS_COMPSTR,
    ImmGetCompositionStringW, ImmGetContext, ImmNotifyIME, ImmReleaseContext, ImmSetCandidateWindow, ImmSetCompositionWindow,
    ImmSetStatusWindowPos, NI_COMPOSITIONSTR, NOTIFY_IME_INDEX,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, GetWindowThreadProcessId, SW_HIDE, ShowWindow};

// 조합 중 문자열에 허용하는 최대 바이트 길이(UTF-16 기준 64글자 = 128바이트) —
// 실제 조합 중 문자열이 이보다 길 일은 사실상 없다(한글 한 음절은 글자 하나,
// 일본어도 변환 전 미확정 상태가 이렇게까지 길어지지 않는다). 이보다 긴 값이
// 보고되면 정상적인 조합 상태가 아니라고 보고 안전하게 포기한다.
const MAX_COMPOSITION_BYTES: i32 = 128;

/// 지금 조합 중인(아직 확정 안 된) 문자열을 읽어온다 — 화면에 "미리보기"로
/// 이어붙여 보여줘서, 한글 IME 가 다음 글자를 쳐야 이전 글자를 확정하는
/// 타이밍 특성 때문에 생기는 "한 글자 늦게 보이는" 느낌을 줄이는 용도. 실제로
/// 저장/전송되는 값은 여전히 char_event 로 확정된 문자만으로 채워지고 이
/// 함수의 결과는 절대 안 섞인다 — 이건 순전히 "화면에 한 프레임(사실상 한
/// 타이밍) 먼저 보여주는" 용도일 뿐이다.
pub fn composition_preview() -> Option<String> {
    unsafe {
        let hwnd = GetActiveWindow();
        if hwnd.is_invalid() {
            return None;
        }
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return None;
        }
        let text = read_compstr(himc);
        let _ = ImmReleaseContext(hwnd, himc);
        text
    }
}

unsafe fn read_compstr(himc: windows::Win32::UI::Input::Ime::HIMC) -> Option<String> {
    // 1차 호출 — 버퍼 없이 필요한 바이트 수만 물어본다. 조합 중이 아니면 0
    // 이하가 온다(문서상 IMM_ERROR_* 는 음수, 조합 문자열이 비어 있으면 0).
    let len = unsafe { ImmGetCompositionStringW(himc, GCS_COMPSTR, None, 0) };
    if len <= 0 {
        return None;
    }
    // 상식 밖으로 큰 값이 보고되면(정상 조합 상태에서는 절대 안 나올 값) 뭔가
    // 잘못됐다고 보고 안전하게 포기한다 — 버퍼를 그 크기로 할당하지도 읽지도
    // 않는다.
    if len > MAX_COMPOSITION_BYTES {
        return None;
    }
    // len 은 바이트 수(UTF-16 이므로 짝수여야 정상) — 홀수처럼 이상한 값이 오면
    // 믿지 않고 그냥 포기한다(그 다음 실제 쓰기 호출에서 버퍼 크기가 안 맞아
    // 넘치는 사고를 막기 위한 방어적 처리).
    if len % 2 != 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize / 2];
    // 2차 호출 — 이번엔 실제 버퍼를 넘긴다. dwbuflen 을 buf 의 실제 바이트
    // 용량(len, 위에서 그 크기로 정확히 할당했다)과 똑같이 줘서, IME 가 그보다
    // 더 많이 쓸 수 없다.
    let actual = unsafe { ImmGetCompositionStringW(himc, GCS_COMPSTR, Some(buf.as_mut_ptr() as *mut _), len as u32) };
    if actual <= 0 {
        return None;
    }
    // actual 도 다시 한번 버퍼 용량 안으로 클램프 — 혹시라도 1차 호출 이후
    // 조합 상태가 바뀌어(예: 다른 스레드/메시지) actual 이 len 보다 커져
    // 있어도, 우리가 실제로 할당한 buf 밖은 절대 안 읽는다.
    let count = (actual as usize / 2).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..count]))
}

/// Windows 자체의 IME 조합창(지금 타이핑 중인 글자를 보여주는 작은 팝업)/
/// 후보 창/상태 창(지금 한글·영문 중 어느 모드인지 보여주는 작은 표시) 세
/// 팝업 전부의 위치를 client 좌표 (x, y) 로 옮겨달라고 알려준다.
///
/// 처음엔 이걸로 그 팝업을 우리 텍스트 캐럿 바로 옆에 세우려고 했다(이 게임이
/// 한 번도 IME 캐럿 위치를 등록해준 적이 없어서, Windows 가 기본값인 창의
/// (0,0) — 화면 왼쪽 위 구석 — 에 그냥 띄우고 있었다). 그런데 실제로 캐럿
/// 옆에 세워보니, Windows 의 둥근 모서리/그림자 있는 현대식 팝업 스타일이
/// 이 게임의 픽셀아트 CRT 화면과 너무 안 어울려서 오히려 더 어색한 이물질처럼
/// 튀어 보였다("이렇게 구현한 건 더 짜쳐" 라는 피드백을 받았다) — 그래서
/// 지금은 이 함수를 호출부(mail.rs)가 항상 화면 밖 좌표로만 불러서, 사실상
/// "이 팝업을 아예 안 보이게 치워라" 용도로 쓰고 있다. 문자열은 전혀 안
/// 건드리고 좌표만 알려주는 호출이라(실제 조합 문자열을 읽거나 그리는 코드는
/// 여전히 하나도 없다) is_composing() 과 마찬가지로 안전하다 — 위치를 옮겨도
/// 조합/확정 자체(실제 문자 입력)엔 전혀 영향이 없다.
pub fn set_composition_pos(x: i32, y: i32) {
    unsafe {
        let hwnd = GetActiveWindow();
        if hwnd.is_invalid() {
            return;
        }
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return;
        }
        let pt = POINT { x, y };
        // CFS_FORCE_POSITION 을 같이 줘서 "그냥 힌트"가 아니라 IME 가 반드시 이
        // 자리를 쓰도록 강제한다 — 이게 없으면 일부 IME 가 자기 나름의 배치
        // 로직으로 화면 안 다른 자리에 도로 띄우는 경우가 있었다.
        let comp = COMPOSITIONFORM { dwStyle: CFS_POINT | CFS_FORCE_POSITION, ptCurrentPos: pt, rcArea: RECT::default() };
        let _ = ImmSetCompositionWindow(himc, &comp);
        let cand = CANDIDATEFORM { dwIndex: 0, dwStyle: CFS_CANDIDATEPOS | CFS_FORCE_POSITION, ptCurrentPos: pt, rcArea: RECT::default() };
        let _ = ImmSetCandidateWindow(himc, &cand);
        // 조합창/후보창과는 또 다른 세 번째 팝업 — 지금 입력 모드가 한/영 중
        // 뭔지 보여주는 "상태 창"(status window). 이것도 안 옮기면 화면 구석에
        // 따로 남아있는다.
        let _ = ImmSetStatusWindowPos(himc, &pt);
        let _ = ImmReleaseContext(hwnd, himc);
    }
}

/// 그런데도 화면 구석에 남아있던 네 번째 팝업의 정체 — 위의 IMM32 API 셋
/// (조합창/후보창/상태창)은 전부 옛날식 API 라, 최신 Windows 의 IME(TSF/Cicero
/// 라는 서브시스템)가 자기 나름으로 띄우는 언어 표시줄/후보 미니 툴바에는
/// 아예 안 먹힌다 — 완전히 다른 창이고 저 API 들이 관여하는 범위 밖이다. 이
/// 팝업은 클래스 이름이 "CiceroUIWndFrame" 로 시작하는 우리 프로세스 소유의
/// 최상위 창으로 실제로 뜬다(Windows 내부적으로 잘 알려진 이름). 위치를
/// 옮기는 API가 없으니, 그냥 매 프레임 찾아서 통째로 숨긴다(ShowWindow
/// SW_HIDE) — 이 창은 순전히 장식용 UI 라, 숨겨도 실제 조합/확정(문자 입력
/// 자체)에는 전혀 영향이 없다.
pub fn hide_cicero_windows() {
    unsafe {
        let my_pid = GetCurrentProcessId();
        let _ = EnumWindows(Some(enum_hide_cicero), LPARAM(my_pid as isize));
    }
}

unsafe extern "system" fn enum_hide_cicero(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let target_pid = lparam.0 as u32;
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == target_pid {
            let mut buf = [0u16; 64];
            let len = GetClassNameW(hwnd, &mut buf);
            if len > 0 {
                let name = String::from_utf16_lossy(&buf[..len as usize]);
                if name.starts_with("CiceroUIWndFrame") {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
        }
        BOOL(1) // 계속 훑는다(다른 최상위 창들도 마저 확인).
    }
}

// 조합을 취소한다 — `ImmSetCompositionStringW` 로 조합 버퍼에 값을 다시
// 써넣어 조합을 이어가는 시도는 TSF 기반 IME 에서 무시되는 것으로 보여
// (경위는 README) 포기했다. 대신 mail.rs 가 이 함수로 조합을 완전히
// 지운 뒤, 자기가 기록해둔 "한 단계 전" 값을 조합이 아니라 평범한 확정
// 문자로 직접 밀어넣는다(composition_history 참고) — 그 이후 이어서
// 타이핑하면 IME 가 새 조합을 시작한다.
pub fn cancel_composition() {
    unsafe {
        let hwnd = GetActiveWindow();
        if hwnd.is_invalid() {
            return;
        }
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return;
        }
        let _ = ImmNotifyIME(himc, NI_COMPOSITIONSTR, NOTIFY_IME_INDEX(CPS_CANCEL.0), 0);
        let _ = ImmReleaseContext(hwnd, himc);
    }
}
