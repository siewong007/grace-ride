//! Family-list errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum FamilyError {
    ActiveOnly,
    AlreadyInFamily,
    RequestPending,
    OwnPhone,
    PhoneAmbiguous,
    PhoneUnavailable,
    Full(i64),
    OwnerCannotLeave,
}

pub fn render(lang: Lang, key: FamilyError) -> String {
    match key {
        FamilyError::ActiveOnly => pair(
            lang,
            "账号通过审核后才能建立或邀请家人",
            "You can create a family or add someone after an admin approves your account",
        ),
        FamilyError::AlreadyInFamily => pair(lang, "已经在一个家庭里了", "Already in a family"),
        FamilyError::RequestPending => pair(
            lang,
            "这个号码已有另一个家庭的待处理邀请",
            "This number already has an open request from another family",
        ),
        FamilyError::OwnPhone => pair(lang, "这是你自己的号码", "That is your own number"),
        FamilyError::PhoneAmbiguous => pair(
            lang,
            "这个号码对应多位会员，请联系管理员",
            "This number matches more than one member. Ask an admin",
        ),
        FamilyError::PhoneUnavailable => pair(
            lang,
            "这个号码无法加入，请联系管理员",
            "This number cannot be added. Ask an admin",
        ),
        FamilyError::Full(max) => match lang {
            Lang::Zh => format!("一个家庭最多 {max} 人"),
            Lang::En => format!("A family can have at most {max} people"),
        },
        FamilyError::OwnerCannotLeave => pair(
            lang,
            "户主请删除整个家庭，而不能退出",
            "The owner deletes the family instead of leaving",
        ),
    }
}
