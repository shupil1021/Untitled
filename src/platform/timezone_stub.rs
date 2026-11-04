//! Windows 가 아닌 곳의 시간대 대체 구현 — 시간대를 알아낼 방법이 없으니 UTC 로 본다.

pub fn local_utc_offset_secs() -> i64 {
    0
}
