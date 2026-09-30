//! 창 안에서 도는 1인칭 3D 게임들(apps/doors_game, apps/maze_game)이 같이 쓰는 도구 —
//! 게임 화면 배치/렌더(View), 마우스 시점 모드(MouseLook), 플레이어(player.rs),
//! 타자기 대화창(dialogue.rs). 3D 장면은 `mesh3d.rs` 로 640x480 오프스크린에 그린 뒤 그
//! 결과를 창 안에 4:3 으로 끼워 넣는다(남는 곳은 검은 띠). CRT 효과는 바깥 OS 화면
//! 전체에 이미 걸려 있어서 따로 안 입힌다. HUD 좌표는 전부 VIEW_W x VIEW_H 기준이고
//! 그릴 때 View 가 배율을 곱해준다.

pub mod dialogue;
pub mod player;

use std::cell::RefCell;

use miniquad::{KeyCode, RenderingBackend, TextureId};

use crate::render::gfx::{Rect, Renderer};
use crate::render::mesh3d::{v_dot, v_sub, Box3D, Camera, Mesh3D};

use super::WinInput;
use player::Player;

// 3D 오프스크린 해상도(4:3) — HUD 좌표도 이 크기 기준.
pub const VIEW_W: f32 = 640.0;
pub const VIEW_H: f32 = 480.0;
pub const FOV_Y: f32 = std::f32::consts::PI / 3.2;
const MOUSE_SENS: f32 = 0.0032; // 실제 화면 픽셀당 라디안

// Mesh3D(오프스크린 타깃 + 셰이더/파이프라인)는 창을 열 때마다 새로 만들면 닫을
// 때 지울 방법이 없어(App 은 Drop 에서 ctx 를 못 받는다) GPU 자원이 계속 쌓인다 —
// 그래서 한 번 만든 걸 스레드 전역에 두고 게임 창들이 같이 재사용한다(3D 게임 창은
// 한 프레임에 하나씩 차례로 그려진다).
thread_local! {
    static MESH3D: RefCell<Option<Mesh3D>> = const { RefCell::new(None) };
}

// 창 영역 안에 4:3 으로 맞춰 넣은 게임 화면 자리 + VIEW 기준 → 실제 화면 배율.
pub struct View {
    pub rect: Rect,
    pub s: f32,
}

impl View {
    pub fn fit(area: Rect) -> View {
        let s = (area.w / VIEW_W).min(area.h / VIEW_H);
        let (w, h) = (VIEW_W * s, VIEW_H * s);
        View { rect: Rect::new(area.x + (area.w - w) / 2.0, area.y + (area.h - h) / 2.0, w, h), s }
    }

    pub fn px(&self, x: f32) -> f32 {
        self.rect.x + x * self.s
    }

    pub fn py(&self, y: f32) -> f32 {
        self.rect.y + y * self.s
    }

    // 3D 장면을 그려서 창에 붙이고(남는 곳은 검은 띠) 조준선까지 그린다.
    pub fn draw_scene(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, clear: [f32; 4], cam: &Camera, boxes: &[Box3D]) {
        let tex = render_scene(ctx, clear, cam, boxes);
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        // 오프스크린 텍스처는 위아래가 뒤집혀 있어 v 를 뒤집어 붙인다(crt.rs 와 같은 이유).
        r.sprite_uv(tex, self.rect.x, self.rect.y, self.rect.w, self.rect.h, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);
        let s = self.s;
        r.rect(self.px(VIEW_W / 2.0 - 1.5), self.py(VIEW_H / 2.0 - 1.5), 3.0 * s, 3.0 * s, [1.0, 1.0, 1.0, 0.7]);
    }

    // 월드 위치 옆에 "[E] ..." 같은 상호작용 안내를 띄운다(카메라 뒤면 안 그림).
    pub fn draw_prompt(&self, r: &mut Renderer, cam: &Camera, pos: [f32; 3], label: &str) {
        let Some((sx, sy)) = world_to_view(cam, pos) else { return };
        let s = self.s;
        let tw = r.text_width(label, 0.7 * s);
        r.rect(self.px(sx + 10.0), self.py(sy - 10.0), tw + 8.0 * s, 16.0 * s, [0.0, 0.0, 0.0, 0.6]);
        r.text(self.px(sx + 14.0), self.py(sy - 8.0), label, 0.7 * s, [0.6, 0.9, 1.0, 1.0]);
    }

    // 화면 아래 가운데의 "클릭해서 시점 조작" 안내.
    pub fn draw_capture_hint(&self, r: &mut Renderer) {
        let s = self.s;
        let hint = "클릭해서 시점 조작 (Esc: 해제)";
        let tw = r.text_width(hint, 0.7 * s);
        let cx = self.px(VIEW_W / 2.0);
        r.rect(cx - tw / 2.0 - 6.0 * s, self.py(VIEW_H - 30.0), tw + 12.0 * s, 20.0 * s, [0.0, 0.0, 0.0, 0.6]);
        r.text(cx - tw / 2.0, self.py(VIEW_H - 27.0), hint, 0.7 * s, [0.9, 0.9, 0.9, 1.0]);
    }
}

fn render_scene(ctx: &mut dyn RenderingBackend, clear: [f32; 4], cam: &Camera, boxes: &[Box3D]) -> TextureId {
    MESH3D.with(|cell| {
        let mut slot = cell.borrow_mut();
        let mesh = slot.get_or_insert_with(|| Mesh3D::new(ctx, VIEW_W as u32, VIEW_H as u32));
        mesh.render(ctx, clear, cam, boxes, FOV_Y);
        mesh.color_texture()
    })
}

// 월드 좌표 → VIEW_W x VIEW_H 기준 화면 좌표. 카메라 뒤면 None.
fn world_to_view(cam: &Camera, p: [f32; 3]) -> Option<(f32, f32)> {
    let rel = v_sub(p, cam.pos);
    let depth = v_dot(rel, cam.forward());
    if depth <= 0.05 {
        return None;
    }
    let half_h = (FOV_Y / 2.0).tan();
    let half_w = half_h * (VIEW_W / VIEW_H);
    let ndc_x = v_dot(rel, cam.right()) / (depth * half_w);
    let ndc_y = v_dot(rel, cam.up()) / (depth * half_h);
    Some(((ndc_x * 0.5 + 0.5) * VIEW_W, (1.0 - (ndc_y * 0.5 + 0.5)) * VIEW_H))
}

// 마우스 시점 모드 — 게임 화면을 클릭하면 켜지고(커서가 사라지고 마우스로 시점이 돈다,
// main.rs 가 scenes::request_mouse_look() 요청을 받아 처리) Esc/포커스 잃음으로 풀린다.
pub struct MouseLook {
    captured: bool,
}

impl MouseLook {
    pub fn new() -> MouseLook {
        MouseLook { captured: false }
    }

    pub fn captured(&self) -> bool {
        self.captured
    }

    // 매 프레임 입력 처리 앞머리에서 부른다 — true 면 이번 프레임 입력은 여기서 다 썼다는
    // 뜻이다(창이 포커스가 아님, 또는 시점 모드로 들어가는 첫 클릭 — 대화 넘기기 등으로 안 샌다).
    pub fn update(&mut self, win: &WinInput, in_view: bool) -> bool {
        if !win.focused {
            self.captured = false;
            return true;
        }
        if win.input.pressed(KeyCode::Escape) {
            self.captured = false;
        }
        if win.mouse_clicked && in_view && !self.captured {
            self.captured = true;
            return true;
        }
        false
    }

    // 시점 모드인 동안 매 프레임 main.rs 에 "유지해달라"고 알린다.
    pub fn request(&self) {
        if self.captured {
            crate::scenes::request_mouse_look();
        }
    }

    // 시점 모드면 이번 프레임에 움직인 마우스만큼 플레이어 시점을 돌린다.
    pub fn apply(&self, player: &mut Player, win: &WinInput) {
        if self.captured {
            let (dx, dy) = win.input.look_delta;
            player.look(dx * MOUSE_SENS, -dy * MOUSE_SENS);
        }
    }
}
