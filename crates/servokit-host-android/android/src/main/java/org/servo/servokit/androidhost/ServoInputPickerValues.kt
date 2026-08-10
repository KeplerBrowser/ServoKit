package org.servo.servokit.androidhost

import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.YearMonth
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.time.temporal.WeekFields
import java.util.Locale

/** Proof-only RGB value helper for Android fallback picker UI. */
data class ServoColorValue(
  val red: Int,
  val green: Int,
  val blue: Int
) {
  init {
    require(red in 0..255) { "red channel must be in range 0..255" }
    require(green in 0..255) { "green channel must be in range 0..255" }
    require(blue in 0..255) { "blue channel must be in range 0..255" }
  }

  fun hex(): String = String.format(Locale.US, "#%02x%02x%02x", red, green, blue)
}

/**
 * Proof-only parser/formatter helpers for Android non-text input fallback UI.
 *
 * This lives in the shared Android host module so RN Android and Kotlin Android
 * examples do not maintain divergent date/time/color/week normalization rules.
 */
object ServoInputPickerValues {
  private val monthFormatter = DateTimeFormatter.ofPattern("yyyy-MM", Locale.US)
  private val dateTimeLocalFormatter = DateTimeFormatter.ofPattern("yyyy-MM-dd'T'HH:mm", Locale.US)
  private val weekPattern = Regex("""^(\d{4})-W(\d{2})$""")
  private val colorPattern = Regex("""^#?([0-9a-fA-F]{6})$""")
  private val isoWeekFields = WeekFields.ISO

  fun isPickerInputType(type: String): Boolean =
    when (type) {
      "color",
      "date",
      "datetime-local",
      "month",
      "time",
      "week" -> true
      else -> false
    }

  fun normalizeColorValue(text: String): String? {
    val match = colorPattern.matchEntire(text.trim()) ?: return null
    return "#${match.groupValues[1].lowercase(Locale.US)}"
  }

  fun initialColorValue(text: String): ServoColorValue {
    val normalized = normalizeColorValue(text) ?: return ServoColorValue(0, 0, 0)
    val color = normalized.removePrefix("#")
    return ServoColorValue(
      red = color.substring(0, 2).toInt(16),
      green = color.substring(2, 4).toInt(16),
      blue = color.substring(4, 6).toInt(16)
    )
  }

  fun initialDate(text: String, fallback: LocalDate): LocalDate =
    runCatching {
      LocalDate.parse(text, DateTimeFormatter.ISO_LOCAL_DATE)
    }.getOrElse { fallback }

  fun initialTime(text: String, fallback: LocalTime): LocalTime =
    runCatching {
      LocalTime.parse(text, DateTimeFormatter.ISO_LOCAL_TIME).truncatedTo(ChronoUnit.MINUTES)
    }.getOrElse { fallback.truncatedTo(ChronoUnit.MINUTES) }

  fun initialDateTimeLocal(text: String, fallback: LocalDateTime): LocalDateTime =
    runCatching {
      LocalDateTime.parse(text, DateTimeFormatter.ISO_LOCAL_DATE_TIME)
        .truncatedTo(ChronoUnit.MINUTES)
    }.getOrElse { fallback.truncatedTo(ChronoUnit.MINUTES) }

  fun initialMonth(text: String, fallback: YearMonth): YearMonth =
    runCatching {
      YearMonth.parse(text, monthFormatter)
    }.getOrElse { fallback }

  fun initialWeekDate(text: String, fallback: LocalDate): LocalDate = parseWeekDate(text) ?: fallback

  fun formatDate(date: LocalDate): String = DateTimeFormatter.ISO_LOCAL_DATE.format(date)

  fun formatTime(time: LocalTime): String =
    time.truncatedTo(ChronoUnit.MINUTES).format(DateTimeFormatter.ofPattern("HH:mm", Locale.US))

  fun formatDateTimeLocal(dateTime: LocalDateTime): String =
    dateTime.truncatedTo(ChronoUnit.MINUTES).format(dateTimeLocalFormatter)

  fun formatMonth(month: YearMonth): String = month.format(monthFormatter)

  fun formatWeek(date: LocalDate): String =
    String.format(
      Locale.US,
      "%04d-W%02d",
      date.get(isoWeekFields.weekBasedYear()),
      date.get(isoWeekFields.weekOfWeekBasedYear())
    )

  private fun parseWeekDate(text: String): LocalDate? {
    val match = weekPattern.matchEntire(text.trim()) ?: return null
    val weekBasedYear = match.groupValues[1].toIntOrNull() ?: return null
    val week = match.groupValues[2].toIntOrNull() ?: return null
    if (week !in 1..53) {
      return null
    }

    return runCatching {
      LocalDate
        .of(weekBasedYear, 1, 4)
        .with(isoWeekFields.weekBasedYear(), weekBasedYear.toLong())
        .with(isoWeekFields.weekOfWeekBasedYear(), week.toLong())
        .with(isoWeekFields.dayOfWeek(), 1)
    }.getOrNull()
  }
}
