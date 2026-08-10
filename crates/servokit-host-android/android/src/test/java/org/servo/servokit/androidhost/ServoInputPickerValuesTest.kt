package org.servo.servokit.androidhost

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.YearMonth

class ServoInputPickerValuesTest {
  @Test
  fun recognizesNativePickerInputTypes() {
    assertTrue(ServoInputPickerValues.isPickerInputType("color"))
    assertTrue(ServoInputPickerValues.isPickerInputType("date"))
    assertTrue(ServoInputPickerValues.isPickerInputType("datetime-local"))
    assertTrue(ServoInputPickerValues.isPickerInputType("month"))
    assertTrue(ServoInputPickerValues.isPickerInputType("time"))
    assertTrue(ServoInputPickerValues.isPickerInputType("week"))
    assertFalse(ServoInputPickerValues.isPickerInputType("text"))
  }

  @Test
  fun normalizesAndParsesColorValues() {
    assertEquals("#336699", ServoInputPickerValues.normalizeColorValue("336699"))
    assertEquals("#336699", ServoInputPickerValues.normalizeColorValue("#336699"))
    assertNull(ServoInputPickerValues.normalizeColorValue("#xyzxyz"))
    assertEquals(
      ServoColorValue(0x33, 0x66, 0x99),
      ServoInputPickerValues.initialColorValue("#336699")
    )
    assertEquals("#336699", ServoInputPickerValues.initialColorValue("#336699").hex())
  }

  @Test
  fun fallsBackForInvalidDateInputs() {
    val fallbackDate = LocalDate.of(2026, 4, 27)
    val fallbackTime = LocalTime.of(8, 15)
    val fallbackDateTime = LocalDateTime.of(2026, 4, 27, 8, 15)
    val fallbackMonth = YearMonth.of(2026, 4)

    assertEquals(
      fallbackDate,
      ServoInputPickerValues.initialDate("invalid", fallbackDate)
    )
    assertEquals(
      fallbackTime,
      ServoInputPickerValues.initialTime("invalid", fallbackTime)
    )
    assertEquals(
      fallbackDateTime,
      ServoInputPickerValues.initialDateTimeLocal("invalid", fallbackDateTime)
    )
    assertEquals(
      fallbackMonth,
      ServoInputPickerValues.initialMonth("invalid", fallbackMonth)
    )
    assertEquals(
      fallbackDate,
      ServoInputPickerValues.initialWeekDate("invalid", fallbackDate)
    )
  }

  @Test
  fun formatsDateLikeValuesForHtmlControls() {
    assertEquals(
      "2026-04-27",
      ServoInputPickerValues.formatDate(LocalDate.of(2026, 4, 27))
    )
    assertEquals(
      "08:15",
      ServoInputPickerValues.formatTime(LocalTime.of(8, 15, 59))
    )
    assertEquals(
      "2026-04-27T08:15",
      ServoInputPickerValues.formatDateTimeLocal(LocalDateTime.of(2026, 4, 27, 8, 15, 59))
    )
    assertEquals(
      "2026-04",
      ServoInputPickerValues.formatMonth(YearMonth.of(2026, 4))
    )
  }

  @Test
  fun parsesAndFormatsIsoWeekValues() {
    val weekDate = ServoInputPickerValues.initialWeekDate("2026-W18", LocalDate.of(2026, 1, 1))
    assertEquals(LocalDate.of(2026, 4, 27), weekDate)
    assertEquals("2026-W18", ServoInputPickerValues.formatWeek(weekDate))
  }
}
