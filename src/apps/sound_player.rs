//! 사운드 플레이어 — 게임 폴더(AppData\test\assets\sound)의 `.wav` 파일을 연다
//! (FileKind::Sound, gamefiles.rs). 소리는 코드로 만들어낸 WAV 라서 열 때 임시 폴더에 한 번
//! 써두고 video.rs 의 `Audio`(WASAPI)로 재생한다 — 그 임시 파일은 게임 폴더가 아니라 플레이어가
//! 볼 일 없는 내부 파일이다. 창에는 파일 이름, 재생/일시정지 버튼, 진행 막대만 있다.
//! 소리 크기는 설정창의 Mp4 Sound/Master 슬라이더(+전체 음소거)를 그대로 따른다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::Settings;
use crate::gamefiles::{sound_wav, wav_duration, SOUND_NAMES};
use crate::platform::video::Audio;
use crate::render::gfx::{Assets, Rect, Renderer};
use crate::strings::{sound_player as s, t};
use crate::ui::*;

use super::{App, AppAction, WinInput};

pub struct SoundApp {
    path: String, // 임시 폴더에 써둔 WAV 의 경로
    duration: f32,
    name: &'static str,
    audio: Option<Audio>,
    elapsed: f32,
    playing: bool,
    last_volume: f32, // 매 프레임 set_volume 호출을 피하기 위한 변경 감지용
    settings: Rc<RefCell<Settings>>,
}

fn volume_of(settings: &Settings) -> f32 {
    if settings.mute_all { 0.0 } else { (settings.mp4_sound * settings.master).clamp(0.0, 1.0) }
}

impl SoundApp {
    pub(super) fn new(idx: usize, settings: Rc<RefCell<Settings>>) -> SoundApp {
        let name = SOUND_NAMES[idx.min(SOUND_NAMES.len() - 1)];
        let wav = sound_wav(idx);
        let dir = std::env::temp_dir().join("palaceos_audio");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(name);
        let _ = std::fs::write(&path, &wav);
        let path = path.to_string_lossy().into_owned();
        let volume = volume_of(&settings.borrow());
        // 열자마자 재생한다.
        let audio = Audio::start(path.clone(), volume, 0.0);
        SoundApp { path, duration: wav_duration(&wav), name, audio, elapsed: 0.0, playing: true, last_volume: volume, settings }
    }

    // 끝까지 재생했거나 멈춘 뒤 다시 처음부터 재생한다.
    fn restart(&mut self) {
        self.audio = Audio::start(self.path.clone(), self.last_volume, 0.0);
        self.elapsed = 0.0;
        self.playing = true;
    }
}

impl App for SoundApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        Some(self.name.to_string())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        let lang = self.settings.borrow().language;
        let volume = volume_of(&self.settings.borrow());
        if volume != self.last_volume {
            if let Some(a) = &self.audio {
                a.set_volume(volume);
            }
            self.last_volume = volume;
        }

        if self.playing {
            self.elapsed += win.dt;
            if self.elapsed >= self.duration {
                // 끝까지 재생했다 — 오디오 스레드를 정리하고 멈춘 상태로.
                self.elapsed = self.duration;
                self.playing = false;
                self.audio = None;
            }
        }

        r.rect(area.x, area.y, area.w, area.h, FACE);
        r.text_clipped(area.x + 10.0, area.y + 8.0, self.name, 0.95, BLACK, area.w - 20.0);

        // 진행 막대 + 시간.
        let bar = Rect::new(area.x + 10.0, area.y + 40.0, area.w - 20.0, 14.0);
        sunken(r, bar.x, bar.y, bar.w, bar.h);
        let frac = (self.elapsed / self.duration).clamp(0.0, 1.0);
        r.rect(bar.x + 2.0, bar.y + 2.0, (bar.w - 4.0) * frac, bar.h - 4.0, NAVY);
        let time = format!("{:.1} / {:.1}s", self.elapsed, self.duration);
        r.text(bar.x, bar.y + 18.0, &time, 0.8, GRAY);

        // 재생 중이면 일시정지, 멈췄으면 재생(끝까지 갔으면 처음부터).
        let (bx, by, bw, bh) = (area.x + 10.0, area.y + area.h - 34.0, 90.0, 24.0);
        let label = if self.playing { t(lang, s::PAUSE) } else { t(lang, s::PLAY) };
        if button(r, bx, by, bw, bh, label, win) {
            if self.playing {
                self.playing = false;
                if let Some(a) = &self.audio {
                    a.set_playing(false);
                }
            } else if self.audio.is_some() && self.elapsed < self.duration {
                self.playing = true;
                if let Some(a) = &self.audio {
                    a.set_playing(true);
                }
            } else {
                self.restart();
            }
        }
        AppAction::None
    }
}
