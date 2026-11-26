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
    PasswordTooLong,
    CurrentPasswordIncorrect,
    InviteExpired,
    InviteAlreadyUsed,
    RecoveryLinkInvalid,
    PasskeyFailed,
    PasskeyRegistrationFailed(&'a str),
    BiometricFailed,
    BiometricRegistrationFailed,
    ChallengeMismatch,
    NameRequired,
    NameTooLong,
    PhoneInvalid,
    RecentSignInRequired,
    EmailAlreadyRegistered,
    TooManyFailedLogins,
    TotpRequired,
    AuthDisabled,
    AccountPending,
    TermsRequired,
    SessionLocked,
    UnlockMethodRequired,
    TurnstileRequired,
    TurnstileFailed,
    GoogleNotConfigured,
    GoogleAuthCancelled,
    GoogleAuthFailed,
    GoogleEmailUnverified,
    GoogleNotMember,
    GoogleNotVerified,
    GoogleEmailMismatch,
    GoogleAlreadyLinked,
    PasswordResetInvalid,
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
        AuthError::PasswordTooLong => pair(
            lang,
            "密码不能超过 1024 个字符",
            "Password cannot exceed 1024 characters",
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
        AuthError::BiometricFailed => pair(lang, "生物识别验证失败", "Biometric verification failed"),
        AuthError::BiometricRegistrationFailed => {
            pair(lang, "生物识别注册失败", "Biometric registration failed")
        }
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
        AuthError::TooManyFailedLogins => pair(
            lang,
            "登入失败次数过多，账号已暂时锁定 15 分钟。可改用通行密钥或 Google 登入。",
            "Too many failed logins; the account is locked for 15 minutes. You can sign in with a passkey or Google instead.",
        ),
        AuthError::TotpRequired => pair(
            lang,
            "请输入 6 位双重验证码或 8 位备用码以完成登入。",
            "Enter your 6-digit authenticator code or 8-character backup code to finish signing in.",
        ),
        AuthError::AuthDisabled => pair(
            lang,
            "此账号已停用，请联系管理员。",
            "This account is disabled. Please contact an admin.",
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
        AuthError::GoogleNotConfigured => pair(
            lang,
            "此服务器尚未开启 Google 登入功能。",
            "Google sign-in is not configured on this server.",
        ),
        AuthError::GoogleAuthCancelled => {
            pair(lang, "已取消 Google 验证。", "Google sign-in was cancelled.")
        }
        AuthError::GoogleAuthFailed => pair(
            lang,
            "Google 验证失败或已过期，请重试。",
            "Google verification failed or expired. Please try again.",
        ),
        AuthError::GoogleEmailUnverified => pair(
            lang,
            "您的 Google 账号电邮尚未通过 Google 验证。",
            "Your Google account email is not verified by Google.",
        ),
        AuthError::GoogleNotMember => pair(
            lang,
            "此 Google 账号不是恩典同行会员。请先向管理员索取邀请链接加入。",
            "This Google account is not registered as a Grace Ride member. Ask an admin for an invite link first.",
        ),
        AuthError::GoogleNotVerified => pair(
            lang,
            "此电邮已是会员，但尚未验证绑定 Google 账号。请先使用通行密钥或密码登入，并在「我的 → 账号安全」中完成 Google 账号验证。",
            "A member account exists for this email, but its Google account has not been verified yet. Sign in with your passkey or password and verify Google in Profile → Account Security.",
        ),
        AuthError::GoogleEmailMismatch => pair(
            lang,
            "所选的 Google 账号电邮与您注册的恩典同行电邮不一致。",
            "The selected Google account email does not match your registered Grace Ride email.",
        ),
        AuthError::GoogleAlreadyLinked => pair(
            lang,
            "此 Google 账号已绑定到另一位会员的账号。",
            "This Google account is already verified on another member account.",
        ),
        AuthError::PasswordResetInvalid => pair(
            lang,
            "密码重置验证无效或已过期，请重新通过 Google 验证。",
            "Password reset verification expired or is invalid. Verify with Google again to continue.",
        ),
    }
}
