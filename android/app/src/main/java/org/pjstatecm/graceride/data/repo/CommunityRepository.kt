package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.apiCall
import java.io.File

class CommunityRepository(private val api: ApiService, private val bus: AuthEventBus) {
    suspend fun announcements(): List<Announcement> = apiCall(bus) { api.announcements() }
    suspend fun notifications(): List<NotificationItem> = apiCall(bus) { api.notifications() }
    suspend fun readAll() = apiCall(bus) { api.readAllNotifications() }
    suspend fun readOne(id: Long) = apiCall(bus) { api.readNotification(id) }
    suspend fun updateProfile(req: ProfileUpdateReq) = apiCall(bus) { api.updateProfile(req) }
    suspend fun upload(file: File, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", file.name, file.asRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }
    suspend fun passkeys() = apiCall(bus) { api.passkeys() }
    suspend fun deletePasskey(id: Long) = apiCall(bus) { api.deletePasskey(id) }
}
