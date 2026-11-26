package org.pjstatecm.graceride.data.session

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.core.content.edit
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.security.ProviderException
import java.util.Base64
import java.util.concurrent.ConcurrentHashMap
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Small key-value store. The real one is AndroidKeyStore-backed; tests use an in-memory map. */
interface SecureStore {
    fun getString(key: String): String?
    fun putString(key: String, value: String)
    fun remove(key: String)
    fun clear()
}

/**
 * Pure JCA AES-256-GCM codec with AAD key-binding (pure JVM/Android, no Tink/security-crypto).
 * Uses `java.util.Base64` (API 26+) so unit tests can exercise it on the JVM without Android stubs.
 */
internal object AesGcmCodec {
    private const val PREFIX = "v2:"
    private const val TRANSFORMATION = "AES/GCM/NoPadding"
    private const val IV_LEN = 12
    private const val TAG_BITS = 128

    fun encryptRawBytes(key: SecretKey, aad: String, plaintext: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        cipher.updateAAD(aad.toByteArray(Charsets.UTF_8))
        val iv = cipher.iv
        require(iv.size == IV_LEN) { "Unexpected GCM IV length: ${iv.size}" }
        val ciphertext = cipher.doFinal(plaintext)
        val packed = ByteArray(iv.size + ciphertext.size)
        System.arraycopy(iv, 0, packed, 0, iv.size)
        System.arraycopy(ciphertext, 0, packed, iv.size, ciphertext.size)
        return packed
    }

    fun decryptRawBytes(key: SecretKey, aad: String, packed: ByteArray): ByteArray {
        if (packed.size <= IV_LEN) {
            throw GeneralSecurityException("Truncated ciphertext")
        }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, packed, 0, IV_LEN))
        cipher.updateAAD(aad.toByteArray(Charsets.UTF_8))
        return cipher.doFinal(packed, IV_LEN, packed.size - IV_LEN)
    }

    fun encrypt(key: SecretKey, aad: String, plaintext: String): String {
        val packed = encryptRawBytes(key, aad, plaintext.toByteArray(Charsets.UTF_8))
        return PREFIX + Base64.getEncoder().encodeToString(packed)
    }

    fun decrypt(key: SecretKey, aad: String, encoded: String): String {
        if (!encoded.startsWith(PREFIX)) {
            throw GeneralSecurityException("Unsupported ciphertext version")
        }
        val packed = try {
            Base64.getDecoder().decode(encoded.substring(PREFIX.length))
        } catch (e: IllegalArgumentException) {
            throw GeneralSecurityException("Invalid ciphertext encoding", e)
        }
        return String(decryptRawBytes(key, aad, packed), Charsets.UTF_8)
    }
}

/**
 * Hardware/OS-backed encrypted key-value store using native `AndroidKeyStore` AES-256-GCM
 * and standard `SharedPreferences`, replacing deprecated `androidx.security:security-crypto`.
 */
class EncryptedPrefsStore(context: Context) : SecureStore {
    private val appContext = context.applicationContext
    private val lock = Any()
    private val cache = ConcurrentHashMap<String, Any>()

    @Volatile
    private var warmed = false

    // Resettable reference so clearing a corrupted store discards the in-memory SharedPreferencesImpl
    // map before deleting the XML file (preventing undecryptable ciphertexts from flushing back).
    @Volatile
    private var prefsRef: SharedPreferences? = null
    @Volatile
    private var legacyCleaned = false

    private fun getPrefs(): SharedPreferences {
        prefsRef?.let { return it }
        if (!legacyCleaned) {
            legacyCleaned = true
            runCatching { appContext.deleteSharedPreferences(LEGACY_PREFS_NAME) }
        }
        return appContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).also { prefsRef = it }
    }

    @Volatile
    private var secretKey: SecretKey? = null

    private fun getOrCreateKey(): SecretKey {
        secretKey?.let { return it }
        synchronized(lock) {
            secretKey?.let { return it }
            val existing = loadKeyFromStore()
            if (existing != null) {
                secretKey = existing
                return existing
            }
            val created = generateKeyInStore()
            secretKey = created
            return created
        }
    }

    private fun resetCorruptedStore() {
        cache.clear()
        secretKey = null
        runCatching { prefsRef?.edit(commit = true) { clear() } }
        prefsRef = null
        runCatching { appContext.deleteSharedPreferences(PREFS_NAME) }
        runCatching {
            val ks = KeyStore.getInstance(ANDROID_KEYSTORE)
            ks.load(null)
            ks.deleteEntry(KEY_ALIAS)
        }
    }

    /** Pre-initializes Keystore key + prefs and hydrates the read cache; safe to call on Dispatchers.IO. */
    fun warmUp() {
        if (warmed) return
        synchronized(lock) {
            if (warmed) return
            val key = runCatching { getOrCreateKey() }.getOrElse {
                resetCorruptedStore()
                getOrCreateKey()
            }
            val entries = getPrefs().all
            var hadFailure = false
            for ((k, v) in entries) {
                if (v is String) {
                    try {
                        cache.putIfAbsent(k, AesGcmCodec.decrypt(key, k, v))
                    } catch (e: Exception) {
                        if (e is GeneralSecurityException || e is ProviderException) {
                            hadFailure = true
                            break
                        }
                        throw e
                    }
                }
            }
            if (hadFailure) {
                resetCorruptedStore()
                getOrCreateKey()
            }
            warmed = true
        }
    }

    override fun getString(key: String): String? {
        cache[key]?.let { cached ->
            return if (cached === NULL_SENTINEL) null else cached as String
        }
        return synchronized(lock) {
            cache[key]?.let { cached ->
                return if (cached === NULL_SENTINEL) null else cached as String
            }
            val encoded = getPrefs().getString(key, null)
            if (encoded == null) {
                cache[key] = NULL_SENTINEL
                return null
            }
            val value = try {
                AesGcmCodec.decrypt(getOrCreateKey(), key, encoded)
            } catch (e: Exception) {
                if (e is GeneralSecurityException || e is ProviderException) {
                    // Corrupted Keystore entry or restored ciphertexts: wipe local encrypted
                    // store so the member gets a clean signed-out state instead of a crash loop.
                    resetCorruptedStore()
                    null
                } else {
                    throw e
                }
            }
            cache[key] = value ?: NULL_SENTINEL
            value
        }
    }

    override fun putString(key: String, value: String) {
        synchronized(lock) {
            val encoded = try {
                AesGcmCodec.encrypt(getOrCreateKey(), key, value)
            } catch (e: Exception) {
                if (e is GeneralSecurityException || e is ProviderException) {
                    resetCorruptedStore()
                    AesGcmCodec.encrypt(getOrCreateKey(), key, value)
                } else {
                    throw e
                }
            }
            cache[key] = value
            getPrefs().edit { putString(key, encoded) }
        }
    }

    override fun remove(key: String) {
        synchronized(lock) {
            cache[key] = NULL_SENTINEL
            getPrefs().edit { remove(key) }
        }
    }

    override fun clear() {
        synchronized(lock) {
            cache.clear()
            warmed = true
            getPrefs().edit { clear() }
        }
    }

    companion object {
        private const val LEGACY_PREFS_NAME = "gr_secure_store"
        private const val PREFS_NAME = "gr_secure_store_v2"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val KEY_ALIAS = "gr_secure_store_aes256_gcm_v2"
        private val NULL_SENTINEL = Any()

        internal fun loadAes256GcmKey(alias: String): SecretKey? {
            val ks = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            return (ks.getEntry(alias, null) as? KeyStore.SecretKeyEntry)?.secretKey
        }

        internal fun generateAes256GcmKey(alias: String): SecretKey {
            val kg = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
            fun buildSpec(strongBox: Boolean): KeyGenParameterSpec = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .setRandomizedEncryptionRequired(true)
                .apply {
                    if (strongBox) {
                        setIsStrongBoxBacked(true)
                    }
                }
                .build()

            return try {
                kg.init(buildSpec(strongBox = true))
                kg.generateKey()
            } catch (e: Exception) {
                if (e is GeneralSecurityException || e is ProviderException) {
                    kg.init(buildSpec(strongBox = false))
                    kg.generateKey()
                } else {
                    throw e
                }
            }
        }

        private fun loadKeyFromStore(): SecretKey? = loadAes256GcmKey(KEY_ALIAS)

        private fun generateKeyInStore(): SecretKey = generateAes256GcmKey(KEY_ALIAS)
    }
}

class InMemoryStore : SecureStore {
    private val map = ConcurrentHashMap<String, String>()
    override fun getString(key: String): String? = map[key]
    override fun putString(key: String, value: String) { map[key] = value }
    override fun remove(key: String) { map.remove(key) }
    override fun clear() { map.clear() }
}
