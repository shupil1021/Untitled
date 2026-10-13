//! 코드로 만드는 이미지 — 게임 폴더의 그림(문/화분/잡음), 서브 게임이 보내는 꽃 사진, 미니게임 보상 사진(눈/뇌/손가락).

use crate::random::Rng;


// IMAGE_NAMES 순서대로 — Assets::load 가 이 순서로 photos 에 올리므로 FileKind::Img(i) 의
// i 가 그대로 이 목록의 인덱스다.
pub fn generated_images() -> [image::RgbaImage; 7] {
    [door_image(), flower_pot_image(), static_image(), flower_image(), eye_image(), brain_image(), finger_image()]
}

// 미니게임 보상 사진 셋(충혈된 눈 / 짓무른 뇌 / 절단된 손가락)이 생성 이미지 목록에서 시작하는 인덱스 —
// MiniKind::image_index() 가 여기에 MiniKind::index() 를 더한다.
pub const REWARD_IMAGE_BASE: usize = 4;

// 충혈된 눈 — 붉게 물든 흰자에 핏줄, 갈색 홍채와 까만 동공.
fn eye_image() -> image::RgbaImage {
    let (w, h) = (64u32, 48u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        let (dx, dy) = (x as f32 - 32.0, y as f32 - 24.0);
        let inside = (dx / 28.0).powi(2) + (dy / 17.0).powi(2) < 1.0;
        if !inside {
            return image::Rgba([12, 12, 14, 255]);
        }
        let r = (dx * dx + dy * dy).sqrt();
        if r < 4.5 {
            return image::Rgba([8, 0, 0, 255]);
        }
        if r < 11.0 {
            let ring = if r > 9.0 { [60, 26, 20] } else { [112, 56, 36] };
            return image::Rgba([ring[0], ring[1], ring[2], 255]);
        }
        // 흰자 — 가장자리로 갈수록 더 붉고, 안쪽에서 퍼지는 핏줄.
        let edge = ((dx / 28.0).powi(2) + (dy / 17.0).powi(2)).clamp(0.0, 1.0);
        let vein = ((dy.atan2(dx) * 9.0 + r * 0.45).sin()).abs() < 0.1 && r > 12.0;
        if vein {
            image::Rgba([186, 28, 30, 255])
        } else {
            let red = 232.0 - 40.0 * edge;
            image::Rgba([red as u8, (212.0 - 92.0 * edge) as u8, (206.0 - 100.0 * edge) as u8, 255])
        }
    })
}

// 짓무른 뇌 — 주름 잡힌 분홍 덩어리에 거뭇하게 곪은 얼룩.
fn brain_image() -> image::RgbaImage {
    let mut rng = Rng::new(0xB4A1);
    let spots: Vec<(f32, f32, f32)> = (0..7).map(|_| (14.0 + rng.unit() * 28.0, 14.0 + rng.unit() * 24.0, 2.5 + rng.unit() * 3.5)).collect();
    image::RgbaImage::from_fn(56, 56, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let (dx, dy) = (fx - 28.0, fy - 26.0);
        let blob = (dx / 23.0).powi(2) + (dy / 19.0).powi(2) < 1.0;
        let stem = (25..31).contains(&x) && (42..53).contains(&y);
        if !blob && !stem {
            return image::Rgba([12, 12, 14, 255]);
        }
        if stem && !blob {
            return image::Rgba([170, 120, 125, 255]);
        }
        if spots.iter().any(|&(sx, sy, sr)| ((fx - sx).powi(2) + (fy - sy).powi(2)).sqrt() < sr) {
            return image::Rgba([102, 104, 62, 255]); // 짓무른 곳
        }
        let fold = ((fx * 0.55 + (fy * 0.45).sin() * 2.4).sin()) > 0.82 || (dx.abs() < 1.2 && dy < 6.0);
        if fold {
            image::Rgba([150, 88, 104, 255])
        } else {
            let shade = 1.0 - (dx * dx / 700.0 + dy * dy / 520.0) * 0.25;
            image::Rgba([(214.0 * shade) as u8, (154.0 * shade) as u8, (166.0 * shade) as u8, 255])
        }
    })
}

// 절단된 손가락 — 손톱이 있는 살색 원통, 잘린 쪽은 붉은 단면과 하얀 뼈.
fn finger_image() -> image::RgbaImage {
    image::RgbaImage::from_fn(32, 64, |x, y| {
        let body = (11..21).contains(&x) && (6..55).contains(&y);
        let tip = (11..21).contains(&x) && (4..8).contains(&y) && !((11..13).contains(&x) && y < 6) && !((19..21).contains(&x) && y < 6);
        if !(body || tip) {
            return image::Rgba([12, 12, 14, 255]);
        }
        if y >= 49 {
            // 잘린 단면 — 가운데 뼈, 둘레는 붉은 살.
            return if (14..18).contains(&x) && (50..54).contains(&y) { image::Rgba([232, 226, 210, 255]) } else { image::Rgba([150, 22, 24, 255]) };
        }
        if (11..21).contains(&x) && (8..17).contains(&y) {
            return image::Rgba([246, 214, 200, 255]); // 손톱
        }
        // 가운데가 밝고 가장자리가 어두운 원통 음영 + 마디 주름.
        let t = (x as f32 - 15.5).abs() / 5.0;
        let knuckle = (31..33).contains(&y);
        let k = 1.0 - 0.28 * t - if knuckle { 0.12 } else { 0.0 };
        image::Rgba([(236.0 * k) as u8, (190.0 * k) as u8, (160.0 * k) as u8, 255])
    })
}


// 네 번째 이미지(FileKind::Img(FLOWER_IMAGE)) — 게임 폴더 파일이 아니라, 서브 게임에서 보낸
// 꽃이 메일 첨부(flower.png)로 돌아올 때 쓰는 보리지꽃 사진.
pub const FLOWER_IMAGE: usize = 3;
pub const FLOWER_IMAGE_NAME: &str = "flower.png";

// 어두운 배경에 줄기와 잎, 그 위에 파란 별 모양 꽃(보리지꽃)과 짙은 가운데.
fn flower_image() -> image::RgbaImage {
    let (w, h) = (48u32, 64u32);
    let (cx, cy) = (24.0f32, 20.0f32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        let (dx, dy) = (x as f32 - cx, y as f32 - cy);
        let r = (dx * dx + dy * dy).sqrt();
        let petal_reach = 5.0 + 8.0 * (2.5 * dy.atan2(dx)).cos().abs();
        let stem = (23..=25).contains(&x) && y >= 30;
        let leaf = (44..52).contains(&y) && (x as f32 - 24.0).abs() < (y as f32 - 44.0) * 1.3 && !(23..=25).contains(&x);
        if r < 3.5 {
            image::Rgba([46, 24, 78, 255])
        } else if r < petal_reach {
            image::Rgba([70, 100, 220, 255])
        } else if stem || leaf {
            image::Rgba([60, 130, 60, 255])
        } else {
            image::Rgba([12, 12, 14, 255])
        }
    })
}

fn door_image() -> image::RgbaImage {
    let (w, h) = (64u32, 96u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        let frame = x < 3 || x >= w - 3 || y < 3 || y >= h - 3;
        let panel = (10..w - 10).contains(&x) && ((10..40).contains(&y) || (50..86).contains(&y));
        let handle = (46..52).contains(&x) && (46..52).contains(&y);
        let c = if handle {
            [204, 184, 115]
        } else if frame {
            [60, 38, 24]
        } else if panel {
            [104, 70, 45]
        } else {
            [89, 61, 41]
        };
        image::Rgba([c[0], c[1], c[2], 255])
    })
}

fn flower_pot_image() -> image::RgbaImage {
    let (w, h) = (48u32, 48u32);
    image::RgbaImage::from_fn(w, h, |x, y| {
        // 위가 넓은 사다리꼴 화분 + 맨 위 흙. 꽃도 줄기도 없다.
        let (left, right) = (10 + (y.saturating_sub(18)) / 3, 38 - (y.saturating_sub(18)) / 3);
        let in_pot = (18..44).contains(&y) && (left..right).contains(&x);
        let soil = (16..20).contains(&y) && (11..37).contains(&x);
        if soil {
            image::Rgba([48, 34, 24, 255])
        } else if in_pot {
            image::Rgba([154, 89, 51, 255])
        } else {
            image::Rgba([12, 12, 14, 255])
        }
    })
}

fn static_image() -> image::RgbaImage {
    let mut rng = Rng::new(0x5EED);
    image::RgbaImage::from_fn(64, 64, |_, _| {
        let v = (rng.unit() * 255.0) as u8;
        image::Rgba([v, v, v, 255])
    })
}
