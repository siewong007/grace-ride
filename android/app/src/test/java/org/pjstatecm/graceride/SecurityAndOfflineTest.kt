package org.pjstatecm.graceride

import android.content.ContextWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.data.repo.AuthState
import org.pjstatecm.graceride.data.session.AesGcmCodec
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.notifications.NotificationHelper
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.BiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.ui.screens.login.LoginViewModel
import java.security.GeneralSecurityException
import java.security.KeyFactory
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import java.security.spec.X509EncodedKeySpec
import java.util.Base64
import javax.crypto.KeyGenerator

@OptIn(ExperimentalCoroutinesApi::class)
class SecurityAndOfflineTest {

    private lateinit var server: MockWebServer
    private lateinit var store: InMemoryStore
    private lateinit var container: AppContainer

    @Before fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        store = InMemoryStore()
        container = AppContainer(null, store)
        container.setBaseUrl(server.url("/").toString())
    }

    @After fun teardown() {
        runCatching { server.shutdown() }
        Dispatchers.resetMain()
    }

    @Test fun authRepositoryRestoresCachedMeOnOfflineColdStartWhenCookiesPresent() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok123; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        assertTrue(container.auth.state.value is AuthState.SignedIn)
        assertTrue(container.cookieJar.hasCookies())
        assertNotNull(store.getString("cached_me_v1"))

        val offlineContainer = AppContainer(null, store)
        offlineContainer.setBaseUrl(server.url("/").toString())
        server.shutdown() // force network IOException (status == 0)

        offlineContainer.auth.refresh()
        val restored = offlineContainer.auth.state.value as? AuthState.SignedIn
        assertNotNull("Expected cached SignedIn state during offline cold start", restored)
        assertEquals("Test Member", restored!!.me.name)
    }

    @Test fun authRepositoryUnauthorizedAndLogoutClearAllLocalArtifacts() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok123; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        val today = KlTime.today()
        val rideJson = """{"id":10,"status":"confirmed","pickup_at":"$today 08:30:00",
            "area":{"id":2,"name":"PJ Section 14"},"destination":"Church Hall","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        OfflineSnapshotStore.save(store, 1L, listOf(apiJson.decodeFromString(Ride.serializer(), rideJson)))
        store.putString("notif_hide_lock_details", "1")

        var clearedCount = 0
        container.onSessionCleared = { clearedCount++ }

        assertNotNull(OfflineSnapshotStore.load(store, 1L))
        assertTrue(container.cookieJar.hasCookies())

        // Simulate 401 on refresh()
        server.enqueue(MockResponse().setResponseCode(401).setBody("""{"error":"expired"}"""))
        container.auth.refresh()

        assertEquals(AuthState.SignedOut, container.auth.state.value)
        assertFalse(container.cookieJar.hasCookies())
        assertNull(store.getString("cached_me_v1"))
        assertNull(store.getString("notif_hide_lock_details"))
        assertNull(OfflineSnapshotStore.load(store, 1L))
        assertEquals(1, clearedCount)

        // Simulate logout() cleanup even if server fails
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok456; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        store.putString("notif_hide_lock_details", "1")
        OfflineSnapshotStore.save(store, 1L, listOf(apiJson.decodeFromString(Ride.serializer(), rideJson)))
        server.enqueue(MockResponse().setResponseCode(500).setBody("""{"error":"down"}"""))
        container.auth.logout()

        assertEquals(AuthState.SignedOut, container.auth.state.value)
        assertFalse(container.cookieJar.hasCookies())
        assertNull(store.getString("cached_me_v1"))
        assertNull(store.getString("notif_hide_lock_details"))
        assertNull(OfflineSnapshotStore.load(store, 1L))
        assertEquals(2, clearedCount)
    }

    @Test fun loginViewModelValidatesEmailAndServerUrl() {
        store.putString("last_email", "saved@example.org")
        val vm = LoginViewModel(container, devAvailable = false)
        assertEquals("saved@example.org", vm.state.value.email)

        vm.applyServer("not-a-valid-url")
        assertEquals(R.string.login_invalid_server, vm.state.value.errorRes)

        val dummyCtx = ContextWrapper(null)
        vm.setEmail("   ")
        vm.signInWithPasskey(dummyCtx)
        assertEquals(R.string.login_email_required, vm.state.value.errorRes)

        vm.setEmail("missing-at-symbol")
        vm.signInWithPasskey(dummyCtx)
        assertEquals(R.string.login_email_required, vm.state.value.errorRes)
    }

    @Test fun notificationHelperRoutesChannelsByPriorityAndType() {
        assertEquals(NotificationHelper.CHANNEL_RIDES_URGENT, NotificationHelper.channelFor("cost", "other", "urgent"))
        assertEquals(NotificationHelper.CHANNEL_CHAT, NotificationHelper.channelFor("ride", "trip_chat", "normal"))
        assertEquals(NotificationHelper.CHANNEL_RIDES_UPDATES, NotificationHelper.channelFor("ride", "assigned", "normal"))
        assertEquals(NotificationHelper.CHANNEL_COMMUNITY, NotificationHelper.channelFor("announcement", "news", "low"))
    }

    @Test fun aesGcmCodecEncryptsWithRandomIvAndBindsKeyAad() {
        val kg = KeyGenerator.getInstance("AES")
        kg.init(256)
        val key = kg.generateKey()

        val c1 = AesGcmCodec.encrypt(key, "gr_cookies_v1", "secret-session-cookie")
        val c2 = AesGcmCodec.encrypt(key, "gr_cookies_v1", "secret-session-cookie")
        assertTrue(c1.startsWith("v2:"))
        assertTrue(c1 != c2) // randomized IV per encryption
        assertEquals("secret-session-cookie", AesGcmCodec.decrypt(key, "gr_cookies_v1", c1))

        // Replaying ciphertext under another preference key must fail AAD check.
        val wrongKeyRejected = runCatching {
            AesGcmCodec.decrypt(key, "other_key", c1)
        }.exceptionOrNull()
        assertTrue(wrongKeyRejected is GeneralSecurityException)

        // Tampering with ciphertext must fail GCM authentication tag.
        val tampered = c1.dropLast(2) + if (c1.endsWith("==")) "AA" else "=="
        val tamperedRejected = runCatching {
            AesGcmCodec.decrypt(key, "gr_cookies_v1", tampered)
        }.exceptionOrNull()
        assertTrue(tamperedRejected is GeneralSecurityException)
    }

    @Test fun biometricEnrollmentDirectLoginAppGateAndInvalidationFlow() = runBlocking {
        val driver = FakeBiometricCryptoDriver()
        val bioContainer = AppContainer(null, store, driver)
        bioContainer.setBaseUrl(server.url("/").toString())

        val ctx = ContextWrapper(null)
        assertEquals(BiometricAvailability.AVAILABLE, bioContainer.biometric.availability(ctx))

        // 1. Initial interactive login offers opt-in and persists dismissal.
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok-init; Path=/; Max-Age=3600; HttpOnly")
                .setBody("""{"ok":true}""")
        )
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        bioContainer.auth.devLogin("member@example.org")
        assertEquals(1L, bioContainer.biometric.pendingOptInUserId.value)
        assertTrue(bioContainer.biometric.shouldOfferOptIn(1L, ctx))
        bioContainer.biometric.dismissOptIn(1L)
        assertNull(bioContainer.biometric.pendingOptInUserId.value)
        assertFalse(bioContainer.biometric.shouldOfferOptIn(1L, ctx))

        // 2. Enroll hardware EC P-256 biometric credential.
        server.enqueue(MockResponse().setBody("""{"challenge_id":"reg-cid-1","challenge":"reg-chal-1"}"""))
        server.enqueue(
            MockResponse().setBody(
                """{"ok":true,"id":42,"credential_id":"bio-cred-0123456789abcdef","label":"Pixel Test"}"""
            )
        )
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        val enrollRes = bioContainer.auth.enrollBiometric(
            context = ctx,
            title = "Enable",
            subtitle = "Verify",
            cancelText = "Cancel",
            label = "Pixel Test",
        )
        assertTrue(enrollRes is BiometricSignResult.Success)
        val regFinishReq = server.takeRequest().let {
            assertEquals("/api/auth/dev-login", it.path)
            server.takeRequest() // /api/auth/me
            server.takeRequest() // /api/auth/biometric/register/start
            server.takeRequest() // /api/auth/biometric/register/finish
        }
        val regBody = apiJson.parseToJsonElement(regFinishReq.body.readUtf8()) as kotlinx.serialization.json.JsonObject
        val sentPub = (regBody["public_key"] as kotlinx.serialization.json.JsonPrimitive).content
        val sentSig = (regBody["signature"] as kotlinx.serialization.json.JsonPrimitive).content
        assertTrue(verifyEcSignature(sentPub, "graceride-biometric-v1:register:reg-chal-1", sentSig))
        server.takeRequest() // /api/auth/me refresh after enroll
        assertEquals("bio-cred-0123456789abcdef", bioContainer.biometric.enrollment.value?.credentialId)
        assertTrue(bioContainer.biometric.appUnlocked.value)

        // 3. Logout clears session cookies but keeps biometric enrollment on device.
        server.enqueue(MockResponse().setBody("""{"ok":true}"""))
        bioContainer.auth.logout()
        assertEquals(AuthState.SignedOut, bioContainer.auth.state.value)
        assertFalse(bioContainer.cookieJar.hasCookies())
        assertFalse(bioContainer.biometric.appUnlocked.value)
        assertEquals("bio-cred-0123456789abcdef", bioContainer.biometric.enrollment.value?.credentialId)
        server.takeRequest() // /api/auth/logout

        // 4. Direct biometric login works without email input.
        server.enqueue(MockResponse().setBody("""{"challenge_id":"log-cid-1","challenge":"log-chal-1"}"""))
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok-bio; Path=/; Max-Age=3600; HttpOnly")
                .setBody("""{"ok":true}""")
        )
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        val loginRes = bioContainer.auth.loginWithBiometric(
            context = ctx,
            title = "Sign in",
            subtitle = "Verify",
            cancelText = "Cancel",
        )
        assertTrue(loginRes is BiometricSignResult.Success)
        assertTrue(bioContainer.auth.state.value is AuthState.SignedIn)
        assertTrue(bioContainer.biometric.appUnlocked.value)
        val loginStartReq = server.takeRequest()
        assertEquals("/api/auth/biometric/login/start", loginStartReq.path)
        assertTrue(loginStartReq.body.readUtf8().contains("bio-cred-0123456789abcdef"))
        val loginFinishReq = server.takeRequest()
        val loginBody = apiJson.parseToJsonElement(loginFinishReq.body.readUtf8()) as kotlinx.serialization.json.JsonObject
        val loginSig = (loginBody["signature"] as kotlinx.serialization.json.JsonPrimitive).content
        assertTrue(verifyEcSignature(sentPub, "graceride-biometric-v1:login:log-chal-1", loginSig))
        server.takeRequest() // /api/auth/me

        // 5. Cold start with existing enrollment + cookie requires app-launch biometric gate.
        val coldContainer = AppContainer(null, store, driver)
        coldContainer.setBaseUrl(server.url("/").toString())
        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        coldContainer.auth.refresh()
        assertTrue(coldContainer.auth.state.value is AuthState.SignedIn)
        assertTrue(coldContainer.biometric.requiresAppGate(1L))
        val localRes = driver.verifyLocalUnlock(ctx, "Unlock", "Verify", "Cancel")
        assertTrue(localRes is BiometricSignResult.Success)
        coldContainer.biometric.markAppUnlocked()
        assertFalse(coldContainer.biometric.requiresAppGate(1L))

        // Short background switch stays unlocked; >= 2 min re-engages app gate.
        coldContainer.biometric.onAppBackgrounded(1_000L)
        coldContainer.biometric.onAppForegrounded(30_000L)
        assertFalse(coldContainer.biometric.requiresAppGate(1L))
        coldContainer.biometric.onAppBackgrounded(30_000L)
        coldContainer.biometric.onAppForegrounded(155_000L)
        assertTrue(coldContainer.biometric.requiresAppGate(1L))

        // 6. OS biometric change invalidates hardware key and wipes local enrollment.
        driver.nextSignInvalidated = true
        server.enqueue(MockResponse().setBody("""{"challenge_id":"log-cid-2","challenge":"log-chal-2"}"""))
        val invRes = coldContainer.auth.loginWithBiometric(
            context = ctx,
            title = "Sign in",
            subtitle = "Verify",
            cancelText = "Cancel",
        )
        assertEquals(BiometricSignResult.KeyInvalidated, invRes)
        assertNull(coldContainer.biometric.enrollment.value)
        assertTrue(coldContainer.biometric.keyInvalidatedNotice.value)
        assertTrue(driver.deleted)
    }

    private fun verifyEcSignature(pubB64Url: String, message: String, sigB64Url: String): Boolean {
        val pubBytes = Base64.getUrlDecoder().decode(pubB64Url)
        val pubKey = KeyFactory.getInstance("EC").generatePublic(X509EncodedKeySpec(pubBytes))
        val sigBytes = Base64.getUrlDecoder().decode(sigB64Url)
        return Signature.getInstance("SHA256withECDSA").run {
            initVerify(pubKey)
            update(message.toByteArray(Charsets.UTF_8))
            verify(sigBytes)
        }
    }

    private class FakeBiometricCryptoDriver : BiometricCryptoDriver {
        var keyPair: KeyPair? = null
        var nextSignInvalidated: Boolean = false
        var deleted: Boolean = false

        override fun availability(context: android.content.Context?): BiometricAvailability =
            BiometricAvailability.AVAILABLE

        override suspend fun generateAndSign(
            context: android.content.Context,
            payload: ByteArray,
            title: String,
            subtitle: String,
            cancelText: String,
            attestationChallenge: ByteArray?,
        ): BiometricSignResult {
            val kpg = KeyPairGenerator.getInstance("EC")
            kpg.initialize(ECGenParameterSpec("secp256r1"))
            val kp = kpg.generateKeyPair()
            keyPair = kp
            deleted = false
            val sig = Signature.getInstance("SHA256withECDSA").apply {
                initSign(kp.private)
                update(payload)
            }.sign()
            return BiometricSignResult.Success(
                signatureB64Url = Base64.getUrlEncoder().withoutPadding().encodeToString(sig),
                publicKeyB64Url = Base64.getUrlEncoder().withoutPadding().encodeToString(kp.public.encoded),
            )
        }

        override suspend fun signWithEnrolledKey(
            context: android.content.Context,
            payload: ByteArray,
            title: String,
            subtitle: String,
            cancelText: String,
        ): BiometricSignResult {
            if (nextSignInvalidated) {
                nextSignInvalidated = false
                deleteKey()
                return BiometricSignResult.KeyInvalidated
            }
            val kp = keyPair ?: return BiometricSignResult.KeyInvalidated
            val sig = Signature.getInstance("SHA256withECDSA").apply {
                initSign(kp.private)
                update(payload)
            }.sign()
            return BiometricSignResult.Success(
                signatureB64Url = Base64.getUrlEncoder().withoutPadding().encodeToString(sig),
                publicKeyB64Url = Base64.getUrlEncoder().withoutPadding().encodeToString(kp.public.encoded),
            )
        }

        override suspend fun verifyLocalUnlock(
            context: android.content.Context,
            title: String,
            subtitle: String,
            cancelText: String,
        ): BiometricSignResult = signWithEnrolledKey(context, ByteArray(32), title, subtitle, cancelText)

        override fun deleteKey() {
            keyPair = null
            deleted = true
        }
    }

    @Test fun offlineCacheEncryptsDiskFilesAndBindsUserKeyAad() {
        val ser = kotlinx.serialization.builtins.ListSerializer(kotlinx.serialization.serializer<String>())
        val tmpDir = java.nio.file.Files.createTempDirectory("gr_enc_cache_test").toFile()
        try {
            val writer = org.pjstatecm.graceride.data.cache.OfflineCache(tmpDir)
            writer.write(1L, "saved_places", listOf("12 Jalan SS2/24 Gate #4321"), ser, nowMillis = 555L)

            val binFile = java.io.File(tmpDir, "u1_saved_places.bin")
            assertTrue("Expected encrypted .bin cache file", binFile.isFile)
            val rawBytes = binFile.readBytes()
            val rawText = String(rawBytes, Charsets.UTF_8)
            assertFalse("Plaintext address leaked to disk", rawText.contains("Jalan SS2/24"))
            assertFalse("Plaintext gate code leaked to disk", rawText.contains("4321"))

            // Copying u1 ciphertext to u2 filename must fail GCM AAD check and purge corrupt file.
            val u2File = java.io.File(tmpDir, "u2_saved_places.bin")
            binFile.copyTo(u2File, overwrite = true)
            val reader = org.pjstatecm.graceride.data.cache.OfflineCache(tmpDir)
            assertNull("Cross-user ciphertext replay must fail AAD check", reader.read(2L, "saved_places", ser))
            assertFalse("Corrupted/tampered cache file should be purged", u2File.exists())

            // Legacy unencrypted .json file must be discarded and deleted on read.
            val legacyJson = java.io.File(tmpDir, "u3_saved_places.json")
            legacyJson.writeText("""{"savedAtMillis":1,"data":["legacy"]}""", Charsets.UTF_8)
            assertNull(reader.read(3L, "saved_places", ser))
            assertFalse(legacyJson.exists())
        } finally {
            tmpDir.deleteRecursively()
        }
    }

    @Test fun persistentCookieJarEnforcesHostAndSecurePrefixes() {
        val httpsUrl = okhttp3.HttpUrl.Builder().scheme("https").host("graceride.saliminn.my").encodedPath("/api/auth/me").build()
        val httpUrl = okhttp3.HttpUrl.Builder().scheme("http").host("graceride.saliminn.my").encodedPath("/api/auth/me").build()
        val jar = org.pjstatecm.graceride.data.session.PersistentCookieJar(InMemoryStore())

        // Invalid __Host- cookies (non-root path or domain-wide or insecure) must be rejected.
        val badHostPath = okhttp3.Cookie.parse(httpsUrl, "__Host-gr_session=bad1; Path=/api; Secure; HttpOnly")!!
        val badHostDomain = okhttp3.Cookie.parse(httpsUrl, "__Host-gr_session=bad2; Domain=graceride.saliminn.my; Path=/; Secure; HttpOnly")!!
        val badSecure = okhttp3.Cookie.parse(httpUrl, "__Secure-tok=bad3; Path=/; HttpOnly")!!
        val goodHost = okhttp3.Cookie.parse(httpsUrl, "__Host-gr_session=good1; Path=/; Secure; HttpOnly")!!
        jar.saveFromResponse(httpUrl, listOf(badSecure))
        jar.saveFromResponse(httpsUrl, listOf(badHostPath, badHostDomain, goodHost))

        val loadedHttps = jar.loadForRequest(httpsUrl)
        assertEquals(listOf("good1"), loadedHttps.map { it.value })
        assertTrue(jar.loadForRequest(httpUrl).isEmpty())
    }

    @Test fun lockStatePurgesOfflineCachesAndSnapshotsImmediately() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=tok-lock; Path=/; Max-Age=3600; HttpOnly")
                .setBody(Fixtures.ME_MEMBER)
        )
        container.auth.refresh()
        val today = KlTime.today()
        val rideJson = """{"id":19,"status":"confirmed","pickup_at":"$today 08:30:00",
            "area":{"id":2,"name":"PJ"},"destination":"Secret Home","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        OfflineSnapshotStore.save(store, 1L, listOf(apiJson.decodeFromString(Ride.serializer(), rideJson)))
        val strSer = kotlinx.serialization.builtins.ListSerializer(kotlinx.serialization.serializer<String>())
        container.offlineCache.write(1L, "saved_places", listOf("Secret Gate #99"), strSer)

        var lockedCallbackCount = 0
        container.onSessionLocked = { lockedCallbackCount++ }

        container.auth.onLocked()
        // Wait briefly for appScope coroutine inside onLockedCallback.
        kotlinx.coroutines.delay(50)

        assertTrue(container.auth.state.value is AuthState.Locked)
        assertNull(OfflineSnapshotStore.load(store, 1L))
        assertNull(container.offlineCache.read(1L, "saved_places", strSer))
        assertEquals(1, lockedCallbackCount)
    }
}
