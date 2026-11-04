//! 작업표시줄 시계와 날짜 팝업이 보여주는 시각/날짜 계산(유닉스 초 → 시:분, 연/월/일).

// 지금 이 PC 의 로컬 유닉스 초 — 시스템 시간(UTC)에 PC 의 시간대 차이를 더한 값. 플레이어가 Windows 설정에서
// 시계를 돌리면 이 값이 그대로 따라간다.
pub(crate) fn local_unix_now() -> i64 {
    miniquad::date::now() as i64 + crate::platform::timezone::local_utc_offset_secs()
}

// ---- PC 시계가 크게 바뀌었는지 알아채기 ----
// 시스템 시계(벽시계)가 흐른 양에서 단조 시계가 잰 양을 빼면, 그 사이 누군가 시계를 돌린 만큼이 남는다(초).
pub(super) fn clock_skew_secs(wall_start: f64, wall_now: f64, mono_elapsed: f64) -> f64 {
    (wall_now - wall_start) - mono_elapsed
}

pub(super) const DAY: f64 = 86_400.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SkewNotice {
    CheckFlower, // 시계를 앞으로 크게 돌렸는데 꽃이 아직 안 폈다 — 서브 게임을 열어 보라고 알려준다
    SetClockBack, // 꽃이 핀 뒤에도 시계가 틀어져 있다 — 원래대로 돌려도 된다고 알려준다
}

// 지금 알려줄 안내가 있으면 그것. skew 는 clock_skew_secs() 값(초), 심었는지/폈는지는 서브 게임 진행.
pub(super) fn skew_notice(skew: f64, planted: bool, bloomed: bool) -> Option<SkewNotice> {
    if bloomed && skew.abs() > DAY {
        Some(SkewNotice::SetClockBack)
    } else if planted && !bloomed && skew > 300.0 * DAY {
        Some(SkewNotice::CheckFlower)
    } else {
        None
    }
}

// 로컬 유닉스 초 → (시, 분).
pub(crate) fn hour_minute(local_unix: i64) -> (u32, u32) {
    let secs = local_unix.rem_euclid(86_400);
    ((secs / 3600) as u32, (secs / 60 % 60) as u32)
}

// 유닉스 초(로컬 시간대로 이미 보정한 값) → (년, 월, 일) — 달력 계산(Howard Hinnant 의 civil_from_days).
pub(crate) fn civil_date(unix: i64) -> (i64, u32, u32) {
    let z = unix.div_euclid(86400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::{civil_date, clock_skew_secs, hour_minute, skew_notice, SkewNotice, DAY};

    #[test]
    fn civil_date_known_values() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29)); // 윤일
        assert_eq!(civil_date(1_759_708_800), (2025, 10, 6));
        assert_eq!(civil_date(-86_400), (1969, 12, 31));
    }

    // 같은 순간이 시간대에 따라 다른 시각/날짜로 보인다 — 한국(+9)과 미국 동부(-5)에서 2025-10-06 23:30 UTC.
    #[test]
    fn the_same_instant_reads_differently_per_timezone() {
        let utc = 1_759_708_800 + 23 * 3600 + 30 * 60; // 2025-10-06 23:30 UTC
        let kst = utc + 9 * 3600;
        let est = utc - 5 * 3600;
        assert_eq!((hour_minute(kst), civil_date(kst)), ((8, 30), (2025, 10, 7)));
        assert_eq!((hour_minute(est), civil_date(est)), ((18, 30), (2025, 10, 6)));
        assert_eq!(hour_minute(-60), (23, 59), "1970 이전에도 시각이 맞다");
    }

    #[test]
    fn skew_is_what_the_wall_clock_gained_beyond_real_elapsed_time() {
        assert_eq!(clock_skew_secs(1000.0, 1010.0, 10.0), 0.0, "정상적으로 흐르면 어긋남이 없다");
        assert_eq!(clock_skew_secs(0.0, 31_536_010.0, 10.0), 31_536_000.0, "1년 앞으로 돌리면 1년만큼 남는다");
        assert!(clock_skew_secs(1000.0, 500.0, 10.0) < 0.0, "뒤로 돌리면 음수다");
    }

    #[test]
    fn notices_depend_on_the_flower_state() {
        let year = 365.0 * DAY;
        assert_eq!(skew_notice(0.0, true, false), None);
        assert_eq!(skew_notice(2.0 * DAY, true, false), None, "며칠 어긋난 정도로는 알리지 않는다");
        assert_eq!(skew_notice(year, true, false), Some(SkewNotice::CheckFlower));
        assert_eq!(skew_notice(year, false, false), None, "아직 안 심었으면 상관없다");
        assert_eq!(skew_notice(year, true, true), Some(SkewNotice::SetClockBack));
        assert_eq!(skew_notice(-year, true, true), Some(SkewNotice::SetClockBack), "뒤로 돌려 둔 채여도 알려준다");
        assert_eq!(skew_notice(0.0, true, true), None, "시계가 정상이면 조용하다");
    }
}

