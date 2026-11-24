//! Auth errors. Both languages live next to each other.

use super::pair;
use crate::i18n::Lang;

pub enum AuthError<'a> {
    InvalidCredentials,
    VerificationExpired,
    InvalidCode,
    CodeAlreadyUsed,
    Invalid2FaSecret,
    TwoFactorAlreadyEnabled,
    TwoFactorNotConfigured,
    StartSetupFirst,
    PasswordTooShort,
    CurrentPasswordIncorrect,
    InviteExpired,
    InviteAlreadyUsed,
    RecoveryLinkInvalid,
    PasskeyFailed,
    PasskeyRegistrationFailed(&'a str),
    ChallengeMismatch,
    NameRequired,
    NameTooLong,
    PhoneInvalid,
    RecentSignInRequired,
    EmailAlreadyRegistered,
    AuthDisabled,
    AccountPending,
    TermsRequired,
    SessionLocked,
    UnlockMethodRequired,
    TurnstileRequired,
    TurnstileFailed,
}

pub fn render(lang: Lang, key: AuthError<'_>) -> String {
    match key {
        AuthError::InvalidCredentials => {
            pair(lang, "电邮或密码不正确", "Incorrect email or password")
        }
        AuthError::VerificationExpired => pair(
            lang,
            "验证已过期，请重试",
            "Verification expired; please try again",
        ),
        AuthError::InvalidCode => pair(lang, "验证码不正确", "Incorrect verification code"),
        AuthError::CodeAlreadyUsed => pair(
            lang,
            "此验证码已使用，请等待下一组数字",
            "This code has already been used; please wait for the next one",
        ),
        AuthError::Invalid2FaSecret => pair(lang, "2FA 密钥无效", "Invalid 2FA secret"),
        AuthError::TwoFactorAlreadyEnabled => pair(
            lang,
            "2FA 已开启。如需更换验证器，请通过管理员的账号恢复程序。",
            "2FA is already on. To change your authenticator, contact an admin for account recovery.",
        ),
        AuthError::TwoFactorNotConfigured => pair(
            lang,
            "尚未设置双重验证（2FA）",
            "Two-factor authentication (2FA) is not configured",
        ),
        AuthError::StartSetupFirst => pair(lang, "请先开始设置", "Please start setup first"),
        AuthError::PasswordTooShort => pair(
            lang,
            "密码至少需要 10 个字符",
            "Password must be at least 10 characters",
        ),
        AuthError::CurrentPasswordIncorrect => {
            pair(lang, "目前密码不正确", "Current password is incorrect")
        }
        AuthError::InviteExpired => pair(
            lang,
            "邀请连结已失效，请联系管理员",
            "This invite link is no longer valid; contact an admin",
        ),
        AuthError::InviteAlreadyUsed => {
            pair(lang, "邀请连结已被使用", "This invite link has already been used")
        }
        AuthError::RecoveryLinkInvalid => pair(lang, "恢复连结无效", "Invalid recovery link"),
        AuthError::PasskeyFailed => pair(lang, "通行密钥验证失败", "Passkey verification failed"),
        AuthError::PasskeyRegistrationFailed(detail) => match lang {
            Lang::Zh => format!("通行密钥注册失败：{detail}"),
            Lang::En => format!("Passkey registration failed: {detail}"),
        },
        AuthError::ChallengeMismatch => pair(lang, "验证不匹配", "Challenge does not match"),
        AuthError::NameRequired => pair(lang, "请填写姓名", "Please enter your name"),
        AuthError::NameTooLong => pair(
            lang,
            "姓名不能超过 60 个字",
            "Name cannot exceed 60 characters",
        ),
        AuthError::PhoneInvalid => pair(
            lang,
            "电话号码需为 10 至 20 个字符",
            "Phone number must be 10 to 20 characters",
        ),
        AuthError::RecentSignInRequired => pair(
            lang,
            "为保护账号，请再次验证身份（使用通行密钥，或登出后重新登录），然后在 15 分钟内完成此设置。",
            "To protect your account, confirm it is you again (with your passkey, or by signing out and back in), then finish this within 15 minutes.",
        ),
        AuthError::EmailAlreadyRegistered => {
            pair(lang, "此电邮已经注册", "This email is already registered")
        }
        AuthError::AuthDisabled => pair(
            lang,
            "此应用使用通行密钥登入，不使用密码。",
            "This app signs in with a passkey, not a password.",
        ),
        AuthError::AccountPending => pair(
            lang,
            "账号尚未通过审核，目前只能使用个人资料。",
            "This account is waiting for approval. Only the profile is available.",
        ),
        AuthError::TermsRequired => pair(
            lang,
            "请先阅读并勾选同意使用条款。",
            "Please read the terms and opt in before continuing.",
        ),
        AuthError::SessionLocked => pair(
            lang,
            "屏幕已锁定，请验证后继续使用。",
            "Screen is locked. Verify your identity to continue.",
        ),
        AuthError::UnlockMethodRequired => pair(
            lang,
            "请提供通行密钥、双重验证码或备用码来解锁。",
            "Provide a passkey, two-factor code, or backup code to unlock.",
        ),
        AuthError::TurnstileRequired => pair(
            lang,
            "请先完成人机验证再继续",
            "Please complete the human verification check before continuing",
        ),
        AuthError::TurnstileFailed => pair(
            lang,
            "人机验证失败或已过期，请重试",
            "Human verification failed or expired; please try again",
        ),
    }
}
