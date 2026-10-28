//! 창이 요청한 동작(DeskAction) 처리와 관련 도우미 — 열려 있는 탐색기/메일/휴지통 창 새로고침, 메일 도착, 저장,
//! 파일 열기(폴더는 탐색기 안에서 드릴다운).

use super::*;

impl DesktopScene {
    // File Explorer 가 지금 열려있으면 내용을 새로고침한다 — 다운로드/잠금해제처럼
    // fs 내용이 바뀌었는데 이미 열려있는 창은 스냅샷이 그대로라 반영 안 되는 경우에
    // 쓴다. 3초마다 도는 주기 새로고침에서도 그대로 쓰이므로, 지금 보고 있던 탭/
    // 하위 폴더 위치를 먼저 downcast 로 읽어뒀다가 새로고침 뒤에도 그 자리 그대로
    // 있도록 explorer_app_refreshed 로 되돌려준다 — 안 그러면 항상 첫 탭(Downloads)
    // 으로 튕겨서, 다른 탭을 보던 중에 새로고침이 일어날 때마다 화면이 튀어보인다.
    pub(super) fn refresh_explorer_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME)
            && self.wm.is_open(explorer_id)
        {
            let loc = self
                .wm
                .app_mut(explorer_id)
                .and_then(|app| app.as_any_mut().downcast_mut::<ExplorerApp>())
                .and_then(|app| app.current_location());
            let app = explorer_app_refreshed(&self.fs, explorer_id, loc, settings);
            self.wm.refresh_app(explorer_id, app);
        }
    }

    // 휴지통이 지금 열려있으면(별개의 RecycleBinApp 창) 내용을 새로고침한다 — Empty
    // Recycle Bin 을 눌러 fs 가 바뀐 뒤에도 그 창은 여는 시점의 스냅샷 그대로라
    // 반영이 안 되는 걸 막는다. RecycleBinApp 은 ExplorerApp 과 달리 탭/위치 상태가
    // 없어서(그냥 목록 하나) open() 으로 통째로 다시 만들면 그만이다.
    pub(super) fn refresh_recycle_bin_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(bin_id) = self.fs.find_by_name(RECYCLE_BIN_NAME)
            && self.wm.is_open(bin_id)
        {
            let op = open(&self.fs, bin_id, settings);
            self.wm.refresh_app(bin_id, op.app);
        }
    }

    // Mail 의 "Write Mail" 첨부 목록(attachable)이 지금 열려있는 창에도 최신
    // 상태로 반영되도록 — 다운로드/이동/삭제로 fs.desktop/downloads 가 바뀌었는데
    // Mail 을 이미 열어둔 채였다면 창을 닫았다 다시 열기 전까진 목록에 새 파일이
    // 안 보이던 문제 — 통째로 새 MailApp 을 만드는 대신(그러면 "Write Mail" 에
    // 작성 중이던 초안이 통째로 날아간다) attachable 목록만 그 자리에서 바꿔치기한다.
    pub(super) fn refresh_mail_attachable_if_open(&mut self) {
        if let Some(mail_id) = self.fs.find_by_name("Mail")
            && self.wm.is_open(mail_id)
            && let Some(app) = self.wm.app_mut(mail_id).and_then(|app| app.as_any_mut().downcast_mut::<MailApp>())
        {
            app.refresh_attachable(mail_attachable_files(&self.fs));
        }
    }

    // 메일이 새로 도착했을 때(fs.mail_arrived 가 막 true 로 바뀐 시점) 지금 Mail
    // 창이 열려있으면 통째로 새로 만들어서 반영한다 — seed_messages 결과 자체가
    // 바뀌므로 refresh_mail_attachable_if_open 처럼 목록 하나만 바꿔치기해선
    // 안 된다. 대신 보고 있던 폴더/선택은 downcast 로 먼저 읽어뒀다가 그대로
    // 되돌려준다("Write Mail" 초안은 새 MailApp 을 만드는 순간 사라지지만, 메일
    // 도착은 세션에 한 번뿐이라 그 시점에 마침 초안을 쓰고 있었을 확률은 낮고,
    // 이 파이프라인 1단계에서는 거기까진 다루지 않는다).
    // 메일을 받은편지함에 도착시키고(이미 왔으면 아무것도 안 함) 열려있는 Mail 을 새로고침한 뒤
    // 우측 하단 토스트(보낸 사람 + 제목)로 알리고 저장한다.
    pub(super) fn deliver_mail(&mut self, id: MailId, settings: &Rc<RefCell<Settings>>) {
        if !self.fs.deliver_mail(id) {
            return;
        }
        self.refresh_mail_if_open(settings);
        let lang = settings.borrow().language;
        self.toast = Some(Toast::new_mail(mail_from(id, lang), mail_subject(id, lang)));
        self.toast_timer = TOAST_DURATION;
        self.write_save(settings);
    }

    pub(super) fn refresh_mail_if_open(&mut self, settings: &Rc<RefCell<Settings>>) {
        if let Some(mail_id) = self.fs.find_by_name("Mail")
            && self.wm.is_open(mail_id)
        {
            let (folder_idx, selected) = self
                .wm
                .app_mut(mail_id)
                .and_then(|app| app.as_any_mut().downcast_mut::<MailApp>())
                .map(|app| (app.folder_idx(), app.selected()))
                .unwrap_or((None, None));
            let op = open(&self.fs, mail_id, settings);
            self.wm.refresh_app(mail_id, op.app);
            if let Some(app) = self.wm.app_mut(mail_id).and_then(|app| app.as_any_mut().downcast_mut::<MailApp>()) {
                app.set_folder_idx(folder_idx);
                app.set_selected(selected);
            }
        }
    }

    // 현재 설정/바탕화면 상태를 그대로 파일에 저장한다 — fs 전체를 그대로 스냅샷하므로
    // 이름 목록을 따로 뽑아 모을 필요가 없다.
    pub(super) fn write_save(&self, settings: &Rc<RefCell<Settings>>) {
        let window_geometry =
            self.window_geometry.iter().map(|(&fid, &(rect, maximized))| (fid, rect.x, rect.y, rect.w, rect.h, maximized)).collect();
        crate::foundation::save(&crate::foundation::SaveData {
            settings: settings.borrow().clone(),
            fs: self.fs.clone(),
            icon_pos: self.icon_pos.clone(),
            window_geometry,
        });
    }

    // 새로 연 창(open() 이 true 를 돌려줬을 때만 — 이미 열려있어서 앞으로만 가져온
    // 경우는 지금 자리를 그대로 둬야 한다)에 기억해둔 마지막 크기/위치가 있으면 적용한다.
    pub(super) fn apply_saved_geometry(&mut self, file: FileId, work: Rect) {
        if let Some(&(rect, maximized)) = self.window_geometry.get(&file) {
            self.wm.set_geometry(file, rect, maximized, work);
        }
    }

    pub(super) fn open_file(&mut self, idx: usize, settings: &Rc<RefCell<Settings>>, work: Rect) {
        let fid = self.fs.desktop[idx];
        if self.is_drilldown_folder(fid) {
            self.open_folder_in_explorer(fid, settings, work);
            return;
        }
        let op = open(&self.fs, fid, settings);
        if self.wm.open(op, Some(fid), work) {
            self.apply_saved_geometry(fid, work);
        }
    }

    // 일반 폴더는 항상 My Computer 창 안에서 드릴다운 탭으로 보여준다. 휴지통(이름이
    // 정확히 "Recycle Bin"인 폴더)만 예외로 별개의 독립 창(RecycleBinApp)에 연다 —
    // apps/mod.rs::open() 이 그 이름을 보고 ExplorerApp 대신 RecycleBinApp 을 골라준다.
    pub(super) fn is_drilldown_folder(&self, id: FileId) -> bool {
        let node = self.fs.get(id);
        matches!(node.kind, FileKind::Folder { .. }) && node.name != RECYCLE_BIN_NAME
    }

    // 폴더를(바탕화면 아이콘이든 다른 경로로든) 열 때 항상 이걸 거친다 — My Computer
    // 가 열려있으면 그 창 "안에서" 드릴다운 탭으로 보여주고, 안 열려있으면 My Computer
    // 자체를 이 폴더가 활성 탭인 상태로 새로 연다.
    pub(super) fn open_folder_in_explorer(&mut self, folder_id: FileId, settings: &Rc<RefCell<Settings>>, work: Rect) {
        let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME) else { return };
        let app = explorer_app_for_folder(&self.fs, explorer_id, folder_id, settings);
        if self.wm.is_open(explorer_id) {
            self.wm.refresh_app(explorer_id, app);
            self.wm.focus(explorer_id);
        } else {
            let lang = settings.borrow().language;
            let op = Opened {
                app,
                title: display_name(lang, &self.fs.get(explorer_id).name).into_owned(),
                size: (500.0, 340.0),
                maximized: false,
                resizable: true,
                maximizable: true,
                movable: true,
                min_size: (340.0, 260.0),
            };
            if self.wm.open(op, Some(explorer_id), work) {
                self.apply_saved_geometry(explorer_id, work);
            }
        }
    }

    // 창/앱이 요청한 동작 하나를 처리한다(창 관리자가 한 프레임에 모아 돌려준 목록의 항목) — 파일 열기/다운로드/삭제,
    // 게임 진행 저장, 메일 도착 같은 것들. 상태가 바뀌는 동작은 그 즉시 저장한다.
    pub(super) fn apply_desk_action(&mut self, a: DeskAction, settings: &Rc<RefCell<Settings>>, work: Rect, m: (f32, f32)) {
        match a {
            DeskAction::Unlock(id) => {
                self.fs.unlock(id);
                self.secret_unlocked = true;
                // 잠금 풀린 폴더는 별도 창을 새로 안 띄운다 — File Explorer 가
                // 열려있으면 그 창 안에서 바로 (원래 있던 카테고리의 하위 폴더로)
                // 들어가 보여주고, 안 열려있으면 다음에 열 때 자연히 폴더 아이콘으로 보인다.
                if let Some(explorer_id) = self.fs.find_by_name(MY_COMPUTER_NAME)
                    && self.wm.is_open(explorer_id)
                {
                    let app = explorer_app_for_folder(&self.fs, explorer_id, id, settings);
                    self.wm.refresh_app(explorer_id, app);
                }
                // 5초 주기 자동저장을 기다리다 그 사이에 창이 갑자기 닫히면(작업
                // 관리자로 강제 종료 등, ShutdownScene 을 안 거치는 경우) 방금 한
                // 행동이 통째로 날아간다 — 그래서 실제로 상태가 바뀌는 이벤트마다
                // 그 즉시 한 번 더 저장해둔다(항목별 즉시 저장).
                self.write_save(settings);
            }
            DeskAction::Open(id) => {
                // 폴더는(잠금 풀린 폴더 포함) 항상 File Explorer 창 안에서 드릴다운
                // 탭으로 보여준다 — 열려있으면 그 창 안에서, 안 열려있으면 새로 연다.
                // 휴지통만 예외 — 별개의 프로그램(RecycleBinApp)으로 독립된 창에 연다.
                if self.is_drilldown_folder(id) {
                    self.open_folder_in_explorer(id, settings, work);
                } else {
                    let op = open(&self.fs, id, settings);
                    if self.wm.open(op, Some(id), work) {
                        self.apply_saved_geometry(id, work);
                    }
                }
            }
            DeskAction::RequestErase => self.erase_confirm = true,
            // 서브 게임에서 씨앗을 심음 — 힌트 메일은 HINT_MAIL_DELAY 뒤에(update 의 타이머).
            DeskAction::SeedPlanted => {
                self.fs.seed_planted = true;
                self.write_save(settings);
            }
            // 서브 게임에서 꽃을 우체통에 넣음 — 꽃 사진이 첨부된 메일이 도착한다(시트의 SUB A+a-17/18).
            // 힌트 메일보다 먼저 와버려도(타이머가 아직인데 시간을 돌린 경우) 순서가 안 꼬이게 힌트도 같이 도착시킨다.
            DeskAction::FlowerSent => {
                self.fs.ensure_flower_image();
                self.deliver_mail(MailId::Hint, settings);
                self.deliver_mail(MailId::Photo, settings);
            }
            // 게임 안에서 방에 꽃이 없다는 걸 확인함 — 꽃 위치 메일이 도착한다.
            DeskAction::FlowerAbsenceChecked => self.deliver_mail(MailId::Flower, settings),
            // 문 게임에서 편지를 우편함에 넣음 — 같은 내용의 편지 메일이 도착한다(시트 오른쪽 표).
            DeskAction::LetterSent => self.deliver_mail(MailId::Letter, settings),
            DeskAction::DeliverMail(id) => self.deliver_mail(id, settings),
            // 미니게임 메일/첨부 다시 받기 — 사라진 첨부가 있으면 새로 만들고, 열려 있는 메일/탐색기를 갱신한다.
            DeskAction::ResendMini(kind) => {
                let changed = self.fs.resend_mini_mails(kind);
                self.refresh_mail_if_open(settings);
                self.refresh_explorer_if_open(settings);
                if changed {
                    let lang = settings.borrow().language;
                    let id = kind.task_mail();
                    self.toast = Some(Toast::new_mail(mail_from(id, lang), mail_subject(id, lang)));
                    self.toast_timer = TOAST_DURATION;
                }
                self.write_save(settings);
            }
            // 창 안 게임의 진행 상황이 바뀌었다 — fs 에 기록하고 즉시 저장해서 창을 닫았다 다시 열어도 이어진다.
            DeskAction::SaveDoors(p) => {
                self.fs.doors_progress = p;
                self.write_save(settings);
            }
            DeskAction::SaveMaze(p) => {
                self.fs.maze_progress = p;
                self.write_save(settings);
            }
            // 설치 마법사 진행바가 다 찬 순간 한 번 온다 — 설치 완료를 기록하고
            // 바탕화면에 게임 아이콘을 만든다(마법사 창은 Finish 로 사용자가 닫는다).
            DeskAction::InstallComplete(Some(kind)) => {
                // 미니게임 설치 — AppData\<게임 이름>\ 폴더가 생기고 바탕화면에 그 게임 아이콘이 생긴다.
                let i = kind.index();
                if !self.fs.mini_installed[i] {
                    self.fs.mini_installed[i] = true;
                    crate::gamefiles::install_mini(&mut self.fs, kind);
                    self.refresh_explorer_if_open(settings);
                    self.add_desktop_icon(kind.exe_name(), FileKind::MiniGame(kind));
                }
                self.write_save(settings);
            }
            DeskAction::SaveMini(kind, p) => {
                self.fs.minis[kind.index()] = p;
                self.write_save(settings);
            }
            DeskAction::InstallComplete(None) => {
                if !self.fs.game_installed {
                    self.fs.game_installed = true;
                    // 게임 안 File Explorer 의 AppData\test\ 에 게임 파일(소스/이미지/사운드)이
                    // 생긴다(gamefiles.rs) — 열려있는 탐색기가 있으면 바로 보이게 새로고침.
                    crate::gamefiles::install_into(&mut self.fs);
                    self.refresh_explorer_if_open(settings);
                    self.add_desktop_icon(GAME_FILE_NAME, FileKind::Game);
                }
                self.write_save(settings);
            }
            DeskAction::Download(id) => {
                self.fs.download(id);
                // 지금 File Explorer 가 열려있으면 Downloads 탭에 바로 반영되도록
                // 새로고침한다 — 안 그러면 창을 닫았다 다시 열어야만 보인다. Mail 의
                // 파일 선택 목록도 같은 이유로 같이 새로고침한다 — 안 그러면 방금
                // 받은 파일을 그 창에서 곧장 첨부할 수가 없었다(다시 열어야만
                // 보이던 문제).
                self.refresh_explorer_if_open(settings);
                self.refresh_mail_attachable_if_open();
                // 다운로드 직후 그 즉시 저장 — 5초 자동저장을 기다리는 사이 창이
                // 닫히면 방금 다운로드한 기록이 통째로 사라지는 문제가 있었다.
                // (예전엔 게임 설치 파일이면 다운로드 즉시 마법사가 자동으로 열렸는데,
                // 다른 파일들처럼 직접 더블클릭해서 열게 바꿨다.)
                self.write_save(settings);
            }
            DeskAction::DeletePermanently(id) => {
                // 파일을 실제로 지운다 — 어디 있었든(바탕화면/Downloads/폴더 안) 다
                // 사라지므로, 열려있던 File Explorer 가 있으면 바로 반영되도록
                // 새로고침한다. 바탕화면에 있었을 수도 있으니(드래그로 옮겨왔을 경우)
                // icon_pos 와 짝을 맞춰 먼저 떼어낸다.
                if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                    self.fs.desktop.remove(pos);
                    self.icon_pos.remove(pos);
                }
                self.fs.trash_origin.retain(|&(tid, _)| tid != id);
                self.fs.delete_permanently(id);
                self.refresh_explorer_if_open(settings);
                self.refresh_mail_attachable_if_open();
                self.write_save(settings);
            }
            DeskAction::MoveFiles(ids, dest) => {
                // ExplorerApp 은 자기 사이드바만 알아서, 창 밖으로 드롭되면 일단
                // Desktop 이라고 요청한다 — 그 지점이 실은 다른 탐색기 창이나
                // 바탕화면의 다른 폴더/휴지통 아이콘 위였을 수도 있어 여기서
                // 재확인한다. 단, 드롭 지점이 My Computer 자기 자신이면(=사이드바
                // "Desktop" 항목 위에 놓은 정상 경우) 재확인을 건너뛴다 —
                // current_location() 은 "지금 보이는 탭"이라, 재확인하면 사이드바에서
                // 고른 게 아니라 마침 보고 있던 다른 탭 위치로 잘못 덮어써버린다.
                let my_computer = self.fs.find_by_name(MY_COMPUTER_NAME);
                let window_at_drop = self.wm.file_at(m);
                let dest = if matches!(dest, MoveDest::Desktop) && window_at_drop != my_computer {
                    self.explorer_drop_target_at(m)
                        .or_else(|| self.desktop_folder_drop_target_at(m, &[]).map(MoveDest::Folder))
                        .unwrap_or(dest)
                } else {
                    dest
                };
                // 바탕화면행인데 그 드롭 지점을 어떤 창이든(My Computer 사이드바
                // 자신 포함 — 그 좌표는 창 안의 좌표라 화면상 빈 자리와 무관하다)
                // 가리고 있으면, 그대로 내려놓지 않고 first_free_tile 로 확실히
                // 비어 보이는 자리에 자동 정렬해서 놓는다(휴지통 Restore 와 같은
                // 이유 — 창 뒤에 숨으면 옮긴 게 아니라 없어진 것처럼 보인다).
                let drop_at = if matches!(dest, MoveDest::Desktop) && window_at_drop.is_some() {
                    let (fc, fr) = self.first_free_tile();
                    Self::tile_to_pos(fc, fr)
                } else {
                    m
                };
                self.move_ids_to(&ids, dest, drop_at);
                self.refresh_explorer_if_open(settings);
                self.refresh_recycle_bin_if_open(settings);
                self.refresh_mail_attachable_if_open();
                self.write_save(settings);
            }
            DeskAction::Restore(ids) => {
                self.restore_from_trash(&ids);
                self.refresh_explorer_if_open(settings);
                self.refresh_recycle_bin_if_open(settings);
                self.refresh_mail_attachable_if_open();
                self.write_save(settings);
            }
            DeskAction::MarkMailRead(i) => {
                // fs 에 기록해야 3초 주기 새로고침은 물론 게임 재시작 후에도 읽음
                // 표시가 유지된다 — MailApp 자체의 read 는 창이 새로 만들어질
                // 때마다(apps/mod.rs::open() 이 매번 fs.mail_read 로 다시 초기화)
                // 사라지는 순전히 화면용 상태라 여기(fs)가 진짜 기록이다.
                if !self.fs.mail_read.contains(&i) {
                    self.fs.mail_read.push(i);
                    self.write_save(settings);
                }
            }
            DeskAction::SendNewMail { to, subject, body, attachments } => {
                // Mail 의 "Write Mail" 탭에서 완전히 새로 작성해 보낸 메일 — 내용째
                // fs.sent_mail 에 쌓아서 Mail 앱의 "Sent Items" 탭에 그대로 보여준다.
                self.fs.sent_mail.push(SentMail { to, subject, body, attachments });
                self.write_save(settings);
            }
            DeskAction::EmptyTrash(ids) => {
                // 휴지통의 "Empty Recycle Bin" — DeletePermanently 와 같은 요령으로
                // 하나씩 영구히 지운다(바탕화면에 있었을 리는 없지만 혹시 몰라 같은
                // 안전장치를 그대로 둔다).
                for id in ids {
                    if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                        self.fs.desktop.remove(pos);
                        self.icon_pos.remove(pos);
                    }
                    self.fs.trash_origin.retain(|&(tid, _)| tid != id);
                    self.fs.delete_permanently(id);
                }
                self.refresh_explorer_if_open(settings);
                self.refresh_recycle_bin_if_open(settings);
                self.refresh_mail_attachable_if_open();
                self.write_save(settings);
            }
        }
    }
}
