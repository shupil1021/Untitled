//! 장면 진행 — 장소 이동(go: 시작방 / 정글 / 미션 방 / 계단 입구)과 문 열림·계단 펼침·페이드 같은 연출 시간 진행,
//! 창이 뒤로 갈 때 서 있던 자리 기억.

use crate::foundation::{MailId, MainPlace};

use super::floor2;
use crate::apps::game3d::ena::{ena_time, smooth};
use crate::apps::game3d::player::Player;
use super::rooms;
use super::world::{DOOR_LIFT, SPAWN};
use super::{DoorsGameApp, DOOR_OPEN_SECS, UNFOLD_SECS};

impl DoorsGameApp {
    // 장소를 옮긴다 — 화면이 가려진 순간에 부른다.
    pub(super) fn go(&mut self, to: MainPlace) {
        self.progress.place = to;
        self.progress.pose = None;
        self.aimed = None;
        let mut start = (rooms::SPAWN, 0.0);
        match to {
            MainPlace::StartRoom => {
                start = (SPAWN, 0.0);
                self.jungle = Vec::new();
                self.barriers = Vec::new();
            }
            MainPlace::Jungle => {
                start = match self.entry_gate.take() {
                    Some(gate) => (floor2::gate_front(gate), floor2::SPAWN_YAW),
                    None => (floor2::SPAWN, floor2::SPAWN_YAW),
                };
                self.jungle = Self::jungle_scene(&self.progress);
                self.barriers = floor2::barriers();
                // 처음 나왔을 때만: 갑자기 바뀐 풍경을 말로 풀어주고 메일("밖이라고?")이 온다(MAIN B-1).
                if !self.progress.jungle_visited {
                    self.progress.jungle_visited = true;
                    self.dialogue.start(vec![
                        Self::line("문 밖은 도시 한복판의 정글이었다."),
                        Self::line("빌딩 사이로 덩굴과 나무가 우거져 있다."),
                        Self::line("길 한복판에 문들이 덩그러니 서 있다."),
                    ]);
                    self.pending_mail = Some(MailId::Outside);
                }
            }
            MainPlace::MissionRoom(_) | MainPlace::StairHall => {
                self.jungle = Vec::new();
                self.barriers = Vec::new();
            }
        }
        self.player = Player::at(start.0, start.1);
    }

    // 문 올라가는 연출, 계단 펼쳐지는 연출, 장소 이동 페이드 같은 장면 진행(대화와 상관없이 매 프레임).
    pub(super) fn advance_scene(&mut self, dt: f32) {
        self.clock += dt;
        if self.unlock_pending && !self.dialogue.active() {
            self.unlock_pending = false;
            self.progress.tutorial_done = true;
            self.door_anim = Some(0.0);
        }
        if let Some(a) = &mut self.door_anim {
            *a += dt;
            // ENA 식으로 뚝뚝 끊겨 올라간다.
            self.door_lift = DOOR_LIFT * smooth(ena_time(*a) / DOOR_OPEN_SECS);
            if *a >= DOOR_OPEN_SECS {
                self.door_anim = None;
                self.door_lift = DOOR_LIFT;
            }
        }
        // 먹이는 대화가 끝나면 계단이 몸을 펼친다.
        if self.progress.chip_fed
            && !self.dialogue.active()
            && let Some(a) = &mut self.unfold_anim
        {
            *a += dt;
            self.unfold = (*a / UNFOLD_SECS).min(1.0);
            if self.unfold >= 1.0 {
                self.unfold_anim = None;
            }
        }
        if !self.dialogue.active() && self.pending_move.is_some() && self.door_anim.is_none() {
            // 문 너머의 빛으로 나갈 땐 하얗게, 나머지는 검게 덮는다.
            if self.pending_move == Some(MainPlace::Jungle) && self.progress.place == MainPlace::StartRoom {
                self.fade.start_colored([1.0, 1.0, 0.92]);
            } else {
                self.fade.start();
            }
        }
        if self.fade.update(dt)
            && let Some(to) = self.pending_move.take()
        {
            self.go(to);
        }
    }

    // 다른 창이 앞에 오는 순간 지금 서 있는 자리를 기억해 둔다(CRT 에서 게임을 오갈 때 위치 유지).
    pub(super) fn remember_pose_on_blur(&mut self, focused: bool) {
        if self.was_focused && !focused && !self.fade.active() {
            self.progress.pose = Some(self.player.pose());
        }
        self.was_focused = focused;
    }
}
