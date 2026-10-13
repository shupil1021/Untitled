//! ENA(유튜브 애니메이션 시리즈) 식 움직임 도구 — 움직임을 낮은 프레임(초당 8장)으로 뚝뚝 끊어 보여주고,
//! 도착할 때 한 번 지나쳤다가 돌아오는(오버슈트) 과장된 타이밍을 쓴다.

pub const ENA_FPS: f32 = 8.0;

// 시간을 ENA_FPS 간격으로 끊는다(같은 칸 안에선 같은 값).
pub fn ena_time(t: f32) -> f32 {
    (t * ENA_FPS).floor() / ENA_FPS
}

// 0 → 1 로 가는데 1 을 살짝 넘었다가 돌아오는 곡선(easeOutBack).
pub fn overshoot(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    const C1: f32 = 1.70158;
    let u = x - 1.0;
    1.0 + (C1 + 1.0) * u * u * u + C1 * u * u
}

// 0 → 1 로 부드럽게 가는 곡선(넘치지 않는다).
pub fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ena_helpers() {
        assert_eq!(ena_time(0.0), 0.0);
        assert_eq!(ena_time(0.12), 0.0, "같은 칸 안에선 값이 안 변한다");
        assert_eq!(ena_time(0.13), 0.125);
        assert_eq!(overshoot(0.0), 0.0);
        assert!((overshoot(1.0) - 1.0).abs() < 1e-5);
        assert!(overshoot(0.8) > 1.0, "도착 직전에 1 을 넘었다가 돌아온다");
        assert_eq!(smooth(0.0), 0.0);
        assert_eq!(smooth(1.0), 1.0);
        assert!(smooth(0.5) > 0.49 && smooth(0.5) < 0.51);
    }
}
