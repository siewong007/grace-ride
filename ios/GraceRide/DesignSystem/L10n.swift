import Foundation
import SwiftUI

/// Runtime bilingual string resolver backing `container.lang` (`"en"` / `"zh"`).
/// Reads compiled `Localizable.xcstrings` -> `{en,zh-Hans}.lproj/Localizable.strings`
/// so toggling language in Login or Profile updates every screen immediately without restart.
public enum L10n {
    private static func bundle(for lang: String) -> Bundle {
        let code = lang.hasPrefix("zh") ? "zh-Hans" : "en"
        if let path = Bundle.main.path(forResource: code, ofType: "lproj"),
           let localized = Bundle(path: path) {
            return localized
        }
        return .main
    }

    public static func tr(_ key: String, lang: String) -> String {
        let b = bundle(for: lang)
        let resolved = b.localizedString(forKey: key, value: nil, table: nil)
        if resolved != key {
            return resolved
        }
        if let enPath = Bundle.main.path(forResource: "en", ofType: "lproj"),
           let enBundle = Bundle(path: enPath) {
            let fallback = enBundle.localizedString(forKey: key, value: key, table: nil)
            if fallback != key {
                return fallback
            }
        }
        return resolved
    }

    public static func tr(_ key: String, lang: String, _ args: CVarArg...) -> String {
        let format = tr(key, lang: lang)
        let locale = Locale(identifier: lang.hasPrefix("zh") ? "zh-Hans" : "en")
        return String(format: format, locale: locale, arguments: args)
    }

    public static func rideStatus(_ status: String, lang: String) -> String {
        switch status {
        case "pending_approval": return tr("ride_status_pending_approval", lang: lang)
        case "pending_driver": return tr("ride_status_pending_driver", lang: lang)
        case "pending_driver_confirm": return tr("ride_status_pending_driver_confirm", lang: lang)
        case "pending_passenger": return tr("ride_status_pending_passenger", lang: lang)
        case "confirmed", "matched": return tr("ride_status_confirmed", lang: lang)
        case "in_progress", "driver_en_route", "picked_up": return tr("ride_status_in_progress", lang: lang)
        case "completed": return tr("ride_status_completed", lang: lang)
        case "cancelled": return tr("ride_status_cancelled", lang: lang)
        case "alt_transport": return tr("ride_status_alt_transport", lang: lang)
        case "no_show": return tr("ride_status_no_show", lang: lang)
        default: return status
        }
    }

    public static func readiness(_ value: String?, lang: String) -> String {
        switch value {
        case "ready": return tr("readiness_ready", lang: lang)
        case "almost_ready": return tr("readiness_almost", lang: lang)
        case "need_time": return tr("readiness_need_time", lang: lang)
        default: return tr("readiness_prompt", lang: lang)
        }
    }

    public static func permissionRole(_ role: String, lang: String) -> String {
        let key = "role_\(role)"
        let resolved = tr(key, lang: lang)
        return resolved == key ? role : resolved
    }

    public static func serviceType(_ value: String, lang: String) -> String {
        switch value {
        case "sunday_morning": return tr("ride_new_service_sunday", lang: lang)
        case "saturday_evening": return tr("ride_new_service_saturday", lang: lang)
        case "friday_youth": return tr("ride_new_service_friday", lang: lang)
        case "special_event": return tr("ride_new_service_special", lang: lang)
        case "adhoc": return tr("ride_new_service_adhoc", lang: lang)
        default: return value
        }
    }

    public static func eventCategory(_ value: String?, lang: String) -> String {
        switch value {
        case "youth": return tr("event_cat_youth", lang: lang)
        case "fellowship": return tr("event_cat_fellowship", lang: lang)
        case "prayer": return tr("event_cat_prayer", lang: lang)
        case "service": return tr("event_cat_service", lang: lang)
        case "outreach": return tr("event_cat_outreach", lang: lang)
        case "special": return tr("event_cat_special", lang: lang)
        case "other": return tr("event_cat_other", lang: lang)
        default: return value ?? tr("event_cat_other", lang: lang)
        }
    }
}

public extension Locale {
    static func grLocale(for lang: String) -> Locale {
        Locale(identifier: lang.hasPrefix("zh") ? "zh-Hans" : "en")
    }
}
