//! 서브 게임 "B+a" — 두 번째 메일 뒤에 바탕화면에 생기는 `test2.exe`(FileKind::SubGame)를
//! 열면 뜨는 게임(시트의 SUB A+a-1 ~ A+a-14). 화면/시점/플레이어/대화창은 doors_game 과
//! 같은 apps/game3d 의 공용 도구를 쓴다.
//!
//! 흐름: (1) 미로(maze.rs)를 걸어 도착 지점의 물뿌리개를 줍는다 → (2) 스테이지 1 방(hub.rs)으로
//! 올라온다. 화분을 조사하면(이미 누가 꺾어갔다) 플레이어가 등을 돌린 사이 남쪽 벽에 사물함이 생기고, 사물함을 조사하면
//! 열쇠 3개가 필요하다는 걸 알면서 북쪽 문 3개가 열린다 → (3) 문마다 새 미로로 들어가 도착
//! 지점의 열쇠를 줍고, 내려온 밧줄을 조사해 방으로 돌아온다(문 3개 반복) → (4) 열쇠 3개로
//! 사물함을 열어 씨앗을 얻고 화분에 심는다("시간이 1년은 필요할 것 같다") → (5) 심은 뒤 잠깐 있다
//! 가 OS 로 힌트 메일이 오고("바탕화면에서 물리적으로 시간을 돌려도 괜찮지 않을까?"), 진짜 컴퓨터의
//! 시스템 시간이 심은 때보다 1년 이상 앞서가면 꽃이 핀다 → (6) 꽃을 꺾어 우체통에 넣으면 OS 로
//! 꽃 사진이 첨부된 메일이 도착한다(시트의 SUB A+a-15 ~ 18). 진행 상황은 바뀔 때마다 OS 에 저장을
//! 요청해서(AppAction::SaveMaze) 창을 닫았다 다시 열어도 이어진다.

mod hub;
mod maze;

use miniquad::{KeyCode, RenderingBackend};

use crate::foundation::{MazeProgress, MAZE_KEYS};
use crate::random::Rng;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::{Box3D, Camera};

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::ena::{ena_time, overshoot};
use super::game3d::item;
use super::game3d::player::{aim_dist, Player};
use super::game3d::sprite;
use super::game3d::{is_on_screen, Fade, MouseLook, RenderSlot, View};
use super::{App, AppAction, WinInput};
use maze::Maze;

const CLEAR_COLOR: [f32; 4] = [0.01, 0.02, 0.02, 1.0];
const KEYS_NEEDED: usize = MAZE_KEYS;
const YEAR_SECS: f64 = 365.0 * 86_400.0; // "시간이 1년은 필요하다" — 심은 때보다 시스템 시간이 이만큼 앞서가면 핀다

// 심은 때(planted_at)보다 시스템 시간(now, 유닉스 초)이 1년 이상 앞서갔는지 — 시계를 뒤로 돌린
// 경우(now < planted_at)는 당연히 아니다.
fn year_passed(planted_at: f64, now: f64) -> bool {
    now - planted_at >= YEAR_SECS
}

// ---- ENA 식 연출(game3d/ena.rs: 뚝뚝 끊는 프레임 + 오버슈트) ----
const ROPE_DROP_SECS: f32 = 1.25; // 밧줄이 내려오는 데 걸리는 시간
const ROPE_LEN: f32 = 2.6;
const CAN_ANIM_SECS: f32 = 1.2; // 물뿌리개가 튀어 오르며 사라지는 시간

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
type Item = item::Item<Target>;

// 대화가 다 끝난 뒤에 할 이동.
#[derive(Clone, Copy)]
enum Move {
    ToHub,
    ToMaze(usize), // 문 i+1 번 미로로
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
    progress: MazeProgress,
    saved: MazeProgress, // 마지막으로 OS 에 저장을 요청한 진행 상황 — 달라지면 다시 요청한다
    pending_move: Option<Move>,
    pending_notify: Option<Notify>,
    fade: Fade,          // 미로 ↔ 방으로 장면이 바뀔 때 화면을 어둡게 덮었다 걷는다
    clock: f32,          // 이 창이 열린 뒤 흐른 시간 — ENA 식 연출의 기준
    rope_since: Option<f32>, // 밧줄이 내려오기 시작한 시각(clock 기준) — None 이면 이미 다 내려와 있다
    can_anim: Option<f32>,   // 물뿌리개를 집은 뒤 흐른 시간 — 도는 동안 플레이어는 멈춘다
}

fn new_maze(kind: MazeKind) -> (Vec<Box3D>, Place, Player) {
    let maze = Maze::generate(&mut Rng::from_time());
    let player = Player::at(maze.start, maze.start_yaw);
    (maze.boxes, Place::Maze { kind, goal: maze.goal }, player)
}

impl MazeGameApp {
    // 이어서 하는 거면(물뿌리개를 이미 얻었으면) 방에서 시작하고, 아니면 첫 미로에서 시작한다.
    // 미로는 열 때마다 새로 만들어지고, 열쇠 미로 도중에 닫았다면 방으로 돌아온 걸로 이어진다.
    pub(super) fn new(progress: MazeProgress) -> MazeGameApp {
        let resume = progress.has_can;
        let (walls, place, player) = if resume { (hub::build_walls(), Place::Hub, Player::at(hub::SPAWN, 0.0)) } else { new_maze(MazeKind::Round1) };
        let mut dialogue = Dialogue::new();
        if !resume {
            // 열자마자 낯선 곳에 서 있는 이유와 할 일을 한마디로 알려준다.
            dialogue.start(vec![Self::line("눈을 떠 보니 어두운 복도 안이다."), Self::line("어딘가에 쓸 만한 물건이 있을 것 같다...")]);
        }
        // 이어서 하는 거면 이미 한 일(씨앗 심음/꽃 발송)을 OS 에 다시 한 번 알린다(메일은 중복으로 안 온다).
        let pending_notify = if progress.flower_sent {
            Some(Notify::FlowerSent)
        } else if progress.planted {
            Some(Notify::SeedPlanted)
        } else {
            None
        };
        MazeGameApp {
            walls,
            place,
            player,
            slot: RenderSlot::acquire(),
            look: MouseLook::new(),
            dialogue,
            aimed: None,
            saved: progress.clone(),
            progress,
            pending_move: None,
            pending_notify,
            fade: Fade::new(),
            clock: 0.0,
            rope_since: None,
            can_anim: None,
        }
    }

    // 지금 곳에 놓인, 조사할 수 있는 물건들(진행 상태에 따라 나타났다 사라진다).
    fn items(&self) -> Vec<Item> {
        let p = &self.progress;
        match &self.place {
            Place::Maze { kind, goal } => {
                let at = |dx: f32, y: f32| [goal[0] + dx, y, goal[2]];
                match *kind {
                    // 물뿌리개는 ENA 식으로 뚝뚝 끊어 위아래로 까딱이며 돈다.
                    MazeKind::Round1 if !p.has_can => {
                        let t = ena_time(self.clock);
                        let mut can = Item::new(Target::Can, at(0.0, 0.2 + 0.06 * (t * 5.0).sin()), [0.15, 0.2, 0.15], [0.35, 0.55, 0.85, 1.0], "[E] Watering can");
                        can.yaw = t * 2.2;
                        vec![can]
                    }
                    MazeKind::Key(i) if !p.keys[i] => {
                        vec![Item::new(Target::Key(i), at(0.0, 0.08), [0.12, 0.08, 0.2], [0.9, 0.8, 0.2, 1.0], "[E] Key")]
                    }
                    // 열쇠를 줍고 나면 위에서 밧줄이 내려온다(모양은 decor()가 그리고, 다 내려온 뒤에야 조사할 수 있다).
                    MazeKind::Key(_) if self.rope_ready() => {
                        vec![Item::marker(Target::Rope, at(0.7, 1.3), [0.1, 1.3, 0.1], "[E] Rope")]
                    }
                    MazeKind::Key(_) => Vec::new(),
                    MazeKind::Round1 => Vec::new(),
                }
            }
            Place::Hub => {
                let mut items = vec![Item::new(Target::Pot, hub::POT.center, hub::POT.half, [0.6, 0.35, 0.2, 1.0], "[E] Flower pot")];
                if p.locker_seen {
                    items.push(Item::new(Target::Locker, hub::LOCKER.center, hub::LOCKER.half, [0.35, 0.4, 0.45, 1.0], "[E] Locker"));
                }
                items.push(Item::new(Target::Mailbox, hub::MAILBOX.center, hub::MAILBOX.half, [0.3, 0.35, 0.55, 1.0], "[E] Mailbox"));
                if p.bloomed && !p.has_flower {
                    // 꽃은 스프라이트로 그린다 — 여기선 조준 대상일 뿐.
                    items.push(Item::marker(Target::Flower, hub::FLOWER.center, hub::FLOWER.half, "[E] Flower"));
                }
                const DOOR_LABELS: [&str; KEYS_NEEDED] = ["[E] Door 1", "[E] Door 2", "[E] Door 3"];
                for (i, slot) in hub::DOORS.iter().enumerate() {
                    // 잠겨 있을 땐 어두운 나무색, 열리면 밝아진다.
                    let color = if p.locker_checked { [0.5, 0.34, 0.2, 1.0] } else { [0.28, 0.2, 0.15, 1.0] };
                    items.push(Item::new(Target::Door(i), slot.center, slot.half, color, DOOR_LABELS[i]));
                }
                items
            }
        }
    }

    // 밧줄이 다 내려와서 잡을 수 있는지.
    fn rope_ready(&self) -> bool {
        self.rope_since.is_none_or(|t0| self.clock - t0 >= ROPE_DROP_SECS)
    }

    // 그리기만 하는 장식 상자들(충돌/조사 없음) — 밧줄 마디, 집어 올린 물뿌리개.
    fn decor(&self) -> Vec<Box3D> {
        let Place::Maze { kind, goal } = &self.place else { return Vec::new() };
        let mut out = Vec::new();
        if let MazeKind::Key(i) = *kind
            && self.progress.keys[i]
        {
            // 밧줄 — 그냥 긴 막대 하나가 천장에서 뚝뚝 끊겨 내려오고(ENA 식), 도착할 때 한 번 더 길게 늘어났다 돌아온다.
            let elapsed = self.rope_since.map_or(ROPE_DROP_SECS, |t0| ena_time(self.clock - t0).max(0.0));
            let len = (overshoot(elapsed / ROPE_DROP_SECS) * ROPE_LEN).max(0.0); // 지금 길이
            if len > 0.01 {
                out.push(Box3D {
                    center: [goal[0] + 0.7, ROPE_LEN - len / 2.0, goal[2]],
                    half: [0.045, len / 2.0, 0.045],
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                    color: [0.7, 0.57, 0.36, 1.0],
                    texture: None,
                    walkable: false,
                    solid: false,
                });
            }
        }
        if let Some(a) = self.can_anim {
            // 물뿌리개 — 주워 든 순간 위로 튀어 오르며(오버슈트) 세로로 길게 늘어났다 납작해지고, 돌면서 사라진다.
            let x = ena_time(a) / CAN_ANIM_SECS;
            let lift = overshoot(x) * 1.1;
            let stretch = 1.0 + 0.5 * (x * std::f32::consts::PI).sin();
            let squash = 1.0 / stretch.sqrt();
            let fade_out = (1.0 - (x - 0.7).max(0.0) / 0.3).clamp(0.0, 1.0);
            out.push(Box3D {
                center: [goal[0], 0.2 + lift, goal[2]],
                half: [0.15 * squash * fade_out, 0.2 * stretch * fade_out, 0.15 * squash * fade_out],
                yaw: x * 14.0,
                pitch: 0.0,
                roll: 0.0,
                color: [0.35, 0.55, 0.85, 1.0],
                texture: None,
                walkable: false,
                solid: false,
            });
        }
        out
    }

    // 화분을 조사한 뒤, 사물함이 놓일 자리가 시야에서 벗어난 순간에 사물함이 생긴다 — 눈앞에서 갑자기
    // 나타나지 않고, 등 뒤에서 소리가 난 걸로 한다. 이번에 생겼으면 true.
    fn reveal_locker(&mut self, cam: &Camera) -> bool {
        let p = &mut self.progress;
        if !matches!(self.place, Place::Hub) || !p.pot_checked || p.locker_seen || self.dialogue.active() || self.fade.active() {
            return false;
        }
        let dx = cam.pos[0] - hub::LOCKER.center[0];
        let dz = cam.pos[2] - hub::LOCKER.center[2];
        if is_on_screen(cam, hub::LOCKER.center) || dx * dx + dz * dz < 2.0 * 2.0 {
            return false;
        }
        p.locker_seen = true;
        self.dialogue.start(vec![Self::line("...등 뒤에서 쿵, 하는 소리가 났다."), Self::line("돌아보니 벽가에 사물함이 놓여 있다. 아까는 없었던 것 같은데.")]);
        true
    }

    // 물뿌리개가 사라지는 연출이 끝나면 대화와 방으로의 이동이 이어진다.
    fn finish_can(&mut self) {
        self.can_anim = None;
        self.dialogue.start(vec![Self::line("물뿌리개를 발견했다."), Self::line("1라운드 클리어."), Self::line("그 순간 주위가 하얗게 흐려졌다...")]);
        self.pending_move = Some(Move::ToHub);
    }

    fn line(text: &str) -> Entry {
        Entry::Line(text.to_string())
    }

    fn go_hub(&mut self) {
        self.walls = hub::build_walls();
        self.place = Place::Hub;
        self.player = Player::at(hub::SPAWN, 0.0);
        // 처음 올라왔을 땐 갑자기 방에 와 있는 이유(물뿌리개를 줍자 주위가 흐려졌다)를 이어서 알려준다.
        if !self.progress.hub_visited {
            self.progress.hub_visited = true;
            self.dialogue.start(vec![Self::line("정신을 차려 보니 낯선 방 안이다."), Self::line("한쪽에 화분이 하나 놓여 있다.")]);
        }
    }

    fn go_maze(&mut self, door: usize) {
        (self.walls, self.place, self.player) = new_maze(MazeKind::Key(door));
        self.dialogue.start(vec![Self::line("문 너머는 또 다른 어두운 복도였다.")]);
    }

    // E — 조준 중인 대상을 조사한다.
    fn interact(&mut self, target: Target) {
        let p = &mut self.progress;
        match target {
            Target::Can => {
                p.has_can = true;
                self.can_anim = Some(0.0);
            }
            Target::Key(i) => {
                p.keys[i] = true;
                self.rope_since = Some(self.clock);
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
                    p.pot_checked = true; // 등을 돌리면 사물함이 나타난다(reveal_locker)
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
        if self.look.update(win, in_view) || self.fade.active() {
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
        self.clock += win.dt;
        if let Some(a) = &mut self.can_anim {
            *a += win.dt;
            if *a >= CAN_ANIM_SECS {
                self.finish_can();
            }
        }

        // 대화가 다 끝났으면 미뤄둔 이동(방/미로 전환)을 위해 화면을 어둡게 덮고, 완전히 어두워진
        // 순간에 장면을 바꾼다(그래서 화면이 확 바뀌어 보이지 않는다).
        if !self.dialogue.active() && self.pending_move.is_some() {
            // 물뿌리개를 줍고 옮겨질 땐 "하얗게 흐려진다"는 대사대로 흰색으로 덮는다.
            if matches!(self.pending_move, Some(Move::ToHub)) && matches!(self.place, Place::Maze { kind: MazeKind::Round1, .. }) {
                self.fade.start_colored([1.0, 1.0, 1.0]);
            } else {
                self.fade.start();
            }
        }
        if self.fade.update(win.dt)
            && let Some(mv) = self.pending_move.take()
        {
            match mv {
                Move::ToHub => self.go_hub(),
                Move::ToMaze(i) => self.go_maze(i),
            }
        }

        self.bloom_if_due(miniquad::date::now());

        let items = self.items();
        // 벽 + 물건 + 장식(막지 않는 상자)을 한 목록에 모아 충돌과 그리기에 같이 쓴다(프레임당 한 번만 만든다).
        let mut scene = self.walls.clone();
        scene.extend(items.iter().filter(|it| it.draw).map(Item::to_box));
        scene.extend(self.decor());
        let frozen = self.dialogue.active() || self.fade.active() || self.can_anim.is_some();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &scene);
            let cam = self.player.camera();
            self.reveal_locker(&cam);
            self.aimed = items
                .iter()
                .filter_map(|it| aim_dist(&cam, it.center).map(|d| (it, d)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(it, _)| it.target);
        }

        let cam = self.player.camera();
        view.draw_scene(ctx, r, area, &self.slot, CLEAR_COLOR, &cam, &scene);
        self.draw_sprout(r, &view, &cam);

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
        self.fade.draw(r, &view);
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
        // 진행 상황이 바뀌었으면 OS 에 저장을 요청한다.
        if self.progress != self.saved {
            self.saved = self.progress.clone();
            return AppAction::SaveMaze(self.saved.clone());
        }
        AppAction::None
    }
}

impl MazeGameApp {
    // 화분 위의 씨앗/꽃 — DOOM 의 적처럼 항상 카메라를 보는 2D 그림(스프라이트)으로 그린다.
    fn draw_sprout(&self, r: &mut Renderer, view: &View, cam: &Camera) {
        let p = &self.progress;
        if !matches!(self.place, Place::Hub) {
            return;
        }
        if p.bloomed && !p.has_flower {
            view.draw_billboard(r, cam, hub::SPROUT_POS, 0.7, &sprite::BLOOM, &self.walls);
        } else if p.planted && !p.bloomed {
            view.draw_billboard(r, cam, hub::SPROUT_POS, 0.22, &sprite::SEED, &self.walls);
        }
    }

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
        let mut app = MazeGameApp::new(MazeProgress::default());
        assert!(has(&app, Target::Can));

        app.interact(Target::Can);
        assert!(app.progress.has_can && app.can_anim.is_some() && app.pending_move.is_none(), "집은 순간 연출이 먼저 돈다");
        assert!(!app.decor().is_empty(), "연출 동안 물뿌리개가 튀어 오른다");
        app.finish_can();
        assert!(matches!(app.pending_move, Some(Move::ToHub)));
        app.pending_move = None;
        app.go_hub();
        assert!(has(&app, Target::Pot) && !has(&app, Target::Locker), "사물함은 화분을 조사한 뒤에 나타난다");

        // 문은 사물함을 조사하기 전엔 잠겨 있다.
        app.interact(Target::Door(0));
        assert!(app.pending_move.is_none());

        app.interact(Target::Pot);
        assert!(app.progress.pot_checked && !has(&app, Target::Locker), "화분을 조사한 직후엔 아직 없다");
        app.dialogue = Dialogue::new();
        // 사물함 쪽을 보고 있는 동안엔 안 생기고, 등을 돌리면 생긴다.
        let toward = Camera { pos: [0.0, 1.55, 0.0], yaw: std::f32::consts::PI, pitch: 0.0 }; // +Z(남쪽 벽) 쪽
        assert!(!app.reveal_locker(&toward) && !has(&app, Target::Locker));
        let away = Camera { pos: [0.0, 1.55, 0.0], yaw: 0.0, pitch: 0.0 }; // 문들 쪽
        assert!(app.reveal_locker(&away) && has(&app, Target::Locker));
        app.dialogue = Dialogue::new();
        app.interact(Target::Locker);
        assert!(app.progress.locker_checked && !app.progress.seed);

        for i in 0..KEYS_NEEDED {
            app.interact(Target::Door(i));
            assert!(matches!(app.pending_move, Some(Move::ToMaze(d)) if d == i));
            app.pending_move = None;
            app.go_maze(i);
            assert!(has(&app, Target::Key(i)) && !has(&app, Target::Rope));
            app.interact(Target::Key(i));
            assert!(app.progress.keys[i] && !has(&app, Target::Key(i)));
            app.clock += 0.5;
            assert!(!has(&app, Target::Rope) && !app.decor().is_empty(), "밧줄은 뚝뚝 내려오는 중엔 아직 못 잡는다");
            app.clock += ROPE_DROP_SECS;
            assert!(has(&app, Target::Rope), "다 내려오면 밧줄을 조사할 수 있다");
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

    // 물뿌리개를 이미 얻었으면 방에서 이어서 시작하고, 이미 한 일은 OS 에 다시 알린다.
    #[test]
    fn resuming_starts_in_the_hub() {
        let fresh = MazeGameApp::new(MazeProgress::default());
        assert!(matches!(fresh.place, Place::Maze { kind: MazeKind::Round1, .. }) && fresh.dialogue.active());

        let saved = MazeProgress { has_can: true, hub_visited: true, planted: true, flower_sent: true, ..MazeProgress::default() };
        let app = MazeGameApp::new(saved.clone());
        assert!(matches!(app.place, Place::Hub) && !app.dialogue.active());
        assert_eq!(app.progress, saved);
        assert!(matches!(app.pending_notify, Some(Notify::FlowerSent)));
        assert!(has(&app, Target::Pot) && has(&app, Target::Mailbox));
    }

    #[test]
    fn year_boundary() {
        assert!(year_passed(0.0, YEAR_SECS));
        assert!(!year_passed(0.0, YEAR_SECS - 1.0));
        assert!(!year_passed(100.0, 50.0));
    }
}
