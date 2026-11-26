package org.pjstatecm.graceride.data.repo

import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.pjstatecm.graceride.data.api.AddAreaPhotoReq
import org.pjstatecm.graceride.data.api.AdminDashboardStatsDto
import org.pjstatecm.graceride.data.api.AdminDriverDto
import org.pjstatecm.graceride.data.api.AdminRidesPageDto
import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.AreaFaresRespDto
import org.pjstatecm.graceride.data.api.AreaPhotosDto
import org.pjstatecm.graceride.data.api.AreaUpsertReq
import org.pjstatecm.graceride.data.api.ArrangePlanReq
import org.pjstatecm.graceride.data.api.ArrangeRespDto
import org.pjstatecm.graceride.data.api.AuthEventBus
import org.pjstatecm.graceride.data.api.CancelReq
import org.pjstatecm.graceride.data.api.CreatePlanDraftReq
import org.pjstatecm.graceride.data.api.DraftLockReq
import org.pjstatecm.graceride.data.api.FileRef
import org.pjstatecm.graceride.data.api.IdRespDto
import org.pjstatecm.graceride.data.api.LiveDriversDto
import org.pjstatecm.graceride.data.api.PlanDataDto
import org.pjstatecm.graceride.data.api.PlanDraftEntryDto
import org.pjstatecm.graceride.data.api.PlanDraftMutationResp
import org.pjstatecm.graceride.data.api.PostEventDriversDto
import org.pjstatecm.graceride.data.api.PostEventReimbursementsReq
import org.pjstatecm.graceride.data.api.PutFareReq
import org.pjstatecm.graceride.data.api.ReimbExcludedRespDto
import org.pjstatecm.graceride.data.api.ReorderAreaPhotosReq
import org.pjstatecm.graceride.data.api.RequestPlanApprovalReq
import org.pjstatecm.graceride.data.api.ReturnPlanDraftReq
import org.pjstatecm.graceride.data.api.ReturnRideDraftReq
import org.pjstatecm.graceride.data.api.RideChangeDraftDto
import org.pjstatecm.graceride.data.api.RideDraftInReq
import org.pjstatecm.graceride.data.api.RideDraftMutationResp
import org.pjstatecm.graceride.data.api.SavePlanDraftReq
import org.pjstatecm.graceride.data.api.SavePlanReq
import org.pjstatecm.graceride.data.api.SpecialReimbReq
import org.pjstatecm.graceride.data.api.StaffReassignReq
import org.pjstatecm.graceride.data.api.StaffReimbExcludedReq
import org.pjstatecm.graceride.data.api.TrafficEventPlanItemDto
import org.pjstatecm.graceride.data.api.UpdateAreaPhotoReq
import org.pjstatecm.graceride.data.api.UpdatePlanDraftReq
import org.pjstatecm.graceride.data.api.VisitorReq
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.runSuspendCatching

/** Repository for Traffic Admin plans, drafts, rides, live map, visitors, post-event claims, areas, photos and fares. */
class TrafficRepository(
    private val apiProvider: ApiProvider,
    private val bus: AuthEventBus,
) {
    private val api get() = apiProvider.get()

    suspend fun dashboardStats(passive: Boolean = false): Result<AdminDashboardStatsDto> =
        runSuspendCatching { apiCall(bus) { api.adminDashboard(if (passive) "1" else null) } }

    suspend fun trafficPlans(): Result<List<TrafficEventPlanItemDto>> =
        runSuspendCatching { apiCall(bus) { api.trafficPlans() } }

    suspend fun adminRides(
        status: String? = null,
        date: String? = null,
        q: String? = null,
        attention: Boolean? = null,
        sort: String? = null,
        dir: String? = null,
        page: Int = 1,
        limit: Int = 50,
    ): Result<AdminRidesPageDto> = runSuspendCatching {
        apiCall(bus) {
            api.adminRides(
                status = status?.ifBlank { null },
                date = date?.ifBlank { null },
                q = q?.trim()?.ifBlank { null },
                attention = attention,
                sort = sort,
                dir = dir,
                page = page,
                limit = limit,
            )
        }
    }

    suspend fun adminLive(): Result<LiveDriversDto> =
        runSuspendCatching { apiCall(bus) { api.adminLive() } }

    suspend fun adminDrivers(): Result<List<AdminDriverDto>> =
        runSuspendCatching { apiCall(bus) { api.adminDrivers() } }

    suspend fun staffApproveRide(rideId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.staffApproveRide(rideId) }; Unit }

    suspend fun staffReturnRide(rideId: Long, reason: String?): Result<Unit> =
        runSuspendCatching {
            apiCall(bus) { api.staffReturnRide(rideId, CancelReq(reason?.trim()?.ifEmpty { null })) }
            Unit
        }

    suspend fun staffReassignRide(rideId: Long, driverId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.staffReassignRide(rideId, StaffReassignReq(driverId)) }; Unit }

    suspend fun staffAltTransportRide(rideId: Long, reason: String?): Result<Unit> =
        runSuspendCatching {
            apiCall(bus) { api.staffAltTransportRide(rideId, CancelReq(reason?.trim()?.ifEmpty { null })) }
            Unit
        }

    suspend fun staffSetReimbursementExcluded(rideId: Long, excluded: Boolean): Result<ReimbExcludedRespDto> =
        runSuspendCatching {
            apiCall(bus) { api.staffSetReimbursementExcluded(rideId, StaffReimbExcludedReq(excluded)) }
        }

    // ---- Ride change drafts ----

    suspend fun listAllRideDrafts(
        status: String? = null,
        rideId: Long? = null,
    ): Result<List<RideChangeDraftDto>> = runSuspendCatching {
        apiCall(bus) { api.listAllRideDrafts(status = status?.ifBlank { null }, rideId = rideId) }
    }

    suspend fun listRideDrafts(rideId: Long): Result<List<RideChangeDraftDto>> =
        runSuspendCatching { apiCall(bus) { api.listRideDrafts(rideId) } }

    suspend fun createRideDraft(rideId: Long, req: RideDraftInReq): Result<RideDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.createRideDraft(rideId, req) } }

    suspend fun updateRideDraft(rideId: Long, draftId: Long, req: RideDraftInReq): Result<RideDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.updateRideDraft(rideId, draftId, req) } }

    suspend fun deleteRideDraft(rideId: Long, draftId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deleteRideDraft(rideId, draftId) }; Unit }

    suspend fun acquireRideDraftLock(
        rideId: Long,
        draftId: Long,
        force: Boolean = false,
    ): Result<RideDraftMutationResp> = runSuspendCatching {
        apiCall(bus) { api.acquireRideDraftLock(rideId, draftId, DraftLockReq(force = force)) }
    }

    suspend fun releaseRideDraftLock(rideId: Long, draftId: Long): Result<RideDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.releaseRideDraftLock(rideId, draftId) } }

    suspend fun requestRideDraftApproval(rideId: Long, draftId: Long): Result<RideDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.requestRideDraftApproval(rideId, draftId) } }

    suspend fun approveRideDraft(rideId: Long, draftId: Long): Result<RideDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.approveRideDraft(rideId, draftId) } }

    suspend fun returnRideDraft(rideId: Long, draftId: Long, notes: String?): Result<RideDraftMutationResp> =
        runSuspendCatching {
            apiCall(bus) { api.returnRideDraft(rideId, draftId, ReturnRideDraftReq(notes?.trim()?.ifEmpty { null })) }
        }

    // ---- Event transport plans & plan drafts ----

    suspend fun eventPlan(eventId: Long): Result<PlanDataDto> =
        runSuspendCatching { apiCall(bus) { api.eventPlan(eventId) } }

    suspend fun saveEventPlan(eventId: Long, req: SavePlanReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.saveEventPlan(eventId, req) }; Unit }

    suspend fun approveEventPlan(eventId: Long, req: SavePlanReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.approveEventPlan(eventId, req) }; Unit }

    suspend fun arrangeEventPlan(eventId: Long, draft: List<PlanDraftEntryDto>): Result<ArrangeRespDto> =
        runSuspendCatching { apiCall(bus) { api.arrangeEventPlan(eventId, ArrangePlanReq(draft)) } }

    suspend fun saveLegacyPlanDraft(eventId: Long, req: SavePlanDraftReq): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.saveLegacyPlanDraft(eventId, req) } }

    suspend fun createPlanDraft(eventId: Long, req: CreatePlanDraftReq): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.createPlanDraft(eventId, req) } }

    suspend fun updatePlanDraft(eventId: Long, draftId: Long, req: UpdatePlanDraftReq): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.updatePlanDraft(eventId, draftId, req) } }

    suspend fun deletePlanDraft(eventId: Long, draftId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deletePlanDraft(eventId, draftId) }; Unit }

    suspend fun acquirePlanDraftLock(
        eventId: Long,
        draftId: Long,
        force: Boolean = false,
    ): Result<PlanDraftMutationResp> = runSuspendCatching {
        apiCall(bus) { api.acquirePlanDraftLock(eventId, draftId, DraftLockReq(force = force)) }
    }

    suspend fun releasePlanDraftLock(eventId: Long, draftId: Long): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.releasePlanDraftLock(eventId, draftId) } }

    suspend fun requestPlanApproval(eventId: Long, req: RequestPlanApprovalReq): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.requestPlanApproval(eventId, req) } }

    suspend fun returnPlanDraft(eventId: Long, req: ReturnPlanDraftReq): Result<PlanDraftMutationResp> =
        runSuspendCatching { apiCall(bus) { api.returnPlanDraft(eventId, req) } }

    // ---- Plan visitors & post-event reimbursements ----

    suspend fun createPlanVisitor(eventId: Long, req: VisitorReq): Result<IdRespDto> =
        runSuspendCatching { apiCall(bus) { api.createPlanVisitor(eventId, req) } }

    suspend fun updatePlanVisitor(eventId: Long, visitorId: Long, req: VisitorReq): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.updatePlanVisitor(eventId, visitorId, req) }; Unit }

    suspend fun deletePlanVisitor(eventId: Long, visitorId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deletePlanVisitor(eventId, visitorId) }; Unit }

    suspend fun requestVisitorSpecialReimbursement(
        eventId: Long,
        visitorId: Long,
        amountCents: Long,
        reason: String,
    ): Result<IdRespDto> = runSuspendCatching {
        apiCall(bus) {
            api.requestVisitorSpecialReimbursement(
                eventId,
                visitorId,
                SpecialReimbReq(amount_cents = amountCents, reason = reason.trim()),
            )
        }
    }

    suspend fun resubmitVisitorSpecialReimbursement(
        eventId: Long,
        visitorId: Long,
        reimbId: Long,
        amountCents: Long,
        reason: String,
    ): Result<Unit> = runSuspendCatching {
        apiCall(bus) {
            api.resubmitVisitorSpecialReimbursement(
                eventId,
                visitorId,
                reimbId,
                SpecialReimbReq(amount_cents = amountCents, reason = reason.trim()),
            )
        }
        Unit
    }

    suspend fun postEventDrivers(eventId: Long): Result<PostEventDriversDto> =
        runSuspendCatching { apiCall(bus) { api.postEventDrivers(eventId) } }

    suspend fun createPostEventReimbursements(
        eventId: Long,
        req: PostEventReimbursementsReq,
    ): Result<Unit> = runSuspendCatching {
        apiCall(bus) { api.createPostEventReimbursements(eventId, req) }
        Unit
    }

    // ---- Areas, area photos & area fares ----

    suspend fun createArea(name: String, active: Boolean = true): Result<IdRespDto> =
        runSuspendCatching { apiCall(bus) { api.createArea(AreaUpsertReq(name.trim(), active)) } }

    suspend fun updateArea(id: Long, name: String, active: Boolean): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.updateArea(id, AreaUpsertReq(name.trim(), active)) }; Unit }

    suspend fun areaPhotos(areaId: Long): Result<AreaPhotosDto> =
        runSuspendCatching { apiCall(bus) { api.areaPhotos(areaId) } }

    suspend fun uploadPhotoFile(bytes: ByteArray, fileName: String, mimeType: String): Result<FileRef> =
        runSuspendCatching {
            val part = MultipartBody.Part.createFormData(
                "file",
                fileName,
                bytes.toRequestBody(mimeType.toMediaTypeOrNull()),
            )
            apiCall(bus) { api.upload(part) }
        }

    suspend fun addAreaPhoto(areaId: Long, fileId: Long, caption: String?): Result<IdRespDto> =
        runSuspendCatching {
            apiCall(bus) { api.addAreaPhoto(areaId, AddAreaPhotoReq(fileId, caption?.trim()?.ifEmpty { null })) }
        }

    suspend fun updateAreaPhoto(areaId: Long, photoId: Long, caption: String?): Result<Unit> =
        runSuspendCatching {
            apiCall(bus) { api.updateAreaPhoto(areaId, photoId, UpdateAreaPhotoReq(caption?.trim().orEmpty())) }
            Unit
        }

    suspend fun deleteAreaPhoto(areaId: Long, photoId: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deleteAreaPhoto(areaId, photoId) }; Unit }

    suspend fun reorderAreaPhotos(areaId: Long, photoIds: List<Long>): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.reorderAreaPhotos(areaId, ReorderAreaPhotosReq(photoIds)) }; Unit }

    suspend fun areaFares(): Result<AreaFaresRespDto> =
        runSuspendCatching { apiCall(bus) { api.areaFares() } }

    suspend fun putAreaFare(areaAId: Long, areaBId: Long, amountCents: Long): Result<IdRespDto> =
        runSuspendCatching { apiCall(bus) { api.putAreaFare(PutFareReq(areaAId, areaBId, amountCents)) } }

    suspend fun deleteAreaFare(id: Long): Result<Unit> =
        runSuspendCatching { apiCall(bus) { api.deleteAreaFare(id) }; Unit }

    suspend fun bulkAreaFares(
        fares: List<org.pjstatecm.graceride.data.api.BulkFareItemReq>,
        createMissingAreas: Boolean,
    ): Result<org.pjstatecm.graceride.data.api.BulkFaresRespDto> =
        runSuspendCatching {
            apiCall(bus) {
                api.bulkAreaFares(
                    org.pjstatecm.graceride.data.api.BulkFaresReq(
                        fares = fares,
                        createMissingAreas = createMissingAreas,
                    ),
                )
            }
        }

    suspend fun exportFaresCsvBytes(): Result<ByteArray> =
        runSuspendCatching { apiCall(bus) { api.exportFaresCsv().bytes() } }
}
