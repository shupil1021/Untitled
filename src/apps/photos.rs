//! Photos — assets/photo/ 안의 사진들을 피드처럼 훑어보고, 클릭하면 별개의 창으로
//! 열어서 자세히 보고 다운로드할 수 있는 앱. assets/photo 밑에는 corpseImage/
//! crackImage/hintImage/normalImage 네 개의 하위 폴더로 사진들이 분류돼 있고(지금은
//! 테스트 이미지), 특정 폴더에 몰리지 않도록 전부 한데 모아 랜덤으로 섞은 뒤 그중
//! 앞쪽 UNLOCKED_PHOTO_COUNT 장만 보여준다(아래 참고 — 나중에 게임 진행에 따라
//! 하나씩 풀리는 시스템으로 바뀔 자리). 이 선택은 예전엔 앱을 열 때마다 매번 다시
//! 섞었지만, 이제 `fs.photos_current`(foundation.rs)에 한 번 정해지면 그대로
//! 저장돼서 — 재연구 업무 보고 메일을 실제로 보내(desktop.rs 의 SendNewMail
//! 처리 참고) `refresh_photos_feed()` 가 불릴 때까지는 창을 몇 번을 열었다 닫아도
//! 항상 같은 사진들이 보인다. 새로 뽑을 땐 `fs.photos_seen`(지금까지 한 번이라도
//! 나왔던 식별자 전부)을 제외해서 이미 봤던 사진이 다시 나오지 않는다. 텍스처는
//! 여전히 지연 로딩이다 — 화면 목록에 오른 사진만 미리 훑고, 실제 디코드+텍스처
//! 업로드는 화면에 실제로 보일 때(피드는 축소판, 자세히 보는 창은 원본) 그때그때
//! 한다(video_player.rs 가 movie.mp4 를 실행 파일 옆에서 런타임에 찾아 읽는 것과
//! 같은 요령 — assets/photo 도 재빌드 없이 사진만 교체해도 된다).
//!
//! 썸네일을 클릭하면 `AppAction::OpenPhoto(파일명)` 을 돌려주고, desktop.rs 가
//! `FileSystem::find_or_add_photo()` 로 그 파일명의 `FileKind::Photo` 노드를(이미
//! 있으면 재사용) 만들어 새 창(PhotoViewerApp)으로 연다 — Mail/Explorer 항목을
//! 열 때와 똑같은 경로라, 같은 사진을 또 클릭해도 창이 여러 개 안 생기고 기존
//! 창이 앞으로 나온다.
//!
//! "Download"는 메일 첨부와 똑같이 게임 내 가상 파일 시스템(File Explorer 의
//! Downloads 탭)에 넣는다 — PhotoViewerApp 하단의 회색 글자 "Download" 를 누르면
//! `AppAction::DownloadPhoto(파일명)` 을 돌려주고, desktop.rs 가 `fs.download()`
//! 한다.

use std::path::PathBuf;

use miniquad::{FilterMode, MipmapFilterMode, RenderingBackend, TextureId};

use crate::foundation::FileSystem;
use crate::gfx::{Assets, Rect, Renderer};
use crate::ui::{self, WHITE};

use super::widgets::scrollbar;
use super::{App, AppAction, WinInput};

const THUMB: f32 = 96.0;
const GAP: f32 = 10.0;

fn photo_dir_candidates() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = vec!["assets/photo".into()];
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        v.push(dir.join("assets/photo"));
        v.push(dir.join("../../assets/photo")); // target/debug → 루트
    }
    v
}

pub(crate) fn find_photo_dir() -> Option<PathBuf> {
    photo_dir_candidates().into_iter().find(|d| d.is_dir())
}

fn is_photo_file(p: &std::path::Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        let e = e.to_ascii_lowercase();
        e == "jpg" || e == "jpeg" || e == "png"
    })
}

// assets/photo 바로 밑의 사진뿐 아니라, corpseImage/crackImage/hintImage/normalImage
// 처럼 한 단계 아래 하위 폴더에 든 사진까지 훑는다(그 이상 깊이는 안 본다 — 지금
// 쓰는 분류 폴더가 딱 한 단계라서 충분하다). 반환값은 (실제 디스크 경로, 식별용
// 문자열) 쌍 — 식별용 문자열은 하위 폴더 안 사진이면 "폴더명/파일명"(예:
// "corpseImage/corpseImage1.jpg"), 바로 밑이면 그냥 "파일명" 이다. 이 식별
// 문자열이 AppAction::OpenPhoto/DownloadPhoto 와 FileKind::Photo 에 그대로
// 쓰이는 "파일명" 값이라, 서로 다른 폴더에 같은 이름의 파일이 있어도 안 겹친다.
fn scan_photos(dir: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return out };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            let folder = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let Ok(sub) = std::fs::read_dir(&path) else { continue };
            for se in sub.flatten() {
                let sp = se.path();
                if is_photo_file(&sp)
                    && let Some(name) = sp.file_name().and_then(|n| n.to_str())
                {
                    out.push((sp.clone(), format!("{folder}/{name}")));
                }
            }
        } else if is_photo_file(&path)
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            out.push((path.clone(), name.to_string()));
        }
    }
    out
}

// 프로젝트 관례대로(공유 모듈 대신 파일마다 따로) xorshift64 — 매번 앱을 열 때마다
// 사진 노출 조합이 조금씩 달라지도록 셔플할 때만 쓴다.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }
}

// 원본을 max_side 안에 들어오게(종횡비 유지) 축소해서 텍스처로 올린다. PhotoViewerApp
// 은 max_side=None(원본 그대로).
pub(crate) fn load_scaled_texture(ctx: &mut dyn RenderingBackend, path: &std::path::Path, max_side: Option<f32>) -> Option<(TextureId, u32, u32)> {
    let img = image::open(path).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let rgba;
    let (tw, th) = match max_side {
        Some(max) if (w as f32).max(h as f32) > max => {
            let scale = max / (w as f32).max(h as f32);
            ((w as f32 * scale).round().max(1.0) as u32, (h as f32 * scale).round().max(1.0) as u32)
        }
        _ => (w, h),
    };
    let data: &[u8] = if (tw, th) == (w, h) {
        &img
    } else {
        rgba = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
        &rgba
    };
    let tex = ctx.new_texture_from_rgba8(tw as u16, th as u16, data);
    ctx.texture_set_filter(tex, FilterMode::Linear, MipmapFilterMode::None);
    Some((tex, tw, th))
}

// 피드 썸네일 전용 — 레이아웃(layout_masonry)이 이미 원본 종횡비 그대로 셀
// 크기를 정해뒀으니, 여기서는 그 크기로 그냥 리사이즈만 한다(자르지 않음).
fn load_cell_texture(ctx: &mut dyn RenderingBackend, path: &std::path::Path, w: u32, h: u32) -> Option<(TextureId, u32, u32)> {
    let img = image::open(path).ok()?.to_rgba8();
    let resized = image::imageops::resize(&img, w.max(1), h.max(1), image::imageops::FilterType::Triangle);
    let tex = ctx.new_texture_from_rgba8(resized.width() as u16, resized.height() as u16, &resized);
    ctx.texture_set_filter(tex, FilterMode::Linear, MipmapFilterMode::None);
    Some((tex, resized.width(), resized.height()))
}

// 핀터레스트 식 매이슨리(masonry) 레이아웃 — 사진마다 원본 종횡비를 그대로
// 유지한 채(자르지도, 레터박스로 여백을 채우지도 않고) 여러 열에 나눠 쌓는다.
// 매 사진을 "지금까지 쌓인 높이가 가장 낮은 열"에 넣어서 열들이 비슷한
// 높이로 맞춰지게 한다. 반환값은 files 와 같은 길이의 (x, y, w, h) 목록(스크롤
// 적용 전, area 왼쪽위 기준 상대좌표) + 전체 콘텐츠 높이.
fn layout_masonry(aspects: &[f32], grid_w: f32) -> (Vec<Rect>, f32) {
    let cols = ((grid_w / (THUMB + GAP)).floor().max(1.0)) as usize;
    let col_w = ((grid_w - GAP * (cols as f32 + 1.0)) / cols as f32).max(1.0);
    let mut col_heights = vec![0.0f32; cols];
    let mut rects = Vec::with_capacity(aspects.len());
    for &aspect in aspects {
        let (col, _) = col_heights.iter().enumerate().min_by(|a, b| a.1.total_cmp(b.1)).unwrap_or((0, &0.0));
        let cell_h = col_w / aspect.max(0.05);
        let x = GAP + col as f32 * (col_w + GAP);
        let y = GAP + col_heights[col];
        rects.push(Rect::new(x, y, col_w, cell_h));
        col_heights[col] += cell_h + GAP;
    }
    let content_h = col_heights.iter().cloned().fold(0.0, f32::max);
    (rects, content_h)
}

pub struct PhotosApp {
    files: Vec<PathBuf>,                        // 실제 디스크 경로(하위 폴더 포함), 랜덤으로 뽑힌 것만
    ids: Vec<String>,                           // files 와 같은 길이 — "폴더명/파일명" 또는 "파일명" 식별자
    aspects: Vec<f32>,                          // files 와 같은 길이 — 원본 가로/세로 비율(w/h)
    thumbs: Vec<Option<(TextureId, u32, u32)>>, // files 와 같은 길이 — 화면에 한 번이라도 보인 것만 Some
    scroll: f32,
    scroll_disp: f32,
    scrollbar_dragging: bool,
}

// 지금 당장은 게임 진행과 연동된 잠금 해제가 없어서, assets/photo(corpseImage/
// crackImage/hintImage/normalImage 네 폴더) 안의 사진 전부를 한꺼번에 보여주는
// 대신 이 개수만큼만 노출해둔다 — 나중에 진행 상황에 따라 사진이 하나씩 더
// 풀리는 시스템이 생기면, 여기 고정 개수 대신 그 진행값(예: fs 에 저장된
// "지금까지 푼 사진 개수")을 넘겨받게 바뀔 자리다. 나머지 사진들은 그대로
// assets/photo 에 남아있고 폴더에서 지우지 않는다 — 나중에 그 풀에서 골라 쓰면
// 된다.
const UNLOCKED_PHOTO_COUNT: usize = 10;

// assets/photo 전체를 훑어서 exclude 에 없는 것들 중 UNLOCKED_PHOTO_COUNT 장을
// 랜덤으로 뽑아 식별자만 돌려준다(디스크 경로는 PhotosApp::new() 가 다시 구한다
// — fs 에는 식별자 문자열만 저장하면 되고, 실행 파일 위치가 바뀌어도 안전하다).
// 정렬 후 앞쪽만 자르면 파일명 알파벳 순서상 앞선 폴더(corpseImage) 사진들로만
// 채워지므로, 네 폴더 전체에서 골고루 섞여 나오도록 랜덤 셔플(Fisher-Yates)한
// 뒤에 자른다.
fn pick_new_photos(exclude: &[String]) -> Vec<String> {
    let mut entries: Vec<(PathBuf, String)> = find_photo_dir().map(|dir| scan_photos(&dir)).unwrap_or_default();
    entries.retain(|(_, id)| !exclude.contains(id));
    let mut rng = Rng::new((miniquad::date::now() * 1e6) as u64);
    for i in (1..entries.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        entries.swap(i, j);
    }
    entries.truncate(UNLOCKED_PHOTO_COUNT);
    entries.into_iter().map(|(_, id)| id).collect()
}

// ????? 를 처음 여는 시점(게임을 새로 시작했거나, 예전 저장 파일이라 아직 한
// 번도 안 뽑아본 경우)에만 새로 뽑는다 — 이미 뽑아둔 게 있으면(fs.photos_current
// 가 비어있지 않으면) 그대로 둔다. desktop.rs::DesktopScene::new() 가 창을 열기
// 전에 미리 호출해서, PhotosApp::new() 는 항상 이미 정해진 목록만 받는다.
pub(crate) fn ensure_photos_selected(fs: &mut FileSystem) {
    if fs.photos_current.is_empty() {
        let picked = pick_new_photos(&fs.photos_seen);
        fs.photos_seen.extend(picked.iter().cloned());
        fs.photos_current = picked;
    }
}

// 재연구 업무 보고 메일을 실제로 보내면(desktop.rs::DeskAction::SendNewMail 참고)
// ????? 피드를 통째로 새로 뽑는다 — 지금까지 한 번이라도 나왔던 사진(fs.photos_seen)
// 은 전부 제외해서, 다시 봤던 사진이 나오는 일은 없다.
pub(crate) fn refresh_photos_feed(fs: &mut FileSystem) {
    let picked = pick_new_photos(&fs.photos_seen);
    fs.photos_seen.extend(picked.iter().cloned());
    fs.photos_current = picked;
}

impl PhotosApp {
    // ids 는 fs.photos_current 를 그대로 받는다 — 여기서 새로 뽑지 않는다
    // (ensure_photos_selected/refresh_photos_feed 가 그 몫을 담당).
    pub fn new(ids: Vec<String>) -> PhotosApp {
        let dir = find_photo_dir();
        let files: Vec<PathBuf> = ids.iter().map(|id| dir.as_deref().map(|d| d.join(id)).unwrap_or_default()).collect();
        // 매이슨리 레이아웃을 짜려면 사진마다 원본 종횡비를 미리 알아야 한다 —
        // 전체 픽셀을 디코드하는 대신 헤더만 읽는 image_dimensions() 를 써서
        // 사진이 많아도(사진 자체는 아직 하나도 안 올린 채로) 빠르게 끝난다.
        let aspects: Vec<f32> = files.iter().map(|p| image::image_dimensions(p).map(|(w, h)| w as f32 / h as f32).unwrap_or(1.0)).collect();
        let n = files.len();
        PhotosApp { files, ids, aspects, thumbs: vec![None; n], scroll: 0.0, scroll_disp: 0.0, scrollbar_dragging: false }
    }
}

impl App for PhotosApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, [0.0, 0.0, 0.0, 1.0]);

        if self.files.is_empty() {
            r.text(area.x + 10.0, area.y + 10.0, "(no photos in assets/photo)", 0.8, ui::GRAY);
            return AppAction::None;
        }

        let sb_w = 14.0;
        let grid_w = area.w - sb_w;
        let visible_h = area.h;
        let (rects, content_h) = layout_masonry(&self.aspects, grid_w);
        let max_scroll = (content_h + GAP - visible_h).max(0.0);
        self.scroll = self.scroll.clamp(0.0, max_scroll);
        if win.focused && win.wheel != 0.0 {
            self.scroll = (self.scroll - win.wheel * 40.0).clamp(0.0, max_scroll);
        }
        super::widgets::ease_scroll(&mut self.scroll_disp, self.scroll, win.dt, true);

        let mut action = AppAction::None;
        for (i, path) in self.files.iter().enumerate() {
            let cell = rects[i];
            let cx = area.x + cell.x;
            let cy = area.y + cell.y - self.scroll_disp;
            if cy + cell.h < area.y || cy > area.y + area.h {
                continue; // 화면 밖 — 그리지도, 굳이 지금 디코드하지도 않는다
            }
            if self.thumbs[i].is_none() {
                self.thumbs[i] = load_cell_texture(ctx, path, cell.w.round() as u32, cell.h.round() as u32);
            }
            if let Some((tex, ..)) = self.thumbs[i] {
                r.sprite(tex, cx, cy, cell.w, cell.h, WHITE);
            } else {
                r.rect(cx, cy, cell.w, cell.h, [0.15, 0.15, 0.15, 1.0]);
            }
            let hover = win.mouse.0 >= cx && win.mouse.0 < cx + cell.w && win.mouse.1 >= cy && win.mouse.1 < cy + cell.h;
            if hover {
                ui::border(r, cx, cy, cell.w, cell.h, WHITE);
                if win.mouse_clicked {
                    action = AppAction::OpenPhoto(self.ids[i].clone());
                }
            }
        }

        if max_scroll > 0.0 {
            let mut dragging = self.scrollbar_dragging;
            scrollbar(r, win, area.x + grid_w, area.y, sb_w, visible_h, (visible_h / (content_h + GAP)).clamp(0.05, 1.0), self.scroll_disp, &mut self.scroll, max_scroll, &mut dragging);
            self.scrollbar_dragging = dragging;
        }

        action
    }
}

// 사진 한 장을 별개의 창으로 보여주는 뷰어 — Photos 피드에서 썸네일을 클릭했을
// 때도, Explorer 의 Downloads 탭에서 이미 받은 사진을 다시 열었을 때도 똑같이
// 이 앱을 쓴다. assets.photos 인덱스가 아니라 assets/photo/ 안의 파일명으로
// 그때그때 디코드한다(그 사진들은 애초에 assets.photos 에 없다 — 위 모듈 설명
// 참고). show_download 가 true 일 때만 이미지 아래 회색 글자 "Download" 를
// 보여준다 — 이미 다운로드해서 Explorer 의 Downloads 탭에서 다시 연 사진은
// 또 받을 이유가 없으니 그 글자 자체를 아예 안 그린다(apps/mod.rs::open() 이
// fs.ever_downloaded 를 보고 이 값을 정해서 넘긴다).
pub struct PhotoViewerApp {
    filename: String,
    tex: Option<(TextureId, u32, u32)>,
    tried: bool,
    show_download: bool,
    download_flash: f32, // "Downloaded!" 문구를 잠깐 보여주는 타이머
}

impl PhotoViewerApp {
    pub fn new(filename: String, show_download: bool) -> PhotoViewerApp {
        PhotoViewerApp { filename, tex: None, tried: false, show_download, download_flash: 0.0 }
    }
}

impl App for PhotoViewerApp {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn update(&mut self, ctx: &mut dyn RenderingBackend, r: &mut Renderer, _assets: &Assets, area: Rect, win: &WinInput) -> AppAction {
        r.rect(area.x, area.y, area.w, area.h, [0.1, 0.1, 0.1, 1.0]);
        if !self.tried {
            self.tried = true;
            if let Some(dir) = find_photo_dir() {
                self.tex = load_scaled_texture(ctx, &dir.join(&self.filename), None);
            }
        }

        if let Some((tex, w, h)) = self.tex {
            let (iw, ih) = (w as f32, h as f32);
            let scale = (area.w / iw).min(area.h / ih);
            let (dw, dh) = (iw * scale, ih * scale);
            let dx = area.x + (area.w - dw) / 2.0;
            let dy = area.y + (area.h - dh) / 2.0;
            r.sprite(tex, dx, dy, dw, dh, WHITE);
        }

        if !self.show_download {
            return AppAction::None;
        }

        // 사진 위에 그대로 겹쳐 그리는 우하단 회색 글자 "Download" — 따로 자리를
        // 안 빼고 사진 자체의 오른쪽 아래 모서리 위에 얹는다. 뒤에 옅은 그림자를
        // 한 번 더 그려서(살짝 어긋난 검은 글자) 밝은 사진 위에서도 안 묻힌다.
        let label = if self.download_flash > 0.0 { "Downloaded" } else { "Download" };
        let tw = r.text_width(label, 0.75);
        let pad = 8.0;
        let tx = area.x + area.w - tw - pad;
        let ty = area.y + area.h - 16.0 - pad;
        let hover = win.mouse.0 >= tx - 6.0 && win.mouse.0 <= area.x + area.w && win.mouse.1 >= ty - 4.0 && win.mouse.1 <= area.y + area.h;
        let color = if hover { [0.9, 0.9, 0.9, 1.0] } else { [0.65, 0.65, 0.65, 1.0] };
        r.text(tx + 1.0, ty + 1.0, label, 0.75, [0.0, 0.0, 0.0, 0.6]);
        r.text(tx, ty, label, 0.75, color);
        self.download_flash = (self.download_flash - win.dt).max(0.0);

        if hover && win.mouse_clicked {
            self.download_flash = 2.0;
            return AppAction::DownloadPhoto(self.filename.clone());
        }
        AppAction::None
    }
}
