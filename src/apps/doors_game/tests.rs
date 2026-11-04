//! 메인 게임(DoorsGameApp)의 진행/장면 이동/상호작용 테스트.

use crate::apps::game3d::player::aim_score;
use super::world::build_room;
use super::*;

fn quiet(app: &mut DoorsGameApp) {
    app.dialogue = Dialogue::new();
}

// 장소를 옮기는 페이드를 끝까지 돌린다.
fn finish_move(app: &mut DoorsGameApp) {
    quiet(app);
    for _ in 0..100 {
        app.advance_scene(0.05);
    }
}

// 시트 오른쪽 표의 흐름 — 수락 → 편지 집기 → 우편함 → 메일을 읽고 돌아옴 → 문 대사 변경 →
// 그 뒤에야 방 물건으로 "꽃이 없다"를 확인할 수 있다.
// 저장된 진행으로 다시 열면 처음 인사 대사 없이 그 상태 그대로 이어진다.
#[test]
fn resuming_keeps_progress_and_skips_the_intro() {
    let fresh = DoorsGameApp::new(DoorsProgress::default());
    assert!(fresh.dialogue.active(), "처음 열면 상황 설명 대사가 나온다");

    let saved = DoorsProgress { quest_accepted: true, has_letter: true, letter_sent: true, ..DoorsProgress::default() };
    let app = DoorsGameApp::new(saved.clone());
    assert!(!app.dialogue.active());
    assert_eq!(app.progress, saved);
    assert_eq!(app.saved, saved, "불러온 값은 바뀐 게 아니라서 저장을 다시 요청하지 않는다");
    assert!(app.letter_notify_pending, "이미 보낸 편지는 OS 에 다시 한 번 알린다(메일은 중복으로 안 온다)");
}

#[test]
fn letter_loop_gates_the_flower_check() {
    let mut app = DoorsGameApp::new(DoorsProgress::default());
    quiet(&mut app);

    app.interact_room(Some(Target::Mailbox));
    assert!(!app.progress.letter_sent, "편지가 없으면 보낼 게 없다");

    app.progress.quest_accepted = true;
    app.interact_room(Some(Target::Prop(0)));
    assert!(!app.progress.flower_absence_checked, "편지 일을 끝내기 전엔 꽃이 없다는 걸 못 깨닫는다");

    app.interact_room(Some(Target::Letter));
    assert!(app.progress.has_letter);
    app.interact_room(Some(Target::Mailbox));
    assert!(app.progress.letter_sent && app.letter_notify_pending);

    quiet(&mut app);
    assert!(!app.notice_mail_read(false, true), "메일을 안 읽었으면 문은 그대로");
    assert!(!app.notice_mail_read(true, false), "창으로 돌아오기 전엔 문은 그대로");
    assert!(app.notice_mail_read(true, true) && app.progress.door_changed);

    quiet(&mut app);
    app.interact_room(Some(Target::Prop(0)));
    assert!(app.progress.flower_absence_checked);
}

// 꽃이 도착 → 문에게 건넴 → 자물쇠가 풀려 문이 올라감 → 문으로 나가 2층 정글에 서고 메일이 온다.
#[test]
fn flower_unlocks_the_door_and_leads_outside() {
    let progress = DoorsProgress { quest_accepted: true, has_letter: true, letter_sent: true, door_changed: true, flower_absence_checked: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);

    assert!(!app.receive_flower(false, true), "꽃을 아직 내려받지 않았으면 안 들어온다");
    assert!(!app.receive_flower(true, false), "이 창이 앞에 오기 전엔 안 들어온다");
    assert!(app.receive_flower(true, true) && app.progress.flower_received);
    assert!(!app.receive_flower(true, true), "꽃은 한 번만 들어온다");

    // 꽃을 갖고 문에게 말을 걸기 전엔 문이 닫혀 있다. 건네면 대사 뒤에 자물쇠가 풀린다.
    quiet(&mut app);
    assert!(!app.progress.tutorial_done && app.door_lift == 0.0);
    app.interact_room(Some(Target::Door));
    assert!(app.progress.flower_given && app.unlock_pending && !app.progress.tutorial_done);
    app.advance_scene(0.1);
    assert!(!app.progress.tutorial_done, "대사가 끝나기 전엔 안 풀린다");
    quiet(&mut app);
    app.advance_scene(0.1);
    assert!(app.progress.tutorial_done && app.door_anim.is_some());
    app.advance_scene(DOOR_OPEN_SECS);
    assert_eq!(app.door_lift, DOOR_LIFT);
    assert!(app.door_anim.is_none());

    // 열린 문으로 나가면 페이드 뒤에 정글에 서고, 처음이라 설명 대사 + 메일이 따라온다.
    quiet(&mut app);
    app.interact_room(Some(Target::Door));
    assert_eq!(app.pending_move, Some(MainPlace::Jungle));
    finish_move(&mut app);
    assert_eq!(app.progress.place, MainPlace::Jungle);
    assert!(app.progress.jungle_visited && app.dialogue.active());
    assert_eq!(app.pending_mail, Some(MailId::Outside));
    assert!(!app.jungle.is_empty() && !app.barriers.is_empty());
}

// 정글에서 닫았다 다시 열면 정글에서 이어지고, 문은 이미 열려 있으며, 설명 대사는 안 나온다.
#[test]
fn resuming_in_the_jungle() {
    let progress = DoorsProgress {
        quest_accepted: true,
        tutorial_done: true,
        flower_received: true,
        flower_given: true,
        jungle_visited: true,
        place: MainPlace::Jungle,
        ..DoorsProgress::default()
    };
    let app = DoorsGameApp::new(progress);
    assert!(!app.dialogue.active() && !app.jungle.is_empty());
    assert_eq!(app.door_lift, DOOR_LIFT);
}

#[test]
fn jungle_has_five_gates_a_sign_and_a_sofa() {
    let items = floor2::items(false);
    assert_eq!(items.iter().filter(|it| matches!(it.target, floor2::Target::Gate(_))).count(), 5);
    assert!(items.iter().any(|it| it.target == floor2::Target::Sign) && items.iter().any(|it| it.target == floor2::Target::Sofa));
    assert!(!items.iter().any(|it| it.target == floor2::Target::Envelope), "교환 전엔 봉투가 없다");
    assert!(floor2::items(true).iter().any(|it| it.target == floor2::Target::Envelope));
}

fn in_jungle() -> DoorsGameApp {
    let progress = DoorsProgress { quest_accepted: true, tutorial_done: true, flower_given: true, jungle_visited: true, place: MainPlace::Jungle, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);
    app
}

// 계단이 먼저 배고프다고 해야 소파가 의뢰를 건네고, 수락하면 미션 문이 열린다(장보기 목록 메일).
#[test]
fn errand_needs_the_stairs_first_and_unlocks_the_mission_doors() {
    let mut app = in_jungle();
    app.interact_jungle(floor2::Target::Gate(Gate::Eye));
    assert!(app.pending_move.is_none(), "의뢰 전엔 미션 문이 잠겨 있다");

    quiet(&mut app);
    app.interact_jungle(floor2::Target::Sofa);
    assert!(app.ask.is_none() && !app.progress.errand_accepted, "계단 얘기를 듣기 전엔 의뢰를 안 준다");

    app.progress.place = MainPlace::StairHall;
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert!(app.progress.stair_met);
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert!(app.dialogue.active(), "의뢰 전에도 같은 말을 한다");

    app.progress.place = MainPlace::Jungle;
    quiet(&mut app);
    app.interact_jungle(floor2::Target::Sofa);
    assert_eq!(app.ask, Some(Ask::Errand));
    app.answer(true);
    assert!(app.progress.errand_accepted && app.pending_mail == Some(MailId::Shopping));

    quiet(&mut app);
    app.interact_jungle(floor2::Target::Gate(Gate::Brain));
    assert_eq!(app.pending_move, Some(MainPlace::MissionRoom(MiniKind::Brain)));
}

// 눈의 방 한 바퀴: 의뢰 → 기계(메일) → 보상 사진 내려받음 → 재료 → 달라진 NPC.
#[test]
fn a_mission_runs_from_request_to_ingredient() {
    let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Eye), tutorial_done: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);

    app.interact_spot(Spot::Machine);
    assert_eq!(app.progress.missions[0], MissionStep::Start, "NPC 의뢰를 듣기 전엔 기계가 안 켜진다");
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(app.progress.missions[0], MissionStep::Talked);
    quiet(&mut app);
    app.interact_spot(Spot::Machine);
    assert_eq!(app.progress.missions[0], MissionStep::Sent);
    assert_eq!(app.pending_mail, Some(MailId::EyeTask));
    assert!(app.progress.pose.is_some(), "게임을 받는 순간 서 있는 자리도 저장된다");

    quiet(&mut app);
    assert!(!app.receive_ingredient([false; 3], true), "사진을 내려받기 전엔 재료가 없다");
    assert!(!app.receive_ingredient([true, false, false], false), "창이 앞에 오기 전엔 안 들어온다");
    assert!(app.receive_ingredient([true, false, false], true));
    assert_eq!(app.progress.missions[0], MissionStep::Received);
    assert_eq!(app.ingredients(), 1);
    quiet(&mut app);
    assert!(!app.receive_ingredient([true, false, false], true), "재료는 한 번만 받는다");

    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(app.progress.missions[0], MissionStep::Done);
    quiet(&mut app);
    app.interact_spot(Spot::Machine);
    assert_eq!(app.progress.missions[0], MissionStep::Done, "끝난 방을 다시 찾아도 보상은 추가로 안 준다");
    assert_eq!(app.ingredients(), 1);
}

// 금고의 방 한 바퀴: 의뢰 → 기계(메일) → 금고 반쪽뿐 → 사진(금고 절반) 내려받음 → 맞춰 완성 → 열어서 소환 조각.
#[test]
fn the_vault_room_runs_from_half_to_fragment() {
    let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Brain), tutorial_done: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    let step = |app: &DoorsGameApp| app.progress.missions[MiniKind::Brain.index()];
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    quiet(&mut app);
    app.interact_spot(Spot::Machine);
    assert_eq!(step(&app), MissionStep::Sent);
    assert_eq!(app.pending_mail, Some(MailId::BrainTask));

    quiet(&mut app);
    app.interact_spot(Spot::Safe);
    assert_eq!(step(&app), MissionStep::Sent, "절반이 없으면 금고는 맞출 수 없다");
    quiet(&mut app);
    assert!(app.receive_ingredient([false, true, false], true));
    assert_eq!(step(&app), MissionStep::Received);
    assert_eq!(app.ingredients(), 0, "금고 절반은 교환 재료가 아니다");

    quiet(&mut app);
    app.interact_spot(Spot::Safe);
    assert_eq!(step(&app), MissionStep::Built);
    quiet(&mut app);
    app.interact_spot(Spot::Safe);
    assert_eq!(step(&app), MissionStep::Fragment);
    assert_eq!(app.ingredients(), 1, "소환 조각(레이싱)만 교환 재료다");
    quiet(&mut app);
    app.interact_spot(Spot::Safe);
    assert_eq!(step(&app), MissionStep::Fragment, "금고의 내용물은 다시 얻을 수 없다");
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(step(&app), MissionStep::Done);
}

// 화장실 한 바퀴: 기사의 의뢰 수락(효과음 + 메일) → 호수 전달 → 변기 속 손 → 손잡이 → 조립 → 물 내리기 → 조각 → 보고.
#[test]
fn the_bathroom_runs_from_the_knights_request_to_the_fragment() {
    let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Finger), tutorial_done: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    let step = |app: &DoorsGameApp| app.progress.missions[MiniKind::Finger.index()];
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(app.ask, Some(Ask::Knight));
    app.answer(false);
    assert_eq!(step(&app), MissionStep::Start, "거절하면 의뢰는 시작되지 않는다");
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    app.answer(true);
    assert_eq!(step(&app), MissionStep::Sent);
    assert_eq!(app.pending_mail, Some(MailId::FingerTask));
    assert_eq!(app.sfx_cue, Some(sfx::Cue::QuestStart));

    // 호수에 손잡이를 넣기 전엔 변기가 비어 있다(신호는 OS 가 쓴다).
    crate::signals::update(|s| s.lake_delivered = false);
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert!(!app.progress.hand_out);
    crate::signals::update(|s| s.lake_delivered = true);
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert!(app.progress.hand_out && step(&app) == MissionStep::Sent, "손이 나오는 것과 손잡이를 받는 것은 따로다");
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert_eq!(step(&app), MissionStep::Received);
    assert!(!app.progress.hand_out && app.ingredients() == 0);
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert_eq!(step(&app), MissionStep::Built);
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert_eq!(step(&app), MissionStep::Revealed);
    assert_eq!(app.sfx_cue, Some(sfx::Cue::Flush));
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert_eq!(step(&app), MissionStep::Fragment);
    assert_eq!(app.ingredients(), 1);
    quiet(&mut app);
    app.interact_spot(Spot::Toilet);
    assert_eq!(step(&app), MissionStep::Fragment, "물을 또 내려도 조각은 더 나오지 않는다");
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(step(&app), MissionStep::Done);
    crate::signals::update(|s| s.lake_delivered = false);
}

// 재료 셋 → 소파에게 교환 → 봉투(Mr. POTATO CHIP) → 계단에게 먹임 → 계단이 펼쳐지고 2층이 끝난다.
#[test]
fn three_ingredients_become_a_chip_that_feeds_the_stairs() {
    let progress = DoorsProgress {
        errand_accepted: true,
        stair_met: true,
        tutorial_done: true,
        place: MainPlace::Jungle,
        jungle_visited: true,
        missions: [MissionStep::Received, MissionStep::Fragment, MissionStep::Fragment],
        ..DoorsProgress::default()
    };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);

    // 둘만 있으면 부족한 걸 알려준다(가진 재료는 그대로).
    app.progress.missions[2] = MissionStep::Talked;
    app.interact_jungle(floor2::Target::Sofa);
    assert!(app.ask.is_none() && app.ingredients() == 2);
    app.progress.missions[2] = MissionStep::Fragment;

    quiet(&mut app);
    app.interact_jungle(floor2::Target::Sofa);
    assert_eq!(app.ask, Some(Ask::Exchange));
    app.answer(true);
    assert!(app.progress.exchanged && app.ingredients() == 0, "세 재료가 한꺼번에 회수된다");

    quiet(&mut app);
    app.interact_jungle(floor2::Target::Envelope);
    assert!(app.progress.chip_taken && app.has_chip());
    assert_eq!(app.pending_mail, Some(MailId::Meal));
    assert!(!floor2::items(app.progress.exchanged && !app.progress.chip_taken).iter().any(|it| it.target == floor2::Target::Envelope));

    app.progress.place = MainPlace::StairHall;
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    assert_eq!(app.ask, Some(Ask::Feed));
    app.answer(false);
    assert!(!app.progress.chip_fed && app.has_chip(), "거절하면 아이템은 그대로");
    quiet(&mut app);
    app.interact_spot(Spot::Npc);
    app.answer(true);
    assert!(app.progress.chip_fed && !app.has_chip(), "먹이면 아이템 1개 소비와 급식 완료가 같이 일어난다");

    quiet(&mut app);
    for _ in 0..100 {
        app.advance_scene(0.1);
    }
    assert_eq!(app.unfold, 1.0);
    assert!(rooms::stair_hall(true, app.unfold).items.iter().any(|it| it.target == Spot::StairStep));
    quiet(&mut app);
    app.interact_spot(Spot::StairStep);
    assert!(app.progress.floor2_done);
}

#[test]
fn leaving_a_room_puts_you_in_front_of_its_door() {
    let progress = DoorsProgress { errand_accepted: true, place: MainPlace::MissionRoom(MiniKind::Finger), tutorial_done: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);
    app.interact_spot(Spot::Exit);
    finish_move(&mut app);
    assert_eq!(app.progress.place, MainPlace::Jungle);
    let front = floor2::gate_front(Gate::Finger);
    let cam = app.player.camera();
    assert!((cam.pos[0] - front[0]).abs() < 0.01 && (cam.pos[2] - front[2]).abs() < 0.01);
}

#[test]
fn focus_loss_remembers_where_you_stood() {
    let mut app = in_jungle();
    assert!(app.progress.pose.is_none());
    app.remember_pose_on_blur(true);
    app.remember_pose_on_blur(false);
    let pose = app.progress.pose.expect("포커스를 잃으면 자리를 기억한다");
    let resumed = DoorsGameApp::new(DoorsProgress { pose: Some(pose), place: MainPlace::Jungle, ..app.progress.clone() });
    let cam = resumed.player.camera();
    assert!((cam.pos[0] - pose[0]).abs() < 0.01 && (cam.pos[2] - pose[2]).abs() < 0.01);
}

// 어느 장소든 시작 자리가 상자 속에 파묻혀 있거나 허공이 아니어야 한다(나무/소파/벽에 끼지 않고 바닥을 딛는다).
#[test]
fn every_spawn_stands_on_the_floor_and_clear_of_walls() {
    use crate::render::mesh3d::{ground_height, resolve_horizontal};
    let jungle = {
        let mut b = floor2::build_scene([false; 3], false);
        b.extend(floor2::items(true).iter().map(|it| it.to_box()));
        b.extend(floor2::barriers());
        b
    };
    let mut cases: Vec<(&str, [f32; 3], Vec<Box3D>)> = vec![
        ("시작방", SPAWN, { let mut b = build_room(0.0); b.extend(super::world::invisible_colliders()); b }),
        ("정글(시작방 문 앞)", floor2::SPAWN, jungle.clone()),
        ("계단 입구", rooms::SPAWN, rooms::stair_hall(false, 0.0).boxes),
    ];
    for gate in [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs] {
        cases.push(("정글(문 앞)", floor2::gate_front(gate), jungle.clone()));
    }
    for kind in MiniKind::ALL {
        let room = rooms::mission_room(kind, MissionStep::Start, false, 0.0);
        let mut b = room.boxes;
        b.extend(room.barriers);
        cases.push(("미션 방", rooms::SPAWN, b));
    }
    for (name, spawn, boxes) in cases {
        let pushed = resolve_horizontal(spawn, 0.3, spawn[1], 1.7, &boxes);
        assert!((pushed[0] - spawn[0]).abs() < 0.01 && (pushed[2] - spawn[2]).abs() < 0.01, "{name}: 시작 자리가 무언가에 겹쳐 있다 {spawn:?}");
        let ground = ground_height(spawn[0], spawn[2], spawn[1] + 1.0, 60.0, &boxes);
        assert!(ground.is_some_and(|g| g.abs() < 0.01), "{name}: 시작 자리 아래에 바닥이 없다 {ground:?}");
    }
}

// 정글의 문과 NPC 들은 걸어서 닿을 수 있는 거리에 조준될 수 있어야 한다(문 앞 자리에서 문이 조준권 안).
#[test]
fn gates_are_aimable_from_the_front() {
    use crate::render::mesh3d::Camera;
    for gate in [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs, Gate::Back] {
        let front = floor2::gate_front(gate);
        let cam = Camera { pos: [front[0], 1.55, front[2] - 0.2], yaw: 0.0, pitch: 0.0 }; // 문 쪽(-Z)을 본다
        let c = gate.center();
        assert!(aim_score(&cam, c).is_some(), "{gate:?}: 문 앞에서 문을 조준할 수 없다");
    }
}

// 장면은 모양이 바뀔 때(문이 올라감, 진행 상태, 장소)만 다시 만든다 — 아무 일도 없으면 키가 그대로다.
#[test]
fn the_scene_is_rebuilt_only_when_its_shape_changes() {
    let mut app = in_jungle();
    let key = app.scene_key();
    assert_eq!(app.scene_key(), key, "아무 일도 없으면 같은 장면");
    app.clock += 5.0;
    assert_eq!(app.scene_key(), key, "시간이 흘러도 정글은 그대로(뇌의 방 풍경만 시간을 쓴다)");

    app.progress.exchanged = true;
    assert_ne!(app.scene_key(), key, "교환하면 소파/봉투 모양이 바뀐다");

    let jungle = app.build_scene(key);
    assert!(jungle.jungle_items.iter().any(|it| it.target == floor2::Target::Sofa) && !jungle.shown.is_empty());
    assert!(jungle.collide.len() > jungle.shown.len(), "충돌 목록에는 눈에 안 보이는 벽이 더 있다");

    let mut room = DoorsGameApp::new(DoorsProgress { place: MainPlace::MissionRoom(MiniKind::Brain), errand_accepted: true, tutorial_done: true, ..DoorsProgress::default() });
    let before = room.scene_key();
    room.clock += 0.2;
    assert_ne!(room.scene_key(), before, "뇌의 방 창밖 풍경은 시간에 따라 바뀐다");
}

// 다시 조사하면 나오는 숨은 한 줄들 — 조건을 채우기 전에는 없고, 채우면 한두 줄이 더 붙는다.
#[test]
fn hidden_lines_appear_only_after_their_conditions() {
    let lines_after = |progress: DoorsProgress, f: &dyn Fn(&mut DoorsGameApp)| {
        let mut app = DoorsGameApp::new(progress);
        quiet(&mut app);
        f(&mut app);
        app.dialogue.entry_count()
    };
    // 정글 표지판 — 계단 NPC 와 이야기한 뒤 뒷면 글씨가 두 줄 더 붙는다.
    let sign = |app: &mut DoorsGameApp| app.interact_jungle(floor2::Target::Sign);
    let jungle = DoorsProgress { place: MainPlace::Jungle, ..DoorsProgress::default() };
    assert_eq!(lines_after(jungle.clone(), &sign), 2);
    assert_eq!(lines_after(DoorsProgress { stair_met: true, ..jungle }, &sign), 4);
    // 시작방 책상 — 문에게 꽃을 준 뒤 한 줄이 더 붙는다.
    let desk = |app: &mut DoorsGameApp| app.interact_room(Some(Target::Prop(0)));
    assert_eq!(lines_after(DoorsProgress::default(), &desk), 1);
    assert_eq!(lines_after(DoorsProgress { flower_given: true, ..DoorsProgress::default() }, &desk), 2);
    // 화장실 거울 — 기사의 일을 다 마친 뒤엔 다른 두 줄이 나온다.
    let mirror = |app: &mut DoorsGameApp| app.interact_spot(Spot::Fixture(2));
    let bath = |step: MissionStep| {
        let mut missions = [MissionStep::Start; 3];
        missions[MiniKind::Finger.index()] = step;
        DoorsProgress { place: MainPlace::MissionRoom(MiniKind::Finger), errand_accepted: true, missions, ..DoorsProgress::default() }
    };
    assert_eq!(lines_after(bath(MissionStep::Start), &mirror), 1);
    assert_eq!(lines_after(bath(MissionStep::Done), &mirror), 2);
}

// 2층을 처음 끝내면 대사가 끝난 뒤 엔딩 카드가 뜨고 "계속됩니다" 메일이 가며, 키로 닫는다. 다시 와도 또 뜨지 않는다.
#[test]
fn finishing_floor_two_shows_the_end_card_once() {
    let progress = DoorsProgress { place: MainPlace::StairHall, chip_fed: true, ..DoorsProgress::default() };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);
    app.unfold = 1.0;
    app.interact_spot(Spot::StairStep);
    assert!(app.progress.floor2_done && app.pending_end_card);
    // 대사가 아직 떠 있는 동안은 카드가 안 뜬다.
    app.update_end_card(0.1);
    assert!(app.end_card.is_none() && app.pending_mail.is_none());
    quiet(&mut app); // 대사가 끝났다
    app.update_end_card(0.1);
    assert_eq!(app.end_card, Some(0.0));
    assert_eq!(app.pending_mail, Some(MailId::End2F));
    // 막 떴을 때는 키로 닫히지 않고, 충분히 지난 뒤에 닫힌다.
    assert!(!app.dismiss_end_card(true));
    app.update_end_card(END_CARD_MIN_SECS);
    assert!(!app.dismiss_end_card(false), "키를 안 눌렀으면 그대로");
    assert!(app.dismiss_end_card(true));
    assert!(app.end_card.is_none());
    // 두 번째로 계단 첫 칸을 조사해도 카드가 또 뜨지 않는다.
    quiet(&mut app);
    app.pending_mail = None;
    app.interact_spot(Spot::StairStep);
    quiet(&mut app);
    app.update_end_card(0.1);
    assert!(app.end_card.is_none() && app.pending_mail.is_none());
}

// 플레이어가 고친 game.ini 에 문이 한마디 하지만, 같은 고침에는 한 번만이고 진행은 바뀌지 않는다.
#[test]
fn the_door_comments_on_an_edited_ini_once() {
    use crate::gamefiles::{IniEdit, IniMood};
    let mut app = DoorsGameApp::new(DoorsProgress::default());
    quiet(&mut app);
    let before = app.progress.clone();
    assert!(app.ini_reaction(IniEdit::None).is_none());
    let lines = app.ini_reaction(IniEdit::FakeFlowers).expect("가짜 꽃에는 문이 말한다");
    assert_eq!(lines.len(), 4);
    assert!(app.ini_reaction(IniEdit::FakeFlowers).is_none(), "같은 고침에는 한 번만");
    assert!(app.ini_reaction(IniEdit::Mood(IniMood::Hopeful)).is_some(), "다른 고침에는 또 말한다");
    assert!(app.ini_reaction(IniEdit::UnknownMood).is_some());
    // 원래대로 되돌리면(None) 다음에 또 고쳤을 때 다시 말한다.
    assert!(app.ini_reaction(IniEdit::None).is_none());
    assert!(app.ini_reaction(IniEdit::UnknownMood).is_some());
    assert_eq!(app.progress, before, "문은 속지 않는다 — 진행은 그대로다");
}

