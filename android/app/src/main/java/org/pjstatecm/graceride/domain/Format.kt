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
        val abs: ULong = if (neg) (-(cents + 1L)).toULong() + 1uL else cents.toULong()
        val ringgit = abs / 100uL
        val sen = (abs % 100uL).toInt()
        val grouped = formatGroupedUnsigned(ringgit)
        return (if (neg) "-" else "") + "RM " + grouped + "." + sen.toString().padStart(2, '0')
    }

    private fun formatGroupedUnsigned(value: ULong): String {
        val raw = value.toString()
        if (raw.length <= 3) return raw
        return buildString(raw.length + (raw.length - 1) / 3) {
            for (i in raw.indices) {
                if (i > 0 && (raw.length - i) % 3 == 0) append(',')
                append(raw[i])
            }
        }
    }

    /**
     * Split [totalCents] equally among [count] members, matching server `equal_split`:
     * remainder cents go to the first `totalCents % count` people so the sum matches exactly.
     */
    fun equalSplit(totalCents: Long, count: Int): List<Long> {
        if (count <= 0) return emptyList()
        val n = count.toLong()
        val safeTotal = totalCents.coerceAtLeast(0L)
        val base = safeTotal / n
        val rem = safeTotal % n
        return List(count) { i -> base + if (i < rem) 1L else 0L }
    }

    /** Parses a non-negative decimal MYR amount string (e.g. "12.50" or "RM 15") to integer cents without float drift. */
    fun parseCents(raw: String): Long? {
        var clean = raw.trim()
        val upper = clean.uppercase(Locale.US)
        when {
            upper.startsWith("MYR") -> clean = clean.substring(3).trim()
            upper.startsWith("RM") -> clean = clean.substring(2).trim()
        }
        clean = clean.replace(",", "")
        if (clean.isEmpty()) return null
        val bd = clean.toBigDecimalOrNull() ?: return null
        if (bd < java.math.BigDecimal.ZERO) return null
        return runCatching {
            bd.movePointRight(2).setScale(0, java.math.RoundingMode.HALF_UP).longValueExact()
        }.getOrNull()
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
    private val OFFSET_RE = Regex("^\\d{4}-\\d{2}-\\d{2}[Tt ]\\d{2}:\\d{2}(:\\d{2}(\\.\\d+)?)?([Zz]|[+-]\\d{2}:?\\d{2})$")
    private val COMPACT_OFFSET_RE = Regex("([+-]\\d{2})(\\d{2})$")
    private val FRACTIONAL_LOCAL_RE = Regex("^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2}\\.\\d+$")

    private fun localizedFormatter(
        pattern: String,
        locale: Locale = Locale.getDefault(),
    ): DateTimeFormatter = localeFormatters.getOrPut(pattern to locale) {
        DateTimeFormatter.ofPattern(pattern, locale)
    }

    /** Normalizes `'T'`/`'t'` to `' '` so KL text timestamps compare chronologically. */
    fun sortKey(raw: String?): String = raw?.trim()?.replace('T', ' ')?.replace('t', ' ').orEmpty()

    fun parse(text: String): LocalDateTime? {
        val clean = text.trim()
        if (clean.isEmpty()) return null
        if (OFFSET_RE.matches(clean)) {
            var iso = clean.replace(' ', 'T').replace('t', 'T')
            if (iso.endsWith("z")) {
                iso = iso.dropLast(1) + "Z"
            } else {
                iso = COMPACT_OFFSET_RE.replace(iso, "$1:$2")
            }
            return runCatching {
                java.time.OffsetDateTime.parse(iso)
                    .atZoneSameInstant(zone)
                    .toLocalDateTime()
            }.getOrNull()
        }
        val normalized = clean.replace('T', ' ').replace('t', ' ')
        val localStr = if (FRACTIONAL_LOCAL_RE.matches(normalized)) {
            normalized.substring(0, 19)
        } else {
            normalized
        }
        return when (localStr.length) {
            19 -> runCatching { LocalDateTime.parse(localStr, full) }.getOrNull()
            16 -> runCatching { LocalDateTime.parse(localStr, minuteFmt) }.getOrNull()
            10 -> runCatching { LocalDate.parse(localStr, dateOnly).atStartOfDay() }.getOrNull()
            else -> null
        }
    }

    private fun isZh(locale: Locale = Locale.getDefault()): Boolean = locale.language == "zh"

    /** "Sat, 21 Nov · 09:30" (EN) / "11月21日 周六 · 09:30" (ZH). */
    fun display(text: String, locale: Locale = Locale.getDefault()): String {
        val dt = parse(text) ?: return text
        val pattern = if (isZh(locale)) "M月d日 EEE · HH:mm" else "EEE, d MMM · HH:mm"
        return dt.format(localizedFormatter(pattern, locale))
    }

    fun displayDate(text: String, locale: Locale = Locale.getDefault()): String {
        val dt = parse(text) ?: return text
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

    fun dayMonthShort(text: String, locale: Locale = Locale.getDefault()): String {
        val dt = parse(text) ?: return text
        val pattern = if (isZh(locale)) "M月d日" else "d MMM"
        return dt.format(localizedFormatter(pattern, locale))
    }

    fun parseEpochMillis(text: String): Long? =
        parse(text)?.atZone(zone)?.toInstant()?.toEpochMilli()

    fun notificationDate(
        text: String,
        locale: Locale = Locale.getDefault(),
        referenceYear: Int = LocalDate.now(zone).year,
    ): String {
        val dt = parse(text) ?: return text
        val pattern = if (dt.year == referenceYear) {
            if (isZh(locale)) "M月d日" else "d MMM"
        } else {
            if (isZh(locale)) "yyyy年M月d日" else "d MMM yyyy"
        }
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
