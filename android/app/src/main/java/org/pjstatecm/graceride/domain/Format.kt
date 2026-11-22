package org.pjstatecm.graceride.domain

import java.time.LocalDate
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale

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
    private val full = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")
    private val minuteFmt = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm")
    private val dateOnly = DateTimeFormatter.ofPattern("yyyy-MM-dd")

    fun parse(text: String): LocalDateTime? {
        // The API emits KL-local stamps; some fields use ISO 'T' separators.
        val t = text.trim().replace('T', ' ')
        return try {
            when {
                t.length >= 19 -> LocalDateTime.parse(t.substring(0, 19), full)
                t.length >= 16 -> LocalDateTime.parse(t.substring(0, 16), minuteFmt)
                t.length >= 10 -> LocalDate.parse(t.substring(0, 10), dateOnly).atStartOfDay()
                else -> null
            }
        } catch (_: Exception) {
            null
        }
    }

    /** "Sat, 21 Nov · 09:30" in the app's current locale (MainActivity sets it). */
    fun display(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("EEE, d MMM · HH:mm", Locale.getDefault()))
    }

    fun displayDate(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()))
    }

    fun displayTime(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("HH:mm", Locale.getDefault()))
    }

    fun weekdayShort(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("EEE", Locale.getDefault()))
    }

    fun dayMonthShort(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("d MMM", Locale.getDefault()))
    }

    fun displayZh(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("M月d日 EEE HH:mm", Locale.CHINESE))
    }

    fun time(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("HH:mm"))
    }

    fun date(text: String): String {
        val dt = parse(text) ?: return text
        return dt.format(DateTimeFormatter.ofPattern("yyyy-MM-dd"))
    }

    fun isToday(text: String): Boolean {
        val dt = parse(text) ?: return false
        return dt.toLocalDate() == LocalDate.now()
    }

    fun today(): String = LocalDate.now().format(dateOnly)

    fun todayPlusDays(days: Long): String = LocalDate.now().plusDays(days).format(dateOnly)

    enum class DayLabel { TODAY, TOMORROW, OTHER }

    /** Relative day for scannable lists: "Today · 20:00" beats a full date. */
    fun dayLabel(text: String): DayLabel {
        val dt = parse(text) ?: return DayLabel.OTHER
        val today = LocalDate.now()
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
