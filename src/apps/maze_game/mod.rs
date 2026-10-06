//! 서브 게임 "B+a" — 두 번째 메일 뒤에 바탕화면에 생기는 `test2.exe`(FileKind::SubGame)를
//! 열면 뜨는 게임(시트의 SUB A+a-1 ~ A+a-14). 화면/시점/플레이어/대화창은 doors_game 과
//! 같은 apps/game3d 의 공용 도구를 쓴다.
//!
//! 흐름: (1) 미로(maze.rs)를 걸어 도착 지점의 물뿌리개를 줍는다 → (2) 스테이지 1 방(hub.rs)으로
//! 올라온다. 화분을 조사하면(이미 누가 꺾어갔다) 뒤에 사물함이 나타나고, 사물함을 조사하면
//! 열쇠 3개가 필요하다는 걸 알면서 북쪽 문 3개가 열린다 → (3) 문마다 새 미로로 들어가 도착
//! 지점의 열쇠를 줍고, 내려온 밧줄을 조사해 방으로 돌아온다(문 3개 반복) → (4) 열쇠 3개로
//! 사물함을 열어 씨앗을 얻고 화분에 심는다("시간이 1년은 필요할 것 같다") → (5) 심은 뒤 잠깐 있다
//! 가 OS 로 힌트 메일이 오고("바탕화면에서 물리적으로 시간을 돌려도 괜찮지 않을까?"), 진짜 컴퓨터의
//! 시스템 시간이 심은 때보다 1년 이상 앞서가면 꽃이 핀다 → (6) 꽃을 꺾어 우체통에 넣으면 OS 로
//! 꽃 사진이 첨부된 메일이 도착한다(시트의 SUB A+a-15 ~ 18). 진행 상태는 저장하지 않는다(창을
//! 닫으면 처음부터).

mod hub;
mod maze;

use miniquad::{KeyCode, RenderingBackend};

use crate::random::Rng;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::player::{aim_dist, Player};
use super::game3d::{MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use maze::Maze;

const CLEAR_COLOR: [f32; 4] = [0.01, 0.02, 0.02, 1.0];
const KEYS_NEEDED: usize = 3;
const YEAR_SECS: f64 = 365.0 * 86_400.0; // "시간이 1년은 필요하다" — 심은 때보다 시스템 시간이 이만큼 앞서가면 핀다

// 심은 때(planted_at)보다 시스템 시간(now, 유닉스 초)이 1년 이상 앞서갔는지 — 시계를 뒤로 돌린
// 경우(now < planted_at)는 당연히 아니다.
fn year_passed(planted_at: f64, now: f64) -> bool {
    now - planted_at >= YEAR_SECS
}

// 지금 있는 곳.
enum Place {
    // 미로 — kind 는 도착 지점에 뭐가 놓였는지, goal 은 그 자리.
    Maze { kind: MazeKind, goal: [f32; 3] },
    Hub,
}

#[derive(Clone, Copy, PartialEq)]
enum MazeKind {
    Round1,      // 도착 지점에 물뿌리개
    Key(usize),  // 도착 지점에 i+1 번 열쇠 (줍고 나면 밧줄이 내려온다)
}

// 조준해서 E 로 조사할 수 있는 것.
#[derive(Clone, Copy, PartialEq)]
enum Target {
    Can,
    Key(usize),
    Rope,
    Pot,
    Locker,
    Door(usize),
    Flower,
    Mailbox,
}

// 화면에 놓인 조사 가능한 물건 하나 — Box3D 로 그려지고 충돌도 있다(통과 못 한다).
struct Item {
    target: Target,
    center: [f32; 3],
    half: [f32; 3],
    color: [f32; 4],
    label: &'static str,
}

impl Item {
    fn to_box(&self) -> Box3D {
        Box3D { center: self.center, half: self.half, yaw: 0.0, pitch: 0.0, roll: 0.0, color: self.color, texture: None, walkable: false, solid: true }
    }
}

// 대화가 다 끝난 뒤에 할 이동.
#[derive(Clone, Copy)]
enum Move {
    ToHub,
    ToMaze(usize), // 문 i+1 번 미로로
}

// 지금까지의 진행.
#[derive(Default)]
struct Progress {
    has_can: bool,
    pot_checked: bool,    // 화분을 조사했다 → 뒤에 사물함이 나타난다
    locker_checked: bool, // 사물함을 조사했다 → 문 3개가 열린다
    keys: [bool; KEYS_NEEDED],
    seed: bool,
    planted: bool,
    planted_at: Option<f64>, // 씨앗을 심은 때(시스템 시간, 유닉스 초)
    bloomed: bool,           // 시스템 시간이 1년 넘게 흘러 꽃이 폈다
    has_flower: bool,        // 꽃을 꺾어서 가지고 있다
    flower_sent: bool,       // 꽃을 우체통에 넣어 보냈다
}

// 대화가 다 끝난 뒤 OS 에 알릴 일 — AppAction 으로 돌려준다.
#[derive(Clone, Copy)]
enum Notify {
    SeedPlanted,
    FlowerSent,
}

pub struct MazeGameApp {
    walls: Vec<Box3D>, // 지금 곳의 벽/바닥/천장
    place: Place,
    player: Player,
    slot: RenderSlot, // 이 창 전용 3D 렌더 타깃(창끼리 화면이 섞이지 않게)
    look: MouseLook,
    dialogue: Dialogue,
    aimed: Option<Target>,
    progress: Progress,
    pending_move: Option<Move>,
    pending_notify: Option<Notify>,
}

fn new_maze(kind: MazeKind) -> (Vec<Box3D>, Place, Player) {
    let maze = Maze::generate(&mut Rng::from_time());
    let player = Player::at(maze.start, maze.start_yaw);
    (maze.boxes, Place::Maze { kind, goal: maze.goal }, player)
}

impl MazeGameApp {
    pub(super) fn new() -> MazeGameApp {
        let (walls, place, player) = new_maze(MazeKind::Round1);
        MazeGameApp {
            walls,
            place,
            player,
            slot: RenderSlot::acquire(),
            look: MouseLook::new(),
            dialogue: Dialogue::new(),
            aimed: None,
            progress: Progress::default(),
            pending_move: None,
            pending_notify: None,
        }
    }

    // 지금 곳에 놓인, 조사할 수 있는 물건들(진행 상태에 따라 나타났다 사라진다).
    fn items(&self) -> Vec<Item> {
        let p = &self.progress;
        match &self.place {
            Place::Maze { kind, goal } => {
                let at = |dx: f32, y: f32| [goal[0] + dx, y, goal[2]];
                match *kind {
                    MazeKind::Round1 if !p.has_can => {
                        vec![Item { target: Target::Can, center: at(0.0, 0.2), half: [0.15, 0.2, 0.15], color: [0.35, 0.55, 0.85, 1.0], label: "[E] Watering can" }]
                    }
                    MazeKind::Key(i) if !p.keys[i] => {
                        vec![Item { target: Target::Key(i), center: at(0.0, 0.08), half: [0.12, 0.08, 0.2], color: [0.9, 0.8, 0.2, 1.0], label: "[E] Key" }]
                    }
                    // 열쇠를 줍고 나면 위에서 밧줄이 내려온다(바닥~천장을 잇는 가는 기둥).
                    MazeKind::Key(_) => {
                        vec![Item { target: Target::Rope, center: at(0.7, 1.3), half: [0.04, 1.3, 0.04], color: [0.75, 0.62, 0.4, 1.0], label: "[E] Rope" }]
                    }
                    MazeKind::Round1 => Vec::new(),
                }
            }
            Place::Hub => {
                let mut items = vec![Item { target: Target::Pot, center: hub::POT.center, half: hub::POT.half, color: [0.6, 0.35, 0.2, 1.0], label: "[E] Flower pot" }];
                if p.pot_checked {
                    items.push(Item { target: Target::Locker, center: hub::LOCKER.center, half: hub::LOCKER.half, color: [0.35, 0.4, 0.45, 1.0], label: "[E] Locker" });
                }
                items.push(Item { target: Target::Mailbox, center: hub::MAILBOX.center, half: hub::MAILBOX.half, color: [0.3, 0.35, 0.55, 1.0], label: "[E] Mailbox" });
                if p.bloomed && !p.has_flower {
                    items.push(Item { target: Target::Flower, center: hub::FLOWER.center, half: hub::FLOWER.half, color: [0.3, 0.4, 0.95, 1.0], label: "[E] Flower" });
                }
                const DOOR_LABELS: [&str; KEYS_NEEDED] = ["[E] Door 1", "[E] Door 2", "[E] Door 3"];
                for (i, slot) in hub::DOORS.iter().enumerate() {
                    // 잠겨 있을 땐 어두운 나무색, 열리면 밝아진다.
                    let color = if p.locker_checked { [0.5, 0.34, 0.2, 1.0] } else { [0.28, 0.2, 0.15, 1.0] };
                    items.push(Item { target: Target::Door(i), center: slot.center, half: slot.half, color, label: DOOR_LABELS[i] });
                }
                items
            }
        }
    }

    fn line(text: &str) -> Entry {
        Entry::Line(text.to_string())
    }

    fn go_hub(&mut self) {
        self.walls = hub::build_walls();
        self.place = Place::Hub;
        self.player = Player::at(hub::SPAWN, 0.0);
    }

    fn go_maze(&mut self, door: usize) {
        (self.walls, self.place, self.player) = new_maze(MazeKind::Key(door));
    }

    // E — 조준 중인 대상을 조사한다.
    fn interact(&mut self, target: Target) {
        let p = &mut self.progress;
        match target {
            Target::Can => {
                p.has_can = true;
                self.dialogue.start(vec![Self::line("물뿌리개를 발견했다."), Self::line("1라운드 클리어.")]);
                self.pending_move = Some(Move::ToHub);
            }
            Target::Key(i) => {
                p.keys[i] = true;
                self.dialogue.start(vec![Self::line(&format!("열쇠 {}번을 얻었다.", i + 1)), Self::line("위에서 밧줄이 내려왔다.")]);
            }
            Target::Rope => {
                self.dialogue.start(vec![Self::line("밧줄을 타고 올라갔다...")]);
                self.pending_move = Some(Move::ToHub);
            }
            Target::Pot => {
                if p.seed && !p.planted {
                    p.planted = true;
                    p.planted_at = Some(miniquad::date::now());
                    self.pending_notify = Some(Notify::SeedPlanted);
                    self.dialogue.start(vec![Self::line("화분에 씨앗을 심었다."), Self::line("시간이 1년은 필요할 것 같다...")]);
                } else if p.planted {
                    self.dialogue.start(vec![Self::line("씨앗이 심겨 있다. 아직 아무 일도 없다.")]);
                } else {
                    p.pot_checked = true; // 뒤에 사물함이 나타난다
                    self.dialogue.start(vec![Self::line("이미 누가 꺾어간 것 같다."), Self::line("새롭게 심어야 할 것 같은데...")]);
                }
            }
            Target::Locker => {
                let got = p.keys.iter().filter(|&&k| k).count();
                if got == KEYS_NEEDED && !p.seed {
                    p.seed = true;
                    self.dialogue.start(vec![Self::line("열쇠로 자물쇠 3개를 풀었다."), Self::line("씨앗을 얻었다.")]);
                } else if p.seed {
                    self.dialogue.start(vec![Self::line("사물함은 비어 있다.")]);
                } else if !p.locker_checked {
                    p.locker_checked = true; // 문 3개가 열린다
                    self.dialogue.start(vec![Self::line("잠겨 있다. 열쇠 3개가 필요할 것 같다."), Self::line("어디선가 문이 열리는 소리가 들렸다.")]);
                } else {
                    self.dialogue.start(vec![Self::line(&format!("열쇠가 부족하다. ({got}/{KEYS_NEEDED})"))]);
                }
            }
            Target::Flower => {
                p.has_flower = true;
                self.dialogue.start(vec![Self::line("꽃을 꺾었다."), Self::line("보리지꽃이다.")]);
            }
            Target::Mailbox => {
                if p.has_flower && !p.flower_sent {
                    p.flower_sent = true;
                    self.pending_notify = Some(Notify::FlowerSent);
                    self.dialogue.start(vec![Self::line("꽃을 우체통에 넣었다."), Self::line("어딘가로 보내진 것 같다...")]);
                } else if p.flower_sent {
                    self.dialogue.start(vec![Self::line("우체통은 비어 있다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("우체통이다. 지금은 보낼 게 없다.")]);
                }
            }
            Target::Door(i) => {
                if !p.locker_checked {
                    self.dialogue.start(vec![Self::line("잠겨 있다.")]);
                } else if p.keys[i] {
                    self.dialogue.start(vec![Self::line("이미 다녀온 문이다.")]);
                } else {
                    self.dialogue.start(vec![Self::line("문이 열려 있다."), Self::line("안으로 들어간다...")]);
                    self.pending_move = Some(Move::ToMaze(i));
                }
            }
        }
    }

    // 씨앗을 심은 뒤 시스템 시간이 1년 넘게 앞서갔으면 꽃이 핀다 — 지금 대화가 떠 있지 않을 때만
    // (이미 나오는 대화를 끊지 않게). 이번에 피었으면 true.
    fn bloom_if_due(&mut self, now: f64) -> bool {
        let p = &mut self.progress;
        if let Some(t0) = p.planted_at
            && !p.bloomed
            && !self.dialogue.active()
            && year_passed(t0, now)
        {
            p.bloomed = true;
            self.dialogue.start(vec![Self::line("화분에서 무언가 달라졌다..."), Self::line("꽃이 피었다.")]);
            return true;
        }
        false
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        if self.look.update(win, in_view) {
            return;
        }
        if self.dialogue.active() {
            self.dialogue.handle_input(win, in_view);
            return;
        }
        if win.input.pressed(KeyCode::E)
            && let Some(target) = self.aimed
        {
            self.interact(target);
        }
    }
}

impl App for MazeGameApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let view = View::fit(area);
        let in_view = view.rect.contains(win.mouse.0, win.mouse.1);

        self.handle_input(win, in_view);
        self.look.request();
        self.dialogue.tick(win.dt);

        // 대화가 다 끝났으면 미뤄둔 이동(방/미로 전환)을 한다.
        if !self.dialogue.active()
            && let Some(mv) = self.pending_move.take()
        {
            match mv {
                Move::ToHub => self.go_hub(),
                Move::ToMaze(i) => self.go_maze(i),
            }
        }

        self.bloom_if_due(miniquad::date::now());

        let items = self.items();
        // 벽 + 물건 전부가 충돌/렌더 대상이다.
        let mut scene = self.walls.clone();
        scene.extend(items.iter().map(Item::to_box));

        let frozen = self.dialogue.active();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &scene);
            let cam = self.player.camera();
            self.aimed = items
                .iter()
                .filter_map(|it| aim_dist(&cam, it.center).map(|d| (it, d)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(it, _)| it.target);
        }

        let cam = self.player.camera();
        view.draw_scene(ctx, r, area, &self.slot, CLEAR_COLOR, &cam, &scene);

        if let Some(target) = self.aimed
            && !frozen
            && let Some(item) = items.iter().find(|it| it.target == target)
        {
            view.draw_prompt(r, &cam, item.center, item.label);
        }
        if !self.look.captured() && !frozen {
            view.draw_capture_hint(r);
        }
        self.draw_inventory(r, &view);
        self.dialogue.draw(r, view.rect.x, view.rect.y, view.s, win.time);

        // 대화가 다 끝났으면 OS 에 알릴 일(씨앗 심음/꽃 발송)을 한 번 알린다.
        if !self.dialogue.active()
            && let Some(n) = self.pending_notify.take()
        {
            return match n {
                Notify::SeedPlanted => AppAction::SeedPlanted,
                Notify::FlowerSent => AppAction::FlowerSent,
            };
        }
        AppAction::None
    }
}

impl MazeGameApp {
    // 왼쪽 위에 가진 것들을 한 줄씩.
    fn draw_inventory(&self, r: &mut Renderer, view: &View) {
        let p = &self.progress;
        let keys = p.keys.iter().filter(|&&k| k).count();
        let mut lines = Vec::new();
        if p.has_can {
            lines.push("물뿌리개".to_string());
        }
        if keys > 0 {
            lines.push(format!("열쇠 {keys}/{KEYS_NEEDED}"));
        }
        if p.seed && !p.planted {
            lines.push("씨앗".to_string());
        }
        if p.has_flower && !p.flower_sent {
            lines.push("보리지꽃".to_string());
        }
        for (i, line) in lines.iter().enumerate() {
            r.text(view.px(10.0), view.py(10.0 + i as f32 * 20.0), line, 0.8 * view.s, [0.75, 0.8, 1.0, 1.0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(app: &MazeGameApp, target: Target) -> bool {
        app.items().iter().any(|it| it.target == target)
    }

    // 시트의 흐름(물뿌리개 → 화분 → 사물함 → 문/열쇠 3번 → 씨앗 → 심기)을 처음부터 끝까지 따라가 본다.
    #[test]
    fn full_progression() {
        let mut app = MazeGameApp::new();
        assert!(has(&app, Target::Can));

        app.interact(Target::Can);
        assert!(app.progress.has_can && matches!(app.pending_move, Some(Move::ToHub)));
        app.pending_move = None;
        app.go_hub();
        assert!(has(&app, Target::Pot) && !has(&app, Target::Locker), "사물함은 화분을 조사한 뒤에 나타난다");

        // 문은 사물함을 조사하기 전엔 잠겨 있다.
        app.interact(Target::Door(0));
        assert!(app.pending_move.is_none());

        app.interact(Target::Pot);
        assert!(app.progress.pot_checked && has(&app, Target::Locker));
        app.interact(Target::Locker);
        assert!(app.progress.locker_checked && !app.progress.seed);

        for i in 0..KEYS_NEEDED {
            app.interact(Target::Door(i));
            assert!(matches!(app.pending_move, Some(Move::ToMaze(d)) if d == i));
            app.pending_move = None;
            app.go_maze(i);
            assert!(has(&app, Target::Key(i)) && !has(&app, Target::Rope));
            app.interact(Target::Key(i));
            assert!(app.progress.keys[i] && has(&app, Target::Rope) && !has(&app, Target::Key(i)), "열쇠를 주우면 밧줄이 내려온다");
            app.interact(Target::Rope);
            assert!(matches!(app.pending_move, Some(Move::ToHub)));
            app.pending_move = None;
            app.go_hub();
            // 이미 다녀온 문은 다시 안 들어가진다.
            app.interact(Target::Door(i));
            assert!(app.pending_move.is_none());
            if i + 1 < KEYS_NEEDED {
                app.interact(Target::Locker);
                assert!(!app.progress.seed, "열쇠가 모자라면 안 열린다");
            }
        }

        app.interact(Target::Locker);
        assert!(app.progress.seed);
        app.interact(Target::Pot);
        assert!(app.progress.planted);
        assert!(matches!(app.pending_notify, Some(Notify::SeedPlanted)), "씨앗을 심으면 OS 에 알린다");
        app.pending_notify = None;

        // 시스템 시간이 1년 안 흘렀으면 안 피고, 넘으면 핀다(대화가 떠 있으면 그동안은 미룬다).
        let t0 = app.progress.planted_at.unwrap();
        app.dialogue = Dialogue::new();
        assert!(!app.bloom_if_due(t0 + YEAR_SECS - 1.0));
        assert!(!has(&app, Target::Flower));
        assert!(!app.bloom_if_due(t0 - YEAR_SECS), "시계를 뒤로 돌려도 안 핀다");
        assert!(app.bloom_if_due(t0 + YEAR_SECS) && has(&app, Target::Flower));

        // 꽃을 꺾어 우체통에 넣으면 OS 에 알린다. 꽃이 없으면 보낼 게 없다.
        app.dialogue = Dialogue::new();
        app.interact(Target::Mailbox);
        assert!(!app.progress.flower_sent);
        app.interact(Target::Flower);
        assert!(app.progress.has_flower && !has(&app, Target::Flower));
        app.interact(Target::Mailbox);
        assert!(app.progress.flower_sent && matches!(app.pending_notify, Some(Notify::FlowerSent)));
    }

    #[test]
    fn year_boundary() {
        assert!(year_passed(0.0, YEAR_SECS));
        assert!(!year_passed(0.0, YEAR_SECS - 1.0));
        assert!(!year_passed(100.0, 50.0));
    }
}
