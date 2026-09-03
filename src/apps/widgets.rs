//! 여러 앱이 같이 쓰는 위젯: 아이콘 격자(탐색기/쓰레기통), 슬라이더, 스크롤바,
//! 아코디언 리스트, 스크롤 이징. ui.rs 의 베벨/버튼 같은 진짜 범용 원시 위젯과
//! 달리, 여기 있는 건 "앱 안에서 쓰는" 좀 더 조립된 위젯이라 apps 밑에 뒀다.

use crate::foundation::{display_name, FileId, Language};
use crate::gfx::{Assets, Rect, Renderer, CELL_H};
use crate::ui::*;

use super::WinInput;

// 목표 스크롤(scroll)을 향해 화면표시용 스크롤(disp)을 매 프레임 부드럽게 따라가게
// 한다. smooth 가 꺼져 있으면 예전처럼 즉시 딱 맞춰서(=하드 스냅) 이동한다.
pub(crate) fn ease_scroll(disp: &mut f32, target: f32, dt: f32, smooth: bool) {
    if smooth {
        *disp += (target - *disp) * (14.0 * dt).min(1.0);
    } else {
        *disp = target;
    }
}

// 세로 스크롤바: 트랙+핸들을 그리고, 트랙을 누르거나 핸들을 드래그해서 마우스로도
// 스크롤할 수 있게 한다. scroll/max_scroll 은 픽셀이든 행(row) 단위든 상관없이 그
// 비율만으로 동작한다 (호출부마다 단위가 달라도 됨). 핸들은 disp(부드럽게 따라가는
// 화면표시용 값) 기준으로 그리고, 드래그하면 scroll(목표값)을 갱신한다.
pub(crate) fn scrollbar(
    r: &mut Renderer, win: &WinInput,
    x: f32, y: f32, w: f32, h: f32,
    visible_frac: f32, disp: f32, scroll: &mut f32, max_scroll: f32, dragging: &mut bool,
) {
    sunken(r, x, y, w, h);
    let thumb_h = (h * visible_frac).clamp(12.0, h);
    let travel = (h - thumb_h).max(1.0);
    let thumb_y = y + travel * (disp / max_scroll.max(0.0001));
    raised(r, x, thumb_y, w, thumb_h);

    let track_hit = win.mouse.0 >= x - 2.0 && win.mouse.0 <= x + w + 2.0 && win.mouse.1 >= y && win.mouse.1 <= y + h;
    if win.mouse_clicked && track_hit {
        *dragging = true;
    }
    if *dragging && win.mouse_down {
        // 핸들 중앙이 마우스를 따라가도록 — 트랙 아무데나 눌러도 그 위치로 바로 점프한다.
        *scroll = (((win.mouse.1 - y - thumb_h / 2.0) / travel) * max_scroll).clamp(0.0, max_scroll);
    }
    if !win.mouse_down {
        *dragging = false;
    }
}

pub(crate) fn draw_slider(r: &mut Renderer, win: &WinInput, x: f32, y: f32, w: f32, name: &str, idx: i32, value: &mut f32, active: &mut i32) {
    label(r, x, y, name, BLACK);
    // 라벨 글자 높이(CELL_H=22)만큼 아래로 확실히 띄워야 트랙/핸들과 안 겹친다.
    let ty = y + 26.0;
    sunken(r, x, ty, w, 8.0);
    let hx = x + *value * (w - 12.0);
    raised(r, hx, ty - 4.0, 12.0, 16.0);
    r.text(x + w + 12.0, ty - 6.0, &format!("{}", (*value * 100.0) as i32), 1.0, BLACK);

    let over = win.mouse.0 >= x - 4.0
        && win.mouse.0 <= x + w + 8.0
        && win.mouse.1 >= ty - 8.0
        && win.mouse.1 <= ty + 16.0;
    if win.mouse_clicked && over {
        *active = idx;
    }
    if *active == idx && win.mouse_down {
        *value = ((win.mouse.0 - x) / (w - 12.0)).clamp(0.0, 1.0);
    }
}

// 스크롤 되는 아이콘 격자. 흰 배경 + 아이콘 + 이름 + 선택 강조(연한 파랑 채움 + 진한
// 파랑 테두리) + 스크롤바 + 바탕화면처럼 빈 자리를 끌면 생기는 고무줄(마퀴) 다중
// 선택. selected 는 이 위젯이 직접 갱신하는 다중 선택 목록, marquee_start/prev_down
// 은 드래그 상태를 프레임 사이에 들고 있으려고 호출부(앱)가 대신 보관해주는 것뿐이다
// (이 함수 자체는 매 프레임 새로 불려서 내부 상태를 못 가짐).
// 이번 프레임에 "그냥 클릭"된 항목 인덱스(더블클릭 판정용, 마퀴 드래그는 해당 없음)를 돌려준다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn icon_grid(
    r: &mut Renderer,
    assets: &Assets,
    win: &WinInput,
    area: Rect,
    items: &[(FileId, String, IconType)],
    selected: &mut Vec<usize>,
    marquee_start: &mut Option<(f32, f32)>,
    prev_down: &mut bool,
    scroll: &mut f32,
    scroll_disp: &mut f32,
    smooth: bool,
    sb_drag: &mut bool,
    lang: Language,
) -> Option<(usize, Rect)> {
    r.rect(area.x, area.y, area.w, area.h, WHITE);
    // 항목이 하나도 없어도(빈 폴더) 마퀴 드래그 자체는 그대로 동작해야 한다 — 여기서
    // 바로 반환하면 폴더가 비어있는 동안은 드래그를 시작할 수조차 없었다. cols/rows
    // 는 items.len()==0 이면 자연히 0이 되니 아래 로직이 그대로 안전하게 처리한다.
    let pad = 14.0; // 탭/사이드바 쪽과 살짝 더 여유를 두려고 8→14
    let cell_w = 92.0;
    let cell_h = 82.0; // 이름이 두 줄로 접혀도(wrap_two_lines) 다음 칸과 안 겹치게 여유를 둠
    let cols = (((area.w - pad - 10.0) / cell_w).floor() as usize).max(1);
    let total_rows = items.len().div_ceil(cols);
    let vis_rows = (((area.h - pad) / cell_h).floor() as usize).max(1);
    let max_scroll = total_rows.saturating_sub(vis_rows) as f32;

    // win.wheel 은 winman.rs 에서 이미 "마우스가 올라간 창"만 걸러서 주지만, 창 안에
    // 스크롤되는 영역이 여럿일 수 있으므로(예: 트리 사이드바 vs 이 격자) 마우스가
    // 이 격자 위에 있을 때만 먹는다.
    if area.contains(win.mouse.0, win.mouse.1) {
        *scroll -= win.wheel / 120.0;
    }
    *scroll = scroll.clamp(0.0, max_scroll);
    ease_scroll(scroll_disp, *scroll, win.dt, smooth);
    let scroll_px = *scroll_disp * cell_h;

    // area 로 클립해서 판정한다 — 안 그러면 스크롤 경계에 걸친(위/아래로 살짝만
    // 보이는) 줄의 칸이 실제로는 안 보이는 영역까지 클릭/마퀴 판정을 흘린다.
    let item_rect = |i: usize| -> Rect {
        let row = i / cols;
        let col = i % cols;
        let iy = area.y + pad + row as f32 * cell_h - scroll_px;
        let ix = area.x + pad + col as f32 * cell_w;
        Rect::new(ix, iy, cell_w - 6.0, cell_h - 4.0).intersect(&area)
    };

    // 빈 자리를 누르면 고무줄 선택 시작, 아이템 위를 누르면(아래 루프에서) 그 아이템만 선택.
    let over_any_item = (0..items.len()).any(|i| item_rect(i).contains(win.mouse.0, win.mouse.1));
    if win.mouse_clicked && !over_any_item && area.contains(win.mouse.0, win.mouse.1) {
        *marquee_start = Some(win.mouse);
        selected.clear();
    }
    if let Some(ms) = *marquee_start
        && win.mouse_down
    {
        let mr = Rect::new(ms.0.min(win.mouse.0), ms.1.min(win.mouse.1), (ms.0 - win.mouse.0).abs(), (ms.1 - win.mouse.1).abs());
        *selected = (0..items.len()).filter(|&i| item_rect(i).intersects(&mr)).collect();
    }
    if *prev_down && !win.mouse_down {
        *marquee_start = None;
    }
    *prev_down = win.mouse_down;

    // set_clip 은 스택이 아니라 통째로 덮어쓰는 값이라(gfx.rs::push_quad), 여기서
    // None 으로 지워버리면 winman.rs 가 app.update() 를 부르기 전에 이미 걸어둔
    // "창 밖으로 안 나가게" 클립까지 같이 풀려버린다 — 그러면 마우스를 창 밖까지
    // 끌고 나간 채 마퀴를 그릴 때 그 사각형이 창 경계를 뚫고 화면 전체에 그려진다.
    // 그래서 끝나고 나서 None 이 아니라 "들어올 때 걸려있던 클립"으로 되돌린다.
    let outer_clip = r.clip();
    let mut clicked = None;
    r.set_clip(Some(area));
    for row in 0..total_rows {
        let iy = area.y + pad + row as f32 * cell_h - scroll_px;
        if iy + cell_h < area.y || iy > area.y + area.h {
            continue; // 화면 밖 행은 건너뛴다 (연속 스크롤이라 행 단위로만 자를 수 없음)
        }
        for col in 0..cols {
            let i = row * cols + col;
            if i >= items.len() {
                break;
            }
            let (_id, name, icon) = &items[i];
            // 항목 이름은 원문 그대로 담겨 있다가 여기서 매 프레임 다시 번역된다 —
            // 창을 이미 연 채로 언어를 바꿔도 그 자리에서 바로 반영되게 하려고.
            let display = display_name(lang, name);
            let ix = area.x + pad + col as f32 * cell_w;
            let sel = selected.contains(&i);
            let cell_rect = Rect::new(ix, iy, cell_w - 6.0, cell_h - 4.0);
            if sel {
                r.rect(cell_rect.x, cell_rect.y, cell_rect.w, cell_rect.h, [0.78, 0.88, 1.0, 1.0]); // 연한 파랑 채움
                border(r, cell_rect.x, cell_rect.y, cell_rect.w, cell_rect.h, NAVY); // 진한 파랑 테두리
            }
            draw_icon(r, assets, icon, ix + cell_w / 2.0 - 19.0, iy + 2.0, 32.0);
            let lines = wrap_two_lines(r, &display, 0.8, cell_w - 6.0);
            let line_h = CELL_H * 0.8 + 2.0;
            for (li, line) in lines.iter().enumerate() {
                let tw = r.text_width(line, 0.8).min(cell_w - 6.0);
                r.text_clipped(ix + (cell_w - 6.0 - tw) / 2.0, iy + 38.0 + li as f32 * line_h, line, 0.8, BLACK, cell_w - 6.0);
            }
            let hit = cell_rect.intersect(&area);
            if win.mouse_clicked && marquee_start.is_none() && hit.contains(win.mouse.0, win.mouse.1) {
                // 마퀴 드래그가 아닌 단순 클릭 — 이미 다중 선택에 들어있는 항목이면
                // 그 선택을 유지한다(그래야 여러 개를 선택한 채로 눌러서 옮길 수 있다).
                if !selected.contains(&i) {
                    *selected = vec![i];
                }
                clicked = Some((i, cell_rect));
            }
        }
    }
    // 마퀴 박스도 아이템 목록과 같은 클립 안에서 그린다(clip 을 이미 풀어버린 뒤에
    // 그리면 창 밖으로 드래그했을 때 사각형이 화면 전체에 삐져나온다).
    if let Some(ms) = *marquee_start {
        let mr = Rect::new(ms.0.min(win.mouse.0), ms.1.min(win.mouse.1), (ms.0 - win.mouse.0).abs(), (ms.1 - win.mouse.1).abs());
        r.rect(mr.x, mr.y, mr.w, mr.h, [0.2, 0.4, 0.9, 0.25]);
        border(r, mr.x, mr.y, mr.w.max(1.0), mr.h.max(1.0), [0.2, 0.4, 0.9, 0.9]);
    }
    r.set_clip(outer_clip);

    if max_scroll > 0.0 {
        let sb_x = area.x + area.w - 8.0;
        let frac = vis_rows as f32 / total_rows as f32;
        scrollbar(r, win, sb_x, area.y, 8.0, area.h, frac, *scroll_disp, scroll, max_scroll, sb_drag);
    }
    clicked
}

// 펼쳐진 아코디언 리스트를 그린다. 항목이 MAX_ROWS 개보다 많으면 리스트 높이를
// MAX_ROWS 줄로 고정하고 리스트 자기 자신만 내부 스크롤한다(스크롤바도 리스트 안쪽
// 오른편에 그림) — 탭 전체를 밀어내리는 대신 리스트만 스크롤되게 하려는 것.
// 클릭된 항목의 인덱스, 이 리스트가 실제로 차지한 높이, 이번 프레임에 휠을 이
// 리스트가 먹었는지(호출부가 탭 전체 스크롤에 또 안 쓰게)를 돌려준다.
#[allow(clippy::too_many_arguments)]
pub(crate) fn accordion_list(
    r: &mut Renderer, win: &WinInput,
    x: f32, y: f32, w: f32,
    options: &[&str], selected_idx: usize,
    scroll: &mut f32, scroll_disp: &mut f32, sb_drag: &mut bool, smooth: bool,
    max_rows: usize,
) -> (Option<usize>, f32, bool) {
    let row_h = ACCORDION_ROW_H;
    let pad = ACCORDION_LIST_PAD;
    let visible_rows = options.len().min(max_rows);
    let content_h = visible_rows as f32 * row_h;
    let panel_h = content_h + pad * 2.0; // sunken 테두리가 보이도록 안쪽에 여백을 둔다.
    let max_scroll = options.len().saturating_sub(max_rows) as f32 * row_h;
    // 마우스가 이 리스트 위에 있을 때만 휠을 먹는다 — 안 그러면 탭 전체 스크롤과
    // 같이 움직여서 둘 다 굴러가는 것처럼 보이는 문제가 생긴다.
    let hover_list = win.mouse.0 >= x && win.mouse.0 < x + w && win.mouse.1 >= y && win.mouse.1 < y + panel_h;
    let delta = -win.wheel / 120.0 * row_h;
    // 이미 스크롤 끝(최상단/최하단)이라 더 이상 그 방향으로 못 움직이면 휠을 먹지 않고
    // 그대로 흘려보내서, 바깥(탭 전체) 스크롤이 이어받아 움직이도록 한다.
    let can_scroll = if delta < 0.0 { *scroll > 0.0 } else if delta > 0.0 { *scroll < max_scroll } else { false };
    // win.wheel 은 winman.rs 에서 이미 마우스가 올라간 창으로 걸러져서 오므로
    // win.focused 는 따로 확인할 필요 없다.
    let wheel_consumed = hover_list && win.wheel != 0.0 && can_scroll;
    if wheel_consumed {
        *scroll += delta;
    }
    *scroll = scroll.clamp(0.0, max_scroll);
    ease_scroll(scroll_disp, *scroll, win.dt, smooth);

    accordion_panel(r, x, y, w, panel_h);
    let has_sb = max_scroll > 0.0;
    let (ix, iw) = (x + pad, w - pad * 2.0);
    let row_w = if has_sb { iw - 10.0 } else { iw };

    // 호출부가 이미 걸어둔 바깥(탭 전체 스크롤) 클립이 있으면 그 경계 안쪽으로만
    // 더 좁힌다 — 그냥 덮어써버리면 이 리스트가 스크롤로 바깥 뷰포트 위로 올라갔을 때
    // 바깥 클립 경계를 무시하고 그대로 그려져서 창 밖/헤더 위로 삐져나와 보인다.
    let outer_clip = r.clip();
    let inner_rect = Rect::new(ix, y + pad, iw, content_h);
    let inner_clip = match outer_clip {
        Some(o) => inner_rect.intersect(&o),
        None => inner_rect,
    };
    r.set_clip(Some(inner_clip));
    let mut clicked = None;
    for (i, name) in options.iter().enumerate() {
        let ry = y + pad + i as f32 * row_h - *scroll_disp;
        if ry + row_h > y + pad && ry < y + pad + content_h && accordion_row(r, win, ix, ry, row_w, name, i == selected_idx) {
            clicked = Some(i);
        }
    }
    // None 이 아니라 바깥 클립으로 되돌린다 — 스크롤바도 같은 바깥 뷰포트 밖으로
    // 삐져나가면 안 되므로 클립이 걸린 채로 그려야 한다.
    r.set_clip(outer_clip);

    if has_sb {
        let sb_x = x + w - pad - 8.0;
        let frac = visible_rows as f32 / options.len() as f32;
        scrollbar(r, win, sb_x, y + pad, 8.0, content_h, frac, *scroll_disp, scroll, max_scroll, sb_drag);
    }
    (clicked, panel_h, wheel_consumed)
}
