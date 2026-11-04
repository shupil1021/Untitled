//! 이야기가 지금 어느 단계인지 — 진행 상태(fs)에서 뽑아낸다. 로컬 플레이 기록(playlog)이 단계마다 얼마나 걸렸는지 잴 때만 쓰고,
//! 화면에는 보이지 않는다. 단계는 이야기 순서이고, 여기서는 계산만 한다.

use serde::{Deserialize, Serialize};

use super::{FileSystem, MailId, MiniKind};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Hash, Serialize, Deserialize)]
pub enum StoryStage {
    #[default]
    WaitMail, // 첫 메일이 오기를 기다린다
    Install,    // 친구가 보낸 게임을 받아 설치한다
    TalkToDoor, // 게임을 열어 문과 이야기한다
    Letter,     // 편지를 우편함에 넣고 방을 살펴본다
    Maze,       // 서브 게임: 미로, 화분, 열쇠, 씨앗
    Flower,     // 꽃을 피우고(시간) 보내고, 문에게 꽃을 준다
    Jungle,     // 2층: 소파와 이야기한다
    Missions,   // 눈 + 소환 조각 둘을 모은다
    Exchange,   // 재료 셋을 소파에게 건넨다
    Feed,       // 봉투를 열어 계단에게 먹인다
    Done,       // 지금 있는 이야기의 끝
}

impl StoryStage {
    pub const ALL: [StoryStage; 11] = [
        StoryStage::WaitMail,
        StoryStage::Install,
        StoryStage::TalkToDoor,
        StoryStage::Letter,
        StoryStage::Maze,
        StoryStage::Flower,
        StoryStage::Jungle,
        StoryStage::Missions,
        StoryStage::Exchange,
        StoryStage::Feed,
        StoryStage::Done,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).expect("ALL 에 모든 단계가 있다")
    }
}


// 지금 이야기 단계.
pub fn story_stage(fs: &FileSystem) -> StoryStage {
    let d = &fs.doors_progress;
    if d.floor2_done {
        return StoryStage::Done;
    }
    // 2층(정글) — 1장을 끝낸 뒤.
    if d.tutorial_done {
        if !d.errand_accepted {
            return StoryStage::Jungle;
        }
        if !d.exchanged {
            let all = MiniKind::ALL.iter().all(|&k| d.missions[k.index()].has_material(k));
            return if all { StoryStage::Exchange } else { StoryStage::Missions };
        }
        return StoryStage::Feed;
    }
    // 1장 — 설치 → 문 → 편지 → 서브 게임(미로, 꽃) → 꽃 전달.
    if !fs.game_installed {
        return if fs.has_mail(MailId::Friend) { StoryStage::Install } else { StoryStage::WaitMail };
    }
    if !d.quest_accepted {
        return StoryStage::TalkToDoor;
    }
    if !d.flower_absence_checked {
        return StoryStage::Letter;
    }
    if !fs.maze_progress.planted {
        return StoryStage::Maze;
    }
    StoryStage::Flower
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::MissionStep;

    #[test]
    fn the_story_walks_through_every_step_in_order() {
        let mut fs = FileSystem::new();
        assert_eq!(story_stage(&fs), StoryStage::WaitMail);
        fs.deliver_mail(MailId::Friend);
        assert_eq!(story_stage(&fs), StoryStage::Install);
        fs.game_installed = true;
        assert_eq!(story_stage(&fs), StoryStage::TalkToDoor);
        fs.doors_progress.quest_accepted = true;
        assert_eq!(story_stage(&fs), StoryStage::Letter);
        fs.doors_progress.flower_absence_checked = true;
        assert_eq!(story_stage(&fs), StoryStage::Maze);
        fs.maze_progress.planted = true;
        assert_eq!(story_stage(&fs), StoryStage::Flower);
        fs.doors_progress.tutorial_done = true;
        assert_eq!(story_stage(&fs), StoryStage::Jungle);
        fs.doors_progress.errand_accepted = true;
        assert_eq!(story_stage(&fs), StoryStage::Missions);
        fs.doors_progress.missions = [MissionStep::Received, MissionStep::Fragment, MissionStep::Fragment];
        assert_eq!(story_stage(&fs), StoryStage::Exchange);
        fs.doors_progress.exchanged = true;
        assert_eq!(story_stage(&fs), StoryStage::Feed);
        fs.doors_progress.floor2_done = true;
        assert_eq!(story_stage(&fs), StoryStage::Done);
    }

    #[test]
    fn a_missing_material_keeps_the_player_on_the_missions_step() {
        let mut fs = FileSystem::new();
        fs.doors_progress.tutorial_done = true;
        fs.doors_progress.errand_accepted = true;
        // 눈과 레이싱 조각은 있지만 리듬은 변기 손잡이(Received)까지만 — 소환 조각(Fragment)이 있어야 재료다.
        fs.doors_progress.missions = [MissionStep::Received, MissionStep::Fragment, MissionStep::Received];
        assert_eq!(story_stage(&fs), StoryStage::Missions);
    }

    #[test]
    fn steps_are_indexed_consistently() {
        for (i, s) in StoryStage::ALL.iter().enumerate() {
            assert_eq!(s.index(), i);
        }
    }
}
