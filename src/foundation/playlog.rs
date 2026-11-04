//! 로컬 플레이 기록 — 단계마다 얼마나 걸렸는지, 어느 미니게임에서 몇 번 졌는지를 JSON 줄로 이 PC 안에만
//! 남긴다(`palaceos_playlog.jsonl`, 저장 파일 옆). **어디로도 전송하지 않는다.** 설정의 "플레이 기록"을 끄면 아무것도 안
//! 쓰고, 파일을 지우면 기록이 사라진다. 목적은 하나 — 플레이어가 어느 단계에서 오래 막히거나 그만두는지 개발자가 직접
//! 확인해서 다음에 손볼 곳을 정하는 것이다(examples/playlog_report.rs 가 요약해 보여 준다).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

const MAX_BYTES: u64 = 256 * 1024; // 이보다 커지면 오래된 줄부터 버린다
const KEEP_BYTES: usize = 128 * 1024;

pub fn log_path() -> PathBuf {
    super::save_dir().join("palaceos_playlog.jsonl")
}

// 이벤트 하나를 기록한다 — enabled 가 false 면(설정에서 껐으면) 아무것도 하지 않는다. 쓰기에 실패해도 게임은 그대로다.
pub fn event(enabled: bool, ev: &str, fields: Value) {
    if !enabled {
        return;
    }
    let _ = append_capped(&log_path(), &line(ev, fields, miniquad::date::now()), MAX_BYTES, KEEP_BYTES);
}

fn line(ev: &str, fields: Value, now: f64) -> String {
    let mut obj = json!({ "t": now.round() as i64, "ev": ev });
    if let (Some(map), Value::Object(extra)) = (obj.as_object_mut(), fields) {
        map.extend(extra);
    }
    obj.to_string()
}

// 줄 하나를 덧붙이고, 파일이 max 바이트를 넘으면 뒤쪽 keep 바이트(줄 경계부터)만 남긴다.
fn append_capped(path: &Path, line: &str, max: u64, keep: usize) -> std::io::Result<()> {
    if std::fs::metadata(path).map(|m| m.len() > max).unwrap_or(false)
        && let Ok(text) = std::fs::read_to_string(path)
    {
        let start = text.len().saturating_sub(keep);
        let cut = text[start..].find('\n').map_or(text.len(), |i| start + i + 1);
        std::fs::write(path, &text[cut.min(text.len())..])?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")
}

// 기록을 단계별로 요약한 것.
#[derive(Default, Debug, PartialEq)]
pub struct Summary {
    pub step_secs: BTreeMap<u64, f64>,   // 단계 번호(StoryStage::index) → 그 단계에서 쓴 총 시간(초)
    pub mini_fails: BTreeMap<String, u64>, // 미니게임 이름 → 실패 횟수
}

pub fn summarize(text: &str) -> Summary {
    let mut s = Summary::default();
    for l in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
        match v["ev"].as_str() {
            Some("step") => {
                if let (Some(from), Some(secs)) = (v["from"].as_u64(), v["secs"].as_f64()) {
                    *s.step_secs.entry(from).or_insert(0.0) += secs;
                }
            }
            Some("mini_fail") => {
                if let Some(kind) = v["kind"].as_str() {
                    *s.mini_fails.entry(kind.to_string()).or_insert(0) += 1;
                }
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("palaceos_playlog_{}_{}.jsonl", name, std::process::id()))
    }

    #[test]
    fn a_line_carries_time_event_and_fields() {
        let l = line("step", json!({ "from": 2, "to": 3, "secs": 41.0 }), 1_700_000_000.4);
        let v: Value = serde_json::from_str(&l).unwrap();
        assert_eq!((v["t"].as_i64(), v["ev"].as_str(), v["from"].as_u64(), v["secs"].as_f64()), (Some(1_700_000_000), Some("step"), Some(2), Some(41.0)));
    }

    #[test]
    fn summary_adds_up_time_per_step_and_counts_failures() {
        let text = [
            line("step", json!({ "from": 1, "to": 2, "secs": 30.0 }), 1.0),
            line("step", json!({ "from": 2, "to": 3, "secs": 90.0 }), 2.0),
            line("step", json!({ "from": 2, "to": 3, "secs": 10.0 }), 3.0), // 같은 단계를 다시 거친 경우도 합친다
            line("mini_fail", json!({ "kind": "PACK" }), 4.0),
            line("mini_fail", json!({ "kind": "PACK" }), 5.0),
            line("mini_fail", json!({ "kind": "SHY_EYE" }), 6.0),
            "깨진 줄은 건너뛴다".to_string(),
        ]
        .join("\n");
        let s = summarize(&text);
        assert_eq!(s.step_secs.get(&2), Some(&100.0));
        assert_eq!(s.step_secs.get(&1), Some(&30.0));
        assert_eq!((s.mini_fails.get("PACK"), s.mini_fails.get("SHY_EYE")), (Some(&2), Some(&1)));
    }

    #[test]
    fn disabled_logging_writes_nothing_and_a_big_file_is_trimmed_from_the_front() {
        // 껐으면 아무것도 쓰지 않는다(전역 경로를 건드리지 않고 enabled=false 경로만 확인).
        event(false, "step", json!({}));
        let p = temp("cap");
        let _ = std::fs::remove_file(&p);
        for i in 0..200 {
            append_capped(&p, &format!("{{\"ev\":\"x\",\"i\":{i}}}"), 1_000, 400).unwrap();
        }
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.len() < 1_100, "한도를 넘지 않는다: {}", text.len());
        assert!(text.lines().all(|l| serde_json::from_str::<Value>(l).is_ok()), "줄 경계에서만 잘린다");
        assert!(text.contains("\"i\":199"), "최근 줄은 남는다");
        assert!(!text.contains("\"i\":0}"), "오래된 줄은 버려진다");
        let _ = std::fs::remove_file(&p);
    }
}
