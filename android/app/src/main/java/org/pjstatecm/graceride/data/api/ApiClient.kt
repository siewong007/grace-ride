package org.pjstatecm.graceride.data.api

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import retrofit2.HttpException
import java.io.IOException

/** Runs a suspending block with [Result] handling while always propagating [CancellationException]. */
suspend inline fun <T> runSuspendCatching(block: () -> T): Result<T> = try {
    Result.success(block())
} catch (ce: CancellationException) {
    throw ce
} catch (t: Throwable) {
    Result.failure(t)
}

/** Server error: HTTP status + the API's stable error code + localized message. */
class ApiException(
    val status: Int,
    val code: String?,
    override val message: String,
) : Exception(message)

sealed interface AuthEvent {
    data object Unauthorized : AuthEvent
    data object Locked : AuthEvent
    data object Pending : AuthEvent
}

class AuthEventBus {
    private val _events = MutableSharedFlow<AuthEvent>(extraBufferCapacity = 8)
    val events: SharedFlow<AuthEvent> = _events
    fun emit(event: AuthEvent) { _events.tryEmit(event) }
}

val apiJson: Json = Json {
    ignoreUnknownKeys = true
    coerceInputValues = true
    explicitNulls = false
}

/**
 * Runs an API call, mapping failures to [ApiException] and broadcasting session
 * events (401 → sign-in, 423 → unlock, 403 ACCOUNT_PENDING → approval screen),
 * mirroring the web client's global handling in frontend/src/api.ts.
 */
suspend fun <T> apiCall(bus: AuthEventBus, block: suspend () -> T): T {
    try {
        return block()
    } catch (e: HttpException) {
        val body = runCatching { e.response()?.errorBody()?.use { it.string() } }.getOrNull()
        var code: String? = null
        var msg = e.message()
        if (!body.isNullOrBlank()) {
            try {
                val obj = apiJson.parseToJsonElement(body)
                val o = obj as? kotlinx.serialization.json.JsonObject
                code = (o?.get("code") as? JsonPrimitive)?.contentOrNull?.takeIf { it.isNotBlank() }
                val err = (o?.get("error") as? JsonPrimitive)?.contentOrNull
                if (!err.isNullOrBlank()) msg = err
            } catch (_: Exception) {
                msg = body.take(300)
            }
        }
        when {
            e.code() == 401 -> bus.emit(AuthEvent.Unauthorized)
            (e.code() == 423 && code != "DRAFT_LOCKED_BY_OTHER") || code == "SESSION_LOCKED" -> {
                bus.emit(AuthEvent.Locked)
            }
            e.code() == 403 && code == "ACCOUNT_PENDING" -> bus.emit(AuthEvent.Pending)
        }
        throw ApiException(e.code(), code, msg)
    } catch (e: IOException) {
        throw ApiException(0, null, e.message ?: "Network error")
    } catch (e: kotlinx.serialization.SerializationException) {
        // A shape the client did not expect must degrade to an error state, never crash.
        throw ApiException(-1, "DECODE_ERROR", e.message ?: "Unexpected server response")
    }
}

/**
 * Runs a public/no-account API call (invite preview/accept, visitor pass, care link,
 * recovery request) without emitting member session [AuthEvent]s on HTTP 401/403/423.
 */
suspend fun <T> publicApiCall(block: suspend () -> T): T {
    try {
        return block()
    } catch (e: HttpException) {
        val body = runCatching { e.response()?.errorBody()?.use { it.string() } }.getOrNull()
        var code: String? = null
        var msg = e.message()
        if (!body.isNullOrBlank()) {
            try {
                val obj = apiJson.parseToJsonElement(body)
                val o = obj as? kotlinx.serialization.json.JsonObject
                code = (o?.get("code") as? JsonPrimitive)?.contentOrNull?.takeIf { it.isNotBlank() }
                val err = (o?.get("error") as? JsonPrimitive)?.contentOrNull
                if (!err.isNullOrBlank()) msg = err
            } catch (_: Exception) {
                msg = body.take(300)
            }
        }
        throw ApiException(e.code(), code, msg)
    } catch (e: IOException) {
        throw ApiException(0, null, e.message ?: "Network error")
    } catch (e: kotlinx.serialization.SerializationException) {
        throw ApiException(-1, "DECODE_ERROR", e.message ?: "Unexpected server response")
    }
}
