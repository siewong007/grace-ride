package org.pjstatecm.graceride.data.api

import okhttp3.MultipartBody
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.Multipart
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
    @POST("api/auth/logout") suspend fun logout(): OkResponse
    @POST("api/auth/logout-all") suspend fun logoutAll(): OkResponse
    @POST("api/auth/lock") suspend fun lock(): OkResponse
    @POST("api/auth/unlock") suspend fun unlock(@Body body: UnlockReq): OkResponse
    @POST("api/auth/terms") suspend fun acceptTerms(@Body body: TermsReq): OkResponse
    @POST("api/auth/passkey/login/start") suspend fun passkeyLoginStart(): PasskeyStart
    @POST("api/auth/passkey/login/finish") suspend fun passkeyLoginFinish(@Body body: PasskeyFinishReq): OkResponse
    @POST("api/auth/passkey/register/start") suspend fun passkeyRegisterStart(): PasskeyStart
    @POST("api/auth/passkey/register/finish") suspend fun passkeyRegisterFinish(@Body body: PasskeyFinishReq): OkResponse
    @GET("api/auth/passkeys") suspend fun passkeys(): List<PasskeyInfo>
    @DELETE("api/auth/passkeys/{id}") suspend fun deletePasskey(@Path("id") id: Long): OkResponse

    // ---- profile & community ----
    @PUT("api/me") suspend fun updateProfile(@Body body: ProfileUpdateReq): OkResponse
    @GET("api/announcements") suspend fun announcements(): List<Announcement>
    @GET("api/notifications") suspend fun notifications(): List<NotificationItem>
    @POST("api/notifications/read-all") suspend fun readAllNotifications(): OkResponse
    @POST("api/notifications/{id}/read") suspend fun readNotification(@Path("id") id: Long): OkResponse

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
    @GET("api/rides") suspend fun myRides(@Query("scope") scope: String? = null): List<Ride>
    @GET("api/rides/{id}") suspend fun ride(@Path("id") id: Long): Ride
    @POST("api/rides") suspend fun createRide(@Body body: CreateRideReq): CreateRideRes
    @POST("api/rides/{id}/cancel") suspend fun cancelRide(@Path("id") id: Long, @Body body: CancelReq): OkResponse
    @POST("api/rides/{id}/respond") suspend fun respondRide(@Path("id") id: Long, @Body body: RespondReq): OkResponse
    @POST("api/rides/{id}/readiness") suspend fun readiness(@Path("id") id: Long, @Body body: ReadinessReq): OkResponse
    @POST("api/rides/{id}/confirm-assignment") suspend fun confirmAssignment(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/start") suspend fun startRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/arrived-gate") suspend fun arrivedGate(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/pickup") suspend fun pickupRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/complete") suspend fun completeRide(@Path("id") id: Long): OkResponse
    @POST("api/rides/{id}/driver-delay") suspend fun driverDelay(@Path("id") id: Long, @Body body: DriverDelayReq): OkResponse
    @POST("api/rides/{id}/withdraw") suspend fun withdrawRide(@Path("id") id: Long, @Body body: WithdrawReq): OkResponse
    @GET("api/driver/schedule") suspend fun driverSchedule(@Query("from") from: String, @Query("to") to: String): List<Ride>
    @GET("api/driver/requests") suspend fun driverRequests(@Query("mine_areas") mineAreas: Boolean? = null): List<Ride>
    @GET("api/driver/profile") suspend fun driverProfile(): DriverProfile
    @PUT("api/driver/profile") suspend fun updateDriverProfile(@Body body: DriverProfileReq): OkResponse
    @POST("api/driver/vehicles") suspend fun createDriverVehicle(@Body body: DriverVehicleReq): DriverVehicle
    @PUT("api/driver/vehicles/{id}") suspend fun updateDriverVehicle(@Path("id") id: Long, @Body body: DriverVehicleReq): DriverVehicle
    @DELETE("api/driver/vehicles/{id}") suspend fun deleteDriverVehicle(@Path("id") id: Long): OkResponse
    @POST("api/driver/vehicles/{id}/select") suspend fun selectDriverVehicle(@Path("id") id: Long): SelectVehicleRes

    // ---- events ----
    @GET("api/events") suspend fun events(@Query("scope") scope: String? = null): List<EventItem>
    @GET("api/events/{id}") suspend fun event(@Path("id") id: Long): EventItem
    @POST("api/events/{id}/signup") suspend fun signup(@Path("id") id: Long, @Body body: SignupReq): OkResponse
    @POST("api/events/{id}/withdraw") suspend fun withdraw(@Path("id") id: Long): OkResponse

    // ---- costs / settlements ----
    @GET("api/me/shares") suspend fun myShares(): List<Share>
    @POST("api/shares/{id}/proof") suspend fun submitProof(@Path("id") id: Long, @Body body: ProofReq): OkResponse

    // ---- files ----
    @Multipart
    @POST("api/files") suspend fun upload(@Part file: MultipartBody.Part): FileRef
}
