//! 메인 게임의 효과음 — 코드로 만든 WAV(gamefiles/sounds.rs)를 임시 폴더에 써서 한 번 재생한다.
//! 퀘스트 시작(방귀 + 중세 나팔, MAIN H-2)과 변기 물 내리기(MAIN H-6). 소리 크기는 설정을 따른다.

use std::cell::RefCell;
use std::rc::Rc;

use crate::foundation::Settings;
use crate::gamefiles::{flush_wav, quest_start_wav, wav_duration};
use crate::platform::video::Audio;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Cue {
    QuestStart,
    Flush,
}

#[derive(Default)]
pub(super) struct Sfx {
    audio: Option<Audio>,
    left: f32, // 재생이 끝날 때까지 남은 시간(끝나면 오디오 스레드를 정리한다)
}

impl Sfx {
    pub(super) fn update(&mut self, cue: Option<Cue>, dt: f32, settings: Option<&Rc<RefCell<Settings>>>) {
        if self.audio.is_some() {
            self.left -= dt;
            if self.left <= 0.0 {
                self.audio = None;
            }
        }
        let (Some(cue), Some(settings)) = (cue, settings) else { return };
        let (name, wav) = match cue {
            Cue::QuestStart => ("quest_start.wav", quest_start_wav()),
            Cue::Flush => ("flush.wav", flush_wav()),
        };
        let dir = std::env::temp_dir().join("palaceos_audio");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        if std::fs::write(&path, &wav).is_err() {
            return;
        }
        let volume = super::bgm::volume_of(&settings.borrow());
        self.audio = Audio::start(path.to_string_lossy().into_owned(), volume, 0.0);
        self.left = wav_duration(&wav) + 0.4;
    }
}
