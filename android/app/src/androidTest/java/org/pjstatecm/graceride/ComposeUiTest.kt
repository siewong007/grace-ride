package org.pjstatecm.graceride

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToIndex
import androidx.test.ext.junit.runners.AndroidJUnit4
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.pjstatecm.graceride.data.api.Ride
import org.pjstatecm.graceride.data.api.RideArea
import org.pjstatecm.graceride.data.api.RidePassenger
import org.pjstatecm.graceride.data.api.RideViewer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.ui.components.EmptyView
import org.pjstatecm.graceride.ui.components.ErrorView
import org.pjstatecm.graceride.ui.components.RideCard
import org.pjstatecm.graceride.ui.screens.login.LoginScreen
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
        compose.onNodeWithTag("termsList").performScrollToIndex(18)
        compose.onNodeWithText("Agree and continue").assertIsNotEnabled()
        compose.onNodeWithText("I have read these terms and the privacy notice, and I opt in", substring = true)
            .performClick()
        compose.onNodeWithText("Agree and continue").assertIsEnabled()
    }

    @Test fun loginScreenRendersPasskeyButton() {
        server.enqueue(MockResponse().setBody("[]")) // dev users
        compose.setContent {
            GraceRideTheme { LoginScreen(container) }
        }
        compose.onNodeWithText("Sign in with passkey").assertIsDisplayed()
        compose.onNodeWithText("Grace Ride").assertIsDisplayed()
    }

    @Test fun loginScreenListsDevAccounts() {
        server.enqueue(
            MockResponse().setBody("""[{"name":"Demo Member","email":"member@example.org","roles":"member"}]""")
        )
        compose.setContent {
            GraceRideTheme { LoginScreen(container) }
        }
        compose.waitUntil(5_000) {
            try {
                compose.onNodeWithText("Demo Member", substring = true).assertIsDisplayed()
                true
            } catch (_: AssertionError) {
                false
            }
        }
    }
}
