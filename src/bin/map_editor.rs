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
//! 왼쪽엔 씬(오브젝트 목록) + 그 아래 인스펙터(선택한 상자 정보 + 텍스처 선택)
//! 패널이 있고, 오른쪽이 3D 뷰포트다. `maps/textures/*.png`에 이미지를 넣어두면
//! 인스펙터의 텍스처 목록에 떠서 클릭 한 번으로 선택한 상자에 입힐 수 있다.
//!
//! 조작(유니티 씬 뷰와 비슷하게 맞췄다):
//! - **뷰포트에서 마우스 왼쪽 클릭**(드래그 없이): 그 자리의 상자를 선택. **왼쪽
//!   패널의 오브젝트/텍스처 목록 클릭**도 마찬가지로 선택/할당한다.
//! - **마우스 오른쪽 버튼을 누른 채** 드래그: 시점 회전(마우스룩), 그 상태에서
//!   W/A/S/D 로 그 방향을 향해 날아다니고 Q/E 로 위/아래로 움직인다(Shift 로 빠르게).
//! - **마우스 왼쪽 버튼을 누른 채(뷰포트에서)** 드래그: 카메라 앞의 한 점을
//!   중심으로 궤도 회전.
//! - 휠: 보고 있는 방향으로 카메라를 앞/뒤로 이동(줌, 아주 조금씩).
//! - Tab / Shift+Tab: 다음/이전 상자 선택, N: 카메라 앞에 새 상자, Delete: 선택 삭제
//! - 방향키 ←/→/↑/↓: 선택한 상자를 세계 X/Z 로 이동, PageUp/PageDown: Y(위/아래) 이동
//! - ,/. : yaw 회전, [/] : pitch 회전, ;/' : roll 회전
//! - U/J, I/K, O/L: 각각 가로/높이/세로 반너비 확대/축소
//! - C: 색상 팔레트 순환, F: walkable 토글, G: solid 토글
//! - P: 플레이어 시작 위치/방향을 지금 선택한 상자 자리로 설정
//! - Ctrl+S: 저장, Ctrl+O: 불러오기, Esc: 종료

use std::collections::HashMap;

use miniquad::*;

use crackhead::gfx::Renderer;
use crackhead::mapfile::{MapBoxData, MapScene};
use crackhead::mesh3d::{v_add, v_scale, Box3D, BoxTexture, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 600.0;
const SIDEBAR_W: f32 = 220.0;
const VIEWPORT_W: f32 = WIN_W - SIDEBAR_W;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const SAVE_PATH: &str = "maps/scene.json";
const TEXTURE_DIR: &str = "maps/textures";
const ROW_H: f32 = 17.0;
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

const MOVE_STEP_SPEED: f32 = 2.5; // 선택한 상자를 옮기는 속도
const ROT_SPEED: f32 = 1.4;
const SCALE_SPEED: f32 = 1.0;
const MIN_HALF: f32 = 0.05;

const FLY_SPEED: f32 = 4.0;
const FLY_SPEED_FAST: f32 = 12.0;
const LOOK_SENS: f32 = 0.0035; // 마우스 1px 당 라디안
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const ORBIT_PIVOT_DIST: f32 = 8.0; // 왼쪽 드래그로 궤도 회전할 때 카메라 앞 몇 미터를 중심점으로 잡을지
const DOLLY_SPEED: f32 = 0.15; // 휠 한 칸당 전진/후진 거리

fn default_box(center: [f32; 3]) -> Box3D {
    Box3D { center, half: [0.5, 0.5, 0.5], yaw: 0.0, pitch: 0.0, roll: 0.0, color: PALETTE[0], texture: None, walkable: true, solid: true }
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
    flying: bool,           // 오른쪽 버튼을 누르고 있는 동안 — 마우스룩 + WASD 비행
    orbiting: bool,         // 왼쪽 버튼을 누르고 있는 동안 — 피벗 중심 궤도 회전
    orbit_pivot: [f32; 3],  // 이번 왼쪽 드래그를 시작할 때 잡은 중심점(드래그 도중엔 고정)
    left_drag_dist: f32,    // 왼쪽 버튼을 누른 뒤 총 이동 거리 — 문턱보다 작으면 드래그가 아니라 클릭
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

    // 경로별로 한 번만 디코드/업로드한다(캐시). 실패하면 상태 표시줄에 알리고
    // 1x1 흰 텍스처(색 곱만 먹는 자리표시자)를 대신 돌려준다.
    fn load_texture(&mut self, path: &str) -> TextureId {
        if let Some(&tex) = self.texture_cache.get(path) {
            return tex;
        }
        let tex = match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as u16, rgba.height() as u16);
                self.ctx.new_texture_from_rgba8(w, h, &rgba)
            }
            Err(e) => {
                self.status = format!("텍스처 로드 실패({path}): {e}");
                self.ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255])
            }
        };
        self.texture_cache.insert(path.to_string(), tex);
        tex
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

    // 선택한 상자에 텍스처를 입히거나(Some) 뗀다(None) — 인스펙터의 텍스처
    // 목록 클릭에서 부른다.
    fn assign_texture(&mut self, path: Option<String>) {
        let tex = path.as_deref().map(|p| self.load_texture(p));
        if let Some(b) = self.boxes.get_mut(self.selected) {
            b.texture = tex.map(|texture| BoxTexture { texture });
        }
        if let Some(p) = self.box_texture_paths.get_mut(self.selected) {
            *p = path;
        }
    }

    fn selected_box(&mut self) -> Option<&mut Box3D> {
        self.boxes.get_mut(self.selected)
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
            self.load_texture(&p);
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

    // 사이드바 레이아웃(씬 목록 → 인스펙터 정보 → 텍스처 목록, 순서대로 아래에
    // 쌓인다) — 그리기(draw)와 클릭 판정(sidebar_click)이 같은 좌표를 쓰도록
    // 한곳에 모아뒀다.
    fn hierarchy_top(&self) -> f32 {
        20.0
    }
    fn inspector_header_top(&self) -> f32 {
        self.hierarchy_top() + self.boxes.len() as f32 * ROW_H + 12.0
    }
    fn inspector_info_top(&self) -> f32 {
        self.inspector_header_top() + 18.0
    }
    fn texture_header_top(&self) -> f32 {
        self.inspector_info_top() + 5.0 * 14.0 + 10.0
    }
    fn texture_list_top(&self) -> f32 {
        self.texture_header_top() + 18.0
    }

    fn sidebar_click(&mut self, x: f32, y: f32) {
        if !(0.0..SIDEBAR_W).contains(&x) {
            return;
        }
        let hier_top = self.hierarchy_top();
        let hier_bottom = hier_top + self.boxes.len() as f32 * ROW_H;
        if y >= hier_top && y < hier_bottom {
            self.selected = ((y - hier_top) / ROW_H) as usize;
            return;
        }
        let tex_top = self.texture_list_top();
        let tex_count = 1 + self.available_textures.len();
        let tex_bottom = tex_top + tex_count as f32 * ROW_H;
        if y >= tex_top && y < tex_bottom {
            let idx = ((y - tex_top) / ROW_H) as usize;
            if idx == 0 {
                self.assign_texture(None);
            } else if let Some(path) = self.available_textures.get(idx - 1).cloned() {
                self.assign_texture(Some(path));
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

        // 카메라 비행(오른쪽 버튼을 누르고 있을 때만) — WASD/QE 가 상자 편집과
        // 겹치지 않게, 상자 편집은 전부 다른 키로 옮겨뒀다(아래).
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

        let move_step = MOVE_STEP_SPEED * dt;
        let rot_step = ROT_SPEED * dt;
        let scale_step = SCALE_SPEED * dt;
        let color_cycle = self.input.pressed(KeyCode::C);
        let toggle_walk = self.input.pressed(KeyCode::F);
        let toggle_solid = self.input.pressed(KeyCode::G);
        let (left, right, up, down) = (
            self.input.is_down(KeyCode::Left),
            self.input.is_down(KeyCode::Right),
            self.input.is_down(KeyCode::Up),
            self.input.is_down(KeyCode::Down),
        );
        let (page_up, page_down) = (self.input.is_down(KeyCode::PageUp), self.input.is_down(KeyCode::PageDown));
        let (comma, period) = (self.input.is_down(KeyCode::Comma), self.input.is_down(KeyCode::Period));
        let (lbracket, rbracket) = (self.input.is_down(KeyCode::LeftBracket), self.input.is_down(KeyCode::RightBracket));
        let (semicolon, apostrophe) = (self.input.is_down(KeyCode::Semicolon), self.input.is_down(KeyCode::Apostrophe));
        let (u, j, i, k, o, l) = (
            self.input.is_down(KeyCode::U),
            self.input.is_down(KeyCode::J),
            self.input.is_down(KeyCode::I),
            self.input.is_down(KeyCode::K),
            self.input.is_down(KeyCode::O),
            self.input.is_down(KeyCode::L),
        );

        if let Some(b) = self.selected_box() {
            if left {
                b.center[0] -= move_step;
            }
            if right {
                b.center[0] += move_step;
            }
            if up {
                b.center[2] -= move_step;
            }
            if down {
                b.center[2] += move_step;
            }
            if page_up {
                b.center[1] += move_step;
            }
            if page_down {
                b.center[1] -= move_step;
            }
            if comma {
                b.yaw -= rot_step;
            }
            if period {
                b.yaw += rot_step;
            }
            if lbracket {
                b.pitch -= rot_step;
            }
            if rbracket {
                b.pitch += rot_step;
            }
            if semicolon {
                b.roll -= rot_step;
            }
            if apostrophe {
                b.roll += rot_step;
            }
            if u {
                b.half[0] = (b.half[0] + scale_step).max(MIN_HALF);
            }
            if j {
                b.half[0] = (b.half[0] - scale_step).max(MIN_HALF);
            }
            if i {
                b.half[1] = (b.half[1] + scale_step).max(MIN_HALF);
            }
            if k {
                b.half[1] = (b.half[1] - scale_step).max(MIN_HALF);
            }
            if o {
                b.half[2] = (b.half[2] + scale_step).max(MIN_HALF);
            }
            if l {
                b.half[2] = (b.half[2] - scale_step).max(MIN_HALF);
            }
            if color_cycle {
                let cur = PALETTE.iter().position(|c| *c == b.color).unwrap_or(0);
                b.color = PALETTE[(cur + 1) % PALETTE.len()];
            }
            if toggle_walk {
                b.walkable = !b.walkable;
            }
            if toggle_solid {
                b.solid = !b.solid;
            }
        }
    }

    // 왼쪽 패널: 씬(오브젝트 목록) → 인스펙터(선택한 상자 정보) → 텍스처 목록.
    // 클릭 판정은 sidebar_click() 이 이 함수와 같은 좌표 계산(hierarchy_top() 등)을
    // 그대로 써서 항상 화면에 보이는 자리와 맞는다.
    fn draw_sidebar(&mut self) {
        self.renderer.rect(0.0, 0.0, SIDEBAR_W, WIN_H, [0.13, 0.13, 0.16, 1.0]);
        self.renderer.text(6.0, 4.0, "Scene", 0.7, [0.8, 0.85, 1.0, 1.0]);

        let hier_top = self.hierarchy_top();
        for i in 0..self.boxes.len() {
            let y = hier_top + i as f32 * ROW_H;
            if i == self.selected {
                self.renderer.rect(2.0, y, SIDEBAR_W - 4.0, ROW_H, [0.25, 0.35, 0.55, 1.0]);
            }
            self.renderer.text(8.0, y + 2.0, &format!("Box {}", i + 1), 0.62, [0.9, 0.9, 0.9, 1.0]);
        }

        let inspector_header_top = self.inspector_header_top();
        self.renderer.text(6.0, inspector_header_top, "Inspector", 0.7, [0.8, 0.85, 1.0, 1.0]);
        let info_top = self.inspector_info_top();
        if let Some(b) = self.boxes.get(self.selected) {
            let lines = [
                format!("pos {:.2},{:.2},{:.2}", b.center[0], b.center[1], b.center[2]),
                format!("half {:.2},{:.2},{:.2}", b.half[0], b.half[1], b.half[2]),
                format!("rot {:.2},{:.2},{:.2}", b.yaw, b.pitch, b.roll),
                format!("walk={} solid={}", b.walkable, b.solid),
                "texture:".to_string(),
            ];
            for (i, line) in lines.iter().enumerate() {
                self.renderer.text(8.0, info_top + i as f32 * 14.0, line, 0.58, [1.0, 1.0, 0.6, 1.0]);
            }
        }

        let texture_header_top = self.texture_header_top();
        self.renderer.text(6.0, texture_header_top, "Textures (maps/textures/*.png)", 0.62, [0.8, 0.85, 1.0, 1.0]);
        let cur_path = self.box_texture_paths.get(self.selected).cloned().flatten();
        let tex_top = self.texture_list_top();
        // "[None]" 행 — 텍스처를 떼고 단색으로 되돌린다.
        if cur_path.is_none() {
            self.renderer.rect(2.0, tex_top, SIDEBAR_W - 4.0, ROW_H, [0.25, 0.35, 0.55, 1.0]);
        }
        self.renderer.text(8.0, tex_top + 2.0, "[None] (flat color)", 0.6, [0.9, 0.9, 0.9, 1.0]);
        if self.available_textures.is_empty() {
            self.renderer.text(8.0, tex_top + ROW_H + 2.0, "(no .png found)", 0.58, [0.6, 0.6, 0.6, 1.0]);
        }
        for (i, path) in self.available_textures.clone().iter().enumerate() {
            let y = tex_top + (i + 1) as f32 * ROW_H;
            if cur_path.as_deref() == Some(path.as_str()) {
                self.renderer.rect(2.0, y, SIDEBAR_W - 4.0, ROW_H, [0.25, 0.35, 0.55, 1.0]);
            }
            let name = path.rsplit('/').next().unwrap_or(path);
            self.renderer.text(8.0, y + 2.0, name, 0.6, [0.9, 0.9, 0.9, 1.0]);
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

        self.renderer.rect(SIDEBAR_W, 0.0, VIEWPORT_W, 56.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(SIDEBAR_W + 6.0, 4.0, "RMB drag: look + WASD/QE fly (Shift fast)", 0.65, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(SIDEBAR_W + 6.0, 19.0, "LMB click: select, LMB drag: orbit, wheel: zoom", 0.65, [1.0, 1.0, 1.0, 1.0]);
        self.renderer.text(SIDEBAR_W + 6.0, 34.0, "arrows+PgUp/Dn move, ,.[]; ' rotate, U/J I/K O/L scale", 0.65, [1.0, 1.0, 1.0, 1.0]);

        self.draw_sidebar();

        self.renderer.rect(0.0, WIN_H - 18.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.6]);
        self.renderer.text(6.0, WIN_H - 15.0, &self.status, 0.68, [0.7, 1.0, 0.7, 1.0]);

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
        let dx = x - self.last_mouse.0;
        let dy = y - self.last_mouse.1;
        self.last_mouse = (x, y);
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
        self.last_mouse = (x, y);
        match button {
            MouseButton::Right => self.flying = true,
            // 사이드바 위에서 누른 왼쪽 클릭은 그 자리에서 바로 처리하고(오브젝트/
            // 텍스처 목록 선택), 뷰포트 오빗 드래그를 시작하지 않는다.
            MouseButton::Left if x < SIDEBAR_W => self.sidebar_click(x, y),
            MouseButton::Left if !self.flying => {
                self.orbiting = true;
                self.left_drag_dist = 0.0;
                self.orbit_pivot = v_add(self.cam.pos, v_scale(self.cam.camera().forward(), ORBIT_PIVOT_DIST));
            }
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        match button {
            MouseButton::Right => self.flying = false,
            MouseButton::Left => {
                // 드래그가 거의 없었으면(문턱 이하) 오빗이 아니라 "그 자리를
                // 클릭해서 선택"한 것으로 친다 — 뷰포트 안에서 뗐을 때만 유효.
                if self.orbiting && self.left_drag_dist < CLICK_DRAG_THRESHOLD && x >= SIDEBAR_W && let Some(i) = self.pick_box(x - SIDEBAR_W, y) {
                    self.selected = i;
                }
                self.orbiting = false;
            }
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        let fwd = self.cam.camera().forward();
        self.cam.pos = v_add(self.cam.pos, v_scale(fwd, y * DOLLY_SPEED));
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "PalaceOS Map Editor".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
