//! DOOM 의 적처럼 "항상 카메라를 바라보는 2D 그림"(빌보드 스프라이트) — 3D 장면 위에 그려진다.
//! 그림은 글자 격자(픽셀 도트)로 정의해서 따로 이미지 파일이 필요 없다. 화면 크기는 거리에
//! 반비례해서 가까우면 크게, 멀면 작게 보이고, 벽 뒤로 가면 가려진다(Box3D 와의 레이 판정).

use crate::render::gfx::Renderer;
use crate::render::mesh3d::{v_dot, v_len, v_scale, v_sub, Box3D, Camera};

use super::{View, FOV_Y, VIEW_H};

pub struct Sprite {
    rows: &'static [&'static str],
}

// '.' 은 투명, 나머지는 팔레트 색.
fn palette(c: char) -> Option<[f32; 4]> {
    let rgb = match c {
        'k' => [0.14, 0.09, 0.05],
        'y' => [0.88, 0.72, 0.36],
        'd' => [0.36, 0.23, 0.12],
        'D' => [0.27, 0.17, 0.09],
        'g' => [0.22, 0.62, 0.27],
        'b' => [0.36, 0.46, 0.96],
        'w' => [0.97, 0.96, 0.72],
        _ => return None,
    };
    Some([rgb[0], rgb[1], rgb[2], 1.0])
}

// 흙 속에 심은 씨앗.
pub const SEED: Sprite = Sprite {
    rows: &[
        "............",
        "............",
        "....kkk.....",
        "...kyyyk....",
        "....kkk.....",
        "..dddddddd..",
        ".dDdDdDdDdd.",
        "dDdDdDdDdDdd",
        "dddddddddddd",
    ],
};

// 핀 보리지꽃.
pub const BLOOM: Sprite = Sprite {
    rows: &[
        "....bbbb....",
        "..bbbbbbbb..",
        ".bbbbwwbbbb.",
        ".bbbwwwwbbb.",
        "..bbbwwbbb..",
        "...bbbbbb...",
        ".....gg.....",
        ".....gg.....",
        "..g..gg..g..",
        "..gg.gg.gg..",
        "...ggggggg..",
        ".....gg.....",
        ".....gg.....",
    ],
};

// 화면에 보이는 스프라이트 하나의 자리 — 아래 가운데가 pos(월드), 세로 길이가 world_h(미터).
// 카메라 뒤이거나 벽에 가려지면 그리지 않는다.
impl View {
    pub fn draw_billboard(&self, r: &mut Renderer, cam: &Camera, pos: [f32; 3], world_h: f32, sprite: &Sprite, occluders: &[Box3D]) {
        let to = v_sub(pos, cam.pos);
        let dist = v_len(to);
        let depth = v_dot(to, cam.forward());
        if depth <= 0.1 || dist <= 0.0 {
            return;
        }
        let dir = v_scale(to, 1.0 / dist);
        if occluders.iter().any(|b| b.ray_intersect(cam.pos, dir).is_some_and(|t| t < dist - 0.05)) {
            return;
        }
        let focal = VIEW_H / (2.0 * (FOV_Y / 2.0).tan());
        let n_rows = sprite.rows.len() as f32;
        let pixel = (world_h / n_rows) * focal / depth; // VIEW 기준 한 칸의 크기
        let n_cols = sprite.rows[0].chars().count() as f32;
        // 아래 가운데 → 화면 좌표(오른쪽/위 성분으로 투영).
        let half_h = (FOV_Y / 2.0).tan();
        let half_w = half_h * (super::VIEW_W / VIEW_H);
        let ndc_x = v_dot(to, cam.right()) / (depth * half_w);
        let ndc_y = v_dot(to, cam.up()) / (depth * half_h);
        let sx = (ndc_x * 0.5 + 0.5) * super::VIEW_W;
        let sy = (1.0 - (ndc_y * 0.5 + 0.5)) * VIEW_H;
        let left = sx - n_cols * pixel / 2.0;
        let top = sy - n_rows * pixel;
        let prev_clip = r.clip();
        r.set_clip(Some(self.rect));
        for (ry, row) in sprite.rows.iter().enumerate() {
            for (cx, c) in row.chars().enumerate() {
                if let Some(color) = palette(c) {
                    // 인접한 칸 사이에 틈이 안 생기게 한 칸을 살짝 크게 그린다.
                    r.rect(self.px(left + cx as f32 * pixel), self.py(top + ry as f32 * pixel), (pixel + 0.6) * self.s, (pixel + 0.6) * self.s, color);
                }
            }
        }
        r.set_clip(prev_clip);
    }
}
