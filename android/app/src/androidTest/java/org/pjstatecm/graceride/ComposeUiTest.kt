package org.pjstatecm.graceride

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performTextInput
import androidx.navigation.compose.rememberNavController
import androidx.test.ext.junit.runners.AndroidJUnit4
import okhttp3.mockwebserver.Dispatcher
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.RecordedRequest
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.BiometricEnrollment
import org.pjstatecm.graceride.data.api.Me
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideArea
import org.pjstatecm.graceride.data.api.RidePassenger
import org.pjstatecm.graceride.data.api.RideViewer
import org.pjstatecm.graceride.data.api.Share
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.platform.BiometricAvailability
import org.pjstatecm.graceride.platform.BiometricCryptoDriver
import org.pjstatecm.graceride.platform.BiometricSignResult
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.components.ShareCard
import org.pjstatecm.graceride.ui.screens.events.EventDetailScreen
import org.pjstatecm.graceride.ui.screens.login.BiometricGateScreen
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
import org.pjstatecm.graceride.ui.screens.login.UnlockScreen
import org.pjstatecm.graceride.ui.screens.profile.ProxyRidersScreen
import org.pjstatecm.graceride.ui.screens.rides.RideNewScreen
import org.pjstatecm.graceride.ui.screens.rides.RidesScreen
import org.pjstatecm.graceride.ui.theme.GraceRideTheme

@RunWith(AndroidJUnit4::class)
class ComposeUiTest {

    @get:Rule
    val compose = createComposeRule()

    private lateinit var server: MockWebServer
    private lateinit var container: AppContainer

    @Before fun setup() {
        server = MockWebServer()
        server.start()
        container = AppContainer(null, InMemoryStore())
        container.setBaseUrl(server.url("/").toString())
    }

    @After fun teardown() {
        server.shutdown()
    }

    private val ride = Ride(
        id = 10,
        status = "confirmed",
        pickupAt = "2026-11-22 09:30:00",
        area = RideArea(2, "PJ Section 14"),
        destination = "Church Hall",
        passenger = RidePassenger(1, "Test Member"),
        viewer = RideViewer(isPassenger = true),
    )

    @Test fun rideCardShowsRouteAndStatus() {
        compose.setContent {
            GraceRideTheme { RideCard(ride, onClick = {}) }
        }
        compose.onNodeWithText("PJ Section 14 → Church Hall", substring = true).assertIsDisplayed()
        compose.onNodeWithText("Confirmed").assertIsDisplayed()
    }

    @Test fun rideCardClickInvokesCallback() {
        var clicked = false
        compose.setContent {
            GraceRideTheme { RideCard(ride, onClick = { clicked = true }) }
        }
        compose.onNodeWithText("PJ Section 14 → Church Hall", substring = true).performClick()
        assertTrue(clicked)
    }

    @Test fun errorViewRetryWorks() {
        var retried = false
        compose.setContent {
            GraceRideTheme { ErrorView("Network down", onRetry = { retried = true }) }
        }
        compose.onNodeWithText("Network down").assertIsDisplayed()
        compose.onNodeWithText("Retry").performClick()
        assertTrue(retried)
    }

    @Test fun emptyViewShowsMessage() {
        compose.setContent {
            GraceRideTheme { EmptyView("No rides to show.") }
        }
        compose.onNodeWithText("No rides to show.").assertIsDisplayed()
    }

    @Test fun termsGateRequiresExplicitOptIn() {
        compose.setContent {
            GraceRideTheme { org.pjstatecm.graceride.ui.screens.login.TermsScreen(container) }
        }
        compose.onNodeWithText("Terms of use and privacy notice").assertIsDisplayed()
        // The opt-in row and agree button are the last items of a long list.
        compose.onNodeWithTag("termsList").performScrollToIndex(19)
        compose.onNodeWithText("Agree and continue").assertIsNotEnabled()
        compose.onNodeWithText("I have read these terms and the privacy notice, and I opt in", substring = true)
            .performClick()
        compose.onNodeWithText("Agree and continue").assertIsEnabled()
    }

    @Test fun loginScreenRendersPasskeyButton() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = when (request.path) {
                "/api/auth/turnstile" -> MockResponse().setBody("""{"enabled":false}""")
                else -> MockResponse().setBody("[]")
            }
        }
        compose.setContent {
            GraceRideTheme { LoginScreen(container) }
        }
        compose.onNodeWithText("Sign in with passkey").assertIsDisplayed()
        compose.onNodeWithText("Grace Ride").assertIsDisplayed()
    }

    @Test fun loginScreenListsDevAccounts() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = when (request.path) {
                "/api/auth/turnstile" -> MockResponse().setBody("""{"enabled":false}""")
                "/api/auth/dev-users" -> MockResponse().setBody("""[{"name":"Demo Member","email":"member@example.org","roles":"member"}]""")
                else -> MockResponse().setBody("[]")
            }
        }
        compose.setContent {
            GraceRideTheme { LoginScreen(container) }
        }
        compose.waitUntil(5_000) {
            compose.onAllNodesWithText("Demo Member", substring = true).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText("Demo Member", substring = true).performScrollTo().assertIsDisplayed()
    }

    @Test fun shareCardNeedsInfoShowsNoteAndResubmit() {
        val share = Share(
            id = 5,
            settlementId = 2,
            title = "Fuel split",
            eventTitle = "Sunday Service",
            amountCents = 1550,
            status = "needs_info",
            note = "Receipt blurry, re-upload",
        )
        var clicked = false
        compose.setContent {
            GraceRideTheme { ShareCard(share, onPay = { clicked = true }) }
        }
        compose.onNodeWithText("More info needed").assertIsDisplayed()
        compose.onNodeWithText("Receipt blurry, re-upload").assertIsDisplayed()
        compose.onNodeWithText("Resubmit proof").performClick()
        assertTrue(clicked)
    }

    @Test fun ridesScreenHasTopAppBarAndBackButton() {
        server.enqueue(MockResponse().setBody("[]"))
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                RidesScreen(container, nav)
            }
        }
        compose.onNodeWithText("My rides").assertIsDisplayed()
        compose.onNodeWithContentDescription("Navigate back").assertIsDisplayed()
    }

    @Test fun unlockScreenBackupModeAcceptsAlphanumericCode() {
        compose.setContent {
            GraceRideTheme { UnlockScreen(container) }
        }
        compose.onNodeWithText("Backup code").performClick()
        compose.onNodeWithText("Backup code").performTextInput("ABCD-1234")
        compose.onNodeWithText("ABCD-1234").assertIsDisplayed()
    }

    @Test fun eventDetailPastEventHidesSignupAndShowsReturnOnlyRide() {
        val pastEvent = """{"id":30,"title":"Past Gathering","starts_at":"2020-01-01 09:00:00","status":"published",
            "my_signup":{"status":"going"},"my_return_ride":{"id":77,"status":"completed"}}"""
        server.enqueue(MockResponse().setBody(pastEvent))
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                EventDetailScreen(container, nav, 30L)
            }
        }
        compose.waitUntil(5_000) {
            try {
                compose.onNodeWithText("Past Gathering").assertIsDisplayed()
                true
            } catch (_: AssertionError) {
                false
            }
        }
        compose.onNodeWithText("Return ride").assertIsDisplayed()
        compose.onNodeWithText("Request a ride").assertDoesNotExist()
        compose.onNodeWithText("Withdraw").assertDoesNotExist()
    }

    @Test fun rideNewWithoutEventHasTopAppBarAndBrowseButton() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = MockResponse().setBody("[]")
        }
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                RideNewScreen(container, nav, eventId = null)
            }
        }
        compose.onNodeWithContentDescription("Navigate back").assertIsDisplayed()
        compose.onNodeWithText("Browse events").assertIsDisplayed()
    }

    @Test fun proxyRidersScreenLocalizesCellMemberLabel() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse =
                when {
                    request.path?.startsWith("/api/proxy-riders") == true ->
                        MockResponse().setBody(
                            """[{"id":4,"name":"Uncle Lim","phone":"0123456","relationship":"cell_member","notes":"","area_id":null,"pickup_address":"10 SS2","gate_notes":""}]"""
                        )
                    else -> MockResponse().setBody("[]")
                }
        }
        compose.setContent {
            GraceRideTheme {
                val nav = rememberNavController()
                ProxyRidersScreen(container, nav)
            }
        }
        compose.waitUntil(5_000) {
            try {
                compose.onNodeWithText("Uncle Lim").assertIsDisplayed()
                true
            } catch (_: AssertionError) {
                false
            }
        }
        compose.onNodeWithText("Cell group member").assertIsDisplayed()
        compose.onNodeWithText("cell_member").assertDoesNotExist()
    }

    @Test fun loginScreenShowsDirectBiometricButtonWhenEnrolled() {
        server.dispatcher = object : Dispatcher() {
            override fun dispatch(request: RecordedRequest): MockResponse = when (request.path) {
                "/api/auth/turnstile" -> MockResponse().setBody("""{"enabled":false}""")
                else -> MockResponse().setBody("[]")
            }
        }
        container.biometric.saveEnrollment(
            BiometricEnrollment(
                id = 1L,
                credentialId = "cred-ui-0123456789abcdef",
                userId = 1L,
                email = "member@example.org",
                userName = "Test Member",
                label = "Pixel",
            )
        )
        compose.setContent {
            GraceRideTheme { LoginScreen(container) }
        }
        compose.onNodeWithText("Sign in with biometrics").assertIsDisplayed()
        compose.onNodeWithText("Test Member", substring = true).assertIsDisplayed()
        compose.onNodeWithText("Sign in with passkey").assertIsDisplayed()
    }

    @Test fun biometricGateScreenUnlocksAppWithHardwareDriver() {
        val fakeDriver = object : BiometricCryptoDriver {
            override fun availability(context: android.content.Context?) = BiometricAvailability.AVAILABLE
            override suspend fun generateAndSign(
                context: android.content.Context,
                payload: ByteArray,
                title: String,
                subtitle: String,
                cancelText: String,
                attestationChallenge: ByteArray?,
            ) = BiometricSignResult.Success("sig", "pub")
            override suspend fun signWithEnrolledKey(
                context: android.content.Context,
                payload: ByteArray,
                title: String,
                subtitle: String,
                cancelText: String,
            ) = BiometricSignResult.Success("sig", "pub")
            override suspend fun verifyLocalUnlock(
                context: android.content.Context,
                title: String,
                subtitle: String,
                cancelText: String,
            ) = BiometricSignResult.Success("sig", "pub")
            override fun deleteKey() {}
        }
        val gatedContainer = AppContainer(null, InMemoryStore(), fakeDriver)
        val me = Me(id = 1L, name = "Test Member", email = "member@example.org", passkeyCount = 1)
        gatedContainer.biometric.saveEnrollment(
            BiometricEnrollment(
                id = 1L,
                credentialId = "cred-ui-0123456789abcdef",
                userId = 1L,
                email = "member@example.org",
                userName = "Test Member",
            )
        )
        compose.setContent {
            GraceRideTheme { BiometricGateScreen(gatedContainer, me) }
        }
        compose.waitUntil(5_000) { gatedContainer.biometric.appUnlocked.value }
        assertTrue(gatedContainer.biometric.appUnlocked.value)
    }
}
