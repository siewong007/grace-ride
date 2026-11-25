package org.pjstatecm.graceride.data.session

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.edit
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKeys
import java.security.KeyStore

/** Small key-value store. The real one is Keystore-backed; tests use an in-memory map. */
interface SecureStore {
    fun getString(key: String): String?
    fun putString(key: String, value: String)
    fun remove(key: String)
    fun clear()
}

class EncryptedPrefsStore(context: Context) : SecureStore {
    private val prefs: SharedPreferences = openOrRecover(context.applicationContext)

    override fun getString(key: String): String? = prefs.getString(key, null)
    override fun putString(key: String, value: String) { prefs.edit { putString(key, value) } }
    override fun remove(key: String) { prefs.edit { remove(key) } }
    override fun clear() { prefs.edit { clear() } }

    companion object {
        private const val PREFS_NAME = "gr_secure_store"

        private fun createPrefs(context: Context): SharedPreferences =
            EncryptedSharedPreferences.create(
                PREFS_NAME,
                MasterKeys.getOrCreate(MasterKeys.AES256_GCM_SPEC),
                context,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )

        private fun openOrRecover(context: Context): SharedPreferences =
            try {
                createPrefs(context)
            } catch (_: Exception) {
                // Corrupted Keystore entry or restored ciphertexts: wipe local encrypted
                // store so the member gets a clean signed-out state instead of a crash loop.
                runCatching { context.deleteSharedPreferences(PREFS_NAME) }
                runCatching {
                    val ks = KeyStore.getInstance("AndroidKeyStore")
                    ks.load(null)
                    ks.deleteEntry(MasterKeys.AES256_GCM_SPEC.keystoreAlias)
                }
                createPrefs(context)
            }
    }
}

class InMemoryStore : SecureStore {
    private val map = mutableMapOf<String, String>()
    override fun getString(key: String): String? = map[key]
    override fun putString(key: String, value: String) { map[key] = value }
    override fun remove(key: String) { map.remove(key) }
    override fun clear() { map.clear() }
}
