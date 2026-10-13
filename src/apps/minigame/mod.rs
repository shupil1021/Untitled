//! 2D 미니게임 셋 — 2층 미션 방(눈/뇌/손가락)에서 "게임 받기"를 누르면 CRT 로 설치 파일이 메일로 오고,
//! 설치하면 바탕화면에 생기는 `SHY_EYE.exe` / `BRAIN_DRIVE.exe` / `PACK.exe`(FileKind::MiniGame).
//! 셋이 같은 틀을 쓴다: 안내(START) → 한 판(Round) → 실패하면 RETRY / 성공하면 CLEAR → 보상 상자에서 재료를
//! 꺼내 → 우체통에 넣으면 보상 메일(사진 첨부)이 CRT 로 간다. 판 하나는 도중에 창을 닫으면 처음부터
//! 다시지만, 클리어한 뒤의 보상 상태(cleared/reward_taken/reward_sent)는 저장돼서 다시 안 해도 이어진다.
//! 창이 포커스가 아닐 땐 시간이 멈춘다(한 번에 한 게임만 진행한다).

mod drive;
mod eye;
mod pack;

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{Language, MiniKind, MiniProgress, Settings};
use crate::random::Rng;
use crate::render::gfx::{Assets, Color, Rect, Renderer};
use crate::strings::{minigame as s, t};
use crate::ui::button;

use super::game3d::{RenderSlot, View};
use super::{App, AppAction, WinInput};
use drive::DriveRound;
use eye::EyeRound;
use pack::{PackRound, Phase};

// 창 크기(테두리 포함) — 클라이언트 영역이 480x360 이 되게 한다(메인 게임과 같은 계산).
pub const WIN_W: f32 = 486.0;
pub const WIN_H: f32 = 386.0;
const W: f32 = 480.0;
const H: f32 = 360.0;
const HUD_H: f32 = 24.0;

// 한 프레임의 입력 — 게임 로직(eye/drive/pack)이 키보드와 직접 얽히지 않게 한다.
#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub dx: f32,
    pub dy: f32,
    pub brake: bool,
    pub hold: bool, // Shift 또는 Space 를 누르고 있다(드리프트)
    pub act: bool, // 눌린 순간 한 번(길게 눌러도 1회)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Running,
    Won,
    Lost,
}

pub fn new_app(kind: MiniKind, progress: MiniProgress, settings: Rc<RefCell<Settings>>) -> Box<dyn App> {
    Box::new(MiniGameApp::new(kind, progress, settings))
}

enum Round {
    Eye(EyeRound),
    Drive(DriveRound),
    Pack(PackRound),
}

impl Round {
    fn new(kind: MiniKind) -> Round {
        let rng = Rng::from_time();
        match kind {
            MiniKind::Eye => Round::Eye(EyeRound::new(rng)),
            MiniKind::Brain => Round::Drive(DriveRound::new(rng)),
            MiniKind::Finger => Round::Pack(PackRound::new()),
        }
    }

    fn step(&mut self, dt: f32, c: &Controls) -> Outcome {
        match self {
            Round::Eye(r) => r.step(dt, c),
            Round::Drive(r) => r.step(dt, c),
            Round::Pack(r) => r.step(dt, c),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Intro,
    Playing,
    Failed,
    Cleared,
    Reward,
    Sent,
}

pub struct MiniGameApp {
    kind: MiniKind,
    settings: Rc<RefCell<Settings>>,
    progress: MiniProgress,
    saved: MiniProgress,
    stage: Stage,
    round: Round,
    clock: f32,
    stage_t: f32,
    hint_shown: bool, // 첫 실패 뒤 한 번만 도움말을 보여준다
    show_hint: bool,
    pending_mail: bool,
    slot: Option<RenderSlot>, // 3D 레이싱(BRAIN_DRIVE)만 쓰는 이 창 전용 3D 렌더 타깃
}

impl MiniGameApp {
    fn new(kind: MiniKind, progress: MiniProgress, settings: Rc<RefCell<Settings>>) -> MiniGameApp {
        let stage = if progress.reward_sent {
            Stage::Sent
        } else if progress.cleared {
            Stage::Reward
        } else {
            Stage::Intro
        };
        MiniGameApp { kind, settings, saved: progress, progress, stage, round: Round::new(kind), clock: 0.0, stage_t: 0.0, hint_shown: false, show_hint: false, pending_mail: false, slot: (kind == MiniKind::Brain).then(RenderSlot::acquire) }
    }

    fn controls(&self, win: &WinInput, pack_button: Rect) -> Controls {
        // 다른 창이 앞에 있을 땐 키 입력이 이 게임에 들어오지 않는다(뒤에서 누른 키가 판정되던 문제).
        if !win.focused {
            return Controls::default();
        }
        let i = win.input;
        let (left, right) = (i.is_down(KeyCode::A) || i.is_down(KeyCode::Left), i.is_down(KeyCode::D) || i.is_down(KeyCode::Right));
        let (up, down) = (i.is_down(KeyCode::W) || i.is_down(KeyCode::Up), i.is_down(KeyCode::S) || i.is_down(KeyCode::Down));
        let axis = |neg: bool, pos: bool| (pos as i32 - neg as i32) as f32;
        let clicked_button = win.mouse_clicked && pack_button.contains(win.mouse.0, win.mouse.1);
        Controls {
            dx: axis(left, right),
            dy: axis(up, down),
            brake: down,
            hold: i.is_down(KeyCode::LeftShift) || i.is_down(KeyCode::RightShift) || i.is_down(KeyCode::Space),
            act: i.pressed(KeyCode::Space) || clicked_button,
        }
    }

    fn lines(&self) -> [crate::strings::S; 3] {
        match self.kind {
            MiniKind::Eye => s::EYE_INTRO,
            MiniKind::Brain => s::DRIVE_INTRO,
            MiniKind::Finger => s::PACK_INTRO,
        }
    }

    fn hint(&self) -> crate::strings::S {
        match self.kind {
            MiniKind::Eye => s::EYE_HINT,
            MiniKind::Brain => s::DRIVE_HINT,
            MiniKind::Finger => s::PACK_HINT,
        }
    }

    fn item_name(&self) -> crate::strings::S {
        match self.kind {
            MiniKind::Eye => s::ITEM_EYE,
            MiniKind::Brain => s::ITEM_BRAIN,
            MiniKind::Finger => s::ITEM_FINGER,
        }
    }
}

// ---------- 그리기 도구 ----------

fn fill(r: &mut Renderer, o: (f32, f32), x: f32, y: f32, w: f32, h: f32, c: Color) {
    r.rect(o.0 + x, o.1 + y, w, h, c);
}

// 가로 줄무늬로 채운 타원.
fn ellipse(r: &mut Renderer, o: (f32, f32), cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
    let step = (ry / 5.0).clamp(1.0, 2.0); // 작은 도형은 줄무늬를 더 잘게
    let mut dy = -ry;
    while dy < ry {
        let k = 1.0 - ((dy + step / 2.0) / ry).powi(2);
        if k > 0.0 {
            let half = rx * k.sqrt();
            r.rect(o.0 + cx - half, o.1 + cy + dy, half * 2.0, step, c);
        }
        dy += step;
    }
}

fn centered(r: &mut Renderer, o: (f32, f32), y: f32, text: &str, scale: f32, c: Color) {
    let tw = r.text_width(text, scale);
    r.text(o.0 + (W - tw) / 2.0, o.1 + y, text, scale, c);
}

// 아몬드(렌즈) 모양 — 양 끝이 뾰족한 눈 모양을 가로 줄무늬로 채운다. tilt 는 줄마다 옆으로 밀어 기울인다.
fn almond(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), tilt: f32, c: Color) {
    let (cx, cy, rx, ry) = shape;
    let step = (ry / 6.0).clamp(1.0, 2.0);
    let mut dy = -ry;
    while dy < ry {
        let k = 1.0 - ((dy + step / 2.0).abs() / ry).powf(1.7);
        if k > 0.0 {
            let half = rx * k;
            r.rect(o.0 + cx - half + dy * tilt, o.1 + cy + dy, half * 2.0, step, c);
        }
        dy += step;
    }
}

// 검정 바탕에 그린 낙서 같은 눈 — 하얀 아몬드 모양에 윗눈꺼풀 그림자, 큰 검은 홍채와 그 안의 하얀 테두리/동공.
// tone 은 흰색의 밝기(배경 눈은 어둡게), closed 는 0 = 활짝 ~ 1 = 감김, look 은 -1..1 로 어느 쪽을 보는지.
fn draw_eye_tone(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), closed: f32, look: (f32, f32), tone: f32, tilt: f32) {
    let (cx, cy, rx, ry) = shape; // 중심 x, y 와 가로/세로 반지름
    let open = (1.0 - closed).max(0.06);
    let white = [tone, tone, tone, 1.0];
    let ink = [0.0, 0.0, 0.0, 1.0];
    almond(r, o, (cx, cy, rx, ry * open), tilt, white);
    if open > 0.25 {
        let small = ry < 6.0;
        if !small {
            // 윗눈꺼풀 그림자 — 하얀 쪽 위를 덮는 검은 초승달과 눈초리의 짧은 낙서 선.
            almond(r, o, (cx, cy - ry * open * 0.72, rx * 0.86, ry * open * 0.28), tilt, ink);
            for k in 0..3 {
                let t = k as f32 * 0.5;
                let len = (3.0 + rx * 0.06).min(7.0);
                r.rect(o.0 + cx - rx * (0.95 - t * 0.1), o.1 + cy + ry * open * (0.1 + t * 0.15), len, 1.0, ink);
                r.rect(o.0 + cx + rx * (0.88 - t * 0.1), o.1 + cy - ry * open * (0.05 + t * 0.15), len, 1.0, ink);
            }
        }
        // 작은 눈(탄막)은 흰자가 보이게 홍채를 작게 해서 "하얀 눈"으로 읽히게 한다.
        let ir = if small { ry * 0.62 } else { (ry * open * 1.05).min(rx * 0.5) };
        let (ix, iy) = (cx + look.0 * (rx - ir) * 0.55, cy + look.1 * ry * open * 0.15);
        ellipse(r, o, ix, iy, ir, ir, ink);
        if ir > 4.0 {
            ellipse(r, o, ix, iy, ir * 0.74, ir * 0.74, white);
            ellipse(r, o, ix, iy, ir * 0.62, ir * 0.62, ink);
            ellipse(r, o, ix - ir * 0.3, iy - ir * 0.3, ir * 0.16, ir * 0.16, white);
        }
    }
}

fn draw_eye(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), closed: f32, look: (f32, f32)) {
    draw_eye_tone(r, o, shape, closed, look, 1.0, 0.0);
}

// 배경에 가득한 눈들 — 어둡게 깔려서 플레이어 쪽을 쳐다보고, 가끔 깜빡인다. closed 로 한꺼번에 감길 수 있다.
fn draw_eye_field(r: &mut Renderer, o: (f32, f32), clock: f32, target: (f32, f32), closed: f32) {
    let mut seed = 0x2545_F491u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for _ in 0..26 {
        let (x, y) = (14.0 + next() * (W - 28.0), 40.0 + next() * (H - 60.0));
        let rx = 14.0 + next() * 18.0;
        let ry = rx * (0.36 + next() * 0.18);
        let tone = 0.16 + next() * 0.18;
        let tilt = (next() - 0.5) * 0.9;
        let phase = next();
        let tick = (clock * 0.28 + phase) % 1.0;
        let blink = if tick < 0.07 { (std::f32::consts::PI * tick / 0.07).sin() } else { 0.0 };
        let look = (((target.0 - x) / 220.0).clamp(-1.0, 1.0), ((target.1 - y) / 150.0).clamp(-1.0, 1.0));
        draw_eye_tone(r, o, (x, y, rx, ry), closed.max(blink), look, tone, tilt);
    }
}

// ---------- 각 게임 그리기 ----------

impl MiniGameApp {
    fn draw_eye_round(&self, r: &mut Renderer, o: (f32, f32), round: &EyeRound, lang: Language, closed: f32) {
        fill(r, o, 0.0, 0.0, W, H, [0.0, 0.0, 0.0, 1.0]);
        let ao = (o.0, o.1 + HUD_H); // 판은 HUD 아래
        let target = (round.px, round.py);
        draw_eye_field(r, ao, self.clock, target, closed);
        // 위쪽의 큰 눈 — 탄막을 쏘는 눈. 플레이어를 따라 보고, 클리어하면 감긴다.
        let look = ((round.px - eye::W / 2.0) / (eye::W / 2.0), 0.8);
        draw_eye(r, ao, (eye::W / 2.0, 20.0, 66.0, 26.0), closed, look);
        // 탄막은 하얀 작은 눈들 — 눈동자가 플레이어를 쳐다본다.
        for b in &round.bullets {
            let look = (((round.px - b.x) / 120.0).clamp(-1.0, 1.0), ((round.py - b.y) / 120.0).clamp(-1.0, 1.0));
            draw_eye_tone(r, ao, (b.x, b.y, 7.5, 4.6), 0.0, look, 1.0, 0.0);
        }
        // 플레이어는 하얀 테두리의 빨간 점(눈과 구분되게).
        let blink = round.invulnerable() && ((self.clock * 14.0) as i32 % 2 == 0);
        if !blink {
            ellipse(r, ao, round.px, round.py, 6.5, 6.5, [1.0, 1.0, 1.0, 1.0]);
            ellipse(r, ao, round.px, round.py, 4.6, 4.6, [0.85, 0.08, 0.1, 1.0]);
        }
        // HUD
        fill(r, o, 0.0, 0.0, W, HUD_H, [0.0, 0.0, 0.0, 1.0]);
        r.rect(o.0, o.1 + HUD_H - 1.0, W, 1.0, [0.35, 0.35, 0.35, 1.0]);
        r.text(o.0 + 8.0, o.1 + 3.0, t(lang, s::LIVES), 0.75, [0.9, 0.9, 0.9, 1.0]);
        for i in 0..eye::LIVES {
            let c = if i < round.lives() { [0.9, 0.12, 0.15, 1.0] } else { [0.28, 0.1, 0.11, 1.0] };
            fill(r, o, 62.0 + i as f32 * 18.0, 7.0, 12.0, 12.0, c);
        }
        let time = format!("{} {:.1} / {:.0}", t(lang, s::TIME), round.time().min(eye::GOAL_SECS), eye::GOAL_SECS);
        r.text(o.0 + W - 8.0 - r.text_width(&time, 0.75), o.1 + 3.0, &time, 0.75, [0.9, 0.9, 0.9, 1.0]);
    }

    // 3D 카트 레이스 — 추적 카메라로 장면을 그리고 그 위에 HUD(랩/순위/속도/부스터/미니맵/카운트다운)를 얹는다.
    fn draw_drive_round(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, round: &DriveRound, lang: Language) {
        let Some(slot) = &self.slot else { return };
        let view = View::fit(area);
        let boxes = round.boxes();
        view.draw_scene_plain(ctx, r, area, slot, [0.55, 0.75, 0.95, 1.0], &round.camera(), &boxes);
        let s = view.s;
        let white = [1.0, 1.0, 1.0, 1.0];
        let me = &round.karts[0];
        // 위: 바퀴 수와 순위
        r.rect(view.px(0.0), view.py(0.0), 640.0 * s, 34.0 * s, [0.0, 0.0, 0.0, 0.45]);
        r.text(view.px(12.0), view.py(6.0), &format!("LAP {}/{}", me.lap_number(), drive::LAPS), 1.2 * s, white);
        let place = round.place();
        let place_color = if place == 1 { [1.0, 0.9, 0.3, 1.0] } else { white };
        let place_text = format!("{} {}/{}", t(lang, s::PLACE), place, drive::RACERS);
        r.text(view.px(628.0) - r.text_width(&place_text, 1.2 * s), view.py(6.0), &place_text, 1.2 * s, place_color);
        // 아래: 속도와 부스터 / 드리프트 충전
        let kmh = (me.speed.max(0.0) * 4.0) as i32;
        r.text(view.px(14.0), view.py(440.0), &format!("{kmh} km/h"), 1.4 * s, white);
        let bar = |r: &mut Renderer, y: f32, ratio: f32, color: [f32; 4]| {
            r.rect(view.px(250.0), view.py(y), 140.0 * s, 10.0 * s, [0.0, 0.0, 0.0, 0.55]);
            r.rect(view.px(250.0), view.py(y), 140.0 * s * ratio, 10.0 * s, color);
        };
        bar(r, 420.0, round.drift_charge(), [0.3, 0.8, 1.0, 1.0]);
        bar(r, 436.0, round.boost_ratio(), [1.0, 0.65, 0.15, 1.0]);
        // 미니맵 — 코스 점들과 카트 점들.
        let (ox, oy, size) = (560.0, 44.0, 76.0);
        r.rect(view.px(ox - 4.0), view.py(oy - 4.0), (size + 8.0) * s, (size + 8.0) * s, [0.0, 0.0, 0.0, 0.5]);
        let scale = size / 260.0;
        let to_map = |p: [f32; 2]| (view.px(ox + (p[0] + 90.0) * scale), view.py(oy + (p[1] + 40.0) * scale));
        for (ri, route) in round.track.routes.iter().enumerate() {
            let color = if ri == 0 { [0.85, 0.85, 0.9, 1.0] } else { [0.5, 0.75, 1.0, 1.0] };
            for p in route.pts.iter().step_by(2) {
                let (x, y) = to_map(*p);
                r.rect(x, y, 2.0 * s, 2.0 * s, color);
            }
        }
        const DOT: [[f32; 4]; drive::RACERS] = [[0.9, 0.12, 0.15, 1.0], [0.2, 0.45, 0.95, 1.0], [0.95, 0.8, 0.15, 1.0], [0.25, 0.8, 0.35, 1.0]];
        for (i, k) in round.karts.iter().enumerate().rev() {
            let (x, y) = to_map(k.pos);
            let d = if i == 0 { 5.0 } else { 3.5 };
            r.rect(x - d * s / 2.0, y - d * s / 2.0, d * s, d * s, DOT[i]);
        }
        // 카운트다운 / GO!
        let cd = round.countdown();
        if cd > 0.0 {
            let n = format!("{}", cd.ceil() as i32);
            r.text(view.px(320.0) - r.text_width(&n, 4.0 * s) / 2.0, view.py(150.0), &n, 4.0 * s, [1.0, 0.95, 0.3, 1.0]);
        } else if cd > -0.9 {
            r.text(view.px(320.0) - r.text_width("GO!", 4.0 * s) / 2.0, view.py(150.0), "GO!", 4.0 * s, [0.4, 1.0, 0.4, 1.0]);
        }
    }

    fn pack_button_rect(&self, o: (f32, f32)) -> Rect {
        Rect::new(o.0 + W / 2.0 - 60.0, o.1 + 296.0, 120.0, 46.0)
    }

    fn draw_pack_round(&self, r: &mut Renderer, o: (f32, f32), round: &PackRound, lang: Language, win: &WinInput) {
        const LINE_X: f32 = pack::LINE_X;
        const BELT_Y: f32 = 190.0;
        let speed = pack::SPEED;
        fill(r, o, 0.0, 0.0, W, H, [0.13, 0.13, 0.15, 1.0]);
        // 컨베이어
        fill(r, o, 0.0, BELT_Y, W, 44.0, [0.28, 0.28, 0.3, 1.0]);
        let mut x = -((self.clock * speed) % 24.0);
        while x < W {
            fill(r, o, x, BELT_Y + 40.0, 12.0, 4.0, [0.4, 0.4, 0.42, 1.0]);
            x += 24.0;
        }
        // 판정선
        let line_c = if round.flash > 0.0 { [1.0, 0.9, 0.3, 1.0] } else { [0.9, 0.9, 0.95, 1.0] };
        fill(r, o, LINE_X - pack::LINE_HALF, BELT_Y - 60.0, pack::LINE_HALF * 2.0, 140.0, line_c);
        ellipse(r, o, LINE_X, BELT_Y - 70.0, 7.0, 7.0, line_c);
        for b in &round.crates {
            let bx = LINE_X - (b.arrive - round.t) * speed;
            if !(-60.0..W + 60.0).contains(&bx) {
                continue;
            }
            fill(r, o, bx - pack::BOX_HALF, BELT_Y - 34.0, pack::BOX_HALF * 2.0, 34.0, [0.68, 0.5, 0.3, 1.0]);
            fill(r, o, bx - pack::BOX_HALF, BELT_Y - 22.0, pack::BOX_HALF * 2.0, 4.0, [0.55, 0.38, 0.2, 1.0]);
            match b.result {
                Some(true) => fill(r, o, bx - 8.0, BELT_Y - 56.0, 16.0, 10.0, [0.2, 0.8, 0.3, 1.0]),
                Some(false) => fill(r, o, bx - 8.0, BELT_Y - 56.0, 16.0, 10.0, [0.85, 0.2, 0.2, 1.0]),
                None => {}
            }
        }
        // 포장 버튼(클릭으로도 누를 수 있다)
        let btn = self.pack_button_rect(o);
        let hover = btn.contains(win.mouse.0, win.mouse.1);
        let pressed = round.flash > 0.0 || (hover && win.mouse_down);
        let off = if pressed { 3.0 } else { 0.0 };
        r.rect(btn.x, btn.y + 6.0, btn.w, btn.h - 6.0, [0.4, 0.08, 0.1, 1.0]);
        r.rect(btn.x, btn.y + off, btn.w, btn.h - 6.0, [0.85, 0.18, 0.2, 1.0]);
        let label = t(lang, s::PACK_BUTTON);
        let tw = r.text_width(label, 1.0);
        r.text(btn.x + (btn.w - tw) / 2.0, btn.y + 10.0 + off, label, 1.0, [1.0, 1.0, 1.0, 1.0]);
        // HUD
        fill(r, o, 0.0, 0.0, W, HUD_H, [0.0, 0.0, 0.0, 0.85]);
        let status = match round.phase {
            Phase::Practice => t(lang, s::PRACTICE).to_string(),
            Phase::Ready => t(lang, s::READY).to_string(),
            Phase::Work | Phase::Done => format!("{} {}/{}   OK {}", t(lang, s::BOXES), round.crates.iter().filter(|c| c.result.is_some()).count(), pack::WORK_N, round.hits()),
        };
        r.text(o.0 + 8.0, o.1 + 3.0, &status, 0.75, [0.9, 0.9, 0.9, 1.0]);
    }

    // 클리어한 뒤의 보상 화면 배경 — 게임마다 한 장면이 달라진다(큰 눈이 감김 / 표지판이 방 번호로 / 박수 치는 장갑).
    fn draw_reward_backdrop(&self, r: &mut Renderer, o: (f32, f32)) {
        match self.kind {
            MiniKind::Eye => {
                // 클리어한 뒤 — 모든 눈이 감겼다. 가운데의 큰 눈도 감겨 있다.
                fill(r, o, 0.0, 0.0, W, H, [0.0, 0.0, 0.0, 1.0]);
                draw_eye_field(r, o, self.clock, (W / 2.0, H / 2.0), 1.0);
                draw_eye(r, o, (W / 2.0, 80.0, 96.0, 38.0), 1.0, (0.0, 0.0));
            }
            MiniKind::Brain => {
                fill(r, o, 0.0, 0.0, W, H, [0.07, 0.12, 0.1, 1.0]);
                fill(r, o, 0.0, 250.0, W, 110.0, [0.2, 0.2, 0.22, 1.0]);
                // 급유소 — 간판이 집의 문패로 보인다.
                fill(r, o, 150.0, 40.0, 180.0, 70.0, [0.55, 0.45, 0.3, 1.0]);
                r.text(o.0 + 168.0, o.1 + 62.0, "NO. 401", 1.0, [0.1, 0.1, 0.1, 1.0]);
                fill(r, o, 190.0, 110.0, 8.0, 140.0, [0.4, 0.4, 0.4, 1.0]);
                fill(r, o, 282.0, 110.0, 8.0, 140.0, [0.4, 0.4, 0.4, 1.0]);
            }
            MiniKind::Finger => {
                fill(r, o, 0.0, 0.0, W, H, [0.13, 0.13, 0.15, 1.0]);
                // 합격 화면 — 급여 봉투와 박수 치는 장갑.
                fill(r, o, 190.0, 40.0, 100.0, 64.0, [0.9, 0.85, 0.7, 1.0]);
                r.text(o.0 + 205.0, o.1 + 62.0, "PASS", 1.2, [0.2, 0.5, 0.2, 1.0]);
                let clap = ((self.clock * 5.0) as i32 % 2) as f32 * 8.0;
                fill(r, o, 70.0 + clap, 70.0, 34.0, 44.0, [0.95, 0.95, 0.95, 1.0]);
                fill(r, o, 376.0 - clap, 70.0, 34.0, 44.0, [0.95, 0.95, 0.95, 1.0]);
            }
        }
    }

    fn draw_item_icon(&self, r: &mut Renderer, x: f32, y: f32, size: f32) {
        let o = (x, y);
        match self.kind {
            MiniKind::Eye => draw_eye(r, o, (size / 2.0, size / 2.0, size / 2.0, size / 3.2), 0.0, (0.0, 0.0)),
            MiniKind::Brain => {
                ellipse(r, o, size / 2.0, size / 2.0, size / 2.0, size / 2.6, [0.85, 0.6, 0.65, 1.0]);
                ellipse(r, o, size * 0.62, size * 0.45, size * 0.1, size * 0.1, [0.4, 0.42, 0.25, 1.0]);
            }
            MiniKind::Finger => {
                r.rect(x + size * 0.38, y + size * 0.1, size * 0.24, size * 0.8, [0.93, 0.74, 0.62, 1.0]);
                r.rect(x + size * 0.38, y + size * 0.78, size * 0.24, size * 0.12, [0.6, 0.08, 0.1, 1.0]);
            }
        }
    }
}

impl App for MiniGameApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        let dt = if win.focused { win.dt.min(0.05) } else { 0.0 };
        self.clock += dt;
        self.stage_t += dt;
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);
        let o = (area.x + (area.w - W) / 2.0, area.y + (area.h - H) / 2.0);
        let panel = Rect::new(o.0, o.1, W, H);
        let name = self.kind.app_name();

        match self.stage {
            Stage::Intro => {
                self.draw_reward_backdrop_for_intro(r, o);
                centered(r, o, 40.0, name, 1.8, [1.0, 0.9, 0.7, 1.0]);
                for (i, line) in self.lines().iter().enumerate() {
                    centered(r, o, 120.0 + i as f32 * 30.0, t(lang, *line), 0.9, [0.9, 0.9, 0.95, 1.0]);
                }
                if button(r, o.0 + W / 2.0 - 60.0, o.1 + 260.0, 120.0, 34.0, t(lang, s::START), win) {
                    self.round = Round::new(self.kind);
                    self.stage = Stage::Playing;
                    self.stage_t = 0.0;
                }
            }
            Stage::Playing | Stage::Failed | Stage::Cleared => {
                let playing = self.stage == Stage::Playing;
                let pack_btn = self.pack_button_rect(o);
                if playing {
                    let c = self.controls(win, pack_btn);
                    let out = self.round.step(dt, &c);
                    match out {
                        Outcome::Running => {}
                        Outcome::Won => {
                            self.progress.cleared = true;
                            self.stage = Stage::Cleared;
                            self.stage_t = 0.0;
                        }
                        Outcome::Lost => {
                            self.stage = Stage::Failed;
                            self.stage_t = 0.0;
                            self.show_hint = !self.hint_shown;
                            self.hint_shown = true;
                        }
                    }
                }
                match &self.round {
                    Round::Eye(round) => self.draw_eye_round(r, o, round, lang, if self.stage == Stage::Cleared { (self.stage_t / 1.2).min(1.0) } else { 0.0 }),
                    Round::Drive(round) => self.draw_drive_round(ctx, r, area, round, lang),
                    Round::Pack(round) => self.draw_pack_round(r, o, round, lang, win),
                }
                // PACK 은 연습이 끝나면 START 를 눌러야 본 작업이 시작한다.
                if let Round::Pack(round) = &self.round
                    && round.phase == Phase::Ready
                    && self.stage == Stage::Playing
                    && button(r, o.0 + W / 2.0 - 60.0, o.1 + 110.0, 120.0, 34.0, t(lang, s::START), win)
                    && let Round::Pack(round) = &mut self.round
                {
                    round.step(0.0, &Controls { act: true, ..Controls::default() });
                }
                if self.stage == Stage::Failed {
                    fill(r, o, 0.0, 90.0, W, 170.0, [0.0, 0.0, 0.0, 0.8]);
                    centered(r, o, 100.0, t(lang, s::FAILED), 1.6, [1.0, 0.4, 0.4, 1.0]);
                    if let Round::Drive(round) = &self.round {
                        centered(r, o, 128.0, &format!("{} {}/{}", t(lang, s::PLACE), round.place(), drive::RACERS), 0.9, [1.0, 1.0, 1.0, 1.0]);
                    }
                    if self.show_hint {
                        let hint = t(lang, self.hint());
                        for (i, line) in crate::ui::wrap_lines(r, hint, 0.8, W - 60.0).iter().enumerate() {
                            r.text(o.0 + 30.0, o.1 + 150.0 + i as f32 * 20.0, line, 0.8, [0.9, 0.9, 0.7, 1.0]);
                        }
                    }
                    if button(r, o.0 + W / 2.0 - 60.0, o.1 + 215.0, 120.0, 34.0, t(lang, s::RETRY), win) {
                        self.round = Round::new(self.kind);
                        self.stage = Stage::Playing;
                        self.stage_t = 0.0;
                    }
                }
                if self.stage == Stage::Cleared {
                    fill(r, o, 0.0, 120.0, W, 90.0, [0.0, 0.0, 0.0, 0.75]);
                    centered(r, o, 140.0, t(lang, s::CLEAR), 2.0, [1.0, 0.95, 0.4, 1.0]);
                    if self.stage_t > 2.2 || (self.stage_t > 0.6 && win.mouse_clicked && panel.contains(win.mouse.0, win.mouse.1)) {
                        self.stage = Stage::Reward;
                        self.stage_t = 0.0;
                    }
                }
            }
            Stage::Reward | Stage::Sent => {
                self.draw_reward_backdrop(r, o);
                let chest = Rect::new(o.0 + 80.0, o.1 + 220.0, 90.0, 64.0);
                let mailbox = Rect::new(o.0 + 330.0, o.1 + 180.0, 60.0, 104.0);
                // 상자 — 꺼내기 전엔 닫혀 있고, 꺼낸 뒤엔 뚜껑이 열린 빈 상자.
                let open = self.progress.reward_taken;
                r.rect(chest.x, chest.y + 18.0, chest.w, chest.h - 18.0, [0.55, 0.36, 0.18, 1.0]);
                r.rect(chest.x, chest.y + if open { 0.0 } else { 8.0 }, chest.w, 14.0, [0.7, 0.48, 0.24, 1.0]);
                r.rect(chest.x + chest.w / 2.0 - 5.0, chest.y + 24.0, 10.0, 10.0, [0.9, 0.8, 0.3, 1.0]);
                // 우체통
                r.rect(mailbox.x, mailbox.y, mailbox.w, mailbox.h, [0.25, 0.3, 0.6, 1.0]);
                r.rect(mailbox.x + 8.0, mailbox.y + 14.0, mailbox.w - 16.0, 8.0, [0.05, 0.05, 0.1, 1.0]);
                let hover_c = chest.contains(win.mouse.0, win.mouse.1);
                let hover_m = mailbox.contains(win.mouse.0, win.mouse.1);
                let can_take = self.stage == Stage::Reward && !self.progress.reward_taken;
                let can_send = self.stage == Stage::Reward && self.progress.reward_taken && !self.progress.reward_sent;
                if (can_take && hover_c) || (can_send && hover_m) {
                    let target = if hover_c { chest } else { mailbox };
                    crate::ui::border(r, target.x - 2.0, target.y - 2.0, target.w + 4.0, target.h + 4.0, [1.0, 1.0, 0.5, 1.0]);
                }
                if win.focused && win.mouse_clicked {
                    if can_take && hover_c {
                        self.progress.reward_taken = true;
                    } else if can_send && hover_m {
                        self.progress.reward_sent = true;
                        self.pending_mail = true;
                        self.stage = Stage::Sent;
                    }
                }
                // 가진 것 — 상자에서 꺼낸 뒤 우체통에 넣기 전까지.
                if self.progress.reward_taken && !self.progress.reward_sent {
                    fill(r, o, 14.0, H - 54.0, 44.0, 44.0, [0.0, 0.0, 0.0, 0.7]);
                    self.draw_item_icon(r, o.0 + 17.0, o.1 + H - 51.0, 38.0);
                    r.text(o.0 + 66.0, o.1 + H - 46.0, &format!("{} {}", t(lang, s::GOT_ITEM), t(lang, self.item_name())), 0.8, [1.0, 1.0, 0.8, 1.0]);
                }
                let msg = if self.stage == Stage::Sent {
                    t(lang, s::REWARD_SENT)
                } else if !self.progress.reward_taken {
                    t(lang, s::REWARD_OPEN)
                } else {
                    t(lang, s::REWARD_MAIL)
                };
                centered(r, o, 20.0, msg, 0.9, [1.0, 1.0, 1.0, 1.0]);
            }
        }

        // 보낸 직후 한 번: 보상 메일을 CRT 로. 그 다음 프레임에 진행 저장.
        if self.pending_mail {
            self.pending_mail = false;
            return AppAction::DeliverMail(self.kind.reward_mail());
        }
        if self.progress != self.saved {
            self.saved = self.progress;
            return AppAction::SaveMini(self.kind, self.progress);
        }
        AppAction::None
    }
}

impl MiniGameApp {
    // 시작 화면의 배경 — 게임마다 분위기만 맞춘 단색.
    fn draw_reward_backdrop_for_intro(&self, r: &mut Renderer, o: (f32, f32)) {
        if self.kind == MiniKind::Eye {
            // 시작 화면 — 어둠 속의 눈들이 이쪽을 쳐다본다.
            fill(r, o, 0.0, 0.0, W, H, [0.0, 0.0, 0.0, 1.0]);
            draw_eye_field(r, o, self.clock, (W / 2.0, H / 2.0), 0.0);
            return;
        }
        let c = match self.kind {
            MiniKind::Eye => [0.0, 0.0, 0.0, 1.0],
            MiniKind::Brain => [0.06, 0.1, 0.08, 1.0],
            MiniKind::Finger => [0.13, 0.13, 0.15, 1.0],
        };
        fill(r, o, 0.0, 0.0, W, H, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(kind: MiniKind, progress: MiniProgress) -> MiniGameApp {
        MiniGameApp::new(kind, progress, Rc::new(RefCell::new(Settings::default())))
    }

    // 저장된 진행으로 다시 열면 이미 한 판을 다시 하지 않고 보상 단계에서 이어진다.
    #[test]
    fn reopening_resumes_at_the_reward() {
        assert!(app(MiniKind::Eye, MiniProgress::default()).stage == Stage::Intro);
        assert!(app(MiniKind::Brain, MiniProgress { cleared: true, ..MiniProgress::default() }).stage == Stage::Reward);
        let sent = MiniProgress { cleared: true, reward_taken: true, reward_sent: true };
        assert!(app(MiniKind::Finger, sent).stage == Stage::Sent);
    }

    #[test]
    fn each_game_has_its_own_mails_and_files() {
        for kind in MiniKind::ALL {
            assert_ne!(kind.task_mail(), kind.reward_mail());
            assert!(kind.setup_name().starts_with(kind.app_name()) && kind.exe_name().starts_with(kind.app_name()));
        }
    }
}
