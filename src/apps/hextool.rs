//! HexTool — Installer 마법사를 끝까지 마치면 바탕화면에 생기는 설치된 프로그램.
//! ????? 에 지금 떠 있는 사진들을 한 장씩 골라 들여다보며 "이상현상 있음"을
//! 체크하고 저장하는 검수 도구다. 위쪽의 "이미지 선택"을 누르면(또는 빈 미리보기
//! 자리를 클릭하면) "My Computer"(File Explorer)와 비슷한 아이콘 그리드 창
//! (apps/hex_picker.rs, 별개의 창으로 뜬다)이 열리고, 거기서 사진을 하나 고르면
//! 곧장 그 사진이 이 창의 미리보기로 들어온다. 오른쪽 패널은 위에서부터: 지금까지
//! 검수한 개수("N개의 이미지 중 M개 검수됨") → 밝기/채도 슬라이더 → 미니맵 →
//! 이상현상 체크박스 → 저장/내보내기 버튼 순서다.
//!
//! 미리보기는 photos.rs 와 같은 요령으로 원본 파일을 그때그때 디코드해 텍스처로
//! 올린다(고른 사진이 바뀔 때만 한 번). 밝기/채도 슬라이더는 이 렌더러에 셰이더
//! 유니폼이 없어서 진짜 픽셀 단위 보정은 못 하고, 밝기는 스프라이트 곱연산
//! 틴트로, 채도는 그 위에 회색 반투명을 덧씌우는 방식으로 흉내만 낸다. 미리보기
//! 위에서 휠을 굴리면 마우스가 가리키는 지점을 기준으로 확대/축소되고, 좌클릭
//! 드래그로는 그 자리에서 원하는 방향으로 이동(pan)할 수 있다 — 슬라이더 밑
//! 미니맵이 지금 보고 있는 영역을 노란 테두리 상자로 보여준다.
//!
//! 검수 결과(사진별 이상현상 체크 여부)는 fs.photo_reviews 에 저장돼 게임을 다시
//! 켜도 유지된다 — "검수 저장"을 누를 때마다 그 사진 하나의 결과만 fs 에 기록
//! 한다. ????? 의 사진을 전부 검수하면(fs.photo_reviews 와 fs.photos_current 의
//! 교집합이 photos_current 전체를 덮으면) 버튼이 "압축파일 내보내기"로 바뀌고,
//! 누르면 이상현상으로 체크된 사진들만 모아 FileKind::PhotoReport 압축파일을
//! 만든다(desktop.rs 참고) — 이걸 재연구 업무 보고 메일에 첨부해 보내면 ?????
//! 피드가 새로 갱신된다.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use miniquad::{RenderingBackend, TextureId};

use crate::foundation::{Language, Settings};
use crate::gfx::{Assets, Rect, Renderer};
use crate::strings::{hextool as s, t};
use crate::ui::*;

use super::photos::{find_photo_dir, load_scaled_texture};
use super::widgets::draw_slider;
use super::{App, AppAction, WinInput};

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 8.0;
const LABEL_H: f32 = 20.0;
const PANEL_W: f32 = 140.0;
const SLIDER_GAP: f32 = 8.0;
const SLIDER_ROW_H: f32 = 40.0; // 슬라이더 두 개 사이 마진

pub struct HexToolApp {
    loaded_photo_id: Option<String>, // 지금 미리보기 중인 assets/photo 식별자
    tex: Option<(TextureId, u32, u32)>, // 지연 로딩된 원본 텍스처
    tex_tried: bool,                    // 지금 고른 사진에 대해 한 번 로딩을 시도했는지
    zoom: f32,             // 1.0 = 전체가 다 보이게 맞춘 배율, 커질수록 확대
    center: (f32, f32),    // 지금 뷰포트 중심의 이미지 내 정규화 좌표(0..1)
    view_frac: (f32, f32), // 지금 뷰포트에 이미지의 몇 %(0..1)가 보이는지 — 미니맵 상자 크기용
    drag_last: Option<(f32, f32)>, // 좌클릭 드래그 중이면 지난 프레임 마우스 위치
    brightness: f32,
    saturation: f32,
    active_slider: i32,
    anomaly: bool, // 지금 로드된 사진의 "이상현상 있음" 체크 상태(저장 전까지는 임시)
    photos_current: Vec<String>,    // ????? 에 지금 떠 있는 사진 식별자 전체 — 진행 상황(N) 계산용
    reviews: HashMap<String, bool>, // fs.photo_reviews 의 로컬 사본 — "저장" 할 때마다 여기도 같이 갱신해서 M 이 그 자리에서 바로 반영된다
    settings: Rc<RefCell<Settings>>,
}

impl HexToolApp {
    pub(super) fn new(photos_current: Vec<String>, reviews: HashMap<String, bool>, settings: Rc<RefCell<Settings>>) -> HexToolApp {
        HexToolApp {
            loaded_photo_id: None,
            tex: None,
            tex_tried: false,
            zoom: MIN_ZOOM,
            center: (0.5, 0.5),
            view_frac: (1.0, 1.0),
            drag_last: None,
            brightness: 0.5,
            saturation: 0.5,
            active_slider: -1,
            anomaly: false,
            photos_current,
            reviews,
            settings,
        }
    }

    // HexPickerApp 에서 사진을 고르면 desktop.rs 가 불러준다 — 미리보기/확대/
    // 이동 상태를 새 사진 기준으로 초기화하고, 이미 저장된 검수 결과가 있으면
    // 체크박스를 그 값으로 미리 채운다.
    pub(crate) fn set_selected_photo(&mut self, id: String) {
        self.anomaly = self.reviews.get(&id).copied().unwrap_or(false);
        self.loaded_photo_id = Some(id);
        self.tex = None;
        self.tex_tried = false;
        self.zoom = MIN_ZOOM;
        self.center = (0.5, 0.5);
        self.drag_last = None;
    }

    // 재연구 업무 보고 메일을 실제로 보내 ????? 피드가 새로 갱신되면 desktop.rs
    // 가 불러준다 — 진행 상황(N개 중 M개) 계산 기준이 되는 photos_current 를
    // 최신으로 바꿔준다.
    pub(crate) fn refresh_photos_current(&mut self, photos_current: Vec<String>) {
        self.photos_current = photos_current;
    }

    // 미리보기 패널 — 원본을 지연 디코드해서(고른 사진이 바뀔 때만 한 번) 그대로
    // 그리고, 밝기/채도 슬라이더 값을 틴트/반투명 오버레이로 흉내내 반영한다.
    // 미리보기 위에서 휠을 굴리면 그 지점을 기준으로 확대/축소.
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
        r.rect(inner.x, inner.y, inner.w, inner.h, BLACK);
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
}

impl App for HexToolApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, FACE);
        let lang = self.settings.borrow().language;
        let body = Rect::new(area.x + 6.0, area.y + 6.0, area.w - 12.0, area.h - 12.0);

        // 위쪽 한 줄 — 지금 보고 있는 사진 이름(없으면 안내 문구) + 오른쪽에
        // "이미지 선택" 버튼(누르면 My Computer 같은 별도 창이 뜬다).
        let select_label = t(lang, s::SELECT_IMAGE);
        let select_w = r.text_width(select_label, 0.8) + 14.0;
        let name_w = (body.w - select_w - 6.0).max(20.0);
        let name_text = match &self.loaded_photo_id {
            Some(id) => id.rsplit('/').next().unwrap_or(id).to_string(),
            None => t(lang, s::NO_FILE_SELECTED).to_string(),
        };
        r.text_clipped(body.x + 4.0, body.y + 4.0, &name_text, 0.8, GRAY, name_w);
        let mut open_picker = button(r, body.x + body.w - select_w, body.y, select_w, LABEL_H - 2.0, select_label, win);

        // 그 아래는 왼쪽 큰 미리보기 + 오른쪽 좁은 패널(검수 현황/슬라이더/
        // 미니맵/체크박스/버튼)로 나눈다.
        let content = Rect::new(body.x, body.y + LABEL_H, body.w, body.h - LABEL_H);
        let panel_w = PANEL_W.min(content.w * 0.4).max(100.0);
        let preview = Rect::new(content.x, content.y, content.w - panel_w - SLIDER_GAP, content.h);
        let panel = Rect::new(preview.x + preview.w + SLIDER_GAP, content.y, panel_w, content.h);

        if self.loaded_photo_id.is_some() {
            self.draw_preview(ctx, r, preview, win, lang);
        } else {
            // 아직 아무 사진도 안 골랐다 — 빈 미리보기 자리를 보여주고, 클릭하면
            // 선택 창이 뜬다(위쪽 버튼과 같은 동작).
            sunken(r, preview.x, preview.y, preview.w, preview.h);
            let hint = t(lang, s::CLICK_TO_SELECT);
            let tw = r.text_width(hint, 0.8);
            r.text(preview.x + (preview.w - tw) / 2.0, preview.y + preview.h / 2.0 - 6.0, hint, 0.8, GRAY);
            if preview.contains(win.mouse.0, win.mouse.1) && win.mouse_clicked {
                open_picker = true;
            }
        }

        if open_picker {
            return AppAction::OpenHexPicker;
        }

        // 검수 진행 상황 — 슬라이더 바로 위. photos_current 와 겹치는 reviews 만
        // 세어서, 이전 배치의 남은 기록이 섞여 잘못 세어지지 않게 한다.
        let total = self.photos_current.len();
        let reviewed = self.reviews.iter().filter(|(id, _)| self.photos_current.contains(id)).count();
        let status =
            t(lang, s::REVIEW_STATUS).replace("{n}", &total.to_string()).replace("{m}", &reviewed.to_string());
        r.text_clipped(panel.x, panel.y + 2.0, &status, 0.72, GRAY, panel.w);

        let sliders_y = panel.y + 22.0;
        let slider_w = (panel.w - 42.0).max(40.0);
        draw_slider(r, win, panel.x, sliders_y, slider_w, t(lang, s::BRIGHTNESS), 0, &mut self.brightness, &mut self.active_slider);
        draw_slider(r, win, panel.x, sliders_y + SLIDER_ROW_H, slider_w, t(lang, s::SATURATION), 1, &mut self.saturation, &mut self.active_slider);
        if !win.mouse_down {
            self.active_slider = -1;
        }

        const CHECK_ROW_H: f32 = 20.0;
        let check_y = sliders_y + SLIDER_ROW_H * 2.0 + SLIDER_GAP;
        checkbox(r, panel.x, check_y + 10.0, t(lang, s::ANOMALY_CHECK), &mut self.anomaly, win);

        let btn_h = 24.0;
        let minimap_y = check_y + CHECK_ROW_H + 6.0;
        let avail_h = (panel.y + panel.h - minimap_y - btn_h - 6.0).max(0.0);
        let side = panel.w.min(avail_h);
        let minimap = Rect::new(panel.x + (panel.w - side) / 2.0, minimap_y, side, side);
        self.draw_minimap(r, minimap);

        let all_reviewed = total > 0 && reviewed >= total;
        let btn_label = if all_reviewed { t(lang, s::EXPORT_ARCHIVE) } else { t(lang, s::SAVE_REVIEW) };
        let enabled = all_reviewed || self.loaded_photo_id.is_some();
        if enabled && button(r, panel.x, panel.y + panel.h - btn_h, panel.w, btn_h, btn_label, win) {
            if all_reviewed {
                let flagged: Vec<String> =
                    self.photos_current.iter().filter(|id| self.reviews.get(*id).copied().unwrap_or(false)).cloned().collect();
                return AppAction::ExportPhotoReport(flagged);
            } else if let Some(id) = self.loaded_photo_id.clone() {
                self.reviews.insert(id.clone(), self.anomaly);
                return AppAction::SavePhotoReview(id, self.anomaly);
            }
        }
        AppAction::None
    }
}
