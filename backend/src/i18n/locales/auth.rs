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
        ErrorKey::NameTooLong => pair(lang, "姓名不能超过 60 个字", "Name cannot exceed 60 characters"),
        ErrorKey::PhoneInvalid => pair(lang, "电话号码需为 10 至 20 个字符", "Phone number must be 10 to 20 characters"),
        ErrorKey::RecentSignInRequired => pair(lang, "为保护账号，请先登出，再用通行密钥登入，然后在 15 分钟内完成此设置。", "To protect your account, sign out and sign in again with your passkey, then finish this within 15 minutes."),
        ErrorKey::EmailAlreadyRegistered => pair(lang, "此电邮已经注册", "This email is already registered"),
        ErrorKey::TooManyFailedLogins => pair(lang, "登入失败次数过多，账号已暂时锁定 15 分钟。可改用通行密钥登入。", "Too many failed logins; the account is locked for 15 minutes. You can sign in with a passkey instead."),
        ErrorKey::AuthDisabled => pair(lang, "此应用使用通行密钥登入，不使用密码。", "This app signs in with a passkey, not a password."),
        ErrorKey::AccountPending => pair(lang, "账号尚未通过审核，目前只能使用个人资料。", "This account is waiting for approval. Only the profile is available."),
        ErrorKey::TermsRequired => pair(lang, "请先阅读并勾选同意使用条款。", "Please read the terms and opt in before continuing."),
        other => panic!("{} is not an auth error", other.code()),
    }
}
