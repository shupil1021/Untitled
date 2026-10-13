//! 게임 폴더 안 글 파일들의 내용(README, 크랙 NFO, 설정, 설치 로그, 소스 코드 조각) — 플레이어가 메모장으로 열어 보는 단서들.


pub(super) const README: &str = "test - cracked edition\r\n\
\r\n\
1. Run test.exe.\r\n\
2. Do not update the game. The updater phones home.\r\n\
3. If a door talks to you, that is normal. Answer it.\r\n\
\r\n\
Known issues\r\n\
  - The room has no flowers. We checked. Twice.\r\n\
  - Some mail comes from senders whose names do not display correctly.\r\n\
\r\n\
Do not delete the saves folder.\r\n";

pub(super) const NFO: &str = "\
 ________________________________________________\r\n\
|                                                |\r\n\
|   R 1 8   p r e s e n t s                      |\r\n\
|                                                |\r\n\
|   test  (cracked)                              |\r\n\
|   ----------------------------------------     |\r\n\
|   protection ...... removed                    |\r\n\
|   release type .... cracked                    |\r\n\
|   size ............ small, for what it does    |\r\n\
|                                                |\r\n\
|   notes:                                       |\r\n\
|     the original had a license check on the    |\r\n\
|     door. we cut it out. the door did not      |\r\n\
|     like that. we do not know why it minds.    |\r\n\
|________________________________________________|\r\n";

pub(super) const GAME_INI: &str = "\
[video]\r\n\
width=640\r\n\
height=480\r\n\
fov=56\r\n\
\r\n\
[game]\r\n\
room=0\r\n\
mouse_sens=0.0032\r\n\
door_mood=lonely\r\n\
flowers_in_room=0\r\n\
\r\n\
[network]\r\n\
mail_hook=enabled\r\n\
sender_name=<corrupted>\r\n";

pub(super) const INSTALL_LOG: &str = "\
[ok]   copied src/main.rs\r\n\
[ok]   copied src/doors.rs\r\n\
[ok]   copied src/maze.rs\r\n\
[ok]   copied assets/door.png\r\n\
[ok]   copied assets/flower_pot.png\r\n\
[ok]   copied assets/static.png\r\n\
[ok]   copied assets/sound/ambience.wav\r\n\
[ok]   copied assets/sound/door_creak.wav\r\n\
[ok]   wrote config/game.ini\r\n\
[warn] license check skipped (cracked)\r\n\
[warn] door: unexpected listener on room 0\r\n\
[ok]   install complete\r\n";

pub(super) const SRC_MAIN: &str = "\
// test -- main loop (decompiled, names recovered where possible)\r\n\
mod doors;\r\n\
mod maze;\r\n\
\r\n\
pub(super) fn main() {\r\n\
    let mut room = Room::load(0);\r\n\
    loop {\r\n\
        room.update();\r\n\
        if room.player_leaves() {\r\n\
            // never reached in the retail build.\r\n\
            break;\r\n\
        }\r\n\
    }\r\n\
}\r\n";

pub(super) const SRC_DOORS: &str = "\
// doors.rs -- every door has its own personality.\r\n\
// the game is cleared when ALL doors are satisfied.\r\n\
\r\n\
pub struct Door {\r\n\
    pub request: &'static str,\r\n\
    pub accepted: bool,\r\n\
    pub satisfied: bool,\r\n\
}\r\n\
\r\n\
pub fn room_zero_door() -> Door {\r\n\
    Door {\r\n\
        request: \"give me a borage flower\",\r\n\
        accepted: false,\r\n\
        satisfied: false,\r\n\
    }\r\n\
}\r\n\
\r\n\
impl Door {\r\n\
    pub fn talk(&mut self, accept: bool) {\r\n\
        // TODO: declining should do something. it does not. yet.\r\n\
        self.accepted |= accept;\r\n\
    }\r\n\
\r\n\
    pub fn give_flower(&mut self) {\r\n\
        // the flower is not in this room.\r\n\
        // see maze.rs for where to look.\r\n\
        self.satisfied = true;\r\n\
    }\r\n\
}\r\n";

pub(super) const SRC_MAZE: &str = "\
// maze.rs -- the other game, hidden inside this one.\r\n\
// 3 keys. 3 doors. a rope comes down once you have the key.\r\n\
\r\n\
pub(super) const KEYS_NEEDED: usize = 3;\r\n\
\r\n\
// a seed grows if you give it time.\r\n\
// the clock on the desktop is not a suggestion.\r\n\
pub fn plant(seed: Seed, pot: &mut Pot) {\r\n\
    pot.seed = Some(seed);\r\n\
    pot.needs_years = 1;\r\n\
}\r\n";
