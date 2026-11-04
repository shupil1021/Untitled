//! 이 PC 의 현재 시간대(UTC 와의 차이) — 작업표시줄 시계와 날짜 팝업이 "진짜 컴퓨터의 시간"을 그대로 보여주려고 쓴다.
//! 서브 게임의 "시스템 시간을 1년 돌리기"는 플레이어가 Windows 의 날짜/시간 설정을 직접 바꾸는 것이라, 게임 안 시계가
//! 그 설정과 같은 시간대로 보여야 헷갈리지 않는다(예전엔 한국 시간으로 고정이라 다른 지역 PC 에서는 시각/날짜가 어긋났다).
//! 지금 시각 기준의 값이라 날짜를 크게 돌린 뒤에는 서머타임 한 시간이 어긋날 수 있다.

use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};

const TIME_ZONE_ID_DAYLIGHT: u32 = 2; // 지금이 서머타임이다(GetTimeZoneInformation 의 반환값)

// UTC 에서 로컬 시간까지 더할 초(예: 한국 +32400, 미국 동부 표준시 -18000).
pub fn local_utc_offset_secs() -> i64 {
    let mut tz: TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
    let id = unsafe { GetTimeZoneInformation(&mut tz) };
    let extra = if id == TIME_ZONE_ID_DAYLIGHT { tz.DaylightBias } else { tz.StandardBias };
    -((tz.Bias + extra) as i64) * 60
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_offset_is_a_plausible_timezone() {
        let off = super::local_utc_offset_secs();
        assert!((-14 * 3600..=14 * 3600).contains(&off), "{off}");
        assert_eq!(off % 900, 0, "시간대는 15분 단위다: {off}");
    }
}
