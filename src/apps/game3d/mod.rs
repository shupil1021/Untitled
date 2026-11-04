//! 창 안에서 도는 1인칭 3D 게임들(apps/doors_game, apps/maze_game)이 같이 쓰는 도구 —
//! 게임 화면 배치/렌더(View), 마우스 시점 모드(MouseLook), 플레이어(player.rs),
//! 타자기 대화창(dialogue.rs). 3D 장면은 `mesh3d.rs` 로 640x480 오프스크린에 그린 뒤 그
//! 결과를 창 안에 4:3 으로 끼워 넣는다(남는 곳은 검은 띠). CRT 효과는 바깥 OS 화면
//! 전체에 이미 걸려 있어서 따로 안 입힌다. HUD 좌표는 전부 VIEW_W x VIEW_H 기준이고
//! 그릴 때 View 가 배율을 곱해준다.

pub mod boxes;
pub mod dialogue;
pub mod ena;
pub mod i18n;
pub mod item;
pub mod keys;
pub mod player;
pub mod sprite;

use std::cell::RefCell;

use miniquad::{KeyCode, RenderingBackend, TextureId};

use crate::render::gfx::{Rect, Renderer};
use crate::render::mesh3d::{v_dot, v_len, v_sub, Box3D, Camera, Mesh3D, MeshObject};

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
    // 확대 정도(0 = 기본 시야) — 멀리 있는 대상과 상호작용할 때 카메라가 살짝 당겨지는 연출(Player::zoom). 시야각(FOV)을 1/(1+zoom) 로
    // 좁히는 것이라 안내 문구/표식/스프라이트 투영도 같은 값을 써야 화면과 맞는다.
    pub zoom: f32,
}

impl View {
    pub fn fit(area: Rect) -> View {
        let s = (area.w / VIEW_W).min(area.h / VIEW_H);
        let (w, h) = (VIEW_W * s, VIEW_H * s);
        View { rect: Rect::new(area.x + (area.w - w) / 2.0, area.y + (area.h - h) / 2.0, w, h), s, zoom: 0.0 }
    }

    // 지금 세로 시야각 — 확대(zoom)할수록 좁아진다.
    pub fn fov(&self) -> f32 {
        fov_for_zoom(self.zoom)
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
        let tex = render_scene(ctx, slot, clear, cam, boxes, self.fov());
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        // 오프스크린 텍스처는 위아래가 뒤집혀 있어 v 를 뒤집어 붙인다(crt.rs 와 같은 이유).
        r.sprite_uv(tex, self.rect.x, self.rect.y, self.rect.w, self.rect.h, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);
        let s = self.s;
        r.rect(self.px(VIEW_W / 2.0 - 1.5), self.py(VIEW_H / 2.0 - 1.5), 3.0 * s, 3.0 * s, [1.0, 1.0, 1.0, 0.7]);
    }

    // 가운데 조준선 없이(레이싱처럼 조준이 없는 화면) 상자와 임의 메시(MeshObject)를 같이 그린다.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_scene_meshes(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D], meshes: &[MeshObject]) {
        let tex = render_scene_meshes(ctx, slot, clear, cam, boxes, meshes, self.fov());
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        r.sprite_uv(tex, self.rect.x, self.rect.y, self.rect.w, self.rect.h, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);
    }

    // draw_scene_meshes 와 같지만 가운데 조준선도 그린다(조준해서 조사하는 화면용).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_scene_meshes_aimed(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D], meshes: &[MeshObject]) {
        self.draw_scene_meshes(ctx, r, area, slot, clear, cam, boxes, meshes);
        let s = self.s;
        r.rect(self.px(VIEW_W / 2.0 - 1.5), self.py(VIEW_H / 2.0 - 1.5), 3.0 * s, 3.0 * s, [1.0, 1.0, 1.0, 0.7]);
    }

    // 월드 위치 옆에 "[E] ..." 같은 상호작용 안내를 띄운다(카메라 뒤면 안 그림).
    // 화면 전체를 덮는 엔딩 카드 — alpha(0~1)만큼 어둡게 덮고 가운데에 제목/부제를, 다 나타나면 아래에 닫는 방법을 쓴다.
    pub fn draw_end_card(&self, r: &mut Renderer, alpha: f32, title: &str, sub: &str, hint: &str, time: f32) {
        let s = self.s;
        let a = alpha.clamp(0.0, 1.0);
        r.rect(self.rect.x, self.rect.y, self.rect.w, self.rect.h, [0.0, 0.0, 0.0, 0.88 * a]);
        if a < 0.5 {
            return;
        }
        let text_a = ((a - 0.5) * 2.0).clamp(0.0, 1.0);
        let cx = self.px(VIEW_W / 2.0);
        let w = r.text_width(title, 2.2 * s);
        r.text(cx - w / 2.0, self.py(VIEW_H * 0.34), title, 2.2 * s, [1.0, 1.0, 1.0, text_a]);
        let w = r.text_width(sub, 1.0 * s);
        r.text(cx - w / 2.0, self.py(VIEW_H * 0.5), sub, 1.0 * s, [0.8, 0.85, 1.0, text_a]);
        if a >= 1.0 && (time * 2.0).sin() > -0.2 {
            let w = r.text_width(hint, 0.8 * s);
            r.text(cx - w / 2.0, self.py(VIEW_H * 0.78), hint, 0.8 * s, [0.65, 0.65, 0.7, 1.0]);
        }
    }

    // 조사할 수 있는 물건이 가까이(MARKER_DIST 안) 있으면 그 위에 작은 "!" 표식을 띄운다 — 조준선을 맞추기 전에도 어디에
    // 뭔가 있는지 알 수 있고, 가까이 갈수록 또렷해진다. 지금 조준 중인 물건은 안내 문구가 대신하므로 호출부에서 뺀다.
    pub fn draw_marker(&self, r: &mut Renderer, cam: &Camera, pos: [f32; 3], time: f32) {
        let dist = v_len(v_sub(pos, cam.pos));
        if dist > MARKER_DIST {
            return;
        }
        let Some((sx, sy)) = world_to_view(cam, pos, self.fov()) else { return };
        let near = 1.0 - dist / MARKER_DIST;
        let alpha = (0.3 + 0.5 * near) * (0.85 + 0.15 * (time * 3.0).sin());
        r.text(self.px(sx - 3.0), self.py(sy - 9.0), "!", 0.9 * self.s * text_scale(), [0.6, 0.9, 1.0, alpha]);
    }

    pub fn draw_prompt(&self, r: &mut Renderer, cam: &Camera, pos: [f32; 3], label: &str) {
        let Some((sx, sy)) = world_to_view(cam, pos, self.fov()) else { return };
        let s = self.s;
        let label = keys::localize_key_hint(&i18n::tr(label));
        let k = text_scale();
        let tw = r.text_width(&label, 0.7 * s * k);
        r.rect(self.px(sx + 10.0), self.py(sy - 10.0), tw + 8.0 * s, 16.0 * s * k, [0.0, 0.0, 0.0, 0.6]);
        r.text(self.px(sx + 14.0), self.py(sy - 8.0), &label, 0.7 * s * k, [0.6, 0.9, 1.0, 1.0]);
    }

    // 화면 아래 가운데의 "클릭해서 시점 조작" 안내.
    pub fn draw_capture_hint(&self, r: &mut Renderer) {
        let s = self.s;
        // "Shift" 는 지금 달리기 키 이름으로 바꿔 보여 준다.
        let hint = i18n::tr_static("클릭해서 시점 조작 (Esc: 해제 / Shift: 달리기)").replace("Shift", &crate::foundation::key_label(keys::code(crate::foundation::Action::Sprint)));
        let k = text_scale();
        let tw = r.text_width(&hint, 0.7 * s * k);
        let cx = self.px(VIEW_W / 2.0);
        r.rect(cx - tw / 2.0 - 6.0 * s, self.py(VIEW_H - 30.0 * k), tw + 12.0 * s, 20.0 * s * k, [0.0, 0.0, 0.0, 0.6]);
        r.text(cx - tw / 2.0, self.py(VIEW_H - 27.0 * k), &hint, 0.7 * s * k, [0.9, 0.9, 0.9, 1.0]);
    }
}

// 상자와 함께 임의 삼각형 메시(MeshObject)도 그리는 버전 — 길처럼 상자로는 이음매 없이 만들기 어려운 곡면용.
fn render_scene_meshes(ctx: &mut dyn RenderingBackend, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D], meshes: &[MeshObject], fov: f32) -> TextureId {
    POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        let mesh = pool.targets[slot.0].get_or_insert_with(|| Mesh3D::new(ctx, VIEW_W as u32, VIEW_H as u32));
        mesh.render_ex(ctx, clear, cam, boxes, meshes, fov);
        mesh.color_texture()
    })
}

fn render_scene(ctx: &mut dyn RenderingBackend, slot: &RenderSlot, clear: [f32; 4], cam: &Camera, boxes: &[Box3D], fov: f32) -> TextureId {
    POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        let mesh = pool.targets[slot.0].get_or_insert_with(|| Mesh3D::new(ctx, VIEW_W as u32, VIEW_H as u32));
        mesh.render(ctx, clear, cam, boxes, fov);
        mesh.color_texture()
    })
}

// 월드 좌표 → VIEW_W x VIEW_H 기준 화면 좌표. 카메라 뒤면 None.
// 게임 안 글자(안내/표식/HUD) 크기 배율 — 설정의 "큰 글자". main 이 프레임마다 알려 준다.
static TEXT_SCALE_BITS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x3f80_0000); // 1.0

pub fn set_text_scale(k: f32) {
    TEXT_SCALE_BITS.store(k.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

pub fn text_scale() -> f32 {
    f32::from_bits(TEXT_SCALE_BITS.load(std::sync::atomic::Ordering::Relaxed))
}

// 이 거리(m) 안의 조사 대상에 "!" 표식을 띄운다.
pub const MARKER_DIST: f32 = 4.5;

// 확대(zoom)에 따른 세로 시야각 — zoom 0 이면 기본 시야(FOV_Y).
pub fn fov_for_zoom(zoom: f32) -> f32 {
    FOV_Y / (1.0 + zoom.max(0.0))
}

fn world_to_view(cam: &Camera, p: [f32; 3], fov: f32) -> Option<(f32, f32)> {
    let rel = v_sub(p, cam.pos);
    let depth = v_dot(rel, cam.forward());
    if depth <= 0.05 {
        return None;
    }
    let half_h = (fov / 2.0).tan();
    let half_w = half_h * (VIEW_W / VIEW_H);
    let ndc_x = v_dot(rel, cam.right()) / (depth * half_w);
    let ndc_y = v_dot(rel, cam.up()) / (depth * half_h);
    Some(((ndc_x * 0.5 + 0.5) * VIEW_W, (1.0 - (ndc_y * 0.5 + 0.5)) * VIEW_H))
}

// 월드 위치가 지금 화면 안(약간의 여유 포함)에 있는지 — 카메라 뒤거나 옆으로 벗어나 있으면 false.
// 등 뒤에서 뭔가 나타나게 할 때, 플레이어가 그 순간을 못 보게 하려고 쓴다.
pub fn is_on_screen(cam: &Camera, p: [f32; 3]) -> bool {
    const MARGIN: f32 = 80.0;
    world_to_view(cam, p, FOV_Y).is_some_and(|(x, y)| (-MARGIN..=VIEW_W + MARGIN).contains(&x) && (-MARGIN..=VIEW_H + MARGIN).contains(&y))
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
    t: f32, // 0 = 완전히 밝음 ~ 1 = 완전히 덮음
    color: [f32; 3],
}

impl Fade {
    pub fn new() -> Fade {
        Fade { phase: FadePhase::Idle, t: 0.0, color: [0.0, 0.0, 0.0] }
    }

    pub fn active(&self) -> bool {
        self.phase != FadePhase::Idle
    }

    // 어두워지기 시작한다(이미 도는 중이면 무시).
    pub fn start(&mut self) {
        self.start_colored([0.0, 0.0, 0.0]);
    }

    // 지정한 색으로 덮는 페이드(하얗게 흐려지는 장면 등).
    pub fn start_colored(&mut self, color: [f32; 3]) {
        if self.phase == FadePhase::Idle {
            self.color = color;
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

    // 게임 화면 위에 막을 덮는다.
    pub fn draw(&self, r: &mut Renderer, view: &View) {
        if self.t > 0.0 {
            r.rect(view.rect.x, view.rect.y, view.rect.w, view.rect.h, [self.color[0], self.color[1], self.color[2], self.t]);
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

    // 확대할수록 시야각이 좁아지고, 0 이면 기본 시야와 같다.
    #[test]
    fn zoom_narrows_the_field_of_view() {
        assert_eq!(fov_for_zoom(0.0), FOV_Y);
        assert!(fov_for_zoom(0.3) < FOV_Y && fov_for_zoom(0.3) > fov_for_zoom(0.6));
        assert_eq!(fov_for_zoom(-1.0), FOV_Y, "음수는 기본 시야");
        let mut v = View::fit(Rect::new(0.0, 0.0, VIEW_W, VIEW_H));
        assert_eq!(v.fov(), FOV_Y);
        v.zoom = 0.3;
        assert!((v.fov() - FOV_Y / 1.3).abs() < 1e-6);
    }
}

