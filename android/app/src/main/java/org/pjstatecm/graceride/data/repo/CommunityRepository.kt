package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.apiCall

class CommunityRepository(private val api: ApiService, private val bus: AuthEventBus) {
    suspend fun announcements(): List<Announcement> = apiCall(bus) { api.announcements() }
    suspend fun notifications(): List<NotificationItem> = apiCall(bus) { api.notifications() }
    suspend fun readAll() = apiCall(bus) { api.readAllNotifications() }
    suspend fun readOne(id: Long) = apiCall(bus) { api.readNotification(id) }
    suspend fun updateProfile(req: ProfileUpdateReq) = apiCall(bus) { api.updateProfile(req) }
    suspend fun passkeys() = apiCall(bus) { api.passkeys() }
    suspend fun deletePasskey(id: Long) = apiCall(bus) { api.deletePasskey(id) }
}
