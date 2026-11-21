package org.pjstatecm.graceride

import okhttp3.HttpUrl.Companion.toHttpUrl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import okhttp3.Cookie

class PersistentCookieJarTest {

    private val url = "https://graceride.example.org/api/auth/me".toHttpUrl()

    private fun cookie(
        name: String = "gr_session",
        value: String = "x".repeat(48),
        domain: String = "graceride.example.org",
        hostOnly: Boolean = true,
        secure: Boolean = true,
        expiresAt: Long = System.currentTimeMillis() + 3600_000,
    ): Cookie = Cookie.Builder()
        .name(name).value(value).domain(domain).path("/")
        .expiresAt(expiresAt)
        .apply {
            if (secure) secure()
            if (hostOnly) hostOnlyDomain(domain)
        }
        .build()

    @Test fun savesAndLoadsSessionCookie() {
        val jar = PersistentCookieJar(InMemoryStore())
        jar.saveFromResponse(url, listOf(cookie()))
        val loaded = jar.loadForRequest(url)
        assertEquals(1, loaded.size)
        assertEquals("gr_session", loaded[0].name)
    }

    @Test fun persistsAcrossInstances() {
        val store = InMemoryStore()
        PersistentCookieJar(store).saveFromResponse(url, listOf(cookie()))
        val restored = PersistentCookieJar(store)
        assertEquals(1, restored.loadForRequest(url).size)
    }

    @Test fun secureCookieNotSentOverHttp() {
        val jar = PersistentCookieJar(InMemoryStore())
        jar.saveFromResponse(url, listOf(cookie(secure = true)))
        val http = "http://graceride.example.org/api/auth/me".toHttpUrl()
        assertEquals(0, jar.loadForRequest(http).size)
    }

    @Test fun hostOnlyCookieNotSentToSubdomain() {
        val jar = PersistentCookieJar(InMemoryStore())
        jar.saveFromResponse(url, listOf(cookie(hostOnly = true)))
        val sub = "https://evil.graceride.example.org/api".toHttpUrl()
        assertEquals(0, jar.loadForRequest(sub).size)
    }

    @Test fun expiredCookiesDropped() {
        val jar = PersistentCookieJar(InMemoryStore())
        jar.saveFromResponse(url, listOf(cookie(expiresAt = System.currentTimeMillis() - 1000)))
        assertEquals(0, jar.loadForRequest(url).size)
    }

    @Test fun replacementUpdatesValue() {
        val jar = PersistentCookieJar(InMemoryStore())
        jar.saveFromResponse(url, listOf(cookie(value = "a".repeat(48))))
        jar.saveFromResponse(url, listOf(cookie(value = "b".repeat(48))))
        val loaded = jar.loadForRequest(url)
        assertEquals(1, loaded.size)
        assertEquals("b".repeat(48), loaded[0].value)
    }

    @Test fun clearRemovesEverything() {
        val store = InMemoryStore()
        val jar = PersistentCookieJar(store)
        jar.saveFromResponse(url, listOf(cookie()))
        jar.clear()
        assertEquals(0, jar.loadForRequest(url).size)
        assertEquals(0, PersistentCookieJar(store).loadForRequest(url).size)
        assertTrue(store.getString("cookies") == null)
    }

    @Test fun devCookieNameAlsoHandled() {
        val jar = PersistentCookieJar(InMemoryStore())
        val devUrl = "http://10.0.2.2:8080/api/auth/me".toHttpUrl()
        jar.saveFromResponse(devUrl, listOf(cookie(name = "gr_session", domain = "10.0.2.2", secure = false)))
        assertEquals(1, jar.loadForRequest(devUrl).size)
    }
}
