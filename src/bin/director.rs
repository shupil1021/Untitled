// 콘솔 창 없이 뜨게(GUI 앱으로) — 이게 없으면 실행할 때마다 검은 콘솔 창이 게임 창과 같이
// 떠서 연출용치고 지저분해 보인다.
#![windows_subsystem = "windows"]

// PalaceOS Director — 연출/영상 제작 전용 창. 실제 구현은 director/real.rs 에 있다
// (Windows 의 WASAPI 로 시스템 소리를 녹음하는 부분이 있어서 Windows 에서만 컴파일한다).

#[cfg(windows)]
#[path = "director/real.rs"]
mod real;

#[cfg(windows)]
fn main() {
    real::main();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("director 는 Windows 전용입니다.");
}
