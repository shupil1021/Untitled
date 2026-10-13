// 개발용 도구 — 카트 레이스 장면을 여러 자리에서 그려 PNG 로 저장한다(눈으로 확인하는 스모크 테스트).
// 사용법: cargo run --example race_preview -- <PNG 를 저장할 폴더>

use crackhead::apps::minigame::drive::DriveRound;
use crackhead::random::Rng;
use crackhead::render::mesh3d::{Camera, Mesh3D};
use miniquad::*;

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    out: String,
    done: bool,
}

fn heading(a: [f32; 2], b: [f32; 2]) -> f32 {
    (-(b[0] - a[0])).atan2(-(b[1] - a[1]))
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if self.done {
            window::order_quit();
            return;
        }
        self.done = true;
        let round = DriveRound::new(Rng::new(5));
        let mut mesh = Mesh3D::new(self.ctx.as_mut(), 640, 480);
        let boxes = round.boxes();
        let meshes = round.meshes();
        // (이름, 길 번호, 점 번호, 뒤로 얼마나 물러서서 볼지)
        let shots: [(&str, usize, usize, f32); 10] = [
            ("start", 0, 160, 6.5),
            ("east_fork_before", 0, 18, 6.5),
            ("east_fork_at", 0, 30, 6.5),
            ("east_fork_inside_main", 0, 46, 6.5),
            ("east_inner_detour", 1, 26, 6.5),
            ("east_outer_detour", 2, 26, 6.5),
            ("east_merge", 0, 70, 6.5),
            ("west_fork_before", 0, 100, 6.5),
            ("west_detour", 3, 26, 6.5),
            ("corner", 0, 100, 6.5),
        ];
        for (name, route, idx, back) in shots {
            let pts = &round.track.routes[route].pts;
            let (a, b) = (pts[idx], pts[idx + 1]);
            let h = heading(a, b);
            let f = [-h.sin(), -h.cos()];
            let cam = Camera { pos: [a[0] - f[0] * back, 3.2, a[1] - f[1] * back], yaw: h, pitch: -0.2 };
            mesh.render_ex(self.ctx.as_mut(), [0.78, 0.78, 0.8, 1.0], &cam, &boxes, &meshes, std::f32::consts::PI / 3.2);
            let mut buf = vec![0u8; 640 * 480 * 4];
            self.ctx.texture_read_pixels(mesh.color_texture(), &mut buf);
            // 위아래가 뒤집혀 나온다.
            let mut img = image::RgbaImage::new(640, 480);
            for y in 0..480u32 {
                for x in 0..640u32 {
                    let i = (((479 - y) * 640 + x) * 4) as usize;
                    img.put_pixel(x, y, image::Rgba([buf[i], buf[i + 1], buf[i + 2], 255]));
                }
            }
            img.save(format!("{}/{}.png", self.out, name)).unwrap();
        }
        // 위에서 내려다본 갈림길 — (이름, 중심 x, 중심 z, 높이)
        for (name, x, z, h) in [("top_east", 118.0f32, 45.0f32, 150.0f32), ("top_west", -35.0, 80.0, 130.0), ("top_start", 20.0, 0.0, 70.0)] {
            let cam = Camera { pos: [x, h, z], yaw: 0.0, pitch: -std::f32::consts::FRAC_PI_2 + 0.001 };
            mesh.render_ex(self.ctx.as_mut(), [0.78, 0.78, 0.8, 1.0], &cam, &boxes, &meshes, std::f32::consts::PI / 3.2);
            let mut buf = vec![0u8; 640 * 480 * 4];
            self.ctx.texture_read_pixels(mesh.color_texture(), &mut buf);
            let mut img = image::RgbaImage::new(640, 480);
            for y in 0..480u32 {
                for x in 0..640u32 {
                    let i = (((479 - y) * 640 + x) * 4) as usize;
                    img.put_pixel(x, y, image::Rgba([buf[i], buf[i + 1], buf[i + 2], 255]));
                }
            }
            img.save(format!("{}/{}.png", self.out, name)).unwrap();
        }
        println!("saved {} shots", shots.len());
        window::order_quit();
    }
}

fn main() {
    let out = std::env::args().nth(1).expect("저장 폴더가 필요하다");
    let conf = conf::Conf { window_title: "race_preview".into(), window_width: 320, window_height: 240, ..Default::default() };
    miniquad::start(conf, move || Box::new(Stage { ctx: window::new_rendering_backend(), out, done: false }));
}
