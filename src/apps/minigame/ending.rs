//! 클리어한 뒤의 보상 단계 — 미니게임마다 엔딩이 다르다(시트의 XA / XB).
//! - 눈: 보상 상자에서 충혈된 눈을 꺼내 우체통에 넣는다(메일로 간다).
//! - 레이싱: 트랙 밖으로 나가(XA-0) 산만큼 거대한 금고 절반을 조사하고(XA-1) 프린터 우편함에 넣는다(XA-3, 메일로 간다).
//! - 리듬: 게임의 주체를 클릭해서 처치하고(XB-1) 바위에 꽂힌 변기 손잡이를 뽑아(XB-2) 호수에 넣는다(XB-3, 메일 없음).
//!
//! 어디까지 했는지는 MiniProgress(way_open / reward_taken / reward_sent)에 저장돼서, 창을 닫았다 열어도 이어진다.

use super::view::{centered, ellipse, fill};
use super::*;

const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
// 리듬 엔딩의 주체를 쓰러뜨리는 데 필요한 클릭 수.
pub(super) const BOSS_HP: i32 = 10;

impl MiniGameApp {
    pub(super) fn update_reward(&mut self, r: &mut Renderer, o: (f32, f32), win: &WinInput, lang: Language) {
        match self.kind {
            MiniKind::Eye => self.reward_eye(r, o, win, lang),
            MiniKind::Brain => self.reward_brain(r, o, win, lang),
            MiniKind::Finger => self.reward_finger(r, o, win, lang),
        }
        // 가진 것 — 꺼낸 뒤 보내기 전까지.
        if self.progress.reward_taken && !self.progress.reward_sent {
            fill(r, o, 14.0, H - 54.0, 44.0, 44.0, [0.0, 0.0, 0.0, 0.7]);
            self.draw_item_icon(r, o.0 + 17.0, o.1 + H - 51.0, 38.0);
            r.text(o.0 + 66.0, o.1 + H - 46.0, &format!("{} {}", t(lang, s::GOT_ITEM), t(lang, self.item_name())), 0.8, [1.0, 1.0, 0.8, 1.0]);
        }
    }

    fn clicked(&self, win: &WinInput) -> bool {
        win.focused && win.mouse_clicked
    }

    fn highlight(r: &mut Renderer, target: Rect) {
        crate::ui::border(r, target.x - 2.0, target.y - 2.0, target.w + 4.0, target.h + 4.0, [1.0, 1.0, 0.5, 1.0]);
    }

    fn reward_eye(&mut self, r: &mut Renderer, o: (f32, f32), win: &WinInput, lang: Language) {
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
            Self::highlight(r, if hover_c { chest } else { mailbox });
        }
        if self.clicked(win) {
            if can_take && hover_c {
                self.progress.reward_taken = true;
            } else if can_send && hover_m {
                self.send_reward();
            }
        }
        let msg = self.reward_message(lang, s::REWARD_OPEN, s::REWARD_MAIL);
        centered(r, o, 20.0, msg, 0.9, WHITE);
    }

    // 레이싱 — 트랙 출구 → 산만 한 금고 절반 → 프린터 우편함.
    fn reward_brain(&mut self, r: &mut Renderer, o: (f32, f32), win: &WinInput, lang: Language) {
        if !self.progress.way_open {
            self.draw_reward_backdrop(r, o);
            // 트랙 끝의 출구 표지.
            let exit = Rect::new(o.0 + 350.0, o.1 + 150.0, 100.0, 90.0);
            r.rect(exit.x, exit.y, exit.w, exit.h, [0.08, 0.35, 0.18, 1.0]);
            r.rect(exit.x + 6.0, exit.y + 6.0, exit.w - 12.0, exit.h - 12.0, [0.12, 0.5, 0.26, 1.0]);
            r.text(exit.x + 24.0, exit.y + 24.0, "EXIT", 1.2, WHITE);
            r.text(exit.x + 30.0, exit.y + 52.0, "->", 1.4, WHITE);
            let hover = exit.contains(win.mouse.0, win.mouse.1);
            if hover && self.stage == Stage::Reward {
                Self::highlight(r, exit);
            }
            if hover && self.stage == Stage::Reward && self.clicked(win) {
                self.progress.way_open = true;
                self.stage_t = 0.0;
            }
            centered(r, o, 20.0, t(lang, s::EXIT_OPEN), 0.9, WHITE);
            return;
        }
        // 트랙 밖 — 하늘, 땅, 그리고 화면을 다 채우는 금고 절반.
        fill(r, o, 0.0, 0.0, W, H, [0.1, 0.17, 0.2, 1.0]);
        fill(r, o, 0.0, 300.0, W, 60.0, [0.12, 0.16, 0.12, 1.0]);
        let vault = Rect::new(o.0 + 20.0, o.1 + 50.0, 270.0, 250.0);
        r.rect(vault.x, vault.y, vault.w, vault.h, [0.42, 0.44, 0.5, 1.0]);
        r.rect(vault.x + 18.0, vault.y + 18.0, vault.w - 36.0, vault.h - 36.0, [0.3, 0.32, 0.37, 1.0]);
        r.rect(vault.x + vault.w - 40.0, vault.y, 40.0, vault.h, [0.12, 0.13, 0.16, 1.0]); // 잘려서 속이 비어 보이는 쪽
        ellipse(r, o, 135.0, 175.0, 56.0, 56.0, [0.62, 0.64, 0.7, 1.0]);
        ellipse(r, o, 135.0, 175.0, 14.0, 14.0, [0.85, 0.72, 0.3, 1.0]);
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::TAU / 8.0;
            r.rect(o.0 + 135.0 + a.cos() * 40.0 - 2.0, o.1 + 175.0 + a.sin() * 40.0 - 2.0, 4.0, 4.0, [0.2, 0.2, 0.24, 1.0]);
        }
        // 비교를 위한 아주 작은 사람.
        r.rect(o.0 + 304.0, o.1 + 282.0, 6.0, 18.0, [0.9, 0.8, 0.6, 1.0]);
        // 프린터 우편함 — 금고 절반을 집은 뒤에 쓸 수 있다.
        let printer = Rect::new(o.0 + 340.0, o.1 + 200.0, 110.0, 90.0);
        r.rect(printer.x, printer.y + 24.0, printer.w, printer.h - 24.0, [0.75, 0.75, 0.72, 1.0]);
        r.rect(printer.x + 12.0, printer.y, printer.w - 24.0, 28.0, [0.95, 0.95, 0.92, 1.0]);
        r.rect(printer.x + 14.0, printer.y + 44.0, printer.w - 28.0, 8.0, [0.1, 0.1, 0.12, 1.0]);
        r.rect(printer.x + 8.0, printer.y + printer.h - 18.0, printer.w - 16.0, 10.0, [0.9, 0.9, 0.86, 1.0]);
        r.rect(printer.x + printer.w - 26.0, printer.y + 60.0, 10.0, 10.0, [0.3, 0.8, 0.4, 1.0]);
        let hover_v = vault.contains(win.mouse.0, win.mouse.1);
        let hover_p = printer.contains(win.mouse.0, win.mouse.1);
        let can_take = self.stage == Stage::Reward && !self.progress.reward_taken;
        let can_send = self.stage == Stage::Reward && self.progress.reward_taken && !self.progress.reward_sent;
        if (can_take && hover_v) || (can_send && hover_p) {
            Self::highlight(r, if hover_v { vault } else { printer });
        }
        if self.clicked(win) {
            if can_take && hover_v {
                self.progress.reward_taken = true;
            } else if can_send && hover_p {
                self.send_reward();
            }
        }
        let msg = self.reward_message(lang, s::VAULT_HUGE, s::PRINTER_SEND);
        centered(r, o, 20.0, msg, 0.9, WHITE);
    }

    // 리듬 — 게임의 주체 처치 → 바위의 변기 손잡이 → 호수.
    fn reward_finger(&mut self, r: &mut Renderer, o: (f32, f32), win: &WinInput, lang: Language) {
        if !self.progress.way_open {
            fill(r, o, 0.0, 0.0, W, H, [0.13, 0.13, 0.15, 1.0]);
            fill(r, o, 0.0, 300.0, W, 60.0, [0.2, 0.2, 0.22, 1.0]);
            self.boss_hit_t = (self.boss_hit_t - win.dt.min(0.05)).max(0.0);
            let shake = if self.boss_hit_t > 0.0 { (self.clock * 60.0).sin() * 6.0 } else { 0.0 };
            let (cx, cy) = (240.0 + shake, 190.0);
            // 주체 — 커다란 하얀 장갑 얼굴.
            r.rect(o.0 + cx - 60.0, o.1 + cy - 10.0, 120.0, 110.0, [0.95, 0.95, 0.95, 1.0]);
            for k in 0..4 {
                r.rect(o.0 + cx - 60.0 + k as f32 * 31.0, o.1 + cy - 70.0 + (k % 2) as f32 * 10.0, 26.0, 64.0, [0.95, 0.95, 0.95, 1.0]);
            }
            r.rect(o.0 + cx - 92.0, o.1 + cy + 20.0, 36.0, 28.0, [0.95, 0.95, 0.95, 1.0]);
            let (mx, my) = (win.mouse.0 - o.0, win.mouse.1 - o.1);
            for ex in [-26.0f32, 26.0] {
                ellipse(r, o, cx + ex, cy + 24.0, 16.0, 16.0, [0.08, 0.08, 0.1, 1.0]);
                let (dx, dy) = ((mx - cx - ex).clamp(-30.0, 30.0) * 0.12, (my - cy - 24.0).clamp(-30.0, 30.0) * 0.12);
                ellipse(r, o, cx + ex + dx, cy + 24.0 + dy, 5.0, 5.0, [0.95, 0.2, 0.2, 1.0]);
            }
            let hurt = (BOSS_HP - self.boss_hp) as f32 / BOSS_HP as f32;
            r.rect(o.0 + cx - 30.0, o.1 + cy + 62.0, 60.0, 8.0 + 8.0 * hurt, [0.1, 0.02, 0.04, 1.0]);
            // 남은 체력
            r.rect(o.0 + 140.0, o.1 + 50.0, 200.0, 10.0, [0.25, 0.05, 0.05, 1.0]);
            r.rect(o.0 + 140.0, o.1 + 50.0, 200.0 * self.boss_hp as f32 / BOSS_HP as f32, 10.0, [0.85, 0.2, 0.2, 1.0]);
            let boss = Rect::new(o.0 + cx - 92.0, o.1 + cy - 70.0, 212.0, 180.0);
            if self.stage == Stage::Reward && self.clicked(win) && boss.contains(win.mouse.0, win.mouse.1) {
                self.boss_hp -= 1;
                self.boss_hit_t = 0.25;
                if self.boss_hp <= 0 {
                    self.progress.way_open = true;
                }
            }
            centered(r, o, 20.0, t(lang, s::KILL_PROMPT), 0.9, WHITE);
            return;
        }
        // 주체가 쓰러진 뒤의 들판 — 왼쪽에 엑스칼리버처럼 손잡이가 꽂힌 바위, 오른쪽에 호수.
        fill(r, o, 0.0, 0.0, W, H, [0.1, 0.14, 0.2, 1.0]);
        fill(r, o, 0.0, 260.0, W, 100.0, [0.14, 0.2, 0.14, 1.0]);
        let rock = Rect::new(o.0 + 50.0, o.1 + 170.0, 140.0, 110.0);
        r.rect(rock.x, rock.y + 30.0, rock.w, rock.h - 30.0, [0.38, 0.38, 0.42, 1.0]);
        r.rect(rock.x + 20.0, rock.y, rock.w - 50.0, 40.0, [0.45, 0.45, 0.5, 1.0]);
        if !self.progress.reward_taken {
            // 꽂힌 변기 손잡이 — 은색 막대 + 둥근 손잡이.
            r.rect(o.0 + 118.0, o.1 + 90.0, 8.0, 100.0, [0.8, 0.82, 0.88, 1.0]);
            ellipse(r, o, 122.0, 86.0, 14.0, 10.0, [0.9, 0.9, 0.95, 1.0]);
        }
        let lake = Rect::new(o.0 + 270.0, o.1 + 230.0, 190.0, 90.0);
        ellipse(r, o, 365.0, 275.0, 95.0, 45.0, [0.15, 0.3, 0.55, 1.0]);
        ellipse(r, o, 365.0, 275.0, 70.0, 30.0, [0.2, 0.4, 0.7, 1.0]);
        if self.progress.reward_sent {
            // 호수에서 올라온 손이 손잡이를 받아 간다.
            let rise = (self.stage_t * 1.5).min(1.0);
            r.rect(o.0 + 360.0, o.1 + 250.0 - 60.0 * rise, 12.0, 60.0 * rise + 8.0, [0.95, 0.85, 0.75, 1.0]);
            r.rect(o.0 + 352.0, o.1 + 240.0 - 60.0 * rise, 28.0, 16.0, [0.95, 0.85, 0.75, 1.0]);
        }
        let hover_r = rock.contains(win.mouse.0, win.mouse.1);
        let hover_l = lake.contains(win.mouse.0, win.mouse.1);
        let can_take = self.stage == Stage::Reward && !self.progress.reward_taken;
        let can_send = self.stage == Stage::Reward && self.progress.reward_taken && !self.progress.reward_sent;
        if (can_take && hover_r) || (can_send && hover_l) {
            Self::highlight(r, if hover_r { rock } else { lake });
        }
        if self.clicked(win) {
            if can_take && hover_r {
                self.progress.reward_taken = true;
            } else if can_send && hover_l {
                self.send_reward();
            }
        }
        let msg = self.reward_message(lang, s::ROCK_PROMPT, s::LAKE_PROMPT);
        centered(r, o, 20.0, msg, 0.9, WHITE);
    }

    // 보냈다 — 메일이 있는 게임(눈/레이싱)은 보상 메일을 CRT 로 요청한다. 리듬은 호수에 넣을 뿐 메일이 없다.
    fn send_reward(&mut self) {
        self.progress.reward_sent = true;
        self.pending_mail = self.kind.reward_mail();
        self.stage = Stage::Sent;
        self.stage_t = 0.0;
    }

    // 위쪽에 띄우는 안내 — 꺼내기 전 / 꺼낸 뒤 / 보낸 뒤.
    fn reward_message(&self, lang: Language, before_take: crate::strings::S, after_take: crate::strings::S) -> &'static str {
        if self.stage == Stage::Sent {
            t(lang, s::REWARD_SENT)
        } else if !self.progress.reward_taken {
            t(lang, before_take)
        } else {
            t(lang, after_take)
        }
    }
}
