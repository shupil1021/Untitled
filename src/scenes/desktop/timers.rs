//! 시간에 따라 일어나는 일들 — 와이파이 상태 갱신, 첫 메일 도착, 시드 심은 뒤 힌트 메일, 서브 게임 다운로드 알림,
//! 우측 하단 알림(토스트)의 수명, 그리고 OS 쪽 상태를 게임 창들에 알리는 신호(signals.rs).

use super::*;

impl DesktopScene {
    // 매 프레임 한 번 — 경과 시간(f.dt)만큼 각 타이머를 센다.
    pub(super) fn update_timers(&mut self, f: &mut Frame) {
        // 와이파이 상태는 매 프레임 물어보기엔 아까우니 몇 초마다 한 번만 갱신한다.
        const WIFI_CHECK_INTERVAL: f32 = 3.0;
        self.wifi_check_timer += f.dt;
        if self.wifi_check_timer >= WIFI_CHECK_INTERVAL {
            self.wifi_check_timer = 0.0;
            self.wifi_connected = network_connected();
        }

        // 게임 다운로드 파일이 첨부된 메일 도착 — 이미 도착했으면(불러온 저장에서도
        // true 로 남아있다) 더는 재지 않는다.
        if !self.fs.has_mail(MailId::Friend) {
            self.mail_timer += f.dt;
            if self.mail_timer >= MAIL_ARRIVAL_DELAY {
                self.deliver_mail(MailId::Friend, &f.settings);
            }
        }
        // 문(도어즈)의 편지 메일을 읽었는지 매 프레임 최신 값을 게임에 알려준다 — 문 게임이 일시
        // 정지에서 풀리는 조건이다(signals.rs).
        crate::signals::set_letter_mail_read(self.fs.is_mail_read(MailId::Letter));
        for kind in crate::foundation::MiniKind::ALL {
            crate::signals::set_reward_downloaded(kind.index(), self.fs.reward_downloaded(kind));
        }
        crate::signals::set_lake_delivered(self.fs.minis[crate::foundation::MiniKind::Finger.index()].reward_sent);
        crate::signals::set_flower_downloaded(self.fs.flower_image.is_some_and(|id| self.fs.ever_downloaded.contains(&id)));
        // 씨앗을 심은 뒤 잠깐 있다가 시간 힌트 메일(시트 SUB A+a-15 의 메일 가이딩)이 온다.
        if self.fs.seed_planted && !self.fs.has_mail(MailId::Hint) {
            self.hint_timer += f.dt;
            if self.hint_timer >= HINT_MAIL_DELAY {
                self.deliver_mail(MailId::Hint, &f.settings);
            }
        }
        // 두 번째 메일이 온 뒤 잠깐 있다가 "다운로드 완료" 알림과 함께 서브 게임 아이콘이 생긴다
        // (시트의 CRT B-0). 저장된 상태(꽃 위치 메일만 온 채 종료)도 이어서 센다.
        if self.fs.has_mail(MailId::Flower) && !self.fs.sub_game_ready {
            self.sub_game_timer += f.dt;
            if self.sub_game_timer >= SUB_GAME_DELAY {
                self.fs.sub_game_ready = true;
                self.add_desktop_icon(SUB_GAME_NAME, FileKind::SubGame);
                self.toast = Some(Toast { title: s::DOWNLOAD_COMPLETE, line1: SUB_GAME_NAME.to_string(), line2: String::new(), opens_mail: false });
                self.toast_timer = TOAST_DURATION;
                self.write_save(&f.settings);
            }
        }
        if self.toast_timer > 0.0 {
            self.toast_timer -= f.dt;
            if self.toast_timer <= 0.0 {
                self.toast = None;
            }
        }
    }
}
