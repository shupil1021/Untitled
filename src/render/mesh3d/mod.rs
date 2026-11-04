//! 진짜 3D 메쉬 렌더러 — GPU 깊이 테스트가 있는 실제 삼각형 래스터라이저(miniquad
//! 파이프라인)라서, 벽/바닥을 전부 `Box3D`(회전 가능한 직육면체) 하나로 표현하고 그 회전
//! (yaw/pitch/roll)만으로 "기울어진 벽"도 "경사로(램프)"도 자연스럽게 나온다 —
//! 별도의 특수 케이스가 필요 없다.
//!
//! 렌더링 방식: `Box3D` 각각을 CPU 에서 월드 공간 삼각형(12개, 면마다 2개)으로 펼친
//! 뒤, 텍스처별로 묶어 GPU 에 올리고 깊이 테스트를 켠 오프스크린 렌더 타깃에 그린다.
//! 그 결과 컬러 텍스처를 `gfx::Renderer::sprite_uv`로 일반 2D 창 안에 스프라이트처럼
//! 끼워 넣으면(video.rs 가 디코딩한 영상 프레임을 텍스처로 올려 보여주는 것과 같은
//! 요령) 나머지 UI(HUD 등)는 지금까지처럼 2D 배칭 렌더러로 그대로 그릴 수 있다.
//!
//! `Box3D` 하나로는 표현 못 하는 형태(블렌더 등에서 만든 임의의 폴리곤 소품)를
//! 위해 `MeshObject`(+ `MeshData`/`parse_obj`)도 있다 — Wavefront OBJ 텍스트
//! 파일을 외부 크레이트 없이 직접 파싱해서 실제 삼각형 메시로 불러오고, 같은
//! `Vertex` 포맷·같은 셰이더/파이프라인을 그대로 타서 `Mesh3D::render_ex()`가
//! `Box3D`와 한 화면에 같이 그린다. 바닥 높이(`mesh_ground_height`)/수평 충돌
//! (`mesh_resolve_horizontal`)도 실제 삼각형 단위로(점-삼각형 최근접점 계산)
//! 판정해서, `Box3D`의 AABB 지름길이 없어도 정밀하게 막히고 밟힌다. 시작 방의 가구와 정글
//! 지형이 이 경로로 들어온다(apps/doors_game/furniture.rs).
//!
//! 파일 구성: `math`(벡터/행렬/카메라) · `shapes`(Box3D, MeshObject, Vertex) · `obj`(OBJ/MTL 파서) ·
//! `collide`(바닥 높이/수평 충돌) · `gpu`(Mesh3D 렌더러, 셰이더, 정점 캐시). 바깥에서는 예전처럼
//! `render::mesh3d::이름` 으로 그대로 쓴다.

mod collide;
mod gpu;
mod math;
mod obj;
mod shapes;
#[cfg(test)]
mod tests;

pub use collide::*;
pub use gpu::*;
pub use math::*;
pub use obj::*;
pub use shapes::*;
