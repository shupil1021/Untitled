//! 바탕화면 아이콘 — 격자 타일 배치, 아이콘 추가/이동/휴지통 복구, 드래그 놓을 곳 찾기, 아이콘/라벨 그리기, 마우스 범위 선택.

use super::*;

impl DesktopScene {
    // 두 점으로부터 정규화된(음수 없는) 사각형을 만든다.
    pub(super) fn marquee_rect(a: (f32, f32), b: (f32, f32)) -> Rect {
        Rect::new(a.0.min(b.0), a.1.min(b.1), (a.0 - b.0).abs(), (a.1 - b.1).abs())
    }

    // mr 과 겹치는 아이콘들을 선택 상태로 갱신한다. 드래그 중에도 매 프레임 불러서
    // 마우스를 떼기 전에도 실시간으로 선택 표시가 되게 한다.
    pub(super) fn update_marquee_selection(&mut self, mr: Rect) {
        self.selected = (0..self.fs.desktop.len())
            .filter(|&i| Rect::new(self.icon_pos[i].0, self.icon_pos[i].1, IC_W, IC_H).intersects(&mr))
            .collect();
    }

    // 새 아이콘이 놓일 기본 자리 — 가장 왼쪽 칸을 1순위로, 그 안에서는 위쪽을 2순위로
    // 채운다(왼쪽 열이 다 차면(6줄) 그 오른쪽 칸으로 한 칸 넘어간다).
    pub(super) fn grid_pos(i: usize) -> (f32, f32) {
        let rows = (GRID_MAX_ROW + 1) as usize;
        let c = (i / rows) as i32;
        let r = (i % rows) as i32;
        (GRID_ORIGIN + c as f32 * TILE_W, GRID_ORIGIN + r as f32 * TILE_H)
    }

    // ---- 타일맵 (아이콘이 스냅되는 격자) ----
    pub(super) fn tile_of(pos: (f32, f32)) -> (i32, i32) {
        (((pos.0 - GRID_ORIGIN) / TILE_W).round() as i32, ((pos.1 - GRID_ORIGIN) / TILE_H).round() as i32)
    }

    pub(super) fn tile_to_pos(c: i32, r: i32) -> (f32, f32) {
        (GRID_ORIGIN + c as f32 * TILE_W, GRID_ORIGIN + r as f32 * TILE_H)
    }

    pub(super) fn tile_occupied(&self, c: i32, r: i32, ignore: usize) -> bool {
        self.icon_pos
            .iter()
            .enumerate()
            .any(|(i, p)| i != ignore && Self::tile_of(*p) == (c, r))
    }

    // (c,r) 에서 가장 가까운 빈 타일을 나선형으로 찾는다 — 손으로 드래그한 아이콘을
    // 놓았을 때 그 근처 빈 칸에 스냅시키는 용도(드롭 지점 근처를 우선한다).
    pub(super) fn nearest_free_tile(&self, c: i32, r: i32, ignore: usize) -> (i32, i32) {
        for radius in 0i32..12 {
            for dr in -radius..=radius {
                for dc in -radius..=radius {
                    if radius > 0 && dc.abs().max(dr.abs()) != radius {
                        continue;
                    }
                    let (nc, nr) = (c + dc, r + dr);
                    if (0..=GRID_MAX_COL).contains(&nc)
                        && (0..=GRID_MAX_ROW).contains(&nr)
                        && !self.tile_occupied(nc, nr, ignore)
                    {
                        return (nc, nr);
                    }
                }
            }
        }
        (c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW))
    }

    // fs 에 새 파일을 만들어 바탕화면에 아이콘으로 추가한다 — 설치 마법사를 끝내면
    // "설치된 프로그램" 아이콘이 실행 중에 새로 생겨야 해서 런타임에 fs.desktop/
    // icon_pos 를 같이 늘리는 경로가 필요하다(둘은 항상 같은 길이여야 한다).
    pub(super) fn add_desktop_icon(&mut self, name: &str, kind: FileKind) {
        let id = self.fs.add(name, kind);
        self.fs.desktop.push(id);
        let (fc, fr) = self.first_free_tile();
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // 지금 비어있는 칸 중 왼쪽을 1순위, 위쪽을 2순위로 격자 전체를 훑어 첫 번째로
    // 찾은 자리 — 새로 생기거나 바탕화면으로 옮겨오는 아이콘 자리를 정할 때 쓴다.
    // 매번 실제로 비어있는 칸을 다시 훑어서, 손으로 옮겨 비운 자리도 자연스럽게 채운다.
    pub(super) fn first_free_tile(&self) -> (i32, i32) {
        for c in 0..=GRID_MAX_COL {
            for r in 0..=GRID_MAX_ROW {
                if !self.tile_occupied(c, r, usize::MAX) {
                    return (c, r);
                }
            }
        }
        (0, 0) // 격자가 완전히 꽉 찼으면 어쩔 수 없이 겹쳐서라도 첫 칸에 둔다.
    }

    // File Explorer 에서 드래그해온 기존 파일(id)을 바탕화면에 놓는다 — Downloads/폴더에서
    // 옮겨오는 경우. drop_at 은 실제로 마우스를 놓은 화면 좌표 — 그 위치에서 가장
    // 가까운 빈 칸(nearest_free_tile)에 둬서 "마우스로 놓은 자리"에 실제로 놓인다.
    pub(super) fn add_existing_to_desktop(&mut self, id: FileId, drop_at: (f32, f32)) {
        if self.fs.desktop.contains(&id) {
            return;
        }
        self.fs.desktop.push(id);
        let (c, r) = Self::tile_of(drop_at);
        let (fc, fr) = self.nearest_free_tile(c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW), usize::MAX);
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // add_existing_to_desktop 과 달리 마우스 좌표에 안 기댄다 — 휴지통 "Restore"
    // 버튼처럼 놓은 지점이라는 게 아예 없는(버튼을 누른 시점의 마우스 좌표는 그
    // 버튼 위, 즉 지금 열려있는 휴지통 창 안이라 그 좌표를 드롭 지점으로 쓰면
    // 복구된 아이콘이 바로 그 창 뒤에 깔려 안 보이는 문제가 있었다) 프로그램적
    // 프로그램적 복구엔 항상 실제로 비어있는 첫 칸(first_free_tile)에 놓는 게 맞다.
    pub(super) fn add_existing_to_desktop_default(&mut self, id: FileId) {
        if self.fs.desktop.contains(&id) {
            return;
        }
        self.fs.desktop.push(id);
        let (fc, fr) = self.first_free_tile();
        self.icon_pos.push(Self::tile_to_pos(fc, fr));
    }

    // File Explorer 사이드바 드래그(DeskAction::MoveFiles)와 바탕화면 아이콘을 열린
    // 탐색기 창으로 끌어다 놓는 경로가 둘 다 "지금 있던 자리에서 떼어 새 자리로"
    // 라는 같은 로직을 쓰므로 하나로 묶어둔다. drop_at 은 MoveDest::Desktop 일 때만
    // 쓰인다(어디에 놓였는지).
    pub(super) fn move_ids_to(&mut self, ids: &[FileId], dest: MoveDest, drop_at: (f32, f32)) {
        // 휴지통(이름이 "Recycle Bin"인 Folder) 으로 들어가는 경우에만 "들어가기
        // 직전에 어디 있었는지" 를 기록해서, 나중에 Restore 가 정확히 그 자리로
        // 되돌릴 수 있게 한다. 그 외의 이동(휴지통에서 나가는 것 포함)은 더 이상
        // 그 기록이 필요 없으니 지운다.
        let into_recycle_bin = matches!(dest, MoveDest::Folder(fid) if self.fs.get(fid).name == RECYCLE_BIN_NAME);
        for &id in ids {
            // My Computer(FileKind::Explorer)는 실제 Windows 도 그렇듯 폴더/휴지통 안으로
            // 옮길 수 없다 — 휴지통에 들어가면 "Empty Recycle Bin" 한 번으로 영구히
            // 사라져서 게임을 진행할 방법 자체가 없어지는 치명적인 경우라, 옮기는 시도
            // 자체를 여기서 막는다(그 무엇으로도 못 지운다는 사실을 애초에 보장). 휴지통
            // 자기 자신도 같은 이유로 막는다 — 이 가드가 없었을 땐 휴지통 아이콘을
            // 휴지통 안으로 끌어놓으면 자기 자신을 자기 children 에 넣어버려서(제보받은
            // "쓰레기통에 쓰레기통" 스크린샷) 휴지통을 열면 그 안에 휴지통이 보이는
            // 자기참조 상태가 됐었다.
            if matches!(dest, MoveDest::Folder(_))
                && (matches!(self.fs.get(id).kind, FileKind::Explorer) || self.fs.get(id).name == RECYCLE_BIN_NAME)
            {
                continue;
            }
            // 지금 창이 열려있는 파일은 휴지통으로 못 보낸다 — 열어서 보고 있는 걸
            // 그대로 지워버리면(휴지통 비우기까지 한 번이면 영구 삭제) 그 창은 이제
            // 존재하지 않는 파일을 가리키는 유령 창이 되어버린다. 실제 Windows 도
            // 열려있는 파일은 삭제를 막거나 경고하는 것과 같은 맥락 — 여기선 그냥
            // 조용히 옮기지 않고 원래 있던 자리에 그대로 둔다.
            if into_recycle_bin && self.wm.is_open(id) {
                continue;
            }
            self.fs.trash_origin.retain(|&(tid, _)| tid != id);
            if into_recycle_bin {
                // locate() 가 못 찾으면(예: 어떤 컨테이너에도 안 속하고 Videos/Images
                // 가상 탭에만 종류로 보이던 사진/동영상) Loose 로 기록한다 — None 으로
                // 그냥 건너뛰면 나중에 복구할 때 이 파일이 "기록이 아예 없던 것"과
                // 구분이 안 돼서 엉뚱하게 바탕화면으로 복구돼버린다.
                let origin = self.fs.locate(id).unwrap_or(FileOrigin::Loose);
                self.fs.trash_origin.push((id, origin));
            }
            if let Some(pos) = self.fs.desktop.iter().position(|&d| d == id) {
                self.fs.desktop.remove(pos);
                self.icon_pos.remove(pos);
            }
            self.fs.detach_from_container(id);
            match dest {
                MoveDest::Desktop => self.add_existing_to_desktop(id, drop_at),
                MoveDest::Downloads => self.fs.download(id),
                MoveDest::Folder(fid) => self.fs.add_to_folder(fid, id),
            }
        }
    }

    // 휴지통의 "Restore" — fs.trash_origin 에 기록해둔 원래 위치로 정확히 되돌린다
    // (기록이 없으면, 예: 이 기능이 생기기 전에 이미 휴지통에 들어가 있던 항목이면
    // 바탕화면으로). 원래 있던 폴더가 그 사이 사라졌을 리는 없지만(폴더 자체를
    // 영구 삭제하는 기능은 없다) 혹시 몰라 여전히 Folder 인지 확인하고 아니면
    // 역시 바탕화면으로 대신 보낸다.
    pub(super) fn restore_from_trash(&mut self, ids: &[FileId]) {
        for &id in ids {
            let origin =
                self.fs.trash_origin.iter().position(|&(tid, _)| tid == id).map(|pos| self.fs.trash_origin.remove(pos).1);
            self.fs.detach_from_container(id);
            match origin {
                Some(FileOrigin::Downloads) => self.fs.download(id),
                Some(FileOrigin::Folder(fid)) if matches!(self.fs.get(fid).kind, FileKind::Folder { .. }) => {
                    self.fs.add_to_folder(fid, id);
                }
                // 애초에 어떤 컨테이너에도 안 속하고 종류(Img/Mp4)만으로 Videos/Images
                // 가상 탭에 보이던 파일 — 방금 detach_from_container 로 휴지통 children
                // 에서 빠졌으니, 여기서 아무 데도 새로 안 넣어도 그 가상 탭에 저절로
                // 다시 나타난다(어딘가에 억지로 넣으면 오히려 중복으로 보인다).
                Some(FileOrigin::Loose) => {}
                _ => self.add_existing_to_desktop_default(id),
            }
        }
    }

    // 바탕화면 아이콘을 드래그해서 놓은 자리(m)가 열려있는 File Explorer 창 위라면,
    // 그 창이 지금 보여주고 있는 폴더/카테고리를 옮길 대상으로 돌려준다 — 창이
    // 없거나 파일 탐색기가 아니거나(Mail/Settings 등), 대상이 될 수 없는 카테고리
    // (Videos 등)면 None.
    pub(super) fn explorer_drop_target_at(&mut self, m: (f32, f32)) -> Option<MoveDest> {
        let win_file = self.wm.file_at(m)?;
        // Credits/Official Site 같은 창은 실제 fs.nodes 항목이 아니라 창
        // 중복-열기 방지용 가짜 FileId(CREDITS_WIN 등, usize::MAX 근처)를 쓴다 —
        // 그 값을 아래 fs.get() 에 그대로 넘기면 배열 범위를 벗어나 패닉한다
        // (제보받은 "크래딧 창 위에 파일을 놓으면 게임이 터진다" 버그의 원인).
        // 진짜 fs.nodes 안의 id 가 아니면 그냥 "옮길 대상 아님"으로 취급한다.
        if !self.fs.contains(win_file) {
            return None;
        }
        // 창 자체가 실제 폴더에 연결돼 있으면(더블클릭으로 드릴다운한 단일 폴더
        // 탐색기 창) 그 폴더가 바로 대상 — 탭이 없어 downcast 로 물어볼 필요가 없다.
        if matches!(self.fs.get(win_file).kind, FileKind::Folder { .. }) {
            return Some(MoveDest::Folder(win_file));
        }
        // 그 외엔 탭이 있는 루트 File Explorer 창일 수 있다 — 지금 활성 탭을 물어본다.
        let explorer = self.wm.app_mut(win_file)?.as_any_mut().downcast_mut::<ExplorerApp>()?;
        match explorer.current_location()? {
            ExplorerLocation::Folder(fid) => Some(MoveDest::Folder(fid)),
            ExplorerLocation::Category(name) => match name.as_str() {
                "Desktop" => Some(MoveDest::Desktop),
                "Downloads" => Some(MoveDest::Downloads),
                _ => None, // Videos/Images 등 — 종류로만 모으는 가상 카테고리라 대상이 될 수 없다
            },
        }
    }

    // 바탕화면 위의 다른 폴더 아이콘 위에 놓았는지 — 있으면 그 폴더 안으로 옮긴다.
    // exclude 는 지금 드래그 중인 아이콘들 자신의 인덱스(자기 자신 위에 놓은 걸
    // "드롭"으로 치면 안 되니 제외).
    pub(super) fn desktop_folder_drop_target_at(&self, m: (f32, f32), exclude: &[usize]) -> Option<FileId> {
        for i in 0..self.fs.desktop.len() {
            if exclude.contains(&i) {
                continue;
            }
            let (x, y) = self.icon_pos[i];
            if Rect::new(x, y, IC_W, IC_H).contains(m.0, m.1) {
                let fid = self.fs.desktop[i];
                if matches!(self.fs.get(fid).kind, FileKind::Folder { .. }) {
                    return Some(fid);
                }
            }
        }
        None
    }

    pub(super) fn icon_hit(&self, m: (f32, f32)) -> Option<usize> {
        for i in 0..self.fs.desktop.len() {
            let (x, y) = self.icon_pos[i];
            if Rect::new(x, y, IC_W, IC_H).contains(m.0, m.1) {
                return Some(i);
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_one_icon(&mut self, r: &mut Renderer, assets: &Assets, x: f32, y: f32, icon: &IconType, name: &str, selected: bool) {
        // 아이콘의 세로 중심은 항상 ICON_AREA_TOP+ICON_BASE_SIZE/2(고정값) 에 맞추고,
        // 그 아래 글자 시작 위치(ty0)도 항상 그 고정값 기준으로만 잡는다.
        let icon_s = IC_SIZE;
        let icon_center_y = y + ICON_AREA_TOP + ICON_BASE_SIZE / 2.0;
        let (icon_x, icon_y) = (x + IC_W / 2.0 - icon_s / 2.0, icon_center_y - icon_s / 2.0);
        draw_icon(r, assets, icon, icon_x, icon_y, icon_s);
        let ls = LABEL_TEXT_SCALE;
        let lines = wrap_two_lines(r, name, ls, LABEL_MAX_W);
        let line_h = LABEL_LINE_H;
        let ty0 = y + ICON_AREA_TOP + ICON_BASE_SIZE + LABEL_GAP;
        // LABEL_MAX_W 로 감싸도, 글자폭 계산(ADVANCE 기준)이 실제 렌더링(CRT
        // 블룸 등)과 완벽히 안 맞으면 여전히 옆 칸 글자와 살짝 겹쳐 보일 수 있다 —
        // 계산에 기대는 대신 타일 폭(IC_W) 밖으로는 물리적으로 아예 못 그리게
        // 클립을 걸어서 확실히 막는다.
        let outer_clip = r.clip();
        let label_rect = Rect::new(x - 1.0, ty0 - 1.0, IC_W + 2.0, line_h * LABEL_MAX_LINES + 2.0);
        r.set_clip(Some(outer_clip.map_or(label_rect, |c| c.intersect(&label_rect))));
        for (i, line) in lines.iter().enumerate() {
            let tw = r.text_width(line, ls);
            let tx = x + (IC_W - tw) / 2.0;
            let ty = ty0 + i as f32 * line_h;
            if selected {
                r.rect(tx - 2.0, ty - 1.0, tw + 4.0, line_h, NAVY);
                r.text(tx, ty, line, ls, WHITE);
            } else {
                r.text(tx, ty, line, ls, WHITE);
            }
        }
        r.set_clip(outer_clip);
    }

    // 드래그 중에도 icon_pos 는 원래 자리 그대로라(더는 실시간으로 안 움직인다 —
    // 대신 update() 가 반투명 고스트를 커서 쪽에 따로 그린다), 예전처럼 드래그 중인
    // 아이콘을 맨 위에 다시 그려줄 필요가 없어져서 한 번에 순서대로만 그리면 된다.
    pub(super) fn draw_icons(&mut self, r: &mut Renderer, assets: &Assets, lang: Language) {
        for i in 0..self.fs.desktop.len() {
            let fid = self.fs.desktop[i];
            let node = self.fs.get(fid);
            let (x, y) = self.icon_pos[i];
            let name = display_name(lang, &node.name).into_owned();
            let icon = icon_of(node);
            let selected = self.selected.contains(&i);
            self.draw_one_icon(r, assets, x, y, &icon, &name, selected);
        }
    }

    // 바탕화면 아이콘 마우스 처리 — 클릭(선택/더블클릭 열기/고무줄 선택 시작), 드래그 이동, 놓기(탐색기·폴더·바탕화면 칸으로
    // 옮기기). consumed 는 위쪽 UI(메뉴/창)가 이미 이번 클릭을 썼다는 뜻이다.
    pub(super) fn handle_icon_mouse(&mut self, f: &mut Frame, m: (f32, f32), click: bool, consumed: bool, work: Rect) {
        // 5) 바탕화면 아이콘 클릭 / 드래그 (최하위)
        if !consumed && click {
            if let Some(i) = self.icon_hit(m) {
                if self.last_idx == i as i32 && f.time - self.last_click < 0.35 {
                    self.open_file(i, &f.settings, work);
                    self.drag = None;
                    self.last_idx = -1;
                } else {
                    // 이미 여러 개가 선택된 상태에서 그 중 하나를 누르면 선택을 유지한 채
                    // 다같이 옮길 수 있게 하고, 선택 안 된 아이콘을 누르면 그것만 새로 선택한다.
                    if !self.selected.contains(&i) {
                        self.selected = vec![i];
                    }
                    let group_start = self.selected.iter().map(|&gi| (gi, self.icon_pos[gi])).collect();
                    // 오프셋은 타일(icon_pos, 64x60) 왼쪽 위가 아니라 실제로 그려지는
                    // 아이콘 글리프 위치(draw_one_icon 의 x+IC_W/2-IC_SIZE/2, y+3 와
                    // 똑같은 계산) 기준으로 잡아야 한다 — 안 그러면 고스트가 처음
                    // 그려지는 순간 실제 아이콘 글리프 자리에서 옆으로 살짝 어긋나
                    // "잡은 위치가 안 맞는" 느낌이 났다.
                    let glyph = (self.icon_pos[i].0 + IC_W / 2.0 - IC_SIZE / 2.0, self.icon_pos[i].1 + 3.0);
                    let offset = (m.0 - glyph.0, m.1 - glyph.1);
                    self.drag = Some(IconDrag { start: m, group_start, offset, moved: false });
                    self.last_idx = i as i32;
                }
                self.last_click = f.time;
            } else {
                // 빈 바탕화면을 누르면 고무줄(마퀴) 선택 시작.
                self.selected.clear();
                self.last_idx = -1;
                self.marquee_start = Some(m);
            }
        }

        // 드래그 이동(버튼 누른 채) — 실제 아이콘 위치(icon_pos)는 여기서 안 바꾼다.
        // 원래 자리에 그대로 둔 채로, 아래에서 반투명 복사본(고스트)만 커서를
        // 따라다니게 그린다(File Explorer 드래그와 같은 느낌) — 실제 이동은 손을
        // 놓는 순간에 한 번만 계산해서 적용한다.
        if !consumed
            && f.input.mouse_down
            && let Some(d) = &mut self.drag
        {
            let dx = m.0 - d.start.0;
            let dy = m.1 - d.start.1;
            d.moved = d.moved || dx * dx + dy * dy > 16.0;
        }

        // 놓기: 드래그 종료
        let released = self.prev_down && !f.input.mouse_down;
        if released {
            if let Some(ms) = self.marquee_start.take() {
                let mr = Self::marquee_rect(ms, m);
                self.update_marquee_selection(mr);
            }
            if let Some(d) = self.drag.take()
                && d.moved
            {
                // 놓은 자리가 열려있는 File Explorer 창 위면 바탕화면이 아니라 그
                // 창이 보여주는 폴더/카테고리로 옮긴다 — id 는 인덱스가 밀리기 전에
                // 먼저 모아둬야 한다(move_ids_to 가 fs.desktop 에서 실제로 빼버린다).
                // 창이 없으면(바탕화면 자체), 놓은 자리에 다른 폴더 아이콘이 있는지도
                // 확인해서(자기 자신은 제외) 있으면 그 폴더 안으로 넣는다 — 창 쪽을
                // 먼저 확인하는 건 창이 아이콘보다 위에 그려져서 실제로 보이는 게
                // 창이면 그쪽이 우선이어야 자연스럽기 때문이다.
                let dragged: Vec<usize> = d.group_start.iter().map(|&(gi, _)| gi).collect();
                let explorer_dest = self
                    .explorer_drop_target_at(m)
                    .or_else(|| self.desktop_folder_drop_target_at(m, &dragged).map(MoveDest::Folder));
                if let Some(dest) = explorer_dest {
                    let ids: Vec<FileId> = d.group_start.iter().filter_map(|&(gi, _)| self.fs.desktop.get(gi).copied()).collect();
                    self.move_ids_to(&ids, dest, m);
                    self.refresh_explorer_if_open(&f.settings);
                    self.refresh_recycle_bin_if_open(&f.settings);
                } else if self.wm.file_at(m).is_some() {
                    // 옮길 대상은 못 되는 다른 창(폴더로 못 옮기는 Notepad/Mail 등)이
                    // 그 자리를 가리고 있다 — 그대로 놓으면 아이콘이 창 뒤에 숨어버리니
                    // icon_pos 를 안 건드려 드래그를 취소한 것처럼 둔다(폴더 창으로 옮기는
                    // 경우는 위 explorer_dest 에서 이미 처리돼 여기까지 안 온다).
                } else {
                    // 실제로 안 움직였던 icon_pos 에 이제야 델타(드래그 시작점부터 놓은
                    // 지점까지)를 한 번에 적용해서 목표 칸을 정한다.
                    let dx = m.0 - d.start.0;
                    let dy = m.1 - d.start.1;
                    for &(gi, gp) in &d.group_start {
                        if gi < self.icon_pos.len() {
                            let (c, r) = Self::tile_of((gp.0 + dx, gp.1 + dy));
                            let (fc, fr) = self.nearest_free_tile(c.clamp(0, GRID_MAX_COL), r.clamp(0, GRID_MAX_ROW), gi);
                            self.icon_pos[gi] = Self::tile_to_pos(fc, fr);
                        }
                    }
                }
                // 아이콘을 옮기거나 옮겨놓은 자리도 그 즉시 저장 — 5초 자동저장을
                // 기다리다 그 사이에 닫으면 방금 한 일이 반영 안 된 채로 남는다.
                self.write_save(&f.settings);
            }
        }
        self.prev_down = f.input.mouse_down;
    }
}
