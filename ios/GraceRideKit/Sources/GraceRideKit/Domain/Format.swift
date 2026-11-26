import Foundation

/// Money is integer cents everywhere (backend convention). Format without floats.
public enum Money {
    public static func rm(_ cents: Int64) -> String {
        let neg = cents < 0
        let absVal: UInt64 = neg ? UInt64(bitPattern: ~cents &+ 1) : UInt64(cents)
        let ringgit = absVal / 100
        let sen = Int(absVal % 100)
        let formatter = NumberFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.usesGroupingSeparator = true
        let grouped = formatter.string(from: NSNumber(value: ringgit)) ?? String(ringgit)
        let senStr = String(format: "%02d", sen)
        return (neg ? "-" : "") + "RM " + grouped + "." + senStr
    }
}

/// Backend timestamps are KL-local text: `"yyyy-MM-dd HH:mm:ss"` (sometimes minute precision or ISO-8601).
public enum KlTime {
    public static let timeZone = TimeZone(identifier: "Asia/Kuala_Lumpur")!

    private static let posixLocale = Locale(identifier: "en_US_POSIX")

    private static func makeFormatter(_ format: String, locale: Locale = posixLocale) -> DateFormatter {
        let df = DateFormatter()
        df.locale = locale
        df.timeZone = timeZone
        df.calendar = Calendar(identifier: .gregorian)
        df.dateFormat = format
        return df
    }

    public static var klCalendar: Calendar {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = timeZone
        return cal
    }

    public static func parse(_ text: String) -> Date? {
        let clean = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !clean.isEmpty else { return nil }

        // ISO8601 with explicit timezone offset / Z
        if clean.hasSuffix("Z") || clean.range(of: #"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(:\d{2}(\.\d+)?)?([+-]\d{2}:?\d{2})$"#, options: .regularExpression) != nil {
            let isoText = clean.replacingOccurrences(of: " ", with: "T")
            let isoFrac = ISO8601DateFormatter()
            isoFrac.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
            if let d = isoFrac.date(from: isoText) { return d }
            let iso = ISO8601DateFormatter()
            iso.formatOptions = [.withInternetDateTime]
            if let d = iso.date(from: isoText) { return d }
        }

        let normalized = clean.replacingOccurrences(of: "T", with: " ")
        if normalized.count >= 19, let d = makeFormatter("yyyy-MM-dd HH:mm:ss").date(from: String(normalized.prefix(19))) {
            return d
        }
        if normalized.count >= 16, let d = makeFormatter("yyyy-MM-dd HH:mm").date(from: String(normalized.prefix(16))) {
            return d
        }
        if normalized.count >= 10, let d = makeFormatter("yyyy-MM-dd").date(from: String(normalized.prefix(10))) {
            return d
        }
        return nil
    }

    private static func displayLocale(for lang: String?) -> Locale {
        if let lang {
            return lang.hasPrefix("zh") ? Locale(identifier: "zh_Hans") : Locale(identifier: "en_US")
        }
        return Locale.current.language.languageCode?.identifier == "zh" ? Locale(identifier: "zh_Hans") : Locale(identifier: "en_US")
    }

    private static func isZh(_ lang: String?) -> Bool {
        if let lang { return lang.hasPrefix("zh") }
        return Locale.current.language.languageCode?.identifier == "zh"
    }

    /// `"Sat, 21 Nov · 09:30"` (EN) / `"11月21日 周六 · 09:30"` (ZH).
    public static func display(_ text: String, lang: String? = nil) -> String {
        guard let d = parse(text) else { return text }
        let pattern = isZh(lang) ? "M月d日 EEE · HH:mm" : "EEE, d MMM · HH:mm"
        return makeFormatter(pattern, locale: displayLocale(for: lang)).string(from: d)
    }

    public static func displayDate(_ text: String, lang: String? = nil) -> String {
        guard let d = parse(text) else { return text }
        let pattern = isZh(lang) ? "M月d日 EEE" : "EEE, d MMM"
        return makeFormatter(pattern, locale: displayLocale(for: lang)).string(from: d)
    }

    public static func displayTime(_ text: String) -> String {
        guard let d = parse(text) else { return text }
        return makeFormatter("HH:mm").string(from: d)
    }

    public static func weekdayShort(_ text: String, lang: String? = nil) -> String {
        guard let d = parse(text) else { return text }
        return makeFormatter("EEE", locale: displayLocale(for: lang)).string(from: d)
    }

    public static func dayMonthShort(_ text: String, lang: String? = nil) -> String {
        guard let d = parse(text) else { return text }
        let pattern = isZh(lang) ? "M月d日" : "d MMM"
        return makeFormatter(pattern, locale: displayLocale(for: lang)).string(from: d)
    }

    public static func time(_ text: String) -> String {
        guard let d = parse(text) else { return text }
        return makeFormatter("HH:mm").string(from: d)
    }

    public static func date(_ text: String) -> String {
        guard let d = parse(text) else { return text }
        return makeFormatter("yyyy-MM-dd").string(from: d)
    }

    public static func isToday(_ text: String, now: Date = Date()) -> Bool {
        guard let d = parse(text) else { return false }
        return klCalendar.isDate(d, inSameDayAs: now)
    }

    public static func today(now: Date = Date()) -> String {
        makeFormatter("yyyy-MM-dd").string(from: now)
    }

    public static func todayPlusDays(_ days: Int, now: Date = Date()) -> String {
        let shifted = klCalendar.date(byAdding: .day, value: days, to: now) ?? now
        return makeFormatter("yyyy-MM-dd").string(from: shifted)
    }

    public static func nowStamp(now: Date = Date()) -> String {
        makeFormatter("yyyy-MM-dd HH:mm").string(from: now)
    }

    public static func formatEpochMillis(_ millis: Int64) -> String {
        let date = Date(timeIntervalSince1970: Double(millis) / 1000.0)
        return makeFormatter("HH:mm").string(from: date)
    }

    public enum DayLabel: Equatable, Sendable {
        case today, tomorrow, other
    }

    public static func dayLabel(_ text: String, now: Date = Date()) -> DayLabel {
        guard let d = parse(text) else { return .other }
        let cal = klCalendar
        if cal.isDate(d, inSameDayAs: now) { return .today }
        if let tomorrow = cal.date(byAdding: .day, value: 1, to: now),
           cal.isDate(d, inSameDayAs: tomorrow) {
            return .tomorrow
        }
        return .other
    }

    public static func apiStamp(date: Date, hour: Int, minute: Int) -> String {
        let cal = klCalendar
        var comps = cal.dateComponents([.year, .month, .day], from: date)
        comps.hour = hour
        comps.minute = minute
        comps.second = 0
        let target = cal.date(from: comps) ?? date
        return makeFormatter("yyyy-MM-dd HH:mm").string(from: target)
    }

    public static func components(_ date: Date) -> DateComponents {
        klCalendar.dateComponents([.year, .month, .day, .hour, .minute, .second], from: date)
    }

    public static func dateFromYMD(year: Int, month: Int, day: Int, hour: Int = 0, minute: Int = 0) -> Date? {
        var comps = DateComponents()
        comps.timeZone = timeZone
        comps.calendar = klCalendar
        comps.year = year
        comps.month = month
        comps.day = day
        comps.hour = hour
        comps.minute = minute
        return klCalendar.date(from: comps)
    }
}

public enum ProxyRelationships {
    public static let keys = ["parent", "cell_member", "spouse", "friend", "other"]
}

public enum AssistanceOptions {
    public static let keys = ["", "wheelchair", "stroller", "oxygen", "other"]
}
