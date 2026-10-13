//! 메일 내용 — 메일 종류(MailId)마다 보낸 사람/제목/본문과, 받은편지함 목록으로 만드는 seed_messages.

use super::*;

// 첫 메일을 보낸 사람 — 도착 토스트(desktop.rs)에서도 같은 주소를 보여줘야 해서 공개.
pub const FIRST_MAIL_FROM: &str = "old.friend@mail.com";

// 두 번째 메일을 보낸 사람 — 이름이 깨져 있다(시트의 "깨져있는 이름의 이메일"). 폰트에
// 없는 글자(U+FFFD)는 렌더러가 깨진 글자(마름모) 자리표시자로 그려서 그대로 깨져 보인다.
pub const SECOND_MAIL_FROM: &str = "\u{FFFD}\u{FFFD}\u{FFFD}@mail.com";

// 메일마다 보낸 사람 — 첫 메일만 친구, 미니게임 메일은 각 방의 NPC 이름이고, 나머지는 이름이 깨져 있다.
// 알림 토스트(desktop.rs)도 쓴다.
pub fn mail_from(id: MailId, lang: Language) -> &'static str {
    match id {
        MailId::Friend => FIRST_MAIL_FROM,
        MailId::EyeTask | MailId::EyeReward => t(lang, s::FROM_EYE),
        MailId::BrainTask | MailId::BrainReward => t(lang, s::FROM_BRAIN),
        MailId::FingerTask | MailId::FingerReward => t(lang, s::FROM_FINGER),
        _ => SECOND_MAIL_FROM,
    }
}

// 메일마다 제목(알림 토스트와 목록에 같은 걸 쓴다).
pub fn mail_subject(id: MailId, lang: Language) -> &'static str {
    t(
        lang,
        match id {
            MailId::Friend => s::GAME_MAIL_SUBJECT,
            MailId::Letter => s::LETTER_MAIL_SUBJECT,
            MailId::Flower => s::FLOWER_MAIL_SUBJECT,
            MailId::Hint => s::HINT_MAIL_SUBJECT,
            MailId::Photo => s::PHOTO_MAIL_SUBJECT,
            MailId::Outside => s::OUTSIDE_MAIL_SUBJECT,
            MailId::Shopping => s::SHOPPING_MAIL_SUBJECT,
            MailId::EyeTask => s::EYE_TASK_SUBJECT,
            MailId::EyeReward => s::EYE_REWARD_SUBJECT,
            MailId::BrainTask => s::BRAIN_TASK_SUBJECT,
            MailId::BrainReward => s::BRAIN_REWARD_SUBJECT,
            MailId::FingerTask => s::FINGER_TASK_SUBJECT,
            MailId::FingerReward => s::FINGER_REWARD_SUBJECT,
            MailId::Meal => s::MEAL_MAIL_SUBJECT,
        },
    )
}

pub(super) fn mail_body(id: MailId, lang: Language) -> &'static str {
    t(
        lang,
        match id {
            MailId::Friend => s::GAME_MAIL_BODY,
            MailId::Letter => s::LETTER_MAIL_BODY,
            MailId::Flower => s::FLOWER_MAIL_BODY,
            MailId::Hint => s::HINT_MAIL_BODY,
            MailId::Photo => s::PHOTO_MAIL_BODY,
            MailId::Outside => s::OUTSIDE_MAIL_BODY,
            MailId::Shopping => s::SHOPPING_MAIL_BODY,
            MailId::EyeTask => s::EYE_TASK_BODY,
            MailId::EyeReward => s::EYE_REWARD_BODY,
            MailId::BrainTask => s::BRAIN_TASK_BODY,
            MailId::BrainReward => s::BRAIN_REWARD_BODY,
            MailId::FingerTask => s::FINGER_TASK_BODY,
            MailId::FingerReward => s::FINGER_REWARD_BODY,
            MailId::Meal => s::MEAL_MAIL_BODY,
        },
    )
}

// 받은편지함의 메일들 — 도착한 순서대로(fs.mail_log). from/to 는 이메일 주소라 언어와 무관하게
// 그대로 두고 subject/body 는 지금 언어로 고른다. attachments 는 메시지와 같은 순서의 첨부
// (fs.mail_attachments() — apps/mod.rs::open() 이 구해 넘긴다).
pub(super) fn seed_messages(log: &[MailId], lang: Language, attachments: Vec<Option<(FileId, String, IconType)>>) -> Vec<MailMsg> {
    log.iter()
        .zip(attachments)
        .map(|(&id, attachment)| MailMsg {
            from: mail_from(id, lang),
            to: "you@mail.com",
            cc: "",
            subject: mail_subject(id, lang),
            body: mail_body(id, lang),
            attachment,
        })
        .collect()
}
