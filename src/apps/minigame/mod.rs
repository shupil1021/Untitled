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
    pub lane_left: bool,
    pub lane_right: bool,
    pub brake: bool,
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
        MiniGameApp { kind, settings, saved: progress, progress, stage, round: Round::new(kind), clock: 0.0, stage_t: 0.0, hint_shown: false, show_hint: false, pending_mail: false }
    }

    fn controls(&self, win: &WinInput, pack_button: Rect) -> Controls {
        let i = win.input;
        let (left, right) = (i.is_down(KeyCode::A) || i.is_down(KeyCode::Left), i.is_down(KeyCode::D) || i.is_down(KeyCode::Right));
        let (up, down) = (i.is_down(KeyCode::W) || i.is_down(KeyCode::Up), i.is_down(KeyCode::S) || i.is_down(KeyCode::Down));
        let axis = |neg: bool, pos: bool| (pos as i32 - neg as i32) as f32;
        let clicked_button = win.mouse_clicked && pack_button.contains(win.mouse.0, win.mouse.1);
        Controls {
            dx: axis(left, right),
            dy: axis(up, down),
            lane_left: i.pressed(KeyCode::A) || i.pressed(KeyCode::Left),
            lane_right: i.pressed(KeyCode::D) || i.pressed(KeyCode::Right),
            brake: down,
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
    let step = 2.0;
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

fn draw_eye(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), closed: f32, look: (f32, f32)) {
    let (cx, cy, rx, ry) = shape; // 중심 x, y 와 가로/세로 반지름
    // closed: 0 = 활짝, 1 = 감김.
    let open = (1.0 - closed).max(0.05);
    ellipse(r, o, cx, cy, rx, ry * open, [0.93, 0.85, 0.84, 1.0]);
    if open > 0.3 {
        ellipse(r, o, cx + look.0 * rx * 0.25, cy + look.1 * ry * 0.2, ry * 0.55 * open.sqrt(), ry * 0.55 * open, [0.45, 0.14, 0.1, 1.0]);
        ellipse(r, o, cx + look.0 * rx * 0.25, cy + look.1 * ry * 0.2, ry * 0.25 * open.sqrt(), ry * 0.25 * open, [0.03, 0.0, 0.0, 1.0]);
    }
    // 핏줄
    if open > 0.4 {
        for k in 0..6 {
            let a = k as f32 * 1.05 + 0.3;
            fill(r, o, cx + a.cos() * rx * 0.85 - 1.0, cy + a.sin() * ry * open * 0.8 - 1.0, 3.0, 3.0, [0.7, 0.1, 0.12, 1.0]);
        }
    }
}

// ---------- 각 게임 그리기 ----------

impl MiniGameApp {
    fn draw_eye_round(&self, r: &mut Renderer, o: (f32, f32), round: &EyeRound, lang: Language) {
        fill(r, o, 0.0, 0.0, W, H, [0.05, 0.02, 0.04, 1.0]);
        let ao = (o.0, o.1 + HUD_H); // 판은 HUD 아래
        // 배경의 큰 눈 — 클리어하면 감긴다(재생 중엔 플레이어를 따라 본다).
        let look = ((round.px - eye::W / 2.0) / (eye::W / 2.0), 0.6);
        draw_eye(r, ao, (eye::W / 2.0, 12.0, 52.0, 20.0), 0.0, look);
        for b in &round.bullets {
            ellipse(r, ao, b.x, b.y, 4.0, 4.0, [0.85, 0.1, 0.14, 1.0]);
            ellipse(r, ao, b.x, b.y, 1.6, 1.6, [1.0, 0.85, 0.85, 1.0]);
        }
        let blink = round.invulnerable() && ((self.clock * 14.0) as i32 % 2 == 0);
        if !blink {
            ellipse(r, ao, round.px, round.py, 5.5, 5.5, [0.7, 0.9, 1.0, 1.0]);
            ellipse(r, ao, round.px, round.py, 2.5, 2.5, [0.1, 0.2, 0.4, 1.0]);
        }
        // HUD
        fill(r, o, 0.0, 0.0, W, HUD_H, [0.0, 0.0, 0.0, 0.85]);
        r.text(o.0 + 8.0, o.1 + 3.0, t(lang, s::LIVES), 0.75, [0.9, 0.9, 0.9, 1.0]);
        for i in 0..eye::LIVES {
            let c = if i < round.lives() { [0.9, 0.15, 0.2, 1.0] } else { [0.3, 0.1, 0.12, 1.0] };
            fill(r, o, 62.0 + i as f32 * 18.0, 7.0, 12.0, 12.0, c);
        }
        let time = format!("{} {:.1} / {:.0}", t(lang, s::TIME), round.time().min(eye::GOAL_SECS), eye::GOAL_SECS);
        r.text(o.0 + W - 8.0 - r.text_width(&time, 0.75), o.1 + 3.0, &time, 0.75, [0.9, 0.9, 0.9, 1.0]);
    }

    fn draw_drive_round(&self, r: &mut Renderer, o: (f32, f32), round: &DriveRound, lang: Language, finished: bool) {
        const PX_PER_M: f32 = 7.0;
        const LANE_W: f32 = 96.0;
        const CAR_Y: f32 = 272.0;
        fill(r, o, 0.0, 0.0, W, H, [0.05, 0.12, 0.06, 1.0]);
        let road_x = W / 2.0 - LANE_W * 1.5;
        fill(r, o, road_x, 0.0, LANE_W * 3.0, H, [0.2, 0.2, 0.22, 1.0]);
        fill(r, o, road_x - 4.0, 0.0, 4.0, H, [0.85, 0.85, 0.8, 1.0]);
        fill(r, o, road_x + LANE_W * 3.0, 0.0, 4.0, H, [0.85, 0.85, 0.8, 1.0]);
        // 차선 점선 — 거리만큼 흘러내린다.
        let scroll = (round.dist * PX_PER_M) % 48.0;
        for lane_line in 1..3 {
            let x = road_x + lane_line as f32 * LANE_W - 2.0;
            let mut y = -48.0 + scroll;
            while y < H {
                fill(r, o, x, y, 4.0, 26.0, [0.8, 0.8, 0.5, 1.0]);
                y += 48.0;
            }
        }
        let lane_x = |lane: f32| road_x + LANE_W * (lane + 0.5);
        for ob in &round.obstacles {
            let y = CAR_Y - (ob.s - round.dist) * PX_PER_M;
            if (-60.0..H + 60.0).contains(&y) {
                let x = lane_x(ob.lane as f32);
                let tone = 0.35 + 0.12 * ((ob.s as i32 % 3) as f32);
                fill(r, o, x - 17.0, y - 26.0, 34.0, 52.0, [0.7, tone, 0.2, 1.0]);
                fill(r, o, x - 12.0, y - 16.0, 24.0, 12.0, [0.15, 0.2, 0.3, 1.0]);
            }
        }
        // 내 차
        let cx = lane_x(round.lane_pos);
        let flicker = round.stunned() && ((self.clock * 12.0) as i32 % 2 == 0);
        if !flicker {
            fill(r, o, cx - 17.0, CAR_Y - 26.0, 34.0, 52.0, [0.85, 0.15, 0.2, 1.0]);
            fill(r, o, cx - 12.0, CAR_Y - 16.0, 24.0, 12.0, [0.6, 0.85, 0.95, 1.0]);
            fill(r, o, cx - 15.0, CAR_Y + 18.0, 8.0, 5.0, [1.0, 0.9, 0.4, 1.0]);
            fill(r, o, cx + 7.0, CAR_Y + 18.0, 8.0, 5.0, [1.0, 0.9, 0.4, 1.0]);
        }
        // 도착지 표지판 — 완주하면 목적지가 집이 아니라 방 번호로 바뀐다.
        fill(r, o, 8.0, 34.0, 76.0, 34.0, [0.1, 0.4, 0.2, 1.0]);
        let dest = if finished { "NO. 401" } else { "HOME" };
        r.text(o.0 + 14.0, o.1 + 41.0, dest, 0.8, [0.95, 0.95, 0.95, 1.0]);
        // HUD
        fill(r, o, 0.0, 0.0, W, HUD_H, [0.0, 0.0, 0.0, 0.85]);
        let hud = format!("{} {:.0}/{:.0} m    {} {:.1}/{:.0}    {} {}/{}", t(lang, s::DISTANCE), round.dist.min(drive::GOAL_M), drive::GOAL_M, t(lang, s::TIME), round.time().min(drive::LIMIT_SECS), drive::LIMIT_SECS, t(lang, s::CRASHES), round.crashes(), drive::MAX_CRASHES);
        r.text(o.0 + 8.0, o.1 + 3.0, &hud, 0.75, [0.9, 0.9, 0.9, 1.0]);
    }

    fn pack_button_rect(&self, o: (f32, f32)) -> Rect {
        Rect::new(o.0 + W / 2.0 - 60.0, o.1 + 296.0, 120.0, 46.0)
    }

    fn draw_pack_round(&self, r: &mut Renderer, o: (f32, f32), round: &PackRound, lang: Language, win: &WinInput) {
        const LINE_X: f32 = 320.0;
        const BELT_Y: f32 = 190.0;
        let speed = (LINE_X + 40.0) / pack::TRAVEL;
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
        fill(r, o, LINE_X - 1.0, BELT_Y - 60.0, 3.0, 140.0, line_c);
        ellipse(r, o, LINE_X, BELT_Y - 70.0, 7.0, 7.0, line_c);
        for b in &round.crates {
            let bx = LINE_X - (b.arrive - round.t) * speed;
            if !(-60.0..W + 60.0).contains(&bx) {
                continue;
            }
            fill(r, o, bx - 18.0, BELT_Y - 34.0, 36.0, 34.0, [0.68, 0.5, 0.3, 1.0]);
            fill(r, o, bx - 18.0, BELT_Y - 22.0, 36.0, 4.0, [0.55, 0.38, 0.2, 1.0]);
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
                fill(r, o, 0.0, 0.0, W, H, [0.05, 0.02, 0.04, 1.0]);
                draw_eye(r, o, (W / 2.0, 70.0, 90.0, 38.0), 0.95, (0.0, 0.0));
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

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
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
                    Round::Eye(round) => self.draw_eye_round(r, o, round, lang),
                    Round::Drive(round) => self.draw_drive_round(r, o, round, lang, self.stage == Stage::Cleared),
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
        let c = match self.kind {
            MiniKind::Eye => [0.05, 0.02, 0.04, 1.0],
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
