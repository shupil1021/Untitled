//! 시계 팝업이 보여주는 날짜 계산(유닉스 초 → 연/월/일).

// 유닉스 초(KST 로 이미 보정한 값) → (년, 월, 일) — 달력 계산(Howard Hinnant 의 civil_from_days).
pub(super) fn civil_date(unix: i64) -> (i64, u32, u32) {
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
    use super::civil_date;

    #[test]
    fn civil_date_known_values() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(951_782_400), (2000, 2, 29)); // 윤일
        assert_eq!(civil_date(1_759_708_800), (2025, 10, 6));
        assert_eq!(civil_date(-86_400), (1969, 12, 31));
    }
}
