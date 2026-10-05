package com.kepler.explorerkit.androidhost

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.YearMonth

class ExplorerInputPickerValuesTest {
  @Test
  fun recognizesNativePickerInputTypes() {
    assertTrue(ExplorerInputPickerValues.isPickerInputType("color"))
    assertTrue(ExplorerInputPickerValues.isPickerInputType("date"))
    assertTrue(ExplorerInputPickerValues.isPickerInputType("datetime-local"))
    assertTrue(ExplorerInputPickerValues.isPickerInputType("month"))
    assertTrue(ExplorerInputPickerValues.isPickerInputType("time"))
    assertTrue(ExplorerInputPickerValues.isPickerInputType("week"))
    assertFalse(ExplorerInputPickerValues.isPickerInputType("text"))
  }

  @Test
  fun normalizesAndParsesColorValues() {
    assertEquals("#336699", ExplorerInputPickerValues.normalizeColorValue("336699"))
    assertEquals("#336699", ExplorerInputPickerValues.normalizeColorValue("#336699"))
    assertNull(ExplorerInputPickerValues.normalizeColorValue("#xyzxyz"))
    assertEquals(
      ExplorerColorValue(0x33, 0x66, 0x99),
      ExplorerInputPickerValues.initialColorValue("#336699")
    )
    assertEquals("#336699", ExplorerInputPickerValues.initialColorValue("#336699").hex())
  }

  @Test
  fun fallsBackForInvalidDateInputs() {
    val fallbackDate = LocalDate.of(2026, 4, 27)
    val fallbackTime = LocalTime.of(8, 15)
    val fallbackDateTime = LocalDateTime.of(2026, 4, 27, 8, 15)
    val fallbackMonth = YearMonth.of(2026, 4)

    assertEquals(
      fallbackDate,
      ExplorerInputPickerValues.initialDate("invalid", fallbackDate)
    )
    assertEquals(
      fallbackTime,
      ExplorerInputPickerValues.initialTime("invalid", fallbackTime)
    )
    assertEquals(
      fallbackDateTime,
      ExplorerInputPickerValues.initialDateTimeLocal("invalid", fallbackDateTime)
    )
    assertEquals(
      fallbackMonth,
      ExplorerInputPickerValues.initialMonth("invalid", fallbackMonth)
    )
    assertEquals(
      fallbackDate,
      ExplorerInputPickerValues.initialWeekDate("invalid", fallbackDate)
    )
  }

  @Test
  fun formatsDateLikeValuesForHtmlControls() {
    assertEquals(
      "2026-04-27",
      ExplorerInputPickerValues.formatDate(LocalDate.of(2026, 4, 27))
    )
    assertEquals(
      "08:15",
      ExplorerInputPickerValues.formatTime(LocalTime.of(8, 15, 59))
    )
    assertEquals(
      "2026-04-27T08:15",
      ExplorerInputPickerValues.formatDateTimeLocal(LocalDateTime.of(2026, 4, 27, 8, 15, 59))
    )
    assertEquals(
      "2026-04",
      ExplorerInputPickerValues.formatMonth(YearMonth.of(2026, 4))
    )
  }

  @Test
  fun parsesAndFormatsIsoWeekValues() {
    val weekDate = ExplorerInputPickerValues.initialWeekDate("2026-W18", LocalDate.of(2026, 1, 1))
    assertEquals(LocalDate.of(2026, 4, 27), weekDate)
    assertEquals("2026-W18", ExplorerInputPickerValues.formatWeek(weekDate))
  }
}
