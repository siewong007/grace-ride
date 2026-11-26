import XCTest
@testable import GraceRideKit

final class DomainTests: XCTestCase {

    func testMoneyFormatsIntegerCentsWithoutFloats() {
        XCTAssertEqual(Money.rm(0), "RM 0.00")
        XCTAssertEqual(Money.rm(5), "RM 0.05")
        XCTAssertEqual(Money.rm(1250), "RM 12.50")
        XCTAssertEqual(Money.rm(123456), "RM 1,234.56")
        XCTAssertEqual(Money.rm(-850), "-RM 8.50")
    }

    func testKlTimeParsesLocalAndIsoTimestamps() {
        XCTAssertNotNil(KlTime.parse("2026-11-25 09:30:00"))
        XCTAssertNotNil(KlTime.parse("2026-11-25 09:30"))
        XCTAssertNotNil(KlTime.parse("2026-11-25"))
        XCTAssertNotNil(KlTime.parse("2026-11-25T09:30:00+08:00"))
        XCTAssertEqual(KlTime.date("2026-11-25 09:30:00"), "2026-11-25")
        XCTAssertEqual(KlTime.time("2026-11-25 09:30:00"), "09:30")
        XCTAssertTrue(KlTime.display("2026-11-25 09:30:00", lang: "en").contains("09:30"))
        XCTAssertTrue(KlTime.display("2026-11-25 09:30:00", lang: "zh").contains("11月25日"))
    }

    func testKlTimeDayLabelMatchesTodayTomorrowOther() {
        let base = KlTime.dateFromYMD(year: 2026, month: 11, day: 25, hour: 8, minute: 0)!
        XCTAssertEqual(KlTime.dayLabel("2026-11-25 19:00:00", now: base), .today)
        XCTAssertEqual(KlTime.dayLabel("2026-11-26 09:00:00", now: base), .tomorrow)
        XCTAssertEqual(KlTime.dayLabel("2026-11-28 09:00:00", now: base), .other)
    }

    func testAreaMatchLongestMatchAndAsciiBoundaries() {
        let areas = [
            Area(id: 1, name: "SS", active: true),
            Area(id: 2, name: "SS2", active: true),
            Area(id: 3, name: "Damansara Utama", active: true),
            Area(id: 4, name: "旧巴生路", active: true),
            Area(id: 5, name: "Inactive Zone", active: false),
        ]
        XCTAssertEqual(AreaMatch.inferAreaId(address: "12 Jalan SS2/66 Petaling Jaya", areas: areas), 2)
        XCTAssertNil(AreaMatch.inferAreaId(address: "No 4 Jalan SS20", areas: areas))
        XCTAssertEqual(AreaMatch.inferAreaId(address: "Near DAMANSARA   UTAMA mall", areas: areas), 3)
        XCTAssertEqual(AreaMatch.inferAreaId(address: "巴刹路旧巴生路88号", areas: areas), 4)
        XCTAssertNil(AreaMatch.inferAreaId(address: "Inactive Zone Street", areas: areas))
    }

    func testRideStatusAndCancellability() {
        XCTAssertFalse(RideStatus.isTerminal("confirmed"))
        XCTAssertTrue(RideStatus.isTerminal("completed"))
        XCTAssertTrue(RideStatus.isTerminal("cancelled"))
        XCTAssertEqual(RideStatus.kind("in_progress"), .active)
        XCTAssertEqual(RideStatus.kind("completed"), .done)
        XCTAssertEqual(RideStatus.kind("no_show"), .cancelled)
        XCTAssertEqual(RideStatus.kind("pending_driver"), .pending)
        XCTAssertTrue(RideStatus.cancellable("confirmed"))
        XCTAssertFalse(RideStatus.cancellable("in_progress"))
    }

    func testPasskeyOptionsBase64UrlAndQrParsing() {
        let raw = Data([0x01, 0xFF, 0x3E, 0x2F, 0xAA])
        let encoded = PasskeyOptions.encodeBase64URL(raw)
        XCTAssertFalse(encoded.contains("+"))
        XCTAssertFalse(encoded.contains("/"))
        XCTAssertFalse(encoded.contains("="))
        XCTAssertEqual(PasskeyOptions.decodeBase64URL(encoded), raw)

        XCTAssertEqual(QrParsing.parseCheckinInput("ABC123", currentEventId: 10), .code("ABC123"))
        XCTAssertEqual(QrParsing.parseCheckinInput("https://graceride.saliminn.my/events/10/checkin?code=CHURCH99", currentEventId: 10), .code("CHURCH99"))
        XCTAssertEqual(QrParsing.parseCheckinInput("https://graceride.saliminn.my/events/11/checkin?code=CHURCH99", currentEventId: 10), .wrongEvent)
        XCTAssertEqual(QrParsing.parseMemberPassInput("GRPASS:10:MEM42", currentEventId: 10), .code("MEM42"))
        XCTAssertEqual(QrParsing.parseMemberPassInput("GRPASS:12:MEM42", currentEventId: 10), .wrongEvent)
    }
}
