package org.pjstatecm.graceride.platform

import android.content.Context
import android.content.ContextWrapper
import android.os.Build
import android.os.SystemClock
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.SecureStore
import java.security.GeneralSecurityException
import java.security.InvalidKeyException
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.PrivateKey
import java.security.ProviderException
import java.security.SecureRandom
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import java.util.Base64
import kotlin.coroutines.resume

enum class BiometricAvailability {
    AVAILABLE,
    NONE_ENROLLED,
    NO_HARDWARE,
    UNAVAILABLE,
}

sealed interface BiometricSignResult {
    data class Success(
        val signatureB64Url: String,
        val publicKeyB64Url: String = "",
        val attestationChain: List<String>? = null,
    ) : BiometricSignResult

    data object Cancelled : BiometricSignResult
    data object KeyInvalidated : BiometricSignResult
    data class Error(val message: String) : BiometricSignResult
}

interface BiometricCryptoDriver {
    fun availability(context: Context?): BiometricAvailability

    suspend fun generateAndSign(
        context: Context,
        payload: ByteArray,
        title: String,
        subtitle: String,
        cancelText: String,
        attestationChallenge: ByteArray? = null,
    ): BiometricSignResult

    suspend fun signWithEnrolledKey(
        context: Context,
        payload: ByteArray,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult

    suspend fun verifyLocalUnlock(
        context: Context,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult

    fun deleteKey()
}

tailrec fun Context.findFragmentActivity(): FragmentActivity? = when (this) {
    is FragmentActivity -> this
    is ContextWrapper -> baseContext?.findFragmentActivity()
    else -> null
}

class AndroidBiometricCryptoDriver : BiometricCryptoDriver {

    override fun availability(context: Context?): BiometricAvailability {
        val ctx = context ?: return BiometricAvailability.NO_HARDWARE
        return when (
            BiometricManager.from(ctx).canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)
        ) {
            BiometricManager.BIOMETRIC_SUCCESS -> BiometricAvailability.AVAILABLE
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricAvailability.NONE_ENROLLED
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE,
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE,
            BiometricManager.BIOMETRIC_ERROR_SECURITY_UPDATE_REQUIRED,
            BiometricManager.BIOMETRIC_ERROR_UNSUPPORTED -> BiometricAvailability.NO_HARDWARE
            else -> BiometricAvailability.UNAVAILABLE
        }
    }

    override suspend fun generateAndSign(
        context: Context,
        payload: ByteArray,
        title: String,
        subtitle: String,
        cancelText: String,
        attestationChallenge: ByteArray?,
    ): BiometricSignResult {
        val activity = context.findFragmentActivity()
            ?: return BiometricSignResult.Error("Biometric prompt requires an active screen")
        val (keyPair, usedAttestation) = try {
            withContext(Dispatchers.IO) {
                deleteKey()
                val kpg = KeyPairGenerator.getInstance(
                    KeyProperties.KEY_ALGORITHM_EC,
                    ANDROID_KEYSTORE,
                )
                fun buildSpec(strongBox: Boolean, withAttestation: Boolean): KeyGenParameterSpec = KeyGenParameterSpec.Builder(
                    KEY_ALIAS,
                    KeyProperties.PURPOSE_SIGN,
                )
                    .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                    .setDigests(KeyProperties.DIGEST_SHA256)
                    .setUserAuthenticationRequired(true)
                    .setInvalidatedByBiometricEnrollment(true)
                    .apply {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                            setUserAuthenticationParameters(
                                0,
                                KeyProperties.AUTH_BIOMETRIC_STRONG,
                            )
                        }
                        if (withAttestation && attestationChallenge != null && attestationChallenge.isNotEmpty()) {
                            setAttestationChallenge(attestationChallenge)
                        }
                        if (strongBox) {
                            setIsStrongBoxBacked(true)
                        }
                    }
                    .build()

                val wantAttestation = attestationChallenge != null && attestationChallenge.isNotEmpty()
                try {
                    kpg.initialize(buildSpec(strongBox = true, withAttestation = wantAttestation))
                    kpg.generateKeyPair() to wantAttestation
                } catch (e: Exception) {
                    if (e !is GeneralSecurityException && e !is ProviderException) throw e
                    try {
                        kpg.initialize(buildSpec(strongBox = false, withAttestation = wantAttestation))
                        kpg.generateKeyPair() to wantAttestation
                    } catch (e2: Exception) {
                        if (!wantAttestation || (e2 !is GeneralSecurityException && e2 !is ProviderException)) {
                            throw e2
                        }
                        kpg.initialize(buildSpec(strongBox = false, withAttestation = false))
                        kpg.generateKeyPair() to false
                    }
                }
            }
        } catch (ce: CancellationException) {
            throw ce
        } catch (e: Exception) {
            return BiometricSignResult.Error(e.message ?: "Could not create biometric hardware key")
        }

        val attestationChain = if (usedAttestation) {
            withContext(Dispatchers.IO) {
                runCatching {
                    val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
                    ks.getCertificateChain(KEY_ALIAS)?.map {
                        Base64.getEncoder().encodeToString(it.encoded)
                    }
                }.getOrNull()
            }
        } else {
            null
        }

        val signature = try {
            Signature.getInstance(SIGN_ALGORITHM).apply {
                initSign(keyPair.private)
            }
        } catch (e: Exception) {
            deleteKey()
            return BiometricSignResult.Error(e.message ?: "Could not initialize biometric signature")
        }

        val pubB64 = Base64.getUrlEncoder().withoutPadding().encodeToString(keyPair.public.encoded)
        val result = promptAndSign(activity, signature, payload, pubB64, title, subtitle, cancelText)
        if (result !is BiometricSignResult.Success) {
            deleteKey()
            return result
        }
        return result.copy(attestationChain = attestationChain)
    }

    override suspend fun signWithEnrolledKey(
        context: Context,
        payload: ByteArray,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult {
        val activity = context.findFragmentActivity()
            ?: return BiometricSignResult.Error("Biometric prompt requires an active screen")
        val entry = withContext(Dispatchers.IO) { loadPrivateKeyEntry() }
            ?: return BiometricSignResult.KeyInvalidated
        val signature = try {
            Signature.getInstance(SIGN_ALGORITHM).apply {
                initSign(entry.privateKey)
            }
        } catch (_: KeyPermanentlyInvalidatedException) {
            deleteKey()
            return BiometricSignResult.KeyInvalidated
        } catch (_: InvalidKeyException) {
            deleteKey()
            return BiometricSignResult.KeyInvalidated
        } catch (e: Exception) {
            return BiometricSignResult.Error(e.message ?: "Could not initialize biometric signature")
        }

        val pubB64 = Base64.getUrlEncoder().withoutPadding().encodeToString(entry.certificate.publicKey.encoded)
        return promptAndSign(activity, signature, payload, pubB64, title, subtitle, cancelText)
    }

    override suspend fun verifyLocalUnlock(
        context: Context,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult {
        val activity = context.findFragmentActivity()
            ?: return BiometricSignResult.Error("Biometric prompt requires an active screen")
        val entry = withContext(Dispatchers.IO) { loadPrivateKeyEntry() }
            ?: return BiometricSignResult.KeyInvalidated
        val signature = try {
            Signature.getInstance(SIGN_ALGORITHM).apply {
                initSign(entry.privateKey)
            }
        } catch (_: KeyPermanentlyInvalidatedException) {
            deleteKey()
            return BiometricSignResult.KeyInvalidated
        } catch (_: InvalidKeyException) {
            deleteKey()
            return BiometricSignResult.KeyInvalidated
        } catch (e: Exception) {
            return BiometricSignResult.Error(e.message ?: "Could not initialize biometric signature")
        }

        val challenge = ByteArray(32).also { SecureRandom().nextBytes(it) }
        val pubB64 = Base64.getUrlEncoder().withoutPadding().encodeToString(entry.certificate.publicKey.encoded)
        val signRes = promptAndSign(activity, signature, challenge, pubB64, title, subtitle, cancelText)
        if (signRes !is BiometricSignResult.Success) {
            return signRes
        }
        return try {
            val sigBytes = Base64.getUrlDecoder().decode(signRes.signatureB64Url)
            val verifier = Signature.getInstance(SIGN_ALGORITHM).apply {
                initVerify(entry.certificate.publicKey)
                update(challenge)
            }
            if (verifier.verify(sigBytes)) {
                signRes
            } else {
                BiometricSignResult.Error("Hardware biometric verification failed")
            }
        } catch (e: Exception) {
            if (e is GeneralSecurityException || e is ProviderException) {
                BiometricSignResult.Error(e.message ?: "Hardware biometric verification failed")
            } else {
                throw e
            }
        }
    }

    override fun deleteKey() {
        runCatching {
            val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            if (ks.containsAlias(KEY_ALIAS)) {
                ks.deleteEntry(KEY_ALIAS)
            }
        }
    }

    private fun loadPrivateKeyEntry(): KeyStore.PrivateKeyEntry? = runCatching {
        val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val entry = ks.getEntry(KEY_ALIAS, null) as? KeyStore.PrivateKeyEntry
        if (entry != null && entry.privateKey is PrivateKey) entry else null
    }.getOrNull()

    private suspend fun promptAndSign(
        activity: FragmentActivity,
        signature: Signature,
        payload: ByteArray,
        pubB64: String,
        title: String,
        subtitle: String,
        cancelText: String,
    ): BiometricSignResult = withContext(Dispatchers.Main.immediate) {
        suspendCancellableCoroutine { cont ->
            val executor = ContextCompat.getMainExecutor(activity)
            val prompt = BiometricPrompt(
                activity,
                executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        if (!cont.isActive) return
                        try {
                            val unlockedSig = result.cryptoObject?.signature ?: signature
                            unlockedSig.update(payload)
                            val signedBytes = unlockedSig.sign()
                            val sigB64 = Base64.getUrlEncoder().withoutPadding().encodeToString(signedBytes)
                            cont.resume(BiometricSignResult.Success(sigB64, pubB64))
                        } catch (_: KeyPermanentlyInvalidatedException) {
                            deleteKey()
                            cont.resume(BiometricSignResult.KeyInvalidated)
                        } catch (e: Exception) {
                            if (e is CancellationException) throw e
                            cont.resume(BiometricSignResult.Error(e.message ?: "Biometric signing failed"))
                        }
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        if (!cont.isActive) return
                        val mapped = when (errorCode) {
                            BiometricPrompt.ERROR_USER_CANCELED,
                            BiometricPrompt.ERROR_NEGATIVE_BUTTON,
                            BiometricPrompt.ERROR_CANCELED -> BiometricSignResult.Cancelled
                            else -> BiometricSignResult.Error(errString.toString())
                        }
                        cont.resume(mapped)
                    }

                    override fun onAuthenticationFailed() {
                        // Prompt stays visible for user retry; do not complete continuation.
                    }
                },
            )
            val info = BiometricPrompt.PromptInfo.Builder()
                .setTitle(title)
                .apply {
                    if (subtitle.isNotBlank()) setSubtitle(subtitle)
                }
                .setNegativeButtonText(cancelText)
                .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                .build()
            cont.invokeOnCancellation { runCatching { prompt.cancelAuthentication() } }
            try {
                prompt.authenticate(info, BiometricPrompt.CryptoObject(signature))
            } catch (e: Exception) {
                if (e is CancellationException) throw e
                if (cont.isActive) {
                    cont.resume(BiometricSignResult.Error(e.message ?: "Biometric prompt failed"))
                }
            }
        }
    }

    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "gr_biometric_login_ec_p256_v1"
        private const val SIGN_ALGORITHM = "SHA256withECDSA"
    }
}

class BiometricAuthManager(
    private val store: SecureStore,
    val driver: BiometricCryptoDriver = AndroidBiometricCryptoDriver(),
    private val clockMs: () -> Long = { SystemClock.elapsedRealtime() },
) {
    private val storeLock = Any()

    @Volatile
    private var loaded: Boolean = store !is org.pjstatecm.graceride.data.session.EncryptedPrefsStore

    private val _enrollment = MutableStateFlow(if (loaded) loadEnrollmentFromStore() else null)
    val enrollment: StateFlow<BiometricEnrollment?> = _enrollment.asStateFlow()

    private val _appUnlocked = MutableStateFlow(false)
    val appUnlocked: StateFlow<Boolean> = _appUnlocked.asStateFlow()

    private val _pendingOptInUserId = MutableStateFlow<Long?>(null)
    val pendingOptInUserId: StateFlow<Long?> = _pendingOptInUserId.asStateFlow()

    private val _keyInvalidatedNotice = MutableStateFlow(false)
    val keyInvalidatedNotice: StateFlow<Boolean> = _keyInvalidatedNotice.asStateFlow()

    @Volatile
    private var backgroundedAtMs: Long? = null

    @Volatile
    var onRelocked: (() -> Unit)? = null

    fun availability(context: Context?): BiometricAvailability = driver.availability(context)

    fun ensureLoaded() {
        if (loaded) return
        synchronized(storeLock) {
            if (!loaded) {
                _enrollment.value = loadEnrollmentFromStore()
                loaded = true
            }
        }
    }

    fun reloadFromStore() {
        synchronized(storeLock) {
            _enrollment.value = loadEnrollmentFromStore()
            loaded = true
        }
    }

    fun saveEnrollment(enrollment: BiometricEnrollment) {
        synchronized(storeLock) {
            store.putString(ENROLLMENT_KEY, apiJson.encodeToString(BiometricEnrollment.serializer(), enrollment))
            _enrollment.value = enrollment
            loaded = true
            _pendingOptInUserId.value = null
            _keyInvalidatedNotice.value = false
        }
    }

    fun clearEnrollmentLocal() {
        synchronized(storeLock) {
            driver.deleteKey()
            store.remove(ENROLLMENT_KEY)
            _enrollment.value = null
            loaded = true
            _appUnlocked.value = false
        }
    }

    fun handleKeyInvalidated() {
        synchronized(storeLock) {
            driver.deleteKey()
            store.remove(ENROLLMENT_KEY)
            _enrollment.value = null
            loaded = true
            _appUnlocked.value = false
            _keyInvalidatedNotice.value = true
        }
    }

    fun clearKeyInvalidatedNotice() {
        _keyInvalidatedNotice.value = false
    }

    fun clearStaleEnrollmentIfOtherUser(userId: Long) {
        val current = _enrollment.value ?: return
        if (current.userId != userId) {
            clearEnrollmentLocal()
        }
    }

    fun requiresAppGate(userId: Long): Boolean {
        val current = _enrollment.value ?: return false
        return current.userId == userId && !_appUnlocked.value
    }

    fun markAppUnlocked() {
        _appUnlocked.value = true
    }

    fun unlockWithVerifiedResult(result: BiometricSignResult.Success) {
        if (result.signatureB64Url.isNotBlank()) {
            _appUnlocked.value = true
        }
    }

    fun resetAppUnlocked() {
        _appUnlocked.value = false
    }

    fun onAppBackgrounded(nowMs: Long = clockMs()) {
        if (_enrollment.value != null) {
            backgroundedAtMs = nowMs
        }
    }

    fun onAppForegrounded(nowMs: Long = clockMs()) {
        val leftAt = backgroundedAtMs
        backgroundedAtMs = null
        if (leftAt != null && _enrollment.value != null && nowMs - leftAt >= IDLE_RELOCK_MS) {
            _appUnlocked.value = false
            onRelocked?.invoke()
        }
    }

    fun offerOptInIfEligible(userId: Long) {
        if (_enrollment.value?.userId == userId) return
        if (isOptInDismissed(userId)) return
        _pendingOptInUserId.value = userId
    }

    fun shouldOfferOptIn(userId: Long, context: Context?): Boolean {
        if (_enrollment.value?.userId == userId) return false
        if (isOptInDismissed(userId)) return false
        return availability(context) == BiometricAvailability.AVAILABLE
    }

    fun dismissOptIn(userId: Long) {
        store.putString(dismissKey(userId), "1")
        if (_pendingOptInUserId.value == userId) {
            _pendingOptInUserId.value = null
        }
    }

    fun clearPendingOptIn() {
        _pendingOptInUserId.value = null
    }

    fun buildSignedPayload(purpose: String, challenge: String): ByteArray =
        "graceride-biometric-v1:$purpose:$challenge".toByteArray(Charsets.UTF_8)

    private fun isOptInDismissed(userId: Long): Boolean = store.getString(dismissKey(userId)) == "1"

    private fun dismissKey(userId: Long): String = "biometric_opt_in_dismissed_$userId"

    private fun loadEnrollmentFromStore(): BiometricEnrollment? {
        val raw = store.getString(ENROLLMENT_KEY) ?: return null
        return runCatching { apiJson.decodeFromString(BiometricEnrollment.serializer(), raw) }.getOrNull()
    }

    companion object {
        const val ENROLLMENT_KEY = "biometric_enrollment_v1"
        const val IDLE_RELOCK_MS = 120_000L
    }
}
