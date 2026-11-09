//! Common & System errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::NotFound => pair(lang, "找不到资料", "Not found"),
        ErrorKey::Unauthorized => pair(lang, "请先登入", "Please log in first"),
        ErrorKey::Forbidden => pair(lang, "无权限", "No permission"),
        ErrorKey::ServerError => pair(
            lang,
            "系统错误，请稍后再试",
            "Something went wrong, please try again later",
        ),
        ErrorKey::InvalidAction => pair(lang, "操作无效", "Invalid action"),
        ErrorKey::InvalidStatus => pair(lang, "状态无效", "Invalid status"),
        ErrorKey::InvalidDate => pair(lang, "日期格式不正确", "Invalid date format"),
        ErrorKey::NoPermission => pair(
            lang,
            "您没有权限执行此操作",
            "You do not have permission to do this",
        ),
        ErrorKey::MfaRequired => pair(
            lang,
            "此功能需要通行密钥或双重验证（2FA）登入",
            "This feature requires a passkey or two-factor (2FA) login",
        ),
        ErrorKey::RateLimited => pair(
            lang,
            "请求过于频繁，请稍后再试",
            "Too many requests; please try again later",
        ),
        other => panic!("{} is not a common error", other.code()),
    }
}
