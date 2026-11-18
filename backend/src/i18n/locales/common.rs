//! Common & System errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum CommonError {
    NotFound,
    Unauthorized,
    Forbidden,
    ServerError,
    InvalidAction,
    InvalidStatus,
    InvalidDate,
    NoPermission,
    MfaRequired,
    RateLimited,
}

pub fn render(lang: Lang, key: CommonError) -> String {
    match key {
        CommonError::NotFound => pair(lang, "找不到资料", "Not found"),
        CommonError::Unauthorized => pair(lang, "请先登入", "Please log in first"),
        CommonError::Forbidden => pair(lang, "无权限", "No permission"),
        CommonError::ServerError => pair(
            lang,
            "系统错误，请稍后再试",
            "Something went wrong, please try again later",
        ),
        CommonError::InvalidAction => pair(lang, "操作无效", "Invalid action"),
        CommonError::InvalidStatus => pair(lang, "状态无效", "Invalid status"),
        CommonError::InvalidDate => pair(lang, "日期格式不正确", "Invalid date format"),
        CommonError::NoPermission => pair(
            lang,
            "您没有权限执行此操作",
            "You do not have permission to do this",
        ),
        CommonError::MfaRequired => pair(
            lang,
            "此功能需要通行密钥或双重验证（2FA）登入",
            "This feature requires a passkey or two-factor (2FA) login",
        ),
        CommonError::RateLimited => pair(
            lang,
            "请求过于频繁，请稍后再试",
            "Too many requests; please try again later",
        ),
    }
}
