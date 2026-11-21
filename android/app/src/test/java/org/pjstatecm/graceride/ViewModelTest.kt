package org.pjstatecm.graceride

import app.cash.turbine.test
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.ui.screens.home.HomeUiState
import org.pjstatecm.graceride.ui.screens.home.HomeViewModel
import org.pjstatecm.graceride.ui.screens.login.LoginUiState
import org.pjstatecm.graceride.ui.screens.login.LoginViewModel
import org.pjstatecm.graceride.ui.screens.rides.RidesUiState
import org.pjstatecm.graceride.ui.screens.rides.RidesViewModel
import kotlin.time.Duration.Companion.seconds

@OptIn(ExperimentalCoroutinesApi::class)
class ViewModelTest {

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before fun setup() {
        Dispatchers.setMain(UnconfinedTestDispatcher())
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After fun teardown() {
        server.shutdown()
        Dispatchers.resetMain()
    }

    private fun json(body: String) = MockResponse().setBody(body)

    private suspend fun <T> StateFlow<T>.awaitUntil(predicate: (T) -> Boolean): T =
        kotlinx.coroutines.withTimeout(10.seconds) {
            var result: T? = null
            test {
                while (true) {
                    val item = awaitItem()
                    if (predicate(item)) { result = item; break }
                }
                cancelAndIgnoreRemainingEvents()
            }
            @Suppress("UNCHECKED_CAST")
            result as T
        }

    private suspend fun signIn() {
        server.enqueue(json(Fixtures.ME_MEMBER))
        container.auth.refresh()
    }

    @Test fun homeLoadsAllSections() = runBlocking {
        signIn()
        server.enqueue(json("[${Fixtures.RIDE}]"))       // rides
        server.enqueue(json("[${Fixtures.EVENT}]"))      // events
        server.enqueue(json("[${Fixtures.SHARE}]"))      // shares
        server.enqueue(json("[${Fixtures.ANN}]"))        // announcements
        server.enqueue(json("[]"))                       // driver schedule

        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertEquals("Test Member", state.me?.name)
        assertEquals(10L, state.nextRide?.id)
        assertEquals(1, state.events.size)
        assertEquals(1, state.unpaidShares.size)
        assertEquals(1, state.announcements.size)
        assertNull(state.error)
    }

    @Test fun homeSurvivesSectionFailure() = runBlocking {
        signIn()
        server.enqueue(MockResponse().setResponseCode(500).setBody("""{"error":"boom"}""")) // rides
        server.enqueue(json("[${Fixtures.EVENT}]"))
        server.enqueue(json("[]"))
        server.enqueue(json("[${Fixtures.ANN}]"))
        server.enqueue(json("[]"))

        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertNull(state.nextRide)
        assertEquals(1, state.events.size)
        assertEquals(1, state.announcements.size)
    }

    @Test fun homeOfflineFlag() = runBlocking {
        signIn()
        server.shutdown() // every section now fails with IOException
        val vm = HomeViewModel(container)
        val state = vm.state.awaitUntil { !it.loading }
        assertTrue(state.offline)
    }

    @Test fun ridesTabsSwitchScopes() = runBlocking {
        server.enqueue(json("[${Fixtures.RIDE}]")) // upcoming
        val vm = RidesViewModel(container)
        val first = vm.state.awaitUntil { !it.loading }
        assertEquals(1, first.rides.size)
        assertEquals("/api/rides?scope=upcoming", server.takeRequest().path)

        server.enqueue(json("[]")) // past
        vm.setTab(1)
        val second = vm.state.awaitUntil { !it.loading && it.tab == 1 }
        assertEquals(0, second.rides.size)
        assertEquals("/api/rides?scope=past", server.takeRequest().path)
    }

    @Test fun loginDevAccountsAndSignIn() = runBlocking {
        server.enqueue(json("""[{"name":"Demo Member","email":"member@example.org","roles":"member"}]"""))
        val vm = LoginViewModel(container)
        val withUsers = vm.state.awaitUntil { it.devUsers.isNotEmpty() }
        assertEquals("member@example.org", withUsers.devUsers[0].email)

        server.enqueue(json("""{"ok":true}"""))
        server.enqueue(json(Fixtures.ME_MEMBER))
        vm.devLogin("member@example.org")
        vm.state.awaitUntil { !it.signingIn }
        assertTrue(container.auth.state.value is org.pjstatecm.graceride.data.repo.AuthState.SignedIn)
    }

    @Test fun loginFailureSurfacesError() = runBlocking {
        server.enqueue(json("[]")) // dev users
        val vm = LoginViewModel(container)
        // Wait until the dev-users request actually landed before queueing the
        // failure response, so the dev-login POST cannot consume the wrong one.
        server.takeRequest(5, java.util.concurrent.TimeUnit.SECONDS)
        server.enqueue(MockResponse().setResponseCode(400).setBody("""{"error":"No such demo account","code":"INVALID_CREDENTIALS"}"""))
        vm.devLogin("nobody@example.org")
        val state = vm.state.awaitUntil { !it.signingIn && it.error != null }
        assertEquals("No such demo account", state.error)
    }
}
