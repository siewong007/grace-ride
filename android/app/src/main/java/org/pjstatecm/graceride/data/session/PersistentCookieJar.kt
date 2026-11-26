package org.pjstatecm.graceride.data.session

import kotlinx.serialization.Serializable
import kotlinx.serialization.builtins.ListSerializer
import kotlinx.serialization.json.Json
import okhttp3.Cookie
import okhttp3.CookieJar
import okhttp3.HttpUrl

/**
 * Persists the HttpOnly session cookie (gr_session / __Host-gr_session) in encrypted
 * storage. The token is a credential: never logged, cleared on sign-out.
 */
class PersistentCookieJar(
    private val store: SecureStore,
    private val json: Json = Json { ignoreUnknownKeys = true },
) : CookieJar {

    @Serializable
    data class StoredCookie(
        val name: String,
        val value: String,
        val domain: String,
        val path: String,
        val expiresAt: Long,
        val secure: Boolean,
        val httpOnly: Boolean,
        val hostOnly: Boolean,
    )

    private val key = "cookies"
    private val lock = Any()
    private var cache: MutableList<StoredCookie>? = null

    private fun ensureLoadedLocked(): MutableList<StoredCookie> {
        cache?.let { return it }
        return load().also { cache = it }
    }

    private fun load(): MutableList<StoredCookie> {
        val raw = store.getString(key) ?: return mutableListOf()
        return try {
            json.decodeFromString(ListSerializer(StoredCookie.serializer()), raw).toMutableList()
        } catch (_: Exception) {
            mutableListOf()
        }
    }

    private fun persist(list: List<StoredCookie>) {
        store.putString(key, json.encodeToString(ListSerializer(StoredCookie.serializer()), list))
    }

    override fun saveFromResponse(url: HttpUrl, cookies: List<Cookie>) {
        synchronized(lock) {
            val list = ensureLoadedLocked()
            val now = System.currentTimeMillis()
            list.removeAll { it.expiresAt <= now }
            for (c in cookies) {
                if (!isValidPrefixPolicy(c.name, c.path, c.secure, c.hostOnly, url.isHttps)) {
                    continue
                }
                list.removeAll { it.name == c.name && it.domain == c.domain && it.path == c.path }
                if (c.expiresAt > now) {
                    list.add(
                        StoredCookie(
                            c.name, c.value, c.domain, c.path, c.expiresAt,
                            c.secure, c.httpOnly, c.hostOnly,
                        )
                    )
                }
            }
            persist(list)
        }
    }

    override fun loadForRequest(url: HttpUrl): List<Cookie> {
        synchronized(lock) {
            val list = ensureLoadedLocked()
            val now = System.currentTimeMillis()
            val matches = list.filter { c ->
                c.expiresAt > now &&
                    isValidPrefixPolicy(c.name, c.path, c.secure, c.hostOnly, url.isHttps) &&
                    (if (c.hostOnly) url.host == c.domain else url.host == c.domain || url.host.endsWith(".${c.domain}")) &&
                    pathMatch(url.encodedPath, c.path) &&
                    (!c.secure || url.scheme == "https")
            }
            return matches.map { c ->
                Cookie.Builder()
                    .name(c.name).value(c.value)
                    .domain(c.domain)
                    .path(c.path)
                    .expiresAt(c.expiresAt)
                    .apply {
                        if (c.secure) secure()
                        if (c.httpOnly) httpOnly()
                        if (c.hostOnly) hostOnlyDomain(c.domain)
                    }
                    .build()
            }
        }
    }

    fun hasCookies(): Boolean = synchronized(lock) {
        val list = ensureLoadedLocked()
        val now = System.currentTimeMillis()
        list.any { it.expiresAt > now }
    }

    fun clear() {
        synchronized(lock) {
            cache = mutableListOf()
            store.remove(key)
        }
    }

    companion object {
        /** Enforce RFC 6265bis `__Host-` and `__Secure-` cookie prefix requirements. */
        fun isValidPrefixPolicy(
            name: String,
            path: String,
            secure: Boolean,
            hostOnly: Boolean,
            isHttps: Boolean,
        ): Boolean = when {
            name.startsWith("__Host-") -> isHttps && secure && hostOnly && path == "/"
            name.startsWith("__Secure-") -> isHttps && secure
            else -> true
        }

        /** RFC 6265 path-match: "/api" must not match "/apix". */
        fun pathMatch(requestPath: String, cookiePath: String): Boolean {
            if (requestPath == cookiePath) return true
            if (!requestPath.startsWith(cookiePath)) return false
            if (cookiePath.endsWith("/")) return true
            return requestPath.length > cookiePath.length && requestPath[cookiePath.length] == '/'
        }
    }
}
