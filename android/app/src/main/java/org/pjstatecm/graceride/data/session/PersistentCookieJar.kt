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
    private var cache: MutableList<StoredCookie> = load()

    private fun load(): MutableList<StoredCookie> {
        val raw = store.getString(key) ?: return mutableListOf()
        return try {
            json.decodeFromString(ListSerializer(StoredCookie.serializer()), raw).toMutableList()
        } catch (_: Exception) {
            mutableListOf()
        }
    }

    private fun persist() {
        store.putString(key, json.encodeToString(ListSerializer(StoredCookie.serializer()), cache))
    }

    override fun saveFromResponse(url: HttpUrl, cookies: List<Cookie>) {
        synchronized(lock) {
            val now = System.currentTimeMillis()
            cache.removeAll { it.expiresAt <= now }
            for (c in cookies) {
                cache.removeAll { it.name == c.name && it.domain == c.domain && it.path == c.path }
                if (c.expiresAt > now) {
                    cache.add(
                        StoredCookie(
                            c.name, c.value, c.domain, c.path, c.expiresAt,
                            c.secure, c.httpOnly, c.hostOnly,
                        )
                    )
                }
            }
            persist()
        }
    }

    override fun loadForRequest(url: HttpUrl): List<Cookie> {
        synchronized(lock) {
            val now = System.currentTimeMillis()
            val matches = cache.filter { c ->
                c.expiresAt > now &&
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

    fun clear() {
        synchronized(lock) {
            cache.clear()
            store.remove(key)
        }
    }

    companion object {
        /** RFC 6265 path-match: "/api" must not match "/apix". */
        fun pathMatch(requestPath: String, cookiePath: String): Boolean {
            if (requestPath == cookiePath) return true
            if (!requestPath.startsWith(cookiePath)) return false
            if (cookiePath.endsWith("/")) return true
            return requestPath.length > cookiePath.length && requestPath[cookiePath.length] == '/'
        }
    }
}
