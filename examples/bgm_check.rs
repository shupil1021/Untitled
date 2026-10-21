// 개발용 — 배경음악 반복 재생이 오류 없이 도는지 몇 초 틀어본다. 사용법: cargo run --example bgm_check -- <wav 경로> <초>
fn main() {
    let path = std::env::args().nth(1).expect("wav 경로가 필요하다");
    let secs: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let a = crackhead::platform::video::Audio::start_looping(path, 0.05);
    println!("started: {}", a.is_some());
    std::thread::sleep(std::time::Duration::from_secs(secs));
    println!("done");
}
