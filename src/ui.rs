//! Windows 9x 스타일 위젯: 베벨/버튼/파일 아이콘.

use miniquad::TextureId;

use crate::apps::WinInput;
use crate::foundation::{FileKind, FileNode};
use crate::gfx::{Assets, Color, Renderer, CELL_H};

// Windows 9x 팔레트
pub const TEAL: Color = [0.0, 0.5, 0.5, 1.0]; // 기본 바탕화면 색
pub const FACE: Color = [0.75, 0.75, 0.75, 1.0]; // 위젯 회색 (192)
pub const WHITE: Color = [1.0, 1.0, 1.0, 1.0]; // 하이라이트
pub const GRAY: Color = [0.5, 0.5, 0.5, 1.0]; // 그림자 (128)
pub const BLACK: Color = [0.0, 0.0, 0.0, 1.0];
pub const NAVY: Color = [0.0, 0.0, 0.5, 1.0]; // 타이틀바
pub const DARK_GRAY: Color = [0.22, 0.22, 0.22, 1.0]; // 짙은 회색 타이틀바 (메시지 패널 등)

pub fn label(r: &mut Renderer, x: f32, y: f32, text: &str, color: Color) {
    r.text(x, y, text, 1.0, color);
}

// 한 번만 접는다 — max_w 안에 들어오는 가장 긴 접두사와 그 나머지를 돌려준다.
// 나머지(두 번째 값)는 여전히 max_w 를 넘을 수 있다 — wrap_two_lines 는 그 나머지를
// 그대로 두 번째 줄로 쓰고, wrap_lines 는 나머지가 다 들어올 때까지 이걸 반복한다.
//
// 글자 단위로 먼저 "아직 안 넘는 가장 긴 접두사"를 찾고, 그 안에 띄어쓰기가 있으면
// (그리고 거기서 끊어도 첫 줄이 너무 짧아지지 않으면) 라틴 단어가 중간에 잘리지
// 않도록 그 마지막 공백에서 끊는다 — 예전엔 반대로 "문자열에 공백이 하나라도
// 있으면" 무조건 단어 단위로 먼저 접었는데, 일본어처럼 원래 띄어쓰기가 없는 문장에
// "PALACE OS" 같은 라틴 단어가 하나 섞이면 그 공백 하나가 (실제로 그 자리에서
// 줄이 넘칠 이유가 없는데도) 마치 엔터를 친 것처럼 강제로 줄을 끊어버리는
// 버그였다("PALACE"/"OS" 가 서로 다른 줄에 떨어져 보인다는 제보로 발견).
fn split_line_once<'a>(r: &Renderer, text: &'a str, scale: f32, max_w: f32) -> (&'a str, &'a str) {
    // split_byte 는 "아직 max_w 를 안 넘는 가장 긴 접두사" 의 끝 지점이어야 한다 —
    // 넘어서기 직전(마지막으로 안전하게 들어맞는 접두사)에서 끊는다.
    let mut split_byte = 0;
    for (i, (byte_idx, _)) in text.char_indices().enumerate() {
        if i == 0 {
            continue;
        }
        if r.text_width(&text[..byte_idx], scale) > max_w {
            break;
        }
        split_byte = byte_idx;
    }
    if split_byte == 0 {
        // 글자 하나도 못 들어갈 만큼 좁다 — 더 못 쪼개니 통째로 몰아준다(호출부의
        // clip 이 최종 안전장치).
        return (text, "");
    }
    // 접두사 안에 있는 마지막 공백에서 끊을 수 있으면 그쪽을 우선한다 — 단, 그
    // 공백이 접두사 시작 쪽에 너무 치우쳐 있으면(첫 줄이 접두사의 절반도 안 남을
    // 만큼 짧아지면) 오히려 어색하니 그냥 글자 경계를 그대로 쓴다. 공백이 접두사
    // 끝부분(잘리는 지점) 가까이에 있는 "평범한 단어 경계" 케이스에서만 적용된다.
    if let Some(last_space) = text[..split_byte].rfind(' ')
        && last_space * 2 >= split_byte
    {
        return (text[..last_space].trim_end(), text[last_space..].trim_start());
    }
    (&text[..split_byte], &text[split_byte..])
}

// 폭 max_w 를 넘는 텍스트를 최대 두 줄로 접는다 — 잘려 보이는(text_clipped) 대신
// 전체 글자가 다 보이게 하고 싶은 라벨(아이콘 이름, 사이드바 탭 등)에서 쓴다. 두
// 줄로도 다 안 들어가면 두 번째 줄이 여전히 max_w 를 넘을 수 있다 — 그 경우 호출부가
// (필요하면 text_clipped 로) 알아서 처리해야 한다. 항상 다 보이게 하고 싶으면 대신
// wrap_lines 를 쓴다.
pub fn wrap_two_lines<'a>(r: &Renderer, text: &'a str, scale: f32, max_w: f32) -> Vec<&'a str> {
    if r.text_width(text, scale) <= max_w {
        return vec![text];
    }
    let (l1, l2) = split_line_once(r, text, scale, max_w);
    if l2.is_empty() { vec![l1] } else { vec![l1, l2] }
}

// split_line_once 를 다 접힐 때까지 반복해서, 줄 수 제한 없이 max_w 안에 실제로
// 들어가는 만큼씩 접는다(\n 은 문단 구분으로 유지) — 언어에 따라 문자 폭이 크게
// 다른 다국어 UI(라틴 vs 한글/한자/가나)에서도 정확한 렌더링 폭을 기준으로
// 줄바꿈되므로, "문자 하나당 고정 폭"으로 어림잡던 예전 방식들과 달리 컨테이너나
// 창 밖으로 텍스트가 넘쳐 잘려 보이는 문제가 없다. Mail 본문, 설치 마법사 안내문,
// 휴지통/압축파일 안내문처럼 여러 줄에 걸친 긴 문단을 접을 때 이 함수 하나로
// 통일해서 쓴다.
pub fn wrap_lines(r: &Renderer, text: &str, scale: f32, max_w: f32) -> Vec<String> {
    let mut out = Vec::new();
    for para in text.split('\n') {
        if para.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut rest = para;
        while !rest.is_empty() {
            // split_line_once 의 글자 단위 폴백은 "아직 안 넘는 가장 긴 접두사"를
            // char_indices 로 훑는데, 그 마지막 글자의 시작 위치까지만 검사하고
            // "전체 문자열이 그대로 다 들어가는지"는 검사하지 않는다 — 그래서
            // 먼저 확인 안 하고 바로 넘기면, 전체가 max_w 안에 이미 들어가는
            // 경우에도 항상 마지막 한 글자가 다음 줄로 잘못 밀려났다(엔터를
            // 안 쳤는데 마지막 글자가 다음 줄에 따로 써지는 것처럼 보이던 버그).
            if r.text_width(rest, scale) <= max_w {
                out.push(rest.to_string());
                break;
            }
            let (line, remainder) = split_line_once(r, rest, scale, max_w);
            out.push(line.to_string());
            if remainder.len() == rest.len() {
                // split_line_once 가 더 못 쪼갠 경우(글자 하나도 안 들어갈 만큼
                // 좁음) — 무한루프 방지로 강제 종료(호출부의 clip 이 최종 안전장치).
                break;
            }
            rest = remainder;
        }
    }
    out
}

// wrap_lines 와 똑같이 접되, 각 줄이 원본 텍스트의 몇 번째 글자(char 인덱스)부터
// 시작하는지도 같이 돌려준다 — 클릭한 화면 좌표를 원본 문자열의 커서 위치로
// 되짚거나(줄을 찾고 그 줄 안에서 x 위치로 글자를 찾은 뒤 시작 오프셋을 더한다),
// 반대로 커서 위치를 화면 좌표로 그리려고(어느 줄의 몇 번째 글자인지 찾는다) 할
// 때 쓴다. Mail 작성 화면의 본문처럼 여러 줄로 접히는 입력칸에서 클릭한 자리부터
// 편집할 수 있게 하려면 필요하다.
pub fn wrap_with_offsets(r: &Renderer, text: &str, scale: f32, max_w: f32) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut char_pos = 0usize;
    for para in text.split('\n') {
        if para.is_empty() {
            out.push((String::new(), char_pos));
            char_pos += 1; // '\n' 자체
            continue;
        }
        let mut rest = para;
        let mut rest_pos = char_pos;
        loop {
            if r.text_width(rest, scale) <= max_w {
                out.push((rest.to_string(), rest_pos));
                rest_pos += rest.chars().count();
                break;
            }
            let (line, remainder) = split_line_once(r, rest, scale, max_w);
            let consumed = rest.chars().count() - remainder.chars().count();
            out.push((line.to_string(), rest_pos));
            if remainder.len() == rest.len() {
                rest_pos += consumed;
                break;
            }
            rest_pos += consumed;
            rest = remainder;
        }
        char_pos = rest_pos + 1; // 다음 문단 시작 — 방금 지나온 '\n' 만큼 하나 더
    }
    out
}

// 한 줄짜리 텍스트에서, 텍스트 시작 지점부터 rel_x 만큼 떨어진 화면 좌표에 가장
// 가까운 글자 경계(char 인덱스)를 찾는다 — 클릭한 자리에 커서를 놓는 용도. 각
// 글자의 가운데를 기준으로 그 앞/뒤 중 더 가까운 쪽으로 판정한다(그래야 글자
// 왼쪽 절반을 클릭하면 그 글자 앞에, 오른쪽 절반을 클릭하면 그 글자 뒤에 커서가
// 붙어서 자연스럽다).
pub fn char_index_at_x(r: &Renderer, text: &str, scale: f32, rel_x: f32) -> usize {
    let mut prev_w = 0.0;
    for (i, (byte_idx, ch)) in text.char_indices().enumerate() {
        let ch_w = r.text_width(&text[byte_idx..byte_idx + ch.len_utf8()], scale);
        if rel_x < prev_w + ch_w / 2.0 {
            return i;
        }
        prev_w += ch_w;
    }
    text.chars().count()
}

// 한 줄로 강제하고 싶은데(예: 목록의 제목 줄) max_w 를 넘으면, 줄바꿈 대신 뒤를
// 잘라내고 "..." 를 붙인다. text_clipped 처럼 그냥 잘리기만 하는 게 아니라 "더
// 있다" 는 걸 눈에 보이게 알려주고 싶을 때 쓴다. 원본을 그대로 못 빌려 쓰고(끝에
// "..." 를 붙여야 하니) String 을 새로 만들어 돌려준다.
pub fn truncate_ellipsis(r: &Renderer, text: &str, scale: f32, max_w: f32) -> String {
    if r.text_width(text, scale) <= max_w {
        return text.to_string();
    }
    const ELLIPSIS: &str = "...";
    let avail = (max_w - r.text_width(ELLIPSIS, scale)).max(0.0);
    let mut split_byte = 0;
    for (i, (byte_idx, _)) in text.char_indices().enumerate() {
        if i == 0 {
            continue;
        }
        if r.text_width(&text[..byte_idx], scale) > avail {
            break;
        }
        split_byte = byte_idx;
    }
    format!("{}{ELLIPSIS}", &text[..split_byte])
}

// ---------------- 커서 ----------------
//
// 시스템 커서 대신 assets/cursor.png(아이콘 스프라이트 시트)에서 잘라 그린다. 일부
// PC(그래픽 드라이버 등)에서 OS 커서가 안 보이는 문제가 있어서, OS 커서는 숨기고 항상
// 이걸로 대신 그려 확실히 보이게 한다. 상황(창 가장자리/드래그/텍스트/클릭 가능
// 영역 등)에 따라 모양이 바뀐다.

// 딱 세 종류만 쓴다: 기본 화살표, 크기조절 화살표(방향별), 선택 가능한 요소 위에서만
// 나오는 손 모양.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKind {
    #[default]
    Arrow,
    ResizeNS,   // 위/아래 가장자리
    ResizeEW,   // 좌/우 가장자리
    ResizeNESW, // 우상단/좌하단 모서리 ("/" 대각선)
    ResizeNWSE, // 좌상단/우하단 모서리 ("\" 대각선)
    Hand,       // 클릭 가능한 아이콘/버튼 위
}

const CURSOR_TEX_W: f32 = 200.0;
const CURSOR_TEX_H: f32 = 113.0;

// 시트 안에서 각 아이콘의 (x0,y0,x1,y1) 픽셀 범위(포함)와 핫스팟(마우스가 가리키는
// 정확한 지점) 픽셀 좌표. Arrow 는 팁(좌상단)이 핫스팟, 나머지는 대체로 중앙.
// ResizeNESW/NWSE 는 실제 스프라이트 모양("/" 인지 "\" 인지)을 직접 확대해서 눈으로
// 확인하고 맞춘 것 — 이전엔 이름과 반대로 잘못 매핑돼 있었다.
fn cursor_sprite(kind: CursorKind) -> (f32, f32, f32, f32, f32, f32) {
    match kind {
        CursorKind::Arrow => (2.0, 1.0, 19.0, 30.0, 2.0, 1.0),
        CursorKind::ResizeNWSE => (93.0, 1.0, 111.0, 30.0, 102.0, 15.5), // "\" 모양
        CursorKind::ResizeEW => (169.0, 1.0, 196.0, 30.0, 182.5, 15.5),
        CursorKind::ResizeNESW => (93.0, 38.0, 111.0, 71.0, 102.0, 54.5), // "/" 모양
        CursorKind::ResizeNS => (181.0, 38.0, 193.0, 71.0, 187.0, 54.5),
        CursorKind::Hand => (128.0, 80.0, 151.0, 111.0, 138.0, 84.0),
    }
}

pub fn draw_cursor(r: &mut Renderer, tex: TextureId, kind: CursorKind, x: f32, y: f32, scale: f32) {
    // 슬라이더를 맨 왼쪽까지 끌면 0 이 될 수 있는데, 그러면 커서가 아예 안 보여서
    // 되돌릴 수도 없게 된다 — 최소 크기를 보장해둔다.
    let scale = scale.max(0.15);
    let (x0, y0, x1, y1, hx, hy) = cursor_sprite(kind);
    let (px, py) = (x.floor(), y.floor());
    let w = (x1 - x0 + 1.0) * scale;
    let h = (y1 - y0 + 1.0) * scale;
    let dx = px - (hx - x0) * scale;
    let dy = py - (hy - y0) * scale;
    let u0 = x0 / CURSOR_TEX_W;
    let v0 = y0 / CURSOR_TEX_H;
    let u1 = (x1 + 1.0) / CURSOR_TEX_W;
    let v1 = (y1 + 1.0) / CURSOR_TEX_H;
    r.sprite_uv(tex, dx, dy, w, h, u0, v0, u1, v1, WHITE);
}

// 돌출된(raised) 3D 테두리: 버튼/패널 기본 모양.
pub fn raised(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32) {
    r.rect(x, y, w, h, FACE);
    r.rect(x, y, w, 1.0, WHITE); // 위 하이라이트
    r.rect(x, y, 1.0, h, WHITE); // 왼쪽 하이라이트
    r.rect(x, y + h - 1.0, w, 1.0, BLACK); // 아래 외곽 그림자
    r.rect(x + w - 1.0, y, 1.0, h, BLACK); // 오른쪽 외곽 그림자
    r.rect(x + 1.0, y + h - 2.0, w - 2.0, 1.0, GRAY); // 아래 내부 그림자
    r.rect(x + w - 2.0, y + 1.0, 1.0, h - 2.0, GRAY); // 오른쪽 내부 그림자
}

// 눌린(sunken) 3D 테두리: 눌린 버튼/입력창.
pub fn sunken(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32) {
    r.rect(x, y, w, h, FACE);
    r.rect(x, y, w, 1.0, GRAY);
    r.rect(x, y, 1.0, h, GRAY);
    r.rect(x, y + h - 1.0, w, 1.0, WHITE);
    r.rect(x + w - 1.0, y, 1.0, h, WHITE);
    r.rect(x + 1.0, y + 1.0, w - 2.0, 1.0, BLACK);
    r.rect(x + 1.0, y + 1.0, 1.0, h - 2.0, BLACK);
}

pub fn panel(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32) {
    raised(r, x, y, w, h);
}

// 고전 대화상자의 "그룹 박스" — 얇게 파인 테두리 위쪽 왼편에 라벨이 테두리 선을
// 뚫고 걸쳐 있는 모양(예: "Options", "Reserved drive letters"). (x,y) 는 테두리
// 자체의 좌상단이고, 라벨은 그 위쪽 선에 걸치도록 세로로 절반씩 걸쳐 그린다 — 그래서
// 호출부는 박스 위에 라벨 한 줄 높이만큼 여유를 미리 비워둬야 한다.
pub fn group_box(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, label: &str) {
    // 파인(etched) 느낌: 위/왼쪽은 그림자색, 아래/오른쪽은 하이라이트 — sunken() 처럼
    // 이중선은 아니고 진짜 대화상자 그룹박스처럼 얇은 1px 단일선.
    r.rect(x, y, w, 1.0, GRAY);
    r.rect(x, y, 1.0, h, GRAY);
    r.rect(x, y + h - 1.0, w, 1.0, WHITE);
    r.rect(x + w - 1.0, y, 1.0, h, WHITE);

    let scale = 0.85;
    let lh = CELL_H * scale;
    let lw = r.text_width(label, scale);
    // 라벨 밑에 깔린 테두리 선을 FACE 색으로 지워서 선이 라벨 글자 뒤로 "뚫고 지나가는"
    // 대신 라벨 앞뒤로 끊겨 보이게 한다 — 진짜 Win9x 그룹박스가 이렇게 생겼다.
    r.rect(x + 6.0, y - lh / 2.0, lw + 6.0, lh, FACE);
    r.text(x + 9.0, y - lh / 2.0 + 1.0, label, scale, BLACK);
}

// 창 안 앱에서 쓰는 버튼: 마우스 상태를 WinInput 에서 바로 가져온다. 클릭되면 true.
pub fn button(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, label: &str, win: &WinInput) -> bool {
    raw_button(r, x, y, w, h, label, win.mouse.0, win.mouse.1, win.mouse_down, win.mouse_clicked)
}

fn raw_button(
    r: &mut Renderer,
    x: f32, y: f32, w: f32, h: f32,
    label: &str,
    mx: f32, my: f32,
    down: bool, clicked: bool,
) -> bool {
    let hover = mx >= x && mx < x + w && my >= y && my < y + h;
    let pressed = hover && down;

    if pressed {
        sunken(r, x, y, w, h);
    } else {
        raised(r, x, y, w, h);
    }

    // 라벨은 strings.rs::t() 로 언어별 문구를 받는 경우가 대부분이라, 버튼 폭을 영어 기준
    // 대충 잡아둔 호출부에서 한국어/일본어 번역이 그보다 넓으면(예: "Download" →
    // "다운로드") 가운데 정렬 텍스트가 버튼 밖으로 그대로 삐져나가 옆 UI와
    // 겹쳐 보이는 문제가 있었다(제보받음). 정상적으로 다 들어가면 그대로
    // 가운데 정렬하되, 안 들어가면 왼쪽 여백까지만 당기고 오른쪽 여백에서
    // 잘라서(text_clipped) 최소한 버튼 밖으로 새지는 않게 막는다 — 각 호출부의
    // 폭 계산 실수를 다 찾아 고치지 않아도 되는 마지막 안전장치.
    let tw = r.text_width(label, 1.0);
    let pad = 3.0;
    let tx = (x + (w - tw) / 2.0).max(x + pad);
    let avail = (x + w - pad - tx).max(0.0);
    let ty = y + (h - CELL_H) / 2.0;
    let off = if pressed { 1.0 } else { 0.0 };
    r.text_clipped(tx + off, ty + off, label, 1.0, BLACK, avail);

    hover && clicked
}

// 체크박스: 라벨을 왼쪽에, 박스를 라벨 오른쪽에 그리고, 클릭되면 checked 를 토글한다.
// 체크됐을 때는 파란 네모로 채우는 대신 체크(✓) 모양을 그린다.
pub fn checkbox(r: &mut Renderer, x: f32, y: f32, label_text: &str, checked: &mut bool, win: &WinInput) {
    let s = 16.0;
    let tw = r.text_width(label_text, 1.0);
    r.text(x, y - 3.0, label_text, 1.0, BLACK);

    let bx = x + tw + 8.0;
    sunken(r, bx, y, s, s);
    if *checked {
        draw_check(r, bx + 2.0, y + 2.0, s - 4.0);
    }

    let hover = win.mouse.0 >= x && win.mouse.0 < bx + s && win.mouse.1 >= y && win.mouse.1 < y + s;
    if hover && win.mouse_clicked {
        *checked = !*checked;
    }
}

// s 크기 안에 짧은 획(왼쪽 아래) + 긴 획(오른쪽 위) 두 개로 체크 표시를 점선으로 찍는다.
fn draw_check(r: &mut Renderer, x: f32, y: f32, s: f32) {
    for i in 0..4 {
        let t = i as f32 / 3.0;
        r.rect(x + t * s * 0.35, y + s * 0.45 + t * s * 0.35, 2.0, 2.0, NAVY);
    }
    for i in 0..7 {
        let t = i as f32 / 6.0;
        r.rect(x + s * 0.35 + t * s * 0.65, y + s * 0.8 - t * s * 0.8, 2.0, 2.0, NAVY);
    }
}

// ---------------- 아코디언 (해상도/프레임레이트처럼 옵션이 많을 때) ----------------

pub const ACCORDION_HEADER_H: f32 = 26.0;
pub const ACCORDION_ROW_H: f32 = 22.0;
// 펼쳐진 리스트 테두리 안쪽 여백 — 행들이 패널 가장자리까지 꽉 채워버리면 sunken()
// 이 그린 파인 테두리 선을 그대로 덮어버려서 파인 느낌이 하나도 안 보이게 된다.
pub const ACCORDION_LIST_PAD: f32 = 3.0;

// 헤더 한 줄: "라벨: 현재값" 을 보여주고 작은 삼각형 화살표(펼침▾/접힘▸)로 상태를
// 나타낸다. 펼쳐졌을 때만 움푹 파인 느낌(펼쳐진 리스트와 이어지는 느낌), 닫혀 있을
// 땐 보통 버튼처럼 돌출된 느낌. 클릭되면 true (호출부에서 펼침/접힘을 토글).
// 진짜 Win9x 콤보박스 느낌으로: 왼쪽은 읽기전용 텍스트 필드(sunken), 오른쪽에
// 따로 떨어진 정사각형 버튼(raised/펼쳤을 땐 sunken)에 ▼ 화살표.
pub fn accordion_header(r: &mut Renderer, win: &WinInput, x: f32, y: f32, w: f32, label: &str, current: &str, expanded: bool) -> bool {
    let h = ACCORDION_HEADER_H;
    const BTN_W: f32 = 20.0;
    let field_w = (w - BTN_W).max(10.0);

    sunken(r, x, y, field_w, h);
    let text = format!("{label}: {current}");
    r.text_clipped(x + 6.0, y + (h - CELL_H) / 2.0 + 2.0, &text, 1.0, BLACK, field_w - 10.0);

    let btn_x = x + field_w;
    if expanded {
        sunken(r, btn_x, y, BTN_W, h);
    } else {
        raised(r, btn_x, y, BTN_W, h);
    }
    draw_accordion_arrow(r, btn_x + BTN_W / 2.0, y + h / 2.0);

    let hover = win.mouse.0 >= x && win.mouse.0 < x + w && win.mouse.1 >= y && win.mouse.1 < y + h;
    hover && win.mouse_clicked
}

// 아래를 가리키는 작은 삼각형 — 콤보박스 버튼의 ▼ 표시. 실제 Win9x 콤보박스는
// 펼쳐져 있어도 화살표 모양이 안 바뀌므로(항상 아래 방향) 상태와 무관하게 고정.
fn draw_accordion_arrow(r: &mut Renderer, cx: f32, cy: f32) {
    for i in 0..4 {
        let t = i as f32;
        let half = 4.0 - t;
        r.rect(cx - half, cy - 2.0 + t, half * 2.0, 1.0, BLACK);
    }
}

// 펼쳐진 아코디언 목록 전체를 감싸는 리스트박스 테두리(움푹 파인 베벨) — "열었을 때
// 그 부분만" 파인 느낌을 준다. 그 위에 행들이 빈틈없이 그려지므로 안쪽은 따로 안 채운다.
pub fn accordion_panel(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32) {
    sunken(r, x, y, w, h);
}

// 펼쳐진 아코디언 안의 선택 항목 한 줄. 흰 배경 + 파란 선택 강조(원래 색감).
// 클릭되면 true.
pub fn accordion_row(r: &mut Renderer, win: &WinInput, x: f32, y: f32, w: f32, label: &str, selected: bool) -> bool {
    let h = ACCORDION_ROW_H;
    let hover = win.mouse.0 >= x && win.mouse.0 < x + w && win.mouse.1 >= y && win.mouse.1 < y + h;
    if selected {
        r.rect(x, y, w, h, NAVY);
    } else if hover {
        r.rect(x, y, w, h, [0.82, 0.88, 0.98, 1.0]);
    } else {
        r.rect(x, y, w, h, WHITE);
    }
    let color = if selected { WHITE } else { BLACK };
    r.text_clipped(x + 14.0, y + 2.0, label, 0.9, color, w - 20.0);
    hover && win.mouse_clicked
}

// ---------------- 파일 아이콘 ----------------

#[derive(Clone, Copy)]
pub enum IconType {
    Txt,
    Mp4,
    Lock,
    Folder,
    Mail,
    Tar,
    Installer,
    HexTool,
    Computer,
    Img,
    PhotosApp, // 바탕화면의 Photos 앱 — 낱장 사진 파일(Img)과 구분되는 "사진첩" 아이콘
    Envelope, // Mail 폴더 트리의 Inbox 아이콘
    RecycleEmpty,
    RecycleFull,
}

// 목록/트리에서 "이 아이콘은 폴더처럼 안을 열어볼 수 있는 대상인가" — 휴지통도
// FileKind 상으로는 그냥 Folder(이름만 "Recycle Bin")라 드래그/열기 동작은 이미
// FileKind::Folder 매칭만으로 다 되지만, 아이콘이 IconType::Folder 가 아니어서
// (비어있는지에 따라 RecycleEmpty/RecycleFull) "폴더처럼 취급"해야 하는 곳들
// (탐색기 목록의 크기 칸 숨김, 트리의 +/- 확장 박스)에서 놓칠 수 있다 — 그 판정을
// 한 군데로 모았다.
pub fn is_folder_like(icon: &IconType) -> bool {
    matches!(icon, IconType::Folder | IconType::RecycleEmpty | IconType::RecycleFull)
}

// 노드 이름까지 봐야 하는 특수 아이콘(휴지통)이 있어서 FileKind 대신 FileNode 전체를
// 받는다 — "Recycle Bin" 이라는 이름의 Folder 는 일반 폴더 아이콘 대신 휴지통
// 아이콘(비었는지에 따라 두 종류)을 쓴다. 그 외엔 FileKind 만으로 정해진다.
pub fn icon_of(node: &FileNode) -> IconType {
    match &node.kind {
        FileKind::Txt(_) => IconType::Txt,
        FileKind::Mp4 => IconType::Mp4,
        FileKind::Lock { .. } => IconType::Lock,
        FileKind::Folder { children } if node.name == "Recycle Bin" => {
            if children.is_empty() { IconType::RecycleEmpty } else { IconType::RecycleFull }
        }
        FileKind::Folder { .. } => IconType::Folder,
        FileKind::Mail { .. } => IconType::Mail,
        FileKind::Explorer => IconType::Computer,
        FileKind::Tar => IconType::Tar,
        FileKind::Installer => IconType::Installer,
        FileKind::HexTool => IconType::HexTool,
        FileKind::Img(_) => IconType::Img,
        FileKind::PhotoGallery => IconType::PhotosApp,
        FileKind::Photo(_) => IconType::Img,
        FileKind::Deleted => IconType::Folder, // 그 무엇에서도 더는 참조 안 되니 실제로 그려질 일이 없다
    }
}

// s 크기의 파일 아이콘을 텍스처로 그린다 (Windows 98 아이콘 팩에서 뽑아온 PNG).
// Tar/Installer/HexTool 은 아직 전용 PNG 에셋이 없어서 텍스처 대신 직접 그리는
// 모양으로 대신한다(draw_scale/draw_wifi 처럼 이 파일에 이미 있는 벡터 아이콘들과
// 같은 요령). Computer 는 원래 이 방식으로 직접 그렸었는데, 사용자가 준 Windows 98
// 아이콘 팩 안에 이미 "컴퓨터 + 탐색기 창" 느낌의 정확히 맞는 아이콘
// (`computer_explorer`, 32x32)이 있어서 그걸 그대로 `assets/icon_computer.png` 로
// 가져와 다른 파일 아이콘들과 같은 텍스처 방식으로 바꿨다.
pub fn draw_icon(r: &mut Renderer, assets: &Assets, icon: &IconType, x: f32, y: f32, s: f32) {
    match icon {
        IconType::Tar => return draw_tar_icon(r, x, y, s),
        IconType::Installer => return draw_installer_icon(r, x, y, s),
        IconType::HexTool => return draw_hextool_icon(r, x, y, s),
        _ => {}
    }
    let tex = match icon {
        IconType::Folder => assets.icon_folder,
        IconType::Txt => assets.icon_txt,
        IconType::Mp4 => assets.icon_mp4,
        IconType::Lock => assets.icon_lock,
        IconType::Mail => assets.icon_mail,
        IconType::Computer => assets.icon_computer,
        IconType::Img => assets.icon_img,
        IconType::Envelope => assets.icon_envelope,
        IconType::RecycleEmpty => assets.icon_recycle_empty,
        IconType::RecycleFull => assets.icon_recycle_full,
        IconType::PhotosApp => assets.icon_photos,
        IconType::Tar | IconType::Installer | IconType::HexTool => unreachable!(),
    };
    r.sprite(tex, x, y, s, s, WHITE);
}

// draw_drag_ghost 가 그리는 아이콘 크기 — 호출부(explorer.rs)가 "고스트 아이콘의
// 정중앙을 커서에 맞추고 싶을 때" 오프셋을 이 값의 절반으로 계산할 수 있게 공개해둔다.
pub const DRAG_GHOST_ICON_S: f32 = 26.0;

// 파일을 드래그로 옮기는 중 커서를 따라다니는 반투명 "복사본" 미리보기 — draw_icon
// 자체엔 알파를 못 넣어서, 평소처럼 그린 뒤 그 위에 옅은 색을 덮어써서 대략 30%
// 정도로 흐리게 보이게 흉내낸다. pos 는 이미 "클릭했던 지점" 오프셋이 반영된
// 좌표라(호출부가 커서 위치에서 그 오프셋을 빼서 넘김) 잡았던 자리 그대로 커서를
// 따라온다 — 창 클립과 무관하게(desktop.rs 가 창들을 다 그린 뒤 맨 위에 그린다)
// 창 경계 밖까지도 자유롭게 나갈 수 있다.
pub fn draw_drag_ghost(r: &mut Renderer, assets: &Assets, icon: &IconType, label: &str, pos: (f32, f32)) {
    let s = DRAG_GHOST_ICON_S;
    let (x, y) = pos;
    draw_icon(r, assets, icon, x, y, s);
    r.rect(x, y, s, s, [0.95, 0.95, 0.97, 0.7]); // 아이콘을 옅게 덮어서 흐린 "복사본" 느낌
    let tw = r.text_width(label, 0.75).min(110.0);
    let label_y = y + s + 2.0;
    r.rect(x + s / 2.0 - tw / 2.0 - 3.0, label_y, tw + 6.0, 16.0, [0.95, 0.95, 0.97, 0.7]);
    r.text_clipped(x + s / 2.0 - tw / 2.0, label_y + 2.0, label, 0.75, [0.25, 0.25, 0.25, 0.7], tw);
}

// .tar 압축파일 아이콘 — 노란 폴더 몸체 위에 지퍼(세로 중앙선 + 지그재그 이빨)를
// 그려서 "압축돼 봉인된 폴더" 라는 걸 한눈에 알아보게 한다(고전 zip 아이콘 느낌).
fn draw_tar_icon(r: &mut Renderer, x: f32, y: f32, s: f32) {
    let body = [0.88, 0.72, 0.2, 1.0];
    let tab = [0.75, 0.6, 0.15, 1.0];
    let zip = [0.35, 0.28, 0.05, 1.0];
    let bx = x + s * 0.1;
    let by = y + s * 0.28;
    let bw = s * 0.8;
    let bh = s * 0.56;
    // 폴더 탭(위쪽으로 살짝 튀어나온 부분)과 몸체.
    r.rect(bx + s * 0.06, by - s * 0.1, s * 0.32, s * 0.1, tab);
    r.rect(bx, by, bw, bh, body);
    border(r, bx, by, bw, bh, BLACK);
    // 지퍼: 세로 중앙선 + 좌우로 번갈아 튀어나온 이빨.
    let cx = x + s * 0.5;
    r.rect(cx - s * 0.015, by, s * 0.03, bh, zip);
    const TEETH: i32 = 5;
    for i in 0..TEETH {
        let t = (i as f32 + 0.5) / TEETH as f32;
        let ty = by + t * bh;
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        r.rect(cx + side * s * 0.03 - s * 0.025, ty - s * 0.025, s * 0.05, s * 0.05, zip);
    }
}

// 설치 프로그램(.exe) 아이콘 — 모니터 모양 안에 아래로 향하는 화살표를 그려서
// "설치/다운로드" 느낌을 낸다. installer.rs 가 설치 마법사 왼쪽 삽화 칸에 크게
// 그릴 때도 재사용하므로 pub(crate) 로 열어둔다.
pub(crate) fn draw_installer_icon(r: &mut Renderer, x: f32, y: f32, s: f32) {
    let screen = [0.85, 0.85, 0.9, 1.0];
    let frame = [0.3, 0.3, 0.35, 1.0];
    let bx = x + s * 0.08;
    let by = y + s * 0.1;
    let bw = s * 0.84;
    let bh = s * 0.58;
    r.rect(bx, by, bw, bh, frame);
    r.rect(bx + s * 0.05, by + s * 0.05, bw - s * 0.1, bh - s * 0.1, screen);
    r.rect(x + s * 0.38, y + s * 0.72, s * 0.24, s * 0.08, frame); // 받침대
    // 화면 안의 아래 화살표(세로 막대 + 삼각형).
    let cx = x + s * 0.5;
    r.rect(cx - s * 0.04, by + s * 0.1, s * 0.08, s * 0.2, NAVY);
    for i in 0..4 {
        let t = i as f32;
        let half = (4.0 - t) * s * 0.03;
        r.rect(cx - half, by + s * 0.3 + t * s * 0.03, half * 2.0, s * 0.03, NAVY);
    }
}

// 설치된 HexTool 프로그램 아이콘 — installer.rs 와 같은 모니터 몸체를 초록빛으로
// 바꾸고, 화살표 대신 톱니바퀴(도구/유틸리티 느낌)를 넣어서 "설치 중"인 Installer
// 아이콘과 구분되게 한다.
fn draw_hextool_icon(r: &mut Renderer, x: f32, y: f32, s: f32) {
    let screen = [0.85, 0.92, 0.85, 1.0];
    let frame = [0.25, 0.32, 0.25, 1.0];
    let bx = x + s * 0.08;
    let by = y + s * 0.1;
    let bw = s * 0.84;
    let bh = s * 0.58;
    r.rect(bx, by, bw, bh, frame);
    r.rect(bx + s * 0.05, by + s * 0.05, bw - s * 0.1, bh - s * 0.1, screen);
    r.rect(x + s * 0.38, y + s * 0.72, s * 0.24, s * 0.08, frame); // 받침대

    let cx = x + s * 0.5;
    let cy = by + s * 0.29;
    let r_out = s * 0.14;
    fill_circle(r, cx, cy, r_out * 0.55, NAVY);
    for i in 0..8 {
        let ang = i as f32 / 8.0 * std::f32::consts::TAU;
        let tx = cx + ang.cos() * r_out;
        let ty = cy + ang.sin() * r_out;
        r.rect(tx - 1.5, ty - 1.5, 3.0, 3.0, NAVY);
    }
}

// 저울(balance scale) — 작업표시줄 시작 버튼용. s 크기, color 색.
pub fn draw_scale(r: &mut Renderer, x: f32, y: f32, s: f32, color: Color) {
    let cx = x + s * 0.5;
    r.rect(x + s * 0.28, y + s * 0.84, s * 0.44, s * 0.12, color); // 받침대
    r.rect(cx - s * 0.05, y + s * 0.18, s * 0.1, s * 0.68, color); // 기둥
    r.rect(cx - s * 0.07, y + s * 0.08, s * 0.14, s * 0.1, color); // 손잡이
    r.rect(x + s * 0.08, y + s * 0.2, s * 0.84, s * 0.08, color); // 빔
    r.rect(x + s * 0.15, y + s * 0.28, s * 0.02, s * 0.18, color); // 좌측 줄
    r.rect(x + s * 0.04, y + s * 0.45, s * 0.24, s * 0.06, color); // 좌측 접시
    r.rect(x + s * 0.83, y + s * 0.28, s * 0.02, s * 0.18, color); // 우측 줄
    r.rect(x + s * 0.72, y + s * 0.45, s * 0.24, s * 0.06, color); // 우측 접시
}

// 와이파이 아이콘 — 점 + 그 위로 갈수록 커지는 곡선 호 3개. 각도(0..π) 방향으로
// 원을 직접 샘플링해서 그리므로(사각형 브래킷 근사가 아니라) 진짜 둥근 곡선으로 보인다.
// x,y 는 아이콘의 좌상단, s 는 한 변 크기. connected 가 false 면(실제 인터넷 연결
// 없음) 아이콘 전체(점+호) 위에 X 를 겹쳐 그리고, blink_visible 로 아이콘 전체를
// 깜빡이게 한다(false 인 프레임엔 아예 아무것도 안 그림 — 호출부가 시간 기반으로
// 주기적으로 껐다 켰다 넘겨준다).
pub fn draw_wifi(r: &mut Renderer, x: f32, y: f32, s: f32, color: Color, connected: bool, blink_visible: bool) {
    if !connected && !blink_visible {
        return;
    }
    let cx = x + s * 0.5;
    let cy = y + s * 0.86;

    fill_circle(r, cx, cy, s * 0.09, color); // 점(가장 안쪽 신호)

    // (안쪽 반지름, 바깥 반지름) — 점에서 멀어질수록 큰 호.
    const RINGS: [(f32, f32); 3] = [(0.20, 0.30), (0.42, 0.52), (0.64, 0.74)];
    for (r_in, r_out) in RINGS {
        fill_upper_ring(r, cx, cy, s * r_in, s * r_out, color);
    }

    if !connected {
        fill_x(r, cx, y + s * 0.52, s * 0.85, color); // 끊김 표시 — 아이콘 전체를 덮도록 크게
    }
}

// cx,cy 를 중심으로 한 변이 size 인 X 자를 대각선 위에 작은 정사각형들을 이어붙여
// 그린다(회전 사각형이 없어서 — 원 라스터화와 같은 요령).
fn fill_x(r: &mut Renderer, cx: f32, cy: f32, size: f32, color: Color) {
    let half = size * 0.5;
    let thick = (size * 0.15).max(1.0);
    let mut t = -half;
    while t <= half {
        r.rect(cx + t - thick * 0.5, cy + t - thick * 0.5, thick, thick, color); // "\" 대각선
        r.rect(cx + t - thick * 0.5, cy - t - thick * 0.5, thick, thick, color); // "/" 대각선
        t += thick * 0.6; // 살짝 겹치게 이어 붙여서 대각선이 끊겨 보이지 않게
    }
}

// 로딩 스피너 — 원 둘레에 점 N개를 놓고, 시간에 따라 밝기가 도는(chasing) 효과로
// 회전하는 것처럼 보이게 한다(실제 회전 없이 색만 바꿔서 매 프레임 다시 그림).
// cx,cy 는 중심, radius 는 점들이 놓이는 원의 반지름.
pub fn draw_spinner(r: &mut Renderer, cx: f32, cy: f32, radius: f32, time: f32) {
    const N: usize = 8;
    const DOT_R: f32 = 3.0;
    for i in 0..N {
        let angle = i as f32 / N as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let dx = angle.cos() * radius;
        let dy = angle.sin() * radius;
        // 이 점이 "지금 막 켜진" 위치로부터 얼마나 지났는지(0=방금 켜짐, 1=한 바퀴 전) —
        // 값이 작을수록 밝게 해서 꼬리를 끌며 도는 것처럼 보이게 한다.
        let phase = (i as f32 / N as f32 - time * 1.2).rem_euclid(1.0);
        let brightness = (1.0 - phase).max(0.15);
        let c = [brightness, brightness, brightness, 1.0];
        fill_circle(r, cx + dx, cy + dy, DOT_R, c);
    }
}

// 꽉 찬 원을 가로줄(1px 높이)들로 라스터화해서 그린다.
fn fill_circle(r: &mut Renderer, cx: f32, cy: f32, radius: f32, color: Color) {
    let mut dy = -radius;
    while dy <= radius {
        let hw = (radius * radius - dy * dy).max(0.0).sqrt();
        if hw > 0.0 {
            r.rect(cx - hw, cy + dy, hw * 2.0, 1.0, color);
        }
        dy += 1.0;
    }
}

// 원의 위쪽 고리(안쪽 반지름~바깥 반지름 사이 띠) 일부만 라스터화해서 그린다 —
// 와이파이 신호 호 모양. dy 를 0(원 중심 높이)까지 다 그리면 양 끝이 옆으로
// 수평까지 내려가 버려서, 중심 위 절반 정도(대략 60도)에서 멈춰 짧은 호로 보이게 한다.
fn fill_upper_ring(r: &mut Renderer, cx: f32, cy: f32, r_in: f32, r_out: f32, color: Color) {
    let y_limit = -r_out * 0.5;
    let mut dy = -r_out;
    while dy <= y_limit {
        let outer_hw = (r_out * r_out - dy * dy).max(0.0).sqrt();
        let inner_hw = if dy.abs() < r_in { (r_in * r_in - dy * dy).max(0.0).sqrt() } else { 0.0 };
        if outer_hw > inner_hw {
            r.rect(cx - outer_hw, cy + dy, outer_hw - inner_hw, 1.0, color);
            r.rect(cx + inner_hw, cy + dy, outer_hw - inner_hw, 1.0, color);
        }
        dy += 1.0;
    }
}

// 1px 테두리 사각형.
pub fn border(r: &mut Renderer, x: f32, y: f32, w: f32, h: f32, color: Color) {
    r.rect(x, y, w, 1.0, color);
    r.rect(x, y + h - 1.0, w, 1.0, color);
    r.rect(x, y, 1.0, h, color);
    r.rect(x + w - 1.0, y, 1.0, h, color);
}
