// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! PICOCAD 스타일의 아주 작은 맵 에디터 — `mesh3d.rs`(진짜 3D 메쉬 렌더러) 위에
//! 상자(`Box3D`)를 놓고 옮기고 회전/크기조절/텍스처 지정해서
//! `mapfile::MapScene`(JSON)으로 저장한다. 이 게임 전용 최소 기능만 넣었다:
//! 정점 단위 편집 같은 진짜 PICOCAD 급 범용 기능은 없다.
//!
//! `cargo run --bin map_editor` 로 띄운다. 저장/불러오기 경로는 고정으로
//! `maps/scene.json`(실행 파일 기준 상대 경로)을 쓴다.
//!
//! UI 는 Win9x 위젯(`ui::button`) 대신 이 파일 안에서 직접 그리는 플랫 다크
//! 테마다(`flat_button`/`stepper_row`) — 유니티 인스펙터/하이어라키(어두운 회색
//! 패널, 파란 강조색, 들여쓰기된 트리)를 참고했다. 가만히 있는 항목은 사각
//! 박스로 안 둘러싸고 글자만 보이다가, 마우스가 올라가거나 선택됐을 때만
//! 배경이 생긴다(전부 박스/버튼처럼 보이면 "짝대기 더미" 같다는 피드백을
//! 받았다) — 실제 입력 필드(숫자 값 칸)만 항상 옅은 배경이 있다.
//! 왼쪽엔 씬(들여쓰기된 오브젝트 트리, 스크롤 가능) + 그 아래 인스펙터(기즈모
//! 모드 버튼/이름 바꾸기/부모 지정/위치·회전·크기 -/+ 필드/walkable·solid
//! 토글/색상 순환/텍스처 목록) 패널이 있고, 오른쪽이 3D 뷰포트다.
//!
//! 조작:
//! - **뷰포트에서 마우스 왼쪽 클릭**(드래그 없이): 그 자리의 상자 하나만 선택.
//!   **드래그**(빈 곳에서 시작): 사각형 안에 중심이 들어오는 상자를 전부 선택
//!   (여러 개 선택 — 유니티 씬 뷰의 드래그 선택과 같다). **왼쪽 패널의 오브젝트
//!   트리 클릭**도 단일 선택, **트리 행을 눌러서 드래그**하면 다른 행(또는 맨
//!   위 "루트로 놓기" 구역) 위에 놓아 부모-자식 관계를 바꿀 수 있다(블렌더/
//!   유니티의 아웃라이너/하이어라키와 같은 방식 — 자기 자신이나 자기 자손
//!   위엔 못 놓는다).
//! - **선택한 상자에 뜨는 기즈모**(빨강=X, 초록=Y, 파랑=Z): 이동 모드는 화살표,
//!   회전 모드는 축 둘레 고리, 크기조절 모드는 끝에 각진 손잡이로 서로 다르게
//!   그린다 — 셋 다 "그냥 막대기"로 보이지 않게. 중심에서 손잡이까지 이어진
//!   선(또는 고리) 어디를 잡아도 된다. `W`=이동, `E`=회전, `R`=크기조절
//!   모드(유니티와 같은 키, 인스펙터의 모드 버튼을 클릭해도 된다) — 오른쪽
//!   버튼을 눌러 카메라를 날리는 중엔 반응하지 않는다(그때는 W가 전진).
//!   이동 모드로 부모를 드래그하면 자식들도, 함께 선택된 다른 상자들도 같은
//!   만큼 같이 움직인다.
//! - **마우스 오른쪽 버튼을 누른 채** 드래그: 시점 회전(마우스룩), 그 상태에서
//!   W/A/S/D 로 그 방향을 향해 날아다니고 Q/E 로 위/아래로 움직인다(Shift 로 빠르게).
//!   **드래그 없이 그냥 눌렀다 떼면**(블렌더/유니티처럼) 그 자리의 오브젝트를
//!   먼저 선택하고 작은 컨텍스트 메뉴(Rename/Duplicate/Unparent/Delete, 빈
//!   곳이면 New Box)를 띄운다. 메뉴가 떠 있을 때 아무 데나 클릭하면(항목 위가
//!   아니면) 그냥 닫히기만 한다.
//! - 휠: 사이드바 위에서는 오브젝트 목록 스크롤, 뷰포트 위에서는 카메라를
//!   보고 있는 방향으로 조금씩 전진/후진(줌).
//! - Tab / Shift+Tab: 다음/이전 상자 단일 선택, N: 카메라 앞에 새 상자,
//!   Delete: 선택한(여러 개면 전부) 상자 삭제
//! - P: 플레이어 시작 위치/방향을 지금 선택한 상자 자리로 설정
//! - Ctrl+S: 저장, Ctrl+O: 불러오기, Esc: 종료(이름 바꾸는 중/메뉴가 떠 있는
//!   중엔 그것부터 취소·닫기)
//!
//! 창 밖으로 마우스가 나가면(`mouse_leave_event`) 비행/기즈모 드래그 상태를
//! 전부 강제로 끈다 — 안 그러면 오른쪽 버튼을 누른 채 화면 가장자리 밖에서
//! 손을 떼는 순간 그 릴리즈 이벤트를 못 받아서 "비행 모드에 계속 갇히는"
//! (이후 마우스를 움직이기만 해도 카메라가 계속 도는) 버그가 있었다. 또한
//! 창 리사이즈를 아예 막고(`window_resizable: false`) 실제 화면 크기가 이
//! 창이 가정하는 가상 해상도(800x600)와 어긋나도 항상 맞게 마우스 좌표를
//! 다시 재는 `to_virtual()`을 매 마우스 이벤트에 건다(main.rs 의 같은 이름
//! 함수와 같은 이유) — 전에 창을 최대화하면 클릭이 죄다 빗나가는 버그가 있었다.

use std::collections::HashMap;

use miniquad::*;

use crackhead::gfx::Renderer;
use crackhead::mapfile::{MapBoxData, MapScene};
use crackhead::mesh3d::{v_add, v_dot, v_scale, v_sub, Box3D, BoxTexture, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 600.0;
const SIDEBAR_W: f32 = 240.0;
const VIEWPORT_W: f32 = WIN_W - SIDEBAR_W;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const SAVE_PATH: &str = "maps/scene.json";
const TEXTURE_DIR: &str = "maps/textures";
const ROW_H: f32 = 22.0;
const PAD: f32 = 6.0;
const INDENT_W: f32 = 14.0; // 하이어라키 트리 한 단계당 들여쓰기(유니티 참고)
const ROOT_ZONE_TOP: f32 = 20.0;
const ROOT_ZONE_H: f32 = 16.0; // 하이어라키 드래그 중 여기에 놓으면 부모를 뗀다(맨 위 루트로)
const HIER_LIST_TOP: f32 = ROOT_ZONE_TOP + ROOT_ZONE_H + 4.0;
const HIER_VISIBLE_ROWS: usize = 6; // 이 이상 쌓이면 스크롤 — 인스펙터가 밀려나지 않게 목록 높이를 고정한다
const MARQUEE_MIN_DRAG: f32 = 4.0; // 이 픽셀 이하로 움직였으면 드래그 선택/재부모 지정이 아니라 클릭으로 친다
const MENU_W: f32 = 130.0; // 우클릭 컨텍스트 메뉴 폭

const SKY_COLOR: [f32; 4] = [0.12, 0.13, 0.17, 1.0];
const PALETTE: [[f32; 4]; 6] = [
    [0.7, 0.7, 0.75, 1.0],
    [0.5, 0.42, 0.3, 1.0],
    [0.32, 0.45, 0.3, 1.0],
    [0.55, 0.3, 0.3, 1.0],
    [0.35, 0.35, 0.6, 1.0],
    [0.8, 0.75, 0.35, 1.0],
];

const MIN_HALF: f32 = 0.05;
const POS_STEP: f32 = 0.25;
const ROT_STEP: f32 = 5.0_f32.to_radians();
const HALF_STEP: f32 = 0.1;

const FLY_SPEED: f32 = 4.0;
const FLY_SPEED_FAST: f32 = 12.0;
const LOOK_SENS: f32 = 0.0035; // 마우스 1px 당 라디안
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const DOLLY_SPEED: f32 = 0.15; // 휠 한 칸당 전진/후진 거리

const GIZMO_HANDLE_LEN: f32 = 1.3; // 이동/회전 손잡이가 중심에서 떨어진 거리(월드 단위)
const GIZMO_LINE_HIT_R: f32 = 14.0; // 손잡이 선/고리 어디든 이 픽셀 반경 안이면 잡힌다
const GIZMO_ROTATE_SENS: f32 = 0.012; // 회전 모드에서 픽셀당 라디안
const GIZMO_RING_SEGMENTS: usize = 24;

const AXIS_COLORS: [[f32; 4]; 3] = [[0.95, 0.35, 0.35, 1.0], [0.4, 0.9, 0.4, 1.0], [0.4, 0.6, 1.0, 1.0]];

// ================= 플랫 다크 테마 UI =================
// Win9x 위젯(ui::button 등)은 이 창의 어두운 배경과 어울리지 않아서(밝은
// 베벨 버튼이 둥둥 떠 보임) 안 쓰고, 유니티 인스펙터/하이어라키를 참고해
// 직접 그린다: 어두운 회색 패널, 그보다 살짝 밝은 사각 필드, 선택/켜짐은
// 파란 강조색, 트리는 들여쓰기.

const COL_FIELD: [f32; 4] = [0.22, 0.22, 0.23, 1.0];
const COL_FIELD_HOVER: [f32; 4] = [0.28, 0.28, 0.3, 1.0];
const COL_ACTIVE: [f32; 4] = [0.16, 0.44, 0.78, 1.0];
const COL_BORDER: [f32; 4] = [0.08, 0.08, 0.09, 1.0];
const COL_TEXT: [f32; 4] = [0.82, 0.82, 0.84, 1.0];
const COL_TEXT_DIM: [f32; 4] = [0.5, 0.5, 0.52, 1.0];
const COL_PANEL_BG: [f32; 4] = [0.145, 0.145, 0.155, 1.0];

// 실제 창 좌표 → 이 창이 가정하는 가상 해상도(800x600). main.rs::to_virtual()
// 과 같은 이유로 필요하다 — 리사이즈를 막아뒀지만(window_resizable: false),
// 실제 화면 크기(window::screen_size())가 어떤 이유로든 어긋나면 마우스 좌표와
// 렌더러가 그리는 좌표계가 안 맞아 클릭이 죄다 빗나간다 — 매 이벤트마다 실제
// 화면 크기를 다시 재서 비율로 보정하면 그 어긋남과 무관하게 항상 맞는다.
fn to_virtual(x: f32, y: f32) -> (f32, f32) {
    let (sw, sh) = window::screen_size();
    if sw <= 0.0 || sh <= 0.0 {
        return (x, y);
    }
    (x * WIN_W / sw, y * WIN_H / sh)
}

fn point_in_rect(mx: f32, my: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
    mx >= x && mx < x + w && my >= y && my < y + h
}

// 클릭됐으면 true 를 돌려준다(mouse: 현재 마우스 위치, clicked: 이번 프레임에
// 왼쪽 버튼이 눌린 순간인지). active 면 파란 강조(선택된 씬 항목/켜진 토글/
// 지금 고른 텍스처 등)로, 마우스가 올라가 있으면 옅은 배경으로 그린다 —
// 유니티/블렌더의 리스트·버튼처럼 가만히 있을 땐 배경 없이 글자만 보이고
// (전부 사각 박스로 둘러싸여 있으면 "짝대기/박스 더미"처럼 보인다는 피드백을
// 받았다), 마우스가 올라가거나 선택됐을 때만 배경이 생긴다.
#[allow(clippy::too_many_arguments)]
fn flat_button(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, label: &str, mouse: (f32, f32), clicked: bool, active: bool) -> bool {
    let hover = point_in_rect(mouse.0, mouse.1, x, y, w, h);
    if active {
        r.rect(x, y, w, h, COL_ACTIVE);
    } else if hover {
        r.rect(x, y, w, h, COL_FIELD_HOVER);
    }
    let tw = r.text_width(label, 0.62);
    let tx = (x + (w - tw) / 2.0).max(x + 4.0);
    r.text(tx, y + h / 2.0 - 6.0, label, 0.62, if active { [1.0, 1.0, 1.0, 1.0] } else { COL_TEXT });
    hover && clicked
}

// 축 색으로 칠해진 라벨 + 현재 값 + 오른쪽 -/+ 버튼 두 개짜리 한 행.
#[allow(clippy::too_many_arguments)]
fn stepper_row(r: &mut Renderer, x: f32, y: f32, w: f32, label: &str, label_color: [f32; 4], value: f32, step: f32, mouse: (f32, f32), clicked: bool) -> Option<f32> {
    let btn_w = 22.0;
    let field_w = w - btn_w * 2.0 - 4.0;
    r.rect(x, y, field_w, ROW_H - 3.0, COL_FIELD);
    r.text(x + 6.0, y + ROW_H / 2.0 - 7.0, label, 0.56, label_color);
    let val_text = format!("{value:.2}");
    let tw = r.text_width(&val_text, 0.58);
    r.text(x + field_w - tw - 6.0, y + ROW_H / 2.0 - 7.0, &val_text, 0.58, COL_TEXT);
    let minus = flat_button(r, x + field_w + 2.0, y, btn_w, ROW_H - 3.0, "-", mouse, clicked, false);
    let plus = flat_button(r, x + field_w + btn_w + 4.0, y, btn_w, ROW_H - 3.0, "+", mouse, clicked, false);
    if minus {
        Some(value - step)
    } else if plus {
        Some(value + step)
    } else {
        None
    }
}

// "라벨: 값" 텍스트 + 오른쪽 -/+ 버튼(다음/이전 후보로 순환) 한 행 — 부모
// 오브젝트 고르기처럼 숫자가 아니라 후보 목록을 순환할 때 쓴다. -1/+1 중
// 눌린 쪽을 돌려준다(없으면 None).
#[allow(clippy::too_many_arguments)]
fn cycle_row(r: &mut Renderer, x: f32, y: f32, w: f32, label: &str, value_text: &str, mouse: (f32, f32), clicked: bool) -> Option<i32> {
    let btn_w = 22.0;
    let field_w = w - btn_w * 2.0 - 4.0;
    r.rect(x, y, field_w, ROW_H - 3.0, COL_FIELD);
    let text = format!("{label}: {value_text}");
    let text = if r.text_width(&text, 0.56) > field_w - 8.0 {
        format!("{label}: …")
    } else {
        text
    };
    r.text(x + 6.0, y + ROW_H / 2.0 - 7.0, &text, 0.56, COL_TEXT);
    let minus = flat_button(r, x + field_w + 2.0, y, btn_w, ROW_H - 3.0, "-", mouse, clicked, false);
    let plus = flat_button(r, x + field_w + btn_w + 4.0, y, btn_w, ROW_H - 3.0, "+", mouse, clicked, false);
    if minus {
        Some(-1)
    } else if plus {
        Some(1)
    } else {
        None
    }
}

// 인스펙터의 위치/회전/크기 -/+ 행 9개를 (라벨, 축색, 현재값, 증감폭, 적용함수) 로
// 표로 짜서 찍어내는 데 쓴다.
type InspectorRow = (&'static str, [f32; 4], f32, f32, fn(&mut Box3D, f32));

fn default_box(center: [f32; 3]) -> Box3D {
    Box3D { center, half: [0.5, 0.5, 0.5], yaw: 0.0, pitch: 0.0, roll: 0.0, color: PALETTE[0], texture: None, walkable: true, solid: true }
}

// 경로별로 한 번만 디코드/업로드한다(캐시). 실패하면 status 에 알리고 1x1 흰
// 텍스처(색 곱만 먹는 자리표시자)를 대신 돌려준다.
fn load_texture(ctx: &mut dyn RenderingBackend, cache: &mut HashMap<String, TextureId>, status: &mut String, path: &str) -> TextureId {
    if let Some(&tex) = cache.get(path) {
        return tex;
    }
    let tex = match image::open(path) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            ctx.new_texture_from_rgba8(rgba.width() as u16, rgba.height() as u16, &rgba)
        }
        Err(e) => {
            *status = format!("텍스처 로드 실패({path}): {e}");
            ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255])
        }
    };
    cache.insert(path.to_string(), tex);
    tex
}

// 점 p 에서 선분 (a→b) 까지의 최단 거리(2D) — 기즈모 손잡이를 "끝점 근처"가
// 아니라 "중심에서 손잡이까지 이어진 선 아무 데나" 클릭해도 잡히게 하는 데 쓴다.
fn dist_point_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 1e-6 { (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (cx, cy) = (ax + dx * t, ay + dy * t);
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

#[derive(Clone, Copy, PartialEq)]
enum GizmoMode {
    Move,
    Rotate,
    Scale,
}

impl GizmoMode {
    fn label(self) -> &'static str {
        match self {
            GizmoMode::Move => "Move",
            GizmoMode::Rotate => "Rotate",
            GizmoMode::Scale => "Scale",
        }
    }
}

// 블렌더/유니티처럼 오른쪽 클릭(드래그 없이 누르고 바로 뗀 경우만 — 드래그하면
// 그냥 시점 회전이다)으로 뜨는 작은 메뉴. target 이 있으면 그 상자에 대한
// 메뉴(이름 바꾸기/복제/부모 떼기/삭제), 없으면 빈 곳 메뉴(새 상자)다.
#[derive(Clone, Copy, PartialEq)]
enum MenuAction {
    Rename,
    Duplicate,
    Unparent,
    Delete,
    NewBox,
}

#[derive(Clone, Copy)]
struct ContextMenu {
    pos: (f32, f32), // 뜬 자리(창 좌표) — 메뉴 판정/그리기 둘 다 이 좌표 기준
    target: Option<usize>,
}

// 유니티 씬 카메라처럼: 위치 + yaw/pitch 를 직접 들고 있는 자유 카메라. 오빗(왼쪽
// 드래그로 궤도 회전)은 없앴다 — 왼쪽 드래그는 이제 다중 선택(마퀴)에 쓴다.
struct FlyCam {
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
}

impl FlyCam {
    fn camera(&self) -> Camera {
        Camera { pos: self.pos, yaw: self.yaw, pitch: self.pitch }
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    mesh3d: Mesh3D,
    boxes: Vec<Box3D>,
    // 아래 세 Vec 은 전부 self.boxes 와 항상 같은 길이/순서로 유지한다.
    box_texture_paths: Vec<Option<String>>, // 저장 포맷은 텍스처를 경로 문자열로 들고 있어야 해서 따로 보관
    names: Vec<String>,                     // 사람이 붙인 이름(하이어라키에 보여줌)
    parents: Vec<Option<usize>>,            // 부모 오브젝트 인덱스 — 유니티처럼 이동 시 자식도 같이 움직인다
    selected: Option<usize>,                // "주" 선택 — 인스펙터/기즈모 대상
    multi_selected: Vec<usize>,             // 마퀴(드래그) 선택으로 한 번에 고른 여러 개 — 삭제/그룹 이동에 쓴다
    renaming: Option<usize>,
    rename_buffer: String,
    hier_scroll: f32,
    player_start: [f32; 3],
    player_start_yaw: f32,
    cam: FlyCam,
    flying: bool, // 오른쪽 버튼을 누르고 있는 동안 — 마우스룩 + WASD 비행
    marqueeing: bool,
    marquee_start: (f32, f32), // 창 좌표(마퀴 드래그를 시작한 지점)
    left_drag_dist: f32,       // 왼쪽 버튼을 누른 뒤 총 이동 거리 — 문턱보다 작으면 드래그가 아니라 클릭
    hier_press: Option<usize>, // 하이어라키 행을 누른 순간의 상자 인덱스(드래그하면 재부모 지정 후보)
    hier_press_pos: (f32, f32),
    hier_dragging: bool,
    rmb_press_pos: (f32, f32), // 오른쪽 버튼을 누른 자리 — 거의 안 움직이고 뗐으면 컨텍스트 메뉴
    rmb_drag_dist: f32,
    context_menu: Option<ContextMenu>,
    gizmo_mode: GizmoMode,
    hovered_axis: Option<usize>,        // 지금 프레임에 마우스가 근처에 있는 기즈모 축(드래그 전 미리보기용)
    drag_axis: Option<usize>,           // 지금 드래그 중인 기즈모 축(0=X/1=Y/2=Z) — None 이면 기즈모 드래그 아님
    drag_start_box: Option<Box3D>,      // 드래그 시작 시점의(주 선택) 상자 스냅샷(그 기준으로 델타를 계산)
    drag_start_mouse: (f32, f32),       // 뷰포트 로컬 좌표(사이드바 폭을 뺀 좌표)
    drag_pixels_per_unit: f32,          // 그 축 방향 1 월드 단위가 화면에서 몇 픽셀인지(드래그 시작 시점 기준)
    drag_screen_dir: (f32, f32),        // 그 축이 화면에 투영된 단위 방향
    drag_group_start: Vec<(usize, [f32; 3])>, // 이동 모드일 때 같이 옮길 자식/다중 선택의 시작 위치
    last_mouse: (f32, f32),
    input: Input,
    ctrl_down: bool,
    status: String,
    last_time: f64,
    texture_cache: HashMap<String, TextureId>,
    available_textures: Vec<String>, // TEXTURE_DIR 에서 찾은 *.png 상대 경로들
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mesh3d = Mesh3D::new(ctx.as_mut(), VIEWPORT_W as u32, WIN_H as u32);
        let boxes = vec![default_box([0.0, 0.0, 0.0])];
        let mut stage = Stage {
            ctx,
            renderer,
            mesh3d,
            boxes,
            box_texture_paths: vec![None],
            names: vec!["Box 1".to_string()],
            parents: vec![None],
            selected: Some(0),
            multi_selected: vec![0],
            renaming: None,
            rename_buffer: String::new(),
            hier_scroll: 0.0,
            player_start: [0.0, 1.0, -3.0],
            player_start_yaw: std::f32::consts::FRAC_PI_2,
            cam: FlyCam { pos: [6.0, 4.0, 6.0], yaw: -std::f32::consts::FRAC_PI_4 * 3.0, pitch: -0.5 },
            flying: false,
            marqueeing: false,
            marquee_start: (0.0, 0.0),
            left_drag_dist: 0.0,
            hier_press: None,
            hier_press_pos: (0.0, 0.0),
            hier_dragging: false,
            rmb_press_pos: (0.0, 0.0),
            rmb_drag_dist: 0.0,
            context_menu: None,
            gizmo_mode: GizmoMode::Move,
            hovered_axis: None,
            drag_axis: None,
            drag_start_box: None,
            drag_start_mouse: (0.0, 0.0),
            drag_pixels_per_unit: 50.0,
            drag_screen_dir: (1.0, 0.0),
            drag_group_start: Vec::new(),
            last_mouse: (0.0, 0.0),
            input: Input::default(),
            ctrl_down: false,
            status: "새 장면".to_string(),
            last_time: date::now(),
            texture_cache: HashMap::new(),
            available_textures: Vec::new(),
        };
        stage.scan_textures();
        stage
    }

    fn scan_textures(&mut self) {
        self.available_textures.clear();
        let Ok(entries) = std::fs::read_dir(TEXTURE_DIR) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) == Some(true) {
                self.available_textures.push(path.to_string_lossy().replace('\\', "/"));
            }
        }
        self.available_textures.sort();
    }

    fn push_box(&mut self, b: Box3D) {
        let name = format!("Box {}", self.boxes.len() + 1);
        self.boxes.push(b);
        self.box_texture_paths.push(None);
        self.names.push(name);
        self.parents.push(None);
        let idx = self.boxes.len() - 1;
        self.selected = Some(idx);
        self.multi_selected = vec![idx];
    }

    // idx 하나를 지우고, 다른 상자들의 parent 인덱스를 지운 자리만큼 다시
    // 맞춘다(그 상자를 부모로 뒀던 자식은 고아가 된다 — parent=None).
    fn remove_box_at(&mut self, idx: usize) {
        self.boxes.remove(idx);
        self.box_texture_paths.remove(idx);
        self.names.remove(idx);
        self.parents.remove(idx);
        for p in self.parents.iter_mut() {
            match *p {
                Some(pi) if pi == idx => *p = None,
                Some(pi) if pi > idx => *p = Some(pi - 1),
                _ => {}
            }
        }
    }

    // 선택된 것(마퀴로 여러 개 골랐으면 전부, 아니면 주 선택 하나)을 삭제한다.
    fn remove_selected(&mut self) {
        let mut idxs = if !self.multi_selected.is_empty() { self.multi_selected.clone() } else { self.selected.into_iter().collect::<Vec<_>>() };
        if idxs.is_empty() {
            return;
        }
        idxs.sort_unstable_by(|a, b| b.cmp(a)); // 뒤에서부터 지워야 앞쪽 인덱스가 안 밀린다
        idxs.dedup();
        for i in idxs {
            if i < self.boxes.len() {
                self.remove_box_at(i);
            }
        }
        if self.boxes.is_empty() {
            self.push_box(default_box([0.0, 0.0, 0.0]));
        } else {
            self.selected = Some(0);
            self.multi_selected = vec![0];
        }
    }

    // 진행 중이던 비행/기즈모 드래그를 전부 강제로 끈다 — 마우스가 창 밖으로
    // 나가서 버튼 릴리즈를 못 받는 경우의 안전장치(mouse_leave_event).
    fn cancel_all_drags(&mut self) {
        self.flying = false;
        self.marqueeing = false;
        self.drag_axis = None;
        self.drag_start_box = None;
        self.drag_group_start.clear();
        self.hier_press = None;
        self.hier_dragging = false;
    }

    // 상자 하나를 복제한다 — 살짝 옆으로 어긋나게, 같은 부모 밑에, 이름 뒤에
    // " Copy"를 붙여서. 복제본을 바로 선택한다(블렌더/유니티와 같은 느낌).
    fn duplicate_box(&mut self, idx: usize) {
        let mut b = self.boxes[idx].clone();
        b.center[0] += 0.3;
        b.center[2] += 0.3;
        let tex = self.box_texture_paths[idx].clone();
        let name = format!("{} Copy", self.names[idx]);
        let parent = self.parents[idx];
        self.boxes.push(b);
        self.box_texture_paths.push(tex);
        self.names.push(name);
        self.parents.push(parent);
        let new_idx = self.boxes.len() - 1;
        self.selected = Some(new_idx);
        self.multi_selected = vec![new_idx];
    }

    // `node` 가 `ancestor` 의 자손인지(부모 체인을 따라 올라가며 확인) — 부모를
    // 고를 때 순환(자기 자손을 부모로 삼는 것)을 막는 데 쓴다.
    fn is_descendant(&self, ancestor: usize, node: usize) -> bool {
        let mut cur = self.parents.get(node).copied().flatten();
        while let Some(p) = cur {
            if p == ancestor {
                return true;
            }
            cur = self.parents.get(p).copied().flatten();
        }
        false
    }

    // node 의 부모가 될 수 있는 후보들 — 자기 자신과 자기 자손은 뺀다(순환 방지).
    // 맨 앞은 항상 None("부모 없음").
    fn candidate_parents(&self, node: usize) -> Vec<Option<usize>> {
        let mut v = vec![None];
        for i in 0..self.boxes.len() {
            if i != node && !self.is_descendant(node, i) {
                v.push(Some(i));
            }
        }
        v
    }

    // node 의 모든 자손(자식의 자식까지 재귀적으로) 인덱스.
    fn descendants(&self, node: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(cur) = stack.pop() {
            for i in 0..self.boxes.len() {
                if self.parents[i] == Some(cur) {
                    out.push(i);
                    stack.push(i);
                }
            }
        }
        out
    }

    // 하이어라키 패널에 보여줄 순서 — 부모 밑에 자식이 바로 이어지는 트리
    // 순서로(들여쓰기 깊이도 같이) 뽑는다. 루트(parent=None)부터 인덱스
    // 순서대로 방문하면서, 각 노드 바로 뒤에 그 자식들을 재귀적으로 잇는다.
    fn hierarchy_order(&self) -> Vec<(usize, u32)> {
        fn visit(cur: usize, depth: u32, parents: &[Option<usize>], out: &mut Vec<(usize, u32)>) {
            out.push((cur, depth));
            for i in 0..parents.len() {
                if parents[i] == Some(cur) {
                    visit(i, depth + 1, parents, out);
                }
            }
        }
        let mut out = Vec::new();
        for i in 0..self.boxes.len() {
            if self.parents[i].is_none() {
                visit(i, 0, &self.parents, &mut out);
            }
        }
        out
    }

    // 창 좌표(mx,my) 아래에 있는 하이어라키 행의 상자 인덱스 — 클릭/드래그
    // 판정과 우클릭 메뉴 대상 찾기에 같이 쓴다. 사이드바 밖이거나 스크롤된
    // 범위 밖이면 None.
    fn hierarchy_row_at(&self, mx: f32, my: f32) -> Option<usize> {
        if !(0.0..SIDEBAR_W).contains(&mx) || my < HIER_LIST_TOP {
            return None;
        }
        let row = ((my - HIER_LIST_TOP) / ROW_H) as usize;
        if row >= HIER_VISIBLE_ROWS {
            return None;
        }
        let order = self.hierarchy_order();
        order.get(self.hier_scroll as usize + row).map(|&(idx, _)| idx)
    }

    fn root_zone_hit(&self, mx: f32, my: f32) -> bool {
        point_in_rect(mx, my, PAD, ROOT_ZONE_TOP, SIDEBAR_W - PAD * 2.0, ROOT_ZONE_H)
    }

    // 컨텍스트 메뉴 항목 목록 — target 이 있으면 그 상자용, 없으면 빈 곳용.
    fn context_menu_items(&self, target: Option<usize>) -> Vec<(&'static str, MenuAction)> {
        if let Some(t) = target {
            let mut items = vec![("Rename", MenuAction::Rename), ("Duplicate", MenuAction::Duplicate)];
            if self.parents.get(t).copied().flatten().is_some() {
                items.push(("Unparent", MenuAction::Unparent));
            }
            items.push(("Delete", MenuAction::Delete));
            items
        } else {
            vec![("New Box", MenuAction::NewBox)]
        }
    }

    // 메뉴의 화면 좌상단(x,y) + 항목 목록 — 화면 밖으로 안 나가게 clamp 한다.
    // 그리기와 클릭 판정이 항상 같은 계산을 쓰도록 한곳에 모아뒀다.
    fn context_menu_layout(&self, menu: &ContextMenu) -> (f32, f32, Vec<(&'static str, MenuAction)>) {
        let items = self.context_menu_items(menu.target);
        let h = items.len() as f32 * ROW_H;
        let x = menu.pos.0.clamp(0.0, WIN_W - MENU_W - 2.0);
        let y = menu.pos.1.clamp(0.0, WIN_H - h - 2.0);
        (x, y, items)
    }

    fn run_menu_action(&mut self, action: MenuAction, target: Option<usize>, pos: (f32, f32)) {
        match action {
            MenuAction::Rename => {
                if let Some(t) = target {
                    self.renaming = Some(t);
                    self.rename_buffer = self.names[t].clone();
                }
            }
            MenuAction::Duplicate => {
                if let Some(t) = target {
                    self.duplicate_box(t);
                }
            }
            MenuAction::Unparent => {
                if let Some(t) = target {
                    self.parents[t] = None;
                }
            }
            MenuAction::Delete => {
                if let Some(t) = target {
                    self.selected = Some(t);
                    self.multi_selected = vec![t];
                    self.remove_selected();
                }
            }
            MenuAction::NewBox => {
                let _ = pos; // 뷰포트 어디를 우클릭했든 지금은 카메라 정면에 놓는다(N 키와 같은 자리)
                let near = v_add(self.cam.pos, v_scale(self.cam.camera().forward(), 5.0));
                self.push_box(default_box(near));
            }
        }
    }

    // 컨텍스트 메뉴가 열려있을 때 왼쪽 클릭을 처리한다 — 메뉴 항목 위였으면 그
    // 동작을 실행하고, 메뉴 바깥이었으면 그냥 닫기만 한다(그 클릭 자체는
    // 선택/기즈모 등 평소 동작으로 이어지지 않는다 — 메뉴를 닫는 클릭 한 번은
    // "그것만" 한다는 보통 에디터들의 관례).
    fn handle_context_menu_click(&mut self, x: f32, y: f32) {
        let Some(menu) = self.context_menu else { return };
        let (mx, my, items) = self.context_menu_layout(&menu);
        if point_in_rect(x, y, mx, my, MENU_W, items.len() as f32 * ROW_H) {
            let row = ((y - my) / ROW_H) as usize;
            if let Some(&(_, action)) = items.get(row) {
                self.run_menu_action(action, menu.target, menu.pos);
            }
        }
        self.context_menu = None;
    }

    fn save(&mut self) {
        let scene = MapScene {
            boxes: (0..self.boxes.len())
                .map(|i| MapBoxData::from_box3d(&self.boxes[i], self.box_texture_paths[i].clone(), self.names[i].clone(), self.parents[i]))
                .collect(),
            player_start: self.player_start,
            player_start_yaw: self.player_start_yaw,
        };
        let _ = std::fs::create_dir_all("maps");
        match scene.save(SAVE_PATH) {
            Ok(()) => self.status = format!("저장함: {SAVE_PATH} ({}개)", self.boxes.len()),
            Err(e) => self.status = format!("저장 실패: {e}"),
        }
    }

    fn load(&mut self) {
        let scene = match MapScene::load(SAVE_PATH) {
            Ok(s) => s,
            Err(e) => {
                self.status = format!("불러오기 실패: {e}");
                return;
            }
        };
        self.box_texture_paths = scene.boxes.iter().map(|b| b.texture.clone()).collect();
        // 각 상자의 texture 문자열을 실제 TextureId 로 바꾼다 — self 를 통째로
        // 빌리는 클로저를 to_box3d 에 못 넘기니(같은 루프 안에서 self.boxes 도
        // 쓸 거라), 먼저 경로들만 전부 로드해 캐시를 채워두고 캐시 조회만 하는
        // 얕은 클로저를 대신 넘긴다.
        for p in self.box_texture_paths.clone().into_iter().flatten() {
            load_texture(self.ctx.as_mut(), &mut self.texture_cache, &mut self.status, &p);
        }
        let cache = &self.texture_cache;
        let mut resolve = |p: &str| -> TextureId { *cache.get(p).expect("위에서 미리 로드해뒀다") };
        self.boxes = scene.boxes.iter().map(|b| b.to_box3d(&mut resolve)).collect();
        self.names = scene.boxes.iter().enumerate().map(|(i, b)| if b.name.is_empty() { format!("Box {}", i + 1) } else { b.name.clone() }).collect();
        self.parents = scene.boxes.iter().map(|b| b.parent.filter(|&p| p < self.boxes.len())).collect();
        if self.boxes.is_empty() {
            self.push_box(default_box([0.0, 0.0, 0.0]));
        } else {
            self.selected = Some(0);
            self.multi_selected = vec![0];
        }
        self.player_start = scene.player_start;
        self.player_start_yaw = scene.player_start_yaw;
        self.status = format!("불러옴: {SAVE_PATH} ({}개)", self.boxes.len());
    }

    // 뷰포트 로컬 좌표(vx,vy, (0,0)이 뷰포트 왼쪽 위)에서 카메라를 통과하는
    // 레이를 쏴서 가장 가까이 맞는 상자의 인덱스를 돌려준다.
    fn pick_box(&self, vx: f32, vy: f32) -> Option<usize> {
        let ndc_x = (vx / VIEWPORT_W) * 2.0 - 1.0;
        let ndc_y = 1.0 - (vy / WIN_H) * 2.0;
        let half_h = (FOV_Y / 2.0).tan();
        let half_w = half_h * self.mesh3d.aspect();
        let cam = self.cam.camera();
        let (fwd, right, up) = (cam.forward(), cam.right(), cam.up());
        let dir = v_add(v_add(v_scale(right, ndc_x * half_w), v_scale(up, ndc_y * half_h)), fwd);
        let mut best: Option<(usize, f32)> = None;
        for (i, b) in self.boxes.iter().enumerate() {
            if let Some(t) = b.ray_intersect(self.cam.pos, dir)
                && best.is_none_or(|(_, bt)| t < bt)
            {
                best = Some((i, t));
            }
        }
        best.map(|(i, _)| i)
    }

    // 월드 좌표 → 뷰포트 로컬 화면 좌표(0,0 이 뷰포트 왼쪽 위, pick_box/기즈모와
    // 같은 좌표계). 카메라 뒤(depth<=0)에 있으면 None.
    fn world_to_screen(&self, p: [f32; 3]) -> Option<(f32, f32)> {
        let cam = self.cam.camera();
        let rel = v_sub(p, cam.pos);
        let (fwd, right, up) = (cam.forward(), cam.right(), cam.up());
        let depth = v_dot(rel, fwd);
        if depth <= 0.05 {
            return None;
        }
        let half_h = (FOV_Y / 2.0).tan();
        let half_w = half_h * self.mesh3d.aspect();
        let ndc_x = v_dot(rel, right) / (depth * half_w);
        let ndc_y = v_dot(rel, up) / (depth * half_h);
        Some(((ndc_x * 0.5 + 0.5) * VIEWPORT_W, (1.0 - (ndc_y * 0.5 + 0.5)) * WIN_H))
    }

    // 선택한 상자의 이동/회전/크기조절 기즈모 손잡이 3개(X/Y/Z)의 월드 위치
    // (회전 모드는 고리를 그리지만, "이 축을 잡았다"의 대표점으로는 여전히 쓴다).
    fn gizmo_handles(&self, b: &Box3D) -> [[f32; 3]; 3] {
        let axes = b.local_axes();
        let mut out = [[0.0f32; 3]; 3];
        for ((o, &ax), &half) in out.iter_mut().zip(axes.iter()).zip(b.half.iter()) {
            let dist = if self.gizmo_mode == GizmoMode::Scale { half + 0.4 } else { GIZMO_HANDLE_LEN };
            *o = v_add(b.center, v_scale(ax, dist));
        }
        out
    }

    // 회전 모드 고리 위의 점들(세계 좌표) — axis 를 뺀 나머지 두 로컬 축이
    // 펼치는 평면 위의 원. Move/Scale 손잡이 선과 달리 회전은 "축 둘레를
    // 도는 고리"로 그려야 뭘 조작하는지 바로 보인다.
    fn gizmo_ring_points(&self, b: &Box3D, axis: usize) -> Vec<[f32; 3]> {
        let axes = b.local_axes();
        let (o1, o2) = (axes[(axis + 1) % 3], axes[(axis + 2) % 3]);
        (0..=GIZMO_RING_SEGMENTS)
            .map(|i| {
                let t = i as f32 / GIZMO_RING_SEGMENTS as f32 * std::f32::consts::TAU;
                v_add(b.center, v_add(v_scale(o1, GIZMO_HANDLE_LEN * t.cos()), v_scale(o2, GIZMO_HANDLE_LEN * t.sin())))
            })
            .collect()
    }

    // 뷰포트 로컬 좌표 근처에서 기즈모의 어느 축을 가리키는지 찾는다 — 회전
    // 모드는 고리 위 샘플점까지 거리로, 이동/크기조절 모드는 중심→손잡이
    // 선분까지 거리로(끝점만이 아니라 선 전체가 판정 대상이라 너그럽다).
    fn nearest_gizmo_axis(&self, vx: f32, vy: f32) -> Option<usize> {
        let sel = self.selected?;
        let b = self.boxes.get(sel)?;
        let mut best: Option<(usize, f32)> = None;
        if self.gizmo_mode == GizmoMode::Rotate {
            for axis in 0..3 {
                for p in self.gizmo_ring_points(b, axis) {
                    if let Some((sx, sy)) = self.world_to_screen(p) {
                        let d = ((sx - vx).powi(2) + (sy - vy).powi(2)).sqrt();
                        if d < GIZMO_LINE_HIT_R && best.is_none_or(|(_, bd)| d < bd) {
                            best = Some((axis, d));
                        }
                    }
                }
            }
        } else {
            let center = self.world_to_screen(b.center)?;
            let handles = self.gizmo_handles(b);
            for (i, h) in handles.iter().enumerate() {
                if let Some((hx, hy)) = self.world_to_screen(*h) {
                    let d = dist_point_to_segment(vx, vy, center.0, center.1, hx, hy);
                    if d < GIZMO_LINE_HIT_R && best.is_none_or(|(_, bd)| d < bd) {
                        best = Some((i, d));
                    }
                }
            }
        }
        best.map(|(i, _)| i)
    }

    fn begin_gizmo_drag(&mut self, axis: usize, vx: f32, vy: f32) {
        let Some(sel) = self.selected else { return };
        let Some(b) = self.boxes.get(sel) else { return };
        let axes = b.local_axes();
        let center = b.center;
        self.drag_start_box = Some(b.clone());
        self.drag_axis = Some(axis);
        self.drag_start_mouse = (vx, vy);
        if let (Some(c), Some(t)) = (self.world_to_screen(center), self.world_to_screen(v_add(center, axes[axis]))) {
            let d = (t.0 - c.0, t.1 - c.1);
            let len = (d.0 * d.0 + d.1 * d.1).sqrt().max(1e-4);
            self.drag_screen_dir = (d.0 / len, d.1 / len);
            self.drag_pixels_per_unit = len;
        } else {
            self.drag_screen_dir = (1.0, 0.0);
            self.drag_pixels_per_unit = 50.0;
        }
        // 이동 모드일 때 같이 딸려갈 대상: 이 상자의 모든 자손 + 함께 선택된
        // (마퀴로 고른) 다른 상자들. 시작 위치를 미리 찍어둬야 드래그 도중
        // 매번 절대 델타를 다시 구해서 적용할 수 있다(상대 누적이면 오차가 쌓인다).
        let mut group: Vec<usize> = self.descendants(sel);
        for &i in &self.multi_selected {
            if i != sel && !group.contains(&i) {
                group.push(i);
            }
        }
        self.drag_group_start = group.iter().map(|&i| (i, self.boxes[i].center)).collect();
    }

    // 기즈모 손잡이를 잡은 채 마우스를 옮길 때마다 부른다 — 드래그 시작점부터의
    // 총 이동을 그 축의 화면-공간 방향에 투영해서(드래그 도중 방향이 바뀌어도
    // 흔들리지 않게 시작 시점 기준으로 고정) 이동/크기(월드 단위) 또는
    // 회전(라디안)으로 바꾼다.
    fn apply_gizmo_drag(&mut self, vx: f32, vy: f32) {
        let Some(sel) = self.selected else { return };
        let (Some(axis), Some(start)) = (self.drag_axis, self.drag_start_box.clone()) else { return };
        let total = (vx - self.drag_start_mouse.0, vy - self.drag_start_mouse.1);
        let proj = total.0 * self.drag_screen_dir.0 + total.1 * self.drag_screen_dir.1;
        match self.gizmo_mode {
            GizmoMode::Move => {
                let delta = proj / self.drag_pixels_per_unit;
                let axes = start.local_axes();
                let new_center = v_add(start.center, v_scale(axes[axis], delta));
                let world_delta = v_sub(new_center, start.center);
                if let Some(b) = self.boxes.get_mut(sel) {
                    b.center = new_center;
                }
                for (i, start_pos) in self.drag_group_start.clone() {
                    if let Some(gb) = self.boxes.get_mut(i) {
                        gb.center = v_add(start_pos, world_delta);
                    }
                }
            }
            GizmoMode::Scale => {
                let delta = proj / self.drag_pixels_per_unit;
                let mut half = start.half;
                half[axis] = (half[axis] + delta).max(MIN_HALF);
                if let Some(b) = self.boxes.get_mut(sel) {
                    b.half = half;
                }
            }
            GizmoMode::Rotate => {
                let delta = proj * GIZMO_ROTATE_SENS;
                if let Some(b) = self.boxes.get_mut(sel) {
                    match axis {
                        0 => b.pitch = start.pitch + delta,
                        1 => b.yaw = start.yaw + delta,
                        _ => b.roll = start.roll + delta,
                    }
                }
            }
        }
    }

    fn handle_input(&mut self, dt: f32) {
        if self.input.pressed(KeyCode::Tab) && !self.boxes.is_empty() {
            let n = self.boxes.len();
            let cur = self.selected.unwrap_or(0);
            let next = if self.input.is_down(KeyCode::LeftShift) || self.input.is_down(KeyCode::RightShift) { (cur + n - 1) % n } else { (cur + 1) % n };
            self.selected = Some(next);
            self.multi_selected = vec![next];
        }
        if self.input.pressed(KeyCode::N) {
            let near = v_add(self.cam.pos, v_scale(self.cam.camera().forward(), 5.0));
            self.push_box(default_box(near));
        }
        if self.input.pressed(KeyCode::Delete) {
            self.remove_selected();
        }
        if self.input.pressed(KeyCode::P) && let Some(sel) = self.selected && let Some(b) = self.boxes.get(sel) {
            self.player_start = [b.center[0], b.center[1] + b.half[1] + 0.1, b.center[2]];
            self.player_start_yaw = self.cam.yaw;
            self.status = "플레이어 시작 위치 설정함".to_string();
        }
        if self.ctrl_down && self.input.pressed(KeyCode::S) {
            self.save();
        }
        if self.ctrl_down && self.input.pressed(KeyCode::O) {
            self.load();
        }

        // W/E/R 로 기즈모 모드 전환(유니티와 같은 키) — 오른쪽 버튼을 눌러
        // 카메라를 날리는 중엔 W 가 "전진"이라 겹치니 그때는 반응하지 않는다.
        if !self.flying {
            if self.input.pressed(KeyCode::W) {
                self.gizmo_mode = GizmoMode::Move;
            }
            if self.input.pressed(KeyCode::E) {
                self.gizmo_mode = GizmoMode::Rotate;
            }
            if self.input.pressed(KeyCode::R) {
                self.gizmo_mode = GizmoMode::Scale;
            }
        }

        // 카메라 비행(오른쪽 버튼을 누르고 있을 때만).
        if self.flying {
            let fast = self.input.is_down(KeyCode::LeftShift) || self.input.is_down(KeyCode::RightShift);
            let speed = if fast { FLY_SPEED_FAST } else { FLY_SPEED } * dt;
            let cam = self.cam.camera();
            let (fwd, right) = (cam.forward(), cam.right());
            if self.input.is_down(KeyCode::W) {
                self.cam.pos = v_add(self.cam.pos, v_scale(fwd, speed));
            }
            if self.input.is_down(KeyCode::S) {
                self.cam.pos = v_add(self.cam.pos, v_scale(fwd, -speed));
            }
            if self.input.is_down(KeyCode::D) {
                self.cam.pos = v_add(self.cam.pos, v_scale(right, speed));
            }
            if self.input.is_down(KeyCode::A) {
                self.cam.pos = v_add(self.cam.pos, v_scale(right, -speed));
            }
            if self.input.is_down(KeyCode::E) {
                self.cam.pos[1] += speed;
            }
            if self.input.is_down(KeyCode::Q) {
                self.cam.pos[1] -= speed;
            }
        }

        if !self.flying && !self.marqueeing && self.drag_axis.is_none() {
            self.hovered_axis = self.nearest_gizmo_axis(self.input.mouse.0 - SIDEBAR_W, self.input.mouse.1);
        } else {
            self.hovered_axis = None;
        }
    }

    // 왼쪽 패널: 씬(들여쓰기된 오브젝트 트리, 고정 높이 + 스크롤) → 인스펙터
    // (기즈모 모드 버튼/이름 바꾸기/부모 지정/위치·회전·크기 -/+ 필드/
    // walkable·solid 토글/색상 순환/텍스처 목록). 목록 높이를 고정해뒀기 때문에
    // 오브젝트가 아무리 쌓여도 인스펙터 시작 위치는 항상 같다.
    fn draw_sidebar(&mut self) {
        self.renderer.rect(0.0, 0.0, SIDEBAR_W, WIN_H, COL_PANEL_BG);
        let mouse = self.input.mouse;
        let clicked = self.input.mouse_clicked;

        self.renderer.text(PAD, 4.0, "SCENE", 0.62, COL_TEXT_DIM);

        // 하이어라키 행을 드래그하는 동안(재부모 지정)에만 뜨는 "루트로 놓기"
        // 구역 — 여기에 놓으면 부모를 뗀다. 평소엔 옅게, 드래그 중 마우스가
        // 위에 있으면 파란 강조로.
        if self.hier_dragging {
            let hover = self.root_zone_hit(mouse.0, mouse.1);
            self.renderer.rect(PAD, ROOT_ZONE_TOP, SIDEBAR_W - PAD * 2.0, ROOT_ZONE_H, if hover { COL_ACTIVE } else { COL_FIELD });
            self.renderer.text(PAD + 4.0, ROOT_ZONE_TOP + 2.0, "— drop here to unparent —", 0.5, COL_TEXT);
        }

        let order = self.hierarchy_order();
        let max_scroll = order.len().saturating_sub(HIER_VISIBLE_ROWS) as f32;
        self.hier_scroll = self.hier_scroll.clamp(0.0, max_scroll);
        let start = self.hier_scroll as usize;
        let mut clicked_row: Option<usize> = None;
        for (row, &(idx, depth)) in order.iter().enumerate().skip(start).take(HIER_VISIBLE_ROWS) {
            let y = HIER_LIST_TOP + (row - start) as f32 * ROW_H;
            let indent = depth as f32 * INDENT_W;
            if self.renaming == Some(idx) {
                self.renderer.rect(PAD + indent, y, SIDEBAR_W - PAD * 2.0 - indent, ROW_H - 3.0, COL_ACTIVE);
                self.renderer.text(PAD + indent + 4.0, y + ROW_H / 2.0 - 7.0, &format!("{}_", self.rename_buffer), 0.6, [1.0, 1.0, 1.0, 1.0]);
            } else if self.hier_dragging && self.hier_press == Some(idx) {
                // 드래그 중인 행 자신 — 옅게 표시만(놓을 수 있는 대상이 아니다).
                self.renderer.text(PAD + indent + 4.0, y + ROW_H / 2.0 - 6.0, &self.names[idx], 0.62, COL_TEXT_DIM);
            } else {
                let is_sel = self.multi_selected.contains(&idx);
                let is_drop_hover = self.hier_dragging && point_in_rect(mouse.0, mouse.1, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0);
                if is_drop_hover {
                    self.renderer.rect(PAD + indent, y, SIDEBAR_W - PAD * 2.0 - indent, ROW_H - 3.0, COL_ACTIVE);
                    self.renderer.text(PAD + indent + 4.0, y + ROW_H / 2.0 - 6.0, &self.names[idx], 0.62, [1.0, 1.0, 1.0, 1.0]);
                } else if flat_button(&mut self.renderer, PAD + indent, y, SIDEBAR_W - PAD * 2.0 - indent, ROW_H - 3.0, &self.names[idx], mouse, clicked, is_sel) {
                    clicked_row = Some(idx);
                }
            }
        }
        if let Some(idx) = clicked_row {
            self.selected = Some(idx);
            self.multi_selected = vec![idx];
        }
        if order.len() > HIER_VISIBLE_ROWS {
            let hint = format!("{}/{} (wheel to scroll)", start + 1, order.len());
            self.renderer.text(PAD, HIER_LIST_TOP + HIER_VISIBLE_ROWS as f32 * ROW_H + 2.0, &hint, 0.5, COL_TEXT_DIM);
        }

        let mut y = HIER_LIST_TOP + HIER_VISIBLE_ROWS as f32 * ROW_H + 16.0;
        self.renderer.text(PAD, y, "INSPECTOR", 0.62, COL_TEXT_DIM);
        y += 18.0;

        let mut pending_texture: Option<Option<String>> = None;
        let mut pending_parent_delta = 0i32;

        if let Some(sel) = self.selected {
            // 기즈모 모드 버튼 3개 — W/E/R 단축키와 같은 동작을 클릭으로도.
            let modes = [GizmoMode::Move, GizmoMode::Rotate, GizmoMode::Scale];
            let btn_w = (SIDEBAR_W - PAD * 2.0 - 4.0) / 3.0;
            for (i, m) in modes.iter().enumerate() {
                let x = PAD + i as f32 * (btn_w + 2.0);
                if flat_button(&mut self.renderer, x, y, btn_w, ROW_H - 3.0, m.label(), mouse, clicked, self.gizmo_mode == *m) {
                    self.gizmo_mode = *m;
                }
            }
            y += ROW_H + 6.0;

            // 이름 바꾸기 — 누르면 하이어라키의 그 행이 바로 편집 모드로 바뀐다
            // (스크롤에서 벗어나 있었다면 보이게 스크롤도 옮긴다).
            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, &format!("Rename \"{}\"", self.names[sel]), mouse, clicked, false) {
                self.renaming = Some(sel);
                self.rename_buffer = self.names[sel].clone();
                if let Some(pos) = order.iter().position(|&(i, _)| i == sel) {
                    self.hier_scroll = (pos as f32).min(self.hier_scroll.max(0.0)).max(pos as f32 - (HIER_VISIBLE_ROWS - 1) as f32).max(0.0);
                }
            }
            y += ROW_H + 6.0;

            // 부모 지정 — 유니티의 "드래그해서 부모 밑에 넣기"까지는 못 하지만,
            // 순환(자기 자손을 부모로) 없이 순환 버튼으로 고를 수 있다.
            let candidates = self.candidate_parents(sel);
            let cur_parent = self.parents[sel];
            let parent_label = cur_parent.map(|p| self.names[p].clone()).unwrap_or_else(|| "None".to_string());
            if let Some(d) = cycle_row(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, "Parent", &parent_label, mouse, clicked) {
                pending_parent_delta = d;
            }
            if pending_parent_delta != 0 {
                let cur_idx = candidates.iter().position(|&c| c == cur_parent).unwrap_or(0) as i32;
                let n = candidates.len() as i32;
                let new_idx = ((cur_idx + pending_parent_delta) % n + n) % n;
                self.parents[sel] = candidates[new_idx as usize];
            }
            y += ROW_H + 8.0;

            let b = &self.boxes[sel];
            let (px, py, pz) = (b.center[0], b.center[1], b.center[2]);
            let (yaw, pitch, roll) = (b.yaw, b.pitch, b.roll);
            let (hx, hy, hz) = (b.half[0], b.half[1], b.half[2]);
            let (walkable, solid, color) = (b.walkable, b.solid, b.color);
            let (ax, ay, az) = (AXIS_COLORS[0], AXIS_COLORS[1], AXIS_COLORS[2]);

            let sections: [(&str, [InspectorRow; 3]); 3] = [
                (
                    "Position",
                    [
                        ("X", ax, px, POS_STEP, (|b, v| b.center[0] = v) as fn(&mut Box3D, f32)),
                        ("Y", ay, py, POS_STEP, |b, v| b.center[1] = v),
                        ("Z", az, pz, POS_STEP, |b, v| b.center[2] = v),
                    ],
                ),
                (
                    "Rotation",
                    [
                        ("Pitch", ax, pitch, ROT_STEP, (|b, v| b.pitch = v) as fn(&mut Box3D, f32)),
                        ("Yaw", ay, yaw, ROT_STEP, |b, v| b.yaw = v),
                        ("Roll", az, roll, ROT_STEP, |b, v| b.roll = v),
                    ],
                ),
                (
                    "Scale (half-extent)",
                    [
                        ("X", ax, hx, HALF_STEP, (|b, v| b.half[0] = v.max(MIN_HALF)) as fn(&mut Box3D, f32)),
                        ("Y", ay, hy, HALF_STEP, |b, v| b.half[1] = v.max(MIN_HALF)),
                        ("Z", az, hz, HALF_STEP, |b, v| b.half[2] = v.max(MIN_HALF)),
                    ],
                ),
            ];
            for (title, rows) in sections {
                self.renderer.text(PAD, y, title, 0.56, COL_TEXT_DIM);
                y += 14.0;
                for (label, label_color, value, step, apply) in rows {
                    if let Some(new_val) = stepper_row(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, label, label_color, value, step, mouse, clicked) {
                        apply(&mut self.boxes[sel], new_val);
                    }
                    y += ROW_H;
                }
                y += 4.0;
            }

            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, if walkable { "Walkable: On" } else { "Walkable: Off" }, mouse, clicked, walkable) {
                self.boxes[sel].walkable = !walkable;
            }
            y += ROW_H;
            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, if solid { "Solid: On" } else { "Solid: Off" }, mouse, clicked, solid) {
                self.boxes[sel].solid = !solid;
            }
            y += ROW_H + 6.0;

            self.renderer.rect(PAD, y + 2.0, 16.0, ROW_H - 7.0, color);
            if flat_button(&mut self.renderer, PAD + 22.0, y, SIDEBAR_W - PAD * 2.0 - 22.0, ROW_H - 3.0, "Cycle Color", mouse, clicked, false) {
                let cur = PALETTE.iter().position(|c| *c == color).unwrap_or(0);
                self.boxes[sel].color = PALETTE[(cur + 1) % PALETTE.len()];
            }
            y += ROW_H + 10.0;

            self.renderer.text(PAD, y, "TEXTURES (maps/textures/*.png)", 0.56, COL_TEXT_DIM);
            y += 16.0;
            let cur_path = self.box_texture_paths.get(sel).cloned().flatten();
            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, "[None] (flat color)", mouse, clicked, cur_path.is_none()) {
                pending_texture = Some(None);
            }
            y += ROW_H;
            if self.available_textures.is_empty() {
                self.renderer.text(PAD + 2.0, y, "(no .png in maps/textures/)", 0.54, COL_TEXT_DIM);
            }
            for path in self.available_textures.clone() {
                let name = path.rsplit('/').next().unwrap_or(&path).to_string();
                let is_cur = cur_path.as_deref() == Some(path.as_str());
                if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, &name, mouse, clicked, is_cur) {
                    pending_texture = Some(Some(path.clone()));
                }
                y += ROW_H;
            }

            if let Some(path) = pending_texture {
                let tex = path.as_deref().map(|p| load_texture(self.ctx.as_mut(), &mut self.texture_cache, &mut self.status, p));
                self.boxes[sel].texture = tex.map(|texture| BoxTexture { texture });
                self.box_texture_paths[sel] = path;
            }
        } else {
            self.renderer.text(PAD, y, "(nothing selected)", 0.58, COL_TEXT_DIM);
        }
    }

    // 우클릭 컨텍스트 메뉴 — 항상 맨 위(다른 모든 UI 위)에 그린다.
    fn draw_context_menu(&mut self) {
        let Some(menu) = self.context_menu else { return };
        let (x, y, items) = self.context_menu_layout(&menu);
        let h = items.len() as f32 * ROW_H;
        self.renderer.rect(x, y, MENU_W, h, COL_BORDER);
        let mouse = self.input.mouse;
        for (i, (label, _)) in items.iter().enumerate() {
            let iy = y + i as f32 * ROW_H;
            let hover = point_in_rect(mouse.0, mouse.1, x, iy, MENU_W, ROW_H);
            self.renderer.rect(x + 1.0, iy + 1.0, MENU_W - 2.0, ROW_H - 2.0, if hover { COL_FIELD_HOVER } else { COL_FIELD });
            self.renderer.text(x + 8.0, iy + ROW_H / 2.0 - 6.0, label, 0.6, COL_TEXT);
        }
    }

    fn draw_screen_dotted_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, thick: f32, color: [f32; 4]) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let steps = (len / 5.0).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            self.renderer.rect(x0 + dx * t - thick / 2.0, y0 + dy * t - thick / 2.0, thick, thick, color);
        }
    }

    // 뷰포트 위에 선택한 상자의 기즈모를 그린다 — 모드마다 다른 모양이라 한눈에
    // "지금 뭘 조작하는지" 알 수 있다: 이동=화살표, 회전=고리, 크기조절=끝에
    // 각진 손잡이. 회전 사각형을 지원 안 하는 렌더러라 선/고리는 작은 사각형
    // 점을 여러 개 이어 찍어서 흉내낸다(ui.rs::draw_scale 등과 같은 요령).
    fn draw_gizmo(&mut self) {
        let Some(sel) = self.selected else { return };
        let Some(b) = self.boxes.get(sel).cloned() else { return };
        if self.gizmo_mode == GizmoMode::Rotate {
            for (axis, &axis_color) in AXIS_COLORS.iter().enumerate() {
                let active = self.drag_axis == Some(axis) || self.hovered_axis == Some(axis);
                let color = if active { [1.0, 1.0, 1.0, 1.0] } else { axis_color };
                let thick = if active { 4.0 } else { 2.5 };
                let pts: Vec<(f32, f32)> = self.gizmo_ring_points(&b, axis).iter().filter_map(|&p| self.world_to_screen(p)).map(|(x, y)| (SIDEBAR_W + x, y)).collect();
                for w in pts.windows(2) {
                    self.draw_screen_dotted_line(w[0].0, w[0].1, w[1].0, w[1].1, thick, color);
                }
            }
            return;
        }

        let handles = self.gizmo_handles(&b);
        let Some(center_screen) = self.world_to_screen(b.center) else { return };
        let (cx, cy) = (SIDEBAR_W + center_screen.0, center_screen.1);
        for (i, h) in handles.iter().enumerate() {
            let Some((hx, hy)) = self.world_to_screen(*h) else { continue };
            let (tx, ty) = (SIDEBAR_W + hx, hy);
            let active = self.drag_axis == Some(i) || self.hovered_axis == Some(i);
            let color = if active { [1.0, 1.0, 1.0, 1.0] } else { AXIS_COLORS[i] };
            let thick = if active { 4.0 } else { 3.0 };
            self.draw_screen_dotted_line(cx, cy, tx, ty, thick, color);

            let (dx, dy) = (tx - cx, ty - cy);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let (ux, uy) = (dx / len, dy / len);
            let (px, py) = (-uy, ux); // 선에 수직인 방향(화살깃/각진 손잡이에 씀)

            match self.gizmo_mode {
                GizmoMode::Move => {
                    // 화살촉 — 끝점보다 살짝 앞에서 좌우로 벌어지는 짧은 선 두 개.
                    let back = 12.0;
                    let spread = 6.0;
                    let base = (tx - ux * back, ty - uy * back);
                    let left = (base.0 + px * spread, base.1 + py * spread);
                    let right = (base.0 - px * spread, base.1 - py * spread);
                    self.draw_screen_dotted_line(tx, ty, left.0, left.1, thick, color);
                    self.draw_screen_dotted_line(tx, ty, right.0, right.1, thick, color);
                }
                GizmoMode::Scale => {
                    // 각진(정사각형) 손잡이 — 화살표와 뚜렷이 구분되는 "블록" 모양.
                    let hs = if active { 7.0 } else { 5.5 };
                    self.renderer.rect(tx - hs, ty - hs, hs * 2.0, hs * 2.0, color);
                }
                GizmoMode::Rotate => unreachable!("위에서 이미 return 했다"),
            }
        }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        self.handle_input(dt);

        self.mesh3d.render(self.ctx.as_mut(), SKY_COLOR, &self.cam.camera(), &self.boxes, FOV_Y);

        self.renderer.begin(WIN_W, WIN_H);
        let tex = self.mesh3d.color_texture();
        self.renderer.sprite_uv(tex, SIDEBAR_W, 0.0, VIEWPORT_W, WIN_H, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        self.draw_gizmo();

        if self.marqueeing {
            let (x0, y0) = self.marquee_start;
            let (x1, y1) = self.input.mouse;
            let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
            let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
            self.renderer.rect(lo_x, lo_y, hi_x - lo_x, hi_y - lo_y, [0.3, 0.55, 0.9, 0.18]);
            self.renderer.rect(lo_x, lo_y, hi_x - lo_x, 1.0, [0.5, 0.75, 1.0, 0.8]);
            self.renderer.rect(lo_x, hi_y - 1.0, hi_x - lo_x, 1.0, [0.5, 0.75, 1.0, 0.8]);
            self.renderer.rect(lo_x, lo_y, 1.0, hi_y - lo_y, [0.5, 0.75, 1.0, 0.8]);
            self.renderer.rect(hi_x - 1.0, lo_y, 1.0, hi_y - lo_y, [0.5, 0.75, 1.0, 0.8]);
        }

        self.renderer.rect(SIDEBAR_W, 0.0, VIEWPORT_W, 40.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(SIDEBAR_W + 6.0, 4.0, "RMB drag: look + WASD/QE fly | LMB: select / drag = box-select", 0.6, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(SIDEBAR_W + 6.0, 19.0, &format!("Drag colored handle to transform | Gizmo: {} (W/E/R)", self.gizmo_mode.label()), 0.6, [1.0, 1.0, 1.0, 1.0]);

        self.draw_sidebar();

        self.renderer.rect(0.0, WIN_H - 18.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(6.0, WIN_H - 15.0, &self.status, 0.68, [0.7, 1.0, 0.7, 1.0]);

        self.draw_context_menu();

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, mods: KeyMods, repeat: bool) {
        // 이름을 입력하는 중엔 Enter/Esc/Backspace 만 반응하고, 그 외엔 Tab/N/
        // Delete/W/E/R 같은 단축키가 절대 안 먹게 여기서 막는다(안 그러면 이름에
        // "Wall" 이라고 치는 것만으로 기즈모 모드가 W/E/R 로 계속 바뀐다).
        if let Some(idx) = self.renaming {
            match keycode {
                KeyCode::Enter | KeyCode::KpEnter => {
                    let trimmed = self.rename_buffer.trim();
                    self.names[idx] = if trimmed.is_empty() { format!("Box {}", idx + 1) } else { trimmed.to_string() };
                    self.renaming = None;
                }
                KeyCode::Escape => self.renaming = None,
                KeyCode::Backspace => {
                    self.rename_buffer.pop();
                }
                _ => {}
            }
            return;
        }
        if keycode == KeyCode::Escape && self.context_menu.is_some() {
            self.context_menu = None;
            return;
        }
        if keycode == KeyCode::Escape {
            window::order_quit();
        }
        self.ctrl_down = mods.ctrl;
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, mods: KeyMods) {
        self.ctrl_down = mods.ctrl;
        self.input.on_key_up(keycode);
    }

    fn char_event(&mut self, character: char, _mods: KeyMods, _repeat: bool) {
        if self.renaming.is_some() && !character.is_control() && self.rename_buffer.chars().count() < 40 {
            self.rename_buffer.push(character);
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let (x, y) = to_virtual(x, y);
        let dx = x - self.last_mouse.0;
        let dy = y - self.last_mouse.1;
        self.last_mouse = (x, y);
        self.input.mouse = (x, y);

        if self.drag_axis.is_some() {
            self.apply_gizmo_drag(x - SIDEBAR_W, y);
            return;
        }
        if self.marqueeing {
            self.left_drag_dist += (dx * dx + dy * dy).sqrt();
        }
        if let Some(_src) = self.hier_press {
            let d = ((x - self.hier_press_pos.0).powi(2) + (y - self.hier_press_pos.1).powi(2)).sqrt();
            if d > MARQUEE_MIN_DRAG {
                self.hier_dragging = true;
            }
        }
        if self.flying {
            self.rmb_drag_dist += (dx * dx + dy * dy).sqrt();
            // 화면 아래로 드래그(dy>0)하면 아래를 보게(pitch 감소) — 마우스가 위로
            // 갈수록 pitch 가 올라가야(위를 봐야) 자연스럽다. yaw 는 forward()가
            // -sin(yaw) 방향이라 마우스를 오른쪽(dx>0)으로 움직이면 yaw 를 줄여야
            // 시점이 실제로 오른쪽을 본다.
            self.cam.yaw -= dx * LOOK_SENS;
            self.cam.pitch = (self.cam.pitch - dy * LOOK_SENS).clamp(-MAX_PITCH, MAX_PITCH);
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let (x, y) = to_virtual(x, y);
        self.last_mouse = (x, y);
        self.input.mouse = (x, y);
        if button == MouseButton::Left {
            self.input.mouse_down = true;
            self.input.mouse_clicked = true;
            // 컨텍스트 메뉴가 떠 있는 동안엔 그 클릭이 메뉴만 처리한다 — 평소
            // 클릭 동작(선택/기즈모/드래그 시작)으로는 절대 안 이어진다.
            if self.context_menu.is_some() {
                self.handle_context_menu_click(x, y);
                return;
            }
        }
        match button {
            MouseButton::Right => {
                self.context_menu = None;
                self.flying = true;
                self.rmb_press_pos = (x, y);
                self.rmb_drag_dist = 0.0;
            }
            MouseButton::Left if x < SIDEBAR_W => {
                if let Some(idx) = self.hierarchy_row_at(x, y)
                    && self.renaming != Some(idx)
                {
                    self.hier_press = Some(idx);
                    self.hier_press_pos = (x, y);
                }
            }
            MouseButton::Left if x >= SIDEBAR_W && !self.flying => {
                if let Some(axis) = self.nearest_gizmo_axis(x - SIDEBAR_W, y) {
                    self.begin_gizmo_drag(axis, x - SIDEBAR_W, y);
                } else {
                    self.marqueeing = true;
                    self.marquee_start = (x, y);
                    self.left_drag_dist = 0.0;
                }
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let (x, y) = to_virtual(x, y);
        if button == MouseButton::Left {
            self.input.mouse_down = false;
        }
        match button {
            MouseButton::Right => {
                self.flying = false;
                if self.rmb_drag_dist < MARQUEE_MIN_DRAG {
                    // 드래그가 아니라 그냥 우클릭 — 블렌더/유니티처럼 그 자리의
                    // 오브젝트(있으면 먼저 선택도 해준다)에 대한 메뉴를 띄운다.
                    let target = if x < SIDEBAR_W { self.hierarchy_row_at(x, y) } else { self.pick_box(x - SIDEBAR_W, y) };
                    if let Some(t) = target {
                        self.selected = Some(t);
                        self.multi_selected = vec![t];
                    }
                    self.context_menu = Some(ContextMenu { pos: (x, y), target });
                }
            }
            MouseButton::Left => {
                if let Some(src) = self.hier_press.take() {
                    if self.hier_dragging {
                        // 드래그해서 재부모 지정 — 루트 구역에 놓으면 부모를 떼고,
                        // 다른(자기 자손이 아닌) 행 위에 놓으면 그 상자를 부모로 삼는다.
                        if self.root_zone_hit(x, y) {
                            self.parents[src] = None;
                            self.status = format!("\"{}\" → 루트로 이동", self.names[src]);
                        } else if let Some(target) = self.hierarchy_row_at(x, y)
                            && target != src
                            && !self.is_descendant(src, target)
                        {
                            self.parents[src] = Some(target);
                            self.status = format!("\"{}\" → \"{}\" 의 자식으로", self.names[src], self.names[target]);
                        }
                    }
                    self.hier_dragging = false;
                } else if self.drag_axis.is_some() {
                    self.drag_axis = None;
                    self.drag_start_box = None;
                    self.drag_group_start.clear();
                } else if self.marqueeing {
                    if self.left_drag_dist < MARQUEE_MIN_DRAG {
                        // 거의 안 움직였으면 드래그 선택이 아니라 그냥 클릭 — 레이로
                        // 찍은 상자 하나만 선택(빈 곳이면 선택 해제).
                        if x >= SIDEBAR_W {
                            match self.pick_box(x - SIDEBAR_W, y) {
                                Some(i) => {
                                    self.selected = Some(i);
                                    self.multi_selected = vec![i];
                                }
                                None => {
                                    self.selected = None;
                                    self.multi_selected.clear();
                                }
                            }
                        }
                    } else {
                        // 드래그 선택 — 사각형 안에 화면상 중심이 들어오는 상자를 전부.
                        let (x0, y0) = (self.marquee_start.0 - SIDEBAR_W, self.marquee_start.1);
                        let (x1, y1) = (x - SIDEBAR_W, y);
                        let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
                        let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
                        let hits: Vec<usize> = self
                            .boxes
                            .iter()
                            .enumerate()
                            .filter_map(|(i, b)| self.world_to_screen(b.center).filter(|&(sx, sy)| sx >= lo_x && sx <= hi_x && sy >= lo_y && sy <= hi_y).map(|_| i))
                            .collect();
                        if !hits.is_empty() {
                            self.selected = hits.first().copied();
                            self.multi_selected = hits;
                        }
                    }
                    self.marqueeing = false;
                }
            }
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        if self.input.mouse.0 < SIDEBAR_W {
            self.hier_scroll -= y;
        } else {
            let fwd = self.cam.camera().forward();
            self.cam.pos = v_add(self.cam.pos, v_scale(fwd, y * DOLLY_SPEED));
        }
    }

    // 마우스가 창 밖으로 나가면(RMB 로 시점을 크게 돌리다 보면 아주 흔하다)
    // 그 바깥에서 손을 뗀 버튼-업 이벤트를 우리가 못 받을 수 있다 — 그러면
    // self.flying/drag_axis 가 계속 true 로 "갇혀서", 창 안으로 마우스가
    // 돌아오기만 해도 카메라가 계속 돌거나 상자가 계속 움직이는 버그가 된다.
    // 이 이벤트는 항상 안정적으로 오니 여기서 강제로 다 꺼서 막는다.
    fn mouse_leave_event(&mut self) {
        self.cancel_all_drags();
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "PalaceOS Map Editor".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        high_dpi: false,
        // miniquad 는 기본적으로 창이 리사이즈 가능하다(`window_resizable` 기본값
        // true) — 이 창을 최대화/리사이즈하면 실제 화면 크기(window::screen_size())가
        // 800x600 을 벗어나는데, 렌더러는 여전히 고정된 800x600 가상 해상도로
        // 그리기 때문에(CRT 가상 해상도 변환이 없는 단순한 창이라) 리사이즈된 창의
        // 실제 마우스 좌표와 그 가상 좌표계가 완전히 어긋나 버튼/기즈모가 전혀
        // 안 눌리는 것처럼 보인다 — 아예 리사이즈를 막아서 그 문제 자체를 없앤다
        // (to_virtual() 로 한 번 더 방어하지만, 제일 확실한 건 애초에 안 어긋나게
        // 하는 것).
        window_resizable: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
