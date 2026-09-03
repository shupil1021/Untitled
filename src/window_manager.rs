//! 창 관리자: z순서, 드래그, 크기조절, 타이틀바 버튼(최소/최대/닫기).

use miniquad::RenderingBackend;

use crate::apps::{App, AppAction, DragGhost, MoveDest, Opened, WinInput};
use crate::foundation::FileId;
use crate::gfx::{Assets, Rect, Renderer};
use crate::scenes::Input;
use crate::ui::*;

const BORDER: f32 = 3.0;
const TITLE_H: f32 = 20.0;
const BTN: f32 = 16.0;

#[derive(Clone, Copy, PartialEq)]
pub enum WinState {
    Normal,
    Maximized,
    Minimized,
}

pub struct Window {
    pub id: u32,
    pub title: String,
    pub rect: Rect,
    restore: Rect,
    pub state: WinState,
    app: Box<dyn App>,
    file: Option<FileId>,
    resizable: bool,
    maximizable: bool,
    movable: bool,
    min_w: f32,
    min_h: f32,
    // 새로 연 창에 남은 "로딩 중" 시간(초) — 0 보다 크면 그 창은 아직 앱 내용을
    // 안 그리고(app.update() 를 아예 안 부른다) 대신 스피너만 보여준다. 옛날 느린
    // 컴퓨터에서 프로그램을 켜면 잠깐 창만 뜨고 안이 그려지기까지 멈칫하던 느낌을
    // 재현한다 — SPAWN_FREEZE_S 참고.
    spawn_freeze: f32,
}

// 창을 새로 열 때 그 안이 "로딩 중"으로 멈춰 보이는 시간(초). 너무 길면 그냥
// 느려 보이고 너무 짧으면 있는지도 모를 테니, 눈에 띄되 거슬리지 않는 선.
const SPAWN_FREEZE_S: f32 = 0.35;

enum Drag {
    Move { off: (f32, f32) },
    Resize { left: bool, right: bool, top: bool, bottom: bool },
}

const EDGE: f32 = 6.0; // 리사이즈 감지 여백

// 데스크톱이 처리해야 할 창-발 동작.
pub enum DeskAction {
    Unlock(FileId),
    Open(FileId),
    OpenPhoto(String),
    RequestErase,
    Download(FileId),
    DownloadPhoto(String),
    InstallComplete,
    DeletePermanently(FileId),
    MoveFiles(Vec<FileId>, MoveDest),
    EmptyTrash(Vec<FileId>),
    MarkMailRead(usize),
    Restore(Vec<FileId>),
    SendNewMail { to: String, subject: String, body: String, attachments: Vec<(FileId, String)> },
}

// 창 관리자에 넘기는 입력 상태.
pub struct Gui<'a> {
    pub mouse: (f32, f32),
    pub down: bool,
    pub clicked: bool,
    pub wheel: f32,
    pub dt: f32,
    pub time: f32,
    pub input: &'a Input,
}

pub struct WmFrame {
    pub actions: Vec<DeskAction>,
    pub consumed: bool,
    pub cursor: CursorKind,
    pub over_window: bool,
    // 지금 어떤 창 안에서 파일을 드래그하는 중이면 그 미리보기 — desktop.rs 가 창을
    // 전부 그린 뒤 클립 없이 맨 위에 그려서 창 경계 밖으로도 나갈 수 있게 한다.
    pub drag_ghost: Option<DragGhost>,
}

pub struct WindowManager {
    windows: Vec<Window>, // 순서 = z 순서 (마지막이 최상단/포커스)
    next_id: u32,
    drag: Option<Drag>,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowManager {
    pub fn new() -> WindowManager {
        WindowManager { windows: Vec::new(), next_id: 1, drag: None }
    }

    // 새 창을 진짜로 만들었으면 true, 이미 열려있어서 앞으로만 가져왔으면 false.
    // (nudge_last 처럼 "방금 새로 스폰된 창"에만 적용해야 하는 후처리가 이미 열려있던
    // 창에도 매번 적용돼서 계속 아래로 밀려 내려가는 문제가 있었다 — 그 구분용.)
    pub fn open(&mut self, opened: Opened, file: Option<FileId>, work: Rect) -> bool {
        // 같은 파일 창이 이미 있으면 새로 열지 않고 앞으로.
        if let Some(fid) = file
            && let Some(i) = self.windows.iter().position(|w| w.file == Some(fid))
        {
            self.windows[i].state = WinState::Normal;
            let w = self.windows.remove(i);
            self.windows.push(w);
            return false;
        }
        let (w, h) = opened.size;
        let n = self.windows.len() as f32;
        let cascade = (n % 6.0) * 18.0;
        // 중앙에서 살짝 위/왼쪽으로 치우치게 스폰시키던 -45/-40 오프셋이, 창 기본
        // 크기가 작업영역 높이에 가까워지면(예: Mail 창이 메뉴바/툴바/폴더 트리가
        // 추가되며 커진 경우) y 를 음수로 만들어 타이틀바가 화면 위로 잘려나가는
        // 문제가 있었다 — 작업영역 안에 완전히 들어오도록 클램프한다.
        let x = (work.x + (work.w - w) / 2.0 - 45.0 + cascade).clamp(work.x, (work.x + work.w - w).max(work.x));
        let y = (work.y + (work.h - h) / 2.0 - 40.0 + cascade).clamp(work.y, (work.y + work.h - h).max(work.y));
        let rect = Rect::new(x, y, w, h);
        let id = self.next_id;
        self.next_id += 1;
        // 최대화로 열면 작업영역을 꽉 채우고, 복원 크기는 중앙 배치 크기로.
        let (state, shown_rect) = if opened.maximized {
            (WinState::Maximized, work)
        } else {
            (WinState::Normal, rect)
        };
        self.windows.push(Window {
            id,
            title: opened.title,
            rect: shown_rect,
            restore: rect,
            state,
            app: opened.app,
            file,
            resizable: opened.resizable,
            maximizable: opened.maximizable,
            movable: opened.movable,
            min_w: opened.min_size.0,
            min_h: opened.min_size.1,
            spawn_freeze: SPAWN_FREEZE_S,
        });
        true
    }

    // open() 직후 방금 연 창의 스폰 위치를 세로로 살짝 옮기고 싶을 때 쓴다(다른
    // 창들의 기본 중앙 배치는 그대로 두고 특정 창만 다르게 스폰시키고 싶은 경우).
    // open() 은 항상 새(또는 이미 열려있어 앞으로 온) 창을 맨 뒤에 두므로 안전하다.
    pub fn nudge_last(&mut self, dy: f32) {
        if let Some(w) = self.windows.last_mut() {
            w.rect.y += dy;
            w.restore.y += dy;
        }
    }

    // 이 좌표가 (최소화되지 않은) 어떤 창 위에 있는지. 바탕화면 우클릭 메뉴처럼
    // "빈 바탕화면"에서만 떠야 하는 UI 가 창 위에서 안 뜨게 걸러낼 때 쓴다.
    pub fn any_window_at(&self, m: (f32, f32)) -> bool {
        self.windows.iter().any(|w| w.state != WinState::Minimized && w.rect.contains(m.0, m.1))
    }

    // 이 좌표 위에 있는 (최소화 안 된) 최상단 창이 연결된 FileId — 창이 없거나 파일과
    // 무관한 창(설정 등)이면 None. 바탕화면 아이콘을 열린 창 위로 끌어다 놓을 때
    // "어느 창에 놓았는지" 알아내는 데 쓴다. self.windows 는 뒤에서 앞(z-순서)이라
    // 뒤에서부터(rev) 찾아야 겹친 창 중 맨 위 것을 고른다.
    pub fn file_at(&self, m: (f32, f32)) -> Option<FileId> {
        self.windows.iter().rev().find(|w| w.state != WinState::Minimized && w.rect.contains(m.0, m.1)).and_then(|w| w.file)
    }

    // 특정 파일에 연결된 창이 지금 열려 있는지.
    pub fn is_open(&self, file: FileId) -> bool {
        self.windows.iter().any(|w| w.file == Some(file))
    }

    // 열려있는 창의 앱을 downcast 해서 상태를 읽으려고 쓴다(예: 새로고침 전에 File
    // Explorer 의 현재 탭 위치를 알아내는 것) — 그 파일의 창이 없으면 None.
    pub fn app_mut(&mut self, file: FileId) -> Option<&mut (dyn App + '_)> {
        match self.windows.iter_mut().find(|w| w.file == Some(file)) {
            Some(w) => Some(w.app.as_mut()),
            None => None,
        }
    }

    // 이미 열려 있는 창의 내용(app)만 새로 갈아끼운다 — 위치/크기/최소화 상태는 그대로
    // 유지된다. 예: 다운로드로 파일 목록이 바뀌었을 때 열려있는 File Explorer 를
    // 새로 열지 않고 내용만 새로고침. 그 파일의 창이 없으면 아무 것도 안 하고 false.
    pub fn refresh_app(&mut self, file: FileId, new_app: Box<dyn App>) -> bool {
        match self.windows.iter_mut().find(|w| w.file == Some(file)) {
            Some(w) => {
                w.app = new_app;
                true
            }
            None => false,
        }
    }

    // 이미 열려있는 창을 맨 앞(z순서 최상단)으로 가져오고 최소화 상태면 풀어준다 —
    // open() 은 그 파일 창이 아직 없을 때만 새로 열고, 이미 있으면 그냥 앞으로 가져오는
    // 로직까지 같이 들어있는데, refresh_app() 경로(예: 바탕화면 폴더를 열었을 때 이미
    // 열려있는 File Explorer 내용만 바꾸는 경우)는 그 포커스 처리를 안 거쳐서 다른
    // 창에 가려진 채로 내용만 바뀌는 문제가 있었다. refresh_app() 뒤에 같이 불러준다.
    pub fn focus(&mut self, file: FileId) {
        if let Some(i) = self.windows.iter().position(|w| w.file == Some(file)) {
            self.windows[i].state = match self.windows[i].state {
                WinState::Minimized => WinState::Normal,
                s => s,
            };
            let w = self.windows.remove(i);
            self.windows.push(w);
        }
    }

    // 지금 열려있는 창들의 (파일, 복원 크기/위치, 최대화 여부) — desktop.rs 가 매
    // 프레임 이걸로 자기 기억(window_geometry 맵)을 갱신해서 저장 파일에 같이 남긴다.
    // 최소화 상태는 따로 저장하지 않는다 — 아이콘을 다시 열 때 최소화된 채로 뜨는 건
    // 의미가 없어서, 다음에 열면 항상 보이는 상태로 뜨고 크기/위치만 이어받는다.
    // w.restore 를 그냥 쓰면 안 된다 — 이건 "최대화하기 직전 크기" 스냅샷일 뿐이라
    // toggle_max()/AppAction::Resize 때만 갱신되고, 타이틀바로 옮기거나 테두리로
    // 리사이즈하는 동안(frame() 의 Drag::Move/Resize)은 w.rect 만 바뀌고 w.restore 는
    // 창을 만들 때 값 그대로 멈춰있다 — 그래서 창을 옮기거나 크기를 바꾼 뒤 닫았다
    // 다시 열어도 처음 스폰됐을 때 위치로 돌아가는 문제가 있었다. 최대화 안 된
    // 동안엔 지금 실제로 보이는 w.rect 가 곧 "복원 크기" 이므로 그걸 쓴다.
    pub fn window_geometry(&self) -> Vec<(FileId, Rect, bool)> {
        self.windows
            .iter()
            .filter_map(|w| w.file.map(|f| (f, if w.state == WinState::Maximized { w.restore } else { w.rect }, w.state == WinState::Maximized)))
            .collect()
    }

    // 방금 새로 연 창에 저장해뒀던 크기/위치를 적용한다 — open() 은 항상 새 창을
    // 화면 중앙 캐스케이드 위치에 기본 크기로 스폰시키므로, 이미 그 파일에 대해
    // 기억해둔 자리가 있으면 이걸로 덮어쓴다. 해상도가 바뀌었거나 창을 화면 밖까지
    // 끌어놨다 저장한 경우를 대비해 작업영역 안으로 다시 당겨 넣는다(타이틀바 드래그
    // 클램프와 같은 여유 60px 규칙).
    pub fn set_geometry(&mut self, file: FileId, rect: Rect, maximized: bool, work: Rect) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.file == Some(file)) {
            let cw = rect.w.max(w.min_w);
            let ch = rect.h.max(w.min_h);
            let cx = rect.x.clamp(work.x - cw + 60.0, work.x + work.w - 60.0);
            let cy = rect.y.clamp(work.y, work.y + work.h - TITLE_H);
            w.restore = Rect::new(cx, cy, cw, ch);
            w.state = if maximized { WinState::Maximized } else { WinState::Normal };
            w.rect = if maximized { work } else { w.restore };
        }
    }

    // 작업표시줄용: (id, 제목, 포커스?, 최소화?). z순서와 무관하게 항상 생성 순서(id)로
    // 정렬해서 반환한다 — 안 그러면 포커스가 바뀔 때마다(=z순서가 바뀔 때마다)
    // 작업표시줄 버튼들의 위치가 뒤섞여 보인다.
    pub fn taskbar_items(&self) -> Vec<(u32, String, bool, bool)> {
        let front = self.windows.len().wrapping_sub(1);
        let mut items: Vec<(u32, String, bool, bool)> = self
            .windows
            .iter()
            .enumerate()
            .map(|(i, w)| {
                (w.id, w.title.clone(), i == front && w.state != WinState::Minimized, w.state == WinState::Minimized)
            })
            .collect();
        items.sort_by_key(|&(id, ..)| id);
        items
    }

    pub fn on_taskbar_click(&mut self, id: u32) {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            let front = self.windows.len() - 1;
            if self.windows[i].state == WinState::Minimized {
                self.windows[i].state = WinState::Normal;
                let w = self.windows.remove(i);
                self.windows.push(w);
            } else if i == front {
                self.windows[i].state = WinState::Minimized;
            } else {
                let w = self.windows.remove(i);
                self.windows.push(w);
            }
        }
    }

    pub fn frame(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, g: &Gui, work: Rect) -> WmFrame {
        let mut consumed = false;
        let mut app_click_front = false;
        let mut actions = Vec::new();
        let mut close_ids: Vec<u32> = Vec::new();

        // 창 관리자 차원의 커서(리사이즈/이동 중 손모양) — 클릭 여부와 무관하게 매 프레임
        // 마우스 아래 최상단 창을 훑어서 미리 보여준다.
        let mut wm_cursor = CursorKind::Arrow;
        // 마우스가 (드래그 중이거나) 어떤 창 위에 있는지 — 데스크톱 쪽에서 아이콘 Hand
        // 커서를 계산할 때 창에 가려진 아이콘 위인데도 Hand 가 뜨는 걸 막는 데 쓴다.
        let mut over_window = false;
        if let Some(drag) = &self.drag {
            over_window = true;
            wm_cursor = match *drag {
                Drag::Move { .. } => CursorKind::Hand,
                Drag::Resize { left, right, top, bottom } => resize_cursor_kind(left, right, top, bottom),
            };
        } else {
            for i in (0..self.windows.len()).rev() {
                let w = &self.windows[i];
                if w.state == WinState::Minimized || !w.rect.contains(g.mouse.0, g.mouse.1) {
                    continue;
                }
                over_window = true;
                let can_resize = w.resizable && w.state == WinState::Normal;
                if can_resize
                    && let Some((l, r_, t, b)) = resize_edges(w.rect, g.mouse.0, g.mouse.1)
                {
                    wm_cursor = resize_cursor_kind(l, r_, t, b);
                }
                break;
            }
        }

        // 1) 진행 중인 드래그
        if let Some(drag) = &self.drag {
            if g.down && let Some(w) = self.windows.last_mut() {
                consumed = true;
                match *drag {
                    Drag::Move { off } => {
                        w.rect.x = (g.mouse.0 - off.0).clamp(work.x - w.rect.w + 60.0, work.x + work.w - 60.0);
                        w.rect.y = (g.mouse.1 - off.1).clamp(work.y, work.y + work.h - TITLE_H);
                    }
                    Drag::Resize { left, right, top, bottom } => {
                        if right {
                            w.rect.w = (g.mouse.0 - w.rect.x).max(w.min_w);
                        }
                        if bottom {
                            w.rect.h = (g.mouse.1 - w.rect.y).max(w.min_h);
                        }
                        if left {
                            let right_x = w.rect.x + w.rect.w;
                            let nx = g.mouse.0.min(right_x - w.min_w);
                            w.rect.w = right_x - nx;
                            w.rect.x = nx;
                        }
                        if top {
                            let bottom_y = w.rect.y + w.rect.h;
                            let ny = g.mouse.1.min(bottom_y - w.min_h);
                            w.rect.h = bottom_y - ny;
                            w.rect.y = ny;
                        }
                    }
                }
            } else {
                self.drag = None;
            }
        } else if g.clicked {
            // 최상단부터 히트 테스트
            let mut hit = None;
            for i in (0..self.windows.len()).rev() {
                if self.windows[i].state != WinState::Minimized
                    && self.windows[i].rect.contains(g.mouse.0, g.mouse.1)
                {
                    hit = Some(i);
                    break;
                }
            }
            if let Some(i) = hit {
                consumed = true;
                let w = self.windows.remove(i);
                self.windows.push(w);
                let idx = self.windows.len() - 1;
                let rect = self.windows[idx].rect;
                let (m0, m1) = g.mouse;
                // 크기고정 창은 테두리 리사이즈 불가. 최대화/복원은 별도 플래그
                // (maximizable) 로 따로 관리한다 — 테두리 리사이즈는 막되 최대화/복원
                // 버튼만 허용하고 싶은 앱이 있을 수 있어서 둘을 분리했다.
                let can_resize = self.windows[idx].resizable && self.windows[idx].state == WinState::Normal;
                let edges = resize_edges(rect, m0, m1);
                if close_btn(rect).contains(m0, m1) {
                    close_ids.push(self.windows[idx].id);
                } else if max_btn(rect).contains(m0, m1) {
                    if self.windows[idx].maximizable {
                        self.toggle_max(idx, work);
                    }
                } else if min_btn(rect).contains(m0, m1) {
                    self.windows[idx].state = WinState::Minimized;
                } else if can_resize && edges.is_some() {
                    let (left, right, top, bottom) = edges.unwrap();
                    self.drag = Some(Drag::Resize { left, right, top, bottom });
                } else if self.windows[idx].movable && title_bar(rect).contains(m0, m1) {
                    self.drag = Some(Drag::Move { off: (m0 - rect.x, m1 - rect.y) });
                } else if client(rect).contains(m0, m1) {
                    app_click_front = true;
                }
            }
        }

        // 휠 스크롤은 포커스(맨 앞 창)가 아니라 지금 마우스가 올라가 있는 창이 받는다
        // — 여러 창이 떠 있을 때 뒤쪽 창 위에서 휠을 굴렸는데 엉뚱하게 맨 앞 창이
        // 스크롤되던 문제를 고치는 것. 겹쳐 있으면 그 중 z-순서상 가장 위(마지막)
        // 것 하나만 받아야 하니 뒤에서부터 찾는다.
        let wheel_target = (0..self.windows.len()).rev().find(|&i| {
            self.windows[i].state != WinState::Minimized && self.windows[i].rect.contains(g.mouse.0, g.mouse.1)
        });

        // 2) 그리기 + 앱 갱신 (뒤에서 앞으로)
        let front = self.windows.len().wrapping_sub(1);
        let mut drag_ghost = None;
        for i in 0..self.windows.len() {
            // 언어처럼 실시간으로 바뀔 수 있는 값에서 제목이 나오는 앱(App::title() 을
            // 오버라이드한 앱)만 매 프레임 갱신한다 — 최소화된 창도 작업표시줄
            // 버튼에 제목 글자가 그대로 보이므로, 아래에서 최소화된 창을 건너뛰기
            // 전에 먼저 갱신해야 한다. 안 그러면 창을 최소화해둔 채로 언어를 바꿨을
            // 때 그 창의 작업표시줄 버튼만 예전 언어로 멈춰 있는다(제보받은 버그).
            if let Some(t) = self.windows[i].app.title() {
                self.windows[i].title = t;
            }
            if self.windows[i].state == WinState::Minimized {
                continue;
            }
            let focused = i == front;
            draw_window(r, &self.windows[i], focused);
            let area = client(self.windows[i].rect);
            // 이 창의 영역 안에 마우스가 있어도, 그 지점에서 실제로 화면에 보이는
            // (z-순서상 더 위인) 다른 창이 덮고 있으면 "안 보이는 곳에서 마우스가
            // 올라가 있다"고 앱에 거짓 정보를 주면 안 된다 — 안 그러면 뒤에 완전히
            // 가려진 창의 버튼도 커서 모양이 바뀌거나 호버 표시가 뜬다. 마우스 위치가
            // topmost(=wheel_target)가 아닌데 자기 rect 안엔 들어와 있는 경우에만
            // 화면 밖 좌표로 숨겨서 그 창의 hit-test 가 전부 실패하게 만든다. focus된
            // (맨 위) 창은 정의상 항상 topmost 이므로 이 처리에 걸리지 않는다.
            let occluded = self.windows[i].rect.contains(g.mouse.0, g.mouse.1) && wheel_target != Some(i);
            let eff_mouse = if occluded { (-1.0e6, -1.0e6) } else { g.mouse };
            let win_in = WinInput {
                // 앱은 절대 가상좌표로 그리고 판정하므로 마우스도 절대좌표로 준다.
                mouse: eff_mouse,
                mouse_down: g.down && focused,
                mouse_clicked: focused && app_click_front,
                focused,
                wheel: if wheel_target == Some(i) { g.wheel } else { 0.0 },
                dt: g.dt,
                time: g.time,
                input: g.input,
            };
            r.set_clip(Some(area));
            // 방금 연 창이면 잠깐 로딩 스피너만 보여주고, 그동안은 app.update() 를
            // 아예 안 불러서 그 안의 클릭/타이핑도 안 먹는다 — 옛날 컴퓨터에서
            // 프로그램을 켜면 잠깐 멈칫하던 느낌.
            let action = if self.windows[i].spawn_freeze > 0.0 {
                self.windows[i].spawn_freeze -= g.dt;
                r.rect(area.x, area.y, area.w, area.h, FACE);
                draw_spinner(r, area.x + area.w / 2.0, area.y + area.h / 2.0, 10.0, g.time);
                AppAction::None
            } else {
                self.windows[i].app.update(ctx, r, assets, area, &win_in)
            };
            r.set_clip(None);
            // 이 창이 지금 드래그 미리보기를 갖고 있으면 기억해둔다 — 창을 다 그린
            // 뒤 desktop.rs 가 클립 없이 맨 위에 그려서 창 경계 밖으로도 나갈 수 있다.
            if let Some(ghost) = self.windows[i].app.drag_ghost() {
                drag_ghost = Some(ghost);
            }
            match action {
                AppAction::None => {}
                AppAction::Close => close_ids.push(self.windows[i].id),
                AppAction::Unlock(id) => {
                    actions.push(DeskAction::Unlock(id));
                    close_ids.push(self.windows[i].id);
                }
                AppAction::Open(id) => actions.push(DeskAction::Open(id)),
                AppAction::OpenPhoto(filename) => actions.push(DeskAction::OpenPhoto(filename)),
                AppAction::RequestErase => actions.push(DeskAction::RequestErase),
                AppAction::Download(id) => actions.push(DeskAction::Download(id)),
                AppAction::DownloadPhoto(filename) => actions.push(DeskAction::DownloadPhoto(filename)),
                // 설치 마법사는 진행바가 다 찬 순간 이걸 한 번만 보내고(바탕화면
                // 아이콘이 그 타이밍에 생기게) 창은 그대로 열어둔 채 Finish 페이지를
                // 계속 보여준다 — Unlock/Download 와 달리 여기선 창을 안 닫는다.
                AppAction::InstallComplete => actions.push(DeskAction::InstallComplete),
                // 앱이 페이지를 넘어가면서 필요한 화면 크기가 확 바뀔 때(예: HexTool 이
                // 파일 고르는 작은 창에서 편집용 큰 창으로) 쓴다. 중심을 그대로 두고
                // 크기만 바꾸되, 작업영역 밖으로 나가면 안으로 당겨 넣는다.
                AppAction::Resize(w, h) => {
                    let win = &mut self.windows[i];
                    let cx = win.rect.x + win.rect.w / 2.0;
                    let cy = win.rect.y + win.rect.h / 2.0;
                    let nx = (cx - w / 2.0).clamp(work.x, (work.x + work.w - w).max(work.x));
                    let ny = (cy - h / 2.0).clamp(work.y, (work.y + work.h - h).max(work.y));
                    win.rect = Rect::new(nx, ny, w, h);
                    win.restore = win.rect;
                }
                AppAction::DeletePermanently(id) => {
                    actions.push(DeskAction::DeletePermanently(id));
                    close_ids.push(self.windows[i].id);
                }
                AppAction::MoveFiles(ids, dest) => actions.push(DeskAction::MoveFiles(ids, dest)),
                AppAction::EmptyTrash(ids) => actions.push(DeskAction::EmptyTrash(ids)),
                AppAction::MarkMailRead(i) => actions.push(DeskAction::MarkMailRead(i)),
                AppAction::SendNewMail { to, subject, body, attachments } => {
                    actions.push(DeskAction::SendNewMail { to, subject, body, attachments })
                }
                AppAction::Restore(ids) => actions.push(DeskAction::Restore(ids)),
            }
        }

        if !close_ids.is_empty() {
            self.windows.retain(|w| !close_ids.contains(&w.id));
        }

        WmFrame { actions, consumed, cursor: wm_cursor, over_window, drag_ghost }
    }

    fn toggle_max(&mut self, idx: usize, work: Rect) {
        let w = &mut self.windows[idx];
        if w.state == WinState::Maximized {
            w.rect = w.restore;
            w.state = WinState::Normal;
        } else {
            w.restore = w.rect;
            w.rect = work;
            w.state = WinState::Maximized;
        }
    }
}

// ---- 창 영역 계산 ----

fn title_bar(rect: Rect) -> Rect {
    Rect::new(rect.x + BORDER, rect.y + BORDER, rect.w - 2.0 * BORDER, TITLE_H)
}
fn client(rect: Rect) -> Rect {
    Rect::new(
        rect.x + BORDER,
        rect.y + BORDER + TITLE_H,
        rect.w - 2.0 * BORDER,
        rect.h - 2.0 * BORDER - TITLE_H,
    )
}
fn close_btn(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - BORDER - BTN - 2.0, rect.y + BORDER + 2.0, BTN, TITLE_H - 4.0)
}
fn max_btn(rect: Rect) -> Rect {
    let c = close_btn(rect);
    Rect::new(c.x - (BTN + 2.0), c.y, BTN, c.h)
}
fn min_btn(rect: Rect) -> Rect {
    let c = close_btn(rect);
    Rect::new(c.x - 2.0 * (BTN + 2.0), c.y, BTN, c.h)
}
fn grip(rect: Rect) -> Rect {
    Rect::new(rect.x + rect.w - 16.0, rect.y + rect.h - 16.0, 16.0, 16.0)
}

// 마우스가 창 어느 변/꼭짓점 근처인지. 하나라도 걸리면 Some((left,right,top,bottom)).
fn resize_edges(rect: Rect, mx: f32, my: f32) -> Option<(bool, bool, bool, bool)> {
    let left = (mx - rect.x).abs() < EDGE;
    let right = (mx - (rect.x + rect.w)).abs() < EDGE;
    let top = (my - rect.y).abs() < EDGE;
    let bottom = (my - (rect.y + rect.h)).abs() < EDGE;
    if left || right || top || bottom {
        Some((left, right, top, bottom))
    } else {
        None
    }
}

// 어느 변/모서리인지에 따라 알맞은 리사이즈 커서 모양을 고른다.
fn resize_cursor_kind(left: bool, right: bool, top: bool, bottom: bool) -> CursorKind {
    if (left && top) || (right && bottom) {
        CursorKind::ResizeNWSE
    } else if (right && top) || (left && bottom) {
        CursorKind::ResizeNESW
    } else if left || right {
        CursorKind::ResizeEW
    } else {
        CursorKind::ResizeNS
    }
}

fn draw_window(r: &mut Renderer, w: &Window, focused: bool) {
    raised(r, w.rect.x, w.rect.y, w.rect.w, w.rect.h);
    let tb = title_bar(w.rect);
    let col = if focused { NAVY } else { GRAY };
    r.rect(tb.x, tb.y, tb.w, tb.h, col);
    // 제목이 버튼과 겹치지 않도록 버튼 앞까지만 그린다.
    let title_max = (min_btn(w.rect).x - tb.x - 6.0).max(0.0);
    r.text_clipped(tb.x + 4.0, tb.y - 1.0, &w.title, 1.0, WHITE, title_max);

    let mn = min_btn(w.rect);
    raised(r, mn.x, mn.y, mn.w, mn.h);
    r.rect(mn.x + 4.0, mn.y + mn.h - 6.0, mn.w - 8.0, 2.0, BLACK);

    let mx = max_btn(w.rect);
    raised(r, mx.x, mx.y, mx.w, mx.h);
    border(r, mx.x + 3.0, mx.y + 3.0, mx.w - 6.0, mx.h - 6.0, BLACK);
    r.rect(mx.x + 3.0, mx.y + 3.0, mx.w - 6.0, 2.0, BLACK);

    let cl = close_btn(w.rect);
    raised(r, cl.x, cl.y, cl.w, cl.h);
    draw_x(r, cl.x + 4.0, cl.y + 4.0, cl.h - 8.0);

    // 크기조절 그립 표시 (우하단 사선) — 크기 고정 창에는 표시하지 않는다.
    if w.resizable && w.state == WinState::Normal {
        let gp = grip(w.rect);
        for k in 0..3 {
            let o = k as f32 * 4.0;
            r.rect(gp.x + 4.0 + o, gp.y + 12.0 - o, 3.0, 3.0, WHITE);
            r.rect(gp.x + 3.0 + o, gp.y + 11.0 - o, 2.0, 2.0, GRAY);
        }
    }
}

// 로비의 Settings 오버레이(창 관리자 없이 떠 있는 패널)도 같은 X 아이콘으로 닫기
// 버튼을 그리려고 pub(crate) 로 열어뒀다.
pub(crate) fn draw_x(r: &mut Renderer, x: f32, y: f32, s: f32) {
    let mut k = 0.0;
    while k < s {
        r.rect(x + k, y + k, 2.0, 2.0, BLACK);
        r.rect(x + k, y + s - 2.0 - k, 2.0, 2.0, BLACK);
        k += 2.0;
    }
}

