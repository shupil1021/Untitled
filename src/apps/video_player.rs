//! .mp4 실제 영상 재생 — 진행바/재생 컨트롤 UI + video.rs(Video/Audio 디코드-재생
//! 엔진)를 연결하는 앱.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver};

use miniquad::{FilterMode, MipmapFilterMode, RenderingBackend, TextureId};

use crate::foundation::Settings;
use crate::gfx::{Assets, Color, Rect, Renderer, CELL_H};
use crate::strings::{t, video_player as s};
use crate::ui::*;
use crate::video::{Audio, Video};

use super::{App, AppAction, WinInput};

const VIDEO_BAR_H: f32 = 22.0; // 진행바 줄 높이 (시간 라벨 글자 높이를 여유있게 담음)
const VIDEO_CTRL_H: f32 = 30.0; // 재생 버튼 줄 높이
const VIDEO_PANEL_H: f32 = VIDEO_BAR_H + VIDEO_CTRL_H;
const TIME_SCALE: f32 = 0.8; // 시간 라벨 글자 크기(작게)

// ▶(dir>0) 또는 ◀(dir<0) 삼각형을 w×h 상자 안에 채운다. (세로 막대를 쌓아 근사)
fn draw_tri(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, dir: f32, color: Color) {
    let n = 8;
    let col_w = w / n as f32;
    for i in 0..n {
        let t = i as f32 / (n as f32 - 1.0);
        let hh = h * (1.0 - t) / 2.0;
        let cx = if dir > 0.0 { x + i as f32 * col_w } else { x + w - (i as f32 + 1.0) * col_w };
        r.rect(cx, y + h / 2.0 - hh, col_w + 0.5, hh * 2.0, color);
    }
}

// ⏸ 아이콘: 세로 막대 두 개.
fn draw_pause_icon(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let bar_w = w * 0.32;
    let gap = w * 0.2;
    r.rect(x + w / 2.0 - gap / 2.0 - bar_w, y, bar_w, h, color);
    r.rect(x + w / 2.0 + gap / 2.0, y, bar_w, h, color);
}

// ⏪/⏩ 아이콘: 작은 삼각형 두 개.
fn draw_skip_icon(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, dir: f32, color: Color) {
    let tw = w * 0.42;
    let gap = w * 0.14;
    draw_tri(r, x, y, tw, h, dir, color);
    draw_tri(r, x + tw + gap, y, tw, h, dir, color);
}

// ↻ 다시재생 아이콘: 원형 호(점으로 근사) + 끝에 작은 화살촉.
fn draw_replay_icon(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let rad = w.min(h) / 2.0;
    const N: i32 = 16;
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let ang = std::f32::consts::FRAC_PI_2 + t * std::f32::consts::PI * 1.5; // 270도짜리 호
        let px = cx + ang.cos() * rad - 1.0;
        let py = cy + ang.sin() * rad - 1.0;
        r.rect(px, py, 2.0, 2.0, color);
    }
    // 호의 끝점에 화살촉을 붙여 방향을 알려준다.
    let end_ang = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * 1.5;
    let ex = cx + end_ang.cos() * rad;
    let ey = cy + end_ang.sin() * rad;
    draw_tri(r, ex - 3.5, ey - 3.5, 7.0, 7.0, 1.0, color);
}

// 아이콘 버튼: ui::button 과 동일한 베벨/클릭 판정이지만 텍스트 대신 그리기 콜백을 쓴다.
fn icon_button(
    r: &mut Renderer,
    x: f32, y: f32, w: f32, h: f32,
    win: &WinInput,
    draw: impl FnOnce(&mut Renderer, f32, f32, f32, f32, Color),
) -> bool {
    let (mx, my) = win.mouse;
    let hover = mx >= x && mx < x + w && my >= y && my < y + h;
    let pressed = hover && win.mouse_down;
    if pressed {
        sunken(r, x, y, w, h);
    } else {
        raised(r, x, y, w, h);
    }
    let pad = w.min(h) * 0.26;
    let off = if pressed { 1.0 } else { 0.0 };
    draw(r, x + pad + off, y + pad + off, w - pad * 2.0, h - pad * 2.0, BLACK);
    hover && win.mouse_clicked
}

pub struct VideoApp {
    video: Option<Video>,
    audio: Option<Audio>,
    tex: Option<TextureId>,
    playing: bool,
    scrubbing: bool, // 진행바를 드래그로 잡고 있는 중인지
    settings: Rc<RefCell<Settings>>,
    last_volume: f32,     // 매 프레임 set_volume 호출을 피하기 위한 변경 감지용
    last_weathering: f32, // 매 프레임 set_weathering 호출을 피하기 위한 변경 감지용
    abs_path: Option<String>, // video_rx 로부터 결과를 받으면 오디오를 이걸로 시작한다
    video_rx: Option<Receiver<Option<Video>>>, // 백그라운드에서 여는 중이면 Some
}

impl VideoApp {
    pub(super) fn new(settings: Rc<RefCell<Settings>>) -> VideoApp {
        // assets/movie.mp4 를 런타임에 읽어 재생 (파일만 교체하면 재빌드 없이 반영).
        // Windows Media Foundation 으로 디코딩하므로 어떤 프로파일이든 재생된다.
        // 실행 위치(cwd)나 exe 위치 어느 쪽이든 찾도록 여러 후보를 시도.
        let mut candidates: Vec<std::path::PathBuf> = vec!["assets/movie.mp4".into()];
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            candidates.push(dir.join("assets/movie.mp4"));
            candidates.push(dir.join("../../assets/movie.mp4")); // target/debug → 루트
        }
        let abs_path = candidates
            .into_iter()
            .find(|p| p.exists())
            .and_then(|p| std::path::absolute(&p).ok()) // MF 는 절대 경로 필요
            .and_then(|p| p.to_str().map(str::to_string));
        let s = settings.borrow();
        let volume = if s.mute_all { 0.0 } else { (s.mp4_sound * s.master).clamp(0.0, 1.0) };
        let weathering = s.weathering;
        drop(s);

        // 창은 바로 뜨고, 실제 파일 열기(MF 리더 생성 + 첫 프레임 디코드)는 백그라운드
        // 스레드에서 한다 — 이게 동기로 돌면 딱 그 시간만큼 창이 늦게 뜨는 것처럼
        // 느껴진다. 로딩 중엔 update() 에서 스피너를 그려서 보여준다.
        let video_rx = abs_path.clone().map(|path| {
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(Video::from_file(&path));
            });
            rx
        });

        VideoApp {
            video: None,
            audio: None,
            tex: None,
            playing: true,
            scrubbing: false,
            settings,
            last_volume: volume,
            last_weathering: weathering,
            abs_path,
            video_rx,
        }
    }

    fn seek(&mut self, delta_secs: f32) {
        if let Some(v) = &mut self.video {
            v.seek(delta_secs);
            self.sync_audio_pos();
        }
    }

    fn seek_to(&mut self, pos_100ns: i64) {
        if let Some(v) = &mut self.video {
            v.seek_to(pos_100ns);
            self.sync_audio_pos();
        }
    }

    fn sync_audio_pos(&self) {
        if let (Some(v), Some(a)) = (&self.video, &self.audio) {
            a.seek_to(v.position_100ns());
        }
    }

    fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
        if let Some(a) = &self.audio {
            a.set_playing(playing);
        }
    }
}

// 100ns 단위 시간을 "M:SS" (1시간 넘으면 "H:MM:SS") 로 표시.
fn format_time(v100ns: i64) -> String {
    let secs = (v100ns.max(0) / 10_000_000) as u64;
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

impl App for VideoApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        // 백그라운드에서 열던 영상이 준비됐으면 반영하고, 그제서야 오디오를 시작한다
        // (오디오도 자체 스레드에서 도니 여기서 시작해도 화면이 안 멎는다).
        if let Some(rx) = &self.video_rx
            && let Ok(result) = rx.try_recv()
        {
            self.video = result;
            self.video_rx = None;
            if self.video.is_some()
                && let Some(path) = self.abs_path.clone()
            {
                self.audio = Audio::start(path, self.last_volume, self.last_weathering);
            }
        }

        // 설정창에서 Mp4 Sound/Master/Weathering 슬라이더를 조절하면 재생 중에도 바로 반영.
        if let Some(a) = &self.audio {
            let s = self.settings.borrow();
            let volume = if s.mute_all { 0.0 } else { (s.mp4_sound * s.master).clamp(0.0, 1.0) };
            let weathering = s.weathering;
            drop(s);
            if volume != self.last_volume {
                a.set_volume(volume);
                self.last_volume = volume;
            }
            if weathering != self.last_weathering {
                a.set_weathering(weathering);
                self.last_weathering = weathering;
            }
        }

        // 디코드 진행 + 텍스처 갱신
        let mut just_ended = false;
        if let Some(v) = &mut self.video {
            v.advance(win.dt, self.playing && !self.scrubbing); // 슬라이더 드래그 중엔 정지
            if v.ended && self.playing {
                just_ended = true;
            }
            match self.tex {
                None => {
                    let t = ctx.new_texture_from_rgba8(v.w as u16, v.h as u16, &v.rgba);
                    ctx.texture_set_filter(t, FilterMode::Linear, MipmapFilterMode::None);
                    self.tex = Some(t);
                    v.dirty = false;
                }
                Some(t) if v.dirty => {
                    ctx.texture_update(t, &v.rgba);
                    v.dirty = false;
                }
                _ => {}
            }
        }
        if just_ended {
            // 끝까지 재생됨 — 자동으로 멈추고(반복 재생 안 함), 오디오도 같이 멈춘다.
            self.set_playing(false);
        }
        let ended = self.video.as_ref().is_some_and(|v| v.ended);

        let vh = area.h - VIDEO_PANEL_H;
        r.rect(area.x, area.y, area.w, vh, BLACK);
        if let (Some(tex), Some(v)) = (self.tex, &self.video) {
            // 비율 유지하며 영역에 맞춤
            let scale = (area.w / v.w as f32).min(vh / v.h as f32);
            let dw = v.w as f32 * scale;
            let dh = v.h as f32 * scale;
            let dx = area.x + (area.w - dw) / 2.0;
            let dy = area.y + (vh - dh) / 2.0;
            r.sprite(tex, dx, dy, dw, dh, WHITE);
            if v.ended {
                // 끝난 화면임을 알 수 있게 살짝 어둡게 톤을 깐다.
                r.rect(dx, dy, dw, dh, [0.0, 0.0, 0.0, 0.45]);
            }
        } else if self.video_rx.is_some() {
            // 아직 백그라운드에서 여는 중 — 스피너로 로딩 중임을 보여준다.
            draw_spinner(r, area.x + area.w / 2.0, area.y + vh / 2.0, 14.0, win.time);
        } else {
            let lang = self.settings.borrow().language;
            label(r, area.x + 10.0, area.y + 12.0, t(lang, s::NO_VIDEO), WHITE);
            label(r, area.x + 10.0, area.y + 34.0, t(lang, s::PUT_VIDEO_AT), GRAY);
            label(r, area.x + 10.0, area.y + 52.0, "assets/movie.mp4", GRAY);
        }

        let by = area.y + vh;
        panel(r, area.x, by, area.w, VIDEO_PANEL_H);

        // ---- 진행바 줄: 현재시간 | 게이지(드래그로 탐색) | 총시간 ----
        // 라벨 너비를 실측해서 그 뒤에 바를 배치하므로 텍스트와 바가 절대 겹치지 않는다.
        if let Some(v) = &self.video {
            let (pos, dur) = (v.position_100ns(), v.duration_100ns());
            let pad = 8.0;
            let gap = 6.0;
            let bar_h = 6.0;
            let text_y = by + (VIDEO_BAR_H - CELL_H * TIME_SCALE) / 2.0;
            let bar_y = by + (VIDEO_BAR_H - bar_h) / 2.0;

            let dur_label = format_time(dur.max(pos));
            let dur_w = r.text_width(&dur_label, TIME_SCALE);
            r.text(area.x + area.w - pad - dur_w, text_y, &dur_label, TIME_SCALE, WHITE);

            // 왼쪽 라벨 자리는 "지금 위치" 글자 폭이 아니라 "총 길이" 글자 폭으로
            // 예약한다 — 재생 중인 pos 는 매초 자릿수가 바뀔 수 있어서(0:09→0:10 은
            // 안 바뀌지만 9:59→10:00 처럼 자릿수가 늘어나는 순간 등) 그걸 기준으로
            // 바 위치를 잡으면 재생/탐색 중에 바 자체가 계속 좌우로 흔들리고, 드래그
            // 스크럽 도중엔 그 흔들림이 위치 계산에 또 들어가서 위치가 튀는 것처럼
            // 느껴진다. dur 는 같은 영상을 재생하는 동안 사실상 고정값이라 이걸
            // 기준으로 하면 바가 완전히 제자리에 고정된다.
            let pos_label = format_time(pos);
            let pos_w = dur_w;
            r.text(area.x + pad, text_y, &pos_label, TIME_SCALE, WHITE);

            let bar_x = area.x + pad + pos_w + gap;
            let bar_w = (area.w - 2.0 * pad - pos_w - dur_w - 2.0 * gap).max(20.0);

            sunken(r, bar_x, bar_y, bar_w, bar_h);
            let frac = if dur > 0 { (pos as f64 / dur as f64).clamp(0.0, 1.0) as f32 } else { 0.0 };
            // 이미 재생한 구간을 파란색으로 채운다.
            let played_w = frac * bar_w - 2.0;
            if played_w > 0.0 {
                r.rect(bar_x + 1.0, bar_y + 1.0, played_w, bar_h - 2.0, NAVY);
            }
            let thumb_w = 6.0;
            let thumb_x = bar_x + frac * (bar_w - thumb_w).max(0.0);
            raised(r, thumb_x, bar_y - 2.0, thumb_w, bar_h + 4.0);

            // 진행바 클릭/드래그로 탐색. 클릭 순간 바로 이동하고(짧은 클릭도 동작해야
            // 하므로), 이후 누른 채 움직이면 계속 따라간다(드래그 스크럽).
            let grab = Rect::new(bar_x - 2.0, bar_y - 6.0, bar_w + 4.0, bar_h + 12.0);
            if win.mouse_clicked && grab.contains(win.mouse.0, win.mouse.1) {
                self.scrubbing = true;
                // 드래그로 위치를 바꾸는 동안은 소리가 튀지 않도록 잠시 음소거.
                if let Some(a) = &self.audio {
                    a.set_playing(false);
                }
            }
            if self.scrubbing {
                if dur > 0 {
                    let f = ((win.mouse.0 - bar_x) / bar_w).clamp(0.0, 1.0);
                    self.seek_to((f as f64 * dur as f64) as i64);
                }
                if !win.mouse_down {
                    self.scrubbing = false;
                    // 원래 재생 상태로 오디오 복귀 (재생 중이었으면 다시 소리 남).
                    if let Some(a) = &self.audio {
                        a.set_playing(self.playing);
                    }
                }
            }
        }

        // ---- 재생 버튼 줄: ⏪5s / ▶‖⏸(가운데, 크게) / ⏩5s (아이콘 버튼) ----
        let crow_y = by + VIDEO_BAR_H;
        let btn_h = VIDEO_CTRL_H - 6.0;
        let big_w = 36.0;
        let side_w = 30.0;
        let cx = area.x + area.w / 2.0;
        let by_row = crow_y + 3.0;

        if icon_button(r, cx - big_w / 2.0 - side_w - 4.0, by_row, side_w, btn_h, win, |r, x, y, w, h, c| {
            draw_skip_icon(r, x, y, w, h, -1.0, c)
        }) {
            self.seek(-5.0);
        }
        let playing = self.playing;
        if icon_button(r, cx - big_w / 2.0, by_row, big_w, btn_h, win, move |r, x, y, w, h, c| {
            if playing {
                draw_pause_icon(r, x, y, w, h, c);
            } else if ended {
                draw_replay_icon(r, x, y, w, h, c);
            } else {
                draw_tri(r, x, y, w, h, 1.0, c);
            }
        }) {
            if ended {
                self.seek_to(0);
                self.set_playing(true);
            } else {
                self.set_playing(!self.playing);
            }
        }
        if icon_button(r, cx + big_w / 2.0 + 4.0, by_row, side_w, btn_h, win, |r, x, y, w, h, c| {
            draw_skip_icon(r, x, y, w, h, 1.0, c)
        }) {
            self.seek(5.0);
        }
        AppAction::None
    }
}
