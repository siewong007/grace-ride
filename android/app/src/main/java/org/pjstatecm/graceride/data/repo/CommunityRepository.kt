package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.Announcement
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CalendarFeedDto
import org.pjstatecm.graceride.data.api.CalendarFeedRotateDto
import org.pjstatecm.graceride.data.api.CreateAnnouncementReq
import org.pjstatecm.graceride.data.api.FamilyAddPersonRespDto
import org.pjstatecm.graceride.data.api.FamilyCreateRespDto
import org.pjstatecm.graceride.data.api.FamilyNameReq
import org.pjstatecm.graceride.data.api.FamilyPhoneReq
import org.pjstatecm.graceride.data.api.FamilyStateDto
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.GroupDto
import org.pjstatecm.graceride.data.api.IdRespDto
import org.pjstatecm.graceride.data.api.MobilePushTokenDel
import org.pjstatecm.graceride.data.api.MobilePushTokenReq
import org.pjstatecm.graceride.data.api.NotificationActionResp
import org.pjstatecm.graceride.data.api.NotificationItem
import org.pjstatecm.graceride.data.api.NotificationPageDto
import org.pjstatecm.graceride.data.api.NotificationPrefsDto
import org.pjstatecm.graceride.data.api.OkResponse
import org.pjstatecm.graceride.data.api.ProfileUpdateReq
import org.pjstatecm.graceride.data.api.PushConfigDto
import org.pjstatecm.graceride.data.api.PushSubscriptionsDto
import org.pjstatecm.graceride.data.api.PushTestRespDto
import org.pjstatecm.graceride.data.api.ReadAllReq
import org.pjstatecm.graceride.data.api.TelegramLinkTokenDto
import org.pjstatecm.graceride.data.api.TelegramTestRespDto
import org.pjstatecm.graceride.data.api.UpdateAnnouncementReq
import org.pjstatecm.graceride.data.api.UpdateNotificationPrefsReq
import org.pjstatecm.graceride.data.api.apiCall
import java.io.File

class CommunityRepository(private val apiProvider: ApiProvider, private val bus: AuthEventBus) {
    private val api: ApiService get() = apiProvider.get()
    suspend fun announcements(manage: Boolean? = null): List<Announcement> =
        apiCall(bus) { api.announcements(manage = manage) }

    suspend fun createAnnouncement(req: CreateAnnouncementReq): IdRespDto =
        apiCall(bus) { api.createAnnouncement(req) }

    suspend fun updateAnnouncement(id: Long, req: UpdateAnnouncementReq): OkResponse =
        apiCall(bus) { api.updateAnnouncement(id, req) }

    suspend fun deleteAnnouncement(id: Long): OkResponse =
        apiCall(bus) { api.deleteAnnouncement(id) }

    suspend fun groups(): List<GroupDto> = apiCall(bus) { api.groups() }

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

    // ---- Calendar Feed ----
    suspend fun calendarFeed(): CalendarFeedDto = apiCall(bus) { api.calendarFeed() }
    suspend fun rotateCalendarFeed(): CalendarFeedRotateDto = apiCall(bus) { api.rotateCalendarFeed() }
    suspend fun revokeCalendarFeed(): OkResponse = apiCall(bus) { api.revokeCalendarFeed() }

    // ---- Push Devices & Telegram ----
    suspend fun pushConfig(): PushConfigDto = apiCall(bus) { api.pushConfig() }
    suspend fun pushSubscriptions(): PushSubscriptionsDto = apiCall(bus) { api.pushSubscriptions() }
    suspend fun deletePushSubscription(id: Long): OkResponse = apiCall(bus) { api.deletePushSubscription(id) }
    suspend fun pushTest(): PushTestRespDto = apiCall(bus) { api.pushTest() }
    suspend fun telegramLinkToken(): TelegramLinkTokenDto = apiCall(bus) { api.telegramLinkToken() }
    suspend fun telegramTest(): TelegramTestRespDto = apiCall(bus) { api.telegramTest() }

    // ---- Family Circle ----
    suspend fun family(): FamilyStateDto = apiCall(bus) { api.family() }
    suspend fun createFamily(name: String): FamilyCreateRespDto =
        apiCall(bus) { api.createFamily(FamilyNameReq(name.trim())) }
    suspend fun renameFamily(name: String): OkResponse =
        apiCall(bus) { api.renameFamily(FamilyNameReq(name.trim())) }
    suspend fun deleteFamily(): OkResponse = apiCall(bus) { api.deleteFamily() }
    suspend fun leaveFamily(): OkResponse = apiCall(bus) { api.leaveFamily() }
    suspend fun addFamilyPerson(phone: String): FamilyAddPersonRespDto =
        apiCall(bus) { api.addFamilyPerson(FamilyPhoneReq(phone.trim())) }
    suspend fun removeFamilyMember(userId: Long): OkResponse =
        apiCall(bus) { api.removeFamilyMember(userId) }
    suspend fun cancelFamilyRequest(id: Long): OkResponse =
        apiCall(bus) { api.cancelFamilyRequest(id) }
    suspend fun acceptFamilyRequest(id: Long): OkResponse =
        apiCall(bus) { api.acceptFamilyRequest(id) }
    suspend fun declineFamilyRequest(id: Long): OkResponse =
        apiCall(bus) { api.declineFamilyRequest(id) }
    suspend fun revokeFamilyInvite(id: Long): OkResponse =
        apiCall(bus) { api.revokeFamilyInvite(id) }
}
