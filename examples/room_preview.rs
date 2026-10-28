// 개발용 도구 — 시작 방(가구 모델 포함)을 여러 시점에서 그려 PNG 로 저장한다.
// 사용법: cargo run --example room_preview -- <PNG 를 저장할 폴더>

use crackhead::apps::preview_start_room;
use crackhead::render::mesh3d::{Camera, Mesh3D};
use miniquad::*;

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    out: String,
    done: bool,
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if self.done {
            window::order_quit();
            return;
        }
        self.done = true;
        let (boxes, meshes) = preview_start_room();
        let mut mesh = Mesh3D::new(self.ctx.as_mut(), 640, 480);
        // (이름, 위치, yaw, pitch) — yaw 0 이 -Z 방향, +yaw 는 왼쪽으로 돈다.
        let shots: [(&str, [f32; 3], f32, f32); 10] = [
            ("spawn", [0.0, 1.6, 1.2], 0.0, 0.0),
            ("door_close", [0.0, 1.4, -1.5], 0.0, 0.0),
            ("outside_back", [0.0, 1.4, -7.0], std::f32::consts::PI, 0.0),
            ("top", [0.0, 12.0, -0.5], 0.0, -1.5),
            ("west_wall", [-0.2, 1.4, -0.5], 1.57, -0.1),
            ("east_wall", [0.0, 1.4, 0.5], -1.57, -0.1),
            ("desk_close", [-1.0, 1.4, -1.5], 1.57, -0.35),
            ("mailbox_close", [-1.4, 1.1, 0.4], 1.57, 0.0),
            ("dresser_close", [1.2, 1.2, 0.5], -1.57, 0.0),
            ("dresser_oblique", [1.4, 1.5, 1.9], -0.9, -0.2),
        ];
        for (name, pos, yaw, pitch) in shots {
            let cam = Camera { pos, yaw, pitch };
            mesh.render_ex(self.ctx.as_mut(), [0.1, 0.1, 0.1, 1.0], &cam, &boxes, &meshes, std::f32::consts::PI / 2.4);
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
    let conf = conf::Conf { window_title: "room_preview".into(), window_width: 320, window_height: 240, ..Default::default() };
    miniquad::start(conf, move || Box::new(Stage { ctx: window::new_rendering_backend(), out, done: false }));
}
