package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import kotlinx.coroutines.withTimeout
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.data.session.OfflineSnapshotStore
import org.pjstatecm.graceride.data.session.PersistentCookieJar
import org.pjstatecm.graceride.data.session.SecureStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.domain.KlTime
import org.pjstatecm.graceride.ui.screens.home.HomeViewModel
import org.pjstatecm.graceride.ui.screens.profile.SavedAddressesViewModel
import org.pjstatecm.graceride.ui.screens.rides.RideNewViewModel
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class PerformanceRegressionTest {

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

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        withTimeout(10.seconds) {
            var result: T? = null
            test {
                while (true) {
                    val item = awaitItem()
                    if (predicate(item)) {
                        result = item
                        break
                    }
                }
                cancelAndIgnoreRemainingEvents()
            }
            @Suppress("UNCHECKED_CAST")
            result as T
        }

    @Test fun ridesRepositoryCachesAreasWithinTtlAndRefreshesWhenForced() = runBlocking {
        val areasJson = """[{"id":2,"name":"PJ Section 14","active":true}]"""
        server.enqueue(MockResponse().setBody(areasJson))
        server.enqueue(MockResponse().setBody(areasJson))
        server.enqueue(MockResponse().setBody(areasJson))

        val first = container.rides.areas()
        val second = container.rides.areas()
        assertEquals(1, first.size)
        assertEquals(first, second)
        assertEquals(1, server.requestCount)

        container.rides.areas(forceRefresh = true)
        assertEquals(2, server.requestCount)

        container.rides.clearAreasCache()
        container.rides.areas()
        assertEquals(3, server.requestCount)
    }

    @Test fun appContainerCachesLangAndSendsUpdatedAcceptLanguageHeader() = runBlocking {
        val countingStore = CountingSecureStore(InMemoryStore())
        val appContainer = AppContainer(null, countingStore)
        appContainer.setBaseUrl(server.url("/").toString())

        val readsBefore = countingStore.getCalls.get()
        appContainer.lang = "zh"
        assertEquals("zh", appContainer.lang)
        assertEquals("zh", appContainer.lang)
        // Reads hit @Volatile cachedLang without hitting underlying SecureStore again.
        assertEquals(readsBefore, countingStore.getCalls.get())

        server.enqueue(MockResponse().setBody("[]"))
        appContainer.rides.areas()
        val req = server.takeRequest()
        assertEquals("zh", req.getHeader("Accept-Language"))
    }

    @Test fun persistentCookieJarDefersStoreReadUntilFirstCookieUse() {
        val countingStore = CountingSecureStore(InMemoryStore())
        val jar = PersistentCookieJar(countingStore)
        assertEquals(0, countingStore.getCalls.get())

        jar.hasCookies()
        assertEquals(1, countingStore.getCalls.get())

        jar.hasCookies()
        assertEquals(1, countingStore.getCalls.get())
    }

    @Test fun rideNewAndSavedAddressesLoadInParallel() = runBlocking {
        val concurrentCount = AtomicInteger(0)
        val maxConcurrent = AtomicInteger(0)
        val rideNewBarrier = CountDownLatch(2)

        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val active = concurrentCount.incrementAndGet()
                maxConcurrent.updateAndGet { prev -> maxOf(prev, active) }
                return try {
                    rideNewBarrier.countDown()
                    rideNewBarrier.await(2, TimeUnit.SECONDS)
                    when {
                        request.path?.startsWith("/api/areas") == true ->
                            MockResponse().setBody("""[{"id":2,"name":"PJ","active":true}]""")
                        request.path?.startsWith("/api/saved-places") == true ->
                            MockResponse().setBody("""[{"id":1,"label":"Home","area_id":2,"address":"10 SS2","is_default":1}]""")
                        request.path?.startsWith("/api/proxy-riders") == true ->
                            MockResponse().setBody("[]")
                        request.path?.startsWith("/api/events/20") == true ->
                            MockResponse().setBody("""{"id":20,"title":"Service","starts_at":"2026-11-29 10:00:00","location":"Church Hall","my_signup":{"status":"going"}}""")
                        else -> MockResponse().setBody("[]")
                    }
                } finally {
                    concurrentCount.decrementAndGet()
                }
            }
        }

        val rideNewVm = RideNewViewModel(container, 20L, initialProxy = false)
        val state = rideNewVm.state.awaitUntil { !it.loading && it.event != null }
        assertEquals(1, state.areas.size)
        assertTrue("Expected concurrent init fetches on RideNewViewModel, saw max=${maxConcurrent.get()}", maxConcurrent.get() >= 2)

        // Now verify SavedAddressesViewModel also overlaps savedPlaces + areas in parallel
        container.rides.clearAreasCache()
        maxConcurrent.set(0)
        val savedBarrier = CountDownLatch(2)
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val active = concurrentCount.incrementAndGet()
                maxConcurrent.updateAndGet { prev -> maxOf(prev, active) }
                return try {
                    savedBarrier.countDown()
                    savedBarrier.await(2, TimeUnit.SECONDS)
                    when {
                        request.path?.startsWith("/api/saved-places") == true ->
                            MockResponse().setBody("""[{"id":1,"label":"Home","area_id":2,"address":"10 SS2","is_default":1}]""")
                        request.path?.startsWith("/api/areas") == true ->
                            MockResponse().setBody("""[{"id":2,"name":"PJ","active":true}]""")
                        else -> MockResponse().setBody("[]")
                    }
                } finally {
                    concurrentCount.decrementAndGet()
                }
            }
        }

        val savedVm = SavedAddressesViewModel(container)
        val savedState = savedVm.state.awaitUntil { !it.loading }
        assertEquals(1, savedState.places.size)
        assertTrue("Expected concurrent fetches on SavedAddressesViewModel, saw max=${maxConcurrent.get()}", maxConcurrent.get() >= 2)
    }

    @Test fun homeViewModelCancelsOverlappingRefreshAndPersistsSnapshotOffMain() = runBlocking {
        val today = KlTime.today()
        val staleRide = """{"id":10,"status":"confirmed","pickup_at":"$today 08:30:00",
            "area":{"id":2,"name":"PJ"},"destination":"Stale Hall","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""
        val freshRide = """{"id":11,"status":"confirmed","pickup_at":"$today 09:30:00",
            "area":{"id":2,"name":"PJ"},"destination":"Fresh Hall","seats":1,
            "passenger":{"id":1,"name":"Test Member"},"viewer":{"is_passenger":true,"is_driver":false,"is_staff":false}}"""

        server.enqueue(MockResponse().setBody(Fixtures.ME_MEMBER))
        container.auth.refresh()
        server.takeRequest()

        val firstStarted = CountDownLatch(1)
        val allowFirstFinish = CountDownLatch(1)
        val rideCallCount = AtomicInteger(0)

        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse {
                val p = request.path ?: ""
                return when {
                    p.startsWith("/api/auth/me") -> MockResponse().setBody(Fixtures.ME_MEMBER)
                    p.startsWith("/api/rides") -> {
                        val callNum = rideCallCount.incrementAndGet()
                        if (callNum == 1) {
                            firstStarted.countDown()
                            allowFirstFinish.await(2, TimeUnit.SECONDS)
                            MockResponse().setBody("[$staleRide]")
                        } else {
                            MockResponse().setBody("[$freshRide]")
                        }
                    }
                    else -> MockResponse().setBody("[]")
                }
            }
        }

        val vm = HomeViewModel(container)
        assertTrue(firstStarted.await(2, TimeUnit.SECONDS))
        // Overlapping refresh must cancel the stale init job before releasing the slow first call.
        vm.refresh()
        delay(50)
        allowFirstFinish.countDown()

        val finalState = vm.state.awaitUntil { !it.loading && !it.refreshing && it.nextRide != null }
        assertEquals("Fresh Hall", finalState.nextRide?.destination)

        val snap = OfflineSnapshotStore.load(store, 1L)
        assertNotNull(snap)
        assertEquals(listOf(11L), snap!!.rides.map { it.id })
    }

    @Test fun setBaseUrlSwapsApiServiceWhilePreservingRepositoryIdentity() = runBlocking {
        val secondServer = MockWebServer()
        secondServer.start()
        try {
            val container = AppContainer(null, InMemoryStore())
            container.setBaseUrl(server.url("/").toString())
            val ridesRepo = container.rides
            val authRepo = container.auth

            server.enqueue(MockResponse().setBody("""[{"id":1,"name":"PJ","active":true}]"""))
            val firstAreas = container.rides.areas()
            assertEquals("PJ", firstAreas.single().name)

            container.setBaseUrl(secondServer.url("/").toString())
            org.junit.Assert.assertSame(ridesRepo, container.rides)
            org.junit.Assert.assertSame(authRepo, container.auth)

            secondServer.enqueue(MockResponse().setBody("""[{"id":2,"name":"Subang","active":true}]"""))
            val secondAreas = container.rides.areas()
            assertEquals("Subang", secondAreas.single().name)
            assertEquals(1, secondServer.requestCount)
        } finally {
            secondServer.shutdown()
        }
    }

    private class CountingSecureStore(private val delegate: SecureStore) : SecureStore {
        val getCalls = AtomicInteger(0)
        override fun getString(key: String): String? {
            getCalls.incrementAndGet()
            return delegate.getString(key)
        }
        override fun putString(key: String, value: String) = delegate.putString(key, value)
        override fun remove(key: String) = delegate.remove(key)
        override fun clear() = delegate.clear()
    }

    @Test fun onLockedPersistsLockedFlagAcrossOfflineColdStart() = runBlocking {
        server.enqueue(
            MockResponse()
                .addHeader("Set-Cookie", "gr_session=test_cookie; Path=/; HttpOnly")
                .setBody(Fixtures.ME_MEMBER),
        )
        container.auth.refresh()
        assertTrue(container.auth.state.value is org.pjstatecm.graceride.data.repo.AuthState.SignedIn)

        container.auth.onLocked()
        assertTrue(container.auth.state.value is org.pjstatecm.graceride.data.repo.AuthState.Locked)

        val closedUrl = server.url("/").toString()
        server.shutdown()
        val coldContainer = AppContainer(null, store)
        coldContainer.setBaseUrl(closedUrl)
        coldContainer.auth.refresh()
        val coldState = coldContainer.auth.state.value
        assertTrue(coldState is org.pjstatecm.graceride.data.repo.AuthState.Locked)
    }

    @Test fun refreshThrowOnErrorPropagatesServerFailure() = runBlocking {
        server.enqueue(MockResponse().setResponseCode(500).setBody("""{"error":"server_down"}"""))
        var caught: org.pjstatecm.graceride.data.api.ApiException? = null
        try {
            container.auth.refresh(throwOnError = true)
        } catch (e: org.pjstatecm.graceride.data.api.ApiException) {
            caught = e
        }
        assertNotNull(caught)
        assertEquals(500, caught?.status)
    }
}
