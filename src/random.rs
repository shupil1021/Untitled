//! 연출용 난수 + 그걸로 만드는 "들쭉날쭉한 로딩 바" 곡선. 부팅 화면/로비 글리치/
//! 설치 마법사/게임 대화창/director 가 똑같은 xorshift64 를 각자 복사해 들고 있던 걸
//! 여기 하나로 모았다. 암호학적 품질은 필요 없고, 매번 조금씩 다른 연출이 나오면 된다
//! (외부 rand 크레이트 없이 충분하다).

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1) // xorshift 는 시드가 0 이면 영원히 0 만 나온다
    }

    // 지금 시각으로 시드 — 실행할 때마다 다른 연출이 나오게.
    pub fn from_time() -> Rng {
        Rng::new((miniquad::date::now() * 1e6) as u64)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }

    // 0.0..1.0
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() % 1_000_000) as f32 / 1_000_000.0
    }

    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.unit() * (max - min)
    }
}

// 로딩 바가 일정한 속도로 차지 않고 멈칫거리다 훅 튀도록 하는 (경과비율, 진행비율)
// 웨이포인트들 — (0,0) 에서 시작해 (1,1) 로 끝나고 구간마다 속도가 들쭉날쭉하다.
// 부팅 화면과 설치 마법사가 같이 쓴다.
pub struct LoadCurve(Vec<(f32, f32)>);

impl LoadCurve {
    pub fn random(rng: &mut Rng) -> LoadCurve {
        const SEGMENTS: usize = 7;
        let normalized = |v: Vec<f32>| {
            let sum: f32 = v.iter().sum();
            v.into_iter().map(|x| x / sum).collect::<Vec<f32>>()
        };
        let xd = normalized((0..SEGMENTS).map(|_| rng.range_f32(0.4, 1.6)).collect());
        // 진행량은 제곱을 줘서 절반은 거의 멈춘 듯 조금씩, 절반은 훅 튀도록 편차를 크게 만든다.
        let yd = normalized((0..SEGMENTS).map(|_| rng.range_f32(0.05, 1.0).powf(2.0)).collect());
        let (mut x, mut y) = (0.0, 0.0);
        let mut points = vec![(0.0, 0.0)];
        for i in 0..SEGMENTS {
            x += xd[i];
            y += yd[i];
            points.push((x, y));
        }
        // 부동소수점 누적 오차로 마지막 점이 (1.0, 1.0) 에 딱 안 맞으면 진행바가 99% 에서
        // 영영 멈춘다(예전 설치 마법사에서 실제로 있었던 버그) — 강제로 맞춘다.
        if let Some(last) = points.last_mut() {
            *last = (1.0, 1.0);
        }
        LoadCurve(points)
    }

    // 웨이포인트 사이를 선형보간해 경과비율 t(0..1) 에서의 진행비율을 구한다.
    pub fn sample(&self, t: f32) -> f32 {
        for w in self.0.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            if t <= x1 {
                let seg_t = if x1 > x0 { ((t - x0) / (x1 - x0)).clamp(0.0, 1.0) } else { 1.0 };
                return y0 + (y1 - y0) * seg_t;
            }
        }
        1.0
    }
}
