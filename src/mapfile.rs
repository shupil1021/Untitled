//! `mesh3d.rs` 장면(상자들)을 디스크에 저장/불러오는 JSON 포맷 — `bin/mesh3d_test.rs`
//! (저장 쪽, `Ctrl+S`)와 게임/테스트 뷰어(로드 쪽) 둘 다 이 구조체를 그대로 쓴다.
//! 텍스처는 `TextureId`(런타임 GPU 핸들)가 아니라 에셋 경로 문자열로 저장해두고,
//! 불러오는 쪽이 실제로 이미지를 읽어 텍스처를 만든 뒤 `to_box3d()`에 그 결과를
//! 넘겨준다 — `mesh3d`/`mapfile` 은 파일 IO 나 miniquad 컨텍스트를 몰라도 된다.

use serde::{Deserialize, Serialize};

use crate::mesh3d::{Box3D, BoxTexture};

#[derive(Clone, Serialize, Deserialize)]
pub struct MapBoxData {
    pub center: [f32; 3],
    pub half: [f32; 3],
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub roll: f32,
    #[serde(default = "default_color")]
    pub color: [f32; 4],
    // assets/ 기준 상대 경로 — 없으면(None) 단색.
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default = "default_true")]
    pub walkable: bool,
    #[serde(default = "default_true")]
    pub solid: bool,
    // 맵 에디터가 씬 목록에 보여주는 사람이 붙인 이름 — 없던 예전 저장 파일은
    // 그냥 빈 문자열로 불러와지고, 에디터가 "Box N"으로 대신 보여준다.
    #[serde(default)]
    pub name: String,
    // 부모 오브젝트의 인덱스(scene.boxes 안에서) — 에디터에서 이동시키면 이
    // 상자를 부모로 둔 자식들도 같은 만큼 같이 움직인다. 게임/뷰어는 지금은
    // 이 필드를 안 쓴다(에디터 전용 정보).
    #[serde(default)]
    pub parent: Option<usize>,
}

fn default_color() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}
fn default_true() -> bool {
    true
}

impl MapBoxData {
    pub fn from_box3d(b: &Box3D, texture_path: Option<String>, name: String, parent: Option<usize>) -> MapBoxData {
        MapBoxData {
            center: b.center,
            half: b.half,
            yaw: b.yaw,
            pitch: b.pitch,
            roll: b.roll,
            color: b.color,
            texture: texture_path,
            walkable: b.walkable,
            solid: b.solid,
            name,
            parent,
        }
    }

    // resolve_tex: 에셋 경로 문자열을 실제 TextureId 로 바꾸는 콜백(캐시는 호출부 책임).
    pub fn to_box3d(&self, resolve_tex: &mut impl FnMut(&str) -> miniquad::TextureId) -> Box3D {
        Box3D {
            center: self.center,
            half: self.half,
            yaw: self.yaw,
            pitch: self.pitch,
            roll: self.roll,
            color: self.color,
            texture: self.texture.as_deref().map(|p| BoxTexture { texture: resolve_tex(p) }),
            walkable: self.walkable,
            solid: self.solid,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MapScene {
    #[serde(default)]
    pub boxes: Vec<MapBoxData>,
    #[serde(default)]
    pub player_start: [f32; 3],
    #[serde(default)]
    pub player_start_yaw: f32,
}

impl Default for MapScene {
    fn default() -> Self {
        MapScene { boxes: Vec::new(), player_start: [0.0, 1.0, 0.0], player_start_yaw: 0.0 }
    }
}

impl MapScene {
    pub fn load(path: &str) -> std::io::Result<MapScene> {
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)
    }
}
