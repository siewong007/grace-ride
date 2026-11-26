import Foundation

public struct TodayRidesSnapshot: Codable, Equatable, Sendable {
    public var userId: Int64
    public var savedAtMillis: Int64
    public var rides: [Ride]

    public init(userId: Int64, savedAtMillis: Int64, rides: [Ride]) {
        self.userId = userId
        self.savedAtMillis = savedAtMillis
        self.rides = rides
    }
}

public enum OfflineSnapshotStore {
    private static let key = "today_rides_snapshot_v1"

    public static func save(
        store: SecureStore,
        userId: Int64,
        rides: [Ride],
        nowMillis: Int64 = PersistentCookieJar.currentMillis()
    ) {
        let today = KlTime.today()
        var seen = Set<Int64>()
        var relevant: [Ride] = []
        for ride in rides {
            guard seen.insert(ride.id).inserted else { continue }
            if RideStatus.active.contains(ride.status) || String(ride.pickupAt.prefix(10)) >= today {
                relevant.append(ride)
            }
        }
        guard !relevant.isEmpty else {
            store.remove(key)
            return
        }
        let snap = TodayRidesSnapshot(userId: userId, savedAtMillis: nowMillis, rides: relevant)
        guard let data = try? JSONEncoder().encode(snap),
              let raw = String(data: data, encoding: .utf8) else {
            return
        }
        store.putString(key, raw)
    }

    public static func load(store: SecureStore, userId: Int64) -> TodayRidesSnapshot? {
        guard let raw = store.getString(key), let data = raw.data(using: .utf8),
              let snap = try? JSONDecoder().decode(TodayRidesSnapshot.self, from: data),
              snap.userId == userId,
              !snap.rides.isEmpty else {
            return nil
        }
        return snap
    }

    public static func clear(store: SecureStore) {
        store.remove(key)
    }
}
