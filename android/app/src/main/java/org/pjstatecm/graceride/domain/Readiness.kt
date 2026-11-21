package org.pjstatecm.graceride.domain

import org.pjstatecm.graceride.R

/** Passenger readiness values accepted by POST /rides/{id}/readiness (backend enum). */
object Readiness {
    const val READY = "ready"
    const val DELAYED_5M = "delayed_5m"
    const val DELAYED_10M = "delayed_10m"
    const val UNABLE = "unable_to_join"

    val values = listOf(READY, DELAYED_5M, DELAYED_10M, UNABLE)

    fun labelRes(value: String): Int = when (value) {
        READY -> R.string.readiness_ready
        DELAYED_5M -> R.string.readiness_delayed_5m
        DELAYED_10M -> R.string.readiness_delayed_10m
        UNABLE -> R.string.readiness_unable
        else -> R.string.readiness_ready
    }
}

/**
 * The passkey start endpoints return {challenge_id, options} where `options`
 * wraps the WebAuthn request under `publicKey` (webauthn-rs shape, same as the
 * web client consumes). Credential Manager wants the inner publicKey object.
 */
object PasskeyOptions {
    fun publicKey(options: kotlinx.serialization.json.JsonObject): kotlinx.serialization.json.JsonObject =
        options["publicKey"] as? kotlinx.serialization.json.JsonObject ?: options
}
