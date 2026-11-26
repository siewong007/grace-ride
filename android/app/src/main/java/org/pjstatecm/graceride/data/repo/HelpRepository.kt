package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.ClaimTicketReq
import org.pjstatecm.graceride.data.api.CreateTicketReq
import org.pjstatecm.graceride.data.api.DemandReportDto
import org.pjstatecm.graceride.data.api.EventsReportDto
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.FinanceReportDto
import org.pjstatecm.graceride.data.api.GuideDetailDto
import org.pjstatecm.graceride.data.api.GuideFeedbackReq
import org.pjstatecm.graceride.data.api.GuideReorderReq
import org.pjstatecm.graceride.data.api.GuideSummaryDto
import org.pjstatecm.graceride.data.api.HelpCountsDto
import org.pjstatecm.graceride.data.api.HelpMemberDto
import org.pjstatecm.graceride.data.api.IdRespDto
import org.pjstatecm.graceride.data.api.MonthReportDto
import org.pjstatecm.graceride.data.api.OverviewReportDto
import org.pjstatecm.graceride.data.api.ReportCapabilitiesDto
import org.pjstatecm.graceride.data.api.SaveGuideReq
import org.pjstatecm.graceride.data.api.TicketDetailDto
import org.pjstatecm.graceride.data.api.TicketReplyReq
import org.pjstatecm.graceride.data.api.TicketSummaryDto
import org.pjstatecm.graceride.data.api.UpdateBadgeIconReq
import org.pjstatecm.graceride.data.api.apiCall

class HelpRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()

    suspend fun uploadFile(bytes: ByteArray, filename: String, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData("file", filename, bytes.toRequestBody(mime.toMediaType()))
        return apiCall(bus) { api.upload(part) }
    }

    // ---- Tickets ----
    suspend fun tickets(
        all: Boolean = false,
        status: String? = null,
        q: String? = null,
        assigned: String? = null,
    ): List<TicketSummaryDto> = apiCall(bus) {
        api.helpTickets(
            all = if (all) true else null,
            status = status?.takeIf { it.isNotBlank() && it != "all" },
            q = q?.trim()?.ifBlank { null },
            assigned = assigned?.takeIf { it.isNotBlank() },
        )
    }

    suspend fun counts(): HelpCountsDto = apiCall(bus) { api.helpCounts() }

    suspend fun members(q: String): List<HelpMemberDto> = apiCall(bus) { api.helpMembers(q) }

    suspend fun ticketDetail(id: Long): TicketDetailDto = apiCall(bus) { api.helpTicketDetail(id) }

    suspend fun createTicket(req: CreateTicketReq): IdRespDto = apiCall(bus) { api.createHelpTicket(req) }

    suspend fun reply(id: Long, body: String, fileId: Long? = null) =
        apiCall(bus) { api.replyHelpTicket(id, TicketReplyReq(body = body.trim(), file_id = fileId)) }

    suspend fun close(id: Long) = apiCall(bus) { api.closeHelpTicket(id) }

    suspend fun reopen(id: Long) = apiCall(bus) { api.reopenHelpTicket(id) }

    suspend fun claim(id: Long, claim: Boolean) =
        apiCall(bus) { api.claimHelpTicket(id, ClaimTicketReq(claim = claim)) }

    // ---- Guides ----
    suspend fun guides(includeDrafts: Boolean = false, full: Boolean = false): List<GuideSummaryDto> =
        apiCall(bus) {
            api.helpGuides(
                includeDrafts = if (includeDrafts) true else null,
                full = if (full) true else null,
            )
        }

    suspend fun guideDetail(key: String): GuideDetailDto = apiCall(bus) { api.helpGuideDetail(key) }

    suspend fun createGuide(req: SaveGuideReq): IdRespDto = apiCall(bus) { api.createHelpGuide(req) }

    suspend fun updateGuide(key: String, req: SaveGuideReq) = apiCall(bus) { api.updateHelpGuide(key, req) }

    suspend fun deleteGuide(key: String) = apiCall(bus) { api.deleteHelpGuide(key) }

    suspend fun reorderGuides(ids: List<Long>) = apiCall(bus) { api.reorderHelpGuides(GuideReorderReq(ids)) }

    suspend fun guideFeedback(id: Long, helpful: Boolean) =
        apiCall(bus) { api.helpGuideFeedback(id, GuideFeedbackReq(helpful)) }

    // ---- Reports & Badges ----
    suspend fun monthlyReport(
        month: String? = null,
        range: String? = null,
        type: String? = null,
        userId: Long? = null,
        lang: String? = null,
    ): MonthReportDto = apiCall(bus) {
        api.monthlyReport(month = month, range = range, type = type, userId = userId, lang = lang)
    }

    suspend fun reportCapabilities(): ReportCapabilitiesDto =
        apiCall(bus) { api.reportCapabilities() }

    suspend fun overviewReport(month: String? = null, range: String? = null): OverviewReportDto =
        apiCall(bus) { api.overviewReport(month = month, range = range) }

    suspend fun eventsReport(month: String? = null, range: String? = null): EventsReportDto =
        apiCall(bus) { api.eventsReport(month = month, range = range) }

    suspend fun demandReport(month: String? = null, range: String? = null): DemandReportDto =
        apiCall(bus) { api.demandReport(month = month, range = range) }

    suspend fun financeReport(month: String? = null, range: String? = null): FinanceReportDto =
        apiCall(bus) { api.financeReport(month = month, range = range) }

    suspend fun updateUserBadgeIcon(userId: Long, badgeKey: String, icon: String) =
        apiCall(bus) { api.updateUserBadgeIcon(userId, badgeKey, UpdateBadgeIconReq(icon)) }
}
