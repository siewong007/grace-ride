package org.pjstatecm.graceride.data.api

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.serialization.json.Json
import retrofit2.HttpException
import java.io.IOException

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
        val body = e.response()?.errorBody()?.string()
        var code: String? = null
        var msg = e.message()
        if (!body.isNullOrBlank()) {
            try {
                val obj = apiJson.parseToJsonElement(body)
                val o = obj as? kotlinx.serialization.json.JsonObject
                code = o?.get("code")?.let { it as? kotlinx.serialization.json.JsonPrimitive }?.content
                val err = o?.get("error")?.let { it as? kotlinx.serialization.json.JsonPrimitive }?.content
                if (!err.isNullOrBlank()) msg = err
            } catch (_: Exception) {
                msg = body.take(300)
            }
        }
        when {
            e.code() == 401 -> bus.emit(AuthEvent.Unauthorized)
            e.code() == 423 || code == "SESSION_LOCKED" -> bus.emit(AuthEvent.Locked)
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
