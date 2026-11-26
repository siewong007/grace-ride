import XCTest
@testable import GraceRideKit

final class GrokBotAndWidgetTests: XCTestCase {

    func testGrokBotSpecParsingEncodingAndSvgExport() {
        XCTAssertEqual(GrokBotSpec.shapes.count, 8)
        XCTAssertEqual(GrokBotSpec.faces.count, 16)
        XCTAssertEqual(GrokBotSpec.colors.count, 12)
        XCTAssertEqual(GrokBotSpec.moves.count, 15)

        let parsed = GrokBotSpec.parse("grokbot:cloud:happy:turquoise")
        XCTAssertEqual(parsed, GrokBotSpec(shape: "cloud", face: "happy", color: "turquoise"))
        XCTAssertEqual(parsed?.encode(), "grokbot:cloud:happy:turquoise")
        XCTAssertEqual(GrokBotSpec.parse("grokbot"), .default)
        XCTAssertNil(GrokBotSpec.parse("grokbot:invalid:happy:blue"))

        let svg = GrokBotGeometry.buildStandaloneSvg(spec: parsed!)
        XCTAssertTrue(svg.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""))
        XCTAssertTrue(svg.contains("id=\"grokbot-mask\""))
        XCTAssertTrue(svg.contains("#2fbfa0"))
    }

    func testAvatarStyleParseAndEncode() {
        let icon = AvatarStyleHelper.encodeAvatarIcon("grok", shape: "cloud", fgColor: "blue")
        XCTAssertEqual(icon, "grok:cloud:blue")
        let decoded = AvatarStyleHelper.parseAvatarIcon(icon)
        XCTAssertEqual(decoded.icon, "grok")
        XCTAssertEqual(decoded.shape, "cloud")
        XCTAssertEqual(decoded.fgColor, "blue")
        XCTAssertEqual(AvatarStyleHelper.encodeAvatarColor("teal", bgShape: "hexagon"), "teal:hexagon")
    }

    func testWidgetSnapshotStalenessWindows() {
        let base = KlTime.dateFromYMD(year: 2026, month: 11, day: 25, hour: 9, minute: 0)!
        let rideSummary = WidgetSummary(
            signedIn: true,
            isRide: true,
            rideId: 7,
            scheduledAt: "2026-11-25 09:00:00",
            badge: "Next Ride",
            title: "Sunday Service",
            subtitle: "Confirmed",
            showReadyButton: true,
            readiness: "unconfirmed"
        )

        // Within 2 hours -> ready button stays visible
        let plus1h = base.addingTimeInterval(3600)
        XCTAssertTrue(WidgetSnapshotStore.freshForDisplay(rideSummary, now: plus1h).showReadyButton)

        // After 2 hours -> ready button hides, title remains
        let plus3h = base.addingTimeInterval(3 * 3600)
        let after2h = WidgetSnapshotStore.freshForDisplay(rideSummary, now: plus3h)
        XCTAssertFalse(after2h.showReadyButton)
        XCTAssertEqual(after2h.title, "Sunday Service")

        // After 6 hours -> cleared to signed-in empty summary
        let plus7h = base.addingTimeInterval(7 * 3600)
        let after6h = WidgetSnapshotStore.freshForDisplay(rideSummary, now: plus7h)
        XCTAssertTrue(after6h.signedIn)
        XCTAssertEqual(after6h.title, "")
    }
}
