//! Notification metadata classification, preference helpers, and deep-link formatting.

pub const VALID_CATEGORIES: &[&str] = &[
    "ride",
    "event",
    "cost",
    "help",
    "announcement",
    "family",
    "admin",
    "chat",
    "general",
];

#[derive(Clone, Debug, Default)]
pub struct NotifMeta {
    pub category: &'static str,
    pub event_type: String,
    pub priority: &'static str,
    pub entity_type: &'static str,
    pub entity_id: Option<i64>,
    pub tag: String,
}

#[derive(Clone, Debug)]
pub struct BatchNotifItem {
    pub user_id: i64,
    pub title_zh: String,
    pub title_en: String,
    pub body_zh: String,
    pub body_en: String,
    pub link: String,
    pub meta: NotifMeta,
}

pub fn validate_telegram_chat_id(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
    (5..=20).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
}

pub fn absolute_url(link: &str) -> String {
    let trimmed = link.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
        return trimmed.to_string();
    }
    let base = crate::util::public_url();
    if base.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        format!("{base}{trimmed}")
    } else {
        format!("{base}/{trimmed}")
    }
}

pub fn infer_meta(link: &str, title_en: &str) -> NotifMeta {
    let clean_path = link.split('?').next().unwrap_or(link).trim();
    let mut category = "general";
    let mut entity_type = "";
    let mut entity_id: Option<i64> = None;
    let mut tag = String::new();

    if let Some(id) = parse_trailing_id(clean_path, "/rides/") {
        category = "ride";
        entity_type = "ride";
        entity_id = Some(id);
        tag = format!("ride-{id}");
    } else if let Some(id) = parse_trailing_id(clean_path, "/admin/rides/") {
        category = "ride";
        entity_type = "ride";
        entity_id = Some(id);
        tag = format!("ride-{id}");
    } else if clean_path == "/drive"
        || clean_path == "/rides"
        || clean_path.starts_with("/admin/rides")
    {
        category = "ride";
    } else if let Some(id) = parse_trailing_id(clean_path, "/events/") {
        category = "event";
        entity_type = "event";
        entity_id = Some(id);
        tag = format!("event-{id}");
    } else if clean_path.starts_with("/events") {
        category = "event";
    } else if let Some(id) = parse_trailing_id(clean_path, "/reimbursements/") {
        category = "cost";
        entity_type = "reimbursement";
        entity_id = Some(id);
        tag = format!("reimb-{id}");
    } else if let Some(id) = parse_trailing_id(clean_path, "/settlements/") {
        category = "cost";
        entity_type = "settlement";
        entity_id = Some(id);
        tag = format!("settlement-{id}");
    } else if clean_path.starts_with("/costs")
        || clean_path.starts_with("/settlements")
        || clean_path.starts_with("/reimbursements")
        || clean_path.starts_with("/admin/reimbursements")
    {
        category = "cost";
    } else if let Some(id) = parse_trailing_id(clean_path, "/help/") {
        category = "help";
        entity_type = "help";
        entity_id = Some(id);
        tag = format!("help-{id}");
    } else if clean_path.starts_with("/help") {
        category = "help";
    } else if clean_path.starts_with("/me/family") {
        category = "family";
    } else if clean_path.starts_with("/admin/") {
        category = "admin";
    } else if title_en.to_ascii_lowercase().contains("announcement") {
        category = "announcement";
    }

    let lower = title_en.to_ascii_lowercase();
    let priority = if link.contains("action=readiness")
        || lower.contains("arrived")
        || lower.contains("delayed")
        || lower.contains("unable to attend")
        || lower.contains("running late")
        || lower.contains("departed")
        || lower.contains("no driver")
        || lower.contains("ride reminder")
        || lower.contains("reassignment")
        || lower.contains("cannot drive")
    {
        "urgent"
    } else if category == "announcement" || lower.contains("report") || lower.contains("mileage") {
        "low"
    } else {
        "normal"
    };

    NotifMeta {
        category,
        event_type: String::new(),
        priority,
        entity_type,
        entity_id,
        tag,
    }
}

fn parse_trailing_id(path: &str, prefix: &str) -> Option<i64> {
    let rest = path.strip_prefix(prefix)?;
    let num_part = rest.split('/').next()?;
    num_part.parse::<i64>().ok()
}

pub fn is_valid_hhmm(raw: &str) -> bool {
    let Some((h, m)) = raw.split_once(':') else {
        return false;
    };
    if h.len() != 2 || m.len() != 2 {
        return false;
    }
    matches!((h.parse::<u8>(), m.parse::<u8>()), (Ok(0..=23), Ok(0..=59)))
}

pub fn is_in_quiet_hours(start: &str, end: &str, now_hhmm: &str) -> bool {
    if start.is_empty() || end.is_empty() || start == end {
        return false;
    }
    if start < end {
        now_hhmm >= start && now_hhmm < end
    } else {
        // Overnight window (e.g. 22:00 to 07:00)
        now_hhmm >= start || now_hhmm < end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_handles_overnight_and_daytime_windows() {
        assert!(is_in_quiet_hours("22:00", "07:00", "23:30"));
        assert!(is_in_quiet_hours("22:00", "07:00", "06:59"));
        assert!(!is_in_quiet_hours("22:00", "07:00", "07:00"));
        assert!(!is_in_quiet_hours("22:00", "07:00", "14:00"));

        assert!(is_in_quiet_hours("13:00", "15:00", "13:45"));
        assert!(!is_in_quiet_hours("13:00", "15:00", "15:00"));
        assert!(!is_in_quiet_hours("", "", "23:00"));
    }

    #[test]
    fn telegram_chat_id_validation() {
        assert!(validate_telegram_chat_id(""));
        assert!(validate_telegram_chat_id("123456789"));
        assert!(validate_telegram_chat_id("-1001234567890"));
        assert!(!validate_telegram_chat_id("@username"));
        assert!(!validate_telegram_chat_id("123"));
        assert!(!validate_telegram_chat_id("abc12345"));
    }

    #[test]
    fn category_and_priority_inference() {
        let m1 = infer_meta("/rides/42?action=readiness", "Ride reminder");
        assert_eq!(m1.category, "ride");
        assert_eq!(m1.priority, "urgent");
        assert_eq!(m1.entity_id, Some(42));
        assert_eq!(m1.tag, "ride-42");

        let m2 = infer_meta("/", "New announcement");
        assert_eq!(m2.category, "announcement");
        assert_eq!(m2.priority, "low");

        let m3 = infer_meta("/help/15", "Help center has replied");
        assert_eq!(m3.category, "help");
        assert_eq!(m3.priority, "normal");
        assert_eq!(m3.entity_id, Some(15));
    }
}
