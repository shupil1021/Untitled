//! Windows 가 아닌 곳의 영상/오디오 대체 구현 — 디코더가 없으니 항상 열기에 실패(None)한다.
//! 호출하는 앱(video_player/sound_player)이 None 을 "재생 불가"로 처리한다.

pub struct Video {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
    pub dirty: bool,
    pub ended: bool,
}

impl Video {
    pub fn from_file(_path: &str) -> Option<Video> {
        None
    }

    pub fn advance(&mut self, _dt: f32, _playing: bool) {}

    pub fn seek(&mut self, _delta_secs: f32) {}

    pub fn seek_to(&mut self, _pos_100ns: i64) {}

    pub fn position_100ns(&self) -> i64 {
        0
    }

    pub fn duration_100ns(&self) -> i64 {
        0
    }
}

pub struct Audio;

impl Audio {
    pub fn start(_path: String, _volume: f32, _weathering: f32) -> Option<Audio> {
        None
    }

    pub fn set_playing(&self, _p: bool) {}

    pub fn set_volume(&self, _v: f32) {}

    pub fn set_weathering(&self, _w: f32) {}

    pub fn seek_to(&self, _pos_100ns: i64) {}
}
