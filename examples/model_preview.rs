// 개발용 도구 — 폴더 안의 모든 OBJ(+같은 이름의 MTL)를 하나씩 앞뒤 두 시점에서 그려 PNG 로 저장한다.
// 사용법: cargo run --release --example model_preview -- <OBJ 폴더> <PNG 를 저장할 폴더>

use std::rc::Rc;

use crackhead::render::mesh3d::{parse_obj_mtl, Camera, Mesh3D, MeshObject};
use miniquad::*;

const W: u32 = 640;
const H: u32 = 480;

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    dir: String,
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
        let mut mesh = Mesh3D::new(self.ctx.as_mut(), W, H);
        let mut paths: Vec<_> = std::fs::read_dir(&self.dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "obj")).collect();
        paths.sort();
        for path in paths {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let text = std::fs::read_to_string(&path).unwrap();
            let mtl = std::fs::read_to_string(path.with_extension("mtl")).unwrap_or_default();
            let data = match parse_obj_mtl(&text, &mtl) {
                Ok(d) => d,
                Err(e) => {
                    println!("{name}: 읽기 실패 — {e}");
                    continue;
                }
            };
            // 경계 상자 → 가장 긴 변이 4 가 되도록 줄이고 원점에 맞춘다.
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for v in &data.vertices {
                for i in 0..3 {
                    lo[i] = lo[i].min(v.pos[i]);
                    hi[i] = hi[i].max(v.pos[i]);
                }
            }
            let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
            let s = 4.0 / size[0].max(size[1]).max(size[2]).max(1e-6);
            let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
            let obj = MeshObject {
                center: [-mid[0] * s, -mid[1] * s, -mid[2] * s],
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                scale: [s, s, s],
                color: [1.0, 1.0, 1.0, 1.0],
                texture: None,
                walkable: false,
                solid: false,
                mesh: Rc::new(data),
            };
            let mut sheet = image::RgbaImage::new(W * 2, H);
            // 앞쪽 3/4(오른쪽 위에서) / 뒤쪽 3/4(왼쪽 위에서).
            for (k, yaw) in [0.6f32, 0.6 + std::f32::consts::PI].into_iter().enumerate() {
                let pitch = -0.4f32;
                let mut cam = Camera { pos: [0.0; 3], yaw, pitch };
                let f = cam.forward();
                let dist = 6.5;
                cam.pos = [-f[0] * dist, -f[1] * dist, -f[2] * dist];
                mesh.render_ex(self.ctx.as_mut(), [0.12, 0.12, 0.15, 1.0], &cam, &[], std::slice::from_ref(&obj), std::f32::consts::PI / 3.0);
                let mut buf = vec![0u8; (W * H * 4) as usize];
                self.ctx.texture_read_pixels(mesh.color_texture(), &mut buf);
                for y in 0..H {
                    for x in 0..W {
                        let i = (((H - 1 - y) * W + x) * 4) as usize;
                        sheet.put_pixel(k as u32 * W + x, y, image::Rgba([buf[i], buf[i + 1], buf[i + 2], 255]));
                    }
                }
            }
            sheet.save(format!("{}/{}.png", self.out, name)).unwrap();
            println!("{name}: size {:.2} x {:.2} x {:.2}, {} tris", size[0], size[1], size[2], obj.mesh.triangles.len());
        }
        window::order_quit();
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("OBJ 폴더가 필요하다");
    let out = args.next().expect("저장 폴더가 필요하다");
    std::fs::create_dir_all(&out).unwrap();
    let conf = conf::Conf { window_title: "model_preview".into(), window_width: 320, window_height: 240, ..Default::default() };
    miniquad::start(conf, move || Box::new(Stage { ctx: window::new_rendering_backend(), dir, out, done: false }));
}
