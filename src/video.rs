//! mp4 재생: Windows Media Foundation(OS 내장 디코더)로 데믹싱+디코딩,
//! WASAPI 로 오디오 재생. Baseline/Main/High 등 어떤 H.264 프로파일이든 재생 가능.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Once};
use std::time::Duration;

use windows::core::{GUID, PCWSTR, PROPVARIANT};
use windows::Win32::Media::Audio::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

static MF_INIT: Once = Once::new();

fn ensure_mf() {
    MF_INIT.call_once(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
    });
}

const VIDEO_STREAM: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const ALL_STREAMS: u32 = MF_SOURCE_READER_ALL_STREAMS.0 as u32;

pub struct Video {
    reader: IMFSourceReader,
    pub w: usize,
    pub h: usize,
    stride: i32, // 음수면 bottom-up
    frame_dt: f32,
    acc: f32,
    pos_100ns: i64,      // 마지막으로 읽은 프레임의 재생 위치 (100ns 단위)
    duration_100ns: i64, // 전체 길이 (100ns 단위, 못 구하면 0)
    pub rgba: Vec<u8>,
    pub dirty: bool,
    pub ended: bool, // 끝까지 재생됨 (더 이상 자동으로 되감아 반복하지 않는다)
    // 꼬리 검은 화면 스캔(별도 스레드) 결과를 기다리는 채널. 도착하면 duration 을
    // 그 값으로 당긴다. None 이면 스캔 자체를 안 했거나 이미 결과를 받았다.
    pending_trim: Option<Receiver<Option<i64>>>,
}

// windows-rs 는 IMFSourceReader(COM 포인터) 를 보수적으로 !Send 로 둔다. 하지만 이
// 프로젝트는 모든 스레드를 COINIT_MULTITHREADED(MTA) 로 초기화하고, MTA 에서는 같은
// 객체를 "동시에" 여러 스레드에서 부르는 게 아니라 "소유권을 완전히 넘기고 그 다음
// 부터는 한 스레드만 쓰는" 핸드오프라면 안전하다(마샬링 불필요). Video 를 여는 배경
// 스레드는 다 만든 뒤 채널로 넘기고 더 이상 건드리지 않으므로 정확히 이 경우에
// 해당한다 — 그래서 여기서 Send 를 명시적으로 허용한다.
unsafe impl Send for Video {}

// 파일에서 RGB32 출력 소스 리더를 열고 (리더, w, h, stride, frame_dt) 반환.
unsafe fn open_reader(path: &str) -> Option<(IMFSourceReader, usize, usize, i32, f32)> {
    unsafe {
        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        // 색변환(RGB32 출력)을 쓰려면 비디오 처리 활성화 속성이 필요.
        let mut attrs_opt: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut attrs_opt, 1).ok()?;
        let attrs = attrs_opt?;
        attrs.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1).ok()?;

        let reader: IMFSourceReader = match MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), Some(&attrs)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[mf] MFCreateSourceReaderFromURL failed: {e:?}");
                return None;
            }
        };
        reader.SetStreamSelection(ALL_STREAMS, false).ok()?;
        reader.SetStreamSelection(VIDEO_STREAM, true).ok()?;

        // 출력 포맷을 RGB32 로 → OS 가 디코더+색변환을 알아서 붙인다.
        let mt: IMFMediaType = MFCreateMediaType().ok()?;
        mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).ok()?;
        mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).ok()?;
        if let Err(e) = reader.SetCurrentMediaType(VIDEO_STREAM, None, &mt) {
            eprintln!("[mf] SetCurrentMediaType(RGB32) failed: {e:?}");
            return None;
        }

        let cur: IMFMediaType = reader.GetCurrentMediaType(VIDEO_STREAM).ok()?;
        let size = cur.GetUINT64(&MF_MT_FRAME_SIZE).ok()?;
        let w = (size >> 32) as usize;
        let h = (size & 0xffff_ffff) as usize;
        if w == 0 || h == 0 {
            return None;
        }
        let stride = cur.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| s as i32).unwrap_or((w * 4) as i32);
        let frame_dt = match cur.GetUINT64(&MF_MT_FRAME_RATE) {
            Ok(fr) => {
                let num = (fr >> 32) as f32;
                let den = (fr & 0xffff_ffff) as f32;
                if num > 0.0 { den / num } else { 1.0 / 30.0 }
            }
            Err(_) => 1.0 / 30.0,
        };
        Some((reader, w, h, stride, frame_dt))
    }
}

// detect_content_end 스캔을 백그라운드 스레드에서 돌리기 위해, 재생용 리더와는
// 완전히 별개인 리더를 이 스레드 안에서 새로 연다(COM 아파트먼트는 스레드마다
// 따로 초기화해야 하므로 audio_thread 와 같은 패턴 — ensure_mf() 의 Once 를
// 재사용하면 이 스레드에선 실제로 초기화가 안 된다).
unsafe fn scan_content_end(path: &str, duration_100ns: i64) -> Option<i64> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);
        let (reader, w, h, stride, frame_dt) = open_reader(path)?;
        let mut v = Video {
            reader,
            w,
            h,
            stride,
            frame_dt,
            acc: 0.0,
            pos_100ns: 0,
            duration_100ns,
            rgba: vec![255u8; w * h * 4],
            dirty: false,
            ended: false,
            pending_trim: None,
        };
        v.detect_content_end()
    }
}

impl Video {
    pub fn from_file(path: &str) -> Option<Video> {
        ensure_mf();
        let (reader, w, h, stride, frame_dt) = unsafe { open_reader(path)? };
        // 전체 길이(있으면). PROPVARIANT -> i64 변환 실패하면 0(=길이 모름) 처리.
        let duration_100ns = unsafe {
            reader
                .GetPresentationAttribute(MF_SOURCE_READER_MEDIASOURCE.0 as u32, &MF_PD_DURATION)
                .ok()
                .and_then(|pv| i64::try_from(&pv).ok())
                .unwrap_or(0)
        };
        let mut v = Video {
            reader,
            w,
            h,
            stride,
            frame_dt,
            acc: 0.0,
            pos_100ns: 0,
            duration_100ns,
            rgba: vec![255u8; w * h * 4],
            dirty: false,
            ended: false,
            pending_trim: None,
        };
        v.read_frame();

        // 일부 인코더/편집툴이 파일 맨 끝에 몇 초짜리 검은/정지 화면을 덧붙여두는
        // 경우가 있다 — 컨테이너의 duration 메타데이터는 그 꼬리까지 포함해서
        // "정확하게" 재는데, 그러면 실제 내용보다 몇 초 더 길게 재생되는 것처럼
        // 느껴진다. 근데 그 지점을 찾으려면 마지막 몇 초 분량을 미리 디코딩해봐야
        // 해서, 여기서 동기로 하면 파일 열 때마다 몇 초씩 멎어 보인다. 그래서 재생은
        // 곧바로 시작하고, 스캔은 별도 스레드+별도 리더로 백그라운드에서 돌려서
        // 결과가 도착하면 그때 duration 을 당긴다(advance 에서 매 프레임 확인).
        if duration_100ns > 0 {
            let (tx, rx) = channel();
            let path = path.to_string();
            std::thread::spawn(move || {
                let trimmed = unsafe { scan_content_end(&path, duration_100ns) };
                let _ = tx.send(trimmed);
            });
            v.pending_trim = Some(rx);
        }
        Some(v)
    }

    // 마지막 5초 구간을 스캔해서, 검은 화면이 아닌 마지막 프레임의 타임스탬프를
    // 찾는다. 꼬리 전체가 콘텐츠라 잘라낼 게 없으면 None.
    fn detect_content_end(&mut self) -> Option<i64> {
        if self.duration_100ns <= 0 {
            return None;
        }
        const SCAN_WINDOW_100NS: i64 = 50_000_000; // 5초
        let scan_start = (self.duration_100ns - SCAN_WINDOW_100NS).max(0);
        self.seek_to(scan_start);
        self.ended = false;
        let mut last_content_ts: Option<i64> = None;
        while self.read_frame() {
            if !self.frame_is_black() {
                last_content_ts = Some(self.pos_100ns);
            }
        }
        match last_content_ts {
            // 스캔 구간 안에 검은 화면이 아닌 프레임이 하나도 없으면(=전부 콘텐츠였거나
            // 스캔 자체가 실패) 잘라내지 않는다 — 오탐으로 멀쩡한 엔딩을 잘라먹는
            // 것보단 안전하게 아무것도 안 하는 쪽이 낫다.
            Some(ts) if ts < self.duration_100ns - 5_000_000 => Some(ts), // 최소 0.5초 이상 검은 꼬리가 있을 때만 자른다
            _ => None,
        }
    }

    // 샘플링한 픽셀들의 평균 밝기가 아주 낮으면 검은 화면으로 본다.
    fn frame_is_black(&self) -> bool {
        let total = self.w * self.h;
        if total == 0 {
            return true;
        }
        let step = (total / 2000).max(1); // 최대 ~2000픽셀만 샘플링해 가볍게
        let mut sum: u64 = 0;
        let mut count: u64 = 0;
        for i in (0..total).step_by(step) {
            let o = i * 4;
            sum += self.rgba[o] as u64 + self.rgba[o + 1] as u64 + self.rgba[o + 2] as u64;
            count += 1;
        }
        (sum as f32 / (count.max(1) as f32 * 3.0)) < 10.0
    }

    // 다음 프레임을 읽어 RGBA 로 변환. 성공하면 true. 끝에 도달하면 더 진행하지 않고
    // ended 를 세워서 멈춘다.
    fn read_frame(&mut self) -> bool {
        if self.ended {
            return false;
        }
        unsafe {
            let mut flags = 0u32;
            let mut ts = 0i64;
            let mut sample: Option<IMFSample> = None;
            if self
                .reader
                .ReadSample(VIDEO_STREAM, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample))
                .is_err()
            {
                return false;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                self.ended = true;
                return false;
            }
            // 꼬리의 검은 화면 구간을 잘라낸 경우, 거기부터는 디코딩만 되고 화면엔
            // 안 보여준 채 곧바로 끝난 것으로 처리한다 (detect_content_end 참고).
            if self.duration_100ns > 0 && ts >= self.duration_100ns {
                self.ended = true;
                return false;
            }
            let Some(sample) = sample else { return false };
            let Ok(buffer) = sample.ConvertToContiguousBuffer() else { return false };
            let mut ptr: *mut u8 = std::ptr::null_mut();
            let mut len = 0u32;
            if buffer.Lock(&mut ptr, None, Some(&mut len)).is_err() {
                return false;
            }
            self.copy_rgb32(ptr, len as usize);
            let _ = buffer.Unlock();
            // 컨테이너 헤더의 명목상 프레임레이트(frame_dt 초기값)는 반올림이나 VFR
            // 때문에 실제 프레임 간격과 미세하게 다를 수 있다 — 그 오차가 수천
            // 프레임에 걸쳐 누적되면 영상이 실제 길이보다 몇 초 더 길게(또는 짧게)
            // 재생되는 드리프트가 생긴다. 매 프레임마다 실제 타임스탬프 간격으로
            // frame_dt 를 다시 맞춰서 진짜 재생 속도를 계속 따라가게 한다.
            let step = ((ts - self.pos_100ns).max(0) as f64 / 10_000_000.0) as f32;
            if step > 0.001 && step < 1.0 {
                self.frame_dt = step;
            }
            self.pos_100ns = ts;
            self.dirty = true;
            true
        }
    }

    // RGB32(B,G,R,X) → RGBA. stride 부호로 상하 뒤집힘 처리.
    // 청크 단위(iterator)로 처리해 opt-level=0 에서도 배열 경계검사 비용을 줄인다.
    unsafe fn copy_rgb32(&mut self, ptr: *const u8, len: usize) {
        let (w, h) = (self.w, self.h);
        let abs_stride = self.stride.unsigned_abs() as usize;
        let bottom_up = self.stride < 0;
        let src = unsafe { std::slice::from_raw_parts(ptr, len) };
        for y in 0..h {
            let sy = if bottom_up { h - 1 - y } else { y };
            let row = sy * abs_stride;
            if row + w * 4 > len {
                break;
            }
            let srow = &src[row..row + w * 4];
            let drow = &mut self.rgba[y * w * 4..(y + 1) * w * 4];
            for (s, d) in srow.chunks_exact(4).zip(drow.chunks_exact_mut(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
        }
    }

    pub fn advance(&mut self, dt: f32, playing: bool) {
        // 백그라운드 꼬리-스캔 결과가 도착했으면 반영한다 (재생 중이 아니어도
        // 확인은 계속해야 진행바 총시간이 늦게라도 갱신된다).
        if let Some(rx) = &self.pending_trim
            && let Ok(result) = rx.try_recv()
        {
            if let Some(trimmed) = result {
                self.duration_100ns = trimmed;
            }
            self.pending_trim = None;
        }
        if !playing {
            return;
        }
        self.acc += dt;
        let mut steps = 0;
        while self.acc >= self.frame_dt && steps < 3 {
            self.acc -= self.frame_dt;
            steps += 1;
            self.read_frame();
        }
        // 디코딩이 실시간을 못 따라가서 한 번에 3프레임 넘게 밀리면 못 따라잡은
        // 나머지가 acc 에 계속 남아 다음 프레임들로 누적된다 — 이게 쌓이면 실제
        // 영상 길이보다 벽시계 기준 재생 시간이 점점 더 길어지는 드리프트가 된다
        // (체감상 "영상이 실제보다 몇 초 더 오래 재생됨"). 한 프레임 분량 넘게
        // 밀린 건 그냥 버려서 이 드리프트가 쌓이지 않게 한다.
        if self.acc > self.frame_dt {
            self.acc = self.frame_dt;
        }
    }

    // 앞/뒤로 델타(초)만큼 이동. 음수면 되감기.
    pub fn seek(&mut self, delta_secs: f32) {
        let delta_100ns = (delta_secs as f64 * 10_000_000.0) as i64;
        self.seek_to(self.pos_100ns + delta_100ns);
    }

    // 절대 위치(100ns 단위)로 이동. 진행바 클릭/드래그, 그리고 끝난 뒤 다시재생에 사용.
    pub fn seek_to(&mut self, pos_100ns: i64) {
        let max = if self.duration_100ns > 0 { self.duration_100ns } else { i64::MAX };
        let new_pos = pos_100ns.clamp(0, max);
        unsafe {
            if self.reader.SetCurrentPosition(&GUID::zeroed(), &PROPVARIANT::from(new_pos)).is_ok() {
                self.ended = false; // 어디로든 seek 하면 "끝남" 상태는 풀린다
                // 프레임 페이싱 누적값도 같이 리셋한다 — 안 그러면 시크 직전에 쌓여있던
                // acc 가 남아서, 재생 중이었다면 시크 직후 다음 advance() 에서 원치 않는
                // 프레임을 하나 더 건너뛰거나 타이밍이 밀린 것처럼 느껴질 수 있다.
                self.acc = 0.0;
                // 우선 요청한 위치로 맞춰둔다 — 그래야 아래 디코딩이 다 실패해도(파일
                // 끝 근처 등) 진행바/시간 표시가 최소한 사용자가 요청한 위치는 보여준다.
                self.pos_100ns = new_pos;

                // 시크 직후 첫 ReadSample 은 컨테이너/디코더에 따라 시크 이전에 이미
                // 파이프라인에 남아있던 "찌꺼기" 프레임을 한 번 그대로 돌려주기도 한다
                // — 그 타임스탬프가 요청한 위치보다 눈에 띄게(0.3초 이상) 이르면 한 번
                // 더 읽어서 건너뛴다(최대 3번까지만, 무한루프 방지). 이게 없으면 반복
                // 탐색/드래그 스크럽 중에 화면이 실제 요청 위치보다 앞선 오래된
                // 프레임에 멈춰있는 것처럼 보이고, 마우스 위치와 표시 위치가 어긋난다.
                for _ in 0..3 {
                    if !self.read_frame() {
                        break;
                    }
                    if self.pos_100ns + 3_000_000 >= new_pos {
                        break; // 0.3초 이내로 따라잡았으면 충분히 정확하다고 본다
                    }
                }
            }
        }
    }

    pub fn position_100ns(&self) -> i64 {
        self.pos_100ns
    }

    pub fn duration_100ns(&self) -> i64 {
        self.duration_100ns
    }
}

// mp4 오디오 트랙을 별도 스레드에서 MF 로 PCM 디코딩해 WASAPI 공유 모드로 재생한다.
// 비디오 디코딩과는 독립적으로 돌기 때문에 완벽히 동기화되진 않지만, 서로 다른
// 스레드에서 각자의 실시간(디스플레이 refresh / 오디오 버퍼)에 맞춰 진행되므로
// 이 정도 규모의 프로젝트에는 충분하다.

enum AudioMsg {
    Seek(i64), // 100ns 단위 절대 위치
}

pub struct Audio {
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,     // f32 비트패턴 (0.0..=1.0) — 설정창의 mp4 사운드 슬라이더와 연결
    weathering: Arc<AtomicU32>, // f32 비트패턴 (0.0..=1.0) — 설정창의 Weathering 슬라이더와 연결
    tx: Sender<AudioMsg>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Audio {
    // 오디오 트랙이 없거나 장치를 열 수 없으면 조용히 None (영상은 무음으로 재생됨).
    pub fn start(path: String, volume: f32, weathering: f32) -> Option<Audio> {
        let playing = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU32::new(volume.clamp(0.0, 1.0).to_bits()));
        let weathering = Arc::new(AtomicU32::new(weathering.clamp(0.0, 1.0).to_bits()));
        let (tx, rx) = channel::<AudioMsg>();
        let playing2 = playing.clone();
        let stop2 = stop.clone();
        let volume2 = volume.clone();
        let weathering2 = weathering.clone();
        let handle = std::thread::spawn(move || {
            if let Err(e) = unsafe { audio_thread(&path, &playing2, &stop2, &volume2, &weathering2, rx) } {
                eprintln!("[audio] stopped: {e:?}");
            }
        });
        Some(Audio { playing, volume, weathering, tx, stop, handle: Some(handle) })
    }

    pub fn set_playing(&self, p: bool) {
        self.playing.store(p, Ordering::Relaxed);
    }

    pub fn set_volume(&self, v: f32) {
        self.volume.store(v.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    // 0=원본 그대로, 1=많이 낡은 소리(먹먹함+히스+지직거리는 크래클).
    pub fn set_weathering(&self, w: f32) {
        self.weathering.store(w.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn seek_to(&self, pos_100ns: i64) {
        let _ = self.tx.send(AudioMsg::Seek(pos_100ns));
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

// "낡은 소리" 흉내 — 톤 자체가 달라지는 효과들을 주로 하고, 노이즈(히스/크래클)는
// 양념 정도로만 얹는다. 노이즈만 세게 넣으면 그냥 "지지직거리는 소리"로만 들리고
// 진짜 "낡았다"는 느낌은 잘 안 나서, 아래 순서로 효과 비중을 뒀다:
//   1) 와우/플러터 — 딜레이 라인을 느린 LFO 로 흔들어 읽어서(=가변 지연) 오래된
//      테이프 특유의 피치가 출렁이는 느낌을 낸다. 필터링만으론 못 내는 "결이
//      다른" 열화라 이게 실제로 제일 "낡았다"고 느끼게 만드는 핵심이다.
//   2) 새추레이션(부드러운 클리핑) — 진폭이 큰 부분을 tanh 로 뭉개서 배음이
//      추가된 따뜻하고 거친 질감을 낸다. 이것도 노이즈가 아니라 파형 자체의 변형.
//   3) 로우패스(고음 깎기) + 하이패스(저음도 같이 깎아 작은 스피커/라디오 톤)
//   4) 비트 감소(양자화) — 진폭을 계단식으로 뭉개서 해상도 자체를 낮춘다.
//   5) 히스(백색소음) — 마지막에 살짝만.
//   6) 가끔 튀는 크래클 — 단발 스파이크가 아니라 몇 샘플에 걸쳐 감쇠하는 "팝"
//      포락선으로 만들어서 히스에 묻히지 않고 또렷한 딸깍임으로 들리게 한다.
// amount(0..1) 가 클수록 다 세지고, 0 이면 process() 호출 자체를 건너뛰어 원본
// 그대로 나간다(audio_thread 의 `if amount > 0.0` 가드). 필터/딜레이 상태를 오디오
// 스레드 수명 내내 들고 다녀야 버퍼 경계에서 끊기지 않아서, 구조체로 만들어 스레드
// 지역변수로 한 번만 만든다.
const WEATHER_DELAY_LEN: usize = 256;

struct Weathering {
    lp_state: Vec<f32>,      // 채널별 로우패스 직전 출력값
    hp_in_prev: Vec<f32>,    // 채널별 하이패스 직전 입력값
    hp_out_prev: Vec<f32>,   // 채널별 하이패스 직전 출력값
    delay: Vec<[f32; WEATHER_DELAY_LEN]>, // 채널별 와우/플러터용 링버퍼
    delay_pos: usize,
    lfo_phase: f32,
    pop_env: f32, // 진행 중인 크래클 팝의 감쇠 포락선(0 이면 없음)
    sample_rate: f32,
    rng: u32, // xorshift32 시드 — 매 샘플 히스/크래클용 난수
}

impl Weathering {
    fn new(channels: usize, sample_rate: f32) -> Weathering {
        Weathering {
            lp_state: vec![0.0; channels],
            hp_in_prev: vec![0.0; channels],
            hp_out_prev: vec![0.0; channels],
            delay: vec![[0.0; WEATHER_DELAY_LEN]; channels],
            delay_pos: 0,
            lfo_phase: 0.0,
            pop_env: 0.0,
            sample_rate: sample_rate.max(1.0),
            rng: 0x9e3779b9,
        }
    }

    // -1.0..1.0 결정적이지 않은(스레드 수명 동안 계속 진행하는) 난수.
    fn next_rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn process(&mut self, samples: &mut [i16], channels: usize, amount: f32) {
        let amount = amount.clamp(0.0, 1.0);
        let n_ch = channels.max(1);
        // amount 가 클수록 로우패스가 더 이전 값에 붙어있게(=고음을 더 깎게) 계수를 낮춘다.
        let lp_coeff = 1.0 - amount * 0.93;
        // 하이패스 계수 a — 1.0 에 가까울수록 컷오프가 낮고(거의 그대로),
        // amount 가 커질수록 확 낮춰서 저음까지 깎아 "작은 스피커/라디오" 톤을 만든다.
        let hp_a = 0.999 - amount * 0.45;
        // 새추레이션 드라이브 — amount 클수록 더 세게 눌러서 배음을 더한다.
        let drive = 1.0 + amount * 5.0;
        let drive_tanh_max = drive.tanh().max(0.0001);
        let hiss_amp = amount * 350.0; // 예전(900)보다 확 줄여서 톤 변화가 주가 되게 했다
        let crackle_chance = amount * 0.0035; // 샘플당 새 팝이 시작될 확률
        let crackle_amp = amount * 11000.0;
        // 비트 감소: amount=0 → 16비트(사실상 무손실), amount=1 → 약 5비트까지 뭉갠다
        // (예전 7비트보다 더 거칠게 — 톤 저하 자체를 더 또렷하게 들리도록).
        let levels = 2f32.powf(16.0 - amount * 11.0);
        let quant_step = 65536.0 / levels;
        // 와우/플러터: 딜레이 라인 읽는 위치를 느린 사인파(3.3Hz — 낡은 테이프 특유의
        // 속도)로 흔든다. depth 는 amount 에 비례한 최대 흔들림 폭(샘플 단위).
        let lfo_rate = 3.3;
        let depth = amount * 14.0;

        for frame in samples.chunks_mut(n_ch) {
            self.lfo_phase = (self.lfo_phase + lfo_rate / self.sample_rate).fract();
            let lfo = (self.lfo_phase * std::f32::consts::TAU).sin();
            let read_offset = WEATHER_DELAY_LEN as f32 * 0.5 + lfo * depth;

            // 크래클 팝: 진행 중인 팝이 없으면 확률적으로 새로 시작하고, 있으면
            // 몇 샘플에 걸쳐 지수적으로 감쇠시킨다 — 단발 임펄스보다 훨씬 "딸깍"
            // 하는 물리적인 클릭처럼 들린다.
            if self.pop_env > 0.02 {
                self.pop_env *= 0.72;
            } else if self.next_rand().abs() < crackle_chance {
                self.pop_env = 1.0;
            } else {
                self.pop_env = 0.0;
            }
            let pop = if self.pop_env > 0.0 { self.next_rand() * crackle_amp * self.pop_env } else { 0.0 };

            for (ch, s) in frame.iter_mut().enumerate() {
                let ch = ch.min(self.lp_state.len().saturating_sub(1));
                let x = *s as f32;

                // 딜레이 라인에 현재 샘플을 쓰고, LFO 로 흔들리는 위치에서 보간해서
                // 읽는다 — 이게 와우/플러터(피치 출렁임)의 핵심이다.
                let buf = &mut self.delay[ch];
                buf[self.delay_pos] = x;
                let read_pos =
                    (self.delay_pos as f32 - read_offset + WEATHER_DELAY_LEN as f32 * 2.0) % WEATHER_DELAY_LEN as f32;
                let i0 = read_pos as usize % WEATHER_DELAY_LEN;
                let i1 = (i0 + 1) % WEATHER_DELAY_LEN;
                let t = read_pos.fract();
                let wobbled = buf[i0] * (1.0 - t) + buf[i1] * t;

                // 새추레이션 — 정규화해서 tanh 로 부드럽게 누른 뒤 다시 스케일. 드라이브가
                // 클수록 배음이 늘어 따뜻하고 거친 질감이 생긴다(노이즈가 아니라 파형
                // 자체의 왜곡이라 히스/크래클과는 결이 다르게 들린다).
                let norm = wobbled / 32768.0;
                let saturated = (norm * drive).tanh() / drive_tanh_max * 32768.0;
                let driven = wobbled * (1.0 - amount * 0.5) + saturated * (amount * 0.5);

                // 로우패스.
                let lp_prev = self.lp_state[ch];
                let lp = lp_prev + (driven - lp_prev) * lp_coeff;
                self.lp_state[ch] = lp;

                // 하이패스(1차 IIR) — 로우패스 결과에 얹어서 살짝만 섞는다(다 섞으면
                // 저음이 통째로 사라져서 너무 인위적으로 들린다).
                let hp = hp_a * (self.hp_out_prev[ch] + lp - self.hp_in_prev[ch]);
                self.hp_in_prev[ch] = lp;
                self.hp_out_prev[ch] = hp;
                let filtered = lp * (1.0 - amount * 0.35) + hp * (amount * 0.35);

                let mut v = filtered + self.next_rand() * hiss_amp + pop;
                v = (v / quant_step).round() * quant_step; // 비트 감소

                *s = v.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
            }
            self.delay_pos = (self.delay_pos + 1) % WEATHER_DELAY_LEN;
        }
    }
}

const AUDIO_STREAM: u32 = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;

unsafe fn audio_thread(
    path: &str,
    playing: &AtomicBool,
    stop: &AtomicBool,
    volume: &AtomicU32,
    weathering: &AtomicU32,
    rx: Receiver<AudioMsg>,
) -> windows::core::Result<()> {
    unsafe {
        // COM MTA 참여는 스레드마다 각자 해줘야 한다 (호출한 스레드에만 적용됨).
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _ = MFStartup(MF_VERSION, MFSTARTUP_FULL);

        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);
        let reader: IMFSourceReader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), None)?;
        reader.SetStreamSelection(ALL_STREAMS, false)?;
        reader.SetStreamSelection(AUDIO_STREAM, true)?;

        // 압축 해제된 PCM 으로 출력하도록 요청 (세부 포맷은 디코더가 채운다).
        let pcm_type: IMFMediaType = MFCreateMediaType()?;
        pcm_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        pcm_type.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM)?;
        reader.SetCurrentMediaType(AUDIO_STREAM, None, &pcm_type)?;
        let cur_type: IMFMediaType = reader.GetCurrentMediaType(AUDIO_STREAM)?;

        let mut wfx_ptr: *mut WAVEFORMATEX = std::ptr::null_mut();
        MFCreateWaveFormatExFromMFMediaType(&cur_type, &mut wfx_ptr, None, 0)?;
        let block_align = (*wfx_ptr).nBlockAlign as usize;
        // Weathering(풍화) 효과는 16비트 PCM 샘플을 직접 만져야 해서, 디코더가 그 외의
        // 비트뎁스로 뱉으면(흔치 않지만) 조용히 건너뛴다 — 잘못 해석해서 노이즈를
        // 만들어내는 것보단 그냥 효과 없이 원본 그대로 재생하는 게 낫다.
        let weather_channels = if (*wfx_ptr).wBitsPerSample == 16 { (*wfx_ptr).nChannels as usize } else { 0 };
        let mut weather = Weathering::new(weather_channels.max(1), (*wfx_ptr).nSamplesPerSec as f32);

        // WASAPI 공유 모드 렌더 클라이언트. AUTOCONVERTPCM 으로 장치 고유 포맷과 다르면
        // 오디오 엔진이 알아서 리샘플/채널변환 해준다 (직접 리샘플러를 안 만들어도 됨).
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
        let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
        client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 5_000_000, 0, wfx_ptr, None)?;
        let buffer_frames = client.GetBufferSize()?;
        let render: IAudioRenderClient = client.GetService()?;
        // 세션 볼륨 — PCM 을 직접 스케일링하지 않고 WASAPI 가 알아서 감쇠해준다.
        let simple_volume: ISimpleAudioVolume = client.GetService()?;
        let mut last_volume = f32::from_bits(volume.load(Ordering::Relaxed)) - 1.0; // 첫 루프에 무조건 반영되도록

        let mut started = false;
        let mut pcm_queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
        let mut ended = false;

        while !stop.load(Ordering::Relaxed) {
            let cur_volume = f32::from_bits(volume.load(Ordering::Relaxed));
            if cur_volume != last_volume {
                let _ = simple_volume.SetMasterVolume(cur_volume, &GUID::zeroed());
                last_volume = cur_volume;
            }

            while let Ok(AudioMsg::Seek(pos)) = rx.try_recv() {
                let _ = reader.SetCurrentPosition(&GUID::zeroed(), &PROPVARIANT::from(pos));
                pcm_queue.clear();
                ended = false;
            }

            if !playing.load(Ordering::Relaxed) {
                if started {
                    let _ = client.Stop();
                    started = false;
                }
                std::thread::sleep(Duration::from_millis(15));
                continue;
            }

            // 재생 시작/재개 전에 버퍼를 먼저 채운다. WASAPI 는 Start() 를 부르는 순간부터
            // 버퍼를 소비하기 시작하므로, 채우기 전에 Start() 하면 그만큼 무음 구간이
            // 생겨 "사운드가 살짝 늦게 나오는" 현상의 원인이 된다.
            let padding = client.GetCurrentPadding().unwrap_or(0);
            let avail = buffer_frames.saturating_sub(padding);
            if avail > 0 {
                let need = avail as usize * block_align;
                while pcm_queue.len() < need && !ended {
                    let mut flags2 = 0u32;
                    let mut sample: Option<IMFSample> = None;
                    if reader.ReadSample(AUDIO_STREAM, 0, None, Some(&mut flags2), None, Some(&mut sample)).is_err() {
                        ended = true;
                        break;
                    }
                    if flags2 & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                        ended = true;
                        break;
                    }
                    let Some(sample) = sample else {
                        ended = true;
                        break;
                    };
                    if let Ok(buffer) = sample.ConvertToContiguousBuffer() {
                        let mut ptr: *mut u8 = std::ptr::null_mut();
                        let mut len = 0u32;
                        if buffer.Lock(&mut ptr, None, Some(&mut len)).is_ok() {
                            pcm_queue.extend(std::slice::from_raw_parts(ptr, len as usize));
                            let _ = buffer.Unlock();
                        }
                    }
                }
                if ended && pcm_queue.is_empty() {
                    // 끝까지 다 읽음 — 영상 쪽처럼 자동으로 되감지 않고 그냥 멈춘다.
                    // VideoApp 이 곧 set_playing(false) 를 불러 이 스레드도 멈춰줄 것이다.
                } else if let Ok(ptr) = render.GetBuffer(avail) {
                    if !ptr.is_null() {
                        let dst = std::slice::from_raw_parts_mut(ptr, need);
                        let n = need.min(pcm_queue.len());
                        for slot in dst.iter_mut().take(n) {
                            *slot = pcm_queue.pop_front().unwrap();
                        }
                        for slot in dst.iter_mut().skip(n) {
                            *slot = 0; // 디코드가 못 따라오면 무음으로 채움
                        }
                        if weather_channels > 0 {
                            let amount = f32::from_bits(weathering.load(Ordering::Relaxed));
                            if amount > 0.0 {
                                // dst 는 16비트 PCM 인터리브 바이트 배열 — 정렬이 안 맞을 수
                                // 있는 raw 버퍼라 align_to_mut 대신 바이트 쌍을 직접 조립한다.
                                let samples = dst.len() / 2;
                                let mut buf: Vec<i16> = (0..samples)
                                    .map(|i| i16::from_le_bytes([dst[i * 2], dst[i * 2 + 1]]))
                                    .collect();
                                weather.process(&mut buf, weather_channels, amount);
                                for (i, s) in buf.into_iter().enumerate() {
                                    let b = s.to_le_bytes();
                                    dst[i * 2] = b[0];
                                    dst[i * 2 + 1] = b[1];
                                }
                            }
                        }
                    }
                    let _ = render.ReleaseBuffer(avail, 0);
                }
            }

            if !started {
                let _ = client.Start();
                started = true;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        let _ = client.Stop();
        Ok(())
    }
}

