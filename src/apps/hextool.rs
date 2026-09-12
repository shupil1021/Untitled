//! HexTool — Installer 마법사를 끝까지 마치면 바탕화면에 생기는 설치된 프로그램.
//! ????? 에 지금 떠 있는 사진들을 한 장씩 골라 들여다보며 시체/글리치/이상현상
//! 없음 중 하나를 체크하고 저장하는 검수 도구다. 빈 미리보기 자리를 클릭하면
//! "My Computer"(File Explorer)와 비슷한 아이콘 그리드 창(apps/hex_picker.rs,
//! 별개의 창으로 뜬다)이 열리고, 거기서 사진을 하나 고르면 곧장 그 사진이 이
//! 창의 미리보기로 들어온다. 검수 저장을 누르면 잠깐(SAVE_DELAY 초) "저장 중"
//! 표시가 뜬 뒤 미리보기가 다시 빈 자리로 돌아간다 — 그 자리를 클릭해서 다음
//! 사진을 고르면 된다. 예전엔 이 선택 기능을 여는 별도 버튼/링크가 따로
//! 있었는데, 어차피 저장할 때마다 빈 자리로 돌아오므로 굳이 필요 없어 없앴다.
//!
//! 오른쪽 패널은 위에서부터: 지금까지 검수한 개수("N개의 이미지 중 M개
//! 검수됨") → 밝기/채도 슬라이더 → 미니맵 → 이상현상 체크박스 3개(시체/글리치/
//! 이상현상 없음, 서로 배타적 — 하나를 반드시 골라야 저장 버튼이 활성화된다,
//! 셋을 그룹 박스로 따로 묶어서 한 세트라는 걸 보여준다) → 저장/내보내기 버튼
//! 순서다. 패널 내용이 창 높이보다 길어지면 마우스 휠/스크롤바로 볼 수 있다.
//!
//! 미리보기는 photos.rs 와 같은 요령으로 원본 파일을 그때그때 디코드해 텍스처로
//! 올린다(고른 사진이 바뀔 때만 한 번). 밝기/채도 슬라이더는 이 렌더러에 셰이더
//! 유니폼이 없어서 진짜 픽셀 단위 보정은 못 하고, 밝기는 스프라이트 곱연산
//! 틴트로, 채도는 그 위에 회색 반투명을 덧씌우는 방식으로 흉내만 낸다. 미리보기
//! 위에서 휠을 굴리면 마우스가 가리키는 지점을 기준으로 확대/축소되고, 좌클릭
//! 드래그로는 그 자리에서 원하는 방향으로 이동(pan)할 수 있다 — 미니맵이 지금
//! 보고 있는 영역을 노란 테두리 상자로 보여준다.
//!
//! 검수 결과(사진별로 고른 카테고리)는 fs.photo_reviews 에 저장돼 게임을 다시
//! 켜도 유지된다 — "검수 저장"을 누를 때마다 그 사진 하나의 결과만 fs 에 기록
//! 한다. ????? 의 사진을 전부 검수하면(fs.photo_reviews 와 fs.photos_current 의
//! 교집합이 photos_current 전체를 덮으면) 버튼이 "압축파일 내보내기"로 바뀌고,
//! 누르면 시체/글리치로 체크된 사진들만 모아 FileKind::PhotoReport 압축파일을
//! 만든다(desktop.rs 참고) — 이걸 재연구 업무 보고 메일에 첨부해 보내면 ?????
//! 피드가 새로 갱신된다.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use miniquad::{RenderingBackend, TextureId};

use crate::foundation::{AnomalyCategory, Language, Settings};
use crate::gfx::{Assets, Rect, Renderer, CELL_H};
use crate::strings::{hextool as s, t};
use crate::ui::*;

use super::photos::{find_photo_dir, load_scaled_texture};
use super::widgets::{draw_slider, ease_scroll, scrollbar};
use super::{App, AppAction, WinInput};

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 8.0;
const LABEL_H: f32 = 20.0;
const PANEL_W: f32 = 150.0;
const SLIDER_GAP: f32 = 8.0;
const SLIDER_ROW_H: f32 = 40.0; // 슬라이더 두 개 사이 마진
const STATUS_ROW_H: f32 = 18.0;
const MINIMAP_SIDE: f32 = 110.0;
// checkbox() 는 라벨을 y-3 에, 16x16 체크박스 사각형을 y..y+16 에 그린다 —
// 아래에서 각 행을 row_y+CHECK_Y_OFFSET 에 그리므로, 한 행이 실제로 차지하는
// 세로 폭(체크박스 사각형 하단까지)은 CHECK_Y_OFFSET+16 이다. 예전엔 이 폭이
// CHECK_ROW_H(그때는 20) 보다 커서(오프셋 10 + 16 = 26 > 20) 다음 줄과
// 겹쳐 보였다("높이가 안 맞는 느낌") — 지금 값은 딱 맞고 조금 여유가 있다.
const CHECK_Y_OFFSET: f32 = 4.0;
const CHECK_ROW_H: f32 = 22.0;
const BTN_H: f32 = 24.0;
const ROW_GAP: f32 = 6.0;
// 체크박스 3개를 감싸는 그룹 박스(ui::group_box) 여백 — settings.rs 의 그룹
// 박스들과 같은 값을 써서 이 게임 안에서 그룹 박스가 항상 같은 비례로 보이게
// 맞췄다.
const BOX_TOP_MARGIN: f32 = 12.0; // 그룹박스 라벨이 위 테두리에 걸치는 만큼 위쪽에 미리 비워둘 여백
const BOX_TOP_INSET: f32 = 14.0; // 박스 테두리 상단에서 첫 체크박스까지
const BOX_BOTTOM_PAD: f32 = 16.0; // 마지막 체크박스 밑에서 박스 테두리까지 — 위쪽 여백과 비슷하게 맞춰서 위아래가 대칭으로 보이게 한다
const CHECK_BOX_H: f32 = BOX_TOP_INSET + CHECK_ROW_H * 3.0 + BOX_BOTTOM_PAD;
// 저장을 누른 뒤 미리보기가 빈 자리로 돌아가기까지의 "저장 중" 표시 시간(초).
const SAVE_DELAY: f32 = 0.5;
// "압축파일 내보내기"를 누른 뒤 실제로 내보내기 전 "내보내는 중" 표시 시간(초) —
// 검수 저장보다 조금 더 걸리게 해서 "장수만큼 뭔가 처리하는" 느낌을 준다(실제로는
// 그냥 연출이고 처리 자체는 즉시 끝난다).
const EXPORT_DELAY: f32 = 0.8;
// 패널 안에서 각 행이 시작하는 y 오프셋(패널 맨 위 기준) — 스크롤(관성/클램프)과
// 각 행의 보임 여부 판정에 쓴다. 순서: 검수 현황 → 밝기 → 채도 → 미니맵 →
// 체크박스 그룹 박스 → 저장/내보내기 버튼.
const STATUS_Y: f32 = 0.0;
const SLIDERS_Y: f32 = STATUS_Y + STATUS_ROW_H + ROW_GAP;
const MINIMAP_Y: f32 = SLIDERS_Y + SLIDER_ROW_H * 2.0 + ROW_GAP;
const CHECK_BOX_Y: f32 = MINIMAP_Y + MINIMAP_SIDE + ROW_GAP + BOX_TOP_MARGIN;
const BTN_Y: f32 = CHECK_BOX_Y + CHECK_BOX_H + ROW_GAP;
const PANEL_CONTENT_H: f32 = BTN_Y + BTN_H;

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
    category: Option<AnomalyCategory>, // 지금 로드된 사진에 대해 고른 체크박스(저장 전까지는 임시) — None 이면 아직 아무것도 안 고름
    saving: Option<f32>, // Some(경과 시간) 이면 "저장 중" 표시 중 — SAVE_DELAY 를 넘으면 미리보기를 비운다
    // Some((경과 시간, 내보낼 사진 목록)) 이면 "압축파일 내보내기"를 막 눌러
    // "내보내는 중" 표시 중 — EXPORT_DELAY 를 넘으면 그제서야 실제로
    // AppAction::ExportPhotoReport 를 보낸다.
    export_pending: Option<(f32, Vec<String>)>,
    photos_current: Vec<String>,                  // ????? 에 지금 떠 있는 사진 식별자 전체 — 진행 상황(N) 계산용
    reviews: HashMap<String, AnomalyCategory>,    // fs.photo_reviews 의 로컬 사본 — "저장" 할 때마다 여기도 같이 갱신해서 M 이 그 자리에서 바로 반영된다
    panel_scroll: f32,
    panel_scroll_disp: f32,
    panel_sb_drag: bool,
    settings: Rc<RefCell<Settings>>,
}

impl HexToolApp {
    pub fn new(photos_current: Vec<String>, reviews: HashMap<String, AnomalyCategory>, settings: Rc<RefCell<Settings>>) -> HexToolApp {
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
            category: None,
            saving: None,
            export_pending: None,
            photos_current,
            reviews,
            panel_scroll: 0.0,
            panel_scroll_disp: 0.0,
            panel_sb_drag: false,
            settings,
        }
    }

    // HexPickerApp 에서 사진을 고르면 desktop.rs 가 불러준다 — 미리보기/확대/
    // 이동 상태를 새 사진 기준으로 초기화하고, 이미 저장된 검수 결과가 있으면
    // 체크박스를 그 값으로 미리 채운다(없으면 None — 다시 골라야 저장 가능).
    pub(crate) fn set_selected_photo(&mut self, id: String) {
        self.category = self.reviews.get(&id).copied();
        self.loaded_photo_id = Some(id);
        self.tex = None;
        self.tex_tried = false;
        self.zoom = MIN_ZOOM;
        self.center = (0.5, 0.5);
        self.drag_last = None;
        self.saving = None;
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

    // 체크박스 하나(시체/글리치/이상현상 없음 중 하나) — 서로 배타적으로 동작
    // 한다: 체크하면 self.category 가 그 값이 되고, 이미 골라져 있던 걸 다시
    // 눌러 끄면 self.category 가 None 으로 돌아간다(그러면 저장 버튼도 다시
    // 비활성화된다).
    fn draw_category_checkbox(&mut self, r: &mut Renderer, x: f32, y: f32, label: &str, cat: AnomalyCategory, win: &WinInput) {
        let mut checked = self.category == Some(cat);
        checkbox(r, x, y, label, &mut checked, win);
        if checked {
            self.category = Some(cat);
        } else if self.category == Some(cat) {
            self.category = None;
        }
    }

    // 패널(검수 현황/슬라이더/미니맵/체크박스/버튼) — 창 높이보다 내용이 길어질
    // 수 있어서 통째로 스크롤 영역으로 감싼다. 반환값은 이번 프레임에 저장/
    // 내보내기 버튼이 눌렸을 때의 AppAction.
    fn update_panel(&mut self, r: &mut Renderer, panel: Rect, win: &WinInput, lang: Language, total: usize, reviewed: usize) -> AppAction {
        let max_scroll = (PANEL_CONTENT_H - panel.h).max(0.0);
        if panel.contains(win.mouse.0, win.mouse.1) {
            self.panel_scroll -= win.wheel / 120.0 * 24.0;
        }
        self.panel_scroll = self.panel_scroll.clamp(0.0, max_scroll);
        let smooth = self.settings.borrow().smooth_scroll;
        ease_scroll(&mut self.panel_scroll_disp, self.panel_scroll, win.dt, smooth);

        let sb_w = if max_scroll > 0.0 { 10.0 } else { 0.0 };
        let content_w = (panel.w - sb_w).max(20.0);
        let top = panel.y - self.panel_scroll_disp;
        // 이 창 좌표계 기준 행 하나가 패널의 보이는 범위 안에 조금이라도 걸치는지 —
        // 걸치지 않으면 그리지도, 입력을 받지도 않는다(스크롤로 가려진 체크박스가
        // 마우스 좌표만 우연히 겹쳐서 몰래 눌리는 일을 막는다).
        let visible = |y: f32, h: f32| y + h >= panel.y && y <= panel.y + panel.h;

        let outer_clip = r.clip();
        r.set_clip(Some(panel));

        let status_y = top + STATUS_Y;
        if visible(status_y, STATUS_ROW_H) {
            let status = t(lang, s::REVIEW_STATUS).replace("{n}", &total.to_string()).replace("{m}", &reviewed.to_string());
            r.text_clipped(panel.x, status_y + 2.0, &status, 0.72, GRAY, content_w);
        }

        let sliders_y = top + SLIDERS_Y;
        let slider_w = (content_w - 42.0).max(40.0);
        if visible(sliders_y, SLIDER_ROW_H) {
            draw_slider(r, win, panel.x, sliders_y, slider_w, t(lang, s::BRIGHTNESS), 0, &mut self.brightness, &mut self.active_slider);
        }
        if visible(sliders_y + SLIDER_ROW_H, SLIDER_ROW_H) {
            draw_slider(
                r, win, panel.x, sliders_y + SLIDER_ROW_H, slider_w, t(lang, s::SATURATION), 1, &mut self.saturation, &mut self.active_slider,
            );
        }
        if !win.mouse_down {
            self.active_slider = -1;
        }

        let minimap_y = top + MINIMAP_Y;
        if visible(minimap_y, MINIMAP_SIDE) {
            let side = MINIMAP_SIDE.min(content_w);
            self.draw_minimap(r, Rect::new(panel.x + (content_w - side) / 2.0, minimap_y, side, side));
        }

        // 체크박스 3개를 그룹 박스로 묶어서 "이 셋이 한 세트"라는 걸 시각적으로
        // 보여준다(settings.rs 의 그룹 박스들과 같은 위젯) — box_y 는 박스 테두리
        // 자체의 좌상단이고, 라벨은 그 위쪽 선에 걸쳐 그려지므로 그 만큼(BOX_TOP_
        // MARGIN)은 미리 위에 비워둔 채로 넘겨받았다(CHECK_BOX_Y 계산 참고).
        let box_y = top + CHECK_BOX_Y;
        if visible(box_y - BOX_TOP_MARGIN, CHECK_BOX_H + BOX_TOP_MARGIN) {
            group_box(r, panel.x, box_y, content_w, CHECK_BOX_H, t(lang, s::ANOMALY_GROUP));
        }
        let checks_y = box_y + BOX_TOP_INSET;
        let rows = [
            (t(lang, s::ANOMALY_CORPSE), AnomalyCategory::Corpse),
            (t(lang, s::ANOMALY_GLITCH), AnomalyCategory::Glitch),
            (t(lang, s::ANOMALY_NONE), AnomalyCategory::NoAnomaly),
        ];
        for (i, (label, cat)) in rows.into_iter().enumerate() {
            let y = checks_y + i as f32 * CHECK_ROW_H;
            if visible(y, CHECK_ROW_H) {
                self.draw_category_checkbox(r, panel.x + 6.0, y + CHECK_Y_OFFSET, label, cat, win);
            }
        }

        let btn_y = top + BTN_Y;
        let all_reviewed = total > 0 && reviewed >= total;
        let btn_label = if all_reviewed { t(lang, s::EXPORT_ARCHIVE) } else { t(lang, s::SAVE_REVIEW) };
        // 내보내는 중(export_pending)이나 저장 중(saving)에는 버튼을 다시 누를 수
        // 없게 막는다 — export_pending 은 "내보내는 중" 표시가 뜬 짧은 시간 동안
        // 또 눌러서 중복 예약하는 걸 막고, saving 은 방금 마지막 사진을 저장해서
        // (이 프레임부터 all_reviewed 가 곧장 true 로 바뀐다) 아직 "저장 중"
        // 표시가 끝나기도 전에 곧장 Export 를 눌러버리는 걸 막는다 — 그러면
        // export_pending 이 saving 보다 먼저 update() 의 우선순위를 가져가서
        // saving 타이머가 멈춘 채로 방치됐다가, 내보내기가 끝난 뒤에야 남은
        // "저장 중" 표시가 다시 잠깐 나타나는 어색한 상태가 됐었다.
        let enabled =
            self.export_pending.is_none() && self.saving.is_none() && (all_reviewed || (self.loaded_photo_id.is_some() && self.category.is_some()));
        let mut result = AppAction::None;
        if visible(btn_y, BTN_H) {
            if enabled && button(r, panel.x, btn_y, content_w, BTN_H, btn_label, win) {
                if all_reviewed {
                    let flagged: Vec<String> = self
                        .photos_current
                        .iter()
                        .filter(|id| matches!(self.reviews.get(*id), Some(AnomalyCategory::Corpse | AnomalyCategory::Glitch)))
                        .cloned()
                        .collect();
                    // 곧장 내보내지 않고 잠깐 "내보내는 중" 표시부터 보여준다 —
                    // update() 의 export_pending 처리가 EXPORT_DELAY 뒤에 실제로
                    // AppAction::ExportPhotoReport 를 보낸다.
                    self.export_pending = Some((0.0, flagged));
                } else if let (Some(id), Some(cat)) = (self.loaded_photo_id.clone(), self.category) {
                    self.reviews.insert(id.clone(), cat);
                    self.saving = Some(0.0);
                    self.category = None;
                    result = AppAction::SavePhotoReview(id, cat);
                }
            } else if !enabled {
                // 비활성 상태 — 눌러도 반응 없는 회색 버튼으로만 그린다. raw_button()
                // 과 똑같은 공식(y + (h - CELL_H) / 2.0)으로 세로 중앙을 맞춘다 — 예전엔
                // 고정값(+5.0)을 써서 버튼 높이가 조금만 달라져도 글자가 위로 치우쳐
                // 보였다.
                raised(r, panel.x, btn_y, content_w, BTN_H);
                let tw = r.text_width(btn_label, 1.0);
                let ty = btn_y + (BTN_H - CELL_H) / 2.0;
                r.text(panel.x + (content_w - tw) / 2.0, ty, btn_label, 1.0, [0.55, 0.55, 0.55, 1.0]);
            }
        }

        r.set_clip(outer_clip);
        if max_scroll > 0.0 {
            let visible_frac = (panel.h / PANEL_CONTENT_H).clamp(0.05, 1.0);
            scrollbar(
                r, win, panel.x + content_w + 2.0, panel.y, sb_w - 2.0, panel.h, visible_frac, self.panel_scroll_disp,
                &mut self.panel_scroll, max_scroll, &mut self.panel_sb_drag,
            );
        }

        result
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

        // 위쪽 한 줄 — 지금 보고 있는 사진 이름(없으면 안내 문구)만 보여준다.
        // 선택 창을 여는 버튼/링크는 따로 없다 — 빈 미리보기 자리를 클릭하면
        // 바로 열린다(아래).
        let name_text = match &self.loaded_photo_id {
            Some(id) => id.rsplit('/').next().unwrap_or(id).to_string(),
            None => t(lang, s::NO_FILE_SELECTED).to_string(),
        };
        r.text_clipped(body.x + 4.0, body.y + 4.0, &name_text, 0.8, GRAY, body.w - 8.0);

        // 그 아래는 왼쪽 큰 미리보기 + 오른쪽 좁은 패널로 나눈다.
        let content = Rect::new(body.x, body.y + LABEL_H, body.w, body.h - LABEL_H);
        let panel_w = PANEL_W.min(content.w * 0.4).max(110.0);
        let preview = Rect::new(content.x, content.y, content.w - panel_w - SLIDER_GAP, content.h);
        let panel = Rect::new(preview.x + preview.w + SLIDER_GAP, content.y, panel_w, content.h);

        // "압축파일 내보내기"가 예약돼 있으면(export_pending) 그것부터 처리한다 —
        // EXPORT_DELAY 를 넘기면 이 프레임에 곧장 AppAction::ExportPhotoReport 를
        // 보내고 끝낸다(그 아래 패널/저장 로직은 이번 프레임엔 그릴 필요가 없다).
        if let Some((elapsed, _)) = &self.export_pending {
            let elapsed = elapsed + win.dt;
            if elapsed >= EXPORT_DELAY {
                let (_, flagged) = self.export_pending.take().unwrap();
                return AppAction::ExportPhotoReport(flagged);
            }
            self.export_pending.as_mut().unwrap().0 = elapsed;
            sunken(r, preview.x, preview.y, preview.w, preview.h);
            let msg = t(lang, s::EXPORTING);
            let tw = r.text_width(msg, 0.8);
            r.text(preview.x + (preview.w - tw) / 2.0, preview.y + preview.h / 2.0 - 6.0, msg, 0.8, GRAY);
            let total = self.photos_current.len();
            let reviewed = self.reviews.iter().filter(|(id, _)| self.photos_current.contains(id)).count();
            self.update_panel(r, panel, win, lang, total, reviewed);
            return AppAction::None;
        }

        let mut open_picker = false;
        if let Some(elapsed) = self.saving {
            // "검수 저장"을 막 눌렀다 — 잠깐 저장 중 표시를 보여준 뒤 미리보기를
            // 빈 자리로 되돌린다(다음 사진은 그 자리를 클릭해서 고른다).
            let elapsed = elapsed + win.dt;
            if elapsed >= SAVE_DELAY {
                self.saving = None;
                self.loaded_photo_id = None;
                self.tex = None;
                self.tex_tried = false;
            } else {
                self.saving = Some(elapsed);
            }
            sunken(r, preview.x, preview.y, preview.w, preview.h);
            let msg = t(lang, s::SAVING);
            let tw = r.text_width(msg, 0.8);
            r.text(preview.x + (preview.w - tw) / 2.0, preview.y + preview.h / 2.0 - 6.0, msg, 0.8, GRAY);
        } else if self.loaded_photo_id.is_some() {
            self.draw_preview(ctx, r, preview, win, lang);
        } else {
            // 아직 아무 사진도 안 골랐다 — 빈 미리보기 자리를 보여주고, 클릭하면
            // 선택 창이 뜬다.
            sunken(r, preview.x, preview.y, preview.w, preview.h);
            let hint = t(lang, s::CLICK_TO_SELECT);
            let tw = r.text_width(hint, 0.8);
            r.text(preview.x + (preview.w - tw) / 2.0, preview.y + preview.h / 2.0 - 6.0, hint, 0.8, GRAY);
            if preview.contains(win.mouse.0, win.mouse.1) && win.mouse_clicked {
                open_picker = true;
            }
        }

        // 검수 진행 상황 — photos_current 와 겹치는 reviews 만 세어서, 이전
        // 배치의 남은 기록이 섞여 잘못 세어지지 않게 한다.
        let total = self.photos_current.len();
        let reviewed = self.reviews.iter().filter(|(id, _)| self.photos_current.contains(id)).count();

        let panel_action = self.update_panel(r, panel, win, lang, total, reviewed);
        if open_picker {
            return AppAction::OpenHexPicker;
        }
        panel_action
    }
}
