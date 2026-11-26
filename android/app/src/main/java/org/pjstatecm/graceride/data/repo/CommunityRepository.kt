package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.MobilePushTokenDel
import org.pjstatecm.graceride.data.api.MobilePushTokenReq
import org.pjstatecm.graceride.data.api.NotificationActionResp
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.api.NotificationPageDto
import org.pjstatecm.graceride.data.api.NotificationPrefsDto
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.ReadAllReq
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.api.apiCall
import java.io.File

class CommunityRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()
    suspend fun announcements(): List<Announcement> = apiCall(bus) { api.announcements() }

    suspend fun notificationsPage(
        before: Long? = null,
        limit: Int = 30,
        unreadOnly: Boolean? = null,
        category: String? = null,
        passive: Boolean = false,
    ): NotificationPageDto = apiCall(bus) {
        api.notificationsPage(
            before = before,
            limit = limit,
            unreadOnly = unreadOnly,
            category = category?.takeIf { it.isNotBlank() },
            passivePoll = if (passive) "1" else null,
        )
    }

    suspend fun notifications(): List<NotificationItem> = notificationsPage(limit = 30).items

    suspend fun readAll(category: String? = null): NotificationActionResp =
        apiCall(bus) { api.readAllNotifications(ReadAllReq(category = category?.takeIf { it.isNotBlank() })) }

    suspend fun readOne(id: Long): NotificationActionResp = apiCall(bus) { api.readNotification(id) }

    suspend fun unreadOne(id: Long): NotificationActionResp = apiCall(bus) { api.unreadNotification(id) }

    suspend fun deleteOne(id: Long): NotificationActionResp = apiCall(bus) { api.deleteNotification(id) }

    suspend fun notificationPrefs(passive: Boolean = false): NotificationPrefsDto =
        apiCall(bus) { api.notificationPrefs(passivePoll = if (passive) "1" else null) }

    suspend fun updateNotificationPrefs(req: UpdateNotificationPrefsReq) =
        apiCall(bus) { api.updateNotificationPrefs(req) }

    suspend fun registerMobileToken(req: MobilePushTokenReq) =
        apiCall(bus) { api.registerMobileToken(req) }

    suspend fun unregisterMobileToken(token: String) =
        apiCall(bus) { api.unregisterMobileToken(MobilePushTokenDel(token)) }

    suspend fun updateProfile(req: ProfileUpdateReq) = apiCall(bus) { api.updateProfile(req) }

    suspend fun upload(file: File, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", file.name, file.asRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }

    suspend fun upload(bytes: ByteArray, filename: String, mime: String): FileRef {
        val part = MultipartBody.Part.createFormData(
            "file", filename, bytes.toRequestBody(mime.toMediaType())
        )
        return apiCall(bus) { api.upload(part) }
    }

    suspend fun passkeys() = apiCall(bus) { api.passkeys() }
    suspend fun deletePasskey(id: Long) = apiCall(bus) { api.deletePasskey(id) }
    suspend fun biometrics() = apiCall(bus) { api.biometrics() }
    suspend fun deleteBiometric(id: Long) = apiCall(bus) { api.deleteBiometric(id) }
}
