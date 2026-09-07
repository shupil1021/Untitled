//! HexTool — Installer 마법사를 끝까지 마치면 바탕화면에 생기는 설치된 프로그램.
//! 임의 파일을 골라 훑어보던 예전의 범용 뷰어 기능은 없앴다 — 이제 이 프로그램은
//! 오직 ????? 사진 순차 검수 하나만 한다:
//!
//! - fs.photos_pending_review 가 true(지금 ?????에 떠 있는 배치를 아직 검수
//!   안 함 — 새 게임이거나, 재연구 업무 보고 메일로 피드가 막 갱신됐을 때)면,
//!   실행하는 순간 곧장 설치 마법사 진행바 같은 로딩 게이지가 잠깐 차오른 뒤
//!   (Stage::Loading) 자동으로 검수 화면(Stage::Reviewing)으로 넘어간다.
//! - 검수 화면은 지금 ?????에 떠 있는 사진(fs.photos_current)을 한 장씩 순서대로
//!   보여주며 "이상현상 있음" 체크를 받는다. 이미지 표시는 photos.rs 와 같은
//!   요령으로 원본을 그때그때 디코드해 텍스처로 올리고, 휠로 확대/축소, 좌클릭
//!   드래그로 이동할 수 있다.
//! - 마지막 장에서 "검수 완료"를 누르면 체크된 사진들로 AppAction::
//!   ExportPhotoReport 를 보낸다(desktop.rs 가 받아서 압축파일을 만들고
//!   fs.photos_pending_review 를 false 로 되돌린다) — 체크된 게 하나도 없어도
//!   이 액션은 보낸다(빈 Vec — 압축파일은 안 만들지만 "검수는 끝났다"는 기록은
//!   필요하다).
//! - pending_review 가 false 면(이미 검수를 끝낸 배치) 게이지 없이 곧장
//!   Stage::Done 으로 열려서 지난 결과 요약만 보여준다.

use std::cell::RefCell;
use std::rc::Rc;

use miniquad::{RenderingBackend, TextureId};

use crate::foundation::{Language, Settings};
use crate::gfx::{Assets, Rect, Renderer};
use crate::secrets;
use crate::strings::{hextool as s, t};
use crate::ui::*;

use super::photos::{find_photo_dir, load_scaled_texture};
use super::{App, AppAction, WinInput};

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 8.0;
const LABEL_H: f32 = 20.0;
const PANEL_W: f32 = 130.0;
const SLIDER_GAP: f32 = 8.0;
const LOADING_DURATION: f32 = 1.6; // 로딩 게이지가 다 차는 데 걸리는 시간(초) — 설치 마법사보다 훨씬 짧다(가짜 스캔이라 굳이 오래 끌 이유가 없다)

// 검수를 시작하기 전(로딩 게이지) / 검수 중 / 검수를 끝낸 뒤(요약) — 세 값 다
// Copy 라 `match self.stage { ... }` 로 값을 복사해 쓰고 나서 그 안에서 다시
// self 를 (재)대여하는 메서드를 자유롭게 부를 수 있다(참조를 들고 있으면
// борrow 충돌이 난다).
#[derive(Clone, Copy)]
enum Stage {
    Loading(f32),      // 경과 시간(초) — LOADING_DURATION 에 도달하면 자동으로 Reviewing 으로
    Reviewing,
    Done(Option<usize>), // 압축파일에 담긴 장수(0장이면 None 과 화면상 구분 안 함 — 둘 다 "이상현상 없음" 문구)
}

pub struct HexToolApp {
    loaded_photo_id: Option<String>, // 지금 미리보기 중인 assets/photo 식별자
    tex: Option<(TextureId, u32, u32)>, // 지연 로딩된 원본 텍스처
    tex_tried: bool,                    // 지금 고른 사진에 대해 한 번 로딩을 시도했는지
    zoom: f32,             // 1.0 = 전체가 다 보이게 맞춘 배율, 커질수록 확대
    center: (f32, f32),    // 지금 뷰포트 중심의 이미지 내 정규화 좌표(0..1)
    drag_last: Option<(f32, f32)>, // 좌클릭 드래그 중이면 지난 프레임 마우스 위치
    settings: Rc<RefCell<Settings>>,
    photos_current: Vec<String>, // ????? 에 지금 떠 있는 사진 식별자 스냅샷 — 로딩이 끝나면 이걸로 검수 대기열을 채운다
    review_queue: Vec<String>,   // 검수 중인 사진 식별자들
    review_flags: Vec<bool>,     // review_queue 와 길이가 같다 — 인덱스별 "이상현상 있음" 체크
    review_index: usize,        // 지금 보고 있는 검수 순번
    stage: Stage,
}

impl HexToolApp {
    pub(super) fn new(photos_current: Vec<String>, pending_review: bool, last_report_count: Option<usize>, settings: Rc<RefCell<Settings>>) -> HexToolApp {
        let stage = if pending_review { Stage::Loading(0.0) } else { Stage::Done(last_report_count) };
        HexToolApp {
            loaded_photo_id: None,
            tex: None,
            tex_tried: false,
            zoom: MIN_ZOOM,
            center: (0.5, 0.5),
            drag_last: None,
            settings,
            photos_current,
            review_queue: Vec::new(),
            review_flags: Vec::new(),
            review_index: 0,
            stage,
        }
    }

    // 로딩 게이지가 다 찼을 때 부른다 — photos_current 스냅샷으로 검수 대기열을
    // 채우고 첫 장을 미리보기에 건다. photos_current 가(있을 수 없지만 방어적으로)
    // 비어있으면 검수할 게 없으니 곧장 Done(None) 으로 끝낸다.
    fn start_review(&mut self) {
        if self.photos_current.is_empty() {
            self.stage = Stage::Done(None);
            return;
        }
        self.review_queue.clone_from(&self.photos_current);
        self.review_flags = vec![false; self.review_queue.len()];
        self.review_index = 0;
        self.loaded_photo_id = self.review_queue.first().cloned();
        self.tex = None;
        self.tex_tried = false;
        self.zoom = MIN_ZOOM;
        self.center = (0.5, 0.5);
        self.drag_last = None;
        self.stage = Stage::Reviewing;
    }

    // 설치 마법사(installer.rs)의 진행바와 같은 느낌의 로딩 게이지 — 실제로 뭘
    // 하는 건 아니고, "?????를 스캔하는 중" 이라는 연출 한 박자만 준다.
    fn draw_loading(&self, r: &mut Renderer, body: Rect, lang: Language, progress: f32) {
        let title = t(lang, s::SCANNING).replace("{app}", secrets::PHOTOS_APP_NAME);
        r.text_clipped(body.x + 4.0, body.y + 4.0, &title, 0.9, BLACK, body.w - 8.0);

        let bar_y = body.y + 34.0;
        const BAR_H: f32 = 18.0;
        sunken(r, body.x, bar_y, body.w, BAR_H);
        let fill_w = (body.w - 4.0) * progress;
        if fill_w > 0.0 {
            r.rect(body.x + 2.0, bar_y + 2.0, fill_w, BAR_H - 4.0, NAVY);
        }
        let pct = format!("{}%", (progress * 100.0) as i32);
        r.text(body.x, bar_y + BAR_H + 8.0, &pct, 0.8, GRAY);
    }

    // 검수를 이미 끝낸 배치를 다시 열었을 때(또는 방금 검수를 끝낸 직후) 보여주는
    // 요약 — 압축파일에 담긴 장수만 알려준다.
    fn draw_done(&self, r: &mut Renderer, body: Rect, lang: Language, count: Option<usize>) {
        let msg = match count {
            Some(n) if n > 0 => {
                t(lang, s::REVIEW_DONE_FOUND).replace("{n}", &n.to_string()).replace("{file}", crate::foundation::PHOTO_REPORT_NAME)
            }
            _ => t(lang, s::REVIEW_DONE_NONE).to_string(),
        };
        r.text_clipped(body.x + 4.0, body.y + 4.0, &msg, 0.85, GRAY, body.w - 8.0);
    }

    // 미리보기 패널 — 원본을 지연 디코드해서(고른 사진이 바뀔 때만 한 번) 그대로
    // 그린다. 미리보기 위에서 휠을 굴리면 마우스가 가리키는 지점을 기준으로
    // 확대/축소되고(zoom/center 로 뷰포트 상태를 들고 있다가, 휠이 들어오면 그
    // 지점의 이미지 좌표가 화면상 같은 자리에 그대로 남도록 center 를 역산한다),
    // 좌클릭 드래그로는 그 자리에서 원하는 방향으로 이동(pan)할 수 있다.
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

        r.set_clip(Some(inner));
        r.sprite(tex, dx, dy, dw, dh, WHITE);

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

    // ????? 사진을 순서대로 한 장씩 보여주며 "이상현상 있음" 체크를 받는 화면.
    // 마지막 장에서 "검수 완료"를 누르면 체크된 사진들로 AppAction::
    // ExportPhotoReport 를 돌려준다(체크된 게 하나도 없어도 빈 Vec 으로 보낸다 —
    // desktop.rs 가 이 액션을 받으면 fs.photos_pending_review 를 항상 false 로
    // 되돌리기 때문에, "검수는 끝났다"는 기록을 남기려면 빈 경우에도 보내야 한다).
    fn update_reviewing(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, body: Rect, win: &WinInput, lang: Language) -> AppAction {
        let total = self.review_queue.len();
        let progress = t(lang, s::REVIEW_PROGRESS).replace("{i}", &(self.review_index + 1).to_string()).replace("{n}", &total.to_string());
        r.text(body.x + 4.0, body.y + 4.0, &progress, 0.8, GRAY);

        let content = Rect::new(body.x, body.y + LABEL_H, body.w, body.h - LABEL_H);
        let panel_w = PANEL_W.min(content.w * 0.4).max(90.0);
        let preview = Rect::new(content.x, content.y, content.w - panel_w - SLIDER_GAP, content.h);
        let panel = Rect::new(preview.x + preview.w + SLIDER_GAP, content.y, panel_w, content.h);

        self.draw_preview(ctx, r, preview, win, lang);

        let mut checked = self.review_flags[self.review_index];
        checkbox(r, panel.x, panel.y + 10.0, t(lang, s::ANOMALY_CHECK), &mut checked, win);
        self.review_flags[self.review_index] = checked;

        let is_last = self.review_index + 1 >= total;
        let next_label = if is_last { t(lang, s::FINISH_REVIEW) } else { t(lang, s::NEXT) };
        let btn_h = 24.0;
        if button(r, panel.x, panel.y + panel.h - btn_h, panel.w, btn_h, next_label, win) {
            self.review_index += 1;
            self.tex = None;
            self.tex_tried = false;
            self.zoom = MIN_ZOOM;
            self.center = (0.5, 0.5);
            self.drag_last = None;
            if self.review_index >= total {
                let flagged: Vec<String> =
                    self.review_queue.iter().zip(self.review_flags.iter()).filter(|&(_, &checked)| checked).map(|(id, _)| id.clone()).collect();
                self.stage = Stage::Done(Some(flagged.len()));
                self.review_queue.clear();
                self.review_flags.clear();
                self.review_index = 0;
                self.loaded_photo_id = None;
                return AppAction::ExportPhotoReport(flagged);
            }
            self.loaded_photo_id = self.review_queue.get(self.review_index).cloned();
        }
        AppAction::None
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

        // self.stage 는 Copy 라 여기서 값을 복사해 매치한다 — 그래야 각 분기
        // 안에서 self.draw_loading()/self.start_review() 처럼 self 를 다시
        // (가변) 대여하는 호출을 해도 self.stage 를 향한 대여와 겹치지 않는다.
        match self.stage {
            Stage::Loading(elapsed) => {
                let elapsed = elapsed + win.dt;
                let t_frac = (elapsed / LOADING_DURATION).min(1.0);
                let progress = 1.0 - (1.0 - t_frac) * (1.0 - t_frac); // ease-out — 끝에서 살짝 느려지는 정도만
                self.draw_loading(r, body, lang, progress);
                if t_frac >= 1.0 {
                    self.start_review();
                } else {
                    self.stage = Stage::Loading(elapsed);
                }
                AppAction::None
            }
            Stage::Reviewing => self.update_reviewing(ctx, r, body, win, lang),
            Stage::Done(count) => {
                self.draw_done(r, body, lang, count);
                AppAction::None
            }
        }
    }
}
