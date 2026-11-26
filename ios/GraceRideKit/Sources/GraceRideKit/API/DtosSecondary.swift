import Foundation

public struct Share: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var settlementId: Int64
    public var title: String
    public var eventTitle: String
    public var amountCents: Int64
    public var status: String
    public var payeeName: String
    public var payeeAccount: String
    public var payeeQrFileId: Int64?
    public var dueDate: String?
    public var note: String
    public var txnRef: String
    public var proofFileId: Int64?

    public init(
        id: Int64,
        settlementId: Int64,
        title: String,
        eventTitle: String = "",
        amountCents: Int64,
        status: String,
        payeeName: String = "",
        payeeAccount: String = "",
        payeeQrFileId: Int64? = nil,
        dueDate: String? = nil,
        note: String = "",
        txnRef: String = "",
        proofFileId: Int64? = nil
    ) {
        self.id = id
        self.settlementId = settlementId
        self.title = title
        self.eventTitle = eventTitle
        self.amountCents = amountCents
        self.status = status
        self.payeeName = payeeName
        self.payeeAccount = payeeAccount
        self.payeeQrFileId = payeeQrFileId
        self.dueDate = dueDate
        self.note = note
        self.txnRef = txnRef
        self.proofFileId = proofFileId
    }

    enum CodingKeys: String, CodingKey {
        case id
        case settlementId = "settlement_id"
        case title
        case eventTitle = "event_title"
        case amountCents = "amount_cents"
        case status
        case payeeName = "payee_name"
        case payeeAccount = "payee_account"
        case payeeQrFileId = "payee_qr_file_id"
        case dueDate = "due_date"
        case note
        case txnRef = "txn_ref"
        case proofFileId = "proof_file_id"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        settlementId = try c.decode(Int64.self, forKey: .settlementId)
        title = try c.decode(String.self, forKey: .title)
        eventTitle = try c.decodeOrDefault(String.self, forKey: .eventTitle, default: "")
        amountCents = try c.decode(Int64.self, forKey: .amountCents)
        status = try c.decode(String.self, forKey: .status)
        payeeName = try c.decodeOrDefault(String.self, forKey: .payeeName, default: "")
        payeeAccount = try c.decodeOrDefault(String.self, forKey: .payeeAccount, default: "")
        payeeQrFileId = try c.decodeIfPresent(Int64.self, forKey: .payeeQrFileId)
        dueDate = try c.decodeIfPresent(String.self, forKey: .dueDate)
        note = try c.decodeOrDefault(String.self, forKey: .note, default: "")
        txnRef = try c.decodeOrDefault(String.self, forKey: .txnRef, default: "")
        proofFileId = try c.decodeIfPresent(Int64.self, forKey: .proofFileId)
    }
}

public struct ProofReq: Codable, Equatable, Sendable {
    public var proofFileId: Int64
    public var txnRef: String
    public var paidDate: String?

    public init(proofFileId: Int64, txnRef: String, paidDate: String? = nil) {
        self.proofFileId = proofFileId
        self.txnRef = txnRef
        self.paidDate = paidDate
    }

    enum CodingKeys: String, CodingKey {
        case proofFileId = "proof_file_id"
        case txnRef = "txn_ref"
        case paidDate = "paid_date"
    }
}

public struct Announcement: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var title: String
    public var body: String
    public var pinned: Bool
    public var by: String
    public var createdAt: String
    public var targeted: Bool
    public var groups: [GroupRef]

    public init(
        id: Int64,
        title: String,
        body: String = "",
        pinned: Bool = false,
        by: String = "",
        createdAt: String = "",
        targeted: Bool = false,
        groups: [GroupRef] = []
    ) {
        self.id = id
        self.title = title
        self.body = body
        self.pinned = pinned
        self.by = by
        self.createdAt = createdAt
        self.targeted = targeted
        self.groups = groups
    }

    enum CodingKeys: String, CodingKey {
        case id, title, body, pinned, by
        case createdAt = "created_at"
        case targeted, groups
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        body = try c.decodeOrDefault(String.self, forKey: .body, default: "")
        pinned = try c.decodeOrDefault(Bool.self, forKey: .pinned, default: false)
        by = try c.decodeOrDefault(String.self, forKey: .by, default: "")
        createdAt = try c.decodeOrDefault(String.self, forKey: .createdAt, default: "")
        targeted = try c.decodeOrDefault(Bool.self, forKey: .targeted, default: false)
        groups = try c.decodeOrDefault([GroupRef].self, forKey: .groups, default: [])
    }
}

public struct NotificationItem: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var title: String
    public var body: String
    public var link: String?
    public var read: Bool
    public var createdAt: String
    public var category: String
    public var eventType: String
    public var priority: String
    public var entityType: String
    public var entityId: Int64?

    public init(
        id: Int64,
        title: String,
        body: String = "",
        link: String? = nil,
        read: Bool = false,
        createdAt: String = "",
        category: String = "general",
        eventType: String = "general",
        priority: String = "normal",
        entityType: String = "",
        entityId: Int64? = nil
    ) {
        self.id = id
        self.title = title
        self.body = body
        self.link = link
        self.read = read
        self.createdAt = createdAt
        self.category = category
        self.eventType = eventType
        self.priority = priority
        self.entityType = entityType
        self.entityId = entityId
    }

    enum CodingKeys: String, CodingKey {
        case id, title, body, link, read
        case createdAt = "created_at"
        case category
        case eventType = "event_type"
        case priority
        case entityType = "entity_type"
        case entityId = "entity_id"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        title = try c.decode(String.self, forKey: .title)
        body = try c.decodeOrDefault(String.self, forKey: .body, default: "")
        link = try c.decodeIfPresent(String.self, forKey: .link)
        read = try c.decodeOrDefault(Bool.self, forKey: .read, default: false)
        createdAt = try c.decodeOrDefault(String.self, forKey: .createdAt, default: "")
        category = try c.decodeOrDefault(String.self, forKey: .category, default: "general")
        eventType = try c.decodeOrDefault(String.self, forKey: .eventType, default: "general")
        priority = try c.decodeOrDefault(String.self, forKey: .priority, default: "normal")
        entityType = try c.decodeOrDefault(String.self, forKey: .entityType, default: "")
        entityId = try c.decodeIfPresent(Int64.self, forKey: .entityId)
    }
}

public struct NotificationPageDto: Codable, Equatable, Sendable {
    public var items: [NotificationItem]
    public var nextBefore: Int64?
    public var unread: Int

    public init(items: [NotificationItem] = [], nextBefore: Int64? = nil, unread: Int = 0) {
        self.items = items
        self.nextBefore = nextBefore
        self.unread = unread
    }

    enum CodingKeys: String, CodingKey {
        case items
        case nextBefore = "next_before"
        case unread
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        items = try c.decodeOrDefault([NotificationItem].self, forKey: .items, default: [])
        nextBefore = try c.decodeIfPresent(Int64.self, forKey: .nextBefore)
        unread = try c.decodeOrDefault(Int.self, forKey: .unread, default: 0)
    }
}

public struct NotificationActionResp: Codable, Equatable, Sendable {
    public var ok: Bool
    public var unread: Int

    public init(ok: Bool = true, unread: Int = 0) {
        self.ok = ok
        self.unread = unread
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        unread = try c.decodeOrDefault(Int.self, forKey: .unread, default: 0)
    }

    enum CodingKeys: String, CodingKey { case ok, unread }
}

public struct ReadAllReq: Codable, Equatable, Sendable {
    public var category: String?
    public init(category: String? = nil) { self.category = category }
}

public struct NotificationPrefsDto: Codable, Equatable, Sendable {
    public var pushOptIn: Bool
    public var whatsappOptIn: Bool
    public var readinessPromptMins: Int
    public var driverRemindMins: Int
    public var mutedCategories: [String]
    public var quietHoursStart: String
    public var quietHoursEnd: String
    public var hideLockDetails: Bool

    public init(
        pushOptIn: Bool = true,
        whatsappOptIn: Bool = false,
        readinessPromptMins: Int = 45,
        driverRemindMins: Int = 60,
        mutedCategories: [String] = [],
        quietHoursStart: String = "",
        quietHoursEnd: String = "",
        hideLockDetails: Bool = false
    ) {
        self.pushOptIn = pushOptIn
        self.whatsappOptIn = whatsappOptIn
        self.readinessPromptMins = readinessPromptMins
        self.driverRemindMins = driverRemindMins
        self.mutedCategories = mutedCategories
        self.quietHoursStart = quietHoursStart
        self.quietHoursEnd = quietHoursEnd
        self.hideLockDetails = hideLockDetails
    }

    enum CodingKeys: String, CodingKey {
        case pushOptIn = "push_opt_in"
        case whatsappOptIn = "whatsapp_opt_in"
        case readinessPromptMins = "readiness_prompt_mins"
        case driverRemindMins = "driver_remind_mins"
        case mutedCategories = "muted_categories"
        case quietHoursStart = "quiet_hours_start"
        case quietHoursEnd = "quiet_hours_end"
        case hideLockDetails = "hide_lock_details"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        pushOptIn = try c.decodeOrDefault(Bool.self, forKey: .pushOptIn, default: true)
        whatsappOptIn = try c.decodeOrDefault(Bool.self, forKey: .whatsappOptIn, default: false)
        readinessPromptMins = try c.decodeOrDefault(Int.self, forKey: .readinessPromptMins, default: 45)
        driverRemindMins = try c.decodeOrDefault(Int.self, forKey: .driverRemindMins, default: 60)
        mutedCategories = try c.decodeOrDefault([String].self, forKey: .mutedCategories, default: [])
        quietHoursStart = try c.decodeOrDefault(String.self, forKey: .quietHoursStart, default: "")
        quietHoursEnd = try c.decodeOrDefault(String.self, forKey: .quietHoursEnd, default: "")
        hideLockDetails = try c.decodeOrDefault(Bool.self, forKey: .hideLockDetails, default: false)
    }
}

public struct UpdateNotificationPrefsReq: Codable, Equatable, Sendable {
    public var pushOptIn: Bool?
    public var whatsappOptIn: Bool?
    public var readinessPromptMins: Int?
    public var driverRemindMins: Int?
    public var mutedCategories: [String]?
    public var quietHoursStart: String?
    public var quietHoursEnd: String?
    public var hideLockDetails: Bool?

    public init(
        pushOptIn: Bool? = nil,
        whatsappOptIn: Bool? = nil,
        readinessPromptMins: Int? = nil,
        driverRemindMins: Int? = nil,
        mutedCategories: [String]? = nil,
        quietHoursStart: String? = nil,
        quietHoursEnd: String? = nil,
        hideLockDetails: Bool? = nil
    ) {
        self.pushOptIn = pushOptIn
        self.whatsappOptIn = whatsappOptIn
        self.readinessPromptMins = readinessPromptMins
        self.driverRemindMins = driverRemindMins
        self.mutedCategories = mutedCategories
        self.quietHoursStart = quietHoursStart
        self.quietHoursEnd = quietHoursEnd
        self.hideLockDetails = hideLockDetails
    }

    enum CodingKeys: String, CodingKey {
        case pushOptIn = "push_opt_in"
        case whatsappOptIn = "whatsapp_opt_in"
        case readinessPromptMins = "readiness_prompt_mins"
        case driverRemindMins = "driver_remind_mins"
        case mutedCategories = "muted_categories"
        case quietHoursStart = "quiet_hours_start"
        case quietHoursEnd = "quiet_hours_end"
        case hideLockDetails = "hide_lock_details"
    }
}

public struct MobilePushTokenReq: Codable, Equatable, Sendable {
    public var platform: String
    public var provider: String
    public var token: String
    public var deviceLabel: String

    public init(
        platform: String = "ios",
        provider: String = "apns",
        token: String,
        deviceLabel: String = ""
    ) {
        self.platform = platform
        self.provider = provider
        self.token = token
        self.deviceLabel = deviceLabel
    }

    enum CodingKeys: String, CodingKey {
        case platform, provider, token
        case deviceLabel = "device_label"
    }
}

public struct MobilePushTokenDel: Codable, Equatable, Sendable {
    public var token: String
    public init(token: String) { self.token = token }
}

public struct LogoutReq: Codable, Equatable, Sendable {
    public var pushEndpoint: String?
    public var mobileToken: String?

    public init(pushEndpoint: String? = nil, mobileToken: String? = nil) {
        self.pushEndpoint = pushEndpoint
        self.mobileToken = mobileToken
    }

    enum CodingKeys: String, CodingKey {
        case pushEndpoint = "push_endpoint"
        case mobileToken = "mobile_token"
    }
}

public struct FileRef: Codable, Equatable, Sendable {
    public var id: Int64
    public var mime: String
    public var name: String

    public init(id: Int64, mime: String = "", name: String = "") {
        self.id = id
        self.mime = mime
        self.name = name
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        mime = try c.decodeOrDefault(String.self, forKey: .mime, default: "")
        name = try c.decodeOrDefault(String.self, forKey: .name, default: "")
    }

    enum CodingKeys: String, CodingKey { case id, mime, name }
}

public struct ProfileUpdateReq: Codable, Equatable, Sendable {
    public var name: String?
    public var phone: String?
    public var lang: String?
    public var autoLockMins: Int?
    public var whatsappOptIn: Bool?
    public var pushOptIn: Bool?
    public var badgesOptOut: Bool?
    public var readinessPromptMins: Int?
    public var photoFileId: Int64?
    public var clearPhoto: Bool?
    public var avatarColor: String?
    public var avatarIcon: String?

    public init(
        name: String? = nil,
        phone: String? = nil,
        lang: String? = nil,
        autoLockMins: Int? = nil,
        whatsappOptIn: Bool? = nil,
        pushOptIn: Bool? = nil,
        badgesOptOut: Bool? = nil,
        readinessPromptMins: Int? = nil,
        photoFileId: Int64? = nil,
        clearPhoto: Bool? = nil,
        avatarColor: String? = nil,
        avatarIcon: String? = nil
    ) {
        self.name = name
        self.phone = phone
        self.lang = lang
        self.autoLockMins = autoLockMins
        self.whatsappOptIn = whatsappOptIn
        self.pushOptIn = pushOptIn
        self.badgesOptOut = badgesOptOut
        self.readinessPromptMins = readinessPromptMins
        self.photoFileId = photoFileId
        self.clearPhoto = clearPhoto
        self.avatarColor = avatarColor
        self.avatarIcon = avatarIcon
    }

    enum CodingKeys: String, CodingKey {
        case name, phone, lang
        case autoLockMins = "auto_lock_mins"
        case whatsappOptIn = "whatsapp_opt_in"
        case pushOptIn = "push_opt_in"
        case badgesOptOut = "badges_opt_out"
        case readinessPromptMins = "readiness_prompt_mins"
        case photoFileId = "photo_file_id"
        case clearPhoto = "clear_photo"
        case avatarColor = "avatar_color"
        case avatarIcon = "avatar_icon"
    }
}

public struct ProxyRider: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var name: String
    public var phone: String
    public var relationship: String
    public var notes: String
    public var areaId: Int64?
    public var pickupAddress: String
    public var gateNotes: String
    public var caregiverUserId: Int64?

    public init(
        id: Int64,
        name: String,
        phone: String = "",
        relationship: String = "",
        notes: String = "",
        areaId: Int64? = nil,
        pickupAddress: String = "",
        gateNotes: String = "",
        caregiverUserId: Int64? = nil
    ) {
        self.id = id
        self.name = name
        self.phone = phone
        self.relationship = relationship
        self.notes = notes
        self.areaId = areaId
        self.pickupAddress = pickupAddress
        self.gateNotes = gateNotes
        self.caregiverUserId = caregiverUserId
    }

    enum CodingKeys: String, CodingKey {
        case id, name, phone, relationship, notes
        case areaId = "area_id"
        case pickupAddress = "pickup_address"
        case gateNotes = "gate_notes"
        case caregiverUserId = "caregiver_user_id"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        name = try c.decode(String.self, forKey: .name)
        phone = try c.decodeOrDefault(String.self, forKey: .phone, default: "")
        relationship = try c.decodeOrDefault(String.self, forKey: .relationship, default: "")
        notes = try c.decodeOrDefault(String.self, forKey: .notes, default: "")
        areaId = try c.decodeIfPresent(Int64.self, forKey: .areaId)
        pickupAddress = try c.decodeOrDefault(String.self, forKey: .pickupAddress, default: "")
        gateNotes = try c.decodeOrDefault(String.self, forKey: .gateNotes, default: "")
        caregiverUserId = try c.decodeIfPresent(Int64.self, forKey: .caregiverUserId)
    }
}

public struct ProxyRiderReq: Codable, Equatable, Sendable {
    public var name: String
    public var phone: String?
    public var relationship: String?
    public var notes: String?
    public var areaId: Int64?
    public var pickupAddress: String?
    public var gateNotes: String?
    public var caregiverUserId: Int64?

    public init(
        name: String,
        phone: String? = nil,
        relationship: String? = nil,
        notes: String? = nil,
        areaId: Int64? = nil,
        pickupAddress: String? = nil,
        gateNotes: String? = nil,
        caregiverUserId: Int64? = nil
    ) {
        self.name = name
        self.phone = phone
        self.relationship = relationship
        self.notes = notes
        self.areaId = areaId
        self.pickupAddress = pickupAddress
        self.gateNotes = gateNotes
        self.caregiverUserId = caregiverUserId
    }

    enum CodingKeys: String, CodingKey {
        case name, phone, relationship, notes
        case areaId = "area_id"
        case pickupAddress = "pickup_address"
        case gateNotes = "gate_notes"
        case caregiverUserId = "caregiver_user_id"
    }
}

public struct CreateProxyRiderRes: Codable, Equatable, Sendable {
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

public struct BadgeTierStep: Codable, Equatable, Hashable, Sendable {
    public var tier: String
    public var target: Int64
    public var reached: Bool

    public init(tier: String, target: Int64, reached: Bool = false) {
        self.tier = tier
        self.target = target
        self.reached = reached
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        tier = try c.decode(String.self, forKey: .tier)
        target = try c.decode(Int64.self, forKey: .target)
        reached = try c.decodeOrDefault(Bool.self, forKey: .reached, default: false)
    }

    enum CodingKeys: String, CodingKey { case tier, target, reached }
}

public struct Badge: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: String { key }
    public var key: String
    public var category: String?
    public var roleMatch: Bool
    public var char: String
    public var icon: String?
    public var name: String
    public var rule: String
    public var charZh: String?
    public var nameZh: String?
    public var ruleZh: String?
    public var progress: Int64
    public var target: Int64
    public var earned: Bool
    public var tier: String
    public var tierLevel: Int
    public var nextTier: String?
    public var nextTarget: Int64?
    public var tiers: [BadgeTierStep]

    public init(
        key: String,
        category: String? = nil,
        roleMatch: Bool = true,
        char: String,
        icon: String? = nil,
        name: String = "",
        rule: String = "",
        charZh: String? = nil,
        nameZh: String? = nil,
        ruleZh: String? = nil,
        progress: Int64 = 0,
        target: Int64 = 0,
        earned: Bool = false,
        tier: String = "none",
        tierLevel: Int = 0,
        nextTier: String? = nil,
        nextTarget: Int64? = nil,
        tiers: [BadgeTierStep] = []
    ) {
        self.key = key
        self.category = category
        self.roleMatch = roleMatch
        self.char = char
        self.icon = icon
        self.name = name
        self.rule = rule
        self.charZh = charZh
        self.nameZh = nameZh
        self.ruleZh = ruleZh
        self.progress = progress
        self.target = target
        self.earned = earned
        self.tier = tier
        self.tierLevel = tierLevel
        self.nextTier = nextTier
        self.nextTarget = nextTarget
        self.tiers = tiers
    }

    enum CodingKeys: String, CodingKey {
        case key, category
        case roleMatch = "role_match"
        case char, icon, name, rule
        case charZh = "char_zh"
        case nameZh = "name_zh"
        case ruleZh = "rule_zh"
        case progress, target, earned, tier
        case tierLevel = "tier_level"
        case nextTier = "next_tier"
        case nextTarget = "next_target"
        case tiers
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        key = try c.decode(String.self, forKey: .key)
        category = try c.decodeIfPresent(String.self, forKey: .category)
        roleMatch = try c.decodeOrDefault(Bool.self, forKey: .roleMatch, default: true)
        char = try c.decode(String.self, forKey: .char)
        icon = try c.decodeIfPresent(String.self, forKey: .icon)
        name = try c.decodeOrDefault(String.self, forKey: .name, default: "")
        rule = try c.decodeOrDefault(String.self, forKey: .rule, default: "")
        charZh = try c.decodeIfPresent(String.self, forKey: .charZh)
        nameZh = try c.decodeIfPresent(String.self, forKey: .nameZh)
        ruleZh = try c.decodeIfPresent(String.self, forKey: .ruleZh)
        progress = try c.decodeOrDefault(Int64.self, forKey: .progress, default: 0)
        target = try c.decodeOrDefault(Int64.self, forKey: .target, default: 0)
        earned = try c.decodeOrDefault(Bool.self, forKey: .earned, default: false)
        tier = try c.decodeOrDefault(String.self, forKey: .tier, default: "none")
        tierLevel = try c.decodeOrDefault(Int.self, forKey: .tierLevel, default: 0)
        nextTier = try c.decodeIfPresent(String.self, forKey: .nextTier)
        nextTarget = try c.decodeIfPresent(Int64.self, forKey: .nextTarget)
        tiers = try c.decodeOrDefault([BadgeTierStep].self, forKey: .tiers, default: [])
    }
}

public struct DriverVehicle: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: Int64?
    public var model: String
    public var color: String
    public var plate: String
    public var seats: Int
    public var vehicleType: String
    public var wheelchairOk: Bool
    public var lowStep: Bool
    public var childSeat: Bool
    public var isActive: Bool

    public init(
        id: Int64? = nil,
        model: String = "",
        color: String = "",
        plate: String = "",
        seats: Int = 5,
        vehicleType: String = "standard",
        wheelchairOk: Bool = false,
        lowStep: Bool = false,
        childSeat: Bool = false,
        isActive: Bool = false
    ) {
        self.id = id
        self.model = model
        self.color = color
        self.plate = plate
        self.seats = seats
        self.vehicleType = vehicleType
        self.wheelchairOk = wheelchairOk
        self.lowStep = lowStep
        self.childSeat = childSeat
        self.isActive = isActive
    }

    enum CodingKeys: String, CodingKey {
        case id, model, color, plate, seats
        case vehicleType = "vehicle_type"
        case wheelchairOk = "wheelchair_ok"
        case lowStep = "low_step"
        case childSeat = "child_seat"
        case isActive = "is_active"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decodeIfPresent(Int64.self, forKey: .id)
        model = try c.decodeOrDefault(String.self, forKey: .model, default: "")
        color = try c.decodeOrDefault(String.self, forKey: .color, default: "")
        plate = try c.decodeOrDefault(String.self, forKey: .plate, default: "")
        seats = try c.decodeOrDefault(Int.self, forKey: .seats, default: 5)
        vehicleType = try c.decodeOrDefault(String.self, forKey: .vehicleType, default: "standard")
        wheelchairOk = try c.decodeOrDefault(Bool.self, forKey: .wheelchairOk, default: false)
        lowStep = try c.decodeOrDefault(Bool.self, forKey: .lowStep, default: false)
        childSeat = try c.decodeOrDefault(Bool.self, forKey: .childSeat, default: false)
        isActive = try c.decodeOrDefault(Bool.self, forKey: .isActive, default: false)
    }
}

public struct DriverProfile: Codable, Equatable, Sendable {
    public var carpoolEnabled: Bool
    public var spareSeats: Int
    public var gateWaitMins: Int
    public var areaIds: [Int64]
    public var vehicle: DriverVehicle?
    public var vehicles: [DriverVehicle]
    public var activeVehicleId: Int64?

    public init(
        carpoolEnabled: Bool = false,
        spareSeats: Int = 1,
        gateWaitMins: Int = 5,
        areaIds: [Int64] = [],
        vehicle: DriverVehicle? = nil,
        vehicles: [DriverVehicle] = [],
        activeVehicleId: Int64? = nil
    ) {
        self.carpoolEnabled = carpoolEnabled
        self.spareSeats = spareSeats
        self.gateWaitMins = gateWaitMins
        self.areaIds = areaIds
        self.vehicle = vehicle
        self.vehicles = vehicles
        self.activeVehicleId = activeVehicleId
    }

    enum CodingKeys: String, CodingKey {
        case carpoolEnabled = "carpool_enabled"
        case spareSeats = "spare_seats"
        case gateWaitMins = "gate_wait_mins"
        case areaIds = "area_ids"
        case vehicle, vehicles
        case activeVehicleId = "active_vehicle_id"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        carpoolEnabled = try c.decodeOrDefault(Bool.self, forKey: .carpoolEnabled, default: false)
        spareSeats = try c.decodeOrDefault(Int.self, forKey: .spareSeats, default: 1)
        gateWaitMins = try c.decodeOrDefault(Int.self, forKey: .gateWaitMins, default: 5)
        areaIds = try c.decodeOrDefault([Int64].self, forKey: .areaIds, default: [])
        vehicle = try c.decodeIfPresent(DriverVehicle.self, forKey: .vehicle)
        vehicles = try c.decodeOrDefault([DriverVehicle].self, forKey: .vehicles, default: [])
        activeVehicleId = try c.decodeIfPresent(Int64.self, forKey: .activeVehicleId)
    }
}

public struct DriverProfileReq: Codable, Equatable, Sendable {
    public var carpoolEnabled: Bool
    public var spareSeats: Int
    public var gateWaitMins: Int
    public var areaIds: [Int64]
    public var activeVehicleId: Int64?

    public init(
        carpoolEnabled: Bool,
        spareSeats: Int,
        gateWaitMins: Int,
        areaIds: [Int64],
        activeVehicleId: Int64? = nil
    ) {
        self.carpoolEnabled = carpoolEnabled
        self.spareSeats = spareSeats
        self.gateWaitMins = gateWaitMins
        self.areaIds = areaIds
        self.activeVehicleId = activeVehicleId
    }

    enum CodingKeys: String, CodingKey {
        case carpoolEnabled = "carpool_enabled"
        case spareSeats = "spare_seats"
        case gateWaitMins = "gate_wait_mins"
        case areaIds = "area_ids"
        case activeVehicleId = "active_vehicle_id"
    }
}

public struct DriverVehicleReq: Codable, Equatable, Sendable {
    public var model: String
    public var color: String
    public var plate: String
    public var seats: Int
    public var vehicleType: String
    public var wheelchairOk: Bool
    public var lowStep: Bool
    public var childSeat: Bool
    public var isActive: Bool

    public init(
        model: String,
        color: String,
        plate: String,
        seats: Int,
        vehicleType: String = "standard",
        wheelchairOk: Bool = false,
        lowStep: Bool = false,
        childSeat: Bool = false,
        isActive: Bool = false
    ) {
        self.model = model
        self.color = color
        self.plate = plate
        self.seats = seats
        self.vehicleType = vehicleType
        self.wheelchairOk = wheelchairOk
        self.lowStep = lowStep
        self.childSeat = childSeat
        self.isActive = isActive
    }

    enum CodingKeys: String, CodingKey {
        case model, color, plate, seats
        case vehicleType = "vehicle_type"
        case wheelchairOk = "wheelchair_ok"
        case lowStep = "low_step"
        case childSeat = "child_seat"
        case isActive = "is_active"
    }
}

public struct SelectVehicleRes: Codable, Equatable, Sendable {
    public var ok: Bool
    public var activeVehicleId: Int64?

    public init(ok: Bool = true, activeVehicleId: Int64? = nil) {
        self.ok = ok
        self.activeVehicleId = activeVehicleId
    }

    enum CodingKeys: String, CodingKey {
        case ok
        case activeVehicleId = "active_vehicle_id"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        activeVehicleId = try c.decodeIfPresent(Int64.self, forKey: .activeVehicleId)
    }
}

// MARK: - Trip Group Chat

public struct ChatReactionDto: Codable, Equatable, Hashable, Sendable {
    public var emoji: String
    public var count: Int
    public var mine: Bool
    public var names: [String]

    public init(emoji: String, count: Int = 0, mine: Bool = false, names: [String] = []) {
        self.emoji = emoji
        self.count = count
        self.mine = mine
        self.names = names
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        emoji = try c.decode(String.self, forKey: .emoji)
        count = try c.decodeOrDefault(Int.self, forKey: .count, default: 0)
        mine = try c.decodeOrDefault(Bool.self, forKey: .mine, default: false)
        names = try c.decodeOrDefault([String].self, forKey: .names, default: [])
    }

    enum CodingKeys: String, CodingKey { case emoji, count, mine, names }
}

public struct ChatMessageDto: Codable, Equatable, Identifiable, Hashable, Sendable {
    public var id: Int64
    public var userId: Int64?
    public var name: String?
    public var body: String
    public var createdAt: String
    public var mine: Bool
    public var status: String?
    public var reactions: [ChatReactionDto]

    public init(
        id: Int64,
        userId: Int64? = nil,
        name: String? = nil,
        body: String = "",
        createdAt: String = "",
        mine: Bool = false,
        status: String? = nil,
        reactions: [ChatReactionDto] = []
    ) {
        self.id = id
        self.userId = userId
        self.name = name
        self.body = body
        self.createdAt = createdAt
        self.mine = mine
        self.status = status
        self.reactions = reactions
    }

    enum CodingKeys: String, CodingKey {
        case id
        case userId = "user_id"
        case name, body
        case createdAt = "created_at"
        case mine, status, reactions
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        userId = try c.decodeIfPresent(Int64.self, forKey: .userId)
        name = try c.decodeIfPresent(String.self, forKey: .name)
        body = try c.decodeOrDefault(String.self, forKey: .body, default: "")
        createdAt = try c.decodeOrDefault(String.self, forKey: .createdAt, default: "")
        mine = try c.decodeOrDefault(Bool.self, forKey: .mine, default: false)
        status = try c.decodeIfPresent(String.self, forKey: .status)
        reactions = try c.decodeOrDefault([ChatReactionDto].self, forKey: .reactions, default: [])
    }
}

public struct ChatThreadDto: Codable, Equatable, Sendable {
    public var open: Bool
    public var canPost: Bool
    public var members: [String]
    public var reactionsAllowed: [String]
    public var messages: [ChatMessageDto]

    public init(
        open: Bool = false,
        canPost: Bool = true,
        members: [String] = [],
        reactionsAllowed: [String] = [],
        messages: [ChatMessageDto] = []
    ) {
        self.open = open
        self.canPost = canPost
        self.members = members
        self.reactionsAllowed = reactionsAllowed
        self.messages = messages
    }

    enum CodingKeys: String, CodingKey {
        case open
        case canPost = "can_post"
        case members
        case reactionsAllowed = "reactions_allowed"
        case messages
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        open = try c.decodeOrDefault(Bool.self, forKey: .open, default: false)
        canPost = try c.decodeOrDefault(Bool.self, forKey: .canPost, default: true)
        members = try c.decodeOrDefault([String].self, forKey: .members, default: [])
        reactionsAllowed = try c.decodeOrDefault([String].self, forKey: .reactionsAllowed, default: [])
        messages = try c.decodeOrDefault([ChatMessageDto].self, forKey: .messages, default: [])
    }
}

public struct PostMessageReq: Codable, Equatable, Sendable {
    public var body: String
    public init(body: String) { self.body = body }
}

public struct PostMessageRes: Codable, Equatable, Sendable {
    public var ok: Bool
    public var id: Int64

    public init(ok: Bool = true, id: Int64 = 0) {
        self.ok = ok
        self.id = id
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        id = try c.decodeOrDefault(Int64.self, forKey: .id, default: 0)
    }

    enum CodingKeys: String, CodingKey { case ok, id }
}

public struct MarkReadReq: Codable, Equatable, Sendable {
    public var upTo: Int64
    public init(upTo: Int64) { self.upTo = upTo }
    enum CodingKeys: String, CodingKey { case upTo = "up_to" }
}

public struct ReactReq: Codable, Equatable, Sendable {
    public var emoji: String
    public init(emoji: String) { self.emoji = emoji }
}

public struct ReactRes: Codable, Equatable, Sendable {
    public var ok: Bool
    public var emoji: String?

    public init(ok: Bool = true, emoji: String? = nil) {
        self.ok = ok
        self.emoji = emoji
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        emoji = try c.decodeIfPresent(String.self, forKey: .emoji)
    }

    enum CodingKeys: String, CodingKey { case ok, emoji }
}

// MARK: - Driver Hub Route & Live Location

public struct RouteOriginDto: Codable, Equatable, Hashable, Sendable {
    public var lat: Double
    public var lng: Double
    public var heading: Double?
    public var updatedAt: String

    public init(lat: Double, lng: Double, heading: Double? = nil, updatedAt: String = "") {
        self.lat = lat
        self.lng = lng
        self.heading = heading
        self.updatedAt = updatedAt
    }

    enum CodingKeys: String, CodingKey {
        case lat, lng, heading
        case updatedAt = "updated_at"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        lat = try c.decode(Double.self, forKey: .lat)
        lng = try c.decode(Double.self, forKey: .lng)
        heading = try c.decodeIfPresent(Double.self, forKey: .heading)
        updatedAt = try c.decodeOrDefault(String.self, forKey: .updatedAt, default: "")
    }
}

public struct RouteStopDto: Codable, Equatable, Hashable, Identifiable, Sendable {
    public var id: Int64 { rideId }
    public var rideId: Int64
    public var passenger: String
    public var address: String
    public var pickupAt: String
    public var seats: Int64
    public var started: Bool
    public var lat: Double?
    public var lng: Double?
    public var location: String?
    public var legDistanceM: Double?
    public var legDurationS: Double?
    public var distanceM: Double?
    public var durationS: Double?
    public var eta: String?

    public init(
        rideId: Int64,
        passenger: String = "",
        address: String = "",
        pickupAt: String = "",
        seats: Int64 = 1,
        started: Bool = false,
        lat: Double? = nil,
        lng: Double? = nil,
        location: String? = nil,
        legDistanceM: Double? = nil,
        legDurationS: Double? = nil,
        distanceM: Double? = nil,
        durationS: Double? = nil,
        eta: String? = nil
    ) {
        self.rideId = rideId
        self.passenger = passenger
        self.address = address
        self.pickupAt = pickupAt
        self.seats = seats
        self.started = started
        self.lat = lat
        self.lng = lng
        self.location = location
        self.legDistanceM = legDistanceM
        self.legDurationS = legDurationS
        self.distanceM = distanceM
        self.durationS = durationS
        self.eta = eta
    }

    enum CodingKeys: String, CodingKey {
        case rideId = "ride_id"
        case passenger, address
        case pickupAt = "pickup_at"
        case seats, started, lat, lng, location
        case legDistanceM = "leg_distance_m"
        case legDurationS = "leg_duration_s"
        case distanceM = "distance_m"
        case durationS = "duration_s"
        case eta
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        rideId = try c.decode(Int64.self, forKey: .rideId)
        passenger = try c.decodeOrDefault(String.self, forKey: .passenger, default: "")
        address = try c.decodeOrDefault(String.self, forKey: .address, default: "")
        pickupAt = try c.decodeOrDefault(String.self, forKey: .pickupAt, default: "")
        seats = try c.decodeOrDefault(Int64.self, forKey: .seats, default: 1)
        started = try c.decodeOrDefault(Bool.self, forKey: .started, default: false)
        lat = try c.decodeIfPresent(Double.self, forKey: .lat)
        lng = try c.decodeIfPresent(Double.self, forKey: .lng)
        location = try c.decodeIfPresent(String.self, forKey: .location)
        legDistanceM = try c.decodeIfPresent(Double.self, forKey: .legDistanceM)
        legDurationS = try c.decodeIfPresent(Double.self, forKey: .legDurationS)
        distanceM = try c.decodeIfPresent(Double.self, forKey: .distanceM)
        durationS = try c.decodeIfPresent(Double.self, forKey: .durationS)
        eta = try c.decodeIfPresent(String.self, forKey: .eta)
    }
}

public struct RouteDestinationDto: Codable, Equatable, Hashable, Sendable {
    public var name: String
    public var lat: Double?
    public var lng: Double?
    public var approx: Bool
    public var distanceM: Double?
    public var durationS: Double?
    public var eta: String?

    public init(
        name: String = "",
        lat: Double? = nil,
        lng: Double? = nil,
        approx: Bool = false,
        distanceM: Double? = nil,
        durationS: Double? = nil,
        eta: String? = nil
    ) {
        self.name = name
        self.lat = lat
        self.lng = lng
        self.approx = approx
        self.distanceM = distanceM
        self.durationS = durationS
        self.eta = eta
    }

    enum CodingKeys: String, CodingKey {
        case name, lat, lng, approx
        case distanceM = "distance_m"
        case durationS = "duration_s"
        case eta
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        name = try c.decodeOrDefault(String.self, forKey: .name, default: "")
        lat = try c.decodeIfPresent(Double.self, forKey: .lat)
        lng = try c.decodeIfPresent(Double.self, forKey: .lng)
        approx = try c.decodeOrDefault(Bool.self, forKey: .approx, default: false)
        distanceM = try c.decodeIfPresent(Double.self, forKey: .distanceM)
        durationS = try c.decodeIfPresent(Double.self, forKey: .durationS)
        eta = try c.decodeIfPresent(String.self, forKey: .eta)
    }
}

public struct TripRouteDto: Codable, Equatable, Hashable, Sendable {
    public var tripId: Int64
    public var live: Bool
    public var origin: RouteOriginDto?
    public var stops: [RouteStopDto]
    public var onboard: Int64
    public var destination: RouteDestinationDto
    public var distanceM: Double
    public var durationS: Double
    public var approximate: Bool
    public var computedAt: String

    public init(
        tripId: Int64,
        live: Bool = false,
        origin: RouteOriginDto? = nil,
        stops: [RouteStopDto] = [],
        onboard: Int64 = 0,
        destination: RouteDestinationDto = RouteDestinationDto(),
        distanceM: Double = 0,
        durationS: Double = 0,
        approximate: Bool = false,
        computedAt: String = ""
    ) {
        self.tripId = tripId
        self.live = live
        self.origin = origin
        self.stops = stops
        self.onboard = onboard
        self.destination = destination
        self.distanceM = distanceM
        self.durationS = durationS
        self.approximate = approximate
        self.computedAt = computedAt
    }

    enum CodingKeys: String, CodingKey {
        case tripId = "trip_id"
        case live, origin, stops, onboard, destination
        case distanceM = "distance_m"
        case durationS = "duration_s"
        case approximate
        case computedAt = "computed_at"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        tripId = try c.decode(Int64.self, forKey: .tripId)
        live = try c.decodeOrDefault(Bool.self, forKey: .live, default: false)
        origin = try c.decodeIfPresent(RouteOriginDto.self, forKey: .origin)
        stops = try c.decodeOrDefault([RouteStopDto].self, forKey: .stops, default: [])
        onboard = try c.decodeOrDefault(Int64.self, forKey: .onboard, default: 0)
        destination = try c.decodeOrDefault(RouteDestinationDto.self, forKey: .destination, default: RouteDestinationDto())
        distanceM = try c.decodeOrDefault(Double.self, forKey: .distanceM, default: 0)
        durationS = try c.decodeOrDefault(Double.self, forKey: .durationS, default: 0)
        approximate = try c.decodeOrDefault(Bool.self, forKey: .approximate, default: false)
        computedAt = try c.decodeOrDefault(String.self, forKey: .computedAt, default: "")
    }
}

public struct DriverRouteResp: Codable, Equatable, Sendable {
    public var route: TripRouteDto?
    public init(route: TripRouteDto? = nil) { self.route = route }
}

public struct DriverLocationReq: Codable, Equatable, Sendable {
    public var lat: Double
    public var lng: Double
    public var heading: Double?

    public init(lat: Double, lng: Double, heading: Double? = nil) {
        self.lat = lat
        self.lng = lng
        self.heading = heading
    }
}

public struct PassengerLocationReq: Codable, Equatable, Sendable {
    public var lat: Double
    public var lng: Double
    public init(lat: Double, lng: Double) {
        self.lat = lat
        self.lng = lng
    }
}

public struct LocationShareResp: Codable, Equatable, Sendable {
    public var ok: Bool
    public var sharing: Bool

    public init(ok: Bool = true, sharing: Bool = false) {
        self.ok = ok
        self.sharing = sharing
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        sharing = try c.decodeOrDefault(Bool.self, forKey: .sharing, default: false)
    }

    enum CodingKeys: String, CodingKey { case ok, sharing }
}

public struct PassengerSpotDto: Codable, Equatable, Hashable, Sendable {
    public var lat: Double
    public var lng: Double
    public var updatedAt: String

    public init(lat: Double, lng: Double, updatedAt: String = "") {
        self.lat = lat
        self.lng = lng
        self.updatedAt = updatedAt
    }

    enum CodingKeys: String, CodingKey {
        case lat, lng
        case updatedAt = "updated_at"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        lat = try c.decode(Double.self, forKey: .lat)
        lng = try c.decode(Double.self, forKey: .lng)
        updatedAt = try c.decodeOrDefault(String.self, forKey: .updatedAt, default: "")
    }
}

public struct RideEtaDto: Codable, Equatable, Hashable, Sendable {
    public var phase: String
    public var located: Bool
    public var stopsBefore: Int
    public var approximate: Bool
    public var computedAt: String
    public var distanceM: Double?
    public var durationS: Double?
    public var eta: String?
    public var destination: String?
    public var location: String?

    public init(
        phase: String = "pickup",
        located: Bool = false,
        stopsBefore: Int = 0,
        approximate: Bool = false,
        computedAt: String = "",
        distanceM: Double? = nil,
        durationS: Double? = nil,
        eta: String? = nil,
        destination: String? = nil,
        location: String? = nil
    ) {
        self.phase = phase
        self.located = located
        self.stopsBefore = stopsBefore
        self.approximate = approximate
        self.computedAt = computedAt
        self.distanceM = distanceM
        self.durationS = durationS
        self.eta = eta
        self.destination = destination
        self.location = location
    }

    enum CodingKeys: String, CodingKey {
        case phase, located
        case stopsBefore = "stops_before"
        case approximate
        case computedAt = "computed_at"
        case distanceM = "distance_m"
        case durationS = "duration_s"
        case eta, destination, location
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        phase = try c.decodeOrDefault(String.self, forKey: .phase, default: "pickup")
        located = try c.decodeOrDefault(Bool.self, forKey: .located, default: false)
        stopsBefore = try c.decodeOrDefault(Int.self, forKey: .stopsBefore, default: 0)
        approximate = try c.decodeOrDefault(Bool.self, forKey: .approximate, default: false)
        computedAt = try c.decodeOrDefault(String.self, forKey: .computedAt, default: "")
        distanceM = try c.decodeIfPresent(Double.self, forKey: .distanceM)
        durationS = try c.decodeIfPresent(Double.self, forKey: .durationS)
        eta = try c.decodeIfPresent(String.self, forKey: .eta)
        destination = try c.decodeIfPresent(String.self, forKey: .destination)
        location = try c.decodeIfPresent(String.self, forKey: .location)
    }
}

public struct RideLocationDto: Codable, Equatable, Hashable, Sendable {
    public var sharing: Bool
    public var lat: Double?
    public var lng: Double?
    public var heading: Double?
    public var updatedAt: String?
    public var passenger: PassengerSpotDto?
    public var eta: RideEtaDto?

    public init(
        sharing: Bool = false,
        lat: Double? = nil,
        lng: Double? = nil,
        heading: Double? = nil,
        updatedAt: String? = nil,
        passenger: PassengerSpotDto? = nil,
        eta: RideEtaDto? = nil
    ) {
        self.sharing = sharing
        self.lat = lat
        self.lng = lng
        self.heading = heading
        self.updatedAt = updatedAt
        self.passenger = passenger
        self.eta = eta
    }

    enum CodingKeys: String, CodingKey {
        case sharing, lat, lng, heading
        case updatedAt = "updated_at"
        case passenger, eta
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        sharing = try c.decodeOrDefault(Bool.self, forKey: .sharing, default: false)
        lat = try c.decodeIfPresent(Double.self, forKey: .lat)
        lng = try c.decodeIfPresent(Double.self, forKey: .lng)
        heading = try c.decodeIfPresent(Double.self, forKey: .heading)
        updatedAt = try c.decodeIfPresent(String.self, forKey: .updatedAt)
        passenger = try c.decodeIfPresent(PassengerSpotDto.self, forKey: .passenger)
        eta = try c.decodeIfPresent(RideEtaDto.self, forKey: .eta)
    }
}

// MARK: - Event Check-In & Attendance

public struct CheckinInfoDto: Codable, Equatable, Sendable {
    public var id: Int64
    public var title: String
    public var startsAt: String
    public var location: String
    public var attendanceEnabled: Bool
    public var myAttended: Bool
    public var isMember: Bool

    public init(
        id: Int64,
        title: String = "",
        startsAt: String = "",
        location: String = "",
        attendanceEnabled: Bool = false,
        myAttended: Bool = false,
        isMember: Bool = false
    ) {
        self.id = id
        self.title = title
        self.startsAt = startsAt
        self.location = location
        self.attendanceEnabled = attendanceEnabled
        self.myAttended = myAttended
        self.isMember = isMember
    }

    enum CodingKeys: String, CodingKey {
        case id, title
        case startsAt = "starts_at"
        case location
        case attendanceEnabled = "attendance_enabled"
        case myAttended = "my_attended"
        case isMember = "is_member"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        id = try c.decode(Int64.self, forKey: .id)
        title = try c.decodeOrDefault(String.self, forKey: .title, default: "")
        startsAt = try c.decodeOrDefault(String.self, forKey: .startsAt, default: "")
        location = try c.decodeOrDefault(String.self, forKey: .location, default: "")
        attendanceEnabled = try c.decodeOrDefault(Bool.self, forKey: .attendanceEnabled, default: false)
        myAttended = try c.decodeOrDefault(Bool.self, forKey: .myAttended, default: false)
        isMember = try c.decodeOrDefault(Bool.self, forKey: .isMember, default: false)
    }
}

public struct CheckinReq: Codable, Equatable, Sendable {
    public var code: String?
    public init(code: String? = nil) { self.code = code }
}

public struct CheckinByCodeReq: Codable, Equatable, Sendable {
    public var code: String
    public init(code: String) { self.code = code }
}

public struct CheckinResp: Codable, Equatable, Sendable {
    public var ok: Bool
    public var alreadyAttended: Bool
    public var eventId: Int64
    public var title: String
    public var startsAt: String
    public var location: String

    public init(
        ok: Bool = true,
        alreadyAttended: Bool = false,
        eventId: Int64 = 0,
        title: String = "",
        startsAt: String = "",
        location: String = ""
    ) {
        self.ok = ok
        self.alreadyAttended = alreadyAttended
        self.eventId = eventId
        self.title = title
        self.startsAt = startsAt
        self.location = location
    }

    enum CodingKeys: String, CodingKey {
        case ok
        case alreadyAttended = "already_attended"
        case eventId = "event_id"
        case title
        case startsAt = "starts_at"
        case location
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        alreadyAttended = try c.decodeOrDefault(Bool.self, forKey: .alreadyAttended, default: false)
        eventId = try c.decodeOrDefault(Int64.self, forKey: .eventId, default: 0)
        title = try c.decodeOrDefault(String.self, forKey: .title, default: "")
        startsAt = try c.decodeOrDefault(String.self, forKey: .startsAt, default: "")
        location = try c.decodeOrDefault(String.self, forKey: .location, default: "")
    }
}

public struct ScanMemberReq: Codable, Equatable, Sendable {
    public var passCode: String
    public init(passCode: String) { self.passCode = passCode }
    enum CodingKeys: String, CodingKey { case passCode = "pass_code" }
}

public struct ScanMemberResp: Codable, Equatable, Sendable {
    public var ok: Bool
    public var userId: Int64
    public var memberName: String
    public var alreadyAttended: Bool

    public init(ok: Bool = true, userId: Int64 = 0, memberName: String = "", alreadyAttended: Bool = false) {
        self.ok = ok
        self.userId = userId
        self.memberName = memberName
        self.alreadyAttended = alreadyAttended
    }

    enum CodingKeys: String, CodingKey {
        case ok
        case userId = "user_id"
        case memberName = "member_name"
        case alreadyAttended = "already_attended"
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        ok = try c.decodeOrDefault(Bool.self, forKey: .ok, default: true)
        userId = try c.decodeOrDefault(Int64.self, forKey: .userId, default: 0)
        memberName = try c.decodeOrDefault(String.self, forKey: .memberName, default: "")
        alreadyAttended = try c.decodeOrDefault(Bool.self, forKey: .alreadyAttended, default: false)
    }
}

public struct MarkAttendanceReq: Codable, Equatable, Sendable {
    public var userId: Int64
    public var attended: Bool?

    public init(userId: Int64, attended: Bool? = nil) {
        self.userId = userId
        self.attended = attended
    }

    enum CodingKeys: String, CodingKey {
        case userId = "user_id"
        case attended
    }
}

// MARK: - Member Reports / Badges

public struct MemberReportResp: Codable, Equatable, Sendable {
    public var badges: [Badge]

    public init(badges: [Badge] = []) {
        self.badges = badges
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        badges = try c.decodeOrDefault([Badge].self, forKey: .badges, default: [])
    }

    enum CodingKeys: String, CodingKey { case badges }
}
