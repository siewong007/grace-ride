//! Internationalization (i18n) module for Grace Ride.
//!
//! Provides a structured, centralized catalog of error messages and translations,
//! keeping error maintenance decoupled from domain and routing logic.

pub mod keys;
pub mod locales;

pub use keys::ErrorKey;
use serde::Serialize;

/// Recipient UI language.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::En => "en",
        }
    }

    pub fn from_code(s: &str) -> Lang {
        match s {
            "en" => Lang::En,
            _ => Lang::Zh,
        }
    }

    /// Unauthenticated requests carry no users.lang; fall back to Accept-Language.
    pub fn from_accept_language(v: &str) -> Lang {
        // "zh-CN,zh;q=0.9,en;q=0.8" → zh; anything else defaults to zh.
        let first = v.split(',').next().unwrap_or("").trim().to_lowercase();
        if first.starts_with("en") {
            Lang::En
        } else {
            Lang::Zh
        }
    }
}

/// Pick the side of a 中文/English string pair.
/// Used for dynamic content such as notifications and chat messages.
pub fn t<'a>(lang: Lang, zh: &'a str, en: &'a str) -> &'a str {
    match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
}
