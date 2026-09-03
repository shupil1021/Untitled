//! HexTool — Installer 마법사를 끝까지 마치면 바탕화면에 생기는 설치된 프로그램.
//! 실행하면 곧장 밝기/채도 편집 화면으로 열리는데, 처음엔 아직 아무 파일도 안
//! 골라서 미리보기 자리가 비어있다 — 그 빈 자리를 클릭하면 시스템의 이미지
//! (Photos 앱에서 다운로드한 사진)/mp4 파일 목록이 미리보기 자리 위에
//! 펼쳐지고(picker_open), 하나를 고르면 그 자리에서 바로 미리보기로 바뀌면서
//! 왼쪽 큰 미리보기 + 오른쪽 밝기/채도 슬라이더 + 미니맵으로 "자세히 들여다볼"
//! 수 있다. 이미지를 한 번 고르고 나면 이미지 자체를 클릭해도 더는 목록이
//! 안 뜬다(왼쪽 드래그가 이제 그 자리를 대신 차지하기 때문) — 다른 파일로
//! 바꾸려면 미니맵 밑의 "새로 선택" 글자 링크를 눌러야 한다. 예전엔 여기서
//! 이상현상 종류를 체크리스트로 골라 제출하는 채점 단계(+ 다 보면 Done 으로
//! 넘어가는 삭제 확인 창)까지 있었는데, 재연구 업무 메일이 말하는 "체크리스트를
//! 작성해 파일로 뽑는" 절차는 나중에 진짜 콘텐츠로 다시 만들 예정이라 지금은
//! 전부 걷어내고 그냥 훑어보는 도구로만 남겨뒀다.
//!
//! 미리보기는 photos.rs 와 같은 요령으로 원본 파일을 그때그때 디코드해 텍스처로
//! 올린다(고른 파일이 바뀔 때만 한 번, PhotoViewerApp::tried 와 같은 지연 로딩
//! 패턴). 밝기/채도 슬라이더는 이 렌더러에 셰이더 유니폼이 없어서 진짜 픽셀
//! 단위 보정은 못 하고, 밝기는 스프라이트 곱연산 틴트로(1.0 을 넘는 값은
//! 렌더러가 알아서 흰색 쪽으로 잘라내니 "밝게"도 어느 정도 먹힌다), 채도는 그
//! 위에 회색 반투명을 덧씌우는 방식으로 흉내만 낸다. 미리보기 위에서 휠을
//! 굴리면 마우스가 가리키는 지점을 기준으로 확대/축소되고(zoom/center 로 뷰포트
//! 상태를 들고 있다가, 휠이 들어오면 그 지점의 이미지 좌표가 화면상 같은 자리에
//! 그대로 남도록 center 를 역산한다), 좌클릭 드래그로는 그 자리에서 원하는
//! 방향으로 이동(pan)할 수 있다. 이미지가 미리보기 자리를 다 못 채우는 부분
//! (레터박스 여백이든 확대해서 잘려나간 부분이든)은 전부 검은색이다. 슬라이더
//! 밑에는 지금 보고 있는 영역을 정사각형 전체 이미지 축소판 위에 노란 테두리
//! 상자로 표시하는 미니맵이 있다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{RenderingBackend, TextureId};

use crate::foundation::{FileId, Language, Settings};
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{hextool as s, t};
use crate::ui::*;

use super::photos::{find_photo_dir, load_scaled_texture};
use super::widgets::{draw_slider, ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 8.0;

pub struct HexToolApp {
    review_files: Vec<(FileId, String, IconType, Option<String>)>, // 고를 수 있는 파일 목록 — desktop.rs 가 open() 시점에 스냅샷으로 넘겨줌
    loaded_name: String, // 지금 미리보기 중인 파일 이름 — 아직 안 골랐으면 빈 문자열
    loaded_id: Option<FileId>,
    loaded_photo_id: Option<String>, // assets/photo/ 안의 실제 식별자 — 원본을 불러올 때 씀(하위 폴더 없으면 None)
    tex: Option<(TextureId, u32, u32)>, // 지연 로딩된 원본 텍스처
    tex_tried: bool,                    // 지금 고른 파일에 대해 한 번 로딩을 시도했는지
    picker_open: bool,                  // 미리보기 자리 위에 파일 목록이 펼쳐져 있는지
    zoom: f32,                // 1.0 = 전체가 다 보이게 맞춘 배율, 커질수록 확대
    center: (f32, f32),       // 지금 뷰포트 중심의 이미지 내 정규화 좌표(0..1)
    view_frac: (f32, f32),    // 지금 뷰포트에 이미지의 몇 %(0..1)가 보이는지 — 미니맵 상자 크기용
    drag_last: Option<(f32, f32)>, // 좌클릭 드래그 중이면 지난 프레임 마우스 위치
    brightness: f32,
    saturation: f32,
    active_slider: i32,
    // 파일 목록(picker)이 화면보다 길어지면 스크롤해서 봐야 한다 — 예전엔 넘치는
    // 항목이 그냥 잘려나가고 스크롤할 방법이 아예 없었다(사용성 문제로 지적받아
    // 추가).
    picker_scroll: f32,
    picker_scroll_disp: f32,
    picker_sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl HexToolApp {
    pub(super) fn new(review_files: Vec<(FileId, String, IconType, Option<String>)>, settings: Rc<RefCell<Settings>>) -> HexToolApp {
        // 처음부터 아무것도 없는 목록이면 어차피 고를 게 없으니, 빈 미리보기 대신
        // 바로 "파일이 없습니다" 안내가 뜨는 게 낫다 — picker 를 열어둔 채로 시작.
        let picker_open = review_files.is_empty();
        HexToolApp {
            review_files,
            loaded_name: String::new(),
            loaded_id: None,
            loaded_photo_id: None,
            tex: None,
            tex_tried: false,
            picker_open,
            zoom: MIN_ZOOM,
            center: (0.5, 0.5),
            view_frac: (1.0, 1.0),
            drag_last: None,
            brightness: 0.5,
            saturation: 0.5,
            active_slider: -1,
            picker_scroll: 0.0,
            picker_scroll_disp: 0.0,
            picker_sb_drag: false,
            settings,
        }
    }

    // 다운로드/이동/삭제 등으로 fs 가 바뀐 뒤 desktop.rs 가 불러준다 — 지금 보고
    // 있는 미리보기/확대/슬라이더 상태는 그대로 두고 고를 수 있는 목록만 최신화
    // 한다(그래서 HexTool 을 열어둔 채로 다른 창에서 파일을 받아도 다시 열지
    // 않고 바로 목록에 나타난다).
    pub(crate) fn refresh_review_files(&mut self, review_files: Vec<(FileId, String, IconType, Option<String>)>) {
        self.review_files = review_files;
    }

    // 미리보기 패널 — 원본을 지연 디코드해서(고른 파일이 바뀔 때만 한 번) 실제
    // 이미지를 그대로 그리고, 밝기/채도 슬라이더 값을 틴트/반투명 오버레이로
    // 흉내내 반영한다. 미리보기 위에서 휠을 굴리면 그 지점을 기준으로 확대/축소.
    fn draw_preview(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, area: Rect, win: &WinInput, lang: Language) {
        sunken(r, area.x, area.y, area.w, area.h);
        let inner = Rect::new(area.x + 3.0, area.y + 3.0, area.w - 6.0, area.h - 6.0);
        // 이미지가 못 채우는 자리는 sunken 배경 대신 검은색으로.
        r.rect(inner.x, inner.y, inner.w, inner.h, BLACK);

        if !self.tex_tried {
            self.tex_tried = true;
            if let Some(photo_id) = &self.loaded_photo_id
                && let Some(dir) = find_photo_dir()
            {
                self.tex = load_scaled_texture(ctx, &dir.join(photo_id), None);
            }
        }

        let Some((tex, w, h)) = self.tex else {
            let msg = t(lang, s::NO_PREVIEW);
            let tw = r.text_width(msg, 0.75);
            r.text(inner.x + ((inner.w - tw) / 2.0).max(0.0), inner.y + inner.h / 2.0 - 6.0, msg, 0.75, GRAY);
            return;
        };

        let (iw, ih) = (w as f32, h as f32);
        let base_scale = (inner.w / iw).min(inner.h / ih);
        let cx = inner.x + inner.w / 2.0;
        let cy = inner.y + inner.h / 2.0;

        // 마우스가 미리보기 위에 있을 때 휠을 굴리면, 그 지점의 이미지 좌표가
        // 확대/축소 후에도 화면상 같은 자리에 그대로 남도록 center 를 역산한다.
        if inner.contains(win.mouse.0, win.mouse.1) && win.wheel != 0.0 {
            let disp_scale = base_scale * self.zoom;
            let (dw, dh) = (iw * disp_scale, ih * disp_scale);
            let dx = cx - self.center.0 * dw;
            let dy = cy - self.center.1 * dh;
            let img_x = (win.mouse.0 - dx) / dw;
            let img_y = (win.mouse.1 - dy) / dh;

            let notches = win.wheel.clamp(-3.0, 3.0);
            let new_zoom = (self.zoom * 1.15f32.powf(notches)).clamp(MIN_ZOOM, MAX_ZOOM);
            let new_disp_scale = base_scale * new_zoom;
            let (ndw, ndh) = (iw * new_disp_scale, ih * new_disp_scale);
            let ndx = win.mouse.0 - img_x * ndw;
            let ndy = win.mouse.1 - img_y * ndh;
            self.center = (((cx - ndx) / ndw).clamp(0.0, 1.0), ((cy - ndy) / ndh).clamp(0.0, 1.0));
            self.zoom = new_zoom;
        }

        let disp_scale = base_scale * self.zoom;
        let (dw, dh) = (iw * disp_scale, ih * disp_scale);

        // 좌클릭 드래그 — 마우스가 움직인 만큼(화면 픽셀) 그 반대 방향으로
        // center 를 옮겨서, 이미지가 손으로 끄는 대로 따라오는 느낌을 낸다.
        if inner.contains(win.mouse.0, win.mouse.1) && win.mouse_down {
            if let Some(last) = self.drag_last {
                let ddx = win.mouse.0 - last.0;
                let ddy = win.mouse.1 - last.1;
                self.center.0 = (self.center.0 - ddx / dw).clamp(0.0, 1.0);
                self.center.1 = (self.center.1 - ddy / dh).clamp(0.0, 1.0);
            }
            self.drag_last = Some(win.mouse);
        } else {
            self.drag_last = None;
        }

        let dx = cx - self.center.0 * dw;
        let dy = cy - self.center.1 * dh;
        self.view_frac = ((inner.w / dw).clamp(0.0, 1.0), (inner.h / dh).clamp(0.0, 1.0));

        r.set_clip(Some(inner));
        let b = 0.35 + self.brightness.clamp(0.0, 1.0) * 1.3;
        r.sprite(tex, dx, dy, dw, dh, [b, b, b, 1.0]);
        let wash = (1.0 - self.saturation.clamp(0.0, 1.0)) * 0.7;
        if wash > 0.02 {
            r.rect(dx, dy, dw, dh, [0.5, 0.5, 0.5, wash]);
        }

        // 아직 한 번도 확대/이동을 안 써본 상태(zoom 이 처음 그대로)에만 조작법을
        // 살짝 알려준다 — 한 번이라도 만지면 다시 안 보인다(계속 떠 있으면
        // 오히려 거슬린다). 사진 배경이 밝든 어둡든 읽히게 그림자를 한 번 더
        // 깔아서 대비를 준다(photos.rs 의 Download 글자와 같은 요령).
        if self.zoom <= MIN_ZOOM + 0.001 {
            let hint = t(lang, s::SCROLL_HINT);
            let hx = inner.x + 6.0;
            let hy = inner.y + inner.h - 16.0;
            r.text(hx + 1.0, hy + 1.0, hint, 0.7, [0.0, 0.0, 0.0, 0.7]);
            r.text(hx, hy, hint, 0.7, [0.9, 0.9, 0.9, 0.9]);
        }
        r.set_clip(None);
    }

    // 슬라이더 밑 미니맵 — 전체 이미지 축소판 위에 지금 뷰포트가 어디를 보고
    // 있는지 노란 테두리 상자로 표시한다.
    fn draw_minimap(&self, r: &mut Renderer, area: Rect) {
        if area.h < 24.0 {
            return;
        }
        sunken(r, area.x, area.y, area.w, area.h);
        let Some((tex, w, h)) = self.tex else {
            return;
        };
        let inner = Rect::new(area.x + 2.0, area.y + 2.0, area.w - 4.0, area.h - 4.0);
        r.rect(inner.x, inner.y, inner.w, inner.h, [0.0, 0.0, 0.0, 1.0]);
        let (iw, ih) = (w as f32, h as f32);
        let scale = (inner.w / iw).min(inner.h / ih);
        let (tw, th) = (iw * scale, ih * scale);
        let tx = inner.x + (inner.w - tw) / 2.0;
        let ty = inner.y + (inner.h - th) / 2.0;
        r.sprite(tex, tx, ty, tw, th, WHITE);

        let (fw, fh) = (self.view_frac.0.min(1.0), self.view_frac.1.min(1.0));
        let vx = tx + (self.center.0 - fw / 2.0).clamp(0.0, 1.0 - fw) * tw;
        let vy = ty + (self.center.1 - fh / 2.0).clamp(0.0, 1.0 - fh) * th;
        border(r, vx, vy, (fw * tw).max(2.0), (fh * th).max(2.0), [1.0, 0.9, 0.2, 1.0]);
    }

    // 미리보기 자리 위에 펼쳐지는 파일 목록 — 처음엔 빈 미리보기를 클릭하면,
    // 이미지를 이미 고른 뒤로는 "새로 선택" 링크를 눌러야만 여기로 온다. 하나를
    // 고르면 그 자리에서 바로 미리보기로 바뀐다(picker_open = false). 목록이
    // 자리보다 길면 휠/스크롤바로 넘겨볼 수 있다.
    fn draw_picker(&mut self, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput, lang: Language) {
        sunken(r, area.x, area.y, area.w, area.h);
        if self.review_files.is_empty() {
            label(r, area.x + 8.0, area.y + 8.0, t(lang, s::NO_FILES_FOUND), GRAY);
            return;
        }
        const ROW_H: f32 = 24.0;
        const SB_W: f32 = 10.0;
        let content_h = self.review_files.len() as f32 * ROW_H;
        let max_scroll = (content_h - area.h).max(0.0);
        if area.contains(win.mouse.0, win.mouse.1) {
            self.picker_scroll -= win.wheel / 120.0 * 3.0 * ROW_H;
        }
        self.picker_scroll = self.picker_scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.picker_scroll_disp, self.picker_scroll, win.dt, smooth);

        let list_w = if max_scroll > 0.0 { area.w - SB_W } else { area.w };
        r.set_clip(Some(area));
        for (i, (id, name, icon, photo_id)) in self.review_files.iter().enumerate() {
            let row = Rect::new(area.x + 2.0, area.y + 2.0 + i as f32 * ROW_H - self.picker_scroll_disp, list_w - 4.0, ROW_H - 2.0);
            if row.y + row.h < area.y || row.y > area.y + area.h {
                continue;
            }
            let hover = row.contains(win.mouse.0, win.mouse.1);
            if hover {
                r.rect(row.x, row.y, row.w, row.h, [0.82, 0.88, 0.98, 1.0]);
            }
            draw_icon(r, assets, icon, row.x + 3.0, row.y + 2.0, 18.0);
            r.text_clipped(row.x + 26.0, row.y + 3.0, name, 0.85, BLACK, row.w - 30.0);
            if hover && win.mouse_clicked {
                self.loaded_name.clone_from(name);
                self.loaded_id = Some(*id);
                self.loaded_photo_id = photo_id.clone();
                self.tex = None;
                self.tex_tried = false;
                self.picker_open = false;
                self.zoom = MIN_ZOOM;
                self.center = (0.5, 0.5);
                self.drag_last = None;
                self.brightness = 0.5;
                self.saturation = 0.5;
            }
        }
        r.set_clip(None);
        if max_scroll > 0.0 {
            let visible_frac = (area.h / content_h).clamp(0.05, 1.0);
            scrollbar(
                r, win, area.x + list_w + 1.0, area.y + 2.0, SB_W - 2.0, area.h - 4.0, visible_frac, self.picker_scroll_disp,
                &mut self.picker_scroll, max_scroll, &mut self.picker_sb_drag,
            );
        }
    }
}

impl App for HexToolApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let lang = self.settings.borrow().language;

        let body = Rect::new(area.x + 6.0, area.y + 6.0, area.w - 12.0, area.h - 12.0);
        const LABEL_H: f32 = 16.0;
        const PANEL_W: f32 = 130.0;
        const SLIDER_GAP: f32 = 8.0;
        const SLIDER_ROW_H: f32 = 40.0; // 슬라이더 두 개 사이 마진

        // 위쪽 한 줄 — 지금 보고 있는 파일명(아직 안 골랐으면 안내 문구). 다른
        // 파일로 바꿔 고르고 싶으면 미리보기 자체를 클릭하면 되므로 별도 버튼은
        // 없다.
        let placeholder = t(lang, s::NO_FILE_SELECTED);
        let name_text = if self.loaded_name.is_empty() { placeholder } else { &self.loaded_name };
        r.text_clipped(body.x + 4.0, body.y + 1.0, name_text, 0.8, GRAY, body.w - 8.0);

        // 그 아래는 왼쪽 큰 미리보기 + 오른쪽 좁은 패널(슬라이더 + 미니맵)로 나눈다.
        let content = Rect::new(body.x, body.y + LABEL_H, body.w, body.h - LABEL_H);
        let panel_w = PANEL_W.min(content.w * 0.4).max(90.0);
        let preview = Rect::new(content.x, content.y, content.w - panel_w - SLIDER_GAP, content.h);
        let panel = Rect::new(preview.x + preview.w + SLIDER_GAP, content.y, panel_w, content.h);

        if self.picker_open {
            self.draw_picker(r, assets, preview, win, lang);
        } else if self.loaded_id.is_some() {
            // 이미지를 한 번 고르고 나면 그 자리는 드래그(이동)용이라, 클릭해도
            // 더는 목록이 안 뜬다 — 미니맵 밑의 "새로 선택" 링크로만 바꿔 고른다.
            self.draw_preview(ctx, r, preview, win, lang);
        } else {
            // 아직 아무 파일도 안 골랐다 — 빈 미리보기 자리를 보여주고, 클릭하면
            // 바로 파일 목록이 그 자리에 펼쳐진다.
            sunken(r, preview.x, preview.y, preview.w, preview.h);
            let hint = t(lang, s::CLICK_TO_SELECT);
            let tw = r.text_width(hint, 0.8);
            r.text(preview.x + (preview.w - tw) / 2.0, preview.y + preview.h / 2.0 - 6.0, hint, 0.8, GRAY);
            if preview.contains(win.mouse.0, win.mouse.1) && win.mouse_clicked {
                self.picker_open = true;
            }
        }

        // 지금 확대 배율(%) — 이미지를 고른 뒤에만 보이고, 클릭하면 확대/이동을
        // 한 번에 원래대로 되돌린다("새로 선택"으로 같은 파일을 다시 골라야만
        // 초기화되던 것보다 훨씬 빠른 지름길).
        const ZOOM_ROW_H: f32 = 16.0;
        let panel_top = panel.y + 4.0;
        if self.loaded_id.is_some() && !self.picker_open {
            let zoom_text = format!("{}: {}%", t(lang, s::ZOOM), (self.zoom * 100.0).round() as i32);
            let at_default = self.zoom <= MIN_ZOOM + 0.001 && (self.center.0 - 0.5).abs() < 0.001 && (self.center.1 - 0.5).abs() < 0.001;
            let hover = !at_default
                && win.mouse.0 >= panel.x
                && win.mouse.0 <= panel.x + panel.w
                && win.mouse.1 >= panel_top
                && win.mouse.1 <= panel_top + ZOOM_ROW_H;
            let color = if at_default { GRAY } else if hover { NAVY } else { [0.35, 0.35, 0.35, 1.0] };
            r.text(panel.x, panel_top + 1.0, &zoom_text, 0.75, color);
            if hover {
                let tw = r.text_width(&zoom_text, 0.75);
                r.rect(panel.x, panel_top + 12.0, tw, 1.0, color);
            }
            if hover && win.mouse_clicked {
                self.zoom = MIN_ZOOM;
                self.center = (0.5, 0.5);
            }
        }

        let sliders_y = panel_top + ZOOM_ROW_H;
        let slider_w = (panel.w - 42.0).max(40.0);
        let brightness_label = t(lang, s::BRIGHTNESS);
        let saturation_label = t(lang, s::SATURATION);
        draw_slider(r, win, panel.x, sliders_y, slider_w, brightness_label, 0, &mut self.brightness, &mut self.active_slider);
        draw_slider(r, win, panel.x, sliders_y + SLIDER_ROW_H, slider_w, saturation_label, 1, &mut self.saturation, &mut self.active_slider);
        if !win.mouse_down {
            self.active_slider = -1;
        }

        // 미니맵은 항상 정사각형(1:1) — 패널 폭과, 밑에 "새로 선택" 링크 한 줄을
        // 뺀 나머지 세로 공간 중 더 좁은 쪽에 맞춰서 정사각형 한 변을 정한다.
        const LINK_ROW_H: f32 = 16.0;
        let minimap_y = sliders_y + SLIDER_ROW_H * 2.0 + SLIDER_GAP;
        let avail_h = (panel.y + panel.h - minimap_y - LINK_ROW_H).max(0.0);
        let side = panel.w.min(avail_h);
        let minimap = Rect::new(panel.x + (panel.w - side) / 2.0, minimap_y, side, side);
        self.draw_minimap(r, minimap);

        // 이미지를 이미 고른 상태에서만 보인다 — "새로 선택"을 눌러야만 다시
        // 파일 목록이 뜨도록(이미지 자체 클릭으로는 더 이상 안 뜬다).
        if self.loaded_id.is_some() && !self.picker_open && side > 0.0 {
            let relink_label = t(lang, s::NEW_SELECTION);
            let tw = r.text_width(relink_label, 0.75);
            let lx = panel.x + (panel.w - tw) / 2.0;
            let ly = minimap.y + minimap.h + 3.0;
            let hover = win.mouse.0 >= lx - 2.0 && win.mouse.0 <= lx + tw + 2.0 && win.mouse.1 >= ly - 2.0 && win.mouse.1 <= ly + 14.0;
            let color = if hover { NAVY } else { GRAY };
            r.text(lx, ly, relink_label, 0.75, color);
            if hover {
                r.rect(lx, ly + 11.0, tw, 1.0, color);
            }
            if hover && win.mouse_clicked {
                self.picker_open = true;
            }
        }

        AppAction::None
    }
}
