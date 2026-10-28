//! 코드로 만드는 사운드(WAV) — 앰비언스와 문 삐걱거림.

use crate::random::Rng;

pub const SOUND_RATE: u32 = 22050;

// SOUND_NAMES[i] 의 WAV 파일 바이트(16비트 모노 PCM) — 사운드 플레이어가 열 때 만든다.
pub fn sound_wav(i: usize) -> Vec<u8> {
    match i {
        0 => ambience_wav(),
        _ => creak_wav(),
    }
}

// WAV 바이트의 재생 시간(초).
pub fn wav_duration(wav: &[u8]) -> f32 {
    (wav.len().saturating_sub(44) / 2) as f32 / SOUND_RATE as f32
}

fn wav_bytes(samples: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt 청크 크기
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // 모노
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // 초당 바이트
    out.extend_from_slice(&2u16.to_le_bytes()); // 블록 크기
    out.extend_from_slice(&16u16.to_le_bytes()); // 비트 수
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

// 낮게 웅웅거리는 방 안 소리 — 55Hz 저음 + 살짝 흔들리는 5도 위 + 느리게 숨 쉬는 노이즈.
fn ambience_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xA11B);
    let secs = 6.0;
    let n = (SOUND_RATE as f32 * secs) as usize;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / SOUND_RATE as f32;
            let tau = std::f32::consts::TAU;
            let hum = 0.25 * (tau * 55.0 * t).sin() + 0.12 * (tau * 82.5 * t + 0.8 * (tau * 0.2 * t).sin()).sin();
            let breath = 0.5 + 0.5 * (tau * 0.3 * t).sin();
            let noise = (rng.unit() * 2.0 - 1.0) * 0.05 * breath;
            let fade = (t / 0.5).min(1.0).min(((secs - t) / 0.5).max(0.0));
            ((hum + noise) * fade * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, SOUND_RATE)
}

// 문 삐걱거리는 소리 — 높은 톱니파가 점점 낮아지며 떨린다.
fn creak_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xC0FFEE);
    let secs = 1.4;
    let n = (SOUND_RATE as f32 * secs) as usize;
    let mut phase = 0.0f32;
    let samples: Vec<i16> = (0..n)
        .map(|i| {
            let t = i as f32 / SOUND_RATE as f32;
            let k = t / secs;
            let freq = 200.0 - 110.0 * k + 6.0 * (std::f32::consts::TAU * 14.0 * t).sin();
            phase = (phase + freq / SOUND_RATE as f32).fract();
            let saw = phase * 2.0 - 1.0;
            let env = (k * 12.0).min(1.0) * (1.0 - k).powf(0.7);
            ((saw * 0.35 + (rng.unit() * 2.0 - 1.0) * 0.1) * env * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, SOUND_RATE)
}

// 퀘스트 시작 효과음(MAIN H-2) — 방귀 소리(떨리며 낮아지는 톱니파 + 노이즈) 다음에 중세풍 나팔 팡파르.
pub fn quest_start_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xFA27);
    let rate = SOUND_RATE as f32;
    let tau = std::f32::consts::TAU;
    let mut samples: Vec<i16> = Vec::new();
    // 방귀 0.9초
    let secs = 0.9;
    let mut phase = 0.0f32;
    for i in 0..(rate * secs) as usize {
        let t = i as f32 / rate;
        let k = t / secs;
        let freq = 95.0 + 38.0 * (tau * 7.0 * t).sin().abs() - 45.0 * k;
        phase = (phase + freq / rate).fract();
        let saw = phase * 2.0 - 1.0;
        let flutter = 0.55 + 0.45 * (tau * 34.0 * t).sin();
        let env = (k * 25.0).min(1.0) * (1.0 - k).powf(0.4);
        samples.push(((saw * 0.55 * flutter + (rng.unit() * 2.0 - 1.0) * 0.18) * env * i16::MAX as f32 * 0.6) as i16);
    }
    samples.extend(std::iter::repeat_n(0i16, (rate * 0.3) as usize));
    // 나팔 팡파르 — 같은 음 셋 + 높은 음 하나(사각파 + 3배음).
    for (freq, secs) in [(392.0f32, 0.2f32), (392.0, 0.2), (392.0, 0.2), (523.25, 0.7)] {
        for i in 0..(rate * secs) as usize {
            let t = i as f32 / rate;
            let wave = (tau * freq * t).sin().signum() * 0.5 + (tau * freq * 3.0 * t).sin() * 0.2;
            let env = (t * 60.0).min(1.0) * (1.0 - t / secs).max(0.0).powf(0.5);
            samples.push((wave * env * i16::MAX as f32 * 0.35) as i16);
        }
    }
    wav_bytes(&samples, SOUND_RATE)
}

// 변기 물 내리는 소리(MAIN H-6) — 높은 소리에서 낮은 소리로 내려가는 노이즈에 꼬르륵거리는 저음.
pub fn flush_wav() -> Vec<u8> {
    let mut rng = Rng::new(0xF105);
    let rate = SOUND_RATE as f32;
    let tau = std::f32::consts::TAU;
    let secs = 2.2;
    let mut low = 0.0f32;
    let samples: Vec<i16> = (0..(rate * secs) as usize)
        .map(|i| {
            let t = i as f32 / rate;
            let k = t / secs;
            // 노이즈를 로우패스 — 시간이 갈수록 컷오프가 내려간다.
            let a = 0.55 - 0.45 * k;
            low += (rng.unit() * 2.0 - 1.0 - low) * a;
            let gurgle = 0.25 * (tau * (60.0 + 25.0 * (tau * 3.0 * t).sin()) * t).sin() * k;
            let env = (t * 8.0).min(1.0) * (1.0 - k).powf(0.6);
            ((low * 0.7 + gurgle) * env * i16::MAX as f32 * 0.6) as i16
        })
        .collect();
    wav_bytes(&samples, SOUND_RATE)
}
