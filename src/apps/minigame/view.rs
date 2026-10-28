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

    // 3차선 아케이드 레이싱 — 뒤에서 본 도로(원근을 줄무늬로 흉내), 앞의 차와 상자, 내 차, 위쪽 HUD.
    pub(super) fn draw_lane_round(&self, r: &mut Renderer, o: (f32, f32), round: &LaneRound, lang: Language) {
        const HORIZON: f32 = 120.0;
        const BOTTOM: f32 = 316.0;
        const LANE_W: f32 = 96.0; // 맨 아래(내 차 위치)에서 한 차선의 너비
        let cx = W / 2.0;
        let span = BOTTOM - HORIZON;
        // 하늘과 멀리 보이는 도시
        fill(r, o, 0.0, 0.0, W, HORIZON, [0.08, 0.12, 0.16, 1.0]);
        for i in 0..14 {
            let h = 18.0 + ((i * 37) % 5) as f32 * 9.0;
            fill(r, o, i as f32 * 36.0 - 6.0, HORIZON - h, 30.0, h, [0.14, 0.2, 0.26, 1.0]);
        }
        // 도로 — 가로 띠마다 먼 곳일수록 좁고, 달린 거리에 따라 색과 점선이 흘러간다.
        let mut y = HORIZON;
        while y < BOTTOM + 44.0 {
            let scale = ((y - HORIZON) / span).max(0.02);
            let dz = 8.0 * (1.0 / scale - 1.0); // 이 줄이 내 앞 몇 m 인지
            let band = (((round.dist + dz) / 9.0).floor() as i32) & 1;
            let grass = if band == 0 { [0.12, 0.22, 0.14, 1.0] } else { [0.1, 0.19, 0.12, 1.0] };
            fill(r, o, 0.0, y, W, 3.0, grass);
            let half = LANE_W * 1.5 * scale;
            let road = if band == 0 { [0.26, 0.26, 0.29, 1.0] } else { [0.23, 0.23, 0.26, 1.0] };
            fill(r, o, cx - half, y, half * 2.0, 3.0, road);
            fill(r, o, cx - half - 3.0 * scale, y, 3.0 * scale + 1.0, 3.0, [0.8, 0.8, 0.85, 1.0]);
            fill(r, o, cx + half, y, 3.0 * scale + 1.0, 3.0, [0.8, 0.8, 0.85, 1.0]);
            if (((round.dist + dz) / 6.0).floor() as i32) & 1 == 0 {
                for k in [-0.5f32, 0.5] {
                    fill(r, o, cx + k * LANE_W * 2.0 * scale / 2.0 * 2.0 - scale, y, (2.0 * scale).max(1.0), 3.0, [0.9, 0.9, 0.6, 1.0]);
                }
            }
            y += 3.0;
        }
        // 장애물 — 먼 것부터 그린다.
        let mut seen: Vec<&lane::Obstacle> = round.obstacles.iter().filter(|ob| ob.alive && ob.at > round.dist - 3.0 && ob.at - round.dist < 120.0).collect();
        seen.sort_by(|a, b| b.at.total_cmp(&a.at));
        for ob in seen {
            let dz = (ob.at - round.dist).max(0.0);
            let scale = 1.0 / (1.0 + dz / 8.0);
            let x = cx + (ob.lane as f32 - 1.0) * LANE_W * scale;
            let ybase = HORIZON + span * scale;
            match ob.kind {
                lane::ObstacleKind::Car => {
                    let (w, h) = (66.0 * scale, 38.0 * scale);
                    fill(r, o, x - w / 2.0, ybase - h, w, h, [0.2, 0.45, 0.85, 1.0]);
                    fill(r, o, x - w * 0.36, ybase - h * 1.0, w * 0.72, h * 0.4, [0.55, 0.75, 0.95, 1.0]);
                    fill(r, o, x - w * 0.42, ybase - h * 0.2, w * 0.2, h * 0.2, [0.95, 0.2, 0.2, 1.0]);
                    fill(r, o, x + w * 0.22, ybase - h * 0.2, w * 0.2, h * 0.2, [0.95, 0.2, 0.2, 1.0]);
                }
                lane::ObstacleKind::Crate => {
                    let (w, h) = (52.0 * scale, 46.0 * scale);
                    fill(r, o, x - w / 2.0, ybase - h, w, h, [0.68, 0.5, 0.3, 1.0]);
                    fill(r, o, x - w / 2.0, ybase - h * 0.55, w, (4.0 * scale).max(1.0), [0.5, 0.35, 0.18, 1.0]);
                    fill(r, o, x - (3.0 * scale).max(1.0), ybase - h, (6.0 * scale).max(1.0), h, [0.5, 0.35, 0.18, 1.0]);
                }
            }
        }
        // 내 차 — 충돌해서 멈춰 있는 동안은 깜빡인다.
        let px = cx + (round.lane_pos - 1.0) * LANE_W;
        if round.stop_left <= 0.0 || (self.clock * 12.0) as i32 % 2 == 0 {
            fill(r, o, px - 33.0, BOTTOM - 38.0 + 22.0, 66.0, 38.0, [0.85, 0.15, 0.18, 1.0]);
            fill(r, o, px - 24.0, BOTTOM - 38.0 + 22.0, 48.0, 14.0, [0.95, 0.8, 0.8, 1.0]);
            fill(r, o, px - 30.0, BOTTOM - 8.0 + 22.0, 14.0, 8.0, [0.08, 0.08, 0.1, 1.0]);
            fill(r, o, px + 16.0, BOTTOM - 8.0 + 22.0, 14.0, 8.0, [0.08, 0.08, 0.1, 1.0]);
            if round.braking && round.stop_left <= 0.0 {
                fill(r, o, px - 30.0, BOTTOM - 20.0 + 22.0, 10.0, 5.0, [1.0, 0.3, 0.3, 1.0]);
                fill(r, o, px + 20.0, BOTTOM - 20.0 + 22.0, 10.0, 5.0, [1.0, 0.3, 0.3, 1.0]);
            }
        }
        // HUD
        fill(r, o, 0.0, 0.0, W, HUD_H, [0.0, 0.0, 0.0, 0.85]);
        let status = format!(
            "{} {:.0}/{:.0}m   {} {:.0}   {} {}/{}",
            t(lang, s::DISTANCE),
            round.dist.min(lane::GOAL_M),
            lane::GOAL_M,
            t(lang, s::TIME),
            round.time_left(),
            t(lang, s::CRASHES),
            round.crashes,
            lane::MAX_CRASHES
        );
        r.text(o.0 + 8.0, o.1 + 3.0, &status, 0.75, [0.9, 0.9, 0.9, 1.0]);
        // 카운트다운 / GO!
        let cd = round.countdown();
        if cd > 0.0 {
            let n = format!("{}", cd.ceil() as i32);
            r.text(o.0 + W / 2.0 - r.text_width(&n, 4.0) / 2.0, o.1 + 120.0, &n, 4.0, [1.0, 0.95, 0.3, 1.0]);
        } else if cd > -0.9 {
            r.text(o.0 + W / 2.0 - r.text_width("GO!", 4.0) / 2.0, o.1 + 120.0, "GO!", 4.0, [0.4, 1.0, 0.4, 1.0]);
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
        // 판정 구간 — 판정선 앞뒤 ±0.25초(소리를 꺼도 이 밝은 띠를 보고 맞출 수 있다)
        fill(r, o, LINE_X - pack::WINDOW * speed, BELT_Y - 60.0, pack::WINDOW * speed * 2.0, 140.0, [1.0, 0.95, 0.5, 0.12]);
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
                // 금고 절반 — 쇠색 몸통, 다이얼, 잘려서 어두운 단면.
                r.rect(x + size * 0.1, y + size * 0.12, size * 0.8, size * 0.76, [0.5, 0.52, 0.58, 1.0]);
                r.rect(x + size * 0.76, y + size * 0.12, size * 0.14, size * 0.76, [0.12, 0.13, 0.16, 1.0]);
                ellipse(r, o, size * 0.4, size * 0.5, size * 0.2, size * 0.2, [0.7, 0.72, 0.78, 1.0]);
                ellipse(r, o, size * 0.4, size * 0.5, size * 0.06, size * 0.06, [0.85, 0.72, 0.3, 1.0]);
            }
            MiniKind::Finger => {
                // 변기 손잡이 — 은색 막대 + 둥근 끝.
                r.rect(x + size * 0.2, y + size * 0.52, size * 0.62, size * 0.12, [0.82, 0.84, 0.9, 1.0]);
                ellipse(r, o, size * 0.82, size * 0.58, size * 0.14, size * 0.14, [0.92, 0.92, 0.97, 1.0]);
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
