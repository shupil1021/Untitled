//! 창 안 3D 게임들(doors_game / maze_game)의 대사·조사 안내 번역. 게임 코드는 대사를 한국어(또는 `[E] ...`
//! 영어 안내)로 적어 두고, 화면에 나가기 직전(Dialogue::start / View::draw_prompt / 인벤토리 그리기)에 여기서
//! 지금 언어의 문구로 바꾼다. 표에 없는 문구는 그대로 나간다 — 그래서 표에 빠진 문구가 없는지는 아래 테스트가
//! 소스 파일을 훑어서 잡고, 일본어에 쓴 한자가 폰트 아틀라스(gfx::KANJI_CHARSET)에 있는지도 같이 확인한다.
//!
//! 현재 언어는 main.rs 가 프레임마다 `set_language` 로 알려 준다(대화가 시작될 때의 언어로 고정된다).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use crate::foundation::Language;

static LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_language(lang: Language) {
    LANG.store(
        match lang {
            Language::En => 0,
            Language::Ko => 1,
            Language::Ja => 2,
        },
        Ordering::Relaxed,
    );
}

pub fn language() -> Language {
    match LANG.load(Ordering::Relaxed) {
        1 => Language::Ko,
        2 => Language::Ja,
        _ => Language::En,
    }
}

// (한국어, 영어, 일본어) — 한국어나 영어 중 어느 쪽 문구로 찾아도 된다(게임 코드가 두 가지로 적혀 있다).
type Row = (&'static str, &'static str, &'static str);

fn index() -> &'static HashMap<&'static str, &'static Row> {
    static INDEX: OnceLock<HashMap<&'static str, &'static Row>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut m = HashMap::new();
        for row in TABLE {
            m.insert(row.0, row);
            m.insert(row.1, row);
        }
        m
    })
}

// 표에 있는 문구면 지금 언어의 것을, 없으면 None.
pub fn lookup(key: &str) -> Option<&'static str> {
    index().get(key).map(|r| match language() {
        Language::En => r.1,
        Language::Ko => r.0,
        Language::Ja => r.2,
    })
}

// 표에 없으면 받은 그대로 돌려준다.
pub fn tr(key: &str) -> String {
    lookup(key).map(str::to_string).unwrap_or_else(|| key.to_string())
}

pub fn tr_static(key: &'static str) -> &'static str {
    lookup(key).unwrap_or(key)
}

// `{}` 자리가 있는 문구 — 틀(template)을 번역한 뒤 자리마다 args(이것도 번역한다)를 차례로 끼운다.
pub fn fmt(template: &str, args: &[&str]) -> String {
    let mut out = tr(template);
    for a in args {
        out = out.replacen("{}", &tr(a), 1);
    }
    out
}

pub static TABLE: &[Row] = &[
    // ---- 조사 안내(정글/방/미로) ----
    ("[E] 문 (눈)", "[E] Door (eye)", "[E] ドア(目)"),
    ("[E] 문 (뇌)", "[E] Door (brain)", "[E] ドア(脳)"),
    ("[E] 문 (손가락)", "[E] Door (finger)", "[E] ドア(指)"),
    ("[E] 문 (계단)", "[E] Door (stairs)", "[E] ドア(階段)"),
    ("[E] 문 (방)", "[E] Door (the room)", "[E] ドア(部屋)"),
    ("[E] 표지판", "[E] Sign", "[E] 看板"),
    ("[E] 토끼", "[E] Rabbit", "[E] ウサギ"),
    ("[E] 봉투", "[E] Envelope", "[E] 封筒"),
    ("[E] 나가기", "[E] Go out", "[E] 外へ出る"),
    ("[E] 조사", "[E] Examine", "[E] 調べる"),
    ("[E] 조사: {}", "[E] Examine {}", "[E] 調べる: {}"),
    ("[E] 편지 줍기", "[E] Take letter", "[E] 手紙を拾う"),
    ("[E] 우편함", "[E] Mailbox", "[E] ポスト"),
    ("[E] 출구", "[E] Exit", "[E] 出口"),
    ("[E] 말 걸기", "[E] Talk", "[E] 話しかける"),
    ("[E] 아케이드 기계", "[E] Arcade machine", "[E] アーケード機"),
    ("[E] 운전대 기계", "[E] Driving machine", "[E] ハンドル機"),
    ("[E] 금고", "[E] Vault", "[E] 金庫"),
    ("[E] 변기", "[E] Toilet", "[E] 便器"),
    ("[E] 욕조", "[E] Bathtub", "[E] 浴槽"),
    ("[E] 세면대", "[E] Sink", "[E] 洗面台"),
    ("[E] 거울", "[E] Mirror", "[E] 鏡"),
    ("[E] 계단", "[E] Stairs", "[E] 階段"),
    ("[E] 물뿌리개", "[E] Watering can", "[E] じょうろ"),
    ("[E] 열쇠", "[E] Key", "[E] 鍵"),
    ("[E] 밧줄", "[E] Rope", "[E] ロープ"),
    ("[E] 화분", "[E] Flower pot", "[E] 植木鉢"),
    ("[E] 사물함", "[E] Locker", "[E] ロッカー"),
    ("[E] 꽃", "[E] Flower", "[E] 花"),
    ("[E] 문 1", "[E] Door 1", "[E] ドア 1"),
    ("[E] 문 2", "[E] Door 2", "[E] ドア 2"),
    ("[E] 문 3", "[E] Door 3", "[E] ドア 3"),
    ("책상", "Desk", "机"),
    ("나무 상자", "Crate", "木箱"),
    ("선반", "Shelf", "棚"),
    ("쓰레기", "Trash", "ごみ"),
    // ---- 선택지 / 화면 문구 ----
    ("[Y] 수락   [N] 거부", "[Y] Accept   [N] Refuse", "[Y] 承諾   [N] 拒否"),
    ("[Y] 먹인다   [N] 나중에", "[Y] Feed it   [N] Later", "[Y] 食べさせる   [N] あとで"),
    ("[Y] 교환하기   [N] 나중에", "[Y] Trade   [N] Later", "[Y] 交換する   [N] あとで"),
    ("[Y] 다시 받기   [N] 괜찮아", "[Y] Get it again   [N] Never mind", "[Y] もう一度もらう   [N] いいよ"),
    ("클릭해서 시점 조작 (Esc: 해제 / Shift: 달리기)", "Click to look around (Esc: release / Shift: run)", "クリックで視点操作(Esc: 解除 / Shift: 走る)"),
    // ---- 인벤토리 / 이름표 ----
    ("보리지꽃", "Borage flower", "ボリジの花"),
    ("금고 절반", "Half a vault", "金庫の半分"),
    ("변기 손잡이", "Toilet handle", "便器のハンドル"),
    ("충혈된 눈", "Bloodshot eye", "充血した目"),
    ("소환 조각(레이싱)", "Summon fragment (racing)", "召喚の欠片(レース)"),
    ("소환 조각(리듬)", "Summon fragment (rhythm)", "召喚の欠片(リズム)"),
    ("눈 표식 문", "the door with the eye mark", "目の印のドア"),
    ("금고가 보이는 문", "the door with the vault", "金庫が見えるドア"),
    ("화장실 문", "the bathroom door", "トイレのドア"),
    ("물뿌리개", "Watering can", "じょうろ"),
    ("씨앗", "Seed", "種"),
    ("열쇠 {}/{}", "Keys {}/{}", "鍵 {}/{}"),
    ("재료 {}/{}", "Ingredients {}/{}", "材料 {}/{}"),
    // ---- 플레이어가 고친 game.ini 에 대한 문의 반응 ----
    ("설정 파일에 방에 꽃이 있다고 적혀 있네.", "The config file says there's a flower in the room.", "設定ファイルに、部屋に花があると書いてある。"),
    ("...냄새가 안 나. 가짜야.", "...It doesn't smell like one. It's fake.", "...においがしない。偽物だよ。"),
    ("진짜 꽃이 필요해.", "I need a real flower.", "本物の花がほしいんだ。"),
    ("설정 파일에 내 기분이 '{}'(이)라고 적혀 있네.", "The config file says I'm '{}'.", "設定ファイルに、私は「{}」だと書いてある。"),
    ("내 기분은 내가 정해.", "I decide how I feel.", "私の気持ちは私が決める。"),
    ("설정 파일에 모르는 말이 적혀 있네.", "The config file has a word I don't know.", "設定ファイルに、知らない言葉が書いてある。"),
    ("나는 그런 기분이 아니야.", "That isn't how I feel.", "そんな気分じゃないよ。"),
    ("쓸쓸하다", "lonely", "寂しい"),
    ("기다리는 중이다", "waiting", "待っている"),
    ("기대하고 있다", "hopeful", "期待している"),
    ("만족했다", "satisfied", "満足した"),
    // ---- 2층 엔딩 카드 ----
    ("2층 끝", "End of 2F", "2階おわり"),
    ("이 아래는 아직 준비 중이에요.", "The way down isn't ready yet.", "この下はまだ準備中です。"),
    ("아무 키나 눌러 닫기", "Press any key to close", "何かキーを押して閉じる"),
    // ---- 시작방 / 도어즈 ----
    ("낡은 방 안이다.", "It's an old room.", "古びた部屋の中だ。"),
    ("정면에 문이 하나 있다.", "There's a door straight ahead.", "正面にドアが一つある。"),
    ("문고리에서 낮은 목소리가 들려왔다.", "A low voice came from the doorknob.", "ドアノブから低い声が聞こえてきた。"),
    ("보리지꽃을 줘.", "Give me a borage flower.", "ボリジの花をちょうだい。"),
    ("문 너머에서 작은 한숨이 새어 나왔다.", "A small sigh leaked from behind the door.", "ドアの向こうから小さなため息がもれた。"),
    ("그러고 보니 책상 위에 편지가 한 통 놓여 있다.", "Now that I look, there's a letter on the desk.", "そういえば、机の上に手紙が一通置いてある。"),
    ("문은 아무 말도 하지 않았다.", "The door said nothing.", "ドアは何も言わなかった。"),
    ("문 쪽에서 인기척이 느껴진다...", "Something stirs on the other side of the door...", "ドアの方から気配を感じる..."),
    ("어디선가 보낸 꽃이 도착했다.", "A flower sent from somewhere has arrived.", "どこかから送られた花が届いた。"),
    ("손에 보리지꽃이 쥐어져 있다.", "A borage flower is in my hand.", "手にボリジの花が握られている。"),
    ("문고리 앞에 보리지꽃을 내밀었다.", "I held the borage flower out to the doorknob.", "ドアノブの前にボリジの花を差し出した。"),
    ("꽃은 받았어. 바깥에서도 이름을 잘 기억해.", "I got the flower. Remember your name out there, too.", "花は受け取ったよ。外でも名前をちゃんと覚えていてね。"),
    ("철컥, 하고 문 안쪽에서 자물쇠가 풀리는 소리가 났다.", "Click — the lock released on the other side of the door.", "カチャッと、ドアの内側で鍵が外れる音がした。"),
    ("편지는 잘 읽었어.", "I read your letter.", "手紙はちゃんと読んだよ。"),
    ("꽃은... 조금만 더 기다려 줘.", "The flower... wait a little longer.", "花は...もう少しだけ待ってて。"),
    ("방에는 꽃이 없는 것 같다...", "There doesn't seem to be a flower in this room...", "この部屋には花がないようだ..."),
    ("그때 어디선가 희미한 알림음이 울렸다.", "Then a faint notification sound rang from somewhere.", "そのとき、どこかでかすかな通知音が鳴った。"),
    ("편지를 집어 들었다.", "I picked up the letter.", "手紙を手に取った。"),
    ("편지를 우편함에 넣었다.", "I put the letter in the mailbox.", "手紙をポストに入れた。"),
    ("어디선가 알림음이 울린다...", "A notification sound rings from somewhere...", "どこかで通知音が鳴っている..."),
    ("우편함은 비어 있다.", "The mailbox is empty.", "ポストは空っぽだ。"),
    ("우편함이 있다. 지금은 넣을 게 없다.", "There's a mailbox. I have nothing to put in it right now.", "ポストがある。今は入れるものがない。"),
    ("낡은 책상이다. 서랍은 텅 비어 있다.", "An old desk. The drawer is completely empty.", "古い机だ。引き出しは空っぽだ。"),
    ("먼지 쌓인 나무 상자다. 뚜껑이 못으로 박혀 있다.", "A dusty wooden crate. The lid is nailed shut.", "ほこりをかぶった木箱だ。ふたは釘で打ちつけられている。"),
    ("빈 선반이다. 먼지 자국만 남아 있다.", "An empty shelf. Only dust marks remain.", "空の棚だ。ほこりの跡だけが残っている。"),
    ("구겨진 종이와 쓰레기가 쌓여 있다.", "Crumpled paper and trash are piled up.", "くしゃくしゃの紙とごみが積まれている。"),
    ("열린 문 너머로 환한 빛이 새어 들어온다.", "Bright light spills in through the open door.", "開いたドアの向こうから明るい光がさしこんでくる。"),
    ("그 빛 속으로 걸어 나갔다...", "I walked out into the light...", "その光の中へ歩み出た..."),
    // ---- 정글 ----
    ("문 밖은 도시 한복판의 정글이었다.", "Outside the door was a jungle in the middle of the city.", "ドアの外は、街のど真ん中にあるジャングルだった。"),
    ("빌딩 사이로 덩굴과 나무가 우거져 있다.", "Vines and trees grow thick between the buildings.", "ビルのあいだに、つると木が生い茂っている。"),
    ("길 한복판에 문들이 덩그러니 서 있다.", "Doors stand all alone in the middle of the road.", "道の真ん中にドアがぽつんと立っている。"),
    ("낡은 표지판이다. 글씨가 거의 지워졌다.", "An old signpost. The writing has almost worn away.", "古い看板だ。文字はほとんど消えかけている。"),
    ("'어디로든 문'이라고 쓰여 있다.", "It says 'Anywhere Door'.", "「どこでもドア」と書いてある。"),
    ("방금 나온 문이다. 안쪽으로 낡은 방이 보인다.", "The door I just came out of. I can see the old room inside.", "今出てきたドアだ。中に古びた部屋が見える。"),
    ("문 안으로 되돌아간다...", "I go back through the door...", "ドアの中へ引き返す..."),
    ("문은 굳게 닫혀 있다.", "The door is shut tight.", "ドアは固く閉ざされている。"),
    ("지금은 열 수 없다. 소파에게 물어보면 알 수 있을까?", "I can't open it now. Maybe the sofa knows something?", "今は開けられない。ソファに聞けばわかるだろうか?"),
    ("봉투를 열었다. 안에서 과자 봉지가 얼굴을 내밀었다.", "I opened the envelope. A snack bag poked its face out.", "封筒を開けた。中からお菓子の袋が顔をのぞかせた。"),
    ("안녕하세요. 저는 간식입니다.", "Hello. I am a snack.", "こんにちは。私はおやつです。"),
    ("'Mr. POTATO CHIP'을 얻었다.", "Got 'Mr. POTATO CHIP'.", "「Mr. POTATO CHIP」を手に入れた。"),
    ("TV 토끼가 기지개를 켠다.", "The TV rabbit stretches.", "テレビウサギがのびをする。"),
    ("맛있게 먹여 줘.", "Feed it nicely.", "おいしく食べさせてあげて。"),
    ("소파 위의 봉투를 가져가.", "Take the envelope on the sofa.", "ソファの上の封筒を持っていって。"),
    ("TV 토끼 화면의 눈이 가늘어진다.", "The eyes on the TV rabbit's screen narrow.", "テレビウサギの画面の目が細くなる。"),
    ("계단 쪽에서 왔어? 아직 아무 얘기도 못 들은 얼굴인데.", "Came from the stairs? You look like you haven't heard anything yet.", "階段の方から来たの? まだ何も聞いてない顔だね。"),
    ("Mr. POTATO CHIP? 있지. 공짜는 아니야.", "Mr. POTATO CHIP? I have it. It isn't free.", "Mr. POTATO CHIP? あるよ。ただじゃないけどね。"),
    ("충혈된 눈 하나, 소환 조각 둘. 소환 조각은 레이싱 쪽과 리듬 쪽, 하나씩이야.", "One bloodshot eye, two summon fragments. One racing, one rhythm.", "充血した目を一つ、召喚の欠片を二つ。欠片はレースとリズムのが一つずつだよ。"),
    ("의뢰를 받아들일래?", "Will you take the request?", "依頼を引き受ける?"),
    ("전부 가져왔어.", "I brought everything.", "全部持ってきたよ。"),
    ("눈, 레이싱 조각, 리듬 조각... 하나씩 다 있다.", "The eye, the racing fragment, the rhythm fragment... I have one of each.", "目、レースの欠片、リズムの欠片...一つずつ全部ある。"),
    ("Mr. POTATO CHIP으로 바꿀까?", "Trade them for Mr. POTATO CHIP?", "Mr. POTATO CHIPと交換する?"),
    ("아직 모자라.", "Not enough yet.", "まだ足りないよ。"),
    ("{}이 필요해. {}으로 가 봐.", "I need {}. Try {}.", "{}が必要だよ。{}へ行ってみて。"),
    ("소파가 느긋하게 몸을 묻었다.", "The sofa settled back comfortably.", "ソファはゆったりと身を沈めた。"),
    ("멀리서 문 세 개가 열리는 소리가 들렸다.", "From afar came the sound of three doors opening.", "遠くでドアが三つ開く音が聞こえた。"),
    ("소파는 느긋하게 기다리고 있다.", "The sofa is waiting patiently.", "ソファはのんびり待っている。"),
    ("눈과 소환 조각 둘을 한꺼번에 소파에게 건넸다.", "I handed the eye and both summon fragments to the sofa.", "目と召喚の欠片二つをまとめてソファに渡した。"),
    ("소파의 빈 자리가 한 칸 늘어났다.", "The empty seat on the sofa grew by one.", "ソファの空席が一つ増えた。"),
    ("소파 위에 봉투가 하나 놓여 있다.", "There's an envelope on the sofa.", "ソファの上に封筒が一つ置いてある。"),
    ("소파가 하품을 한다.", "The sofa yawns.", "ソファがあくびをする。"),
    ("다시 받기", "Get again", "もらい直す"),
    ("게임 파일을 다시 받을까?", "Get the game file again?", "ゲームファイルをもう一度もらう?"),
    ("기사가 보낸 게임 파일을 다시 받을까?", "Get the game file the knight sent again?", "騎士が送ったゲームファイルをもう一度もらう?"),
    ("다시 한 번 받았다.", "I got it once more.", "もう一度もらった。"),
    ("컴퓨터에서 확인해 보자.", "Let's check on the computer.", "パソコンで確認してみよう。"),
    ("괜찮다고 했다.", "I said it was fine.", "大丈夫だと言った。"),
    ("기사는 무겁게 한숨을 쉬었다.", "The knight sighed heavily.", "騎士は重いため息をついた。"),
    // ---- 눈의 방 ----
    ("렌즈 얼굴이 천천히 이쪽을 돌아본다.", "The lens face slowly turns toward me.", "レンズの顔がゆっくりこちらを振り向く。"),
    ("계속 피하기만 하면 돼. 내가 대신 깜빡여 줄게.", "All you have to do is keep dodging. I'll blink for you.", "ずっと避けるだけでいいよ。かわりにまばたきしてあげる。"),
    ("그는 구석의 아케이드 기계를 가리켰다.", "He pointed at the arcade machine in the corner.", "彼は隅のアーケード機を指さした。"),
    ("저 기계에서 게임을 받아.", "Get the game from that machine.", "あの機械でゲームをもらって。"),
    ("아케이드 기계는 꺼져 있다.", "The arcade machine is off.", "アーケード機は消えている。"),
    ("먼저 렌즈 얼굴과 이야기해 보자.", "Let's talk to the lens face first.", "まずレンズの顔と話してみよう。"),
    ("아케이드 기계의 '게임 받기'를 눌렀다.", "I pressed 'Get game' on the arcade machine.", "アーケード機の「ゲームをもらう」を押した。"),
    ("이미 게임을 받았다.", "I already got the game.", "もうゲームはもらった。"),
    ("어디선가 보낸 충혈된 눈이 손에 쥐어져 있다.", "A bloodshot eye sent from somewhere is in my hand.", "どこかから送られた充血した目が手に握られている。"),
    ("'충혈된 눈'을 얻었다.", "Got the 'Bloodshot eye'.", "「充血した目」を手に入れた。"),
    ("이제 나는 쉬어도 되겠네.", "Guess I can rest now.", "これで私は休めるね。"),
    ("렌즈가 꺼졌는데, 시선은 출구를 따라온다.", "The lens is off, yet its gaze follows me to the exit.", "レンズは消えたのに、視線は出口までついてくる。"),
    ("...쉬는 중이야.", "...I'm resting.", "...休んでるところだよ。"),
    ("아케이드 기계는 조용하다.", "The arcade machine is quiet.", "アーケード機は静かだ。"),
    // ---- 금고의 방 ----
    ("바닥의 도로 위에 표지판이 서 있다.", "A signpost stands on the road painted on the floor.", "床の道路の上に看板が立っている。"),
    ("집까지 가는 길이야. 이 길 끝에 금고의 나머지 절반이 있어.", "This is the way home. The other half of the vault is at the end of this road.", "家までの道だよ。この道の先に金庫の残り半分がある。"),
    ("표지판이 운전대 모양의 기계 쪽을 가리킨다.", "The signpost points toward a steering-wheel-shaped machine.", "看板がハンドルの形をした機械の方を指している。"),
    ("운전대 기계는 꺼져 있다.", "The driving machine is off.", "ハンドル機は消えている。"),
    ("먼저 표지판과 이야기해 보자.", "Let's talk to the signpost first.", "まず看板と話してみよう。"),
    ("운전대 기계의 '게임 받기'를 눌렀다.", "I pressed 'Get game' on the driving machine.", "ハンドル機の「ゲームをもらう」を押した。"),
    ("어디선가 보낸 금고 절반이 손에 쥐어져 있다.", "Half a vault sent from somewhere is in my hand.", "どこかから送られた金庫の半分が手に握られている。"),
    ("'금고 절반'을 얻었다.", "Got 'Half a vault'.", "「金庫の半分」を手に入れた。"),
    ("방에 서 있는 금고 반쪽에 맞춰 보자.", "Let's fit it to the half-vault standing in the room.", "部屋に立っている金庫の片方に合わせてみよう。"),
    ("돌아왔는데, 정말 같은 곳일까?", "I'm back, but is this really the same place?", "戻ってきたけど、本当に同じ場所だろうか?"),
    ("창밖 풍경이 멈췄다. 방은 그대로다.", "The scenery outside the window stopped. The room is unchanged.", "窓の外の景色が止まった。部屋はそのままだ。"),
    ("...여기가 집이었나.", "...Was this home?", "...ここが家だったのかな。"),
    ("운전대 기계는 조용하다.", "The driving machine is quiet.", "ハンドル機は静かだ。"),
    ("받은 금고 절반을 방의 금고에 맞춰 봐.", "Fit the half vault you received to the vault in the room.", "受け取った金庫の半分を、部屋の金庫に合わせてみて。"),
    ("이제 열 수 있어. 다이얼을 돌려 봐.", "Now it can be opened. Turn the dial.", "もう開けられるよ。ダイヤルを回してみて。"),
    ("금고가 한쪽 절반만 서 있다.", "Only half of the vault is standing.", "金庫は片方の半分しか立っていない。"),
    ("잘려 나간 단면 너머로 속이 텅 비어 보인다.", "Past the cut surface, the inside looks hollow.", "切り取られた断面の向こうは、中がからっぽに見える。"),
    ("나머지 절반이 필요하다.", "The other half is needed.", "残りの半分が必要だ。"),
    ("받은 금고 절반을 잘려 나간 면에 맞췄다.", "I fitted the half vault to the cut surface.", "受け取った金庫の半分を切り取られた面に合わせた。"),
    ("쿵, 하고 두 절반이 하나로 붙었다.", "Thud — the two halves joined into one.", "ドスンと、二つの半分がひとつにくっついた。"),
    ("금고가 온전해졌다.", "The vault is whole.", "金庫が元の形に戻った。"),
    ("다이얼을 돌리자 육중한 금고 문이 열렸다.", "I turned the dial and the heavy vault door opened.", "ダイヤルを回すと、重い金庫の扉が開いた。"),
    ("안쪽에 작은 조각이 하나 들어 있다.", "A small fragment is inside.", "中に小さな欠片が一つ入っている。"),
    ("'소환 조각'(레이싱)을 얻었다.", "Got a 'Summon fragment' (racing).", "「召喚の欠片」(レース)を手に入れた。"),
    ("금고는 텅 비어 있다.", "The vault is empty.", "金庫はからっぽだ。"),
    // ---- 화장실 ----
    ("갑옷 차림의 기사가 변기 앞을 서성이고 있다.", "A knight in armor is pacing in front of the toilet.", "鎧姿の騎士が便器の前をうろうろしている。"),
    ("내 칼을 잃어버려서 화장실을 못 쓰겠소.", "I lost my sword, so I cannot use the restroom.", "剣をなくしてしまい、トイレが使えぬのだ。"),
    ("내 칼을 찾아주시오.", "Please find my sword.", "私の剣を探してくれ。"),
    ("기사의 의뢰를 받아들일래?", "Will you take the knight's request?", "騎士の依頼を引き受ける?"),
    ("이 게임의 끝에 나의 칼이 있소.", "My sword lies at the end of this game.", "このゲームの最後に私の剣がある。"),
    ("...찾아주시오.", "...Please find it.", "...探してくれ。"),
    ("손이 내민 변기 손잡이를 받았다.", "I took the toilet handle the hand held out.", "手が差し出した便器のハンドルを受け取った。"),
    ("손은 다시 변기 속으로 들어갔다.", "The hand went back down into the toilet.", "手はまた便器の中へ戻っていった。"),
    ("'변기 손잡이'를 얻었다.", "Got the 'Toilet handle'.", "「便器のハンドル」を手に入れた。"),
    ("기사가 변기 손잡이를 쓰다듬는다.", "The knight strokes the toilet handle.", "騎士が便器のハンドルをなでる。"),
    ("나의 칼이 돌아왔소. 이제 화장실을 쓸 수 있겠군.", "My sword has returned. Now I can use the restroom.", "私の剣が戻ってきた。これでトイレが使えるな。"),
    ("기사는 만족스럽게 변기 앞에 자리를 잡았다.", "The knight settled in front of the toilet, satisfied.", "騎士は満足そうに便器の前に陣取った。"),
    ("...사용 중이오.", "...It is occupied.", "...使用中だ。"),
    ("내 칼이 돌아왔군! 그 칼을 변기에 꽂아 주시오.", "My sword has returned! Please plunge it into the toilet.", "私の剣が戻ったか! その剣を便器に挿してくれ。"),
    ("물을 내려 보시오. 칼의 힘이 깨어날 것이오.", "Flush it. The sword's power will awaken.", "水を流してみてくれ。剣の力が目覚めるだろう。"),
    ("낡은 욕조다.", "An old bathtub.", "古い浴槽だ。"),
    ("물이 오래 고여 있던 자국이 남아 있다.", "Marks remain where water stood for a long time.", "水が長くたまっていた跡が残っている。"),
    ("세면대의 수도꼭지에서 물이 한 방울씩 떨어진다.", "Water drips one drop at a time from the sink's faucet.", "洗面台の蛇口から水が一滴ずつ落ちている。"),
    ("거울 속의 내가 잠깐 한 박자 늦게 움직인 것 같다.", "The me in the mirror seemed to move a beat late.", "鏡の中の私が、一拍遅れて動いた気がする。"),
    ("기사가 의뢰를 받아들이자 어디선가 뿌웅, 하는 소리가 울렸다.", "As the knight accepted the request, a loud toot rang out from somewhere.", "騎士が依頼を受け入れると、どこかでブーッと音が鳴った。"),
    ("[ 퀘스트 시작: 기사의 잃어버린 칼 ]", "[ Quest started: The Knight's Lost Sword ]", "[ クエスト開始: 騎士のなくした剣 ]"),
    ("(중세풍 나팔 소리가 울려 퍼졌다.)", "(A medieval trumpet fanfare rang out.)", "(中世風のラッパの音が鳴り響いた。)"),
    ("변기의 물 내리는 손잡이 자리가 비어 있다.", "The spot for the toilet's flush handle is empty.", "便器の水を流すハンドルの場所が空いている。"),
    ("먼저 기사와 이야기해 보자.", "Let's talk to the knight first.", "まず騎士と話してみよう。"),
    ("변기에는 손잡이가 달려 있어야 할 자리가 비어 있다.", "The toilet has an empty spot where a handle should be.", "便器には、ハンドルがあるはずの場所が空いている。"),
    ("구멍 속은 어둡고 축축하다.", "The hole is dark and damp.", "穴の中は暗くて湿っている。"),
    ("변기 속에서 물소리가 났다.", "A sound of water came from inside the toilet.", "便器の中から水の音がした。"),
    ("하얀 손이 불쑥 올라와 무언가를 내밀고 있다.", "A white hand suddenly rises up, holding something out.", "白い手がぬっと現れて、何かを差し出している。"),
    ("호수에 넣었던 변기 손잡이다.", "It's the toilet handle I put in the lake.", "湖に入れた便器のハンドルだ。"),
    ("비어 있던 자리에 변기 손잡이를 끼웠다.", "I fitted the toilet handle into the empty spot.", "空いていた場所に便器のハンドルをはめた。"),
    ("딸깍. 변기가 온전해졌다.", "Click. The toilet is whole.", "カチッ。便器が元通りになった。"),
    ("손잡이를 눌렀다.", "I pressed the handle.", "ハンドルを押した。"),
    ("쏴아아— 물이 소용돌이치며 내려간다.", "Whoooosh — the water swirls down.", "ザーッ— 水が渦を巻いて流れていく。"),
    ("변기 안에서 무언가가 떠올랐다.", "Something floated up in the toilet.", "便器の中で何かが浮かび上がった。"),
    ("물 위에 떠 있는 조각을 집어 들었다.", "I picked up the fragment floating on the water.", "水面に浮かぶ欠片を拾い上げた。"),
    ("'소환 조각'(리듬)을 얻었다.", "Got a 'Summon fragment' (rhythm).", "「召喚の欠片」(リズム)を手に入れた。"),
    ("물을 내려 보았다.", "I flushed it.", "水を流してみた。"),
    ("더는 아무것도 나오지 않는다.", "Nothing more comes out.", "もう何も出てこない。"),
    // ---- 계단 ----
    ("계단이 몸을 웅크린 채 길을 막고 있다.", "The stairs are crouched, blocking the way.", "階段は体を丸めて道をふさいでいる。"),
    ("계단 자체가 입을 열고 말했다.", "The staircase itself opened its mouth and spoke.", "階段そのものが口を開いて言った。"),
    ("배고파. Mr. POTATO CHIP이 먹고 싶어.", "I'm hungry. I want Mr. POTATO CHIP.", "おなかすいた。Mr. POTATO CHIPが食べたい。"),
    ("미스터가 붙은 것만 먹어. 예의가 있잖아.", "I only eat things with a 'Mr.' in front. It's good manners.", "ミスターがついているものしか食べないよ。礼儀があるからね。"),
    ("계단이 입을 벌리고 있다.", "The stairs have their mouth open.", "階段が口を開けている。"),
    ("빈손은 바삭하지 않아.", "An empty hand isn't crispy.", "からっぽの手はサクサクじゃないよ。"),
    ("Mr. POTATO CHIP이 먹고 싶어.", "I want Mr. POTATO CHIP.", "Mr. POTATO CHIPが食べたい。"),
    ("계단이 입을 쩍 벌렸다.", "The stairs opened wide.", "階段が大きく口を開けた。"),
    ("Mr. POTATO CHIP을 입에 넣어 주었다.", "I put Mr. POTATO CHIP in its mouth.", "Mr. POTATO CHIPを口に入れてあげた。"),
    ("계단이 바삭, 하고 한입에 먹었다.", "The stairs ate it in one crunchy bite.", "階段はサクッと一口で食べた。"),
    ("계단은 입을 벌린 채 기다린다.", "The stairs wait with their mouth open.", "階段は口を開けたまま待っている。"),
    ("계단: 잘 먹었습니다. 이제 내려가세요.", "Stairs: Thank you for the meal. Please go down now.", "階段: ごちそうさまでした。さあ、降りてください。"),
    ("계단이 아래층으로 길게 이어져 있다.", "The stairs stretch long toward the floor below.", "階段が下の階へ長くのびている。"),
    ("계단 아래에서 희미한 소리가 올라온다...", "A faint sound rises from below the stairs...", "階段の下からかすかな音がのぼってくる..."),
    ("아직 내려갈 준비가 되지 않았다.", "I'm not ready to go down yet.", "まだ降りる準備ができていない。"),
    // ---- 숨은 한 줄(다시 조사하면 나오는 것) ----
    ("뒷면에도 지워진 글씨가 있다.", "There's faded writing on the back, too.", "裏側にも消えかけた文字がある。"),
    ("'...나가는 문은 하나뿐이었다.'", "'...There was only one door out.'", "「...出口のドアはひとつだけだった。」"),
    ("거울 속의 내가 이번에는 정확히 같이 움직였다.", "This time the me in the mirror moved exactly in step.", "今度は鏡の中の私がぴったり同じに動いた。"),
    ("...그게 더 이상했다.", "...That was somehow stranger.", "...そのほうがかえって変だった。"),
    ("서랍 안쪽에 아주 작게 '고마워'라고 긁혀 있다.", "Scratched very small inside the drawer: 'thank you'.", "引き出しの内側に、とても小さく「ありがとう」と引っかいてある。"),
    // ---- 서브 게임(미로 / 화분 방) ----
    ("눈을 떠 보니 어두운 복도 안이다.", "I open my eyes to find myself in a dark corridor.", "目を開けると、暗い廊下の中だった。"),
    ("어딘가에 쓸 만한 물건이 있을 것 같다...", "There might be something useful somewhere...", "どこかに使えるものがありそうだ..."),
    ("...등 뒤에서 쿵, 하는 소리가 났다.", "...A thud sounded behind me.", "...背後でドスンと音がした。"),
    ("돌아보니 벽가에 사물함이 놓여 있다. 아까는 없었던 것 같은데.", "I turned around and there's a locker by the wall. I don't think it was there before.", "振り返ると、壁ぎわにロッカーが置いてある。さっきはなかった気がするけど。"),
    ("물뿌리개를 발견했다.", "I found a watering can.", "じょうろを見つけた。"),
    ("1라운드 클리어.", "Round 1 clear.", "ラウンド1クリア。"),
    ("그 순간 주위가 하얗게 흐려졌다...", "At that moment, everything around me faded to white...", "その瞬間、あたりが白くかすんだ..."),
    ("정신을 차려 보니 낯선 방 안이다.", "I came to and found myself in an unfamiliar room.", "気がつくと、見知らぬ部屋の中だった。"),
    ("한쪽에 화분이 하나 놓여 있다.", "A flower pot sits to one side.", "片隅に植木鉢が一つ置いてある。"),
    ("문 너머는 또 다른 어두운 복도였다.", "Beyond the door was another dark corridor.", "ドアの向こうは、また別の暗い廊下だった。"),
    ("열쇠 {}번을 얻었다.", "Got key number {}.", "鍵{}番を手に入れた。"),
    ("위에서 밧줄이 내려왔다.", "A rope came down from above.", "上からロープが下りてきた。"),
    ("밧줄을 타고 올라갔다...", "I climbed up the rope...", "ロープをつたって登っていった..."),
    ("화분에 씨앗을 심었다.", "I planted the seed in the pot.", "植木鉢に種を植えた。"),
    ("시간이 1년은 필요할 것 같다...", "It probably needs a year...", "時間が1年は必要そうだ..."),
    ("씨앗이 심겨 있다. 아직 아무 일도 없다.", "A seed is planted. Nothing has happened yet.", "種が植えてある。まだ何も起きていない。"),
    ("이미 누가 꺾어간 것 같다.", "Someone seems to have picked it already.", "もう誰かが摘んでいったようだ。"),
    ("새롭게 심어야 할 것 같은데...", "I think I'll have to plant a new one...", "新しく植えないといけないようだけど..."),
    ("열쇠로 자물쇠 3개를 풀었다.", "I opened the three locks with the keys.", "鍵で三つの錠を外した。"),
    ("씨앗을 얻었다.", "Got a seed.", "種を手に入れた。"),
    ("사물함은 비어 있다.", "The locker is empty.", "ロッカーは空っぽだ。"),
    ("잠겨 있다. 열쇠 3개가 필요할 것 같다.", "It's locked. I think I need three keys.", "鍵がかかっている。鍵が3つ必要そうだ。"),
    ("어디선가 문이 열리는 소리가 들렸다.", "I heard a door open somewhere.", "どこかでドアが開く音が聞こえた。"),
    ("열쇠가 부족하다. ({}/{})", "Not enough keys. ({}/{})", "鍵が足りない。({}/{})"),
    ("꽃을 꺾었다.", "I picked the flower.", "花を摘んだ。"),
    ("보리지꽃이다.", "It's a borage flower.", "ボリジの花だ。"),
    ("꽃을 우체통에 넣었다.", "I put the flower in the mailbox.", "花をポストに入れた。"),
    ("어딘가로 보내진 것 같다...", "It seems to have been sent somewhere...", "どこかへ送られたようだ..."),
    ("우체통은 비어 있다.", "The post box is empty.", "ポストは空っぽだ。"),
    ("우체통이다. 지금은 보낼 게 없다.", "A mailbox. I have nothing to send right now.", "ポストだ。今は送るものがない。"),
    ("잠겨 있다.", "It's locked.", "鍵がかかっている。"),
    ("이미 다녀온 문이다.", "I've already been through this door.", "もう行ってきたドアだ。"),
    ("문이 열려 있다.", "The door is open.", "ドアが開いている。"),
    ("안으로 들어간다...", "I go inside...", "中へ入っていく..."),
    ("화분에서 무언가 달라졌다...", "Something has changed in the pot...", "植木鉢の中で何かが変わった..."),
    ("꽃이 피었다.", "The flower has bloomed.", "花が咲いた。"),
];

#[cfg(test)]
mod tests {
    use super::*;

    // 이 파일들에 적힌 한국어 문구와 `[E]` 안내는 전부 표에 있어야 한다(테스트 모듈 안의 문자열은 건너뛴다).
    const SOURCES: &[(&str, &str)] = &[
        ("doors_game/interact.rs", include_str!("../doors_game/interact.rs")),
        ("doors_game/mod.rs", include_str!("../doors_game/mod.rs")),
        ("doors_game/rooms.rs", include_str!("../doors_game/rooms.rs")),
        ("doors_game/texts.rs", include_str!("../doors_game/texts.rs")),
        ("doors_game/travel.rs", include_str!("../doors_game/travel.rs")),
        ("doors_game/world.rs", include_str!("../doors_game/world.rs")),
        ("doors_game/floor2.rs", include_str!("../doors_game/floor2.rs")),
        ("maze_game/mod.rs", include_str!("../maze_game/mod.rs")),
        ("game3d/dialogue.rs", include_str!("dialogue.rs")),
        ("game3d/mod.rs", include_str!("mod.rs")),
    ];

    fn has_hangul(s: &str) -> bool {
        s.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c))
    }

    // 소스에서 `"..."` 문자열 리터럴만 뽑는다(주석 줄과 #[cfg(test)] 이후는 버린다).
    fn literals(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in src.lines() {
            if line.contains("#[cfg(test)]") {
                break;
            }
            if line.contains("eprintln!") {
                continue; // 개발자용 로그는 번역 대상이 아니다
            }
            let (mut cur, mut in_str, mut esc) = (String::new(), false, false);
            let chars: Vec<char> = line.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                let c = chars[i];
                if in_str {
                    if esc {
                        esc = false;
                        cur.push(c);
                    } else if c == '\\' {
                        esc = true;
                        cur.push(c);
                    } else if c == '"' {
                        in_str = false;
                        out.push(std::mem::take(&mut cur));
                    } else {
                        cur.push(c);
                    }
                } else if c == '"' {
                    in_str = true;
                } else if c == '/' && chars.get(i + 1) == Some(&'/') {
                    break; // 문자열 밖의 주석은 버린다
                }
                i += 1;
            }
        }
        out
    }

    #[test]
    fn every_game_text_is_translated() {
        // 대사가 아닌 한국어 문자열(패닉 메시지 등)은 여기 적어 둔다.
        let skip = ["내장 모델 OBJ 가 잘못됨", "방금 만들었다"];
        let mut missing = Vec::new();
        for (file, src) in SOURCES {
            for lit in literals(src) {
                let needs = has_hangul(&lit) || lit.starts_with("[E]");
                if needs && !skip.contains(&lit.as_str()) && !index().contains_key(lit.as_str()) {
                    missing.push(format!("{file}: {lit}"));
                }
            }
        }
        assert!(missing.is_empty(), "번역표에 없는 문구:\n{}", missing.join("\n"));
    }

    #[test]
    fn table_has_no_empty_or_mismatched_placeholders() {
        for (ko, en, ja) in TABLE {
            assert!(!ko.is_empty() && !en.is_empty() && !ja.is_empty(), "빈 번역: {ko}");
            let n = ko.matches("{}").count();
            assert_eq!(en.matches("{}").count(), n, "영어 자리표시자 개수: {ko}");
            assert_eq!(ja.matches("{}").count(), n, "일본어 자리표시자 개수: {ko}");
        }
    }

    #[test]
    fn no_duplicate_keys() {
        let mut seen = std::collections::HashSet::new();
        for (ko, en, _) in TABLE {
            assert!(seen.insert(*ko), "한국어 중복: {ko}");
            // 영어는 한국어와 같아도 안 되고(검색 충돌) 서로 겹쳐도 안 된다 — 단 같은 행의 ko == en 은 허용하지 않는다.
            assert!(seen.insert(*en) || en == ko, "영어 문구 중복: {en}");
        }
    }

    #[test]
    fn japanese_kanji_are_in_the_font_atlas() {
        let allowed = crate::render::gfx::KANJI_CHARSET;
        let mut missing = std::collections::BTreeSet::new();
        for (_, _, ja) in TABLE {
            for c in ja.chars() {
                let kana_or_symbol = c.is_ascii() || ('\u{3000}'..='\u{30FF}').contains(&c) || ('\u{FF00}'..='\u{FFEF}').contains(&c) || c == '—' || c == '「' || c == '」';
                if !kana_or_symbol && !allowed.contains(c) {
                    missing.insert(c);
                }
            }
        }
        assert!(missing.is_empty(), "폰트 아틀라스에 없는 한자: {}", missing.into_iter().collect::<String>());
    }

    #[test]
    fn fmt_fills_placeholders_in_each_language() {
        set_language(Language::En);
        assert_eq!(fmt("열쇠 {}번을 얻었다.", &["2"]), "Got key number 2.");
        set_language(Language::Ja);
        assert_eq!(fmt("열쇠 {}/{}", &["1", "3"]), "鍵 1/3");
        set_language(Language::Ko);
        assert_eq!(fmt("[E] Examine {}", &["Desk"]), "[E] 조사: 책상");
        set_language(Language::En);
    }
}
