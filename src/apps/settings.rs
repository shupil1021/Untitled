//! 설정 (탭: Graphics / Audio / Interface).

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::RenderingBackend;

use crate::foundation::{Language, Settings, BG_COLORS, FPS_OPTS, RES_OPTS, TABS};
use crate::gfx::{Assets, Rect, Renderer, CELL_H};
use crate::strings::{common, settings as s, t};
use crate::ui::*;

use super::widgets::{accordion_list, draw_slider, ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

// TABS(foundation.rs) 의 한국어/일본어 번역 — TABS 는 순전히 표시용 상수라 다른
// 곳에서 원문 문자열에 기대는 코드가 없어서, 그냥 여기 나란히 두고 언어에 따라 골라 쓴다.
const KO_TABS: [&str; 3] = ["그래픽", "오디오", "인터페이스"];
const JA_TABS: [&str; 3] = ["グラフィック", "オーディオ", "インターフェース"];

// 언어 선택지 이름 — Resolution/Frame rate 아코디언과 똑같은 accordion_header/
// accordion_list 조합으로 그리려고, 세 언어 이름을 "지금 UI 언어"에 상관없이
// 항상 그 언어 자신의 이름으로 보여준다(예: 한국어 UI에서도 "English"/"日本語"
// 라고 써 있어야 사용자가 못 읽는 언어에서도 자기 언어를 찾아 돌아올 수 있다).
const LANG_NAMES: [&str; 3] = ["English", "한국어", "日本語"];

pub struct SettingsApp {
    settings: Rc<RefCell<Settings>>,
    show_interface: bool, // false 면 Interface 탭을 아예 안 보여준다 (로비의 Settings 패널용)
    tab: usize,
    active_slider: i32,
    video_scroll: f32,      // Video 탭: 목표 스크롤(다 안 보일 때 스크롤된 픽셀 양)
    video_scroll_disp: f32, // 화면표시용(smooth 켜져 있으면 목표를 부드럽게 따라감)
    video_sb_drag: bool,
    res_expanded: bool, // Graphics 탭: Resolution 아코디언이 펼쳐져 있는지
    fps_expanded: bool, // Graphics 탭: Frame rate 아코디언이 펼쳐져 있는지
    graphics_scroll: f32,
    graphics_scroll_disp: f32,
    graphics_sb_drag: bool,
    res_list_scroll: f32, // Resolution 아코디언 리스트 자체의 내부 스크롤(항목이 많아서)
    res_list_scroll_disp: f32,
    res_list_sb_drag: bool,
    fps_list_scroll: f32,
    fps_list_scroll_disp: f32,
    fps_list_sb_drag: bool,
    lang_expanded: bool, // Interface 탭: Language 아코디언이 펼쳐져 있는지
    lang_list_scroll: f32,
    lang_list_scroll_disp: f32,
    lang_list_sb_drag: bool,
    interface_scroll: f32, // Interface 탭 전체 스크롤(Graphics 탭과 같은 구조)
    interface_scroll_disp: f32,
    interface_sb_drag: bool,
    has_data: bool, // Erase All Memory 버튼 활성화 여부 — 지울 저장 데이터가 있을 때만 눌리게 한다
}

impl SettingsApp {
    pub fn new(settings: Rc<RefCell<Settings>>) -> SettingsApp {
        // 데스크톱 안에서 여는 설정창은 이미 세이브가 로드된 상태에서만 뜨므로
        // 항상 지울 데이터가 있다.
        Self::new_with_tabs(settings, true, true)
    }

    // 로비의 Settings 패널처럼 바탕화면 색상 등 데스크톱 전용 취향 설정(Interface 탭)이
    // 의미 없는 자리에서 쓸 때 그 탭을 아예 감출 수 있게 하던 옵션이었지만, 지금은
    // 언어 설정(Interface 탭 안)을 로비에서도 볼 수 있어야 해서 로비도 show_interface=true
    // 로 연다 — 매개변수 자체는 나중에 정말 숨겨야 할 자리가 생길 수 있어 남겨뒀다.
    // has_data 는 지울 저장 데이터가 실제로 있는지(로비는 Continue 활성화 여부와 같음).
    pub fn new_with_tabs(settings: Rc<RefCell<Settings>>, show_interface: bool, has_data: bool) -> SettingsApp {
        SettingsApp {
            settings,
            show_interface,
            tab: 0,
            active_slider: -1,
            video_scroll: 0.0,
            video_scroll_disp: 0.0,
            video_sb_drag: false,
            res_expanded: false,
            fps_expanded: false,
            graphics_scroll: 0.0,
            graphics_scroll_disp: 0.0,
            graphics_sb_drag: false,
            res_list_scroll: 0.0,
            res_list_scroll_disp: 0.0,
            res_list_sb_drag: false,
            fps_list_scroll: 0.0,
            fps_list_scroll_disp: 0.0,
            fps_list_sb_drag: false,
            lang_expanded: false,
            lang_list_scroll: 0.0,
            lang_list_scroll_disp: 0.0,
            lang_list_sb_drag: false,
            interface_scroll: 0.0,
            interface_scroll_disp: 0.0,
            interface_sb_drag: false,
            has_data,
        }
    }
}

impl App for SettingsApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn title(&self) -> Option<String> {
        let lang = self.settings.borrow().language;
        Some(t(lang, s::TITLE).to_string())
    }

    fn update(&mut self, _ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        if !win.mouse_down {
            self.active_slider = -1;
        }
        r.rect(area.x, area.y, area.w, area.h, FACE);

        let mut s = self.settings.borrow_mut();
        let lang = s.language;

        // 탭 바 — 오른쪽에 빈 공간이 안 남게, 창 너비에 맞춰 탭 너비를 늘려서 꽉 채운다.
        let all_tabs: &[&str] = match lang {
            Language::Ko => &KO_TABS,
            Language::Ja => &JA_TABS,
            Language::En => &TABS,
        };
        let tabs: &[&str] = if self.show_interface { all_tabs } else { &all_tabs[..2] };
        const TAB_GAP: f32 = 2.0;
        let tw = ((area.w - 12.0 - TAB_GAP * (tabs.len() - 1) as f32) / tabs.len() as f32).max(60.0);
        for (i, t) in tabs.iter().enumerate() {
            let bx = area.x + 6.0 + i as f32 * (tw + TAB_GAP);
            let by = area.y + 6.0;
            if i == self.tab {
                sunken(r, bx, by, tw, 22.0);
            } else {
                raised(r, bx, by, tw, 22.0);
            }
            let ttw = r.text_width(t, 0.9);
            r.text(bx + (tw - ttw) / 2.0, by + 3.0, t, 0.9, BLACK);
            if win.mouse_clicked && Rect::new(bx, by, tw, 22.0).contains(win.mouse.0, win.mouse.1) {
                self.tab = i;
            }
        }

        let cx = area.x + 14.0;
        let cy = area.y + 42.0;
        match self.tab {
            0 => {
                // Graphics: 레퍼런스의 "Options"/"Reserved drive letters" 같은 그룹박스
                // 느낌으로 두 묶음("Display"=해상도/주사율 콤보박스 2개, "CRT Effects"=
                // 슬라이더 3개)으로 나눠서 그린다. 해상도/주사율은 옵션이 많아서(9개)
                // 콤보박스를 펼치면 안에 리스트가 나오는 아코디언 그대로 쓰되, 이제
                // accordion_header 자체가 필드+화살표 버튼이 분리된 진짜 콤보박스
                // 모양이라 그룹박스 안에 넣어도 위화감이 없다.
                const GAP: f32 = 8.0; // 그룹박스 사이 간격
                const SLIDER_ROW_H: f32 = 46.0;
                const SB_RESERVE: f32 = 16.0; // 스크롤바 폭(8) + 여백 — 항상 미리 자리를 비워둬서 나타날 때 안 겹치게 한다.
                const BOX_PAD: f32 = 10.0; // 그룹박스 안쪽 좌우 여백
                // 그룹박스 라벨은 테두리 위 선에 걸쳐 그려지므로(group_box 참고), 그
                // 라벨과 첫 컨트롤이 겹치지 않으려면 테두리 아래로 이만큼은 띄워야
                // 한다 — 처음엔 이 간격이 없어서 "Display"/"CRT Effects" 라벨이 바로
                // 아래 컨트롤과 겹쳐 보였다.
                const BOX_TOP_INSET: f32 = 16.0;
                const BOX_BOTTOM_PAD: f32 = 8.0;
                const BOX_TOP_MARGIN: f32 = 12.0; // 그룹박스 라벨이 위 테두리에 걸치는 만큼 위쪽에 미리 비워둘 여백
                let header_h = ACCORDION_HEADER_H;
                let row_h = ACCORDION_ROW_H;
                let list_w = (area.w - 28.0 - SB_RESERVE - BOX_PAD * 2.0).max(120.0);
                let bx = cx + BOX_PAD; // 그룹박스 안쪽 콘텐츠의 x
                let viewport_top = cy;
                let viewport_h = (area.y + area.h - 40.0 - viewport_top).max(0.0);

                // 리스트 자체가 내부 스크롤을 가지므로(최대 4줄까지만 펼침) 전체 높이
                // 계산에도 그 상한을 그대로 쓴다. 아코디언 줄 수를 줄이는 대신, 둘 다
                // 펼쳐서 다 안 들어가면 그냥 Graphics 탭 전체를 스크롤해서 보여준다.
                const MAX_LIST_ROWS: usize = 4;
                // accordion_list 가 안쪽에 ACCORDION_LIST_PAD 만큼 여백을 두고 그리므로
                // (sunken 테두리가 가려지지 않도록) 바깥 레이아웃도 그만큼 더 잡아줘야 겹치지 않는다.
                let list_pad2 = ACCORDION_LIST_PAD * 2.0;
                let res_list_h = if self.res_expanded { RES_OPTS.len().min(MAX_LIST_ROWS) as f32 * row_h + list_pad2 } else { 0.0 };
                let fps_list_h = if self.fps_expanded { FPS_OPTS.len().min(MAX_LIST_ROWS) as f32 * row_h + list_pad2 } else { 0.0 };

                // "Display" 그룹박스: 콤보박스 두 개(펼쳐지면 그만큼 늘어남).
                let display_content_h = header_h + res_list_h + header_h + fps_list_h;
                let display_box_h = BOX_TOP_INSET + display_content_h + BOX_BOTTOM_PAD;
                // "CRT Effects" 그룹박스: 슬라이더 3개.
                let crt_content_h = SLIDER_ROW_H * 3.0;
                let crt_box_h = BOX_TOP_INSET + crt_content_h + BOX_BOTTOM_PAD;

                let total_h = BOX_TOP_MARGIN + display_box_h + GAP + BOX_TOP_MARGIN + crt_box_h;
                let max_scroll = (total_h - viewport_h).max(0.0);
                // 휠 반영은 아래에서 펼쳐진 리스트들이 이번 프레임에 직접 먹었는지 확인한
                // 뒤에 한다 — 안 그러면 리스트 위에서 굴려도 탭 전체가 같이 스크롤된다.
                self.graphics_scroll = self.graphics_scroll.clamp(0.0, max_scroll);
                ease_scroll(&mut self.graphics_scroll_disp, self.graphics_scroll, win.dt, s.smooth_scroll);
                let mut inner_wheel_used = false;

                let outer_clip = Rect::new(area.x, viewport_top, area.w, viewport_h);
                r.set_clip(Some(outer_clip));
                let mut iy = viewport_top - self.graphics_scroll_disp;
                let visible = |y: f32, h: f32| y + h > viewport_top && y < viewport_top + viewport_h;

                iy += BOX_TOP_MARGIN;
                let display_box_y = iy;
                if visible(display_box_y, display_box_h) {
                    group_box(r, cx, display_box_y, list_w + BOX_PAD * 2.0, display_box_h, t(lang, s::DISPLAY));
                }
                iy += BOX_TOP_INSET;

                if visible(iy, header_h)
                    && accordion_header(
                        r, win, bx, iy, list_w, t(lang, s::RESOLUTION), RES_OPTS[s.res_idx].0, self.res_expanded,
                    )
                {
                    self.res_expanded = !self.res_expanded;
                }
                iy += header_h;
                if self.res_expanded && visible(iy, res_list_h) {
                    let res_names: [&str; RES_OPTS.len()] = std::array::from_fn(|i| RES_OPTS[i].0);
                    let (clicked, _, wheel_used) = accordion_list(
                        r, win, bx, iy, list_w, &res_names, s.res_idx,
                        &mut self.res_list_scroll, &mut self.res_list_scroll_disp, &mut self.res_list_sb_drag, s.smooth_scroll,
                        MAX_LIST_ROWS,
                    );
                    inner_wheel_used |= wheel_used;
                    if let Some(i) = clicked {
                        s.res_idx = i;
                    }
                    r.set_clip(Some(outer_clip)); // accordion_list 가 끝에서 클립을 풀어주므로 되돌려놓는다.
                }
                iy += res_list_h;

                if visible(iy, header_h)
                    && accordion_header(
                        r, win, bx, iy, list_w, t(lang, s::FRAME_RATE), FPS_OPTS[s.fps_idx].0, self.fps_expanded,
                    )
                {
                    self.fps_expanded = !self.fps_expanded;
                }
                iy += header_h;
                if self.fps_expanded && visible(iy, fps_list_h) {
                    let fps_names: [&str; FPS_OPTS.len()] = std::array::from_fn(|i| FPS_OPTS[i].0);
                    let (clicked, _, wheel_used) = accordion_list(
                        r, win, bx, iy, list_w, &fps_names, s.fps_idx,
                        &mut self.fps_list_scroll, &mut self.fps_list_scroll_disp, &mut self.fps_list_sb_drag, s.smooth_scroll,
                        MAX_LIST_ROWS,
                    );
                    inner_wheel_used |= wheel_used;
                    if let Some(i) = clicked {
                        s.fps_idx = i;
                    }
                    r.set_clip(Some(outer_clip));
                }

                // 정확히 박스 바닥으로 이동(중간에 누적한 값들의 오차가 안 쌓이게) 하고 GAP.
                iy = display_box_y + display_box_h + GAP;

                iy += BOX_TOP_MARGIN;
                let crt_box_y = iy;
                if visible(crt_box_y, crt_box_h) {
                    group_box(r, cx, crt_box_y, list_w + BOX_PAD * 2.0, crt_box_h, t(lang, s::CRT_EFFECTS));
                }
                iy += BOX_TOP_INSET;
                if visible(iy, SLIDER_ROW_H) {
                    draw_slider(r, win, bx, iy, 180.0, t(lang, s::CRT_INTENSITY), 0, &mut s.crt_intensity, &mut self.active_slider);
                }
                iy += SLIDER_ROW_H;
                if visible(iy, SLIDER_ROW_H) {
                    draw_slider(
                        r, win, bx, iy, 180.0, t(lang, s::CHROMATIC_ABERRATION), 1, &mut s.chromatic_aberration,
                        &mut self.active_slider,
                    );
                }
                iy += SLIDER_ROW_H;
                if visible(iy, SLIDER_ROW_H) {
                    draw_slider(r, win, bx, iy, 180.0, t(lang, s::CURSOR_SIZE), 2, &mut s.cursor_scale, &mut self.active_slider);
                }
                r.set_clip(None);

                // 펼쳐진 리스트가 이번 프레임에 휠을 안 먹었고, 마우스가 이 뷰포트
                // 위에 있을 때만 탭 전체를 스크롤한다 — 안 그러면 OK 버튼 등 탭 바깥
                // 다른 컨트롤 위에서 굴려도 이 탭이 같이 스크롤돼버린다.
                if !inner_wheel_used && outer_clip.contains(win.mouse.0, win.mouse.1) {
                    self.graphics_scroll = (self.graphics_scroll - win.wheel / 120.0 * 24.0).clamp(0.0, max_scroll);
                }

                if max_scroll > 0.0 {
                    let sb_x = cx + list_w + BOX_PAD * 2.0 + 4.0;
                    let frac = viewport_h / total_h;
                    scrollbar(
                        r, win, sb_x, viewport_top, 8.0, viewport_h, frac,
                        self.graphics_scroll_disp, &mut self.graphics_scroll, max_scroll, &mut self.graphics_sb_drag,
                    );
                }
            }
            1 => {
                // Audio: "Volume" 그룹박스(SFX/BGM/Mp4 Sound/MASTER) + "Effects" 그룹박스
                // (Weathering 음질 효과 + Mute All). 창 높이보다 많아서 다 안 들어가므로,
                // 세로로 스크롤되는 목록으로 그린다.
                const ROW_H: f32 = 46.0;
                const GAP: f32 = 8.0; // 그룹박스 사이 간격
                const BOX_PAD: f32 = 10.0;
                const BOX_TOP_INSET: f32 = 16.0; // 그룹박스 라벨과 첫 컨트롤이 안 겹치게 테두리 아래로 띄우는 간격
                const BOX_BOTTOM_PAD: f32 = 8.0;
                const BOX_TOP_MARGIN: f32 = 12.0;
                let box_w = (area.w - 28.0).max(220.0);
                let bx = cx + BOX_PAD;
                let viewport_top = cy;
                let viewport_h = (area.y + area.h - 40.0 - viewport_top).max(0.0);

                // RefMut<Settings> 를 통해선 필드별로 나눠 빌릴 수 없으니, 한 번 &mut Settings
                // 로 리보로우해서 그 위에서 서로 다른 필드를 동시에 빌린다.
                let sref: &mut Settings = &mut s;
                let volume_sliders: [(&str, &mut f32); 4] = [
                    (t(lang, s::SFX), &mut sref.sfx),
                    (t(lang, s::BGM), &mut sref.bgm),
                    (t(lang, s::MP4_SOUND), &mut sref.mp4_sound),
                    (t(lang, s::MASTER), &mut sref.master),
                ];
                let n_volume = volume_sliders.len();
                let volume_box_h = BOX_TOP_INSET + n_volume as f32 * ROW_H + BOX_BOTTOM_PAD;
                let effects_box_h = BOX_TOP_INSET + ROW_H * 2.0 + BOX_BOTTOM_PAD;
                let total_h = BOX_TOP_MARGIN + volume_box_h + GAP + BOX_TOP_MARGIN + effects_box_h;
                let max_scroll = (total_h - viewport_h).max(0.0);
                // win.wheel 은 winman.rs 에서 이미 마우스가 올라간 창으로 걸러져서 오지만,
                // 이 탭 안에서도 뷰포트 위에 있을 때만 스크롤되게 한 번 더 좁힌다.
                let audio_viewport = Rect::new(area.x, viewport_top, area.w, viewport_h);
                if audio_viewport.contains(win.mouse.0, win.mouse.1) {
                    self.video_scroll -= win.wheel / 120.0 * (ROW_H * 0.6);
                }
                self.video_scroll = self.video_scroll.clamp(0.0, max_scroll);
                ease_scroll(&mut self.video_scroll_disp, self.video_scroll, win.dt, sref.smooth_scroll);

                r.set_clip(Some(Rect::new(area.x, viewport_top, area.w, viewport_h)));
                let base_y = viewport_top - self.video_scroll_disp;
                let visible = |y: f32, h: f32| y + h > viewport_top && y < viewport_top + viewport_h;

                let volume_box_y = base_y + BOX_TOP_MARGIN;
                if visible(volume_box_y, volume_box_h) {
                    group_box(r, cx, volume_box_y, box_w, volume_box_h, t(lang, s::VOLUME));
                }
                let volume_content_y = volume_box_y + BOX_TOP_INSET;
                for (i, (name, val)) in volume_sliders.into_iter().enumerate() {
                    let row_y = volume_content_y + i as f32 * ROW_H;
                    if visible(row_y, ROW_H) {
                        draw_slider(r, win, bx, row_y, 180.0, name, i as i32, val, &mut self.active_slider);
                    }
                }

                let effects_box_y = volume_box_y + volume_box_h + GAP + BOX_TOP_MARGIN;
                if visible(effects_box_y, effects_box_h) {
                    group_box(r, cx, effects_box_y, box_w, effects_box_h, t(lang, s::EFFECTS));
                }
                let weather_y = effects_box_y + BOX_TOP_INSET;
                if visible(weather_y, ROW_H) {
                    // 오디오에 로우패스+히스+크래클을 입혀서 낡은 소리로 들리게 한다
                    // (video.rs 의 Weathering) — 볼륨이 아니라 "소리를 어떻게 낼지" 를
                    // 정하는 음질 효과라 Mute All 과 묶어서 같은 그룹박스에 뒀다. 지금은
                    // 실제로 붙어있는 오디오가 .mp4 뿐이라 그쪽에만 적용되지만, 나중에
                    // 다른 소리가 추가돼도 같이 먹도록 이름을 일반적으로 뒀다.
                    draw_slider(
                        r, win, bx, weather_y, 180.0, t(lang, s::WEATHERING), n_volume as i32, &mut sref.weathering,
                        &mut self.active_slider,
                    );
                }
                let mute_y = weather_y + ROW_H;
                if visible(mute_y, ROW_H) {
                    checkbox(r, bx, mute_y + 14.0, t(lang, s::MUTE_ALL), &mut sref.mute_all, win);
                }
                r.set_clip(None);

                // 스크롤바 (다 안 보일 때만) — 마우스로 드래그해서 스크롤 가능.
                if max_scroll > 0.0 {
                    let sb_x = area.x + area.w - 14.0;
                    let frac = viewport_h / total_h;
                    scrollbar(r, win, sb_x, viewport_top, 8.0, viewport_h, frac, self.video_scroll_disp, &mut self.video_scroll, max_scroll, &mut self.video_sb_drag);
                }
            }
            2 => {
                // Interface: 조작감 + 언어 + 바탕화면 색상은 "Appearance" 그룹박스로
                // 묶고, Erase All Memory 는 위험한 동작이라 그룹박스 밖에 따로 떨어뜨려
                // 둔다(다른 취향 설정과 같은 묶음에 있으면 실수로 옆에 있는 걸 누를
                // 위험이 커 보인다). 언어는 옵션이 3개뿐이라 토글 버튼으로도 충분했지만,
                // Resolution/Frame rate 와 같은 아코디언 콤보박스 모양으로 통일해서
                // "설정값 목록에서 고르는" 느낌을 일관되게 맞췄다. 아코디언이 펼쳐지면
                // 그 아래 배경색 줄이 밀려 내려가야 하므로, Graphics 탭과 같은 구조로
                // 전체를 스크롤 가능한 뷰포트 안에 그린다(창 높이가 작은 로비의 Settings
                // 패널에서도 안 잘리게).
                const GAP: f32 = 8.0;
                const SB_RESERVE: f32 = 16.0;
                const BOX_PAD: f32 = 10.0;
                // 그룹박스 라벨이 테두리 위 선에 걸쳐 그려지므로, 첫 컨트롤이 그 라벨과
                // 안 겹치려면 테두리 아래로 이만큼은 띄워야 한다(Graphics/Audio 탭과 동일).
                const BOX_TOP_INSET: f32 = 16.0;
                const BOX_BOTTOM_PAD: f32 = 8.0;
                const BOX_TOP_MARGIN: f32 = 12.0;
                const CHECK_ROW_H: f32 = 32.0;
                const SWATCH: f32 = 28.0;
                const SWATCH_GAP: f32 = 8.0;
                const SWATCH_LABEL_H: f32 = 26.0;
                const SWATCH_ROW_H: f32 = SWATCH + 10.0;
                let header_h = ACCORDION_HEADER_H;
                let row_h = ACCORDION_ROW_H;
                let list_w = (area.w - 28.0 - SB_RESERVE - BOX_PAD * 2.0).max(120.0);
                let bx = cx + BOX_PAD;
                let viewport_top = cy;
                let viewport_h = (area.y + area.h - 40.0 - viewport_top).max(0.0);

                const MAX_LIST_ROWS: usize = 3; // 언어는 3개뿐이라 내부 스크롤 없이 전부 펼쳐진다
                let list_pad2 = ACCORDION_LIST_PAD * 2.0;
                let lang_list_h = if self.lang_expanded { LANG_NAMES.len().min(MAX_LIST_ROWS) as f32 * row_h + list_pad2 } else { 0.0 };

                let appearance_content_h = CHECK_ROW_H + header_h + lang_list_h + SWATCH_LABEL_H + SWATCH_ROW_H;
                let appearance_box_h = BOX_TOP_INSET + appearance_content_h + BOX_BOTTOM_PAD;
                let erase_h = 24.0;
                let total_h = BOX_TOP_MARGIN + appearance_box_h + GAP + erase_h;
                let max_scroll = (total_h - viewport_h).max(0.0);
                self.interface_scroll = self.interface_scroll.clamp(0.0, max_scroll);
                ease_scroll(&mut self.interface_scroll_disp, self.interface_scroll, win.dt, s.smooth_scroll);
                let mut inner_wheel_used = false;

                let outer_clip = Rect::new(area.x, viewport_top, area.w, viewport_h);
                r.set_clip(Some(outer_clip));
                let mut iy = viewport_top - self.interface_scroll_disp;
                let visible = |y: f32, h: f32| y + h > viewport_top && y < viewport_top + viewport_h;

                iy += BOX_TOP_MARGIN;
                let box_y = iy;
                if visible(box_y, appearance_box_h) {
                    group_box(r, cx, box_y, list_w + BOX_PAD * 2.0, appearance_box_h, t(lang, s::APPEARANCE));
                }
                iy += BOX_TOP_INSET;

                if visible(iy, CHECK_ROW_H) {
                    checkbox(r, bx, iy + 5.0, t(lang, s::SMOOTH_SCROLL), &mut s.smooth_scroll, win);
                }
                iy += CHECK_ROW_H;

                // 언어 선택지 이름은 항상 그 언어 자신의 표기(LANG_NAMES)로 보여준다 —
                // "언어" 라벨만 t() 로 지금 UI 언어에 맞춘다.
                if visible(iy, header_h)
                    && accordion_header(r, win, bx, iy, list_w, t(lang, s::LANGUAGE), LANG_NAMES[lang as usize], self.lang_expanded)
                {
                    self.lang_expanded = !self.lang_expanded;
                }
                iy += header_h;
                if self.lang_expanded && visible(iy, lang_list_h) {
                    let (clicked, _, wheel_used) = accordion_list(
                        r, win, bx, iy, list_w, &LANG_NAMES, lang as usize,
                        &mut self.lang_list_scroll, &mut self.lang_list_scroll_disp, &mut self.lang_list_sb_drag, s.smooth_scroll,
                        MAX_LIST_ROWS,
                    );
                    inner_wheel_used |= wheel_used;
                    if let Some(i) = clicked {
                        s.language = [Language::En, Language::Ko, Language::Ja][i];
                        // 언어는 바꾸자마자 바로 적용되는 게 자연스러우니(다음 프레임부터
                        // 이 창을 포함해 이미 열려있는 다른 창들도 lang 을 매 프레임
                        // s.language 에서 새로 읽으므로 즉시 반영됨), 5초 주기 자동저장을
                        // 기다리지 않고 이 순간 바로 취향 설정 파일에 써둔다.
                        crate::foundation::save_settings(&s);
                    }
                    r.set_clip(Some(outer_clip));
                }
                iy += lang_list_h;

                if visible(iy, SWATCH_LABEL_H) {
                    label(r, bx, iy, t(lang, s::BACKGROUND_COLOR), BLACK);
                }
                iy += SWATCH_LABEL_H;
                if visible(iy, SWATCH_ROW_H) {
                    for (i, (_, color)) in BG_COLORS.iter().enumerate() {
                        let sx = bx + i as f32 * (SWATCH + SWATCH_GAP);
                        r.rect(sx, iy, SWATCH, SWATCH, *color);
                        if i == s.bg_color_idx {
                            border(r, sx - 2.0, iy - 2.0, SWATCH + 4.0, SWATCH + 4.0, BLACK);
                        } else {
                            border(r, sx, iy, SWATCH, SWATCH, GRAY);
                        }
                        if win.mouse_clicked && Rect::new(sx, iy, SWATCH, SWATCH).contains(win.mouse.0, win.mouse.1) {
                            s.bg_color_idx = i;
                        }
                    }
                }

                // 저장 파일을 지우고 부팅 화면부터 다시 시작하는 위험한 동작이라, 여기서
                // 바로 실행하지 않고 데스크톱한테 확인창을 띄워달라고 요청만 한다 —
                // 그래야 이 설정 창뿐 아니라 화면 전체(다른 창 포함)를 덮는 진짜 모달로
                // 띄울 수 있다(DesktopScene::erase_confirm 참고). 지울 저장 데이터
                // 자체가 없으면(has_data == false, 예: 로비에서 Continue 가 비활성인
                // 상태) 버튼을 눌러도 아무 일도 안 일어나게 회색으로 비활성 표시한다.
                let erase_y = box_y + appearance_box_h + GAP;
                let erase_label = t(lang, s::ERASE_ALL_MEMORY);
                let btn_w = r.text_width(erase_label, 1.0) + 20.0;
                if visible(erase_y, erase_h) {
                    if self.has_data {
                        if button(r, cx, erase_y, btn_w, erase_h, erase_label, win) {
                            r.set_clip(None);
                            return AppAction::RequestErase;
                        }
                    } else {
                        raised(r, cx, erase_y, btn_w, erase_h);
                        let tw = r.text_width(erase_label, 1.0);
                        r.text(cx + (btn_w - tw) / 2.0, erase_y + (erase_h - CELL_H) / 2.0, erase_label, 1.0, GRAY);
                    }
                }
                r.set_clip(None);

                if !inner_wheel_used && outer_clip.contains(win.mouse.0, win.mouse.1) {
                    self.interface_scroll = (self.interface_scroll - win.wheel / 120.0 * 24.0).clamp(0.0, max_scroll);
                }

                if max_scroll > 0.0 {
                    let sb_x = cx + list_w + BOX_PAD * 2.0 + 4.0;
                    let frac = viewport_h / total_h;
                    scrollbar(
                        r, win, sb_x, viewport_top, 8.0, viewport_h, frac,
                        self.interface_scroll_disp, &mut self.interface_scroll, max_scroll, &mut self.interface_sb_drag,
                    );
                }
            }
            _ => {}
        }

        let ok_label = t(lang, common::OK);
        let ok_w = (r.text_width(ok_label, 1.0) + 20.0).max(70.0);
        if button(r, area.x + area.w - 10.0 - ok_w, area.y + area.h - 32.0, ok_w, 24.0, ok_label, win) {
            return AppAction::Close;
        }
        AppAction::None
    }
}
