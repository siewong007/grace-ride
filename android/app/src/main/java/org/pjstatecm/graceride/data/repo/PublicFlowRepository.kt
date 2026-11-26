package org.pjstatecm.graceride.data.repo

import org.pjstatecm.graceride.data.api.ApiProvider
import org.pjstatecm.graceride.data.api.ApiService
import org.pjstatecm.graceride.data.api.CareStatusDto
import org.pjstatecm.graceride.data.api.InviteAcceptReq
import org.pjstatecm.graceride.data.api.InvitePreviewDto
import org.pjstatecm.graceride.data.api.LicensePackageDto
import org.pjstatecm.graceride.data.api.OkResponse
import org.pjstatecm.graceride.data.api.ReadinessReq
import org.pjstatecm.graceride.data.api.RecoveryRequestIn
import org.pjstatecm.graceride.data.api.RideLocationDto
import org.pjstatecm.graceride.data.api.TurnstileConfigDto
import org.pjstatecm.graceride.data.api.VisitClaimReq
import org.pjstatecm.graceride.data.api.VisitPreviewDto
import org.pjstatecm.graceride.data.api.VisitRideDto
import org.pjstatecm.graceride.data.api.publicApiCall

/**
 * Repository for public / no-account endpoints (`recovery-request`, `/invites`, `/visit`, `/care`,
 * `/third-party-licenses.json`). Uses [publicApiCall] so 401/403/423 never wipe member auth state.
 */
class PublicFlowRepository(private val apiProvider: ApiProvider) {
    private val api: ApiService get() = apiProvider.get()

    suspend fun turnstileConfig(): TurnstileConfigDto = publicApiCall { api.turnstileConfig() }

    suspend fun recoveryRequest(
        email: String,
        reason: String,
        turnstileToken: String? = null,
    ): OkResponse = publicApiCall {
        api.recoveryRequest(
            RecoveryRequestIn(
                email = email.trim(),
                reason = reason.trim(),
                turnstile_token = turnstileToken,
            ),
        )
    }

    suspend fun invitePreview(token: String): InvitePreviewDto =
        publicApiCall { api.invitePreview(token) }

    suspend fun inviteAccept(token: String, req: InviteAcceptReq): OkResponse =
        publicApiCall { api.inviteAccept(token, req) }

    suspend fun visitPreview(token: String): VisitPreviewDto =
        publicApiCall { api.visitPreview(token) }

    suspend fun visitClaim(token: String, turnstileToken: String? = null): OkResponse =
        publicApiCall { api.visitClaim(token, VisitClaimReq(turnstile_token = turnstileToken)) }

    suspend fun visitMe(): VisitRideDto =
        publicApiCall { api.visitMe() }

    suspend fun visitLocation(): RideLocationDto =
        publicApiCall { api.visitLocation() }

    suspend fun visitReadiness(status: String, notes: String? = null): OkResponse =
        publicApiCall { api.visitReadiness(ReadinessReq(status = status, notes = notes)) }

    suspend fun careStatus(token: String): CareStatusDto =
        publicApiCall { api.careStatus(token) }

    suspend fun careReadiness(token: String, status: String, notes: String? = null): OkResponse =
        publicApiCall { api.careReadiness(token, ReadinessReq(status = status, notes = notes)) }

    suspend fun thirdPartyLicenses(): List<LicensePackageDto> =
        publicApiCall { api.thirdPartyLicenses() }
}
