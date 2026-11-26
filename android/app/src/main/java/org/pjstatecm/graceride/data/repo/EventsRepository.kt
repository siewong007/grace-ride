package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CheckinByCodeReq
import org.pjstatecm.graceride.data.api.CheckinInfoDto
import org.pjstatecm.graceride.data.api.CheckinReq
import org.pjstatecm.graceride.data.api.CheckinResp
import org.pjstatecm.graceride.data.api.CreateVisitorPassReq
import org.pjstatecm.graceride.data.api.CreateVisitorPassRespDto
import org.pjstatecm.graceride.data.api.EventCheckinResetRespDto
import org.pjstatecm.graceride.data.api.EventCreateRespDto
import org.pjstatecm.graceride.data.api.EventDuplicateReq
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.EventSignupItemDto
import org.pjstatecm.graceride.data.api.EventSignupRespDto
import org.pjstatecm.graceride.data.api.EventTransportReq
import org.pjstatecm.graceride.data.api.EventUpsertReq
import org.pjstatecm.graceride.data.api.MarkAttendanceReq
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.data.api.PostEventReimbBatchRespDto
import org.pjstatecm.graceride.data.api.PostEventReimbursementsReq
import org.pjstatecm.graceride.data.api.ScanMemberReq
import org.pjstatecm.graceride.data.api.ScanMemberResp
import org.pjstatecm.graceride.data.api.SignupReq
import org.pjstatecm.graceride.data.api.VisitorPassDto
import org.pjstatecm.graceride.data.api.VisitorUrlRespDto
import org.pjstatecm.graceride.data.api.apiCall

class EventsRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()

    suspend fun upcoming(passive: Boolean = false): List<EventItem> =
        apiCall(bus) { api.events("upcoming", passivePoll = if (passive) "1" else null) }
    suspend fun past(): List<EventItem> = apiCall(bus) { api.events("past") }
    suspend fun managing(hideDrafts: Boolean = false): List<EventItem> =
        apiCall(bus) { api.events("managing", if (hideDrafts) true else null) }
    suspend fun detail(id: Long): EventItem = apiCall(bus) { api.event(id) }
    suspend fun create(req: EventUpsertReq): EventCreateRespDto =
        apiCall(bus) { api.createEvent(req) }
    suspend fun update(id: Long, req: EventUpsertReq) =
        apiCall(bus) { api.updateEvent(id, req) }
    suspend fun duplicate(id: Long, req: EventDuplicateReq): EventCreateRespDto =
        apiCall(bus) { api.duplicateEvent(id, req) }
    suspend fun signup(
        id: Long,
        needRide: Boolean,
        offerDrive: Boolean,
        asGroupId: Long?,
        offerSeats: Int? = null,
        selfTransport: Boolean? = null,
    ): EventSignupRespDto = apiCall(bus) {
        api.signup(id, SignupReq(needRide, offerDrive, offerSeats?.toLong(), asGroupId, selfTransport))
    }
    suspend fun setSelfTransport(id: Long, enabled: Boolean) =
        apiCall(bus) { api.setEventTransport(id, EventTransportReq(enabled)) }
    suspend fun withdraw(id: Long) = apiCall(bus) { api.withdraw(id) }
    suspend fun publish(id: Long) = apiCall(bus) { api.publishEvent(id) }
    suspend fun unpublish(id: Long) = apiCall(bus) { api.unpublishEvent(id) }
    suspend fun cancel(id: Long) = apiCall(bus) { api.cancelEvent(id) }
    suspend fun finish(id: Long) = apiCall(bus) { api.finishEvent(id) }
    suspend fun signups(id: Long): List<EventSignupItemDto> =
        apiCall(bus) { api.eventSignups(id) }
    suspend fun resetCheckinCode(id: Long): EventCheckinResetRespDto =
        apiCall(bus) { api.resetEventCheckinCode(id) }
    suspend fun visitors(id: Long): List<VisitorPassDto> =
        apiCall(bus) { api.eventVisitors(id) }
    suspend fun createVisitor(id: Long, req: CreateVisitorPassReq): CreateVisitorPassRespDto =
        apiCall(bus) { api.createEventVisitor(id, req) }
    suspend fun reissueVisitor(passId: Long): VisitorUrlRespDto =
        apiCall(bus) { api.reissueVisitorPass(passId) }
    suspend fun revokeVisitor(passId: Long) =
        apiCall(bus) { api.revokeVisitorPass(passId) }
    suspend fun postEventDrivers(id: Long): PostEventDriversDto =
        apiCall(bus) { api.postEventDrivers(id) }
    suspend fun createPostEventReimbursements(
        id: Long,
        req: PostEventReimbursementsReq,
    ): PostEventReimbBatchRespDto = apiCall(bus) { api.createPostEventReimbursements(id, req) }
    suspend fun checkinInfo(id: Long): CheckinInfoDto = apiCall(bus) { api.eventCheckinInfo(id) }
    suspend fun selfCheckin(id: Long, code: String? = null): CheckinResp =
        apiCall(bus) { api.selfCheckin(id, CheckinReq(code)) }
    suspend fun checkinByCode(code: String): CheckinResp =
        apiCall(bus) { api.checkinByCode(CheckinByCodeReq(code)) }
    suspend fun scanMemberPass(id: Long, passCode: String): ScanMemberResp =
        apiCall(bus) { api.scanMemberPass(id, ScanMemberReq(passCode)) }
    suspend fun markAttendance(id: Long, userId: Long, attended: Boolean?) =
        apiCall(bus) { api.markAttendance(id, MarkAttendanceReq(userId, attended)) }
}
