import Foundation

public struct OkResponse: Codable, Equatable, Sendable {
    public var ok: Bool

    public init(ok: Bool = true) {
        self.ok = ok
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
    }

    enum CodingKeys: String, CodingKey { case ok }
}

public struct GroupRef: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: Int64
    public var name: String

    public init(id: Int64, name: String = "") {
        self.id = id
        self.name = name
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        name = try c.decodeOrDefault(String.self, forKey: .name, default: "")
    }

    enum CodingKeys: String, CodingKey { case id, name }
}

public struct Me: Codable, Equatable, Identifiable, Sendable {
    public var id: Int64
    public var name: String
    public var email: String
    public var phone: String
    public var photoFileId: Int64?
    public var avatarColor: String?
    public var avatarIcon: String?
    public var roles: [String]
    public var roleNames: [String: String]
    public var permissions: [String]
    public var status: String
    public var lang: String
    public var termsAccepted: Bool
    public var mfaOk: Bool
    public var totpEnabled: Bool
    public var hasPassword: Bool
    public var passkeyCount: Int
    public var autoLockMins: Int
    public var sessionTimeoutMins: Int
    public var backupCodesCount: Int
    public var locked: Bool
    public var privileged: Bool
    public var unread: Int
    public var groups: [GroupRef]
    public var devMode: Bool
    public var pushOptIn: Bool
    public var onboardingDismissed: Bool
    public var whatsappOptIn: Bool
    public var badgesOptOut: Bool
    public var readinessPromptMins: Int?

    public init(
        id: Int64,
        name: String,
        email: String,
        phone: String = "",
        photoFileId: Int64? = nil,
        avatarColor: String? = nil,
        avatarIcon: String? = nil,
        roles: [String] = [],
        roleNames: [String: String] = [:],
        permissions: [String] = [],
        status: String = "active",
        lang: String = "zh",
        termsAccepted: Bool = true,
        mfaOk: Bool = false,
        totpEnabled: Bool = false,
        hasPassword: Bool = false,
        passkeyCount: Int = 0,
        autoLockMins: Int = 0,
        sessionTimeoutMins: Int = 0,
        backupCodesCount: Int = 0,
        locked: Bool = false,
        privileged: Bool = false,
        unread: Int = 0,
        groups: [GroupRef] = [],
        devMode: Bool = false,
        pushOptIn: Bool = true,
        onboardingDismissed: Bool = false,
        whatsappOptIn: Bool = false,
        badgesOptOut: Bool = false,
        readinessPromptMins: Int? = nil
    ) {
        self.id = id
        self.name = name
        self.email = email
        self.phone = phone
        self.photoFileId = photoFileId
        self.avatarColor = avatarColor
        self.avatarIcon = avatarIcon
        self.roles = roles
        self.roleNames = roleNames
        self.permissions = permissions
        self.status = status
        self.lang = lang
        self.termsAccepted = termsAccepted
        self.mfaOk = mfaOk
        self.totpEnabled = totpEnabled
        self.hasPassword = hasPassword
        self.passkeyCount = passkeyCount
        self.autoLockMins = autoLockMins
        self.sessionTimeoutMins = sessionTimeoutMins
        self.backupCodesCount = backupCodesCount
        self.locked = locked
        self.privileged = privileged
        self.unread = unread
        self.groups = groups
        self.devMode = devMode
        self.pushOptIn = pushOptIn
        self.onboardingDismissed = onboardingDismissed
        self.whatsappOptIn = whatsappOptIn
        self.badgesOptOut = badgesOptOut
        self.readinessPromptMins = readinessPromptMins
    }

    public func can(_ page: String, act: String = "view") -> Bool {
        permissions.contains("\(page).\(act)")
    }

    public var hasStaffAccess: Bool {
        permissions.contains { $0.hasPrefix("admin_") }
    }

    enum CodingKeys: String, CodingKey {
        case id, name, email, phone
        case photoFileId = "photo_file_id"
        case avatarColor = "avatar_color"
        case avatarIcon = "avatar_icon"
        case roles
        case roleNames = "role_names"
        case permissions, status, lang
        case termsAccepted = "terms_accepted"
        case mfaOk = "mfa_ok"
        case totpEnabled = "totp_enabled"
        case hasPassword = "has_password"
        case passkeyCount = "passkey_count"
        case autoLockMins = "auto_lock_mins"
        case sessionTimeoutMins = "session_timeout_mins"
        case backupCodesCount = "backup_codes_count"
        case locked, privileged, unread, groups
        case devMode = "dev_mode"
        case pushOptIn = "push_opt_in"
        case onboardingDismissed = "onboarding_dismissed"
        case whatsappOptIn = "whatsapp_opt_in"
        case badgesOptOut = "badges_opt_out"
        case readinessPromptMins = "readiness_prompt_mins"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        name = try c.decode(String.self, forKey: .name)
        email = try c.decode(String.self, forKey: .email)
        phone = try c.decodeOrDefault(String.self, forKey: .phone, default: "")
        photoFileId = try c.decodeIfPresent(Int64.self, forKey: .photoFileId)
        avatarColor = try c.decodeIfPresent(String.self, forKey: .avatarColor)
        avatarIcon = try c.decodeIfPresent(String.self, forKey: .avatarIcon)
        roles = try c.decodeOrDefault([String].self, forKey: .roles, default: [])
        roleNames = try c.decodeOrDefault([String: String].self, forKey: .roleNames, default: [:])
        permissions = try c.decodeOrDefault([String].self, forKey: .permissions, default: [])
        status = try c.decodeOrDefault(String.self, forKey: .status, default: "active")
        lang = try c.decodeOrDefault(String.self, forKey: .lang, default: "zh")
        termsAccepted = try c.decodeOrDefault(Bool.self, forKey: .termsAccepted, default: true)
        mfaOk = try c.decodeOrDefault(Bool.self, forKey: .mfaOk, default: false)
        totpEnabled = try c.decodeOrDefault(Bool.self, forKey: .totpEnabled, default: false)
        hasPassword = try c.decodeOrDefault(Bool.self, forKey: .hasPassword, default: false)
        passkeyCount = try c.decodeOrDefault(Int.self, forKey: .passkeyCount, default: 0)
        autoLockMins = try c.decodeOrDefault(Int.self, forKey: .autoLockMins, default: 0)
        sessionTimeoutMins = try c.decodeOrDefault(Int.self, forKey: .sessionTimeoutMins, default: 0)
        backupCodesCount = try c.decodeOrDefault(Int.self, forKey: .backupCodesCount, default: 0)
        locked = try c.decodeOrDefault(Bool.self, forKey: .locked, default: false)
        privileged = try c.decodeOrDefault(Bool.self, forKey: .privileged, default: false)
        unread = try c.decodeOrDefault(Int.self, forKey: .unread, default: 0)
        groups = try c.decodeOrDefault([GroupRef].self, forKey: .groups, default: [])
        devMode = try c.decodeOrDefault(Bool.self, forKey: .devMode, default: false)
        pushOptIn = try c.decodeOrDefault(Bool.self, forKey: .pushOptIn, default: true)
        onboardingDismissed = try c.decodeOrDefault(Bool.self, forKey: .onboardingDismissed, default: false)
        whatsappOptIn = try c.decodeOrDefault(Bool.self, forKey: .whatsappOptIn, default: false)
        badgesOptOut = try c.decodeOrDefault(Bool.self, forKey: .badgesOptOut, default: false)
        readinessPromptMins = try c.decodeIfPresent(Int.self, forKey: .readinessPromptMins)
    }
}

public struct DevUser: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: String { email }
    public var name: String
    public var email: String
    public var roles: String

    public init(name: String, email: String, roles: String = "") {
        self.name = name
        self.email = email
        self.roles = roles
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        name = try c.decode(String.self, forKey: .name)
        email = try c.decode(String.self, forKey: .email)
        roles = try c.decodeOrDefault(String.self, forKey: .roles, default: "")
    }

    enum CodingKeys: String, CodingKey { case name, email, roles }
}

public struct TurnstileConfigDto: Codable, Equatable, Sendable {
    public var enabled: Bool
    public var siteKey: String?

    public init(enabled: Bool = false, siteKey: String? = nil) {
        self.enabled = enabled
        self.siteKey = siteKey
    }

    enum CodingKeys: String, CodingKey {
        case enabled
        case siteKey = "site_key"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try c.decodeOrDefault(Bool.self, forKey: .enabled, default: false)
        siteKey = try c.decodeIfPresent(String.self, forKey: .siteKey)
    }
}

public struct PasskeyStartReq: Codable, Equatable, Sendable {
    public var email: String
    public var turnstileToken: String?

    public init(email: String, turnstileToken: String? = nil) {
        self.email = email
        self.turnstileToken = turnstileToken
    }

    enum CodingKeys: String, CodingKey {
        case email
        case turnstileToken = "turnstile_token"
    }
}

public struct PasskeyStart: Codable, Equatable, Sendable {
    public var challengeId: String
    public var options: [String: JSONValue]

    public init(challengeId: String, options: [String: JSONValue]) {
        self.challengeId = challengeId
        self.options = options
    }

    enum CodingKeys: String, CodingKey {
        case challengeId = "challenge_id"
        case options
    }
}

public struct PasskeyFinishReq: Codable, Equatable, Sendable {
    public var challengeId: String
    public var credential: JSONValue
    public var label: String?

    public init(challengeId: String, credential: JSONValue, label: String? = nil) {
        self.challengeId = challengeId
        self.credential = credential
        self.label = label
    }

    enum CodingKeys: String, CodingKey {
        case challengeId = "challenge_id"
        case credential
        case label
    }
}

public struct PasskeyInfo: Codable, Equatable, Identifiable, Sendable {
    public var id: Int64
    public var label: String
    public var createdAt: String
    public var lastUsed: String?

    public init(id: Int64, label: String = "", createdAt: String = "", lastUsed: String? = nil) {
        self.id = id
        self.label = label
        self.createdAt = createdAt
        self.lastUsed = lastUsed
    }

    enum CodingKeys: String, CodingKey {
        case id, label
        case createdAt = "created_at"
        case lastUsed = "last_used"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        label = try c.decodeOrDefault(String.self, forKey: .label, default: "")
        createdAt = try c.decodeOrDefault(String.self, forKey: .createdAt, default: "")
        lastUsed = try c.decodeIfPresent(String.self, forKey: .lastUsed)
    }
}

public struct UnlockReq: Codable, Equatable, Sendable {
    public var totpCode: String?
    public var backupCode: String?
    public var devUnlock: Bool?

    public init(totpCode: String? = nil, backupCode: String? = nil, devUnlock: Bool? = nil) {
        self.totpCode = totpCode
        self.backupCode = backupCode
        self.devUnlock = devUnlock
    }

    enum CodingKeys: String, CodingKey {
        case totpCode = "totp_code"
        case backupCode = "backup_code"
        case devUnlock = "dev_unlock"
    }
}

public struct TermsReq: Codable, Equatable, Sendable {
    public var accepted: Bool
    public init(accepted: Bool) { self.accepted = accepted }
}

public struct Area: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var name: String
    public var active: Bool

    public init(id: Int64, name: String, active: Bool = true) {
        self.id = id
        self.name = name
        self.active = active
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        name = try c.decode(String.self, forKey: .name)
        active = try c.decodeOrDefault(Bool.self, forKey: .active, default: true)
    }

    enum CodingKeys: String, CodingKey { case id, name, active }
}

public struct Place: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: String { "\(label)-\(lat)-\(lng)" }
    public var label: String
    public var lat: Double
    public var lng: Double

    public init(label: String, lat: Double = 0, lng: Double = 0) {
        self.label = label
        self.lat = lat
        self.lng = lng
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        label = try c.decode(String.self, forKey: .label)
        lat = try c.decodeOrDefault(Double.self, forKey: .lat, default: 0)
        lng = try c.decodeOrDefault(Double.self, forKey: .lng, default: 0)
    }

    enum CodingKeys: String, CodingKey { case label, lat, lng }
}

public struct SavedPlace: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var label: String
    public var areaId: Int64?
    public var areaName: String?
    public var address: String
    public var gateNotes: String
    public var isDefault: Int
    public var lat: Double?
    public var lng: Double?

    public init(
        id: Int64,
        label: String,
        areaId: Int64? = nil,
        areaName: String? = nil,
        address: String = "",
        gateNotes: String = "",
        isDefault: Int = 0,
        lat: Double? = nil,
        lng: Double? = nil
    ) {
        self.id = id
        self.label = label
        self.areaId = areaId
        self.areaName = areaName
        self.address = address
        self.gateNotes = gateNotes
        self.isDefault = isDefault
        self.lat = lat
        self.lng = lng
    }

    enum CodingKeys: String, CodingKey {
        case id, label
        case areaId = "area_id"
        case areaName = "area_name"
        case address
        case gateNotes = "gate_notes"
        case isDefault = "is_default"
        case lat, lng
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        label = try c.decode(String.self, forKey: .label)
        areaId = try c.decodeIfPresent(Int64.self, forKey: .areaId)
        areaName = try c.decodeIfPresent(String.self, forKey: .areaName)
        address = try c.decodeOrDefault(String.self, forKey: .address, default: "")
        gateNotes = try c.decodeOrDefault(String.self, forKey: .gateNotes, default: "")
        isDefault = try c.decodeOrDefault(Int.self, forKey: .isDefault, default: 0)
        lat = try c.decodeIfPresent(Double.self, forKey: .lat)
        lng = try c.decodeIfPresent(Double.self, forKey: .lng)
    }
}

public struct SavedPlaceReq: Codable, Equatable, Sendable {
    public var label: String
    public var areaId: Int64?
    public var address: String
    public var gateNotes: String?
    public var isDefault: Bool?
    public var lat: Double?
    public var lng: Double?
    public var clearCoords: Bool?

    public init(
        label: String,
        areaId: Int64? = nil,
        address: String,
        gateNotes: String? = nil,
        isDefault: Bool? = nil,
        lat: Double? = nil,
        lng: Double? = nil,
        clearCoords: Bool? = nil
    ) {
        self.label = label
        self.areaId = areaId
        self.address = address
        self.gateNotes = gateNotes
        self.isDefault = isDefault
        self.lat = lat
        self.lng = lng
        self.clearCoords = clearCoords
    }

    enum CodingKeys: String, CodingKey {
        case label
        case areaId = "area_id"
        case address
        case gateNotes = "gate_notes"
        case isDefault = "is_default"
        case lat, lng
        case clearCoords = "clear_coords"
    }
}

public struct CreateSavedPlaceRes: Codable, Equatable, Sendable {
    public var id: Int64
    public var ok: Bool

    public init(id: Int64, ok: Bool = true) {
        self.id = id
        self.ok = ok
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
    }

    enum CodingKeys: String, CodingKey { case id, ok }
}

public struct RideArea: Codable, Equatable, Hashable, Sendable {
    public var id: Int64
    public var name: String

    public init(id: Int64, name: String = "") {
        self.id = id
        self.name = name
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        name = try c.decodeOrDefault(String.self, forKey: .name, default: "")
    }

    enum CodingKeys: String, CodingKey { case id, name }
}

public struct RideEvent: Codable, Equatable, Hashable, Sendable {
    public var id: Int64?
    public var title: String?
    public var isPrivate: Bool
    public var mileageClaimable: Bool

    public init(id: Int64? = nil, title: String? = nil, isPrivate: Bool = false, mileageClaimable: Bool = false) {
        self.id = id
        self.title = title
        self.isPrivate = isPrivate
        self.mileageClaimable = mileageClaimable
    }

    enum CodingKeys: String, CodingKey {
        case id, title
        case isPrivate = "private"
        case mileageClaimable = "mileage_claimable"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decodeIfPresent(Int64.self, forKey: .id)
        title = try c.decodeIfPresent(String.self, forKey: .title)
        isPrivate = try c.decodeOrDefault(Bool.self, forKey: .isPrivate, default: false)
        mileageClaimable = try c.decodeOrDefault(Bool.self, forKey: .mileageClaimable, default: false)
    }
}

public struct Vehicle: Codable, Equatable, Hashable, Sendable {
    public var model: String
    public var color: String?
    public var plate: String?
    public var type: String?
    public var wheelchairOk: Bool
    public var lowStep: Bool
    public var childSeat: Bool

    public init(
        model: String = "",
        color: String? = nil,
        plate: String? = nil,
        type: String? = nil,
        wheelchairOk: Bool = false,
        lowStep: Bool = false,
        childSeat: Bool = false
    ) {
        self.model = model
        self.color = color
        self.plate = plate
        self.type = type
        self.wheelchairOk = wheelchairOk
        self.lowStep = lowStep
        self.childSeat = childSeat
    }

    enum CodingKeys: String, CodingKey {
        case model, color, plate, type
        case wheelchairOk = "wheelchair_ok"
        case lowStep = "low_step"
        case childSeat = "child_seat"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        model = try c.decodeOrDefault(String.self, forKey: .model, default: "")
        color = try c.decodeIfPresent(String.self, forKey: .color)
        plate = try c.decodeIfPresent(String.self, forKey: .plate)
        type = try c.decodeIfPresent(String.self, forKey: .type)
        wheelchairOk = try c.decodeOrDefault(Bool.self, forKey: .wheelchairOk, default: false)
        lowStep = try c.decodeOrDefault(Bool.self, forKey: .lowStep, default: false)
        childSeat = try c.decodeOrDefault(Bool.self, forKey: .childSeat, default: false)
    }
}

public struct RideDriver: Codable, Equatable, Hashable, Sendable {
    public var id: Int64
    public var name: String?
    public var photoFileId: Int64?
    public var avatarColor: String?
    public var avatarIcon: String?
    public var phone: String?
    public var vehicle: Vehicle?

    public init(
        id: Int64,
        name: String? = nil,
        photoFileId: Int64? = nil,
        avatarColor: String? = nil,
        avatarIcon: String? = nil,
        phone: String? = nil,
        vehicle: Vehicle? = nil
    ) {
        self.id = id
        self.name = name
        self.photoFileId = photoFileId
        self.avatarColor = avatarColor
        self.avatarIcon = avatarIcon
        self.phone = phone
        self.vehicle = vehicle
    }

    enum CodingKeys: String, CodingKey {
        case id, name
        case photoFileId = "photo_file_id"
        case avatarColor = "avatar_color"
        case avatarIcon = "avatar_icon"
        case phone, vehicle
    }
}

public struct RidePassenger: Codable, Equatable, Hashable, Sendable {
    public var id: Int64
    public var name: String
    public var phone: String?

    public init(id: Int64, name: String, phone: String? = nil) {
        self.id = id
        self.name = name
        self.phone = phone
    }
}

public struct RideProxy: Codable, Equatable, Hashable, Sendable {
    public var isProxy: Bool
    public var riderName: String
    public var riderPhone: String?
    public var riderRelationship: String
    public var bookerName: String
    public var bookerPhone: String?

    public init(
        isProxy: Bool = true,
        riderName: String = "",
        riderPhone: String? = nil,
        riderRelationship: String = "",
        bookerName: String = "",
        bookerPhone: String? = nil
    ) {
        self.isProxy = isProxy
        self.riderName = riderName
        self.riderPhone = riderPhone
        self.riderRelationship = riderRelationship
        self.bookerName = bookerName
        self.bookerPhone = bookerPhone
    }

    enum CodingKeys: String, CodingKey {
        case isProxy = "is_proxy"
        case riderName = "rider_name"
        case riderPhone = "rider_phone"
        case riderRelationship = "rider_relationship"
        case bookerName = "booker_name"
        case bookerPhone = "booker_phone"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        isProxy = try c.decodeOrDefault(Bool.self, forKey: .isProxy, default: true)
        riderName = try c.decodeOrDefault(String.self, forKey: .riderName, default: "")
        riderPhone = try c.decodeIfPresent(String.self, forKey: .riderPhone)
        riderRelationship = try c.decodeOrDefault(String.self, forKey: .riderRelationship, default: "")
        bookerName = try c.decodeOrDefault(String.self, forKey: .bookerName, default: "")
        bookerPhone = try c.decodeIfPresent(String.self, forKey: .bookerPhone)
    }
}

public struct RideViewer: Codable, Equatable, Hashable, Sendable {
    public var isPassenger: Bool
    public var isDriver: Bool
    public var isStaff: Bool

    public init(isPassenger: Bool = false, isDriver: Bool = false, isStaff: Bool = false) {
        self.isPassenger = isPassenger
        self.isDriver = isDriver
        self.isStaff = isStaff
    }

    enum CodingKeys: String, CodingKey {
        case isPassenger = "is_passenger"
        case isDriver = "is_driver"
        case isStaff = "is_staff"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        isPassenger = try c.decodeOrDefault(Bool.self, forKey: .isPassenger, default: false)
        isDriver = try c.decodeOrDefault(Bool.self, forKey: .isDriver, default: false)
        isStaff = try c.decodeOrDefault(Bool.self, forKey: .isStaff, default: false)
    }
}

public struct RouteFare: Codable, Equatable, Hashable, Sendable {
    public var from: String
    public var to: String
    public var amountCents: Int64?

    public init(from: String = "", to: String = "", amountCents: Int64? = nil) {
        self.from = from
        self.to = to
        self.amountCents = amountCents
    }

    enum CodingKeys: String, CodingKey {
        case from, to
        case amountCents = "amount_cents"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        from = try c.decodeOrDefault(String.self, forKey: .from, default: "")
        to = try c.decodeOrDefault(String.self, forKey: .to, default: "")
        amountCents = try c.decodeIfPresent(Int64.self, forKey: .amountCents)
    }
}

public struct Ride: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: Int64
    public var status: String
    public var leg: String
    public var pairedRideId: Int64?
    public var pickupAt: String
    public var area: RideArea
    public var pickupAddress: String?
    public var destination: String
    public var seats: Int64
    public var assistance: String
    public var notes: String
    public var event: RideEvent?
    public var asGroup: GroupRef?
    public var tripId: Int64?
    public var routeFare: RouteFare?
    public var distanceKm: Int64?
    public var reimbursementAmountCents: Int64?
    public var mileageReceiptNeeded: Bool
    public var readinessStatus: String
    public var readinessUpdatedAt: String?
    public var delayMinutes: Int
    public var arrivedAtGateAt: String?
    public var supperChoice: String
    public var isProxy: Bool
    public var proxy: RideProxy?
    public var passenger: RidePassenger
    public var driver: RideDriver?
    public var needsAttention: Bool
    public var attentionReason: String
    public var startedAt: String?
    public var pickedUpAt: String?
    public var completedAt: String?
    public var createdAt: String
    public var updatedAt: String
    public var viewer: RideViewer

    public init(
        id: Int64,
        status: String,
        leg: String = "outbound",
        pairedRideId: Int64? = nil,
        pickupAt: String,
        area: RideArea,
        pickupAddress: String? = nil,
        destination: String = "",
        seats: Int64 = 1,
        assistance: String = "",
        notes: String = "",
        event: RideEvent? = nil,
        asGroup: GroupRef? = nil,
        tripId: Int64? = nil,
        routeFare: RouteFare? = nil,
        distanceKm: Int64? = nil,
        reimbursementAmountCents: Int64? = nil,
        mileageReceiptNeeded: Bool = false,
        readinessStatus: String = "unconfirmed",
        readinessUpdatedAt: String? = nil,
        delayMinutes: Int = 0,
        arrivedAtGateAt: String? = nil,
        supperChoice: String = "unspecified",
        isProxy: Bool = false,
        proxy: RideProxy? = nil,
        passenger: RidePassenger,
        driver: RideDriver? = nil,
        needsAttention: Bool = false,
        attentionReason: String = "",
        startedAt: String? = nil,
        pickedUpAt: String? = nil,
        completedAt: String? = nil,
        createdAt: String = "",
        updatedAt: String = "",
        viewer: RideViewer = RideViewer()
    ) {
        self.id = id
        self.status = status
        self.leg = leg
        self.pairedRideId = pairedRideId
        self.pickupAt = pickupAt
        self.area = area
        self.pickupAddress = pickupAddress
        self.destination = destination
        self.seats = seats
        self.assistance = assistance
        self.notes = notes
        self.event = event
        self.asGroup = asGroup
        self.tripId = tripId
        self.routeFare = routeFare
        self.distanceKm = distanceKm
        self.reimbursementAmountCents = reimbursementAmountCents
        self.mileageReceiptNeeded = mileageReceiptNeeded
        self.readinessStatus = readinessStatus
        self.readinessUpdatedAt = readinessUpdatedAt
        self.delayMinutes = delayMinutes
        self.arrivedAtGateAt = arrivedAtGateAt
        self.supperChoice = supperChoice
        self.isProxy = isProxy
        self.proxy = proxy
        self.passenger = passenger
        self.driver = driver
        self.needsAttention = needsAttention
        self.attentionReason = attentionReason
        self.startedAt = startedAt
        self.pickedUpAt = pickedUpAt
        self.completedAt = completedAt
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.viewer = viewer
    }

    enum CodingKeys: String, CodingKey {
        case id, status, leg
        case pairedRideId = "paired_ride_id"
        case pickupAt = "pickup_at"
        case area
        case pickupAddress = "pickup_address"
        case destination, seats, assistance, notes, event
        case asGroup = "as_group"
        case tripId = "trip_id"
        case routeFare = "route_fare"
        case distanceKm = "distance_km"
        case reimbursementAmountCents = "reimbursement_amount_cents"
        case mileageReceiptNeeded = "mileage_receipt_needed"
        case readinessStatus = "readiness_status"
        case readinessUpdatedAt = "readiness_updated_at"
        case delayMinutes = "delay_minutes"
        case arrivedAtGateAt = "arrived_at_gate_at"
        case supperChoice = "supper_choice"
        case isProxy = "is_proxy"
        case proxy, passenger, driver
        case needsAttention = "needs_attention"
        case attentionReason = "attention_reason"
        case startedAt = "started_at"
        case pickedUpAt = "picked_up_at"
        case completedAt = "completed_at"
        case createdAt = "created_at"
        case updatedAt = "updated_at"
        case viewer
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        status = try c.decode(String.self, forKey: .status)
        leg = try c.decodeOrDefault(String.self, forKey: .leg, default: "outbound")
        pairedRideId = try c.decodeIfPresent(Int64.self, forKey: .pairedRideId)
        pickupAt = try c.decode(String.self, forKey: .pickupAt)
        area = try c.decode(RideArea.self, forKey: .area)
        pickupAddress = try c.decodeIfPresent(String.self, forKey: .pickupAddress)
        destination = try c.decodeOrDefault(String.self, forKey: .destination, default: "")
        seats = try c.decodeOrDefault(Int64.self, forKey: .seats, default: 1)
        assistance = try c.decodeOrDefault(String.self, forKey: .assistance, default: "")
        notes = try c.decodeOrDefault(String.self, forKey: .notes, default: "")
        event = try c.decodeIfPresent(RideEvent.self, forKey: .event)
        asGroup = try c.decodeIfPresent(GroupRef.self, forKey: .asGroup)
        tripId = try c.decodeIfPresent(Int64.self, forKey: .tripId)
        routeFare = try c.decodeIfPresent(RouteFare.self, forKey: .routeFare)
        distanceKm = try c.decodeIfPresent(Int64.self, forKey: .distanceKm)
        reimbursementAmountCents = try c.decodeIfPresent(Int64.self, forKey: .reimbursementAmountCents)
        mileageReceiptNeeded = try c.decodeOrDefault(Bool.self, forKey: .mileageReceiptNeeded, default: false)
        readinessStatus = try c.decodeOrDefault(String.self, forKey: .readinessStatus, default: "unconfirmed")
        readinessUpdatedAt = try c.decodeIfPresent(String.self, forKey: .readinessUpdatedAt)
        delayMinutes = try c.decodeOrDefault(Int.self, forKey: .delayMinutes, default: 0)
        arrivedAtGateAt = try c.decodeIfPresent(String.self, forKey: .arrivedAtGateAt)
        supperChoice = try c.decodeOrDefault(String.self, forKey: .supperChoice, default: "unspecified")
        isProxy = try c.decodeOrDefault(Bool.self, forKey: .isProxy, default: false)
        proxy = try c.decodeIfPresent(RideProxy.self, forKey: .proxy)
        passenger = try c.decode(RidePassenger.self, forKey: .passenger)
        driver = try c.decodeIfPresent(RideDriver.self, forKey: .driver)
        needsAttention = try c.decodeOrDefault(Bool.self, forKey: .needsAttention, default: false)
        attentionReason = try c.decodeOrDefault(String.self, forKey: .attentionReason, default: "")
        startedAt = try c.decodeIfPresent(String.self, forKey: .startedAt)
        pickedUpAt = try c.decodeIfPresent(String.self, forKey: .pickedUpAt)
        completedAt = try c.decodeIfPresent(String.self, forKey: .completedAt)
        createdAt = try c.decodeOrDefault(String.self, forKey: .createdAt, default: "")
        updatedAt = try c.decodeOrDefault(String.self, forKey: .updatedAt, default: "")
        viewer = try c.decodeOrDefault(RideViewer.self, forKey: .viewer, default: RideViewer())
    }
}

public struct CreateRideReq: Codable, Equatable, Sendable {
    public var pickupAt: String
    public var pickupAreaId: Int64
    public var pickupAddress: String
    public var destination: String
    public var seats: Int64?
    public var assistance: String?
    public var notes: String?
    public var eventId: Int64?
    public var leg: String?
    public var isProxy: Bool?
    public var riderName: String?
    public var riderPhone: String?
    public var riderRelationship: String?
    public var saveProxyRider: Bool?
    public var returnPickupAt: String?
    public var returnPickupAreaId: Int64?
    public var returnPickupAddress: String?
    public var returnDestination: String?

    public init(
        pickupAt: String,
        pickupAreaId: Int64,
        pickupAddress: String,
        destination: String,
        seats: Int64? = nil,
        assistance: String? = nil,
        notes: String? = nil,
        eventId: Int64? = nil,
        leg: String? = nil,
        isProxy: Bool? = nil,
        riderName: String? = nil,
        riderPhone: String? = nil,
        riderRelationship: String? = nil,
        saveProxyRider: Bool? = nil,
        returnPickupAt: String? = nil,
        returnPickupAreaId: Int64? = nil,
        returnPickupAddress: String? = nil,
        returnDestination: String? = nil
    ) {
        self.pickupAt = pickupAt
        self.pickupAreaId = pickupAreaId
        self.pickupAddress = pickupAddress
        self.destination = destination
        self.seats = seats
        self.assistance = assistance
        self.notes = notes
        self.eventId = eventId
        self.leg = leg
        self.isProxy = isProxy
        self.riderName = riderName
        self.riderPhone = riderPhone
        self.riderRelationship = riderRelationship
        self.saveProxyRider = saveProxyRider
        self.returnPickupAt = returnPickupAt
        self.returnPickupAreaId = returnPickupAreaId
        self.returnPickupAddress = returnPickupAddress
        self.returnDestination = returnDestination
    }

    enum CodingKeys: String, CodingKey {
        case pickupAt = "pickup_at"
        case pickupAreaId = "pickup_area_id"
        case pickupAddress = "pickup_address"
        case destination, seats, assistance, notes
        case eventId = "event_id"
        case leg
        case isProxy = "is_proxy"
        case riderName = "rider_name"
        case riderPhone = "rider_phone"
        case riderRelationship = "rider_relationship"
        case saveProxyRider = "save_proxy_rider"
        case returnPickupAt = "return_pickup_at"
        case returnPickupAreaId = "return_pickup_area_id"
        case returnPickupAddress = "return_pickup_address"
        case returnDestination = "return_destination"
    }
}

public struct CreateRideRes: Codable, Equatable, Sendable {
    public var id: Int64
    public var returnRideId: Int64?

    public init(id: Int64, returnRideId: Int64? = nil) {
        self.id = id
        self.returnRideId = returnRideId
    }

    enum CodingKeys: String, CodingKey {
        case id
        case returnRideId = "return_ride_id"
    }
}

public struct ReadinessReq: Codable, Equatable, Sendable {
    public var status: String
    public var notes: String?
    public init(status: String, notes: String? = nil) {
        self.status = status
        self.notes = notes
    }
}

public struct RespondReq: Codable, Equatable, Sendable {
    public var accept: Bool
    public init(accept: Bool) { self.accept = accept }
}

public struct CancelReq: Codable, Equatable, Sendable {
    public var reason: String?
    public init(reason: String? = nil) { self.reason = reason }
}

public struct DriverDelayReq: Codable, Equatable, Sendable {
    public var delayMinutes: Int
    public init(delayMinutes: Int) { self.delayMinutes = delayMinutes }
    enum CodingKeys: String, CodingKey { case delayMinutes = "delay_minutes" }
}

public struct WithdrawReq: Codable, Equatable, Sendable {
    public var reason: String?
    public init(reason: String? = nil) { self.reason = reason }
}

public struct EventCreator: Codable, Equatable, Hashable, Sendable {
    public var id: Int64
    public var name: String
    public init(id: Int64, name: String) {
        self.id = id
        self.name = name
    }
}

public struct EventRideRef: Codable, Equatable, Hashable, Sendable {
    public var id: Int64
    public var status: String
    public init(id: Int64, status: String) {
        self.id = id
        self.status = status
    }
}

public struct MySignup: Codable, Equatable, Hashable, Sendable {
    public var status: String
    public var needRide: Bool
    public var offerDrive: Bool
    public var selfTransport: Bool
    public var attended: Bool?
    public var passCode: String?
    public var asGroup: GroupRef?

    public init(
        status: String,
        needRide: Bool = false,
        offerDrive: Bool = false,
        selfTransport: Bool = false,
        attended: Bool? = nil,
        passCode: String? = nil,
        asGroup: GroupRef? = nil
    ) {
        self.status = status
        self.needRide = needRide
        self.offerDrive = offerDrive
        self.selfTransport = selfTransport
        self.attended = attended
        self.passCode = passCode
        self.asGroup = asGroup
    }

    enum CodingKeys: String, CodingKey {
        case status
        case needRide = "need_ride"
        case offerDrive = "offer_drive"
        case selfTransport = "self_transport"
        case attended
        case passCode = "pass_code"
        case asGroup = "as_group"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        status = try c.decode(String.self, forKey: .status)
        needRide = try c.decodeOrDefault(Bool.self, forKey: .needRide, default: false)
        offerDrive = try c.decodeOrDefault(Bool.self, forKey: .offerDrive, default: false)
        selfTransport = try c.decodeOrDefault(Bool.self, forKey: .selfTransport, default: false)
        attended = try c.decodeFlexibleBoolIfPresent(forKey: .attended)
        passCode = try c.decodeIfPresent(String.self, forKey: .passCode)
        asGroup = try c.decodeIfPresent(GroupRef.self, forKey: .asGroup)
    }
}

public struct EventItem: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: Int64
    public var title: String
    public var kind: String
    public var icon: String?
    public var description: String
    public var startsAt: String
    public var location: String
    public var capacity: Int64
    public var costNote: String
    public var signupDeadline: String?
    public var status: String
    public var createdBy: EventCreator?
    public var going: Int64
    public var waitlist: Int64
    public var spotsLeft: Int64?
    public var mySignup: MySignup?
    public var myRide: EventRideRef?
    public var myReturnRide: EventRideRef?
    public var groups: String
    public var photoFileId: Int64?
    public var attendanceEnabled: Bool
    public var mileageClaimable: Bool
    public var area: RideArea?
    public var checkinCode: String?
    public var canManage: Bool
    public var canPublish: Bool
    public var canUnpublish: Bool
    public var canCancel: Bool
    public var canSeeSignups: Bool
    public var joinAs: [GroupRef]

    public init(
        id: Int64,
        title: String,
        kind: String = "",
        icon: String? = nil,
        description: String = "",
        startsAt: String,
        location: String = "",
        capacity: Int64 = 0,
        costNote: String = "",
        signupDeadline: String? = nil,
        status: String = "published",
        createdBy: EventCreator? = nil,
        going: Int64 = 0,
        waitlist: Int64 = 0,
        spotsLeft: Int64? = nil,
        mySignup: MySignup? = nil,
        myRide: EventRideRef? = nil,
        myReturnRide: EventRideRef? = nil,
        groups: String = "",
        photoFileId: Int64? = nil,
        attendanceEnabled: Bool = false,
        mileageClaimable: Bool = false,
        area: RideArea? = nil,
        checkinCode: String? = nil,
        canManage: Bool = false,
        canPublish: Bool = false,
        canUnpublish: Bool = false,
        canCancel: Bool = false,
        canSeeSignups: Bool = false,
        joinAs: [GroupRef] = []
    ) {
        self.id = id
        self.title = title
        self.kind = kind
        self.icon = icon
        self.description = description
        self.startsAt = startsAt
        self.location = location
        self.capacity = capacity
        self.costNote = costNote
        self.signupDeadline = signupDeadline
        self.status = status
        self.createdBy = createdBy
        self.going = going
        self.waitlist = waitlist
        self.spotsLeft = spotsLeft
        self.mySignup = mySignup
        self.myRide = myRide
        self.myReturnRide = myReturnRide
        self.groups = groups
        self.photoFileId = photoFileId
        self.attendanceEnabled = attendanceEnabled
        self.mileageClaimable = mileageClaimable
        self.area = area
        self.checkinCode = checkinCode
        self.canManage = canManage
        self.canPublish = canPublish
        self.canUnpublish = canUnpublish
        self.canCancel = canCancel
        self.canSeeSignups = canSeeSignups
        self.joinAs = joinAs
    }

    enum CodingKeys: String, CodingKey {
        case id, title, kind, icon, description
        case startsAt = "starts_at"
        case location, capacity
        case costNote = "cost_note"
        case signupDeadline = "signup_deadline"
        case status
        case createdBy = "created_by"
        case going, waitlist
        case spotsLeft = "spots_left"
        case mySignup = "my_signup"
        case myRide = "my_ride"
        case myReturnRide = "my_return_ride"
        case groups
        case photoFileId = "photo_file_id"
        case attendanceEnabled = "attendance_enabled"
        case mileageClaimable = "mileage_claimable"
        case area
        case checkinCode = "checkin_code"
        case canManage = "can_manage"
        case canPublish = "can_publish"
        case canUnpublish = "can_unpublish"
        case canCancel = "can_cancel"
        case canSeeSignups = "can_see_signups"
        case joinAs = "join_as"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        kind = try c.decodeOrDefault(String.self, forKey: .kind, default: "")
        icon = try c.decodeIfPresent(String.self, forKey: .icon)
        description = try c.decodeOrDefault(String.self, forKey: .description, default: "")
        startsAt = try c.decode(String.self, forKey: .startsAt)
        location = try c.decodeOrDefault(String.self, forKey: .location, default: "")
        capacity = try c.decodeOrDefault(Int64.self, forKey: .capacity, default: 0)
        costNote = try c.decodeOrDefault(String.self, forKey: .costNote, default: "")
        signupDeadline = try c.decodeIfPresent(String.self, forKey: .signupDeadline)
        status = try c.decodeOrDefault(String.self, forKey: .status, default: "published")
        createdBy = try c.decodeIfPresent(EventCreator.self, forKey: .createdBy)
        going = try c.decodeOrDefault(Int64.self, forKey: .going, default: 0)
        waitlist = try c.decodeOrDefault(Int64.self, forKey: .waitlist, default: 0)
        spotsLeft = try c.decodeIfPresent(Int64.self, forKey: .spotsLeft)
        mySignup = try c.decodeIfPresent(MySignup.self, forKey: .mySignup)
        myRide = try c.decodeIfPresent(EventRideRef.self, forKey: .myRide)
        myReturnRide = try c.decodeIfPresent(EventRideRef.self, forKey: .myReturnRide)
        groups = try c.decodeOrDefault(String.self, forKey: .groups, default: "")
        photoFileId = try c.decodeIfPresent(Int64.self, forKey: .photoFileId)
        attendanceEnabled = try c.decodeOrDefault(Bool.self, forKey: .attendanceEnabled, default: false)
        mileageClaimable = try c.decodeOrDefault(Bool.self, forKey: .mileageClaimable, default: false)
        area = try c.decodeIfPresent(RideArea.self, forKey: .area)
        checkinCode = try c.decodeIfPresent(String.self, forKey: .checkinCode)
        canManage = try c.decodeOrDefault(Bool.self, forKey: .canManage, default: false)
        canPublish = try c.decodeOrDefault(Bool.self, forKey: .canPublish, default: false)
        canUnpublish = try c.decodeOrDefault(Bool.self, forKey: .canUnpublish, default: false)
        canCancel = try c.decodeOrDefault(Bool.self, forKey: .canCancel, default: false)
        canSeeSignups = try c.decodeOrDefault(Bool.self, forKey: .canSeeSignups, default: false)
        joinAs = try c.decodeOrDefault([GroupRef].self, forKey: .joinAs, default: [])
    }
}

public struct SignupReq: Codable, Equatable, Sendable {
    public var needRide: Bool?
    public var offerDrive: Bool?
    public var offerSeats: Int64?
    public var asGroupId: Int64?

    public init(needRide: Bool? = nil, offerDrive: Bool? = nil, offerSeats: Int64? = nil, asGroupId: Int64? = nil) {
        self.needRide = needRide
        self.offerDrive = offerDrive
        self.offerSeats = offerSeats
        self.asGroupId = asGroupId
    }

    enum CodingKeys: String, CodingKey {
        case needRide = "need_ride"
        case offerDrive = "offer_drive"
        case offerSeats = "offer_seats"
        case asGroupId = "as_group_id"
    }
}
