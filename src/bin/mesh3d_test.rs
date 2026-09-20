// 콘솔 창 없이 뜨게(GUI 앱으로) — director.rs/director_panel.rs 와 같은 이유.
#![windows_subsystem = "windows"]

//! `crate::mesh3d`(진짜 3D 메쉬 렌더러)만 따로 띄워서 확인하는 테스트 창.
//! `cargo run --bin mesh3d_test` 로 띄운다.
//!
//! 장면은 전부 `Box3D`(회전 가능한 직육면체) 하나로만 만든다: 평평한 바닥,
//! 기울어진 경사로(램프, pitch 회전), 그 위 높은 발판, 옆으로 기운 벽(roll 회전)
//! — 바닥 높낮이도 기울어진 벽/경사로도 전부 같은 상자 타입 하나로 표현된다는 걸
//! 보여주는 게 목적이다.
//!
//! 조작: W/S 전진/후진, A/D 좌우 회전, ↑/↓ 로 위아래를 본다, Space 로 점프,
//! `[`/`]` 로 색상 단계 수(`Mesh3D::set_retro_shading`) 줄이기/늘리기, `;`/`'`
//! 로 디더링 세기 줄이기/늘리기, `\` 로 레트로 프리셋 켬/끔 토글, Esc 로 종료.

use miniquad::*;

use crackhead::gfx::Renderer;
use crackhead::mesh3d::{ground_height, resolve_horizontal, Box3D, Camera, Mesh3D};
use crackhead::scenes::Input;

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;
const FOV_Y: f32 = std::f32::consts::PI / 3.2;

const MOVE_SPEED: f32 = 3.2;
const TURN_SPEED: f32 = 2.4;
const LOOK_SPEED: f32 = 1.6;
const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_2 - 0.05;
const PLAYER_RADIUS: f32 = 0.3;
const PLAYER_HEIGHT: f32 = 1.7;
const EYE_OFFSET: f32 = 1.55; // 발 기준 눈 높이
const GRAVITY: f32 = -12.0;
const JUMP_SPEED: f32 = 4.6;

const SKY_COLOR: [f32; 4] = [0.55, 0.65, 0.78, 1.0];
const GROUND_COLOR: [f32; 4] = [0.32, 0.45, 0.3, 1.0];
const RAMP_COLOR: [f32; 4] = [0.5, 0.42, 0.3, 1.0];
const PLATFORM_COLOR: [f32; 4] = [0.45, 0.45, 0.5, 1.0];
const WALL_COLOR: [f32; 4] = [0.55, 0.3, 0.3, 1.0];
const LEANING_WALL_COLOR: [f32; 4] = [0.35, 0.35, 0.6, 1.0];

// 바닥 높낮이(경사로 → 발판)와 기울어진 벽을 한 장면에 같이 두고 확인한다.
fn build_scene() -> Vec<Box3D> {
    let mut boxes = Vec::new();

    // 바닥 — 아주 넓은 평평한 상자 하나.
    boxes.push(Box3D {
        center: [0.0, -0.25, 0.0],
        half: [15.0, 0.25, 15.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: GROUND_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 경사로 — pitch 회전 하나로 "바닥 높이가 서서히 올라가는" 구간을 만든다.
    // 로컬 +Z 축이 pitch 만큼 기울어지면서 월드 +Z 로 갈수록 y 가 올라간다.
    let ramp_start_z: f32 = 3.0;
    let ramp_run: f32 = 6.0; // 이 구간의 세계-Z 방향 길이
    let ramp_rise: f32 = 1.6; // 이 구간에서 올라가는 높이
    let ramp_angle = ramp_rise.atan2(ramp_run);
    let ramp_half_len = (ramp_run * ramp_run + ramp_rise * ramp_rise).sqrt() / 2.0;
    boxes.push(Box3D {
        center: [0.0, ramp_rise / 2.0, ramp_start_z + ramp_run / 2.0],
        half: [2.2, 0.15, ramp_half_len],
        yaw: 0.0,
        pitch: -ramp_angle,
        roll: 0.0,
        color: RAMP_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 경사로 꼭대기로 이어지는 높은 발판.
    boxes.push(Box3D {
        center: [0.0, ramp_rise - 0.25, ramp_start_z + ramp_run + 3.0],
        half: [3.0, 0.25, 3.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: PLATFORM_COLOR,
        texture: None,
        walkable: true,
        solid: true,
    });

    // 발판 가장자리에 낮은 난간 벽(평범한 축정렬 벽 — 비교용).
    boxes.push(Box3D {
        center: [3.0, ramp_rise + 0.5, ramp_start_z + ramp_run + 3.0],
        half: [0.15, 0.75, 3.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        color: WALL_COLOR,
        texture: None,
        walkable: false,
        solid: true,
    });

    // 옆으로 기운 벽 — roll 회전만으로 "기울어진 벽"이 그대로 나온다는 걸 보여준다.
    // walkable=false 라 그 위로 올라타지지 않고 순수하게 막는 장애물로만 동작한다.
    boxes.push(Box3D {
        center: [-4.5, 1.2, -3.0],
        half: [2.2, 1.6, 0.15],
        yaw: 0.4,
        pitch: 0.0,
        roll: 0.3,
        color: LEANING_WALL_COLOR,
        texture: None,
        walkable: false,
        solid: true,
    });

    boxes
}

struct Player {
    feet: [f32; 3],
    yaw: f32,
    pitch: f32,
    vel_y: f32,
    grounded: bool,
}

impl Player {
    fn camera(&self) -> Camera {
        Camera { pos: [self.feet[0], self.feet[1] + EYE_OFFSET, self.feet[2]], yaw: self.yaw, pitch: self.pitch }
    }

    fn update(&mut self, input: &Input, dt: f32, boxes: &[Box3D]) {
        // forward_flat()=[-sin(yaw),0,-cos(yaw)] 기준으로, yaw 를 줄이는 쪽이
        // 화면상 오른쪽으로 도는 것이다(D가 오른쪽으로 돌아야 하니 반대로 A는 +).
        if input.is_down(KeyCode::A) {
            self.yaw += TURN_SPEED * dt;
        }
        if input.is_down(KeyCode::D) {
            self.yaw -= TURN_SPEED * dt;
        }
        if input.is_down(KeyCode::Up) {
            self.pitch = (self.pitch + LOOK_SPEED * dt).min(MAX_PITCH);
        }
        if input.is_down(KeyCode::Down) {
            self.pitch = (self.pitch - LOOK_SPEED * dt).max(-MAX_PITCH);
        }

        let cam = self.camera();
        let fwd = cam.forward_flat();
        let mut move_dir = [0.0f32, 0.0, 0.0];
        if input.is_down(KeyCode::W) {
            move_dir = crackhead::mesh3d::v_add(move_dir, fwd);
        }
        if input.is_down(KeyCode::S) {
            move_dir = crackhead::mesh3d::v_sub(move_dir, fwd);
        }
        let len = crackhead::mesh3d::v_len(move_dir);
        if len > 1e-4 {
            let step = crackhead::mesh3d::v_scale(move_dir, MOVE_SPEED * dt / len);
            self.feet[0] += step[0];
            self.feet[2] += step[2];
        }

        if input.pressed(KeyCode::Space) && self.grounded {
            self.vel_y = JUMP_SPEED;
            self.grounded = false;
        }

        let pushed = resolve_horizontal(self.feet, PLAYER_RADIUS, self.feet[1], PLAYER_HEIGHT, boxes);
        self.feet[0] = pushed[0];
        self.feet[2] = pushed[2];

        self.vel_y += GRAVITY * dt;
        let predicted_y = self.feet[1] + self.vel_y * dt;
        match ground_height(self.feet[0], self.feet[2], self.feet[1], 60.0, boxes) {
            Some(g) if predicted_y <= g && self.vel_y <= 0.0 => {
                self.feet[1] = g;
                self.vel_y = 0.0;
                self.grounded = true;
            }
            _ => {
                self.feet[1] = predicted_y;
                self.grounded = false;
            }
        }
    }
}

struct Stage {
    ctx: Box<dyn RenderingBackend>,
    renderer: Renderer,
    mesh3d: Mesh3D,
    boxes: Vec<Box3D>,
    player: Player,
    input: Input,
    last_time: f64,
}

impl Stage {
    fn new() -> Stage {
        let mut ctx: Box<dyn RenderingBackend> = window::new_rendering_backend();
        let renderer = Renderer::new(ctx.as_mut());
        let mut mesh3d = Mesh3D::new(ctx.as_mut(), WIN_W as u32, WIN_H as u32);
        // 새로 추가한 디더링/색상 제한 셰이더를 바로 보여주려고 PS1 스타일
        // 프리셋을 켜둔다 — `\` 키로 끄고 켤 수 있다.
        mesh3d.set_retro_shading(1.0, 5.0);
        let boxes = build_scene();
        let player = Player { feet: [0.0, 0.0, -3.0], yaw: std::f32::consts::FRAC_PI_2, pitch: 0.0, vel_y: 0.0, grounded: true };
        Stage { ctx, renderer, mesh3d, boxes, player, input: Input::default(), last_time: date::now() }
    }
}

impl EventHandler for Stage {
    fn update(&mut self) {}

    fn draw(&mut self) {
        let now = date::now();
        let dt = ((now - self.last_time) as f32).min(0.5);
        self.last_time = now;

        self.player.update(&self.input, dt, &self.boxes);

        self.mesh3d.render(self.ctx.as_mut(), SKY_COLOR, &self.player.camera(), &self.boxes, FOV_Y);

        self.renderer.begin(WIN_W, WIN_H);
        // mesh3d 오프스크린 결과를 창 전체에 스프라이트로 끼워 넣는다 — v 를 뒤집는 건
        // crt.rs 의 오프스크린 합성과 같은 이유(오프스크린 텍스처는 위아래가 뒤집혀 있다).
        let tex = self.mesh3d.color_texture();
        self.renderer.sprite_uv(tex, 0.0, 0.0, WIN_W, WIN_H, 0.0, 1.0, 1.0, 0.0, [1.0, 1.0, 1.0, 1.0]);

        self.renderer.rect(0.0, 0.0, WIN_W, 18.0, [0.0, 0.0, 0.0, 0.55]);
        self.renderer.text(6.0, 3.0, "mesh3d.rs test - WASD move/turn, Up/Down look, Space jump, Esc quit", 0.7, [1.0, 1.0, 1.0, 1.0]);
        let (dither, levels) = self.mesh3d.retro_shading();
        let status = format!(
            "pos=({:.1},{:.1},{:.1}) grounded={} | dither={:.2}([;/') levels={:.0}([/]) \\=toggle",
            self.player.feet[0], self.player.feet[1], self.player.feet[2], self.player.grounded, dither, levels
        );
        self.renderer.text(6.0, WIN_H - 16.0, &status, 0.7, [1.0, 1.0, 0.6, 1.0]);

        self.ctx.begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.renderer.flush(self.ctx.as_mut());
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        self.input.end_frame();
    }

    fn key_down_event(&mut self, keycode: KeyCode, _mods: KeyMods, repeat: bool) {
        if keycode == KeyCode::Escape {
            window::order_quit();
        }
        if !repeat {
            let (dither, levels) = self.mesh3d.retro_shading();
            match keycode {
                KeyCode::LeftBracket => self.mesh3d.set_retro_shading(dither, levels - 1.0),
                KeyCode::RightBracket => self.mesh3d.set_retro_shading(dither, levels + 1.0),
                KeyCode::Semicolon => self.mesh3d.set_retro_shading(dither - 0.1, levels),
                KeyCode::Apostrophe => self.mesh3d.set_retro_shading(dither + 0.1, levels),
                // 지금 켜져 있으면(디더나 색상 제한이 조금이라도 걸려 있으면) 완전히
                // 끄고, 꺼져 있으면 PS1 스타일 프리셋(채널당 5단계 + 최대 디더)을 켠다.
                KeyCode::Backslash => {
                    if dither > 0.0 || levels < 255.0 {
                        self.mesh3d.set_retro_shading(0.0, 256.0);
                    } else {
                        self.mesh3d.set_retro_shading(1.0, 5.0);
                    }
                }
                _ => {}
            }
        }
        self.input.on_key_down(keycode, repeat);
    }

    fn key_up_event(&mut self, keycode: KeyCode, _mods: KeyMods) {
        self.input.on_key_up(keycode);
    }
}

fn main() {
    let conf = conf::Conf {
        window_title: "Mesh3D Test".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        fullscreen: false,
        high_dpi: false,
        ..Default::default()
    };
    miniquad::start(conf, || Box::new(Stage::new()));
}
