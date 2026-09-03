//! 탐색기 — 고전 Windows 탐색기 느낌의 주소창 + 왼쪽 트리 사이드바 + Name/Size
//! 컬럼 목록/아이콘 격자 + 하단 상태바. 바탕화면의 File Explorer 아이콘은 탭이 있는
//! 모드(new_tabbed)로 열려서 Downloads/Desktop/Videos/Images 를 트리에서 오갈 수
//! 있고, 그 외의 일반 폴더는 탭 없이(new) 내용물만 보여준다(그 경우 트리 없이
//! 주소창/상태바만 두른다). 메뉴바/툴바(뒤로·앞으로·Cut·Copy 등)는 장식 요소가
//! 너무 많아 뺐다 — 보기 전환(격자/목록)만 우측 하단 버튼으로 남아있다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{category_label, display_name, FileId, Language, Settings, MY_COMPUTER_NAME};
use crate::gfx::{Assets, Color, Rect, Renderer, CELL_H};
use crate::strings::{common, explorer as s, t};
use crate::ui::*;

use super::widgets::{ease_scroll, icon_grid, scrollbar};
use super::{App, AppAction, DragGhost, MoveDest, WinInput};

type Items = Vec<(FileId, String, IconType)>;

// is_folder_like() 는 "크기 칸을 비워둘 폴더류인가"(목록 뷰) 기준이라 휴지통도
// 포함한다 — 하지만 왼쪽 트리에서 "펼쳐서 탭으로 끼워 넣을 수 있는가"는 다른
// 질문이다: 휴지통은 실제 파일 취급이 아니라 독립된 프로그램(RecycleBinApp)으로
// 취급해야 하므로, 트리 확장(+/- 박스, next_unexpanded_child)에서는 일반
// 폴더만 후보로 친다.
fn tree_expandable(icon: &IconType) -> bool {
    matches!(icon, IconType::Folder)
}

const SIDEBAR_MIN: f32 = 64.0;
const SIDEBAR_MAX_FRAC: f32 = 0.6; // 창 폭의 이 비율보다 더 넓게는 못 끈다
// 글자 높이는 CELL_H(22)*스케일 이라, 그보다 얕은 sunken 박스에 넣으면 위아래로
// 삐져나와 보인다(테두리 안에 안 들어가고 걸침) — 0.8 스케일 글자 높이(17.6px)보다
// 넉넉하게 잡는다.
const ADDR_H: f32 = 28.0;
const STATUS_H: f32 = 22.0;
const VIEW_TOGGLE_H: f32 = 24.0; // 우측 하단 격자/목록 전환 버튼 자리

// 우측 하단 보기 전환 버튼에 쓰는 작은 아이콘들 — 2x2 사각형(격자 보기) / 가로줄 3개(목록 보기).
fn draw_grid_glyph(r: &mut Renderer, x: f32, y: f32, s: f32, color: Color) {
    let cell = (s - 5.0) / 2.0;
    for row in 0..2 {
        for col in 0..2 {
            r.rect(x + col as f32 * (cell + 3.0), y + row as f32 * (cell + 3.0), cell, cell, color);
        }
    }
}
fn draw_list_glyph(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let gap = (h - 6.0) / 2.0;
    for i in 0..3 {
        r.rect(x, y + i as f32 * gap, w, 2.0, color);
    }
}

// 최신순(FileId 큰 쪽이 나중에 생성된 것) 가로 목록 보기 — 고전 탐색기처럼 위에
// Name/Size 컬럼 헤더가 달려있다. icon_grid 와 구조는 비슷하지만 칸이 아니라 전체
// 폭을 쓰는 줄 단위라 따로 그린다. icon_grid 처럼 self 를 통째로 빌리지 않고 필요한
// 필드만 따로 받는다 — 호출부에서 self.tabs 를 빌린 items 와 self.scroll 등을 동시에
// 빌려야 해서, 메서드로 만들면 &mut self 전체가 items 의 불변 대여와 충돌한다.
const SIZE_COL_W: f32 = 64.0;
const LIST_HEADER_H: f32 = 22.0; // 0.85 스케일 글자 높이(CELL_H*0.85≈18.7)보다 여유 있게

#[allow(clippy::too_many_arguments)]
fn draw_list_view(
    r: &mut Renderer,
    assets: &Assets,
    win: &WinInput,
    area: Rect,
    items: &Items,
    selected: &mut Vec<usize>,
    marquee_start: &mut Option<(f32, f32)>,
    prev_down: &mut bool,
    scroll: &mut f32,
    scroll_disp: &mut f32,
    sb_drag: &mut bool,
    smooth: bool,
    lang: Language,
) -> Option<(usize, Rect)> {
    r.rect(area.x, area.y, area.w, area.h, WHITE);
    // 항목이 하나도 없어도(빈 폴더) 마퀴 드래그 자체는 그대로 동작해야 한다 — 여기서
    // 바로 반환하면 폴더가 비어있는 동안은 드래그를 시작할 수조차 없었다. order/
    // total_h 등은 items.len()==0 이면 자연히 0이 되니 아래 로직이 안전하게 처리한다.

    // 컬럼 헤더 — 고전 탐색기의 정렬 버튼처럼 raised 바 두 칸(Name/Size).
    let name_col_w = (area.w - SIZE_COL_W).max(20.0);
    raised(r, area.x, area.y, name_col_w, LIST_HEADER_H);
    raised(r, area.x + name_col_w, area.y, SIZE_COL_W, LIST_HEADER_H);
    let header_ty = area.y + (LIST_HEADER_H - CELL_H * 0.85) / 2.0;
    let name_label = t(lang, common::NAME);
    let size_label = t(lang, common::SIZE);
    r.text(area.x + 6.0, header_ty, name_label, 0.85, BLACK);
    let size_label_w = r.text_width(size_label, 0.85);
    r.text(area.x + name_col_w + SIZE_COL_W - size_label_w - 8.0, header_ty, size_label, 0.85, BLACK);

    const ROW_H: f32 = 22.0;
    let list_top = area.y + LIST_HEADER_H;
    let list_h = (area.h - LIST_HEADER_H).max(0.0);
    let list_area = Rect::new(area.x, list_top, area.w, list_h);

    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| items[b].0.cmp(&items[a].0));

    let total_h = items.len() as f32 * ROW_H;
    let max_scroll = (total_h - list_h).max(0.0);
    // 마우스가 이 목록 위에 있을 때만 휠을 먹는다 — 안 그러면 트리 사이드바 등 다른
    // 곳에 마우스가 있어도 이 목록이 같이 스크롤돼버린다.
    if area.contains(win.mouse.0, win.mouse.1) {
        *scroll -= win.wheel / 120.0 * (ROW_H * 3.0);
    }
    *scroll = scroll.clamp(0.0, max_scroll);
    ease_scroll(scroll_disp, *scroll, win.dt, smooth);
    let scroll_px = *scroll_disp;

    // 화면에 보이는 행(row, order 상의 위치) 하나의 사각형 — 마퀴 판정과 클릭 판정에
    // 같이 쓴다(icon_grid 의 item_rect 와 같은 요령).
    let row_rect = |row: usize| -> Rect { Rect::new(area.x, list_top + row as f32 * ROW_H - scroll_px, area.w, ROW_H).intersect(&list_area) };

    // 빈 자리를 누르면 고무줄 선택 시작, 항목 위를 누르면(아래 루프에서) 그 항목만(또는
    // 이미 다중 선택돼 있으면 선택 유지) 처리 — icon_grid 와 완전히 같은 패턴.
    let over_any_row = (0..order.len()).any(|row| row_rect(row).contains(win.mouse.0, win.mouse.1));
    if win.mouse_clicked && !over_any_row && list_area.contains(win.mouse.0, win.mouse.1) {
        *marquee_start = Some(win.mouse);
        selected.clear();
    }
    if let Some(ms) = *marquee_start
        && win.mouse_down
    {
        let mr = Rect::new(ms.0.min(win.mouse.0), ms.1.min(win.mouse.1), (ms.0 - win.mouse.0).abs(), (ms.1 - win.mouse.1).abs());
        *selected = (0..order.len()).filter(|&row| row_rect(row).intersects(&mr)).map(|row| order[row]).collect();
    }
    if *prev_down && !win.mouse_down {
        *marquee_start = None;
    }
    *prev_down = win.mouse_down;

    // set_clip 은 스택이 아니라 통째로 덮어쓰는 값이라(gfx.rs::push_quad), 여기서
    // None 으로 지워버리면 winman.rs 가 걸어둔 "창 밖으로 안 나가게" 클립까지 같이
    // 풀려서, 마우스를 창 밖까지 끌고 나간 채 마퀴를 그리면 창 경계를 뚫고 화면
    // 전체에 그려진다 — 그래서 끝나고 나서 None 대신 "들어올 때 걸려있던 클립"으로 되돌린다.
    let outer_clip = r.clip();
    let mut clicked = None;
    r.set_clip(Some(list_area));
    for (row, &idx) in order.iter().enumerate() {
        let ry = list_top + row as f32 * ROW_H - scroll_px;
        if ry + ROW_H < list_top || ry > list_top + list_h {
            continue;
        }
        let (_id, name, icon) = &items[idx];
        // 항목 이름은 원문 그대로 담겨 있다가 여기서 매 프레임 다시 번역된다 —
        // 창을 이미 연 채로 언어를 바꿔도 그 자리에서 바로 반영되게 하려고
        // (apps/mod.rs::folder_items 주석 참고).
        let display = display_name(lang, name);
        let sel = selected.contains(&idx);
        let hover = row_rect(row).contains(win.mouse.0, win.mouse.1);
        if sel {
            r.rect(area.x, ry, area.w, ROW_H, [0.78, 0.88, 1.0, 1.0]); // 연한 파랑 채움
            border(r, area.x, ry, area.w, ROW_H, NAVY); // 진한 파랑 테두리
        } else if hover {
            r.rect(area.x, ry, area.w, ROW_H, [0.92, 0.95, 1.0, 1.0]);
        }
        draw_icon(r, assets, icon, area.x + 4.0, ry + 2.0, 18.0);
        r.text_clipped(area.x + 28.0, ry + 3.0, &display, 0.85, BLACK, name_col_w - 34.0);
        // 폴더류(File Explorer 의 카테고리, 잠금 풀린 폴더)는 실제 파일 개념이
        // 아니라 크기를 안 보여준다 — 진짜 탐색기도 폴더 칸은 비워둔다.
        if !is_folder_like(icon) {
            let size_text = "--";
            let tw = r.text_width(size_text, 0.85);
            r.text(area.x + name_col_w + SIZE_COL_W - tw - 8.0, ry + 3.0, size_text, 0.85, GRAY);
        }
        if win.mouse_clicked && marquee_start.is_none() && hover {
            // 마퀴 드래그가 아닌 단순 클릭 — 이미 다중 선택에 들어있는 항목이면 그
            // 선택을 유지한다(그래야 여러 개를 선택한 채로 눌러서 옮길 수 있다).
            if !selected.contains(&idx) {
                *selected = vec![idx];
            }
            clicked = Some((idx, row_rect(row)));
        }
    }
    r.rect(area.x + name_col_w, list_top, 1.0, list_h, [0.85, 0.85, 0.85, 1.0]); // 컬럼 세로 구분선

    // 마퀴 박스도 아직 클립이 걸린 채로 그린다(풀어버린 뒤에 그리면 창 밖으로
    // 드래그했을 때 사각형이 화면 전체에 삐져나온다).
    if let Some(ms) = *marquee_start {
        r.rect(
            ms.0.min(win.mouse.0), ms.1.min(win.mouse.1), (ms.0 - win.mouse.0).abs().max(1.0), (ms.1 - win.mouse.1).abs().max(1.0),
            [0.2, 0.4, 0.9, 0.25],
        );
        border(
            r, ms.0.min(win.mouse.0), ms.1.min(win.mouse.1), (ms.0 - win.mouse.0).abs().max(1.0), (ms.1 - win.mouse.1).abs().max(1.0),
            [0.2, 0.4, 0.9, 0.9],
        );
    }
    r.set_clip(outer_clip);

    if max_scroll > 0.0 {
        let sb_x = area.x + area.w - 8.0;
        let frac = (list_h / total_h).min(1.0);
        scrollbar(r, win, sb_x, list_top, 8.0, list_h, frac, *scroll_disp, scroll, max_scroll, sb_drag);
    }
    clicked
}

// (탭 이름, 항목들, 부모 카테고리 이름 — Some 이면 그 카테고리의 하위 폴더, 그
// 폴더 자신의 FileId — Some 이면 드릴다운으로 끼워진 하위 폴더 탭이라 새로고침 때
// current_location() 으로 다시 찾아 되돌아갈 수 있다. 고정 카테고리 탭 4개는 None).
type Tab = (String, Items, Option<String>, Option<FileId>);

// 새로고침(refresh_app) 전에 지금 보고 있던 위치를 기억해뒀다가, 새로고침 후에도
// 그 자리로 되돌아가려고 쓴다 — as_any_mut 로 downcast 한 뒤 current_location() 을
// 불러서 얻는다.
pub enum ExplorerLocation {
    Category(String), // 고정 카테고리 탭 (Downloads/Desktop/Videos/Images) — 이름으로 다시 찾는다
    Folder(FileId),    // 드릴다운으로 들어간 하위 폴더 — 그 폴더의 FileId 로 다시 찾는다
}

pub struct ExplorerApp {
    tabs: Vec<Tab>, // 탭 하나짜리면 트리 사이드바 자체를 안 그린다.
    tab: usize,
    sidebar_w: f32,
    divider_drag: bool,
    list_view: bool, // false = 아이콘 격자, true = Name/Size 컬럼 가로 목록
    selected: Vec<usize>, // 다중 선택(바탕화면처럼 고무줄 드래그로 여러 개 고를 수 있음)
    marquee_start: Option<(f32, f32)>, // 고무줄 드래그 시작점 — icon_grid 가 프레임 사이 유지하려고 여기 보관
    prev_down: bool,
    scroll: f32,
    scroll_disp: f32,
    sb_drag: bool,
    last_click: f32,
    last_idx: i32,
    // 사이드바로 드래그해서 파일을 옮기는 중이면 그 시작점 — 일정 거리 넘게
    // 움직여야(item_drag_moved) 진짜 드래그로 친다(그냥 클릭과 구분하려고).
    item_drag: Option<(f32, f32)>,
    // 눌렀던 항목의 좌상단에서 클릭 지점까지의 오프셋 — 고스트가 "잡은 지점" 그대로
    // 커서를 따라오게 하는 데 쓴다.
    item_drag_offset: (f32, f32),
    // drag_ghost() 는 win: &WinInput 없이(update() 밖에서, 매 프레임과 무관하게)
    // 불릴 수 있어서, 그릴 때 필요한 마지막 마우스 좌표를 update() 가 매번 여기 남겨둔다.
    last_mouse: (f32, f32),
    // 창 제목을 매 프레임 다시 번역하려고 원문(fs 이름) 그대로 들고 있는다 —
    // "My Computer"/"Recycle Bin" 처럼 언어별로 바뀌는 특수 이름일 수도, 일반
    // 폴더의 실제 이름(번역 대상 아님)일 수도 있는데 display_name() 이 알아서
    // 구분해준다.
    raw_title: String,
    settings: Rc<RefCell<Settings>>,
}

impl ExplorerApp {
    pub(super) fn new(items: Items, raw_title: String, settings: Rc<RefCell<Settings>>) -> ExplorerApp {
        Self::from_tabs(vec![(String::new(), items, None, None)], 0, raw_title, settings)
    }

    pub(super) fn new_tabbed(tabs: Vec<Tab>, raw_title: String, settings: Rc<RefCell<Settings>>) -> ExplorerApp {
        Self::from_tabs(tabs, 0, raw_title, settings)
    }

    // 폴더를 더블클릭했을 때 새 창 대신 지금 File Explorer 창 안에서 바로 그 내용을
    // 보여주려고 쓴다 — 기존 카테고리 탭들 뒤에 그 폴더를 하위 폴더 탭으로 붙이고
    // 그걸 바로 활성화한다(사이드바에서 다른 카테고리를 누르면 평소처럼 빠져나간다).
    pub(super) fn new_tabbed_active(tabs: Vec<Tab>, active: usize, raw_title: String, settings: Rc<RefCell<Settings>>) -> ExplorerApp {
        Self::from_tabs(tabs, active, raw_title, settings)
    }

    fn from_tabs(tabs: Vec<Tab>, active: usize, raw_title: String, settings: Rc<RefCell<Settings>>) -> ExplorerApp {
        ExplorerApp {
            tabs,
            tab: active,
            sidebar_w: 150.0,
            divider_drag: false,
            list_view: true, // 기본은 가로로 긴 Name/Size 목록 보기
            selected: Vec::new(),
            marquee_start: None,
            prev_down: false,
            scroll: 0.0,
            scroll_disp: 0.0,
            sb_drag: false,
            last_click: -1.0,
            last_idx: -1,
            item_drag: None,
            item_drag_offset: (0.0, 0.0),
            last_mouse: (0.0, 0.0),
            raw_title,
            settings,
        }
    }

    // 탭/보기 모드를 바꾸면 이전 스크롤/선택/더블클릭 추적이 그대로 남아있으면
    // 안 되니 다 초기화한다.
    fn reset_view_state(&mut self) {
        self.selected.clear();
        self.marquee_start = None;
        self.item_drag = None;
        self.scroll = 0.0;
        self.scroll_disp = 0.0;
        self.last_idx = -1;
    }

    fn switch_tab(&mut self, i: usize) {
        self.tab = i;
        self.reset_view_state();
    }

    // 이 탭의 항목들 중 폴더가 있는지 — 있어야만 +/- 확장 박스가 실제로 뭔가를 한다
    // (리프 카테고리는 real Explorer 처럼 박스 자체를 안 그린다). 휴지통은 여기선
    // 폴더로 안 친다(tree_expandable 참고) — 트리에 탭으로 끼워 넣는 대신 항상
    // 독립된 프로그램(RecycleBinApp)으로 열어야 하기 때문.
    fn has_expandable_children(&self, i: usize) -> bool {
        self.tabs[i].1.iter().any(|(_, _, icon)| tree_expandable(icon))
    }

    // 이름이 name 인 탭이 지금 트리에서 펼쳐져 있는지 — 그 탭을 parent 로 삼는 하위
    // 탭이 이미 있으면 펼쳐진 것으로 본다.
    fn is_expanded(&self, name: &str) -> bool {
        self.tabs.iter().any(|t| t.2.as_deref() == Some(name))
    }

    // 아직 트리에 탭으로 끼워지지 않은 폴더 자식이 있으면 그 FileId 를 돌려준다 —
    // 있으면 AppAction::Open 으로 desktop.rs 에게 (fs 를 읽어서) 하위 탭을 끼워달라고
    // 요청한다(더블클릭으로 폴더를 열 때와 완전히 같은 경로). 휴지통은 건너뛴다 —
    // desktop.rs::is_drilldown_folder 가 휴지통을 걸러서 탭 대신 독립 창으로
    // 여니 여기서도 애초에 후보로 안 주는 게 맞다(안 그러면 데스크톱 트리를
    // 펼칠 때 휴지통이 계속 후보로 잡혀서 그 뒤에 있는 진짜 하위 폴더들을
    // 영영 못 펼치는 문제가 있었다).
    fn next_unexpanded_child(&self, i: usize) -> Option<FileId> {
        self.tabs[i].1.iter().find_map(|(fid, _, icon)| {
            let is_folder = tree_expandable(icon);
            let already_tab = self.tabs.iter().any(|t| t.3 == Some(*fid));
            (is_folder && !already_tab).then_some(*fid)
        })
    }

    // 펼쳐져 있던 탭(name)을 접는다 — name 을 parent 로 둔 하위 탭들과, 그 하위 탭을
    // 다시 parent 로 둔 손자 탭들까지 연쇄적으로 전부 제거한다(fs 접근이 필요 없는
    // 로컬 상태 변경이라 확장과 달리 AppAction 없이 바로 처리할 수 있다). 지금 보고
    // 있던 탭이 접혀서 없어졌으면 접은 탭(name) 자신으로 돌아간다.
    fn collapse(&mut self, name: &str) {
        let active_name = self.tabs[self.tab].0.clone();
        // ancestors 는 "이 이름을 parent 로 둔 탭을 찾는다" 는 매칭에만 쓰고, 실제로
        // 지울 이름은 removed_names 에 따로 모은다 — name 자기 자신은 매칭 기준일
        // 뿐 지울 대상이 아니다.
        let mut ancestors = vec![name.to_string()];
        let mut removed_names: Vec<String> = Vec::new();
        loop {
            let mut grew = false;
            for t in &self.tabs {
                if let Some(parent) = &t.2
                    && ancestors.contains(parent)
                    && !removed_names.contains(&t.0)
                {
                    removed_names.push(t.0.clone());
                    ancestors.push(t.0.clone());
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        self.tabs.retain(|t| !removed_names.contains(&t.0));
        self.tab = self
            .tabs
            .iter()
            .position(|t| t.0 == active_name)
            .or_else(|| self.tabs.iter().position(|t| t.0 == name))
            .unwrap_or(0);
    }

    // 지금 활성 탭이 새로고침 뒤에도 다시 찾아갈 수 있는 위치인지 — 드릴다운 탭이면
    // 그 폴더의 FileId 로, 고정 카테고리 탭이면 이름으로. desktop.rs 의 refresh_explorer_if_open
    // 이 as_any_mut 로 downcast 한 뒤 새로고침 직전에 불러서 위치를 저장해둔다.
    pub fn current_location(&self) -> Option<ExplorerLocation> {
        let (name, _, _, folder_id) = self.tabs.get(self.tab)?;
        match folder_id {
            Some(id) => Some(ExplorerLocation::Folder(*id)),
            None => Some(ExplorerLocation::Category(name.clone())),
        }
    }

    // 주소창 — 지금 보고 있는 카테고리를 가짜 경로처럼 읽기전용으로 보여준다(입력 불가).
    fn draw_address_bar(&self, r: &mut Renderer, area: Rect, lang: Language) {
        r.rect(area.x, area.y, area.w, ADDR_H, FACE);
        let label = t(lang, common::ADDRESS);
        r.text(area.x + 4.0, area.y + (ADDR_H - CELL_H * 0.8) / 2.0, label, 0.8, BLACK);
        let lw = r.text_width(label, 0.8) + 8.0;
        let field = Rect::new(area.x + lw, area.y + 2.0, area.w - lw - 4.0, ADDR_H - 6.0);
        sunken(r, field.x, field.y, field.w, field.h);
        let my_computer = display_name(lang, MY_COMPUTER_NAME);
        let path = match self.tabs.get(self.tab) {
            Some((name, _, Some(parent), _)) => format!("{my_computer}\\{}\\{}", category_label(lang, parent), category_label(lang, name)),
            Some((name, _, None, _)) if !name.is_empty() => format!("{my_computer}\\{}", category_label(lang, name)),
            _ => my_computer.into_owned(),
        };
        // 좌우 8px 여유(테두리에 안 닿게) + 글자 높이(CELL_H*스케일)에 맞춰 세로 중앙 정렬.
        let ty = field.y + (field.h - CELL_H * 0.8) / 2.0;
        r.text_clipped(field.x + 8.0, ty, &path, 0.8, BLACK, field.w - 16.0);
    }

    // 상태바 — 항목 수(+선택된 개수)와 자리표시자 여유 공간 표시.
    fn draw_status_bar(&self, r: &mut Renderer, area: Rect, lang: Language) {
        let n = self.tabs.get(self.tab).map(|t| t.1.len()).unwrap_or(0);
        let text = if self.selected.is_empty() {
            t(lang, common::OBJECT_COUNT).replace("{n}", &n.to_string())
        } else {
            t(lang, common::OBJECT_COUNT_SELECTED).replace("{n}", &n.to_string()).replace("{s}", &self.selected.len().to_string())
        };
        let ty = area.y + (STATUS_H - CELL_H * 0.8) / 2.0;
        let seg_w = (area.w * 0.55).max(80.0);
        sunken(r, area.x, area.y, seg_w, STATUS_H);
        r.text_clipped(area.x + 8.0, ty, &text, 0.8, BLACK, seg_w - 16.0);
        sunken(r, area.x + seg_w + 2.0, area.y, area.w - seg_w - 2.0, STATUS_H);
        r.text_clipped(area.x + seg_w + 10.0, ty, &display_name(lang, MY_COMPUTER_NAME), 0.8, BLACK, area.w - seg_w - 20.0);
    }
}

impl App for ExplorerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(display_name(lang, &self.raw_title).into_owned())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        // 사이드바로 드래그해 파일을 옮기는 중이었는지의 "뗌" 판정은 이 프레임에서
        // 그리기 전에 미리 스냅샷해야 한다 — draw_list_view/icon_grid 가 마퀴 선택용으로
        // self.prev_down 을 이번 프레임 값으로 덮어써버리기 때문.
        let released_this_frame = self.prev_down && !win.mouse_down;
        // drag_ghost() 가 update() 밖에서도 그릴 위치를 알 수 있게 매 프레임 남겨둔다.
        self.last_mouse = win.mouse;
        let lang = self.settings.borrow().language;

        self.draw_address_bar(r, Rect::new(area.x, area.y, area.w, ADDR_H), lang);

        let body_y = area.y + ADDR_H;
        let body_h = (area.y + area.h - STATUS_H - body_y).max(0.0);
        let body = Rect::new(area.x, body_y, area.w, body_h);
        let mut content = body;
        // 트리의 +/- 박스로 폴더를 펼치면(더블클릭과 같은 경로) desktop.rs 가 fs 를
        // 읽어서 하위 탭을 끼워 넣도록 AppAction::Open 을 돌려줘야 한다 — if 블록
        // 안에서만 쓰면 범위를 못 벗어나므로 미리 밖에 선언해둔다.
        let mut expand_request: Option<FileId> = None;
        // 지금 파일을 사이드바로 드래그해서 옮기는 중인지 — 시작점에서 일정 거리
        // 넘게 움직여야 진짜 드래그로 친다(그냥 클릭과 구분). 사이드바 루프에서 이
        // 값을 보고 드롭 가능한 탭을 하이라이트하고, hover_drop 에 그 대상을 담아둔다.
        let dragging_files = self.item_drag.is_some_and(|(sx, sy)| {
            let dx = win.mouse.0 - sx;
            let dy = win.mouse.1 - sy;
            dx * dx + dy * dy > 16.0
        });
        let mut hover_drop: Option<MoveDest> = None;

        if self.tabs.len() > 1 {
            // 고전 탐색기의 "폴더" 트리 창 느낌: 흰 배경 + 위쪽 "Folders" 라벨 + 각 줄
            // 앞에 +/- 확장 박스(지금은 하위 폴더가 없어 장식용). 아이콘은 전부 같은
            // 폴더 모양. 폭은 사이드바-콘텐츠 경계선을 드래그해서 조절 가능.
            self.sidebar_w = self.sidebar_w.clamp(SIDEBAR_MIN, body.w * SIDEBAR_MAX_FRAC);
            const TEXT_SCALE: f32 = 0.85;
            const EXPAND_BOX: f32 = 9.0;
            const ICON_S: f32 = 16.0;
            let line_h = CELL_H * TEXT_SCALE + 2.0;
            r.rect(body.x, body.y, self.sidebar_w, body.h, WHITE);
            r.text(body.x + 6.0, body.y + 4.0, t(lang, s::FOLDERS), TEXT_SCALE, BLACK);
            r.rect(body.x, body.y + 20.0, self.sidebar_w, 1.0, GRAY);

            const INDENT: f32 = 14.0; // 부모 카테고리의 하위 폴더 탭은 이만큼 들여쓴다
            let icon_x = 4.0 + EXPAND_BOX + 4.0;
            let text_x = icon_x + ICON_S + 2.0;
            let row_h = (line_h + 4.0).max(20.0);
            let mut ry = body.y + 24.0;
            let mut collapse_request: Option<String> = None;
            for i in 0..self.tabs.len() {
                // name 을 String 으로 복제해두는 이유: 뒤에서 collapse()/switch_tab() 처럼
                // self 를 다시 빌리는 호출과 얽히면 안 되기 때문(빌림 규칙상 self.tabs 를
                // 참조로 붙들고 있는 채로 self 를 mutably 못 씀).
                let name = self.tabs[i].0.clone();
                let indent = if self.tabs[i].2.is_some() { INDENT } else { 0.0 };
                let has_children = self.has_expandable_children(i);
                let expanded = has_children && self.is_expanded(&name);
                let row = Rect::new(body.x + 2.0 + indent, ry, (self.sidebar_w - 4.0 - indent).max(10.0), row_h);
                let hover = row.contains(win.mouse.0, win.mouse.1);
                let active = i == self.tab;
                // 드래그해온 파일을 옮길 수 있는 대상인지 — Desktop/Downloads 는 이름으로
                // 구분하고, 실제 폴더 탭은 자기 FileId 로 가리킨다. Videos 처럼 종류로만
                // 모아 보여주는 가상 카테고리는(folder_id 가 없음) 대상이 될 수 없다.
                let drop_dest = match name.as_str() {
                    "Desktop" => Some(MoveDest::Desktop),
                    "Downloads" => Some(MoveDest::Downloads),
                    _ => self.tabs[i].3.map(MoveDest::Folder),
                };
                // 드롭 대상 여부는 계속 추적하되(hover_drop), 화면에 초록색 등으로 따로
                // 강조하진 않는다 — 그냥 평소 hover 하이라이트만 그대로 보여준다.
                if dragging_files && hover && drop_dest.is_some() {
                    hover_drop = drop_dest;
                }
                if active {
                    r.rect(row.x, row.y, row.w, row.h, [0.78, 0.88, 1.0, 1.0]);
                    border(r, row.x, row.y, row.w, row.h, NAVY);
                } else if hover {
                    r.rect(row.x, row.y, row.w, row.h, [0.90, 0.90, 0.92, 1.0]);
                }

                // +/- 확장 박스 — 고전 탐색기 느낌을 유지하려고 모든 행에 기본으로
                // 그린다. 다만 실제로 펼칠 하위 폴더가 있는 항목만 안에 +/- 표시가 그려지고
                // 클릭에 반응한다 — 눌러서 펼치면(더블클릭과 같은 경로로) 하위 폴더가
                // 트리에 들여쓰기로 끼워지고, 다시 누르면(접기) 그 하위 탭들이 로컬에서
                // 바로 사라진다.
                let box_rect = Rect::new(row.x + 4.0, row.y + (row.h - EXPAND_BOX) / 2.0, EXPAND_BOX, EXPAND_BOX);
                let box_hover = has_children && box_rect.contains(win.mouse.0, win.mouse.1);
                sunken(r, box_rect.x, box_rect.y, box_rect.w, box_rect.h);
                if has_children {
                    let midy = box_rect.y + EXPAND_BOX / 2.0 - 0.5;
                    r.rect(box_rect.x + 2.0, midy, EXPAND_BOX - 4.0, 1.0, BLACK); // "-" 가로선(항상)
                    if !expanded {
                        let midx = box_rect.x + EXPAND_BOX / 2.0 - 0.5;
                        r.rect(midx, box_rect.y + 2.0, 1.0, EXPAND_BOX - 4.0, BLACK); // 세로선 추가하면 "+"
                    }
                    if win.mouse_clicked && box_hover {
                        if expanded {
                            collapse_request = Some(name.clone());
                        } else if let Some(fid) = self.next_unexpanded_child(i) {
                            expand_request = Some(fid);
                        }
                    }
                }

                let icon_y = row.y + (row.h - ICON_S) / 2.0;
                draw_icon(r, assets, &IconType::Folder, row.x + icon_x, icon_y, ICON_S);
                let text_color = if active { NAVY } else { BLACK };
                let text_max_w = (row.w - text_x - 4.0).max(10.0);
                // name 은 고정 카테고리 키("Desktop" 등)일 수도, 드릴다운으로 끼워진
                // 실제 폴더의 fs 이름("Recycle Bin" 등)일 수도 있어서 두 변환을 순서
                // 상관없이 같이 적용한다(서로 겹치지 않는 문자열 집합이라 안전).
                let category = category_label(lang, &name);
                let display = display_name(lang, &category);
                r.text_clipped(row.x + text_x, row.y + (row.h - CELL_H * TEXT_SCALE) / 2.0, &display, TEXT_SCALE, text_color, text_max_w);
                if win.mouse_clicked && hover && !box_hover && !active {
                    self.switch_tab(i);
                }
                ry += row_h;
            }
            if let Some(name) = collapse_request {
                self.collapse(&name);
            }

            // 사이드바 ↔ 콘텐츠 경계선 — 드래그해서 폭을 늘리거나 줄일 수 있다.
            const DIVIDER_W: f32 = 5.0;
            let divider = Rect::new(body.x + self.sidebar_w - DIVIDER_W / 2.0, body.y, DIVIDER_W, body.h);
            let divider_hover = divider.contains(win.mouse.0, win.mouse.1);
            if win.mouse_clicked && divider_hover {
                self.divider_drag = true;
            }
            if self.divider_drag && win.mouse_down {
                self.sidebar_w = (win.mouse.0 - body.x).clamp(SIDEBAR_MIN, body.w * SIDEBAR_MAX_FRAC);
            }
            if !win.mouse_down {
                self.divider_drag = false;
            }
            let line_color = if divider_hover || self.divider_drag { NAVY } else { [0.75, 0.75, 0.77, 1.0] };
            r.rect(body.x + self.sidebar_w - 1.0, body.y, 1.0, body.h, line_color);

            content = Rect::new(body.x + self.sidebar_w, body.y, body.w - self.sidebar_w, body.h);
        }

        // 우측 하단 보기 전환 버튼(격자/목록).
        const BTN_S: f32 = 20.0;
        let list_btn = Rect::new(content.x + content.w - BTN_S - 6.0, content.y + content.h - VIEW_TOGGLE_H + 2.0, BTN_S, BTN_S);
        let grid_btn = Rect::new(list_btn.x - BTN_S - 4.0, list_btn.y, BTN_S, BTN_S);
        r.rect(content.x, content.y + content.h - VIEW_TOGGLE_H, content.w, VIEW_TOGGLE_H, FACE);
        r.rect(content.x, content.y + content.h - VIEW_TOGGLE_H, content.w, 1.0, GRAY);
        for (btn, active, is_list) in [(grid_btn, !self.list_view, false), (list_btn, self.list_view, true)] {
            if active {
                sunken(r, btn.x, btn.y, btn.w, btn.h);
            } else {
                raised(r, btn.x, btn.y, btn.w, btn.h);
            }
            let glyph_color = if active { NAVY } else { BLACK };
            if is_list {
                draw_list_glyph(r, btn.x + 4.0, btn.y + 4.0, btn.w - 8.0, btn.h - 8.0, glyph_color);
            } else {
                draw_grid_glyph(r, btn.x + 3.0, btn.y + 3.0, btn.w - 6.0, glyph_color);
            }
            if win.mouse_clicked && btn.contains(win.mouse.0, win.mouse.1) && self.list_view != is_list {
                self.list_view = is_list;
                self.reset_view_state();
            }
        }
        let list_area = Rect::new(content.x, content.y, content.w, content.h - VIEW_TOGGLE_H);

        let smooth = self.settings.borrow().smooth_scroll;
        let items = &self.tabs[self.tab].1;
        let clicked = if self.list_view {
            draw_list_view(
                r, assets, win, list_area, items, &mut self.selected, &mut self.marquee_start, &mut self.prev_down, &mut self.scroll,
                &mut self.scroll_disp, &mut self.sb_drag, smooth, lang,
            )
        } else {
            icon_grid(
                r,
                assets,
                win,
                list_area,
                items,
                &mut self.selected,
                &mut self.marquee_start,
                &mut self.prev_down,
                &mut self.scroll,
                &mut self.scroll_disp,
                smooth,
                &mut self.sb_drag,
                lang,
            )
        };
        let opened = if let Some((i, _)) = clicked {
            if self.last_idx == i as i32 && win.time - self.last_click < 0.35 {
                self.last_idx = -1;
                Some(AppAction::Open(items[i].0))
            } else {
                self.last_idx = i as i32;
                self.last_click = win.time;
                None
            }
        } else {
            None
        };

        // 항목을 막 눌렀으면(마퀴 드래그가 아닌 진짜 클릭) 드래그 시작점으로 기록해둔다
        // — 다음 프레임부터 이 시작점 기준으로 사이드바까지 옮기는 중인지 판단한다.
        if let Some((_, rect)) = clicked {
            self.item_drag = Some(win.mouse);
            self.item_drag_offset = if self.list_view {
                // 목록 보기는 행 전체가 창 폭만큼 넓은데 고스트는 아이콘 하나 크기뿐이라,
                // 클릭했던 x 좌표를(행 오른쪽 끝 근처를 눌렀으면 아주 커질 수 있다) 그대로
                // 오프셋으로 쓰면 고스트가 커서에서 한참 떨어져 보인다 — 대신 고스트
                // 아이콘의 정중앙이 항상 커서에 오도록 고정 오프셋을 쓴다.
                (DRAG_GHOST_ICON_S / 2.0, DRAG_GHOST_ICON_S / 2.0)
            } else {
                // 격자 보기는 칸 크기가 아이콘과 비슷해서, 잡았던 지점 그대로(좌상단
                // 기준 오프셋) 따라오는 편이 더 자연스럽다.
                (win.mouse.0 - rect.x, win.mouse.1 - rect.y)
            };
        }
        // 뗀 프레임에 실제로 옮기던 중이었고(dragging_files) 사이드바의 옮길 수 있는
        // 탭 위에서 놓았으면 그 파일들을 옮겨달라고 요청한다. 사이드바가 아니어도
        // 이 창 밖에서 놓았으면(바탕화면이든 다른 창이든) 바로 바탕화면으로 옮긴다 —
        // 그래야 File Explorer 밖으로 끌어다 놓기만 해도 바탕화면에 바로 둘 수 있다.
        // 창 안에서(사이드바도 아닌 자리에) 그냥 놓았으면 아무 일도 안 하고 선택만 남는다.
        //
        // win.focused 를 반드시 같이 검사한다 — 더블클릭으로 항목을 열면 두 번째
        // 클릭이 처리되는 바로 그 프레임에 새 창이 뜨면서 이 File Explorer 창이 곧장
        // 포커스를 잃는데, window_manager.rs 는 포커스를 잃은(=가려진) 창에는 실제
        // 좌표 대신 화면 밖 좌표(-1e6,-1e6, occluded 처리)를 준다. 그 가짜 좌표가
        // "마우스가 창 area 밖으로 나갔다"로 오인되면서 방금 연 항목이 그 프레임에
        // (여기 있던) MoveDest::Desktop 으로 다시 "이동"돼 실제 커서 위치(=새로 뜬
        // 창에 가려진 자리)로 옮겨져 바탕화면에서 사라진 것처럼 보이는 버그가 있었다
        // — 포커스를 잃은 프레임의 좌표는 애초에 신뢰할 수 없으니 그 프레임엔 드롭
        // 판정 자체를 하지 않는다(item_drag 정리는 그대로 진행되어 다음 상호작용엔 영향 없음).
        let move_action = if dragging_files && released_this_frame && win.focused {
            let dest = hover_drop.or_else(|| (!area.contains(win.mouse.0, win.mouse.1)).then_some(MoveDest::Desktop));
            dest.and_then(|dest| {
                let ids: Vec<FileId> = self.selected.iter().filter_map(|&i| items.get(i).map(|(id, ..)| *id)).collect();
                (!ids.is_empty()).then_some(AppAction::MoveFiles(ids, dest))
            })
        } else {
            None
        };
        if released_this_frame {
            self.item_drag = None;
        }

        self.draw_status_bar(r, Rect::new(area.x, area.y + area.h - STATUS_H, area.w, STATUS_H), lang);

        // 우선순위: 파일 옮기기 > 트리에서 +/- 로 펼친 요청 > 더블클릭 열기 — 같은
        // 프레임에 여러 개가 동시에 일어날 일은 없지만 순서는 이렇게 둔다.
        move_action.or_else(|| expand_request.map(AppAction::Open)).or(opened).unwrap_or(AppAction::None)
    }

    fn drag_ghost(&self) -> Option<DragGhost> {
        let (sx, sy) = self.item_drag?;
        let dx = self.last_mouse.0 - sx;
        let dy = self.last_mouse.1 - sy;
        if dx * dx + dy * dy <= 16.0 {
            return None; // 아직 문턱을 안 넘었으면(그냥 클릭) 고스트를 안 그린다.
        }
        let &first = self.selected.first()?;
        let (_, name, icon) = self.tabs.get(self.tab)?.1.get(first)?;
        let lang = self.settings.borrow().language;
        let label = if self.selected.len() == 1 {
            display_name(lang, name).into_owned()
        } else {
            t(lang, s::ITEMS_COUNT).replace("{n}", &self.selected.len().to_string())
        };
        let pos = (self.last_mouse.0 - self.item_drag_offset.0, self.last_mouse.1 - self.item_drag_offset.1);
        Some(DragGhost { icon: *icon, label, pos })
    }
}
