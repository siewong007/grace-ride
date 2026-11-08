//! Auth errors. Both languages live next to each other.

use super::pair;
use crate::i18n::{keys::ErrorKey, Lang};

pub fn render(lang: Lang, key: &ErrorKey) -> String {
    match key {
        ErrorKey::InvalidCredentials => pair(lang, "电邮或密码不正确", "Incorrect email or password"),
        ErrorKey::VerificationExpired => pair(lang, "验证已过期，请重试", "Verification expired; please try again"),
        ErrorKey::InvalidCode => pair(lang, "验证码不正确", "Incorrect verification code"),
        ErrorKey::CodeAlreadyUsed => pair(lang, "此验证码已使用，请等待下一组数字", "This code has already been used; please wait for the next one"),
        ErrorKey::Invalid2FaSecret => pair(lang, "2FA 密钥无效", "Invalid 2FA secret"),
        ErrorKey::TwoFactorAlreadyEnabled => pair(lang, "2FA 已开启。如需更换验证器，请通过管理员的账号恢复程序。", "2FA is already on. To change your authenticator, contact an admin for account recovery."),
        ErrorKey::StartSetupFirst => pair(lang, "请先开始设置", "Please start setup first"),
        ErrorKey::PasswordTooShort => pair(lang, "密码至少需要 10 个字符", "Password must be at least 10 characters"),
        ErrorKey::CurrentPasswordIncorrect => pair(lang, "目前密码不正确", "Current password is incorrect"),
        ErrorKey::InviteExpired => pair(lang, "邀请连结已失效，请联系管理员", "This invite link is no longer valid; contact an admin"),
        ErrorKey::InviteAlreadyUsed => pair(lang, "邀请连结已被使用", "This invite link has already been used"),
        ErrorKey::RecoveryLinkInvalid => pair(lang, "恢复连结无效", "Invalid recovery link"),
        ErrorKey::PasskeyNotFound => pair(lang, "此电邮尚未设置通行密钥", "No passkey is set up for this email"),
        ErrorKey::PasskeyFailed => pair(lang, "通行密钥验证失败", "Passkey verification failed"),
        ErrorKey::PasskeyRegistrationFailed(detail) => match lang {
            Lang::Zh => format!("通行密钥注册失败：{detail}"),
            Lang::En => format!("Passkey registration failed: {detail}"),
        },
        ErrorKey::ChallengeMismatch => pair(lang, "验证不匹配", "Challenge does not match"),
        ErrorKey::NameRequired => pair(lang, "请填写姓名", "Please enter your name"),
        ErrorKey::EmailAlreadyRegistered => pair(lang, "此电邮已经注册", "This email is already registered"),
        ErrorKey::TooManyFailedLogins => pair(lang, "登入失败次数过多，账号已暂时锁定 15 分钟。可改用通行密钥登入。", "Too many failed logins; the account is locked for 15 minutes. You can sign in with a passkey instead."),
        other => panic!("{} is not an auth error", other.code()),
    }
}
