package org.pjstatecm.graceride.data.cache

import android.content.Context
import kotlinx.serialization.KSerializer
import kotlinx.serialization.Serializable
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.session.AesGcmCodec
import org.pjstatecm.graceride.data.session.EncryptedPrefsStore
import java.io.File
import java.util.concurrent.ConcurrentHashMap
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey

@Serializable
data class CachedEntry<T>(
    val savedAtMillis: Long,
    val data: T,
)

/**
 * Per-user encrypted atomic JSON offline cache backed by `filesDir/offline_cache` on Android
 * using an AndroidKeyStore AES-256-GCM key with filename AAD binding (or an in-process AES-256
 * key in host JVM unit tests when `context == null`).
 */
class OfflineCache(
    private val cacheDir: File?,
    private val context: Context? = null,
    private val testKeyOverride: SecretKey? = null,
) {
    constructor(context: Context?) : this(
        cacheDir = context?.applicationContext?.filesDir?.let { File(it, "offline_cache") },
        context = context?.applicationContext,
    )

    private val memory = ConcurrentHashMap<String, ByteArray>()
    private val fileLock = Any()
    private val keyLock = Any()
    @Volatile
    private var secretKey: SecretKey? = null

    private fun getOrCreateKey(): SecretKey {
        testKeyOverride?.let { return it }
        secretKey?.let { return it }
        return synchronized(keyLock) {
            secretKey?.let { return@synchronized it }
            val resolved = if (context != null) {
                EncryptedPrefsStore.loadAes256GcmKey(KEY_ALIAS)
                    ?: EncryptedPrefsStore.generateAes256GcmKey(KEY_ALIAS)
            } else {
                jvmTestFallbackKey
            }
            secretKey = resolved
            resolved
        }
    }

    private fun seal(filename: String, payloadUtf8: ByteArray): ByteArray {
        val packed = AesGcmCodec.encryptRawBytes(getOrCreateKey(), filename, payloadUtf8)
        return ByteArray(MAGIC.size + packed.size).apply {
            System.arraycopy(MAGIC, 0, this, 0, MAGIC.size)
            System.arraycopy(packed, 0, this, MAGIC.size, packed.size)
        }
    }

    private fun unseal(filename: String, blob: ByteArray): ByteArray {
        if (blob.size <= MAGIC.size || !blob.copyOfRange(0, MAGIC.size).contentEquals(MAGIC)) {
            throw java.security.GeneralSecurityException("Unrecognized offline cache ciphertext header")
        }
        val packed = blob.copyOfRange(MAGIC.size, blob.size)
        return AesGcmCodec.decryptRawBytes(getOrCreateKey(), filename, packed)
    }

    fun <T> write(
        userId: Long,
        key: String,
        value: T,
        serializer: KSerializer<T>,
        nowMillis: Long = System.currentTimeMillis(),
    ) {
        val filename = entryKey(userId, key)
        val payload = apiJson.encodeToString(CachedEntry.serializer(serializer), CachedEntry(nowMillis, value))
        val sealed = seal(filename, payload.toByteArray(Charsets.UTF_8))
        memory[filename] = sealed
        val dir = cacheDir ?: return
        synchronized(fileLock) {
            runCatching {
                if (!dir.exists()) dir.mkdirs()
                File(dir, "$filename.json").takeIf { it.exists() }?.delete()
                val tmp = File(dir, "$filename.tmp")
                val dst = File(dir, "$filename.bin")
                tmp.writeBytes(sealed)
                if (!tmp.renameTo(dst)) {
                    dst.writeBytes(sealed)
                    tmp.delete()
                }
            }
        }
    }

    fun <T> read(
        userId: Long,
        key: String,
        serializer: KSerializer<T>,
    ): CachedEntry<T>? {
        val filename = entryKey(userId, key)
        val sealed = memory[filename] ?: cacheDir?.let { dir ->
            synchronized(fileLock) {
                File(dir, "$filename.json").takeIf { it.exists() }?.delete()
                val f = File(dir, "$filename.bin")
                if (f.isFile) runCatching { f.readBytes() }.getOrNull()?.also {
                    memory[filename] = it
                } else {
                    null
                }
            }
        } ?: return null

        val plain = runCatching {
            String(unseal(filename, sealed), Charsets.UTF_8)
        }.getOrElse {
            memory.remove(filename)
            cacheDir?.let { dir ->
                synchronized(fileLock) {
                    runCatching { File(dir, "$filename.bin").delete() }
                }
            }
            return null
        }

        return runCatching {
            apiJson.decodeFromString(CachedEntry.serializer(serializer), plain)
        }.getOrNull()
    }

    fun clearUser(userId: Long) {
        val prefix = "u${userId}_"
        memory.keys.filter { it.startsWith(prefix) }.forEach { memory.remove(it) }
        val dir = cacheDir ?: return
        synchronized(fileLock) {
            dir.listFiles { f -> f.name.startsWith(prefix) }?.forEach { it.delete() }
        }
    }

    fun clearAll() {
        memory.clear()
        val dir = cacheDir ?: return
        synchronized(fileLock) {
            dir.listFiles()?.forEach { it.delete() }
        }
    }

    private fun entryKey(userId: Long, key: String): String {
        val clean = key.replace(Regex("[^A-Za-z0-9_-]"), "_")
        return "u${userId}_$clean"
    }

    companion object {
        private const val KEY_ALIAS = "gr_offline_cache_aes256_v1"
        private val MAGIC = byteArrayOf('G'.code.toByte(), 'R'.code.toByte(), 'C'.code.toByte(), 1)
        private val jvmTestFallbackKey: SecretKey by lazy {
            KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        }
    }
}
