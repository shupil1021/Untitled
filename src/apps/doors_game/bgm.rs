//! 시작 방의 배경음악 — assets/audio/bgm.wav 를 무한 반복한다. 시작 방에 있는 동안만 울리고, 다른 곳으로 가면
//! 멈춘다. 소리 크기는 설정창의 Mp4 Sound/Master 슬라이더(+전체 음소거)를 따른다. 파일은 실행 파일에 내장돼
//! 있어서(Media Foundation 이 경로를 필요로 하므로) 처음 틀 때 임시 폴더에 한 번 써둔다.

use std::cell::RefCell;
use std::rc::Rc;

use crate::foundation::Settings;
use crate::platform::video::Audio;

const BGM_WAV: &[u8] = include_bytes!("../../../assets/audio/bgm.wav");

#[derive(Default)]
pub(super) struct Bgm {
    audio: Option<Audio>,
    volume: f32,
}

pub(super) fn volume_of(settings: &Settings) -> f32 {
    if settings.mute_all { 0.0 } else { (settings.mp4_sound * settings.master).clamp(0.0, 1.0) }
}

fn extract() -> Option<String> {
    let dir = std::env::temp_dir().join("palaceos_audio");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("bgm.wav");
    if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(BGM_WAV.len() as u64) {
        std::fs::write(&path, BGM_WAV).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}

impl Bgm {
    // 매 프레임 — want 가 켜져 있으면 틀고(이미 틀고 있으면 그대로) 꺼지면 멈춘다. settings 가 없으면(테스트 등) 안 튼다.
    pub(super) fn update(&mut self, want: bool, settings: Option<&Rc<RefCell<Settings>>>) {
        let Some(settings) = settings else { return };
        if !want {
            self.audio = None;
            return;
        }
        let volume = volume_of(&settings.borrow());
        match &self.audio {
            Some(a) if volume != self.volume => a.set_volume(volume),
            Some(_) => {}
            None => self.audio = extract().and_then(|p| Audio::start_looping(p, volume)),
        }
        self.volume = volume;
    }
}
