package org.pjstatecm.graceride

import org.junit.Assert.assertEquals
import org.junit.Test
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.domain.Money
import org.pjstatecm.graceride.domain.RideStatus
import org.pjstatecm.graceride.domain.RideStatusKind
import java.time.LocalDate

class MoneyTest {
    @Test fun zero() = assertEquals("RM 0.00", Money.rm(0))
    @Test fun senOnly() = assertEquals("RM 0.05", Money.rm(5))
    @Test fun ringgitAndSen() = assertEquals("RM 12.34", Money.rm(1234))
    @Test fun thousandsGrouped() = assertEquals("RM 1,000.00", Money.rm(100000))
    @Test fun negative() = assertEquals("-RM 3.50", Money.rm(-350))
    @Test fun largeAmount() = assertEquals("RM 12,345.67", Money.rm(1234567))
}

class KlTimeTest {
    @Test fun parsesFullStamp() {
        val dt = KlTime.parse("2026-11-21 09:30:00")
        assertEquals(2026, dt?.year)
        assertEquals(9, dt?.hour)
        assertEquals(30, dt?.minute)
    }

    @Test fun parsesMinutePrecision() {
        val dt = KlTime.parse("2026-11-21 09:30")
        assertEquals(30, dt?.minute)
    }

    @Test fun parsesDateOnly() {
        val dt = KlTime.parse("2026-11-21")
        assertEquals(0, dt?.hour)
    }

    @Test fun parsesIsoTSeparator() {
        val dt = KlTime.parse("2026-11-22T07:00")
        assertEquals(7, dt?.hour)
        assertEquals("Sun, 22 Nov · 07:00", KlTime.display("2026-11-22T07:00"))
    }

    @Test fun rejectsGarbage() {
        assertEquals(null, KlTime.parse("not a date"))
        assertEquals(null, KlTime.parse(""))
    }

    @Test fun apiStampFormat() {
        assertEquals("2026-11-21 09:05", KlTime.apiStamp(LocalDate.of(2026, 11, 21), 9, 5))
    }

    @Test fun displayFallsBackToRawText() {
        assertEquals("whenever", KlTime.display("whenever"))
    }
}

class RideStatusTest {
    @Test fun activeStatuses() {
        assertEquals(RideStatusKind.ACTIVE, RideStatus.kind("confirmed"))
        assertEquals(RideStatusKind.ACTIVE, RideStatus.kind("in_progress"))
    }

    @Test fun pendingStatuses() {
        listOf("pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm").forEach {
            assertEquals(RideStatusKind.PENDING, RideStatus.kind(it))
        }
    }

    @Test fun terminalStatuses() {
        assertEquals(RideStatusKind.DONE, RideStatus.kind("completed"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("cancelled"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("no_show"))
        assertEquals(RideStatusKind.CANCELLED, RideStatus.kind("alt_transport"))
    }

    @Test fun cancellableMirrorsBackendGuard() {
        listOf("pending_driver", "pending_passenger", "pending_approval", "pending_driver_confirm", "confirmed").forEach {
            assertEquals(true, RideStatus.cancellable(it))
        }
        listOf("in_progress", "completed", "cancelled", "no_show", "alt_transport").forEach {
            assertEquals(false, RideStatus.cancellable(it))
        }
    }

    @Test fun everyKnownStatusHasALabel() {
        (RideStatus.active + RideStatus.terminal).forEach {
            assertEquals(true, RideStatus.labelRes(it) != 0)
        }
    }
}
