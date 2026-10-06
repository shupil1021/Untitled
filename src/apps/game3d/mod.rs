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

// 게임 창마다 자기 전용 3D 오프스크린 타깃(Mesh3D)이 필요하다 — 창 안 장면은 이 텍스처를
// 2D 렌더러에 "스프라이트로 그리라"고 예약만 해두고 실제 그리기는 프레임 끝에 한꺼번에
// 일어나서, 창들이 타깃 하나를 같이 쓰면 마지막에 렌더한 창의 장면이 모든 창에 똑같이
// 나온다(예전 버그). 그렇다고 창을 열 때마다 새로 만들면 닫을 때 지울 방법이 없어(App 은
// Drop 에서 ctx 를 못 받는다) GPU 자원이 계속 쌓인다 — 그래서 타깃을 스레드 전역 풀에 두고
// 창이 RenderSlot 으로 한 칸을 빌려 쓰다가(Drop 때 반납) 다음 창이 그 칸을 재사용한다.
struct Pool {
    targets: Vec<Option<Mesh3D>>, // 칸마다 처음 쓸 때 만든다
    in_use: Vec<bool>,
}

thread_local! {
    static POOL: RefCell<Pool> = const { RefCell::new(Pool { targets: Vec::new(), in_use: Vec::new() }) };
}

// 풀에서 빌린 3D 타깃 한 칸 — 게임 앱이 들고 있다가 View::draw_scene 에 넘긴다.
pub struct RenderSlot(usize);

impl RenderSlot {
    pub fn acquire() -> RenderSlot {
        POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            let i = match pool.in_use.iter().position(|&used| !used) {
                Some(i) => i,
                None => {
                    pool.in_use.push(false);
                    pool.targets.push(None);
                    pool.in_use.len() - 1
                }
            };
            pool.in_use[i] = true;
            RenderSlot(i)
        })
    }
}

impl Drop for RenderSlot {
    fn drop(&mut self) {
        POOL.with(|pool| pool.borrow_mut().in_use[self.0] = false);
    }
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
    #[allow(clippy::too_many_arguments)]
    pub fn draw_scene(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D]) {
        let tex = render_scene(ctx, slot, clear, cam, boxes);
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

fn render_scene(ctx: &mut dyn RenderingBackend, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D]) -> TextureId {
    POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        let mesh = pool.targets[slot.0].get_or_insert_with(|| Mesh3D::new(ctx, VIEW_W as u32, VIEW_H as u32));
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

// 장면이 확 바뀌는 곳(미로 ↔ 방)에서 쓰는 페이드 — 화면이 검게 어두워졌다가(Out) 그 순간 장면을
// 바꾸고(update 가 true 를 돌려준다) 다시 밝아진다(In). 도는 동안은 게임이 입력/이동을 멈춰야
// 해서 active() 로 알려준다.
const FADE_SECS: f32 = 0.6; // 어두워지는 데/밝아지는 데 각각 걸리는 시간

#[derive(Clone, Copy, PartialEq)]
enum FadePhase {
    Idle,
    Out,
    In,
}

pub struct Fade {
    phase: FadePhase,
    t: f32, // 0 = 완전히 밝음 ~ 1 = 완전히 검음
}

impl Fade {
    pub fn new() -> Fade {
        Fade { phase: FadePhase::Idle, t: 0.0 }
    }

    pub fn active(&self) -> bool {
        self.phase != FadePhase::Idle
    }

    // 어두워지기 시작한다(이미 도는 중이면 무시).
    pub fn start(&mut self) {
        if self.phase == FadePhase::Idle {
            self.phase = FadePhase::Out;
            self.t = 0.0;
        }
    }

    // 매 프레임 부른다 — 화면이 완전히 검어진 바로 그 프레임에만 true(이때 장면을 바꾼다).
    pub fn update(&mut self, dt: f32) -> bool {
        match self.phase {
            FadePhase::Idle => false,
            FadePhase::Out => {
                self.t += dt / FADE_SECS;
                if self.t >= 1.0 {
                    self.t = 1.0;
                    self.phase = FadePhase::In;
                    return true;
                }
                false
            }
            FadePhase::In => {
                self.t -= dt / FADE_SECS;
                if self.t <= 0.0 {
                    self.t = 0.0;
                    self.phase = FadePhase::Idle;
                }
                false
            }
        }
    }

    // 게임 화면 위에 검은 막을 덮는다.
    pub fn draw(&self, r: &mut Renderer, view: &View) {
        if self.t > 0.0 {
            r.rect(view.rect.x, view.rect.y, view.rect.w, view.rect.h, [0.0, 0.0, 0.0, self.t]);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // 동시에 열린 창은 서로 다른 타깃 칸을 받고(화면이 섞이지 않는다), 닫힌 창의 칸은 재사용된다.
    #[test]
    fn slots_are_distinct_while_open_and_reused_after_drop() {
        let a = RenderSlot::acquire();
        let b = RenderSlot::acquire();
        assert_ne!(a.0, b.0);
        let a_index = a.0;
        drop(a);
        let c = RenderSlot::acquire();
        assert_eq!(c.0, a_index);
        assert_ne!(c.0, b.0);
    }

    // 페이드는 어두워진 한 프레임에서만 true 를 주고, 끝나면 다시 멈춘다.
    #[test]
    fn fade_switches_exactly_once_at_black() {
        let mut f = Fade::new();
        assert!(!f.active() && !f.update(0.1));
        f.start();
        let mut switched = 0;
        for _ in 0..100 {
            if f.update(0.05) {
                switched += 1;
                assert_eq!(f.t, 1.0);
            }
        }
        assert_eq!(switched, 1);
        assert!(!f.active());
    }
}
