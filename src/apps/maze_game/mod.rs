//! 서브 게임 "B+a" 1스테이지 — 두 번째 메일 뒤에 바탕화면에 생기는 `test2.exe`
//! (FileKind::SubGame)를 열면 뜨는 미로 게임(시트의 SUB A+a-1). 화면/시점/플레이어/
//! 대화창은 doors_game 과 같은 apps/game3d 의 공용 도구를 쓴다.
//!
//! 열 때마다 새로 만들어지는 작은 미로(maze.rs)에서 출발해 도착 지점까지 걸어가면 물뿌리개가
//! 놓여 있고, `E`로 줍는다 — 1라운드 클리어. 아직은 여기까지다(시트의 다음 이벤트인 화분/
//! 사물함 스테이지는 아직 없다).

mod maze;

use miniquad::{KeyCode, RenderingBackend};

use crate::random::Rng;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::render::mesh3d::Box3D;

use super::game3d::dialogue::{Dialogue, Entry};
use super::game3d::player::{aim_dist, Player};
use super::game3d::{MouseLook, View};
use super::{App, AppAction, WinInput};
use maze::Maze;

const CLEAR_COLOR: [f32; 4] = [0.01, 0.02, 0.02, 1.0];
const CAN_HALF: [f32; 3] = [0.15, 0.2, 0.15];
const CAN_COLOR: [f32; 4] = [0.35, 0.55, 0.85, 1.0];

pub struct MazeGameApp {
    boxes: Vec<Box3D>, // 미로 벽/바닥/천장 (+ 아직 안 주웠으면 물뿌리개)
    can: Option<Box3D>, // 도착 지점의 물뿌리개 — 주우면 None
    player: Player,
    look: MouseLook,
    dialogue: Dialogue,
    can_aimed: bool,
    has_can: bool, // 1라운드 클리어(물뿌리개 획득)
}

impl MazeGameApp {
    pub(super) fn new() -> MazeGameApp {
        let maze = Maze::generate(&mut Rng::from_time());
        let can_center = [maze.goal[0], maze.goal[1] + CAN_HALF[1], maze.goal[2]];
        MazeGameApp {
            boxes: maze.boxes,
            can: Some(Box3D { center: can_center, half: CAN_HALF, yaw: 0.0, pitch: 0.0, roll: 0.0, color: CAN_COLOR, texture: None, walkable: false, solid: false }),
            player: Player::at(maze.start, maze.start_yaw),
            look: MouseLook::new(),
            dialogue: Dialogue::new(),
            can_aimed: false,
            has_can: false,
        }
    }

    fn handle_input(&mut self, win: &WinInput, in_view: bool) {
        if self.look.update(win, in_view) {
            return;
        }
        if self.dialogue.active() {
            self.dialogue.handle_input(win, in_view);
            return;
        }
        if win.input.pressed(KeyCode::E) && self.can_aimed {
            self.can = None;
            self.has_can = true;
            self.dialogue.start(vec![Entry::Line("물뿌리개를 발견했다.".to_string()), Entry::Line("1라운드 클리어.".to_string())]);
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

        let frozen = self.dialogue.active();
        if !frozen {
            self.look.apply(&mut self.player, win);
            self.player.update(win.input, win.focused, win.dt, &self.boxes);
            let cam = self.player.camera();
            self.can_aimed = self.can.as_ref().is_some_and(|c| aim_dist(&cam, c.center).is_some());
        }

        let cam = self.player.camera();
        let mut scene = self.boxes.clone();
        scene.extend(self.can.clone());
        view.draw_scene(ctx, r, area, CLEAR_COLOR, &cam, &scene);

        if let Some(can) = &self.can
            && self.can_aimed
            && !frozen
        {
            view.draw_prompt(r, &cam, can.center, "[E] Watering can");
        }
        if !self.look.captured() && !frozen {
            view.draw_capture_hint(r);
        }
        if self.has_can && !frozen {
            r.text(view.px(10.0), view.py(10.0), "물뿌리개", 0.8 * view.s, [0.75, 0.8, 1.0, 1.0]);
        }
        self.dialogue.draw(r, view.rect.x, view.rect.y, view.s, win.time);

        AppAction::None
    }
}
