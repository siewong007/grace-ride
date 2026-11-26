package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CheckinByCodeReq
import org.pjstatecm.graceride.data.api.CheckinInfoDto
import org.pjstatecm.graceride.data.api.CheckinReq
import org.pjstatecm.graceride.data.api.CheckinResp
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.MarkAttendanceReq
import org.pjstatecm.graceride.data.api.ScanMemberReq
import org.pjstatecm.graceride.data.api.ScanMemberResp
import org.pjstatecm.graceride.data.api.SignupReq
import org.pjstatecm.graceride.data.api.apiCall

class EventsRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()

    suspend fun upcoming(passive: Boolean = false): List<EventItem> =
        apiCall(bus) { api.events("upcoming", passivePoll = if (passive) "1" else null) }
    suspend fun past(): List<EventItem> = apiCall(bus) { api.events("past") }
    suspend fun managing(hideDrafts: Boolean = false): List<EventItem> =
        apiCall(bus) { api.events("managing", if (hideDrafts) true else null) }
    suspend fun detail(id: Long): EventItem = apiCall(bus) { api.event(id) }
    suspend fun signup(id: Long, needRide: Boolean, offerDrive: Boolean, asGroupId: Long?, offerSeats: Int? = null) =
        apiCall(bus) { api.signup(id, SignupReq(needRide, offerDrive, offerSeats?.toLong(), asGroupId)) }
    suspend fun withdraw(id: Long) = apiCall(bus) { api.withdraw(id) }
    suspend fun publish(id: Long) = apiCall(bus) { api.publishEvent(id) }
    suspend fun unpublish(id: Long) = apiCall(bus) { api.unpublishEvent(id) }
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
