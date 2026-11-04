// 개발용 도구 — 로컬 플레이 기록(palaceos_playlog.jsonl)을 읽어 단계별로 요약해 보여 준다.
// 사용법: cargo run --example playlog_report -- [기록 파일 경로]   (경로를 생략하면 저장 폴더의 기록을 쓴다)
// 기록은 이 PC 안에만 남고 어디로도 전송되지 않는다 — 오래 걸리거나 많이 지는 단계를 찾아 다음에 손볼 곳을 정하는 데 쓴다.

use crackhead::foundation::playlog::{log_path, summarize};
use crackhead::foundation::StoryStage;

fn main() {
    let path = std::env::args().nth(1).map(std::path::PathBuf::from).unwrap_or_else(log_path);
    let Ok(text) = std::fs::read_to_string(&path) else {
        println!("기록이 없다: {}", path.display());
        return;
    };
    let s = summarize(&text);
    println!("기록 파일: {}
", path.display());
    println!("단계별 머문 시간(초):");
    let mut rows: Vec<_> = s.step_secs.iter().collect();
    rows.sort_by(|a, b| b.1.total_cmp(a.1));
    for (idx, secs) in rows {
        let name = StoryStage::ALL.get(*idx as usize).map_or("?".to_string(), |st| format!("{st:?}"));
        println!("  {name:<12} {secs:>8.0}");
    }
    println!("
미니게임 실패 횟수:");
    for (kind, n) in &s.mini_fails {
        println!("  {kind:<12} {n}");
    }
}
