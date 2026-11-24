package org.pjstatecm.graceride

import androidx.compose.ui.graphics.Color
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.pjstatecm.graceride.data.api.ApiException
import org.pjstatecm.graceride.data.api.Badge
import org.pjstatecm.graceride.data.api.EventItem
import org.pjstatecm.graceride.data.api.apiCall
import org.pjstatecm.graceride.data.api.apiJson
import org.pjstatecm.graceride.di.AppContainer
import org.pjstatecm.graceride.data.session.InMemoryStore
import org.pjstatecm.graceride.ui.Routes
import org.pjstatecm.graceride.ui.components.AvatarStyle
import org.pjstatecm.graceride.ui.components.BadgeIcons
import org.pjstatecm.graceride.ui.components.BadgeTierStyle
import org.pjstatecm.graceride.ui.components.EventIcons

/** Regressions found by live emulator testing against the real dev backend. */
@OptIn(ExperimentalCoroutinesApi::class)
class RegressionTest {

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

    @Test fun eventGroupsIsADisplayStringNotAnArray() {
        // event_json renders `groups` as joined display names ("全体会员"),
        // while `join_as` (detail only) is the array of {id,name}.
        val json = """{"id":4,"title":"晨走","starts_at":"2026-11-22 07:00:00","kind":"walk",
            "going":0,"groups":"全体会员","join_as":[{"id":1,"name":"全体会员"}]}"""
        val event = apiJson.decodeFromString(EventItem.serializer(), json)
        assertEquals("全体会员", event.groups)
        assertEquals(1, event.joinAs.size)
        assertEquals("全体会员", event.joinAs[0].name)
    }

    @Test fun unexpectedJsonShapeDegradesToErrorNotCrash() = runTest {
        server.enqueue(MockResponse().setBody("""[{"id":"not-a-number","status":"confirmed"}]"""))
        try {
            apiCall(container.authEvents) { container.api.myRides("upcoming") }
            throw AssertionError("expected ApiException")
        } catch (e: ApiException) {
            assertEquals("DECODE_ERROR", e.code)
            assertTrue(e.status != 0) // not misreported as offline
        }
    }

    @Test fun eventItemDeserializesMyRide() {
        val json = """{"id":11,"title":"Worship","starts_at":"2026-11-29 09:00:00","my_ride":{"id":55,"status":"confirmed"}}"""
        val event = apiJson.decodeFromString(EventItem.serializer(), json)
        assertEquals(55L, event.myRide?.id)
        assertEquals("confirmed", event.myRide?.status)
    }

    @Test fun badgeDeserializesCustomIcon() {
        val json = """{"key":"night_owl","char":"夜","icon":"moon","name":"Night Owl","rule":"3 late rides"}"""
        val badge = apiJson.decodeFromString(Badge.serializer(), json)
        assertEquals("moon", badge.icon)
        assertEquals("night_owl", badge.key)
    }

    @Test fun rideNewRouteBuildsQueryCombinations() {
        assertEquals("ride-new", Routes.rideNew())
        assertEquals("ride-new?event=12", Routes.rideNew(eventId = 12L))
        assertEquals("ride-new?proxy=1", Routes.rideNew(proxy = true))
        assertEquals("ride-new?event=12&proxy=1", Routes.rideNew(eventId = 12L, proxy = true))
    }

    @Test fun animatedAvatarsAndBadgeIconsResolve() {
        for (anim in listOf(
            "grace_car", "peace_dove", "sunny_cloud", "happy_bear",
            "fellowship_cat", "joy_puppy", "star_lantern", "blossom_sprout",
            "grok", "muse",
        )) {
            assertTrue("$anim should be animated", AvatarStyle.isAnimatedAvatar(anim))
        }
        assertFalse(AvatarStyle.isAnimatedAvatar("car"))
        assertFalse(AvatarStyle.isAnimatedAvatar(null))

        val reg = BadgeIcons.resolve(icon = "moon")
        assertTrue(reg is BadgeIcons.ResolvedBadgeIcon.GifAsset)
        assertEquals(
            "file:///android_asset/icons/badges/moon.gif",
            (reg as BadgeIcons.ResolvedBadgeIcon.GifAsset).assetUri,
        )

        for (animBadge in listOf(
            "badge_joy_rider", "badge_fellowship_sprout", "badge_punctual_clock", "badge_warm_welcome",
            "badge_helping_wheels", "badge_traffic_light", "badge_route_compass", "badge_traffic_whistle", "badge_care_shield",
        )) {
            val resolved = BadgeIcons.resolve(icon = animBadge)
            assertTrue("$animBadge should resolve to GifAsset", resolved is BadgeIcons.ResolvedBadgeIcon.GifAsset)
            assertEquals(
                "file:///android_asset/icons/badges/$animBadge.gif",
                (resolved as BadgeIcons.ResolvedBadgeIcon.GifAsset).assetUri,
            )
        }

        val emoji = BadgeIcons.resolve(icon = "🦉")
        assertEquals(BadgeIcons.ResolvedBadgeIcon.Text("🦉"), emoji)

        val badge = Badge(key = "k", char = "N", icon = null)
        val fallback = BadgeIcons.resolve(badge = badge, fallbackChar = "R")
        assertEquals(BadgeIcons.ResolvedBadgeIcon.Text("R"), fallback)

        assertEquals("file:///android_asset/icons/avatars/grace_car.gif", AvatarStyle.gifAssetUri("grace_car"))
        assertEquals("file:///android_asset/icons/avatars/grok_gray.gif", AvatarStyle.gifAssetUri("grok"))
        assertEquals("file:///android_asset/icons/avatars/grok_coral.gif", AvatarStyle.gifAssetUri("grok", "coral"))
        assertEquals("file:///android_asset/icons/avatars/grok_coral.gif", AvatarStyle.gifAssetUri("grok:circle:coral", "white:hexagon"))
        assertEquals("file:///android_asset/icons/avatars/muse_amber.gif", AvatarStyle.gifAssetUri("muse"))
        assertEquals("file:///android_asset/icons/avatars/muse_pink.gif", AvatarStyle.gifAssetUri("muse", "pink"))
        assertEquals("file:///android_asset/icons/avatars/car.gif", AvatarStyle.gifAssetUri("car"))
        assertEquals(null, AvatarStyle.gifAssetUri(null))
        assertEquals(null, AvatarStyle.gifAssetUri("initials::teal"))

        assertEquals("file:///android_asset/icons/events/utensils.gif", EventIcons.gifAssetUri("utensils"))
        assertEquals("file:///android_asset/icons/events/trophy.gif", EventIcons.gifAssetUri(null, "badminton"))
        assertEquals(null, EventIcons.gifAssetUri("🍲"))
    }

    @Test fun bundledGifAssetsExistAndAreGif89a() {
        val baseDir = listOf(
            File("src/main/assets/icons"),
            File("android/app/src/main/assets/icons"),
        ).firstOrNull { it.isDirectory }
            ?: throw AssertionError("Could not locate android/app/src/main/assets/icons")

        val expectedFiles = mutableSetOf<String>()
        for (icon in AvatarStyle.ALL_ICONS) {
            expectedFiles += "avatars/$icon.gif"
            if (icon == "grok" || icon == "muse") {
                for (color in AvatarStyle.COLORS) {
                    expectedFiles += "avatars/${icon}_$color.gif"
                }
            }
        }
        assertEquals(40, expectedFiles.count { it.startsWith("avatars/") })

        for (badgeKey in BadgeIcons.allGifKeys) {
            expectedFiles += "badges/$badgeKey.gif"
        }
        assertEquals(29, expectedFiles.count { it.startsWith("badges/") })

        for (eventKey in EventIcons.PRESET_KEYS) {
            expectedFiles += "events/$eventKey.gif"
        }
        assertEquals(28, expectedFiles.count { it.startsWith("events/") })
        assertEquals(97, expectedFiles.size)

        for (rel in expectedFiles) {
            val file = File(baseDir, rel)
            assertTrue("Missing asset $rel at ${file.absolutePath}", file.isFile)
            val bytes = file.readBytes()
            assertTrue("Asset $rel too short", bytes.size >= 6)
            assertEquals("GIF89a", String(bytes, 0, 6, Charsets.US_ASCII))
        }
    }

    @Test fun whiteAvatarAndMultiTierBadgesAreSupported() {
        assertTrue("white" in AvatarStyle.COLORS)
        assertEquals(Color(0xFFFFFFFF), AvatarStyle.grokSkinColor("white"))
        assertEquals(Color(0xFF18181B), AvatarStyle.grokEyeColor("white"))
        assertEquals(Color(0xFFFFFFFF), AvatarStyle.grokEyeColor("gray"))
        assertEquals(Color(0xFFFFFFFF) to Color(0xFFE2E8F0), AvatarStyle.musePlushColors("white"))

        val tieredJson = """
            {
              "key":"member_first_journey",
              "category":"member",
              "role_match":true,
              "char":"J",
              "icon":"badge_joy_rider",
              "name":"Joyful Rider",
              "rule":"Complete rides",
              "progress":28,
              "target":50,
              "earned":true,
              "tier":"gold",
              "tier_level":3,
              "next_tier":"platinum",
              "next_target":50,
              "tiers":[
                {"tier":"bronze","target":3,"reached":true},
                {"tier":"silver","target":10,"reached":true},
                {"tier":"gold","target":25,"reached":true},
                {"tier":"platinum","target":50,"reached":false}
              ]
            }
        """.trimIndent()
        val parsed = apiJson.decodeFromString(Badge.serializer(), tieredJson)
        assertEquals("gold", parsed.tier)
        assertEquals(3, parsed.tierLevel)
        assertEquals("platinum", parsed.nextTier)
        assertEquals(50L, parsed.nextTarget)
        assertEquals(4, parsed.tiers.size)
        assertEquals("gold", BadgeTierStyle.resolveTier(parsed))

        val distinctBgs = BadgeTierStyle.ALL_TIERS.map { BadgeTierStyle.palette(it).bg }.toSet()
        assertEquals(4, distinctBgs.size)
    }

    @Test
    fun avatarStyleCodecSupportsShapesAndForegroundColors() {
        assertEquals(8, AvatarStyle.AVATAR_SHAPES.size)
        assertEquals(6, AvatarStyle.AVATAR_BG_SHAPES.size)
        assertEquals(9, AvatarStyle.COLORS.size)

        assertEquals("white:hexagon", AvatarStyle.encodeAvatarColor("white", "hexagon"))
        assertEquals("amber", AvatarStyle.encodeAvatarColor("amber", "circle"))
        assertEquals(":squircle", AvatarStyle.encodeAvatarColor("", "squircle"))

        val parsedColor = AvatarStyle.parseAvatarColor("white:hexagon")
        assertEquals("white", parsedColor.bgColor)
        assertEquals("hexagon", parsedColor.bgShape)

        val defaultColor = AvatarStyle.parseAvatarColor("blue")
        assertEquals("blue", defaultColor.bgColor)
        assertEquals("circle", defaultColor.bgShape)

        assertEquals("grok:circle:coral", AvatarStyle.encodeAvatarIcon("grok", "circle", "coral"))
        assertEquals("grok::coral", AvatarStyle.encodeAvatarIcon("grok", "hexagon", "coral"))
        assertEquals("grok", AvatarStyle.encodeAvatarIcon("grok", "hexagon", null))
        assertEquals("muse:droplet:pink", AvatarStyle.encodeAvatarIcon("muse", "droplet", "pink"))
        assertEquals("muse", AvatarStyle.encodeAvatarIcon("muse", "cloud", null))
        assertEquals("car::purple", AvatarStyle.encodeAvatarIcon("car", "droplet", "purple"))
        assertEquals("initials::teal", AvatarStyle.encodeAvatarIcon("initials", null, "teal"))
        assertEquals("", AvatarStyle.encodeAvatarIcon("initials", null, null))

        val parsedIcon = AvatarStyle.parseAvatarIcon("grok:circle:coral")
        assertEquals("grok", parsedIcon.icon)
        assertEquals("circle", parsedIcon.shape)
        assertEquals("coral", parsedIcon.fgColor)
        assertTrue(AvatarStyle.isAnimatedAvatar("grok:circle:coral"))
        assertTrue(AvatarStyle.isShapeableAvatar("muse:droplet:pink"))
        assertFalse(AvatarStyle.isShapeableAvatar("car::purple"))
        assertEquals("blue", AvatarStyle.defaultColorFor("grace_car"))
        assertEquals(
            "file:///android_asset/icons/avatars/grok_coral.gif",
            AvatarStyle.gifAssetUri("grok::coral", "white:hexagon"),
        )
        assertEquals(null, AvatarStyle.gifAssetUri("initials::teal", "white:rounded"))
    }
}
