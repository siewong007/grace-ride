package org.pjstatecm.graceride.data.api

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

// ---------------- Help Center & Guides ----------------

@Serializable
data class GuideStepDto(
    val id: String? = null,
    @SerialName("title_en") val titleEn: String = "",
    @SerialName("title_zh") val titleZh: String = "",
    @SerialName("body_en") val bodyEn: String = "",
    @SerialName("body_zh") val bodyZh: String = "",
    val link: String = "",
    @SerialName("image_file_id") val imageFileId: Long? = null,
    @SerialName("note_kind") val noteKind: String? = null,
    @SerialName("note_en") val noteEn: String? = null,
    @SerialName("note_zh") val noteZh: String? = null,
)

@Serializable
data class GuideSummaryDto(
    val id: Long,
    val slug: String = "",
    @SerialName("title_en") val titleEn: String = "",
    @SerialName("title_zh") val titleZh: String = "",
    @SerialName("summary_en") val summaryEn: String = "",
    @SerialName("summary_zh") val summaryZh: String = "",
    val published: Boolean = true,
    val audience: String = "member",
    @SerialName("step_count") val stepCount: Int = 0,
    val steps: List<GuideStepDto> = emptyList(),
)

@Serializable
data class GuideDetailDto(
    val id: Long,
    val slug: String = "",
    @SerialName("title_en") val titleEn: String = "",
    @SerialName("title_zh") val titleZh: String = "",
    @SerialName("summary_en") val summaryEn: String = "",
    @SerialName("summary_zh") val summaryZh: String = "",
    val published: Boolean = true,
    val audience: String = "member",
    val steps: List<GuideStepDto> = emptyList(),
    @SerialName("helpful_count") val helpfulCount: Long? = null,
    @SerialName("not_helpful_count") val notHelpfulCount: Long? = null,
    @SerialName("my_feedback") val myFeedback: Boolean? = null,
)

@Serializable
data class GuideStepInReq(
    val id: String? = null,
    val title_en: String,
    val title_zh: String,
    val body_en: String = "",
    val body_zh: String = "",
    val link: String? = null,
    val image_file_id: Long? = null,
    val note_kind: String? = null,
    val note_en: String? = null,
    val note_zh: String? = null,
)

@Serializable
data class SaveGuideReq(
    val slug: String,
    val title_en: String,
    val title_zh: String,
    val summary_en: String = "",
    val summary_zh: String = "",
    val published: Boolean = true,
    val audience: String = "member",
    val steps: List<GuideStepInReq>,
)

@Serializable
data class GuideReorderReq(val ids: List<Long>)

@Serializable
data class GuideFeedbackReq(val helpful: Boolean)

@Serializable
data class GuideFeedbackResp(
    val ok: Boolean = true,
    val helpful: Boolean? = null,
)

@Serializable
data class TicketSummaryDto(
    val id: Long,
    val subject: String = "",
    @SerialName("user_id") val userId: Long = 0,
    @SerialName("user_name") val userName: String = "",
    val status: String = "open",
    @SerialName("ride_id") val rideId: Long? = null,
    @SerialName("created_at") val createdAt: String = "",
    @SerialName("updated_at") val updatedAt: String = "",
    @SerialName("last_body") val lastBody: String? = null,
    @SerialName("last_staff") val lastStaff: Boolean? = null,
    @SerialName("last_at") val lastAt: String? = null,
    @SerialName("assignee_id") val assigneeId: Long? = null,
    @SerialName("assignee_name") val assigneeName: String? = null,
)

@Serializable
data class HelpCountsDto(
    val open: Long = 0,
    val answered: Long = 0,
    val closed: Long = 0,
)

@Serializable
data class HelpMemberDto(
    val id: Long,
    val name: String = "",
)

@Serializable
data class TicketMemberMetaDto(
    val name: String = "",
    val phone: String = "",
    @SerialName("other_open") val otherOpen: List<Long> = emptyList(),
)

@Serializable
data class TicketReplyDto(
    val id: Long,
    @SerialName("user_id") val userId: Long = 0,
    val name: String = "",
    val body: String = "",
    @SerialName("created_at") val createdAt: String = "",
    val staff: Boolean = false,
    @SerialName("file_id") val fileId: Long? = null,
)

@Serializable
data class TicketDetailDto(
    val id: Long,
    val subject: String = "",
    val status: String = "open",
    @SerialName("ride_id") val rideId: Long? = null,
    @SerialName("user_id") val userId: Long = 0,
    val member: TicketMemberMetaDto? = null,
    @SerialName("assignee_id") val assigneeId: Long? = null,
    @SerialName("assignee_name") val assigneeName: String? = null,
    val replies: List<TicketReplyDto> = emptyList(),
)

@Serializable
data class CreateTicketReq(
    val subject: String,
    val body: String,
    val ride_id: Long? = null,
    val for_user_id: Long? = null,
    val file_id: Long? = null,
)

@Serializable
data class TicketReplyReq(
    val body: String = "",
    val file_id: Long? = null,
)

@Serializable
data class ClaimTicketReq(val claim: Boolean)

// ---------------- Personal & Staff Reports ----------------

@Serializable
data class MonthReportVisibleSectionsDto(
    val events: Boolean = true,
    val driving: Boolean = true,
    @SerialName("passenger_rides") val passengerRides: Boolean = true,
    @SerialName("costs_paid") val costsPaid: Boolean = true,
    val reimbursements: Boolean = true,
    val organised: Boolean = true,
    val badges: Boolean = true,
)

@Serializable
data class MonthReportReimbSummaryDto(
    @SerialName("claim_count") val claimCount: Long = 0,
    @SerialName("pending_count") val pendingCount: Long? = null,
    @SerialName("pending_cents") val pendingCents: Long? = null,
    @SerialName("approved_count") val approvedCount: Long = 0,
    @SerialName("approved_cents") val approvedCents: Long = 0,
    @SerialName("paid_count") val paidCount: Long = 0,
    @SerialName("paid_cents") val paidCents: Long = 0,
    @SerialName("returned_count") val returnedCount: Long? = null,
    @SerialName("rejected_count") val rejectedCount: Long? = null,
)

@Serializable
data class MonthReportEventKindDto(val kind: String = "", val count: Long = 0)

@Serializable
data class MonthReportEventRowDto(
    val id: Long,
    val title: String = "",
    val kind: String = "",
    @SerialName("starts_at") val startsAt: String = "",
)

@Serializable
data class MonthReportDto(
    val user: UserRefDto,
    val month: String = "",
    val range: String = "monthly",
    val type: String = "all",
    @SerialName("period_start") val periodStart: String = "",
    @SerialName("period_end") val periodEnd: String = "",
    @SerialName("period_end_inclusive") val periodEndInclusive: String? = null,
    @SerialName("rides_driven") val ridesDriven: Long? = null,
    @SerialName("rides_received") val ridesReceived: Long? = null,
    @SerialName("distance_km") val distanceKm: Long? = null,
    @SerialName("reimbursement_cents") val reimbursementCents: Long? = null,
    @SerialName("events_attended") val eventsAttended: Long? = null,
    @SerialName("by_kind") val byKind: List<MonthReportEventKindDto> = emptyList(),
    val events: List<MonthReportEventRowDto> = emptyList(),
    @SerialName("costs_paid_cents") val costsPaidCents: Long? = null,
    @SerialName("visible_sections") val visibleSections: MonthReportVisibleSectionsDto? = null,
    @SerialName("reimbursements_summary") val reimbursementsSummary: MonthReportReimbSummaryDto? = null,
    val badges: List<Badge> = emptyList(),
)

@Serializable
data class FinanceSectionsCapsDto(
    @SerialName("reimb_review") val reimbReview: Boolean = false,
    @SerialName("reimb_payouts") val reimbPayouts: Boolean = false,
    val bills: Boolean = false,
)

@Serializable
data class ReportCapabilitiesDto(
    val overview: Boolean = false,
    @SerialName("member_reports_global") val memberReportsGlobal: Boolean = false,
    @SerialName("member_badge_icon_edit") val memberBadgeIconEdit: Boolean = false,
    val demand: Boolean = false,
    @SerialName("demand_scope") val demandScope: String = "none",
    val events: Boolean = false,
    @SerialName("events_scope") val eventsScope: String = "none",
    val finance: Boolean = false,
    @SerialName("finance_sections") val financeSections: FinanceSectionsCapsDto = FinanceSectionsCapsDto(),
    val membership: Boolean = false,
)

@Serializable
data class OverviewTransportDto(
    val scope: String = "global",
    @SerialName("total_rides") val totalRides: Long = 0,
    @SerialName("completed_rides") val completedRides: Long = 0,
    @SerialName("pending_confirmation_rides") val pendingConfirmationRides: Long? = null,
    @SerialName("unfilled_rides") val unfilledRides: Long = 0,
    @SerialName("cancelled_rides") val cancelledRides: Long = 0,
    @SerialName("no_show_rides") val noShowRides: Long = 0,
    @SerialName("alt_transport_rides") val altTransportRides: Long = 0,
    @SerialName("fulfillment_rate_pct") val fulfillmentRatePct: Double = 0.0,
    @SerialName("total_distance_km") val totalDistanceKm: Long = 0,
    @SerialName("active_drivers") val activeDrivers: Long = 0,
    @SerialName("unique_passengers") val uniquePassengers: Long = 0,
)

@Serializable
data class OverviewEventsDto(
    val scope: String = "global",
    @SerialName("events_count") val eventsCount: Long = 0,
    @SerialName("total_going") val totalGoing: Long = 0,
    @SerialName("total_attended") val totalAttended: Long = 0,
    @SerialName("attendance_rate_pct") val attendanceRatePct: Double = 0.0,
)

@Serializable
data class OverviewReimbReviewDto(
    @SerialName("pending_count") val pendingCount: Long = 0,
    @SerialName("pending_cents") val pendingCents: Long = 0,
    @SerialName("returned_count") val returnedCount: Long = 0,
    @SerialName("approved_count") val approvedCount: Long = 0,
    @SerialName("approved_cents") val approvedCents: Long = 0,
)

@Serializable
data class OverviewPayoutsDto(
    @SerialName("approved_unpaid_count") val approvedUnpaidCount: Long = 0,
    @SerialName("approved_unpaid_cents") val approvedUnpaidCents: Long = 0,
    @SerialName("paid_count") val paidCount: Long = 0,
    @SerialName("paid_cents") val paidCents: Long = 0,
    @SerialName("open_batches") val openBatches: Long = 0,
    @SerialName("paid_batches") val paidBatches: Long = 0,
)

@Serializable
data class OverviewBillsDto(
    @SerialName("published_count") val publishedCount: Long = 0,
    @SerialName("closed_count") val closedCount: Long = 0,
    @SerialName("gross_cents") val grossCents: Long = 0,
    @SerialName("subsidy_cents") val subsidyCents: Long = 0,
    @SerialName("collected_cents") val collectedCents: Long = 0,
    @SerialName("outstanding_cents") val outstandingCents: Long = 0,
    @SerialName("collection_rate_pct") val collectionRatePct: Double = 0.0,
)

@Serializable
data class OverviewFinanceDto(
    @SerialName("reimb_payouts") val reimbPayouts: OverviewPayoutsDto? = null,
    val bills: OverviewBillsDto? = null,
)

@Serializable
data class OverviewMembershipDto(
    @SerialName("active_members") val activeMembers: Long = 0,
    @SerialName("pending_members") val pendingMembers: Long = 0,
    @SerialName("new_members_in_period") val newMembersInPeriod: Long = 0,
    @SerialName("active_drivers_total") val activeDriversTotal: Long = 0,
)

@Serializable
data class OverviewReportDto(
    val period: String = "",
    val range: String = "monthly",
    val from: String = "",
    val to: String = "",
    val transport: OverviewTransportDto? = null,
    val events: OverviewEventsDto? = null,
    @SerialName("reimb_review") val reimbReview: OverviewReimbReviewDto? = null,
    val finance: OverviewFinanceDto? = null,
    val membership: OverviewMembershipDto? = null,
)

@Serializable
data class EventsReportTotalsDto(
    @SerialName("events_count") val eventsCount: Long = 0,
    @SerialName("cancelled_events") val cancelledEvents: Long = 0,
    @SerialName("total_going") val totalGoing: Long = 0,
    @SerialName("total_waitlist") val totalWaitlist: Long = 0,
    @SerialName("total_attended") val totalAttended: Long = 0,
    @SerialName("attendance_rate_pct") val attendanceRatePct: Double = 0.0,
    @SerialName("need_ride_count") val needRideCount: Long = 0,
    @SerialName("offer_drive_count") val offerDriveCount: Long = 0,
    @SerialName("self_transport_count") val selfTransportCount: Long = 0,
)

@Serializable
data class EventsReportRowDto(
    val id: Long,
    val title: String = "",
    val kind: String = "",
    @SerialName("starts_at") val startsAt: String = "",
    val status: String = "",
    val going: Long = 0,
    val attended: Long = 0,
    @SerialName("attendance_rate_pct") val attendanceRatePct: Double? = null,
)

@Serializable
data class EventsReportDto(
    val scope: String = "global",
    val period: String = "",
    val range: String = "monthly",
    val from: String = "",
    val to: String = "",
    val totals: EventsReportTotalsDto = EventsReportTotalsDto(),
    val events: List<EventsReportRowDto> = emptyList(),
)

@Serializable
data class DemandTotalsDto(
    @SerialName("total_requests") val totalRequests: Long = 0,
    val fulfilled: Long = 0,
    val unfilled: Long = 0,
    val completed: Long = 0,
    @SerialName("fulfillment_rate_pct") val fulfillmentRatePct: Double = 0.0,
    @SerialName("active_drivers") val activeDrivers: Long = 0,
    @SerialName("unique_passengers") val uniquePassengers: Long = 0,
)

@Serializable
data class DemandAreaRowDto(
    @SerialName("area_id") val areaId: Long,
    @SerialName("area_name_zh") val areaNameZh: String = "",
    @SerialName("area_name_en") val areaNameEn: String = "",
    val total: Long = 0,
    val fulfilled: Long = 0,
    val unfilled: Long = 0,
    @SerialName("fulfillment_rate_pct") val fulfillmentRatePct: Double = 0.0,
)

@Serializable
data class DemandReportDto(
    val scope: String = "global",
    val period: String = "",
    val range: String = "monthly",
    val from: String = "",
    val to: String = "",
    val totals: DemandTotalsDto = DemandTotalsDto(),
    @SerialName("by_area") val byArea: List<DemandAreaRowDto> = emptyList(),
)

@Serializable
data class FinanceBillsTotalsDto(
    @SerialName("total_settlements") val totalSettlements: Long = 0,
    @SerialName("gross_cents") val grossCents: Long = 0,
    @SerialName("subsidy_cents") val subsidyCents: Long = 0,
    @SerialName("net_billed_cents") val netBilledCents: Long = 0,
    @SerialName("collected_cents") val collectedCents: Long = 0,
    @SerialName("unpaid_cents") val unpaidCents: Long = 0,
    @SerialName("collection_rate_pct") val collectionRatePct: Double = 0.0,
)

@Serializable
data class FinanceBillsSectionDto(
    val totals: FinanceBillsTotalsDto = FinanceBillsTotalsDto(),
)

@Serializable
data class FinanceReportDto(
    val period: String = "",
    val range: String = "monthly",
    val from: String = "",
    val to: String = "",
    @SerialName("reimbursement_review") val reimbursementReview: OverviewReimbReviewDto? = null,
    @SerialName("reimbursement_payouts") val reimbursementPayouts: OverviewPayoutsDto? = null,
    val bills: FinanceBillsSectionDto? = null,
)

@Serializable
data class UpdateBadgeIconReq(val icon: String)
