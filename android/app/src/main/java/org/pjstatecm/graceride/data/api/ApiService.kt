package org.pjstatecm.graceride.data.api

import okhttp3.MultipartBody
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.Multipart
import retrofit2.http.PATCH
import retrofit2.http.POST
import retrofit2.http.PUT
import retrofit2.http.Part
import retrofit2.http.Path
import retrofit2.http.Query

/**
 * Typed client for the existing Grace Ride REST API. The backend is reused as-is;
 * no Origin header is sent (the server's origin guard admits non-browser clients).
 */
interface ApiService {

    // ---- auth & session ----
    @GET("api/auth/me") suspend fun me(): Me
    @GET("api/auth/dev-users") suspend fun devUsers(): List<DevUser>
    @POST("api/auth/dev-login") suspend fun devLogin(@Body body: Map<String, String>): OkResponse
    @POST("api/auth/logout") suspend fun logout(@Body body: LogoutReq = LogoutReq()): OkResponse
    @POST("api/auth/logout-all") suspend fun logoutAll(): OkResponse
    @POST("api/auth/lock") suspend fun lock(): LockRespDto
    @POST("api/auth/unlock") suspend fun unlock(@Body body: UnlockReq): OkResponse
    @POST("api/auth/terms") suspend fun acceptTerms(@Body body: TermsReq): OkResponse
    @POST("api/auth/recovery-request") suspend fun recoveryRequest(@Body body: RecoveryRequestIn): OkResponse
    @POST("api/auth/totp/setup") suspend fun totpSetup(): TotpSetupDto
    @POST("api/auth/totp/enable") suspend fun totpEnable(@Body body: TotpCodeReq): TotpEnableRespDto
    @POST("api/auth/totp/verify") suspend fun totpVerify(@Body body: TotpCodeReq): OkResponse
    @POST("api/auth/totp/backup-codes/regenerate") suspend fun regenerateBackupCodes(): BackupCodesRespDto
    @GET("api/invites/{token}") suspend fun invitePreview(@Path("token") token: String): InvitePreviewDto
    @POST("api/invites/{token}/accept") suspend fun inviteAccept(@Path("token") token: String, @Body body: InviteAcceptReq): OkResponse
    @GET("api/visit/{token}") suspend fun visitPreview(@Path("token") token: String): VisitPreviewDto
    @POST("api/visit/{token}/claim") suspend fun visitClaim(@Path("token") token: String, @Body body: VisitClaimReq = VisitClaimReq()): OkResponse
    @GET("api/visit/me") suspend fun visitMe(): VisitRideDto
    @GET("api/visit/me/location") suspend fun visitLocation(): RideLocationDto
    @POST("api/visit/me/readiness") suspend fun visitReadiness(@Body body: ReadinessReq): OkResponse
    @GET("api/care/{token}") suspend fun careStatus(@Path("token") token: String): CareStatusDto
    @POST("api/care/{token}/readiness") suspend fun careReadiness(@Path("token") token: String, @Body body: ReadinessReq): OkResponse
    @GET("third-party-licenses.json") suspend fun thirdPartyLicenses(): List<LicensePackageDto>
    @GET("api/auth/turnstile") suspend fun turnstileConfig(): TurnstileConfigDto
    @POST("api/auth/passkey/login/start") suspend fun passkeyLoginStart(@Body body: PasskeyStartReq): PasskeyStart
    @POST("api/auth/passkey/login/finish") suspend fun passkeyLoginFinish(@Body body: PasskeyFinishReq): OkResponse
    @POST("api/auth/passkey/reauth/start") suspend fun passkeyReauthStart(): PasskeyStart
    @POST("api/auth/passkey/reauth/finish") suspend fun passkeyReauthFinish(@Body body: PasskeyFinishReq): OkResponse
    @POST("api/auth/passkey/register/start") suspend fun passkeyRegisterStart(): PasskeyStart
    @POST("api/auth/passkey/register/finish") suspend fun passkeyRegisterFinish(@Body body: PasskeyFinishReq): OkResponse
    @GET("api/auth/passkeys") suspend fun passkeys(): List<PasskeyInfo>
    @DELETE("api/auth/passkeys/{id}") suspend fun deletePasskey(@Path("id") id: Long): OkResponse
    @POST("api/auth/biometric/register/start") suspend fun biometricRegisterStart(): BiometricChallengeRes
    @POST("api/auth/biometric/register/finish") suspend fun biometricRegisterFinish(@Body body: BiometricRegisterFinishReq): BiometricRegisterFinishRes
    @POST("api/auth/biometric/login/start") suspend fun biometricLoginStart(@Body body: BiometricLoginStartReq): BiometricChallengeRes
    @POST("api/auth/biometric/login/finish") suspend fun biometricLoginFinish(@Body body: BiometricFinishReq): OkResponse
    @POST("api/auth/biometric/reauth/start") suspend fun biometricReauthStart(@Body body: BiometricReauthStartReq): BiometricChallengeRes
    @POST("api/auth/biometric/reauth/finish") suspend fun biometricReauthFinish(@Body body: BiometricFinishReq): OkResponse
    @GET("api/auth/biometrics") suspend fun biometrics(): List<BiometricInfo>
    @DELETE("api/auth/biometrics/{id}") suspend fun deleteBiometric(@Path("id") id: Long): OkResponse

    // ---- profile & community ----
    @PUT("api/me") suspend fun updateProfile(@Body body: ProfileUpdateReq): OkResponse
    @GET("api/announcements") suspend fun announcements(@Query("manage") manage: Boolean? = null): List<Announcement>
    @POST("api/announcements") suspend fun createAnnouncement(@Body body: CreateAnnouncementReq): IdRespDto
    @PUT("api/announcements/{id}") suspend fun updateAnnouncement(
        @Path("id") id: Long,
        @Body body: UpdateAnnouncementReq,
    ): OkResponse
    @DELETE("api/announcements/{id}") suspend fun deleteAnnouncement(@Path("id") id: Long): OkResponse
    @GET("api/groups") suspend fun groups(): List<GroupDto>
    @GET("api/notifications") suspend fun notificationsPage(
        @Query("before") before: Long? = null,
        @Query("limit") limit: Int = 30,
        @Query("unread_only") unreadOnly: Boolean? = null,
        @Query("category") category: String? = null,
        @retrofit2.http.Header("X-Passive-Poll") passivePoll: String? = null,
    ): NotificationPageDto
    @POST("api/notifications/read-all") suspend fun readAllNotifications(@Body body: ReadAllReq = ReadAllReq()): NotificationActionResp
    @POST("api/notifications/{id}/read") suspend fun readNotification(@Path("id") id: Long): NotificationActionResp
    @POST("api/notifications/{id}/unread") suspend fun unreadNotification(@Path("id") id: Long): NotificationActionResp
    @DELETE("api/notifications/{id}") suspend fun deleteNotification(@Path("id") id: Long): NotificationActionResp
    @GET("api/notifications/preferences") suspend fun notificationPrefs(
        @retrofit2.http.Header("X-Passive-Poll") passivePoll: String? = null,
    ): NotificationPrefsDto
    @PUT("api/notifications/preferences") suspend fun updateNotificationPrefs(@Body body: UpdateNotificationPrefsReq): OkResponse
    @POST("api/push/mobile-token") suspend fun registerMobileToken(@Body body: MobilePushTokenReq): OkResponse
    @retrofit2.http.HTTP(method = "DELETE", path = "api/push/mobile-token", hasBody = true)
    suspend fun unregisterMobileToken(@Body body: MobilePushTokenDel): OkResponse
    @GET("api/push/config") suspend fun pushConfig(): PushConfigDto
    @GET("api/push/subscriptions") suspend fun pushSubscriptions(): PushSubscriptionsDto
    @DELETE("api/push/subscriptions/{id}") suspend fun deletePushSubscription(@Path("id") id: Long): OkResponse
    @POST("api/push/test") suspend fun pushTest(): PushTestRespDto
    @POST("api/push/telegram/link-token") suspend fun telegramLinkToken(): TelegramLinkTokenDto
    @POST("api/push/telegram/test") suspend fun telegramTest(): TelegramTestRespDto
    @GET("api/me/calendar-feed") suspend fun calendarFeed(): CalendarFeedDto
    @POST("api/me/calendar-feed/rotate") suspend fun rotateCalendarFeed(): CalendarFeedRotateDto
    @DELETE("api/me/calendar-feed") suspend fun revokeCalendarFeed(): OkResponse
    @GET("api/me/family") suspend fun family(): FamilyStateDto
    @POST("api/me/family") suspend fun createFamily(@Body body: FamilyNameReq): FamilyCreateRespDto
    @PATCH("api/me/family") suspend fun renameFamily(@Body body: FamilyNameReq): OkResponse
    @DELETE("api/me/family") suspend fun deleteFamily(): OkResponse
    @POST("api/me/family/leave") suspend fun leaveFamily(): OkResponse
    @POST("api/me/family/people") suspend fun addFamilyPerson(@Body body: FamilyPhoneReq): FamilyAddPersonRespDto
    @DELETE("api/me/family/members/{user_id}") suspend fun removeFamilyMember(@Path("user_id") userId: Long): OkResponse
    @DELETE("api/me/family/requests/{id}") suspend fun cancelFamilyRequest(@Path("id") id: Long): OkResponse
    @POST("api/me/family/requests/{id}/accept") suspend fun acceptFamilyRequest(@Path("id") id: Long): OkResponse
    @POST("api/me/family/requests/{id}/decline") suspend fun declineFamilyRequest(@Path("id") id: Long): OkResponse
    @DELETE("api/me/family/invites/{id}") suspend fun revokeFamilyInvite(@Path("id") id: Long): OkResponse

    // ---- rides ----
    @GET("api/areas") suspend fun areas(): List<Area>
    @GET("api/places") suspend fun places(@Query("q") q: String): List<Place>
    @GET("api/saved-places") suspend fun savedPlaces(): List<SavedPlace>
    @POST("api/saved-places") suspend fun createSavedPlace(@Body body: SavedPlaceReq): CreateSavedPlaceRes
    @PUT("api/saved-places/{id}") suspend fun updateSavedPlace(@Path("id") id: Long, @Body body: SavedPlaceReq): OkResponse
    @DELETE("api/saved-places/{id}") suspend fun deleteSavedPlace(@Path("id") id: Long): OkResponse
    @GET("api/proxy-riders") suspend fun proxyRiders(): List<ProxyRider>
    @POST("api/proxy-riders") suspend fun createProxyRider(@Body body: ProxyRiderReq): CreateProxyRiderRes
    @PUT("api/proxy-riders/{id}") suspend fun updateProxyRider(@Path("id") id: Long, @Body body: ProxyRiderReq): OkResponse
    @DELETE("api/proxy-riders/{id}") suspend fun deleteProxyRider(@Path("id") id: Long): OkResponse
    @GET("api/rides") suspend fun myRides(
        @Query("scope") scope: String? = null,
        @retrofit2.http.Header("X-Passive-Poll") passivePoll: String? = null,
    ): List<Ride>
    @GET("api/rides/{id}") suspend fun ride(@Path("id") id: Long): Ride
    @POST("api/rides") suspend fun createRide(@Body body: CreateRideReq): CreateRideRes
    @POST("api/rides/{id}/cancel") suspend fun cancelRide(@Path("id") id: Long, @Body body: CancelReq): OkResponse
    @POST("api/rides/{id}/respond") suspend fun respondRide(@Path("id") id: Long, @Body body: RespondReq): OkResponse
    @POST("api/rides/{id}/readiness") suspend fun readiness(@Path("id") id: Long, @Body body: ReadinessReq): OkResponse
    @POST("api/rides/{id}/confirm-assignment") suspend fun confirmAssignment(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/start") suspend fun startRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/arrived-gate") suspend fun arrivedGate(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/pickup") suspend fun pickupRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/no-show") suspend fun noShowRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/complete") suspend fun completeRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/supper-choice") suspend fun supperChoice(@Path("id") id: Long, @Body body: SupperChoiceReq): OkResponse
    @POST("api/rides/{id}/driver-delay") suspend fun driverDelay(@Path("id") id: Long, @Body body: DriverDelayReq): OkResponse
    @POST("api/rides/{id}/withdraw") suspend fun withdrawRide(@Path("id") id: Long, @Body body: WithdrawReq): OkResponse
    @GET("api/driver/schedule") suspend fun driverSchedule(@Query("from") from: String, @Query("to") to: String): List<Ride>
    @GET("api/driver/requests") suspend fun driverRequests(
        @Query("mine_areas") mineAreas: Boolean? = null,
        @Query("event_id") eventId: Long? = null,
        @Query("area_id") areaId: Long? = null,
        @Query("date") date: String? = null,
    ): List<Ride>
    @POST("api/rides/{id}/claim") suspend fun claimRide(@Path("id") id: Long): OkResponse
    @GET("api/driver/route") suspend fun driverRoute(): DriverRouteResp
    @POST("api/driver/location") suspend fun postDriverLocation(@Body body: DriverLocationReq): LocationShareResp
    @DELETE("api/driver/location") suspend fun stopDriverLocation(): OkResponse
    @GET("api/rides/{id}/location") suspend fun rideLocation(@Path("id") id: Long): RideLocationDto
    @POST("api/rides/{id}/passenger-location") suspend fun postPassengerLocation(@Path("id") id: Long, @Body body: PassengerLocationReq): LocationShareResp
    @DELETE("api/rides/{id}/passenger-location") suspend fun stopPassengerLocation(@Path("id") id: Long): OkResponse
    @GET("api/rides/{id}/messages") suspend fun rideMessages(@Path("id") id: Long, @Query("after") after: Long? = null): ChatThreadDto
    @POST("api/rides/{id}/messages") suspend fun postRideMessage(@Path("id") id: Long, @Body body: PostMessageReq): PostMessageRes
    @POST("api/rides/{id}/messages/read") suspend fun markRideChatRead(@Path("id") id: Long, @Body body: MarkReadReq): OkResponse
    @POST("api/rides/{id}/messages/{mid}/reactions") suspend fun reactRideMessage(@Path("id") id: Long, @Path("mid") mid: Long, @Body body: ReactReq): ReactRes
    @GET("api/driver/profile") suspend fun driverProfile(): DriverProfile
    @PUT("api/driver/profile") suspend fun updateDriverProfile(@Body body: DriverProfileReq): OkResponse
    @POST("api/driver/vehicles") suspend fun createDriverVehicle(@Body body: DriverVehicleReq): DriverVehicle
    @PUT("api/driver/vehicles/{id}") suspend fun updateDriverVehicle(@Path("id") id: Long, @Body body: DriverVehicleReq): DriverVehicle
    @DELETE("api/driver/vehicles/{id}") suspend fun deleteDriverVehicle(@Path("id") id: Long): OkResponse
    @POST("api/driver/vehicles/{id}/select") suspend fun selectDriverVehicle(@Path("id") id: Long): SelectVehicleRes

    // ---- events ----
    @GET("api/events") suspend fun events(
        @Query("scope") scope: String? = null,
        @Query("hide_drafts") hideDrafts: Boolean? = null,
        @retrofit2.http.Header("X-Passive-Poll") passivePoll: String? = null,
    ): List<EventItem>
    @GET("api/events/{id}") suspend fun event(@Path("id") id: Long): EventItem
    @POST("api/events") suspend fun createEvent(@Body body: EventUpsertReq): EventCreateRespDto
    @PUT("api/events/{id}") suspend fun updateEvent(@Path("id") id: Long, @Body body: EventUpsertReq): OkResponse
    @POST("api/events/{id}/duplicate") suspend fun duplicateEvent(@Path("id") id: Long, @Body body: EventDuplicateReq): EventCreateRespDto
    @POST("api/events/{id}/cancel") suspend fun cancelEvent(@Path("id") id: Long): OkResponse
    @POST("api/events/{id}/finish") suspend fun finishEvent(@Path("id") id: Long): OkResponse
    @GET("api/events/{id}/signups") suspend fun eventSignups(@Path("id") id: Long): List<EventSignupItemDto>
    @POST("api/events/{id}/checkin-code/reset") suspend fun resetEventCheckinCode(@Path("id") id: Long): EventCheckinResetRespDto
    @GET("api/events/{id}/visitors") suspend fun eventVisitors(@Path("id") id: Long): List<VisitorPassDto>
    @POST("api/events/{id}/visitors") suspend fun createEventVisitor(@Path("id") id: Long, @Body body: CreateVisitorPassReq): CreateVisitorPassRespDto
    @POST("api/visitor-passes/{id}/reissue") suspend fun reissueVisitorPass(@Path("id") id: Long): VisitorUrlRespDto
    @POST("api/visitor-passes/{id}/revoke") suspend fun revokeVisitorPass(@Path("id") id: Long): OkResponse
    @GET("api/events/{id}/post-event-drivers") suspend fun postEventDrivers(@Path("id") id: Long): PostEventDriversDto
    @POST("api/events/{id}/post-event-reimbursements") suspend fun createPostEventReimbursements(
        @Path("id") id: Long,
        @Body body: PostEventReimbursementsReq,
    ): PostEventReimbBatchRespDto
    @POST("api/events/{id}/signup") suspend fun signup(@Path("id") id: Long, @Body body: SignupReq): EventSignupRespDto
    @POST("api/events/{id}/transport") suspend fun setEventTransport(@Path("id") id: Long, @Body body: EventTransportReq): OkResponse
    @POST("api/events/{id}/withdraw") suspend fun withdraw(@Path("id") id: Long): OkResponse
    @POST("api/events/{id}/publish") suspend fun publishEvent(@Path("id") id: Long): OkResponse
    @POST("api/events/{id}/unpublish") suspend fun unpublishEvent(@Path("id") id: Long): OkResponse
    @GET("api/events/{id}/checkin") suspend fun eventCheckinInfo(@Path("id") id: Long): CheckinInfoDto
    @POST("api/events/{id}/checkin") suspend fun selfCheckin(@Path("id") id: Long, @Body body: CheckinReq): CheckinResp
    @POST("api/events/checkin") suspend fun checkinByCode(@Body body: CheckinByCodeReq): CheckinResp
    @POST("api/events/{id}/checkin/scan-member") suspend fun scanMemberPass(@Path("id") id: Long, @Body body: ScanMemberReq): ScanMemberResp
    @POST("api/events/{id}/attendance") suspend fun markAttendance(@Path("id") id: Long, @Body body: MarkAttendanceReq): OkResponse

    // ---- costs / settlements ----
    @GET("api/me/shares") suspend fun myShares(): List<Share>
    @POST("api/shares/{id}/proof") suspend fun submitProof(@Path("id") id: Long, @Body body: ProofReq): OkResponse

    // ---- traffic & admin ----
    @GET("api/admin/dashboard") suspend fun adminDashboard(
        @retrofit2.http.Header("X-Passive-Poll") passivePoll: String? = null,
    ): AdminDashboardStatsDto
    @GET("api/admin/rides") suspend fun adminRides(
        @Query("status") status: String? = null,
        @Query("date") date: String? = null,
        @Query("q") q: String? = null,
        @Query("attention") attention: Boolean? = null,
        @Query("sort") sort: String? = null,
        @Query("dir") dir: String? = null,
        @Query("page") page: Int = 1,
        @Query("limit") limit: Int = 50,
    ): AdminRidesPageDto
    @GET("api/admin/live") suspend fun adminLive(): LiveDriversDto
    @GET("api/admin/drivers") suspend fun adminDrivers(): List<AdminDriverDto>
    @POST("api/rides/{id}/approve") suspend fun staffApproveRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/return") suspend fun staffReturnRide(@Path("id") id: Long, @Body body: CancelReq): OkResponse
    @POST("api/rides/{id}/reassign") suspend fun staffReassignRide(@Path("id") id: Long, @Body body: StaffReassignReq): OkResponse
    @POST("api/rides/{id}/alt-transport") suspend fun staffAltTransportRide(@Path("id") id: Long, @Body body: CancelReq): OkResponse
    @PUT("api/rides/{id}/reimbursement-excluded") suspend fun staffSetReimbursementExcluded(
        @Path("id") id: Long,
        @Body body: StaffReimbExcludedReq,
    ): ReimbExcludedRespDto

    // ---- ride change drafts ----
    @GET("api/admin/ride-drafts") suspend fun listAllRideDrafts(
        @Query("status") status: String? = null,
        @Query("ride_id") rideId: Long? = null,
    ): List<RideChangeDraftDto>
    @GET("api/rides/{id}/drafts") suspend fun listRideDrafts(@Path("id") id: Long): List<RideChangeDraftDto>
    @POST("api/rides/{id}/drafts") suspend fun createRideDraft(
        @Path("id") id: Long,
        @Body body: RideDraftInReq,
    ): RideDraftMutationResp
    @PUT("api/rides/{id}/drafts/{did}") suspend fun updateRideDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
        @Body body: RideDraftInReq,
    ): RideDraftMutationResp
    @DELETE("api/rides/{id}/drafts/{did}") suspend fun deleteRideDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): OkResponse
    @POST("api/rides/{id}/drafts/{did}/lock") suspend fun acquireRideDraftLock(
        @Path("id") id: Long,
        @Path("did") did: Long,
        @Body body: DraftLockReq,
    ): RideDraftMutationResp
    @DELETE("api/rides/{id}/drafts/{did}/lock") suspend fun releaseRideDraftLock(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): RideDraftMutationResp
    @POST("api/rides/{id}/drafts/{did}/request-approval") suspend fun requestRideDraftApproval(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): RideDraftMutationResp
    @POST("api/rides/{id}/drafts/{did}/approve") suspend fun approveRideDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): RideDraftMutationResp
    @POST("api/rides/{id}/drafts/{did}/return") suspend fun returnRideDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
        @Body body: ReturnRideDraftReq,
    ): RideDraftMutationResp

    // ---- event transport plans & drafts ----
    @GET("api/admin/traffic/plans") suspend fun trafficPlans(): List<TrafficEventPlanItemDto>
    @GET("api/admin/events/{id}/plan") suspend fun eventPlan(@Path("id") id: Long): PlanDataDto
    @POST("api/admin/events/{id}/plan") suspend fun saveEventPlan(
        @Path("id") id: Long,
        @Body body: SavePlanReq,
    ): OkResponse
    @POST("api/admin/events/{id}/plan/approve") suspend fun approveEventPlan(
        @Path("id") id: Long,
        @Body body: SavePlanReq,
    ): OkResponse
    @POST("api/admin/events/{id}/plan/arrange") suspend fun arrangeEventPlan(
        @Path("id") id: Long,
        @Body body: ArrangePlanReq,
    ): ArrangeRespDto
    @POST("api/admin/events/{id}/plan/draft") suspend fun saveLegacyPlanDraft(
        @Path("id") id: Long,
        @Body body: SavePlanDraftReq,
    ): PlanDraftMutationResp
    @POST("api/admin/events/{id}/plan/drafts") suspend fun createPlanDraft(
        @Path("id") id: Long,
        @Body body: CreatePlanDraftReq,
    ): PlanDraftMutationResp
    @PUT("api/admin/events/{id}/plan/drafts/{did}") suspend fun updatePlanDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
        @Body body: UpdatePlanDraftReq,
    ): PlanDraftMutationResp
    @DELETE("api/admin/events/{id}/plan/drafts/{did}") suspend fun deletePlanDraft(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): OkResponse
    @POST("api/admin/events/{id}/plan/drafts/{did}/lock") suspend fun acquirePlanDraftLock(
        @Path("id") id: Long,
        @Path("did") did: Long,
        @Body body: DraftLockReq,
    ): PlanDraftMutationResp
    @DELETE("api/admin/events/{id}/plan/drafts/{did}/lock") suspend fun releasePlanDraftLock(
        @Path("id") id: Long,
        @Path("did") did: Long,
    ): PlanDraftMutationResp
    @POST("api/admin/events/{id}/plan/request-approval") suspend fun requestPlanApproval(
        @Path("id") id: Long,
        @Body body: RequestPlanApprovalReq,
    ): PlanDraftMutationResp
    @POST("api/admin/events/{id}/plan/return") suspend fun returnPlanDraft(
        @Path("id") id: Long,
        @Body body: ReturnPlanDraftReq,
    ): PlanDraftMutationResp

    // ---- plan visitors & post-event reimbursements ----
    @POST("api/admin/events/{id}/plan/visitors") suspend fun createPlanVisitor(
        @Path("id") eventId: Long,
        @Body body: VisitorReq,
    ): IdRespDto
    @PUT("api/admin/events/{id}/plan/visitors/{vid}") suspend fun updatePlanVisitor(
        @Path("id") eventId: Long,
        @Path("vid") visitorId: Long,
        @Body body: VisitorReq,
    ): OkResponse
    @DELETE("api/admin/events/{id}/plan/visitors/{vid}") suspend fun deletePlanVisitor(
        @Path("id") eventId: Long,
        @Path("vid") visitorId: Long,
    ): OkResponse
    @POST("api/admin/events/{id}/plan/visitors/{vid}/reimbursement") suspend fun requestVisitorSpecialReimbursement(
        @Path("id") eventId: Long,
        @Path("vid") visitorId: Long,
        @Body body: SpecialReimbReq,
    ): IdRespDto
    @PUT("api/admin/events/{id}/plan/visitors/{vid}/reimbursement/{rid}") suspend fun resubmitVisitorSpecialReimbursement(
        @Path("id") eventId: Long,
        @Path("vid") visitorId: Long,
        @Path("rid") reimbId: Long,
        @Body body: SpecialReimbReq,
    ): OkResponse

    // ---- areas, area photos & area fares ----
    @POST("api/admin/areas") suspend fun createArea(@Body body: AreaUpsertReq): IdRespDto
    @PUT("api/admin/areas/{id}") suspend fun updateArea(@Path("id") id: Long, @Body body: AreaUpsertReq): OkResponse
    @GET("api/areas/{id}/photos") suspend fun areaPhotos(@Path("id") areaId: Long): AreaPhotosDto
    @POST("api/areas/{id}/photos") suspend fun addAreaPhoto(
        @Path("id") areaId: Long,
        @Body body: AddAreaPhotoReq,
    ): IdRespDto
    @PUT("api/areas/{id}/photos/{pid}") suspend fun updateAreaPhoto(
        @Path("id") areaId: Long,
        @Path("pid") photoId: Long,
        @Body body: UpdateAreaPhotoReq,
    ): OkResponse
    @DELETE("api/areas/{id}/photos/{pid}") suspend fun deleteAreaPhoto(
        @Path("id") areaId: Long,
        @Path("pid") photoId: Long,
    ): OkResponse
    @PUT("api/areas/{id}/photos/order") suspend fun reorderAreaPhotos(
        @Path("id") areaId: Long,
        @Body body: ReorderAreaPhotosReq,
    ): OkResponse
    @GET("api/admin/settings/fares") suspend fun areaFares(): AreaFaresRespDto
    @PUT("api/admin/settings/fares") suspend fun putAreaFare(@Body body: PutFareReq): IdRespDto
    @DELETE("api/admin/settings/fares/{id}") suspend fun deleteAreaFare(@Path("id") id: Long): OkResponse

    // ---- admin user management (members, roles, invites, recovery) ----
    @GET("api/admin/users") suspend fun adminUsers(
        @Query("q") q: String? = null,
        @Query("role") role: String? = null,
        @Query("status") status: String? = null,
        @Query("sort") sort: String? = null,
        @Query("dir") dir: String? = null,
        @Query("page") page: Int = 1,
        @Query("limit") limit: Int = 50,
    ): AdminUsersPageDto
    @PUT("api/admin/users/{id}") suspend fun updateAdminUser(
        @Path("id") id: Long,
        @Body body: UpdateAdminUserReq,
    ): OkResponse
    @GET("api/admin/roles") suspend fun adminRoles(): AdminRolesRespDto
    @GET("api/admin/invites") suspend fun adminInvites(): List<AdminInviteDto>
    @POST("api/admin/invites") suspend fun createAdminInvite(): CreateInviteResDto
    @DELETE("api/admin/invites/{id}") suspend fun revokeAdminInvite(@Path("id") id: Long): OkResponse
    @GET("api/admin/recovery") suspend fun adminRecovery(): List<RecoveryRequestDto>
    @POST("api/admin/recovery/{id}/approve") suspend fun approveRecovery(@Path("id") id: Long): ApproveRecoveryResDto
    @POST("api/admin/recovery/{id}/reject") suspend fun rejectRecovery(@Path("id") id: Long): OkResponse

    // ---- files ----
    @Multipart
    @POST("api/files") suspend fun upload(@Part file: MultipartBody.Part): FileRef
}
