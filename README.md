# Untitled

Rust + [miniquad](https://github.com/not-fl3/miniquad)로 만든 **가짜 Windows 9x 데스크톱 OS** 위의
아날로그 호러 / ARG 게임. 화면은 CRT 모니터처럼 곡률·스캔라인·색수차가 입혀져 있다.

## 개요

플레이어는 낡은 PC 앞에 앉아 있다. 데스크톱에는 메일, 파일 탐색기, 메모장, 영상/이미지/사운드
플레이어 같은 프로그램이 있고, 어느 날 친구가 메일로 게임 하나를 보내온다. 설치해서 실행하면
방 안에서 말하는 문(도어즈)을 만나고, 그 부탁을 들어주려고 게임 안과 PC 화면을 오가게 된다 —
게임 속에서 한 일이 메일로 도착하고, 메일의 내용이 다시 게임에 영향을 준다.

- 전체화면 4:3 CRT 연출, 모든 UI 문자열은 영어/한국어/일본어
- 창 안에서 도는 1인칭 3D 게임(자체 3D 렌더러)과 그 안의 미니 게임
- 이야기는 [STORY.md](STORY.md), 제작 순서는 [ROADMAP.md](ROADMAP.md)

## 실행

Windows 전용이다(영상 재생에 Media Foundation, 오디오에 WASAPI, 웹 창에 WebView2를 쓴다).

```bash
cargo run              # 게임
cargo run --release    # 빠른 빌드
cargo test --lib       # 단위 테스트
```

릴리스 빌드는 [Releases](https://github.com/shupil1021/Untitled/releases)에서 받을 수 있다.

## 폴더 구조

```
src/
├── main.rs            진입점(창/프레임 루프)
├── lib.rs             라이브러리 진입점
├── scenes/            화면 단위: 로비, 부팅, 데스크톱, 종료 …
├── apps/              창 안에서 도는 프로그램: 메일, 탐색기, 메모장, 플레이어, 게임들
│   ├── doors_game/      메인 게임(방 + 문)
│   ├── maze_game/       서브 게임(미로)
│   └── game3d/          창 안 3D 게임이 같이 쓰는 도구
├── render/            2D/3D 렌더러, CRT 효과
├── platform/          Windows 전용 기능(영상·오디오, 웹뷰, IME)
├── bin/               녹화용 도구(director)
├── foundation.rs      가짜 파일 시스템 · 설정 · 저장
├── window_manager.rs  창 관리자
├── ui.rs · strings.rs 위젯 · 다국어 문자열
└── …                  기타 작은 모듈(random, signals, gamefiles 등)
assets/                아이콘 · 폰트 · 커서
docs/                  구현 상세 문서(IMPLEMENTATION.md)
STORY.md · ROADMAP.md · GITHUB.md
```

구현 방식과 동작 상세는 [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md)에 정리돼 있다.
