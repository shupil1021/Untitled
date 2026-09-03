//! 2D 배칭 렌더러(픽셀 좌표) + 폰트(Terrarum Sans Bitmap 을 시작할 때
//! 래스터라이즈해서 만든 아틀라스, 한/영 겸용) + 에셋 로딩.

use std::collections::HashMap;

use miniquad::*;

pub type Color = [f32; 4];

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && self.x + self.w > o.x && self.y < o.y + o.h && self.y + self.h > o.y
    }
    // 두 사각형이 겹치는 부분만 남긴다. (중첩된 스크롤/클립 영역을 합칠 때 사용 —
    // 안쪽 클립이 바깥 클립 경계를 무시하고 그냥 덮어써버리면 스크롤된 내용이 바깥
    // 뷰포트 밖으로 삐져나가 보이는 문제가 생긴다.)
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = (self.x + self.w).min(o.x + o.w);
        let y1 = (self.y + self.h).min(o.y + o.h);
        Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
    }
}

pub const CELL_H: f32 = 22.0;
// 예전엔 미리 구워둔 ASCII 전용 고정폭 비트맵 폰트(font.png, 글자당 11px)를 썼는데,
// 한글(Terrarum Sans Bitmap)을 지원하려고 실제 폰트 파일을 시작할 때 래스터라이즈
// 하는 방식으로 바꿨다. 이 폰트는 라틴 문자가 진짜 고정폭이라, 예전처럼
// text_width() 를 안 부르고 글자수 × ADVANCE 로 크기를 어림잡던 몇몇 자리
// (로고/메뉴 폭, 줄바꿈 문자수 계산 등)도 그대로 정확하게 맞는다.
pub const ADVANCE: f32 = 11.0;
// 폰트를 래스터라이즈할 픽셀 크기 — CELL_H 와 맞춰서, scale=1.0 일 때 예전 비트맵
// 폰트와 세로 크기가 비슷하게 보이도록 했다. 폰트 자체를 가공(작게 그려 확대하거나
// 알파를 이진화하는 등)하지 않고 fontdue 가 그려주는 그대로 쓴다.
const FONT_PX: f32 = 22.0;

struct GlyphInfo {
    u0: f32, v0: f32, u1: f32, v1: f32, // 아틀라스 안의 UV 사각형
    w: f32, h: f32,                     // 래스터라이즈된 비트맵 크기(FONT_PX 기준 픽셀)
    xmin: f32, ymin: f32,               // 베이스라인 기준 오프셋(fontdue Metrics 그대로, FONT_PX 기준)
    advance: f32,                       // 다음 글자 펜 위치까지의 이동폭(FONT_PX 기준)
}

const MAX_VERTS: usize = 16384;
const MAX_INDICES: usize = 24576;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: Color,
}

impl Vertex {
    const ZERO: Vertex = Vertex { pos: [0.0, 0.0], uv: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.0] };
}

#[repr(C)]
struct ScreenUniform {
    screen: [f32; 2],
}

struct Batch {
    tex: TextureId,
    start: usize,
    count: usize,
}

pub struct Renderer {
    pipeline: Pipeline,
    bindings: Bindings,
    white: TextureId,
    font: TextureId,
    font_glyphs: HashMap<char, GlyphInfo>,
    font_ascent: f32, // 베이스라인 위로 몇 픽셀(FONT_PX 기준)인지 — 글자를 세로로 위치시키는 기준
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    batches: Vec<Batch>,
    screen: (f32, f32),
    clip: Option<Rect>,
}

pub struct Assets {
    pub cursor: TextureId, // 커서 스프라이트 시트 (200x113, 아이콘별 하위 사각형으로 잘라 씀)
    // 바탕화면/탐색기 파일 아이콘 — Windows 98 아이콘 팩(사용자 제공)에서 32x32 로 뽑아온 것들.
    pub icon_mail: TextureId,
    pub icon_folder: TextureId,
    pub icon_txt: TextureId,
    pub icon_mp4: TextureId,
    pub icon_lock: TextureId,
    pub icon_computer: TextureId,
    pub icon_img: TextureId,
    pub icon_envelope: TextureId, // Mail 앱 폴더 트리의 Inbox 아이콘
    pub icon_recycle_empty: TextureId, // 휴지통(비어있음)
    pub icon_recycle_full: TextureId,  // 휴지통(안에 파일이 있음)
    pub icon_photos: TextureId, // Photos 앱(바탕화면 이름은 일부러 깨진 글자) 아이콘 — 사용자 제공
    // "팔라스 OS가 생성한 이미지" 로 바탕화면에 놓이는 실제 사진들 — FileKind::Img(idx) 의
    // idx 가 이 Vec 의 인덱스다. (텍스처, 원본 픽셀 폭, 원본 픽셀 높이) — 종횡비를 살려서
    // 레터박스로 그리려면 원본 크기가 필요해서 아이콘 텍스처들과 달리 크기도 같이 들고 있는다.
    // 사진을 늘릴수록 여기 include_bytes! 한 줄만 추가하면 된다.
    pub photos: Vec<(TextureId, u32, u32)>,
}

impl Assets {
    pub fn load(ctx: &mut dyn RenderingBackend) -> Assets {
        Assets {
            cursor: load_texture(ctx, include_bytes!("../assets/cursor.png")),
            icon_mail: load_texture(ctx, include_bytes!("../assets/icon_mail.png")),
            icon_folder: load_texture(ctx, include_bytes!("../assets/icon_folder.png")),
            icon_txt: load_texture(ctx, include_bytes!("../assets/icon_txt.png")),
            icon_mp4: load_texture(ctx, include_bytes!("../assets/icon_mp4.png")),
            icon_lock: load_texture(ctx, include_bytes!("../assets/icon_lock.png")),
            icon_computer: load_texture(ctx, include_bytes!("../assets/icon_computer.png")),
            icon_img: load_texture(ctx, include_bytes!("../assets/icon_img.png")),
            icon_envelope: load_texture(ctx, include_bytes!("../assets/icon_envelope.png")),
            icon_recycle_empty: load_texture(ctx, include_bytes!("../assets/icon_recycle_empty.png")),
            icon_recycle_full: load_texture(ctx, include_bytes!("../assets/icon_recycle_full.png")),
            icon_photos: load_texture(ctx, include_bytes!("../assets/icon_photos.png")),
            // Photos.tar/photo01·02.jpg 플레이스홀더 스토리 콘텐츠를 걷어내면서
            // 비웠다 — FileKind::Img(usize)/ImageViewerApp 자체는 나중에 진짜
            // Chapter 1 사진이 생기면 그대로 다시 쓸 수 있게 남겨뒀다.
            photos: vec![],
        }
    }
}

fn load_texture(ctx: &mut dyn RenderingBackend, png: &[u8]) -> TextureId {
    let img = image::load_from_memory(png).expect("PNG 디코드 실패").to_rgba8();
    let (w, h) = img.dimensions();
    let tex = ctx.new_texture_from_rgba8(w as u16, h as u16, &img);
    ctx.texture_set_filter(tex, FilterMode::Nearest, MipmapFilterMode::None);
    tex
}

fn build_font_atlas(ctx: &mut dyn RenderingBackend) -> (TextureId, HashMap<char, GlyphInfo>, f32) {
    let ttf = include_bytes!("../assets/TerrarumSansBitmap.otf") as &[u8];
    let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()).expect("Terrarum Sans Bitmap 파싱 실패");

    // ASCII 32(스페이스)~126(~) + 완성형 한글 전체(U+AC00~U+D7A3, 11,172 자) +
    // 낱자(자모) 한글 두 블록 + 히라가나/가타카나 전체(각각 96자라 통째로 넣어도
    // 저렴함) + 실제로 쓰이는 한자만 골라 담은 KANJI_CHARSET(로컬라이제이션
    // 문자열이 늘어날 때마다 여기에 새로 쓰는 한자를 추가해야 함 — strings.rs 의
    // 각 S{ en, ko, ja } 상수 중 ja 필드들을 훑어서 뽑는다).
    //
    // 자음/모음 하나만 입력하고 다음 글자로 안 넘어가면(예: "ㄱ" 만 치고 끝) IME 가
    // 완성형 음절이 아니라 "호환용 자모"(U+3131~U+318E, 예: "ㄱ" "ㅏ")로 확정해서
    // 보내는데, 이 블록은 완성형 한글(U+AC00~) 범위 밖이라 지금까지 폰트 아틀라스에
    // 아예 없었다 — 그 글자만 빈칸(공백)처럼 보이던 버그(제보받음)의 원인. "조합 중
    // 내부용" 자모 블록(U+1100~U+11FF)도 혹시 몰라 같이 넣었다 — 둘 다 실제로
    // 폰트에 글자가 있는지 fontTools 로 미리 확인했다(94/94, 256/256 다 있음).
    const KANJI_CHARSET: &str =
        "一上下不中了人仕他付以件任作使依保信個像元先入全公内再写凍出切初利削前力効動去収取受同名場変外契存完定宛容少属差帰常度式強当形彩影待復必意感態憶成戻所折択抽持接数整新日旧明時景更最有未本枚果権機次正残況注消添済然特状理用画異発登目真着知短確示空箱約終続置者背能自色荷行表複要見規観解言記設許認語読諾護負責起込返送逃進違選重量録長閉開間除音響項頼\
         事会似体例修傷処判別割加務勤危可合含告員問困囲在報安実害審対念提損撃攻料断映期査検業様歓殿活無物現生的皆直研社祈移究等範級結維覧討証該説調識象貴資迎近遂部閲険難題類\
         刻大小弊拡深縮致被覚際非";
    let ascii = (0x20u32..0x7F).filter_map(char::from_u32);
    let hangul = (0xAC00u32..=0xD7A3).filter_map(char::from_u32);
    let hangul_jamo = (0x1100u32..=0x11FFu32).filter_map(char::from_u32);
    let hangul_compat_jamo = (0x3131u32..=0x318Eu32).filter_map(char::from_u32);
    // CJK 기호/구두점(U+3000~303F) — 、(U+3001)와 。(U+3002) 처럼 일본어 문장에
    // 흔히 쓰이는 전각 쉼표/마침표가 여기 있는데 지금까지 아틀라스에 없었다.
    // 예전엔 그냥 빈 칸으로 사라져서 안 보였는데(Renderer::glyph_advance 의
    // "그냥 넘어가되" 주석 참고), Photos 아이콘의 "진짜 깨진 글자"(tofu 자리
    // 표시자)를 만들면서 그 fallback 이 이제 항상 마름모+물음표를 그리게
    // 바뀌는 바람에, 이 문장부호가 다 깨진 것처럼 보이는 회귀가 생겼다(제보
    // 받음: 휴지통 안내문의 쉼표/마침표 자리마다 ◆ 가 찍힘). 이 블록을 통째로
    // 넣어서 근본적으로 고친다.
    let cjk_punct = (0x3000u32..=0x303Fu32).filter_map(char::from_u32);
    let hiragana = (0x3040u32..=0x309F).filter_map(char::from_u32);
    let katakana = (0x30A0u32..=0x30FF).filter_map(char::from_u32);
    let kanji = KANJI_CHARSET.chars();
    let chars: Vec<char> = ascii
        .chain(hangul)
        .chain(hangul_jamo)
        .chain(hangul_compat_jamo)
        .chain(cjk_punct)
        .chain(hiragana)
        .chain(katakana)
        .chain(kanji)
        .collect();

    let rasters: Vec<(char, fontdue::Metrics, Vec<u8>)> =
        chars.into_iter().map(|ch| { let (m, bmp) = font.rasterize(ch, FONT_PX); (ch, m, bmp) }).collect();

    // 가장 큰 글자 크기에 맞춘 고정 칸 격자로 단순하게 패킹한다(진짜 폰트 아틀라스
    // 패커처럼 자투리 공간을 아끼진 않지만, 훨씬 단순하고 버그 날 일이 적다).
    let cell_w = rasters.iter().map(|(_, m, _)| m.width as u32).max().unwrap_or(1).max(1) + 1;
    let cell_h = rasters.iter().map(|(_, m, _)| m.height as u32).max().unwrap_or(1).max(1) + 1;
    let cols = 128u32;
    let rows = (rasters.len() as u32).div_ceil(cols).max(1);
    let atlas_w = cols * cell_w;
    let atlas_h = rows * cell_h;

    let mut pixels = vec![0u8; (atlas_w * atlas_h * 4) as usize];
    let mut glyphs = HashMap::with_capacity(rasters.len());
    for (i, (ch, m, bmp)) in rasters.iter().enumerate() {
        let ox = (i as u32 % cols) * cell_w;
        let oy = (i as u32 / cols) * cell_h;
        for yy in 0..m.height {
            for xx in 0..m.width {
                let a = bmp[yy * m.width + xx];
                let px = ox as usize + xx;
                let py = oy as usize + yy;
                let idx = (py * atlas_w as usize + px) * 4;
                // 흰색 RGB + 커버리지를 알파로 — 렌더러가 색은 매 draw 마다 곱해서
                // 입히므로(push_quad 의 color), 아틀라스 자체는 무채색이면 된다.
                pixels[idx] = 255;
                pixels[idx + 1] = 255;
                pixels[idx + 2] = 255;
                pixels[idx + 3] = a;
            }
        }
        glyphs.insert(
            *ch,
            GlyphInfo {
                u0: ox as f32 / atlas_w as f32,
                v0: oy as f32 / atlas_h as f32,
                u1: (ox + m.width as u32) as f32 / atlas_w as f32,
                v1: (oy + m.height as u32) as f32 / atlas_h as f32,
                w: m.width as f32,
                h: m.height as f32,
                xmin: m.xmin as f32,
                ymin: m.ymin as f32,
                advance: m.advance_width,
            },
        );
    }

    let tex = ctx.new_texture_from_rgba8(atlas_w as u16, atlas_h as u16, &pixels);
    // Linear 로 그려봤더니 CRT 셰이더(색수차 등)와 겹쳐서 너무 흐릿해 픽셀 느낌이
    // 안 산다는 피드백을 받았다 — 글자 모양 자체(fontdue 가 그린 안티에일리어싱
    // 커버리지)는 그대로 두되, 다른 텍스처(아이콘 등)와 같은 Nearest 로 그려서
    // 텍셀 사이를 안 뭉개지게 했다. 이건 이미 그려진 텍스처를 "어떻게 표시하는지"
    // 만 바꾸는 것이라 글자 모양 자체를 가공하는 것과는 다르다.
    ctx.texture_set_filter(tex, FilterMode::Nearest, MipmapFilterMode::None);

    let ascent = font.horizontal_line_metrics(FONT_PX).map(|lm| lm.ascent).unwrap_or(FONT_PX * 0.8);
    (tex, glyphs, ascent)
}

impl Renderer {
    pub fn new(ctx: &mut dyn RenderingBackend) -> Renderer {
        let white = ctx.new_texture_from_rgba8(1, 1, &[255, 255, 255, 255]);
        ctx.texture_set_filter(white, FilterMode::Nearest, MipmapFilterMode::None);
        let (font, font_glyphs, font_ascent) = build_font_atlas(ctx);

        let vbuf = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Stream,
            BufferSource::slice(&[Vertex::ZERO; MAX_VERTS]),
        );
        let ibuf = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Stream,
            BufferSource::slice(&[0u16; MAX_INDICES]),
        );

        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: SHADER2D_VS, fragment: SHADER2D_FS },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("screen", UniformType::Float2)],
                    },
                },
            )
            .expect("2D 셰이더 컴파일 실패");

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
                VertexAttribute::new("in_color", VertexFormat::Float4),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );

        let bindings = Bindings {
            vertex_buffers: vec![vbuf],
            index_buffer: ibuf,
            images: vec![white],
        };

        Renderer {
            pipeline,
            bindings,
            white,
            font,
            font_glyphs,
            font_ascent,
            vertices: Vec::with_capacity(MAX_VERTS),
            indices: Vec::with_capacity(MAX_INDICES),
            batches: Vec::new(),
            screen: (0.0, 0.0),
            clip: None,
        }
    }

    pub fn begin(&mut self, vw: f32, vh: f32) {
        self.vertices.clear();
        self.indices.clear();
        self.batches.clear();
        self.screen = (vw, vh);
        self.clip = None;
    }

    // 이후 그려지는 것들을 rect 영역 밖으로 못 나가게 자른다. None 이면 클리핑 해제.
    // (창 안 앱 내용을 창 밖으로 삐져나오지 않게 하는 용도)
    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }

    pub fn clip(&self) -> Option<Rect> {
        self.clip
    }

    fn push_quad(
        &mut self,
        tex: TextureId,
        x: f32, y: f32, w: f32, h: f32,
        u0: f32, v0: f32, u1: f32, v1: f32,
        color: Color,
    ) {
        let (mut x, mut y, mut w, mut h) = (x, y, w, h);
        let (mut u0, mut v0, mut u1, mut v1) = (u0, v0, u1, v1);
        if let Some(c) = self.clip {
            let x0 = x.max(c.x);
            let y0 = y.max(c.y);
            let x1 = (x + w).min(c.x + c.w);
            let y1 = (y + h).min(c.y + c.h);
            if x1 <= x0 || y1 <= y0 {
                return;
            }
            if w > 0.0 {
                let lu = (x0 - x) / w;
                let ru = (x1 - x) / w;
                let (ou0, ou1) = (u0, u1);
                u0 = ou0 + (ou1 - ou0) * lu;
                u1 = ou0 + (ou1 - ou0) * ru;
            }
            if h > 0.0 {
                let lv = (y0 - y) / h;
                let rv = (y1 - y) / h;
                let (ov0, ov1) = (v0, v1);
                v0 = ov0 + (ov1 - ov0) * lv;
                v1 = ov0 + (ov1 - ov0) * rv;
            }
            x = x0;
            y = y0;
            w = x1 - x0;
            h = y1 - y0;
        }
        if self.vertices.len() + 4 > MAX_VERTS {
            return;
        }
        let base = self.vertices.len() as u16;
        self.vertices.push(Vertex { pos: [x, y], uv: [u0, v0], color });
        self.vertices.push(Vertex { pos: [x + w, y], uv: [u1, v0], color });
        self.vertices.push(Vertex { pos: [x + w, y + h], uv: [u1, v1], color });
        self.vertices.push(Vertex { pos: [x, y + h], uv: [u0, v1], color });

        let need_new = match self.batches.last() {
            Some(b) => b.tex != tex,
            None => true,
        };
        if need_new {
            self.batches.push(Batch { tex, start: self.indices.len(), count: 0 });
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.batches.last_mut().unwrap().count += 6;
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let white = self.white;
        self.push_quad(white, x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
    }

    pub fn sprite(&mut self, tex: TextureId, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.push_quad(tex, x, y, w, h, 0.0, 0.0, 1.0, 1.0, color);
    }

    // 스프라이트 시트에서 (u0,v0)-(u1,v1) 부분만 잘라서 그린다.
    #[allow(clippy::too_many_arguments)]
    pub fn sprite_uv(&mut self, tex: TextureId, x: f32, y: f32, w: f32, h: f32, u0: f32, v0: f32, u1: f32, v1: f32, color: Color) {
        self.push_quad(tex, x, y, w, h, u0, v0, u1, v1, color);
    }

    // 글자 하나의 다음 펜 위치까지의 이동폭(FONT_PX 기준 → scale 곱해서 실제 픽셀로).
    // 아틀라스에 없는 글자(한글 자모 낱자, 옛한글 등 완성형 밖의 극히 일부)는
    // draw_tofu() 가 그리는 자리표시자 폭만큼 움직여서 겹쳐 보이지 않게 한다.
    fn glyph_advance(&self, ch: char) -> f32 {
        self.font_glyphs.get(&ch).map(|g| g.advance).unwrap_or(FONT_PX * 0.75)
    }

    pub fn text_width(&self, text: &str, scale: f32) -> f32 {
        text.chars().map(|c| self.glyph_advance(c)).sum::<f32>() * scale
    }

    fn glyph(&mut self, x: f32, y: f32, ch: char, scale: f32, color: Color) {
        let Some(g) = self.font_glyphs.get(&ch) else {
            self.draw_tofu(x, y, scale);
            return;
        };
        if g.w <= 0.0 || g.h <= 0.0 {
            return; // 스페이스 등 — 그릴 비트맵이 없다(펜 이동은 호출부가 advance 로 처리).
        }
        let (u0, v0, u1, v1, w, h, xmin, ymin) = (g.u0, g.v0, g.u1, g.v1, g.w, g.h, g.xmin, g.ymin);
        let font = self.font;
        let ascent = self.font_ascent;
        // 베이스라인 = 이 줄의 top(y) + ascent — fontdue 의 xmin/ymin 은 베이스라인
        // 기준 오프셋(ymin 이 음수면 베이스라인보다 아래로 내려간다는 뜻)이라, 글자
        // 비트맵의 실제 좌상단은 "베이스라인 - (ymin+h)" 로 구한다.
        let baseline = y + ascent * scale;
        let gx = x + xmin * scale;
        let gy = baseline - (ymin + h) * scale;
        self.push_quad(font, gx, gy, w * scale, h * scale, u0, v0, u1, v1, color);
    }

    // 아틀라스에 없는 글자 대신 그리는 "깨진 글자" 자리표시자 — 지원 안 되는
    // 코드페이지의 문자를 만났을 때 Windows 가 흔히 보여주는 마름모(다이아몬드)
    // + 물음표 모양(U+FFFD 렌더링과 같은 그 모양)을 흉내낸다. Photos 앱의
    // 일부러 깨뜨린 이름에 쓰려고 추가했다 — 진짜 인코딩이 깨진 것처럼
    // 보이려면 자모를 흩어놓는 것만으론 부족하고, 아예 렌더링이 안 되는
    // 문자를 섞어야 그 특유의 "마름모에 물음표" 모양이 나온다. 사각형 텍스처
    // 하나로는 마름모를 못 그리니(회전 사각형 지원이 없다), fill_circle 처럼
    // 가로줄(1px 높이)들을 라스터화해서 흉내낸다 — 줄마다 마름모 중심에서
    // 멀어질수록 폭이 좁아지게. 텍스트 색과 무관하게 항상 어두운 마름모+흰
    // 물음표로 그린다(실제 broken-glyph 표시가 늘 그렇듯 원래 색과 무관하게 눈에 띔).
    fn draw_tofu(&mut self, x: f32, y: f32, scale: f32) {
        let box_w = (FONT_PX * 0.62 * scale).max(4.0);
        let box_h = (FONT_PX * 0.8 * scale).max(4.0);
        let by = y + FONT_PX * 0.1 * scale;
        let (cx, cy) = (x + scale + box_w / 2.0, by + box_h / 2.0);
        let (hw, hh) = (box_w / 2.0, box_h / 2.0);
        let tofu = [0.05, 0.05, 0.05, 1.0];
        let mut dy = -hh;
        while dy <= hh {
            let half = hw * (1.0 - (dy.abs() / hh).min(1.0));
            if half > 0.0 {
                self.rect(cx - half, cy + dy, half * 2.0, 1.0, tofu);
            }
            dy += 1.0;
        }
        self.glyph(x + scale + box_w * 0.24, y, '?', scale * 0.72, [1.0, 1.0, 1.0, 1.0]);
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color) {
        let mut pen = x;
        for ch in text.chars() {
            self.glyph(pen, y, ch, scale, color);
            pen += self.glyph_advance(ch) * scale;
        }
    }

    // text() 와 달리 매 글자를 실제 폭 대신 고정폭(advance 인자)으로 강제 이동시킨다
    // — PalaceOS 피겨렛 로고(boot.rs::LOGO)처럼 여러 줄에 걸쳐 문자 위치가 그대로
    // 세로로 정렬돼야 하는 아스키 아트에 안전하게 쓴다.
    pub fn text_mono(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color, advance: f32) {
        let mut pen = x;
        for ch in text.chars() {
            self.glyph(pen, y, ch, scale, color);
            pen += advance * scale;
        }
    }

    // max_w 픽셀을 넘어가면 잘라서 그린다 (창 밖으로 넘치지 않게).
    pub fn text_clipped(&mut self, x: f32, y: f32, text: &str, scale: f32, color: Color, max_w: f32) {
        let mut pen = x;
        for ch in text.chars() {
            let adv = self.glyph_advance(ch) * scale;
            if pen + adv > x + max_w {
                break;
            }
            self.glyph(pen, y, ch, scale, color);
            pen += adv;
        }
    }

    pub fn flush(&mut self, ctx: &mut dyn RenderingBackend) {
        if self.indices.is_empty() {
            return;
        }
        ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(&self.vertices));
        ctx.buffer_update(self.bindings.index_buffer, BufferSource::slice(&self.indices));

        ctx.apply_pipeline(&self.pipeline);
        let u = ScreenUniform { screen: [self.screen.0, self.screen.1] };
        for b in &self.batches {
            self.bindings.images[0] = b.tex;
            ctx.apply_bindings(&self.bindings);
            ctx.apply_uniforms(UniformsSource::table(&u));
            ctx.draw(b.start as i32, b.count as i32, 1);
        }
    }
}

const SHADER2D_VS: &str = r#"#version 100
attribute vec2 in_pos;
attribute vec2 in_uv;
attribute vec4 in_color;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform vec2 screen;
void main() {
    vec2 p = in_pos / screen * 2.0 - 1.0;
    p.y = -p.y;
    gl_Position = vec4(p, 0.0, 1.0);
    uv = in_uv;
    color = in_color;
}
"#;

const SHADER2D_FS: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D tex;
void main() {
    gl_FragColor = texture2D(tex, uv) * color;
}
"#;
