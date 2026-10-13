//! 미니게임 화면 그리기 — 작은 그리기 도구(사각형/타원/아몬드 모양 눈), 눈 낙서풍 SHY_EYE, 포장 PACK, 3D 레이스의 HUD,
//! 클리어 뒤 보상 화면의 배경과 아이템 아이콘.

use super::*;

// ---------- 그리기 도구 ----------

pub(super) fn fill(r: &mut Renderer, o: (f32, f32), x: f32, y: f32, w: f32, h: f32, c: Color) {
    r.rect(o.0 + x, o.1 + y, w, h, c);
}

// 가로 줄무늬로 채운 타원.
pub(super) fn ellipse(r: &mut Renderer, o: (f32, f32), cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
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

pub(super) fn centered(r: &mut Renderer, o: (f32, f32), y: f32, text: &str, scale: f32, c: Color) {
    let tw = r.text_width(text, scale);
    r.text(o.0 + (W - tw) / 2.0, o.1 + y, text, scale, c);
}

// 아몬드(렌즈) 모양 — 양 끝이 뾰족한 눈 모양을 가로 줄무늬로 채운다. tilt 는 줄마다 옆으로 밀어 기울인다.
pub(super) fn almond(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), tilt: f32, c: Color) {
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
pub(super) fn draw_eye_tone(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), closed: f32, look: (f32, f32), tone: f32, tilt: f32) {
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

pub(super) fn draw_eye(r: &mut Renderer, o: (f32, f32), shape: (f32, f32, f32, f32), closed: f32, look: (f32, f32)) {
    draw_eye_tone(r, o, shape, closed, look, 1.0, 0.0);
}

// 배경에 가득한 눈들 — 어둡게 깔려서 플레이어 쪽을 쳐다보고, 가끔 깜빡인다. closed 로 한꺼번에 감길 수 있다.
pub(super) fn draw_eye_field(r: &mut Renderer, o: (f32, f32), clock: f32, target: (f32, f32), closed: f32) {
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
    pub(super) fn draw_eye_round(&self, r: &mut Renderer, o: (f32, f32), round: &EyeRound, lang: Language, closed: f32) {
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
    pub(super) fn draw_drive_round(&self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, round: &DriveRound, lang: Language) {
        let Some(slot) = &self.slot else { return };
        let view = View::fit(area);
        let boxes = round.boxes();
        view.draw_scene_meshes(ctx, r, area, slot, [0.78, 0.78, 0.8, 1.0], &round.camera(), &boxes, &round.meshes());
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
        // 아래: 속도
        let kmh = (me.speed.max(0.0) * 4.0) as i32;
        r.text(view.px(14.0), view.py(440.0), &format!("{kmh} km/h"), 1.4 * s, white);
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

    pub(super) fn pack_button_rect(&self, o: (f32, f32)) -> Rect {
        Rect::new(o.0 + W / 2.0 - 60.0, o.1 + 296.0, 120.0, 46.0)
    }

    pub(super) fn draw_pack_round(&self, r: &mut Renderer, o: (f32, f32), round: &PackRound, lang: Language, win: &WinInput) {
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
    pub(super) fn draw_reward_backdrop(&self, r: &mut Renderer, o: (f32, f32)) {
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

    pub(super) fn draw_item_icon(&self, r: &mut Renderer, x: f32, y: f32, size: f32) {
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

impl MiniGameApp {
    // 시작 화면의 배경 — 게임마다 분위기만 맞춘 단색.
    pub(super) fn draw_reward_backdrop_for_intro(&self, r: &mut Renderer, o: (f32, f32)) {
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
