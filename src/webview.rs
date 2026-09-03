//! Official Site 창용: 실제 웹페이지를 WebView2로 백그라운드에서 렌더링하고,
//! 주기적으로 화면을 캡처해서 RGBA 로 넘겨준다. 창에는 절대 안 보이는 숨은 호스트
//! 창을 하나 만들어서 그 안에 WebView2 컨트롤을 붙이고, 그 결과 이미지만 꺼내
//! 우리 텍스처로 올린다 — 그러면 그 이미지도 다른 화면들처럼 CRT 셰이더를 그대로
//! 탄다(진짜 라이브 임베드는 우리 렌더 파이프라인 밖이라 셰이더가 안 먹는다).

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// 게임을 끌 때 열려있는 웹뷰들이 있으면 그 브라우저 하위 프로세스가 아직 폴더 파일을
// 붙잡고 있어서 삭제가 실패했다 — 그래서 종료 시점에 "지금 살아있는 웹뷰 스레드
// 개수"를 확인하고, GLOBAL_STOP 으로 전부 정지 신호를 보낸 뒤, 다 끝날 때까지
// (짧게, 무한정은 아니게) 기다렸다가 지우도록 한다. 프로세스 전역이라 static 으로 둔다.
static GLOBAL_STOP: AtomicBool = AtomicBool::new(false);
static ACTIVE_WEBVIEWS: AtomicUsize = AtomicUsize::new(0);

use webview2_com::{
    CapturePreviewCompletedHandler, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, ExecuteScriptCompletedHandler, Microsoft::Web::WebView2::Win32::*,
};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::Com::{CoInitializeEx, StructuredStorage::CreateStreamOnHGlobal, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

// 표시 영역 안에서의 위치를 0..1 비율(가로/세로 각각)로 담은 입력 이벤트.
// OfficialSiteApp 이 마우스를 이 비율로 변환해서 보낸다.
//
// 처음엔 실제 Win32 입력 메시지를 WebView2 의 내부 자식 창(Chrome_RenderWidgetHostHWND)
// 으로 PostMessage 합성해서 넣으려 했는데, 계속 반응이 없었다 — Chromium 은 진짜
// 하드웨어에서 온 입력이 아니면 무시하는 내부 검증이 있어서 이 방식은 신뢰할 수 없다.
// 대신 ExecuteScript 로 페이지 DOM 에 직접 MouseEvent 를 dispatch 하는 방식으로 바꿨다
// — 이건 웹페이지 관점에서 진짜 이벤트 리스너가 그대로 받는 표준 동작이라 훨씬 확실하다.
//
// 좌표는 픽셀이 아니라 0..1 비율로 넘긴다 — 캡처 해상도(w,h) 와 실제 페이지의
// CSS 뷰포트 크기가 정확히 일치한다는 보장이 없어서(스케일링/리사이즈 등으로 살짝
// 어긋날 수 있음), 픽셀 그대로 쓰면 클릭 위치가 미묘하게 틀어진다. 스크립트 쪽에서
// 그 순간의 실제 window.innerWidth/innerHeight 를 기준으로 다시 픽셀로 환산하면
// 항상 정확하게 맞는다.
// Down/Up 을 따로 보내야 JS 로 만든 드래그(mousedown → 문서 전체 mousemove → mouseup
// 리스너 패턴)가 동작한다 — 이전처럼 down+up+click 을 한 번에 묶어 보내면 그 사이의
// mousemove 가 하나도 없어서 드래그가 안 먹혔다.
//
// ⚠ 한계: 순수 CSS `:hover` 만으로 되는 효과(자바스크립트 리스너 없이 CSS 만으로
// 색이 바뀌는 버튼 등)는 이 방식으로는 못 흉내낸다 — 그건 Chromium 내부적으로 진짜
// 마우스 위치를 추적해서 켜지는 상태라, 합성 이벤트를 아무리 dispatch 해도 안 켜진다.
// mouseover/mouseenter 같은 JS 이벤트 리스너 기반 호버는 아래에서 흉내낸다.
pub enum WebviewInput {
    Move(f32, f32),
    Down(f32, f32),
    Up(f32, f32),
    Wheel(f32), // 스크롤 델타(휠 노치 단위, 위/아래 부호 있음)
    // 키보드도 마우스처럼 네이티브 메시지 대신 document.activeElement 를 직접
    // 조작하는 JS 로 흉내낸다 — 합성 keydown 은 실제로 글자를 입력해주지 않는다
    // (신뢰된 입력에만 반응하는 브라우저 기본 동작이라 마우스 클릭과 같은 제약).
    Type(char),
    Backspace,
    Enter,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// WebView2 프로필(캐시/쿠키) 저장 폴더 경로. 다른 데스크톱 앱들처럼 %LOCALAPPDATA% 밑
// 우리 전용 폴더에 둔다 — 유저가 평소에 들여다볼 일 없는 곳이고, 지정을 안 해주면
// exe 옆에 생기는 "<exe이름>.WebView2" 보다 훨씬 낫다.
fn user_data_dir_path() -> std::path::PathBuf {
    std::env::var("LOCALAPPDATA")
        .map(|p| std::path::PathBuf::from(p).join("PalaceOS").join("WebView2"))
        .unwrap_or_else(|_| std::env::temp_dir().join("PalaceOS_WebView2"))
}

// 지울 땐 WebView2 하위 폴더만이 아니라 "PalaceOS" 폴더 자체를 통째로 지운다 —
// 지금은 그 밑에 WebView2 말고 다른 용도로 쓰는 게 없어서, 남겨둘 이유가 없다.
fn cleanup_root_path() -> std::path::PathBuf {
    let dir = user_data_dir_path();
    dir.parent().map(|p| p.to_path_buf()).unwrap_or(dir)
}

// run() 이 어느 경로로 리턴하든(성공/실패/조기 반환 전부) ACTIVE_WEBVIEWS 가 확실히
// 줄어들게 하려고 RAII 로 관리한다 — 그래야 shutdown_all_and_cleanup() 이 "이제 다
// 끝났다" 를 정확히 알 수 있다.
struct ActiveGuard;
impl ActiveGuard {
    fn new() -> Self {
        ACTIVE_WEBVIEWS.fetch_add(1, Ordering::SeqCst);
        ActiveGuard
    }
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        ACTIVE_WEBVIEWS.fetch_sub(1, Ordering::SeqCst);
    }
}

// 게임을 끌 때 호출 — 열려있는 웹뷰가 있으면 전부 멈추라는 신호를 보내고, 브라우저
// 하위 프로세스까지 실제로 정리될 때까지(짧게, 무한정은 아니게) 기다린 다음에
// PalaceOS 폴더를 지운다. 이렇게 해야 Official Site 를 켜둔 채로 꺼도 확실히 지워진다.
pub fn shutdown_all_and_cleanup() {
    GLOBAL_STOP.store(true, Ordering::SeqCst);
    // 최대 ~3초 기다린다 — 그동안 열려있던 웹뷰 스레드들이 다음 루프 반복에서
    // GLOBAL_STOP 을 보고 Close()/DestroyWindow() 까지 마치고 스스로 끝난다.
    for _ in 0..30 {
        if ACTIVE_WEBVIEWS.load(Ordering::SeqCst) == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let dir = cleanup_root_path();
    for _ in 0..5 {
        if std::fs::remove_dir_all(&dir).is_ok() || !dir.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

// (너비, 높이, RGBA 픽셀) — 캡처된 프레임 한 장.
type Frame = (u32, u32, Vec<u8>);

pub struct WebviewHandle {
    latest: Arc<Mutex<Option<Frame>>>,
    failed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    input_tx: Sender<WebviewInput>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl WebviewHandle {
    pub fn open(url: String, w: u32, h: u32) -> WebviewHandle {
        let latest = Arc::new(Mutex::new(None));
        let failed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (input_tx, input_rx) = channel();
        let (latest2, failed2, stop2) = (latest.clone(), failed.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            if run(&url, w, h, &latest2, &stop2, input_rx).is_none() {
                failed2.store(true, Ordering::Relaxed);
            }
        });
        WebviewHandle { latest, failed, stop, input_tx, handle: Some(handle) }
    }

    // 새로 캡처된 프레임이 있으면 꺼내온다(없으면 이전과 같으니 None).
    pub fn take_frame(&self) -> Option<Frame> {
        self.latest.lock().unwrap().take()
    }

    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    // 클릭/스크롤 등을 실제 WebView2 렌더링 창으로 합성 입력해서 전달한다.
    pub fn send_input(&self, ev: WebviewInput) {
        let _ = self.input_tx.send(ev);
    }
}

impl Drop for WebviewHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

// 숨은 호스트 창의 WndProc — 아무것도 안 하고 기본 처리만 한다(이 창은 절대 안 보임).
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

// 실제 작업. 실패하면 None (호출부가 failed 플래그를 세운다). 이 스레드는 WebView2 가
// 요구하는 대로 STA(고전 메시지펌프) 로 초기화한다 — 프로젝트의 다른 스레드(MF/오디오)
// 는 전부 MTA 라서 완전히 독립적으로 분리해뒀다.
fn run(
    url: &str,
    w: u32,
    h: u32,
    latest: &Mutex<Option<Frame>>,
    stop: &AtomicBool,
    input_rx: Receiver<WebviewInput>,
) -> Option<()> {
    let _guard = ActiveGuard::new(); // 함수가 어떤 경로로 끝나든 드롭되면서 카운트가 줄어든다
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let class_name = wide("PalaceOSWebviewHost");
        let hinstance: windows::Win32::Foundation::HINSTANCE = GetModuleHandleW(None).ok()?.into();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassW(&wc);

        // 화면 밖 멀리, 안 보이는 상태로 만든다 — 이 창은 오직 WebView2 컨트롤을
        // 붙이기 위한 껍데기일 뿐, 사람이 볼 일은 절대 없다. WS_EX_TOOLWINDOW 를 줘야
        // 작업표시줄/Alt+Tab 목록에 안 뜬다(그냥 화면 밖에 두는 것만으론 목록엔 여전히 보임).
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP,
            -32000,
            -32000,
            w as i32,
            h as i32,
            None,
            None,
            hinstance,
            None,
        )
        .ok()?;
        // 창이 "떠 있는" 상태가 아니면 내부 Chromium 위젯이 입력 라우팅을 제대로 안
        // 붙일 수 있다 — 화면 밖에 있어서 사람 눈엔 안 보이지만, OS 상으로는 정상적인
        // "떠 있는" 창으로 만든다(SW_SHOWNOACTIVATE 라 포커스는 안 뺏어감).
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        let user_data_dir = user_data_dir_path();
        let _ = std::fs::create_dir_all(&user_data_dir);
        let user_data_dir_w = wide(&user_data_dir.to_string_lossy());

        let environment: ICoreWebView2Environment = {
            let (tx, rx) = channel();
            CreateCoreWebView2EnvironmentCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| {
                    CreateCoreWebView2EnvironmentWithOptions(
                        PCWSTR::null(),
                        PCWSTR(user_data_dir_w.as_ptr()),
                        None,
                        &handler,
                    )
                    .map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |_err, env| {
                    let _ = tx.send(env);
                    Ok(())
                }),
            )
            .ok()?;
            rx.recv().ok()??
        };

        let controller: ICoreWebView2Controller = {
            let (tx, rx) = channel();
            CreateCoreWebView2ControllerCompletedHandler::wait_for_async_operation(
                Box::new(move |handler| {
                    environment.CreateCoreWebView2Controller(hwnd, &handler).map_err(webview2_com::Error::WindowsError)
                }),
                Box::new(move |_err, ctrl| {
                    let _ = tx.send(ctrl);
                    Ok(())
                }),
            )
            .ok()?;
            rx.recv().ok()??
        };

        controller.SetBounds(RECT { left: 0, top: 0, right: w as i32, bottom: h as i32 }).ok()?;
        let _ = controller.SetIsVisible(true);
        // 시스템 DPI 배율에 영향받지 않도록 확대 배율을 명시적으로 1.0 으로 고정한다
        // (그래야 캡처 해상도 == 실제 CSS 픽셀 수가 그대로 맞아떨어진다).
        let _ = controller.SetZoomFactor(1.0);

        let core = controller.CoreWebView2().ok()?;
        let url_w = wide(url);
        core.Navigate(PCWSTR(url_w.as_ptr())).ok()?;

        // 디코딩(+리사이즈)을 별도 스레드로 돌려서, 그동안 이 스레드는 곧바로 다음
        // 캡처를 요청할 수 있게 한다(파이프라인). 큐 크기 1 + try_send 로 "항상 최신
        // 프레임만" 유지한다 — 디코딩이 밀리면 오래된 프레임은 그냥 버리지, 쌓아두지
        // 않는다(그래야 화면이 점점 뒤처지는 대신 항상 최신에 가깝게 유지된다).
        let (raw_tx, raw_rx) = sync_channel::<Vec<u8>>(1);
        let latest_decode = Arc::new(Mutex::new(None));
        // latest 는 &Mutex 라 스레드로 못 옮기니, 디코드 스레드는 자기만의 슬롯에 쓰고
        // 메인 루프가 매 프레임 그걸 latest 로 옮겨 담는다.
        let decode_thread = std::thread::spawn({
            let latest_decode = latest_decode.clone();
            move || {
                while let Ok(jpeg) = raw_rx.recv() {
                    if let Some(rgba) = decode_frame(&jpeg, w, h) {
                        *latest_decode.lock().unwrap() = Some(rgba);
                    }
                }
            }
        });

        // 캡처를 최대한 빠르게 계속 요청하면(예전 방식) 캡처마다 나오는 큰 RGBA
        // 프레임을 메인 스레드가 매번 GPU 텍스처로 업로드해야 해서, Official Site 를
        // 열어두면 게임 전체 프레임레이트가 떨어지는 원인이 됐다 — 스크린샷 캡처
        // 특성상 초당 30장이면 충분히 부드러워서, 그 이상은 어차피 안 보이는데
        // 업로드 비용만 늘리는 낭비였다. 그래서 캡처 자체를 30fps 로 상한을 둔다.
        const CAPTURE_INTERVAL: Duration = Duration::from_millis(33);
        let mut last_capture = std::time::Instant::now()
            .checked_sub(CAPTURE_INTERVAL)
            .unwrap_or_else(std::time::Instant::now); // 시작하자마자 첫 캡처가 나가게

        while !stop.load(Ordering::Relaxed) && !GLOBAL_STOP.load(Ordering::Relaxed) {
            if last_capture.elapsed() >= CAPTURE_INTERVAL {
                last_capture = std::time::Instant::now();
                if let Some(jpeg) = capture_raw(&core) {
                    let _ = raw_tx.try_send(jpeg); // 꽉 차 있으면(디코드가 못 따라오면) 그냥 버림
                }
            } else {
                // 캡처할 차례가 아니면 스레드를 잠깐 놓아준다 — 그래도 아래 입력/메시지
                // 펌프는 매번 돌려서 클릭/스크롤 반응성은 그대로 유지한다.
                std::thread::sleep(Duration::from_millis(4));
            }
            if let Some(rgba) = latest_decode.lock().unwrap().take() {
                *latest.lock().unwrap() = Some((w, h, rgba));
            }
            pump_pending();

            while let Ok(ev) = input_rx.try_recv() {
                match ev {
                    WebviewInput::Move(fx, fy) => exec_script(&core, move_script(fx, fy)),
                    WebviewInput::Down(x, y) => exec_script(&core, down_script(x, y)),
                    WebviewInput::Up(x, y) => exec_script(&core, up_script(x, y)),
                    WebviewInput::Wheel(delta) => {
                        exec_script(&core, format!("window.scrollBy(0,{});", -delta * 100.0));
                    }
                    WebviewInput::Type(c) => exec_script(&core, type_script(c)),
                    WebviewInput::Backspace => exec_script(&core, backspace_script()),
                    WebviewInput::Enter => exec_script(&core, enter_script()),
                }
            }
        }

        drop(raw_tx); // 디코드 스레드의 recv() 가 풀려서 자연스럽게 끝나게 한다
        let _ = decode_thread.join();

        let _ = controller.Close();
        let _ = DestroyWindow(hwnd);

        // 창을 닫자마자 유저 데이터 폴더를 지운다 — 근데 브라우저 하위 프로세스가
        // 파일을 바로는 안 놓아줄 수 있어서, 여기서 재시도를 기다리면 이 스레드를
        // join() 하는 메인 스레드(게임 자체)가 그동안 멈춰버린다. 그래서 기다리지
        // 않고 완전히 분리된(join 안 하는) 스레드에 맡겨서, 게임은 그대로 계속
        // 돌아가는 동안 백그라운드에서 몇 초에 걸쳐 재시도하게 한다. 그래도 못
        // 지우면(예: 게임 자체가 그새 꺼짐) 다음에 다시 열 때 미리 지우는 로직이
        // 마저 정리해준다.
        let cleanup_dir = cleanup_root_path();
        std::thread::spawn(move || {
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(300));
                if std::fs::remove_dir_all(&cleanup_dir).is_ok() {
                    break;
                }
            }
        });

        Some(())
    }
}

// fx,fy 는 0..1 비율 — 매번 그 순간의 window.innerWidth/innerHeight 로 픽셀 환산해서
// 캡처 해상도와 실제 CSS 뷰포트 크기가 안 맞아도 항상 정확한 위치를 가리키게 한다.
fn down_script(fx: f32, fy: f32) -> String {
    format!(
        "(function(){{var x=Math.round({fx}*window.innerWidth),y=Math.round({fy}*window.innerHeight);\
         var el=document.elementFromPoint(x,y);if(!el)return;window.__downEl=el;\
         el.dispatchEvent(new MouseEvent('mousedown',{{clientX:x,clientY:y,bubbles:true,cancelable:true,view:window,button:0,buttons:1}}));\
         if(typeof el.focus==='function'){{\
           try{{el.focus({{preventScroll:true}});}}catch(e){{el.focus();}}\
         }}}})();"
    )
}

// mouseup + click(누른 지점 요소 기준) 을 함께 보낸다 — 실제 브라우저에서도 mousedown
// 요소와 같은 요소 위에서 mouseup 하면 그 요소에 click 이 발생하는 것과 비슷하게 흉내.
fn up_script(fx: f32, fy: f32) -> String {
    format!(
        "(function(){{var x=Math.round({fx}*window.innerWidth),y=Math.round({fy}*window.innerHeight);\
         var el=document.elementFromPoint(x,y);if(!el)return;\
         var o={{clientX:x,clientY:y,bubbles:true,cancelable:true,view:window,button:0}};\
         el.dispatchEvent(new MouseEvent('mouseup',o));\
         (window.__downEl||el).dispatchEvent(new MouseEvent('click',o));window.__downEl=null;}})();"
    )
}

// mouseover/mouseout(및 enter/leave) 로 진입/이탈 전환을 흉내낸다 — JS 리스너 기반
// 호버 메뉴 등은 이걸로 반응한다(순수 CSS :hover 는 여전히 안 됨, 위 주석 참고).
//
// ⚠ CDP(Input.dispatchMouseEvent) 로 "진짜" 입력을 보내서 순수 CSS :hover 까지
// 켜보려고 시도했었는데, WebView2 는 화면 밖/숨겨진 호스트 창처럼 오프스크린으로
// 쓰는 구성에서 CDP 의 Input 도메인 호출이 에러 없이 조용히 씹힌다(공식적으로
// 지원 안 되는 조합 — MicrosoftEdge/WebView2Feedback 에도 비슷한 보고가 있다).
// 제대로 하려면 지금의 "일반 윈도우 호스팅(ICoreWebView2Controller)" 자체를
// "컴포지션 호스팅(ICoreWebView2CompositionController + SendMouseInput)" 으로
// 통째로 바꿔야 하는데, 이건 렌더링 파이프라인을 새로 짜야 하는 큰 작업이라
// 여기서는 되돌렸다.
//
// mouseenter/mouseleave 는 스펙상 버블링이 안 돼서, elementFromPoint 로 찾은 그
// 엘리먼트 하나에만 dispatch 하면 실제로 호버 처리를 하는 조상 엘리먼트(예:
// <button><span>아이콘</span>글자</button> 에서 elementFromPoint 는 안쪽 span 을
// 돌려주는데, 호버 스타일/리스너는 button 에 달려있는 흔한 구조)까지는 전달이 안
// 된다 — 그래서 실제 브라우저처럼 "공통 조상까지" 경로를 걸어가며 각 엘리먼트에
// 따로 dispatch 해준다. mouseover/mouseout 은 원래 버블링되니 그대로 el/prev 에만
// 보내면 조상 리스너에도 자연히 전달된다.
fn move_script(fx: f32, fy: f32) -> String {
    format!(
        "(function(){{var x=Math.round({fx}*window.innerWidth),y=Math.round({fy}*window.innerHeight);\
         var el=document.elementFromPoint(x,y);var prev=window.__hoverEl;\
         function path(n){{var p=[];while(n){{p.push(n);n=n.parentElement;}}return p;}}\
         if(prev!==el){{\
           if(prev)prev.dispatchEvent(new MouseEvent('mouseout',{{clientX:x,clientY:y,bubbles:true,cancelable:true,view:window,relatedTarget:el}}));\
           if(el)el.dispatchEvent(new MouseEvent('mouseover',{{clientX:x,clientY:y,bubbles:true,cancelable:true,view:window,relatedTarget:prev}}));\
           var pp=path(prev),ep=path(el),common=null;\
           for(var i=0;i<pp.length;i++){{if(ep.indexOf(pp[i])!==-1){{common=pp[i];break;}}}}\
           for(var i=0;i<pp.length&&pp[i]!==common;i++)pp[i].dispatchEvent(new MouseEvent('mouseleave',{{clientX:x,clientY:y,bubbles:false,cancelable:false,view:window,relatedTarget:el}}));\
           var toEnter=[];for(var i=0;i<ep.length&&ep[i]!==common;i++)toEnter.push(ep[i]);\
           for(var i=toEnter.length-1;i>=0;i--)toEnter[i].dispatchEvent(new MouseEvent('mouseenter',{{clientX:x,clientY:y,bubbles:false,cancelable:false,view:window,relatedTarget:prev}}));\
           window.__hoverEl=el;}}\
         if(el)el.dispatchEvent(new MouseEvent('mousemove',{{clientX:x,clientY:y,bubbles:true,cancelable:true,view:window,buttons:window.__downEl?1:0}}));}})();"
    )
}

// 문자 하나를 JS 문자열 리터럴 안에 안전하게 넣을 수 있게 이스케이프한다(작은따옴표/
// 백슬래시/줄바꿈 정도만 처리하면 충분 — 문자 하나짜리라 복잡한 이스케이프는 불필요).
fn js_char_literal(c: char) -> String {
    match c {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => String::new(), // 캐리지리턴은 그냥 무시
        _ => c.to_string(),
    }
}

// document.activeElement 에 문자 하나를 커서 위치에 끼워 넣는다. 합성 keydown 은
// 실제 텍스트 입력을 유발하지 않아서(신뢰된 입력만 인정) 직접 value 를 바꿔야 한다.
fn type_script(c: char) -> String {
    let esc = js_char_literal(c);
    format!(
        "(function(){{var el=document.activeElement;if(!el)return;var t=el.tagName;\
         if(t!=='INPUT'&&t!=='TEXTAREA'&&!el.isContentEditable)return;\
         if(typeof el.selectionStart==='number'){{\
           var s=el.selectionStart,e=el.selectionEnd;\
           el.value=el.value.slice(0,s)+'{esc}'+el.value.slice(e);\
           el.selectionStart=el.selectionEnd=s+1;\
         }}else{{el.value=(el.value||'')+'{esc}';}}\
         el.dispatchEvent(new Event('input',{{bubbles:true}}));}})();"
    )
}

fn backspace_script() -> String {
    "(function(){var el=document.activeElement;if(!el)return;var t=el.tagName;\
     if(t!=='INPUT'&&t!=='TEXTAREA'&&!el.isContentEditable)return;\
     if(typeof el.selectionStart==='number'){\
       var s=el.selectionStart,e=el.selectionEnd;\
       if(s===e){if(s>0){el.value=el.value.slice(0,s-1)+el.value.slice(e);s=s-1;}}\
       else{el.value=el.value.slice(0,s)+el.value.slice(e);}\
       el.selectionStart=el.selectionEnd=s;\
     }else{el.value=(el.value||'').slice(0,-1);}\
     el.dispatchEvent(new Event('input',{bubbles:true}));})();"
        .to_string()
}

// 텍스트를 직접 끼워넣진 않고 keydown/keyup 만 보낸다 — Enter 는 보통 "제출/검색"
// 처리를 위한 JS 리스너용이라 그걸로 충분하고, 텍스트박스 줄바꿈까지는 신경 안 쓴다.
fn enter_script() -> String {
    "(function(){var el=document.activeElement;if(!el)return;\
     var o={key:'Enter',code:'Enter',keyCode:13,which:13,bubbles:true,cancelable:true};\
     el.dispatchEvent(new KeyboardEvent('keydown',o));\
     el.dispatchEvent(new KeyboardEvent('keyup',o));})();"
        .to_string()
}

// 결과는 신경 안 쓰는 fire-and-forget 실행 — 매 프레임 여러 번 부를 수 있어서 응답을
// 기다리며 블로킹하면 캡처 루프가 밀린다.
fn exec_script(core: &ICoreWebView2, script: String) {
    let script_w = wide(&script);
    let handler = ExecuteScriptCompletedHandler::create(Box::new(|_err, _result| Ok(())));
    unsafe {
        let _ = core.ExecuteScript(PCWSTR(script_w.as_ptr()), &handler);
    }
}

// 대기 중인 윈도우 메시지를 블로킹 없이 다 처리한다. CapturePreview 자체는
// wait_for_async_operation 이 알아서 메시지를 펌프하며 기다리므로, 여기서는 캡처
// 사이사이에 남는 이벤트(내비게이션 완료 등)만 정리해주면 된다.
unsafe fn pump_pending() {
    unsafe {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

// WebView2 에게 캡처를 요청해서 압축된(JPEG) 바이트만 받아온다 — 디코딩은 일부러
// 안 한다. 디코딩(+ 필요하면 리사이즈)은 꽤 걸리는 CPU 작업이라, 여기서 같이 하면
// 그동안 다음 캡처 요청을 못 걸어서 초당 캡처 횟수가 줄어든다. 압축 바이트만 뽑아서
// 최대한 빨리 다음 캡처를 다시 요청할 수 있게, 디코딩은 별도 스레드로 넘긴다
// (run() 의 파이프라인 구조 참고).
unsafe fn capture_raw(core: &ICoreWebView2) -> Option<Vec<u8>> {
    unsafe {
        let stream = CreateStreamOnHGlobal(None, true).ok()?;
        let core = core.clone(); // COM 참조 카운트만 올리는 가벼운 복제 — 'static 클로저에 담기 위함.
        let (tx, rx) = channel::<windows::core::Result<()>>();
        CapturePreviewCompletedHandler::wait_for_async_operation(
            Box::new({
                let stream = stream.clone();
                move |handler| {
                    // JPEG 이 PNG 보다 인코딩이 훨씬 가벼워서(특히 이런 텍스트 많은
                    // 스크린샷) 초당 캡처 횟수가 눈에 띄게 늘어난다.
                    core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG, &stream, &handler)
                        .map_err(webview2_com::Error::WindowsError)
                }
            }),
            Box::new(move |err| {
                let _ = tx.send(err);
                Ok(())
            }),
        )
        .ok()?;
        rx.recv().ok()?.ok()?;

        use windows::Win32::System::Com::{STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET};
        let mut stat = STATSTG::default();
        stream.Seek(0, STREAM_SEEK_SET, None).ok()?;
        stream.Stat(&mut stat, STATFLAG_NONAME).ok()?;
        stream.Seek(0, STREAM_SEEK_SET, None).ok()?;
        let size = stat.cbSize as usize;
        if size == 0 {
            return None;
        }
        let mut jpeg = vec![0u8; size];
        let mut read = 0u32;
        stream.Read(jpeg.as_mut_ptr() as *mut _, size as u32, Some(&mut read)).ok().ok()?;
        jpeg.truncate(read as usize);
        Some(jpeg)
    }
}

// 압축 바이트 → RGBA. 별도 스레드에서 돌며 capture_raw 와 겹쳐서 실행된다.
fn decode_frame(jpeg: &[u8], w: u32, h: u32) -> Option<Vec<u8>> {
    let img = image::load_from_memory(jpeg).ok()?.to_rgba8();
    // 캡처 해상도가 요청한 w,h 와 다를 수 있어(스케일링 등) 우리 텍스처 크기에
    // 맞춰 리사이즈해둔다 — 호출부는 항상 w×h 짜리 버퍼를 받는다고 가정한다.
    let resized = if img.width() != w || img.height() != h {
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    Some(resized.into_raw())
}
