package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.R
import java.time.Instant
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap

/** Money is integer cents everywhere (backend convention). Format without floats. */
object Money {
    fun rm(cents: Long): String {
        val neg = cents < 0
        val abs = if (neg) -cents else cents
        val ringgit = abs / 100
        val sen = (abs % 100).toInt()
        val grouped = String.format(Locale.US, "%,d", ringgit)
        return (if (neg) "-" else "") + "RM " + grouped + "." + sen.toString().padStart(2, '0')
    }
}

/** Backend timestamps are KL-local text: "yyyy-MM-dd HH:mm:ss" (sometimes minute precision). */
object KlTime {
    val zone: ZoneId = ZoneId.of("Asia/Kuala_Lumpur")
    private val full = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss", Locale.US)
    private val minuteFmt = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm", Locale.US)
    private val dateOnly = DateTimeFormatter.ofPattern("yyyy-MM-dd", Locale.US)
    private val time24 = DateTimeFormatter.ofPattern("HH:mm", Locale.US)
    private val zhDisplay = DateTimeFormatter.ofPattern("M月d日 EEE HH:mm", Locale.CHINESE)
    private val localeFormatters = ConcurrentHashMap<Pair<String, Locale>, DateTimeFormatter>()

    private fun localizedFormatter(
        pattern: String,
        locale: Locale = Locale.getDefault(),
    ): DateTimeFormatter = localeFormatters.getOrPut(pattern to locale) {
        DateTimeFormatter.ofPattern(pattern, locale)
    }

    fun parse(text: String): LocalDateTime? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        if (clean.endsWith("Z") || clean.matches(Regex("^\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}(:\\d{2}(\\.\\d+)?)?([+-]\\d{2}:?\\d{2})$"))) {
            runCatching {
                return java.time.OffsetDateTime.parse(clean.replace(' ', 'T'))
                    .atZoneSameInstant(zone)
                    .toLocalDateTime()
            }
        }
        val t = clean.replace('T', ' ')
        return runCatching { LocalDateTime.parse(t.take(19), full) }.getOrNull()
            ?: runCatching { LocalDateTime.parse(t.take(16), minuteFmt) }.getOrNull()
            ?: runCatching { LocalDate.parse(t.take(10), dateOnly).atStartOfDay() }.getOrNull()
    }

    private fun isZh(locale: Locale = Locale.getDefault()): Boolean = locale.language == "zh"

    /** "Sat, 21 Nov · 09:30" (EN) / "11月21日 周六 · 09:30" (ZH). */
    fun display(text: String): String {
        val dt = parse(text) ?: return text
        val locale = Locale.getDefault()
        val pattern = if (isZh(locale)) "M月d日 EEE · HH:mm" else "EEE, d MMM · HH:mm"
        return dt.format(localizedFormatter(pattern, locale))
    }

    fun displayDate(text: String): String {
        val dt = parse(text) ?: return text
        val locale = Locale.getDefault()
        val pattern = if (isZh(locale)) "M月d日 EEE" else "EEE, d MMM"
        return dt.format(localizedFormatter(pattern, locale))
    }

    fun displayTime(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(localizedFormatter("HH:mm"))
    }

    fun weekdayShort(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(localizedFormatter("EEE"))
    }

    fun dayMonthShort(text: String): String {
        val dt = parse(text) ?: return text
        val locale = Locale.getDefault()
        val pattern = if (isZh(locale)) "M月d日" else "d MMM"
        return dt.format(localizedFormatter(pattern, locale))
    }

    fun displayZh(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(zhDisplay)
    }

    fun time(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(time24)
    }

    fun date(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(dateOnly)
    }

    fun isToday(text: String): Boolean {
        val dt = parse(text) ?: return false
        return dt.toLocalDate() == LocalDate.now(zone)
    }

    fun today(): String = LocalDate.now(zone).format(dateOnly)

    fun todayDate(): LocalDate = LocalDate.now(zone)

    fun todayPlusDays(days: Long): String = LocalDate.now(zone).plusDays(days).format(dateOnly)

    /** Current Kuala Lumpur local date-time. */
    fun now(): LocalDateTime = LocalDateTime.now(zone)

    /** Current Kuala Lumpur timestamp to minute precision ("yyyy-MM-dd HH:mm"). */
    fun nowStamp(): String = ZonedDateTime.now(zone).format(minuteFmt)

    fun formatEpochMillis(millis: Long): String =
        Instant.ofEpochMilli(millis).atZone(zone).format(localizedFormatter("HH:mm"))

    enum class DayLabel { TODAY, TOMORROW, OTHER }

    /** Relative day for scannable lists: "Today · 20:00" beats a full date. */
    fun dayLabel(text: String): DayLabel {
        val dt = parse(text) ?: return DayLabel.OTHER
        val today = LocalDate.now(zone)
        return when (dt.toLocalDate()) {
            today -> DayLabel.TODAY
            today.plusDays(1) -> DayLabel.TOMORROW
            else -> DayLabel.OTHER
        }
    }

    /** API pickup_at format from date + hour/minute. */
    fun apiStamp(date: LocalDate, hour: Int, minute: Int): String =
        date.atTime(hour, minute).format(minuteFmt)
}

/** Material 3 DatePicker operates on UTC midnight timestamps, regardless of device timezone. */
object DatePickerUtc {
    fun toPickerMillis(date: LocalDate): Long =
        date.atStartOfDay(ZoneOffset.UTC).toInstant().toEpochMilli()

    fun fromPickerMillis(millis: Long): LocalDate =
        Instant.ofEpochMilli(millis).atZone(ZoneOffset.UTC).toLocalDate()
}

/** Shared proxy rider relationship keys and localized labels across booking and management screens. */
object ProxyRelationships {
    val keys = listOf("parent", "cell_member", "spouse", "friend", "other")

    fun labelRes(key: String): Int? = when (key.trim().lowercase()) {
        "parent" -> R.string.ride_new_relationship_parent
        "cell_member" -> R.string.proxy_rel_cell
        "child" -> R.string.ride_new_relationship_child
        "spouse" -> R.string.ride_new_relationship_spouse
        "friend" -> R.string.ride_new_relationship_friend
        "other" -> R.string.ride_new_relationship_other
        else -> null
    }
}

/** Shared ride assistance keys and localized labels across booking and detail screens. */
object Assistance {
    fun labelRes(key: String): Int? = when (key.trim().lowercase()) {
        "" -> R.string.ride_new_assist_none
        "wheelchair" -> R.string.ride_new_assist_wheelchair
        "stroller" -> R.string.ride_new_assist_stroller
        "oxygen" -> R.string.ride_new_assist_oxygen
        "other" -> R.string.ride_new_assist_other
        else -> null
    }
}
