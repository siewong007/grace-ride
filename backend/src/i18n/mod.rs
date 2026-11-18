//! Internationalization (i18n) module for Grace Ride.
//!
//! Provides a structured, centralized catalog of error messages and translations,
//! keeping error maintenance decoupled from domain and routing logic.

pub mod keys;
pub mod locales;

use axum::http::{header, HeaderMap};
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

    /// Return `Some(Lang)` only when an `Accept-Language` header value explicitly
    /// requests a supported `zh*` or `en*` tag (ignoring `q=0` exclusions).
    pub fn parse_accept_language(v: &str) -> Option<Lang> {
        for raw_item in v.split(',') {
            let mut parts = raw_item.split(';');
            let tag = parts.next().unwrap_or("").trim().to_ascii_lowercase();
            if tag.is_empty() {
                continue;
            }
            let excluded = parts.any(|param| {
                param
                    .trim()
                    .strip_prefix("q=")
                    .and_then(|q| q.trim().parse::<f32>().ok())
                    .is_some_and(|q| q <= 0.0)
            });
            if excluded {
                continue;
            }
            if tag == "zh" || tag.starts_with("zh-") || tag.starts_with("zh_") {
                return Some(Lang::Zh);
            }
            if tag == "en" || tag.starts_with("en-") || tag.starts_with("en_") {
                return Some(Lang::En);
            }
        }
        None
    }

    /// Unauthenticated requests carry no `users.lang`; fall back to `Accept-Language`,
    /// defaulting unknown or absent locales consistently to `Lang::Zh`.
    pub fn from_accept_language(v: &str) -> Lang {
        Self::parse_accept_language(v).unwrap_or(Lang::Zh)
    }

    /// Extract the visitor language from HTTP request headers (`Accept-Language`, default `Lang::Zh`).
    pub fn from_headers(headers: &HeaderMap) -> Lang {
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .map(Self::from_accept_language)
            .unwrap_or(Lang::Zh)
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

#[cfg(test)]
mod tests {
    use super::Lang;
    use axum::http::{header, HeaderMap, HeaderValue};

    #[test]
    fn accept_language_resolves_explicit_and_unknown_locales() {
        assert_eq!(
            Lang::parse_accept_language("en-MY,en;q=0.9"),
            Some(Lang::En)
        );
        assert_eq!(
            Lang::parse_accept_language("zh-CN,en;q=0.8"),
            Some(Lang::Zh)
        );
        assert_eq!(
            Lang::parse_accept_language("ms-MY,en;q=0.8"),
            Some(Lang::En)
        );
        assert_eq!(Lang::parse_accept_language("ms-MY"), None);
        assert_eq!(Lang::parse_accept_language(""), None);
        assert_eq!(Lang::from_accept_language("ms-MY"), Lang::Zh);
        assert_eq!(Lang::from_accept_language("en"), Lang::En);

        let mut h = HeaderMap::new();
        assert_eq!(Lang::from_headers(&h), Lang::Zh);
        h.insert(header::ACCEPT_LANGUAGE, HeaderValue::from_static("en-US"));
        assert_eq!(Lang::from_headers(&h), Lang::En);
    }
}
