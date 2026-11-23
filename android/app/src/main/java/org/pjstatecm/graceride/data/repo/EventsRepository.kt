package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.SignupReq
import org.pjstatecm.graceride.data.api.apiCall

class EventsRepository(private val api: ApiService, private val bus: AuthEventBus) {
    suspend fun upcoming(): List<EventItem> = apiCall(bus) { api.events("upcoming") }
    suspend fun past(): List<EventItem> = apiCall(bus) { api.events("past") }
    suspend fun managing(hideDrafts: Boolean = false): List<EventItem> =
        apiCall(bus) { api.events("managing", if (hideDrafts) true else null) }
    suspend fun detail(id: Long): EventItem = apiCall(bus) { api.event(id) }
    suspend fun signup(id: Long, needRide: Boolean, offerDrive: Boolean, asGroupId: Long?) =
        apiCall(bus) { api.signup(id, SignupReq(needRide, offerDrive, null, asGroupId)) }
    suspend fun withdraw(id: Long) = apiCall(bus) { api.withdraw(id) }
    suspend fun publish(id: Long) = apiCall(bus) { api.publishEvent(id) }
    suspend fun unpublish(id: Long) = apiCall(bus) { api.unpublishEvent(id) }
}
