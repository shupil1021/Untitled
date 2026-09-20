// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! PICOCAD 스타일의 아주 작은 맵 에디터 — `mesh3d.rs`(진짜 3D 메쉬 렌더러) 위에
//! 상자(`Box3D`)를 놓고 옮기고 회전/크기조절/텍스처 지정해서
//! `mapfile::MapScene`(JSON)으로 저장한다. 이 게임 전용 최소 기능만 넣었다:
//! 여러 오브젝트 조합/정점 단위 편집 같은 진짜 PICOCAD 급 범용 기능은 없다.
//!
//! `cargo run --bin map_editor` 로 띄운다. 저장/불러오기 경로는 고정으로
//! `maps/scene.json`(실행 파일 기준 상대 경로)을 쓴다.
//!
//! UI 는 Win9x 위젯(`ui::button`) 대신 이 파일 안에서 직접 그리는 플랫 다크
//! 테마다(`flat_button`/`stepper_row`) — 유니티 인스펙터(어두운 회색 패널,
//! 살짝 밝은 사각 필드, 파란 강조색)를 참고했다. 왼쪽엔 씬(오브젝트 목록) +
//! 그 아래 인스펙터(기즈모 모드 버튼 + 위치/회전/크기 -/+ 필드 + walkable/
//! solid 토글 + 색상 순환 + 텍스처 목록) 패널이 있고, 오른쪽이 3D 뷰포트다.
//! 오브젝트를 다루는 키보드 단축키(예전엔 화살표/,.[];'/UIOJKL 로 이동·회전·
//! 크기조절을 했었다)는 전부 없앴다 — 인스펙터 UI 버튼이나 뷰포트의 기즈모를
//! 직접 드래그해서 조절한다.
//!
//! 조작:
//! - **뷰포트에서 마우스 왼쪽 클릭**(드래그 없이): 그 자리의 상자를 선택.
//!   **왼쪽 패널의 오브젝트 목록 클릭**도 마찬가지.
//! - **선택한 상자에 뜨는 기즈모**(빨강=X, 초록=Y, 파랑=Z 축 손잡이)를 왼쪽
//!   버튼으로 잡고 드래그하면 그 축을 따라 이동/회전/크기조절한다. 중심에서
//!   손잡이까지 이어진 선 어디를 잡아도 된다(끝점만 정확히 노려야 하는 게
//!   아니다). `W`=이동, `E`=회전, `R`=크기조절 모드(유니티와 같은 키, 또는
//!   인스펙터의 모드 버튼을 클릭해도 된다) — 마우스 오른쪽 버튼을 누르고
//!   있지 않을 때만 반응한다(그때는 W가 카메라 비행에 쓰인다).
//! - **마우스 오른쪽 버튼을 누른 채** 드래그: 시점 회전(마우스룩), 그 상태에서
//!   W/A/S/D 로 그 방향을 향해 날아다니고 Q/E 로 위/아래로 움직인다(Shift 로 빠르게).
//! - **마우스 왼쪽 버튼을 누른 채(뷰포트의 빈 곳)** 드래그: 카메라 앞의 한 점을
//!   중심으로 궤도 회전.
//! - 휠: 보고 있는 방향으로 카메라를 앞/뒤로 이동(줌, 아주 조금씩).
//! - Tab / Shift+Tab: 다음/이전 상자 선택, N: 카메라 앞에 새 상자, Delete: 선택 삭제
//! - P: 플레이어 시작 위치/방향을 지금 선택한 상자 자리로 설정
//! - Ctrl+S: 저장, Ctrl+O: 불러오기, Esc: 종료
//!
//! 창 밖으로 마우스가 나가면(`mouse_leave_event`) 비행/오빗/기즈모 드래그
//! 상태를 전부 강제로 끈다 — 안 그러면 오른쪽 버튼을 누른 채 화면 가장자리
//! 밖에서 손을 떼는 순간 그 릴리즈 이벤트를 못 받아서 "비행 모드에 계속
//! 갇히는"(이후 마우스를 움직이기만 해도 카메라가 계속 도는) 버그가 있었다.

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
const CLICK_DRAG_THRESHOLD: f32 = 4.0; // 이 픽셀 이하로 움직였으면 드래그가 아니라 클릭으로 친다

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
const ORBIT_PIVOT_DIST: f32 = 8.0; // 왼쪽 드래그로 궤도 회전할 때 카메라 앞 몇 미터를 중심점으로 잡을지
const DOLLY_SPEED: f32 = 0.15; // 휠 한 칸당 전진/후진 거리

const GIZMO_HANDLE_LEN: f32 = 1.3; // 이동/회전 손잡이가 중심에서 떨어진 거리(월드 단위)
const GIZMO_LINE_HIT_R: f32 = 14.0; // 중심→손잡이 선 어디든 이 픽셀 반경 안이면 잡힌다
const GIZMO_ROTATE_SENS: f32 = 0.012; // 회전 모드에서 픽셀당 라디안

const AXIS_COLORS: [[f32; 4]; 3] = [[0.95, 0.35, 0.35, 1.0], [0.4, 0.9, 0.4, 1.0], [0.4, 0.6, 1.0, 1.0]];

// ================= 플랫 다크 테마 UI =================
// Win9x 위젯(ui::button 등)은 이 창의 어두운 배경과 어울리지 않아서(밝은
// 베벨 버튼이 둥둥 떠 보임) 안 쓰고, 유니티 인스펙터를 참고해 직접 그린다:
// 어두운 회색 패널, 그보다 살짝 밝은 사각 필드, 선택/켜짐은 파란 강조색.

const COL_FIELD: [f32; 4] = [0.22, 0.22, 0.23, 1.0];
const COL_FIELD_HOVER: [f32; 4] = [0.28, 0.28, 0.3, 1.0];
const COL_ACTIVE: [f32; 4] = [0.16, 0.44, 0.78, 1.0];
const COL_BORDER: [f32; 4] = [0.08, 0.08, 0.09, 1.0];
const COL_TEXT: [f32; 4] = [0.82, 0.82, 0.84, 1.0];
const COL_TEXT_DIM: [f32; 4] = [0.5, 0.5, 0.52, 1.0];
const COL_PANEL_BG: [f32; 4] = [0.145, 0.145, 0.155, 1.0];

// 실제 창 좌표 → 이 창이 가정하는 가상 해상도(800x600). main.rs::to_virtual()
// 과 같은 이유로 필요하다 — high_dpi:false 로 대부분 막히지만(director_panel.rs
// 에도 같은 조치가 있다), 실제 화면 크기(window::screen_size())가 conf 에 넣은
// window_width/height 와 어떤 이유로든 어긋나면(드문 DPI/창관리자 조합) 마우스
// 좌표와 렌더러가 그리는 좌표계가 안 맞아 클릭이 죄다 빗나간다 — 매 이벤트마다
// 실제 화면 크기를 다시 재서 비율로 보정하면 그 어긋남과 무관하게 항상 맞는다.
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
// 지금 고른 텍스처 등)로 그린다.
#[allow(clippy::too_many_arguments)]
fn flat_button(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, label: &str, mouse: (f32, f32), clicked: bool, active: bool) -> bool {
    let hover = point_in_rect(mouse.0, mouse.1, x, y, w, h);
    let bg = if active { COL_ACTIVE } else if hover { COL_FIELD_HOVER } else { COL_FIELD };
    r.rect(x, y, w, h, COL_BORDER);
    r.rect(x + 1.0, y + 1.0, w - 2.0, h - 2.0, bg);
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

// 유니티 씬 카메라처럼: 위치 + yaw/pitch 를 직접 들고 있는 자유 카메라. 오빗(왼쪽
// 드래그)은 이 위/피치를 바꾼 뒤 "카메라 앞 고정 거리의 피벗을 중심으로 그
// 거리만큼 뒤로" 위치를 다시 계산하는 방식으로 흉내낸다 — 별도의 "타깃" 상태를
// 계속 들고 다닐 필요가 없다.
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
    // self.boxes 와 항상 같은 길이/순서로 유지한다 — 저장 포맷은 텍스처를
    // TextureId(런타임 GPU 핸들)가 아니라 경로 문자열로 들고 있어야 해서
    // (mapfile::MapBoxData), Box3D 자체엔 없는 이 정보를 따로 보관한다.
    box_texture_paths: Vec<Option<String>>,
    selected: usize,
    player_start: [f32; 3],
    player_start_yaw: f32,
    cam: FlyCam,
    flying: bool,          // 오른쪽 버튼을 누르고 있는 동안 — 마우스룩 + WASD 비행
    orbiting: bool,        // 왼쪽 버튼을 누르고 있는 동안(뷰포트, 기즈모 아닌 빈 곳) — 피벗 중심 궤도 회전
    orbit_pivot: [f32; 3], // 이번 왼쪽 드래그를 시작할 때 잡은 중심점(드래그 도중엔 고정)
    left_drag_dist: f32,   // 왼쪽 버튼을 누른 뒤 총 이동 거리 — 문턱보다 작으면 드래그가 아니라 클릭
    gizmo_mode: GizmoMode,
    hovered_axis: Option<usize>,   // 지금 프레임에 마우스가 근처에 있는 기즈모 축(드래그 전 미리보기용)
    drag_axis: Option<usize>,      // 지금 드래그 중인 기즈모 축(0=X/1=Y/2=Z) — None 이면 기즈모 드래그 아님
    drag_start_box: Option<Box3D>, // 드래그 시작 시점의 상자 스냅샷(그 기준으로 델타를 계산)
    drag_start_mouse: (f32, f32),  // 뷰포트 로컬 좌표(사이드바 폭을 뺀 좌표)
    drag_pixels_per_unit: f32,     // 그 축 방향 1 월드 단위가 화면에서 몇 픽셀인지(드래그 시작 시점 기준)
    drag_screen_dir: (f32, f32),   // 그 축이 화면에 투영된 단위 방향
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
            selected: 0,
            player_start: [0.0, 1.0, -3.0],
            player_start_yaw: std::f32::consts::FRAC_PI_2,
            cam: FlyCam { pos: [6.0, 4.0, 6.0], yaw: -std::f32::consts::FRAC_PI_4 * 3.0, pitch: -0.5 },
            flying: false,
            orbiting: false,
            orbit_pivot: [0.0, 0.0, 0.0],
            left_drag_dist: 0.0,
            gizmo_mode: GizmoMode::Move,
            hovered_axis: None,
            drag_axis: None,
            drag_start_box: None,
            drag_start_mouse: (0.0, 0.0),
            drag_pixels_per_unit: 50.0,
            drag_screen_dir: (1.0, 0.0),
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
        self.boxes.push(b);
        self.box_texture_paths.push(None);
        self.selected = self.boxes.len() - 1;
    }

    fn remove_selected_box(&mut self) {
        if self.boxes.is_empty() {
            return;
        }
        self.boxes.remove(self.selected);
        self.box_texture_paths.remove(self.selected);
        if self.selected >= self.boxes.len() && self.selected > 0 {
            self.selected -= 1;
        }
        if self.boxes.is_empty() {
            self.boxes.push(default_box([0.0, 0.0, 0.0]));
            self.box_texture_paths.push(None);
            self.selected = 0;
        }
    }

    // 진행 중이던 비행/오빗/기즈모 드래그를 전부 강제로 끈다 — 마우스가 창
    // 밖으로 나가서 버튼 릴리즈를 못 받는 경우의 안전장치(mouse_leave_event).
    fn cancel_all_drags(&mut self) {
        self.flying = false;
        self.orbiting = false;
        self.drag_axis = None;
        self.drag_start_box = None;
    }

    fn save(&mut self) {
        let scene = MapScene {
            boxes: self.boxes.iter().zip(self.box_texture_paths.iter()).map(|(b, p)| MapBoxData::from_box3d(b, p.clone())).collect(),
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
        if self.boxes.is_empty() {
            self.boxes.push(default_box([0.0, 0.0, 0.0]));
            self.box_texture_paths.push(None);
        }
        self.player_start = scene.player_start;
        self.player_start_yaw = scene.player_start_yaw;
        self.selected = 0;
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

    // 선택한 상자의 이동/회전/크기조절 기즈모 손잡이 3개(X/Y/Z)의 월드 위치.
    fn gizmo_handles(&self, b: &Box3D) -> [[f32; 3]; 3] {
        let axes = b.local_axes();
        let mut out = [[0.0f32; 3]; 3];
        for ((o, &ax), &half) in out.iter_mut().zip(axes.iter()).zip(b.half.iter()) {
            let dist = if self.gizmo_mode == GizmoMode::Scale { half + 0.4 } else { GIZMO_HANDLE_LEN };
            *o = v_add(b.center, v_scale(ax, dist));
        }
        out
    }

    // 뷰포트 로컬 좌표 근처에서 기즈모의 중심→손잡이 선 중 가장 가까운 축을
    // 찾는다(끝점만이 아니라 선 전체가 클릭 판정 대상이라 훨씬 너그럽다).
    fn nearest_gizmo_axis(&self, vx: f32, vy: f32) -> Option<usize> {
        let b = self.boxes.get(self.selected)?;
        let center = self.world_to_screen(b.center)?;
        let handles = self.gizmo_handles(b);
        let mut best: Option<(usize, f32)> = None;
        for (i, h) in handles.iter().enumerate() {
            if let Some((hx, hy)) = self.world_to_screen(*h) {
                let d = dist_point_to_segment(vx, vy, center.0, center.1, hx, hy);
                if d < GIZMO_LINE_HIT_R && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    fn begin_gizmo_drag(&mut self, axis: usize, vx: f32, vy: f32) {
        let Some(b) = self.boxes.get(self.selected) else { return };
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
    }

    // 기즈모 손잡이를 잡은 채 마우스를 옮길 때마다 부른다 — 드래그 시작점부터의
    // 총 이동을 그 축의 화면-공간 방향에 투영해서(드래그 도중 방향이 바뀌어도
    // 흔들리지 않게 시작 시점 기준으로 고정) 이동/크기(월드 단위) 또는
    // 회전(라디안)으로 바꾼다.
    fn apply_gizmo_drag(&mut self, vx: f32, vy: f32) {
        let (Some(axis), Some(start)) = (self.drag_axis, self.drag_start_box.clone()) else { return };
        let total = (vx - self.drag_start_mouse.0, vy - self.drag_start_mouse.1);
        let proj = total.0 * self.drag_screen_dir.0 + total.1 * self.drag_screen_dir.1;
        let Some(b) = self.boxes.get_mut(self.selected) else { return };
        match self.gizmo_mode {
            GizmoMode::Move => {
                let delta = proj / self.drag_pixels_per_unit;
                let axes = start.local_axes();
                b.center = v_add(start.center, v_scale(axes[axis], delta));
            }
            GizmoMode::Scale => {
                let delta = proj / self.drag_pixels_per_unit;
                let mut half = start.half;
                half[axis] = (half[axis] + delta).max(MIN_HALF);
                b.half = half;
            }
            GizmoMode::Rotate => {
                let delta = proj * GIZMO_ROTATE_SENS;
                match axis {
                    0 => b.pitch = start.pitch + delta,
                    1 => b.yaw = start.yaw + delta,
                    _ => b.roll = start.roll + delta,
                }
            }
        }
    }

    fn handle_input(&mut self, dt: f32) {
        if self.input.pressed(KeyCode::Tab) && !self.boxes.is_empty() {
            let n = self.boxes.len();
            self.selected = if self.input.is_down(KeyCode::LeftShift) || self.input.is_down(KeyCode::RightShift) {
                (self.selected + n - 1) % n
            } else {
                (self.selected + 1) % n
            };
        }
        if self.input.pressed(KeyCode::N) {
            let near = v_add(self.cam.pos, v_scale(self.cam.camera().forward(), 5.0));
            self.push_box(default_box(near));
        }
        if self.input.pressed(KeyCode::Delete) && !self.boxes.is_empty() {
            self.remove_selected_box();
        }
        if self.input.pressed(KeyCode::P) && let Some(b) = self.boxes.get(self.selected) {
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

        if !self.flying && !self.orbiting && self.drag_axis.is_none() {
            self.hovered_axis = self.nearest_gizmo_axis(self.input.mouse.0 - SIDEBAR_W, self.input.mouse.1);
        } else {
            self.hovered_axis = None;
        }
    }

    // 왼쪽 패널: 씬(오브젝트 목록) → 인스펙터(기즈모 모드 버튼 + 위치/회전/
    // 크기 -/+ 필드 + walkable/solid 토글 + 색상 순환 + 텍스처 목록). 전부
    // flat_button/stepper_row(이 파일에서 직접 그리는 플랫 다크 위젯)라서
    // self 를 안 빌리는 자유함수 호출뿐이고, 그 자리에서 바로 self.boxes 를
    // 고쳐도 borrow 문제가 없다.
    fn draw_sidebar(&mut self) {
        self.renderer.rect(0.0, 0.0, SIDEBAR_W, WIN_H, COL_PANEL_BG);
        let mouse = self.input.mouse;
        let clicked = self.input.mouse_clicked;

        self.renderer.text(PAD, 6.0, "SCENE", 0.62, COL_TEXT_DIM);
        let mut y = 22.0;
        for i in 0..self.boxes.len() {
            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, &format!("Box {}", i + 1), mouse, clicked, i == self.selected) {
                self.selected = i;
            }
            y += ROW_H;
        }

        y += 10.0;
        self.renderer.text(PAD, y, "INSPECTOR", 0.62, COL_TEXT_DIM);
        y += 18.0;

        let mut pending_texture: Option<Option<String>> = None;

        if self.selected < self.boxes.len() {
            // 기즈모 모드 버튼 3개 — W/E/R 단축키와 같은 동작을 클릭으로도.
            let modes = [GizmoMode::Move, GizmoMode::Rotate, GizmoMode::Scale];
            let btn_w = (SIDEBAR_W - PAD * 2.0 - 4.0) / 3.0;
            for (i, m) in modes.iter().enumerate() {
                let x = PAD + i as f32 * (btn_w + 2.0);
                if flat_button(&mut self.renderer, x, y, btn_w, ROW_H - 3.0, m.label(), mouse, clicked, self.gizmo_mode == *m) {
                    self.gizmo_mode = *m;
                }
            }
            y += ROW_H + 8.0;

            let b = &self.boxes[self.selected];
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
                        apply(&mut self.boxes[self.selected], new_val);
                    }
                    y += ROW_H;
                }
                y += 4.0;
            }

            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, if walkable { "Walkable: On" } else { "Walkable: Off" }, mouse, clicked, walkable) {
                self.boxes[self.selected].walkable = !walkable;
            }
            y += ROW_H;
            if flat_button(&mut self.renderer, PAD, y, SIDEBAR_W - PAD * 2.0, ROW_H - 3.0, if solid { "Solid: On" } else { "Solid: Off" }, mouse, clicked, solid) {
                self.boxes[self.selected].solid = !solid;
            }
            y += ROW_H + 6.0;

            self.renderer.rect(PAD, y + 2.0, 16.0, ROW_H - 7.0, color);
            if flat_button(&mut self.renderer, PAD + 22.0, y, SIDEBAR_W - PAD * 2.0 - 22.0, ROW_H - 3.0, "Cycle Color", mouse, clicked, false) {
                let cur = PALETTE.iter().position(|c| *c == color).unwrap_or(0);
                self.boxes[self.selected].color = PALETTE[(cur + 1) % PALETTE.len()];
            }
            y += ROW_H + 10.0;

            self.renderer.text(PAD, y, "TEXTURES (maps/textures/*.png)", 0.56, COL_TEXT_DIM);
            y += 16.0;
            let cur_path = self.box_texture_paths.get(self.selected).cloned().flatten();
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
        }

        if let Some(path) = pending_texture {
            let tex = path.as_deref().map(|p| load_texture(self.ctx.as_mut(), &mut self.texture_cache, &mut self.status, p));
            self.boxes[self.selected].texture = tex.map(|texture| BoxTexture { texture });
            self.box_texture_paths[self.selected] = path;
        }
    }

    // 뷰포트 위에 선택한 상자의 기즈모(중심→손잡이 선 + 손잡이 사각형)를
    // 그린다. 회전 사각형을 지원 안 하는 렌더러라, 선은 작은 사각형 점을 여러
    // 개 이어 찍어서 흉내낸다(ui.rs::draw_scale 등과 같은 요령). 지금 드래그
    // 중이거나 마우스가 근처에 있는 축은 더 굵고 밝게 그려서 어느 축을 잡게
    // 될지 드래그 전에 미리 알 수 있게 한다.
    fn draw_gizmo(&mut self) {
        let Some(b) = self.boxes.get(self.selected) else { return };
        let handles = self.gizmo_handles(b);
        let Some(center_screen) = self.world_to_screen(b.center) else { return };
        for (i, h) in handles.iter().enumerate() {
            let Some((hx, hy)) = self.world_to_screen(*h) else { continue };
            let (cx, cy) = (SIDEBAR_W + center_screen.0, center_screen.1);
            let (tx, ty) = (SIDEBAR_W + hx, hy);
            let active = self.drag_axis == Some(i) || self.hovered_axis == Some(i);
            let color = if active { [1.0, 1.0, 1.0, 1.0] } else { AXIS_COLORS[i] };
            let thick = if active { 4.0 } else { 3.0 };
            let dx = tx - cx;
            let dy = ty - cy;
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let steps = (len / 5.0).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                self.renderer.rect(cx + dx * t - thick / 2.0, cy + dy * t - thick / 2.0, thick, thick, color);
            }
            let hs = if active { 7.0 } else { 5.0 };
            self.renderer.rect(tx - hs, ty - hs, hs * 2.0, hs * 2.0, AXIS_COLORS[i]);
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

        self.renderer.rect(SIDEBAR_W, 0.0, VIEWPORT_W, 40.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(SIDEBAR_W + 6.0, 4.0, "RMB drag: look + WASD/QE fly | LMB: select/orbit | wheel: zoom", 0.6, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(SIDEBAR_W + 6.0, 19.0, &format!("Drag colored handle to transform | Gizmo: {} (W/E/R)", self.gizmo_mode.label()), 0.6, [1.0, 1.0, 1.0, 1.0]);

        self.draw_sidebar();

        self.renderer.rect(0.0, WIN_H - 18.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.6]);
        let debug = format!(
            "mouse=({:.0},{:.0}) fly={} orbit={} drag={:?} | {}",
            self.input.mouse.0, self.input.mouse.1, self.flying, self.orbiting, self.drag_axis, self.status
        );
        self.renderer.text(6.0, WIN_H - 15.0, &debug, 0.6, [0.7, 1.0, 0.7, 1.0]);

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, mods: KeyMods, repeat: bool) {
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
        if self.orbiting {
            self.left_drag_dist += (dx * dx + dy * dy).sqrt();
        }
        if self.flying {
            // 화면 아래로 드래그(dy>0)하면 아래를 보게(pitch 감소) — 마우스가 위로
            // 갈수록 pitch 가 올라가야(위를 봐야) 자연스럽다. yaw 는 forward()가
            // -sin(yaw) 방향이라 마우스를 오른쪽(dx>0)으로 움직이면 yaw 를 줄여야
            // 시점이 실제로 오른쪽을 본다.
            self.cam.yaw -= dx * LOOK_SENS;
            self.cam.pitch = (self.cam.pitch - dy * LOOK_SENS).clamp(-MAX_PITCH, MAX_PITCH);
        } else if self.orbiting {
            self.cam.yaw -= dx * LOOK_SENS;
            self.cam.pitch = (self.cam.pitch - dy * LOOK_SENS).clamp(-MAX_PITCH, MAX_PITCH);
            // 드래그를 시작할 때 잡아둔 피벗을 중심으로, 새 yaw/pitch 방향에서
            // 고정 거리만큼 뒤로 물러난 자리가 새 카메라 위치다 — 카메라가 항상
            // 그 피벗을 바라보게 된다(유니티의 알트+왼쪽 드래그와 같은 느낌).
            let fwd = self.cam.camera().forward();
            self.cam.pos = v_add(self.orbit_pivot, v_scale(fwd, -ORBIT_PIVOT_DIST));
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let (x, y) = to_virtual(x, y);
        self.last_mouse = (x, y);
        self.input.mouse = (x, y);
        if button == MouseButton::Left {
            self.input.mouse_down = true;
            self.input.mouse_clicked = true;
        }
        match button {
            MouseButton::Right => self.flying = true,
            MouseButton::Left if x >= SIDEBAR_W && !self.flying => {
                if let Some(axis) = self.nearest_gizmo_axis(x - SIDEBAR_W, y) {
                    self.begin_gizmo_drag(axis, x - SIDEBAR_W, y);
                } else {
                    self.orbiting = true;
                    self.left_drag_dist = 0.0;
                    self.orbit_pivot = v_add(self.cam.pos, v_scale(self.cam.camera().forward(), ORBIT_PIVOT_DIST));
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
            MouseButton::Right => self.flying = false,
            MouseButton::Left => {
                if self.drag_axis.is_some() {
                    self.drag_axis = None;
                    self.drag_start_box = None;
                } else {
                    // 드래그가 거의 없었으면(문턱 이하) 오빗이 아니라 "그 자리를
                    // 클릭해서 선택"한 것으로 친다 — 뷰포트 안에서 뗐을 때만 유효.
                    if self.orbiting && self.left_drag_dist < CLICK_DRAG_THRESHOLD && x >= SIDEBAR_W && let Some(i) = self.pick_box(x - SIDEBAR_W, y) {
                        self.selected = i;
                    }
                    self.orbiting = false;
                }
            }
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        let fwd = self.cam.camera().forward();
        self.cam.pos = v_add(self.cam.pos, v_scale(fwd, y * DOLLY_SPEED));
    }

    // 마우스가 창 밖으로 나가면(RMB 로 시점을 크게 돌리다 보면 아주 흔하다)
    // 그 바깥에서 손을 뗀 버튼-업 이벤트를 우리가 못 받을 수 있다 — 그러면
    // self.flying/orbiting/drag_axis 가 계속 true 로 "갇혀서", 창 안으로 마우스가
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
