//! 메인 게임(DoorsGameApp)의 진행/장면 이동/상호작용 테스트.

use crate::apps::game3d::player::aim_dist;
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

// 재료 셋 → 소파에게 교환 → 봉투(Mr. POTATO CHIP) → 계단에게 먹임 → 계단이 펼쳐지고 2층이 끝난다.
#[test]
fn three_ingredients_become_a_chip_that_feeds_the_stairs() {
    let progress = DoorsProgress {
        errand_accepted: true,
        stair_met: true,
        tutorial_done: true,
        place: MainPlace::Jungle,
        jungle_visited: true,
        missions: [MissionStep::Received, MissionStep::Done, MissionStep::Received],
        ..DoorsProgress::default()
    };
    let mut app = DoorsGameApp::new(progress);
    quiet(&mut app);

    // 둘만 있으면 부족한 걸 알려준다(가진 재료는 그대로).
    app.progress.missions[2] = MissionStep::Talked;
    app.interact_jungle(floor2::Target::Sofa);
    assert!(app.ask.is_none() && app.ingredients() == 2);
    app.progress.missions[2] = MissionStep::Received;

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
        ("시작방", SPAWN, build_room(0.0)),
        ("정글(시작방 문 앞)", floor2::SPAWN, jungle.clone()),
        ("계단 입구", rooms::SPAWN, rooms::stair_hall(false, 0.0).boxes),
    ];
    for gate in [Gate::Eye, Gate::Brain, Gate::Finger, Gate::Stairs] {
        cases.push(("정글(문 앞)", floor2::gate_front(gate), jungle.clone()));
    }
    for kind in MiniKind::ALL {
        let room = rooms::mission_room(kind, false, false, 0.0);
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
        assert!(aim_dist(&cam, c).is_some(), "{gate:?}: 문 앞에서 문을 조준할 수 없다");
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
