package org.pjstatecm.graceride

import kotlin.math.abs
import kotlin.math.hypot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.pjstatecm.graceride.ui.components.GrokBotGeometry
import org.pjstatecm.graceride.ui.components.GrokBotMotion
import org.pjstatecm.graceride.ui.components.GrokEllipse

/** Pins the Android complex moves to the web engine (`frontend/src/grokBotMotion.ts`). */
class GrokBotMotionTest {
    private val eps = 1e-4f
    private val faceEyes = GrokBotGeometry.computeFaceEyePoses("neutral")
    private fun rest(shape: String) = GrokBotGeometry.SHAPE_COORDS.getValue(shape)

    @Test fun piecewiseLookupClampsAndInterpolates() {
        val x = GrokBotMotion.ALERT_X
        assertEquals(-8.4f, GrokBotMotion.pl(-1f, x), eps)
        assertEquals(73.57f, GrokBotMotion.pl(1.65f, x), eps)
        assertEquals(10.25f, GrokBotMotion.pl(2.05f, x), eps)
        assertEquals(10.25f, GrokBotMotion.pl(3f, x), eps)
        assertEquals((32.58f + 46.9f) / 2f, GrokBotMotion.alertX(0.85f), 1e-3f)
        assertEquals(46f, GrokBotMotion.pl(0.15f, GrokBotMotion.ORBIT_SPIN_LAG), eps)
        assertEquals(0f, GrokBotMotion.pl(0.6f, GrokBotMotion.ORBIT_SPIN_LAG), eps)
    }

    @Test fun thinkingDotsSwellHoldAndSnapInTurn() {
        // Dot 0 ramps from 0.05 s, holds from 0.40 s to 0.75 s, then snaps back.
        assertEquals(0f, GrokBotMotion.thinkSwell(0.05f, 0), eps)
        assertEquals(Math.pow(0.5, 1.4).toFloat(), GrokBotMotion.thinkSwell(0.225f, 0), 1e-3f)
        assertEquals(1f, GrokBotMotion.thinkSwell(0.6f, 0), eps)
        assertEquals(0f, GrokBotMotion.thinkSwell(0.8f, 0), eps)
        // Each dot starts 0.5 s after the previous one, and the cycle repeats every 1.5 s.
        assertEquals(0f, GrokBotMotion.thinkSwell(0.55f, 1), eps)
        assertEquals(1f, GrokBotMotion.thinkSwell(0.95f, 1), eps)
        assertEquals(GrokBotMotion.thinkSwell(0.1f, 0), GrokBotMotion.thinkSwell(1.6f, 0), 1e-4f)
        assertEquals(GrokBotMotion.thinkSwell(0.3f, 2), GrokBotMotion.thinkSwell(-1.2f, 2), 1e-4f)
    }

    @Test fun orbitRoundsBetween175And235Seconds() {
        assertEquals(0f, GrokBotMotion.orbitRound(1.0f), eps)
        assertEquals(0f, GrokBotMotion.orbitRound(1.75f), eps)
        assertEquals(0.5f, GrokBotMotion.orbitRound(2.05f), 1e-4f)
        assertEquals(1f, GrokBotMotion.orbitRound(2.35f), eps)
        assertEquals(1f, GrokBotMotion.orbitRound(2.8f), eps)
    }

    @Test fun glyphsAreExact64SegmentOutlines() {
        for (g in listOf(GrokBotMotion.GLYPH_ALERT, GrokBotMotion.GLYPH_EXCLAMATION, GrokBotMotion.GLYPH_ORBIT)) {
            assertEquals(386, g.size)
        }
        assertEquals(0, GrokBotMotion.ALERT_DOT_POINTS.size % 2)
        assertEquals(6, GrokBotMotion.RINGS.size)
        assertEquals(4, GrokBotMotion.COMET.size)
    }

    @Test fun ellipseArcPutsTheLeftHalfInFront() {
        val e = GrokEllipse(0f, 0f, 100f, 40f, 0f, 5f, -100f, 0f, 100f, 0f, longArrayOf(0, 0, 0))
        val arc = GrokBotMotion.ellArc(e, 0f, 1f)
        assertEquals(1, arc.front.size)
        assertEquals(2, arc.back.size)
        val slack = 100f * 0.06f // the crossing segment joins the new side
        arc.front.forEach { pts -> for (i in pts.indices step 2) assertTrue(pts[i] < slack) }
        arc.back.forEach { pts -> for (i in pts.indices step 2) assertTrue(pts[i] > -slack) }
        assertTrue(arc.front[0].filterIndexed { i, _ -> i % 2 == 0 }.any { it < -99f })
        assertTrue(GrokBotMotion.ellArc(e, 0.3f, 0.301f).front.isEmpty())
    }

    @Test fun inkMatchesTheWebPalette() {
        assertEquals(0xFF808080.toInt(), GrokBotMotion.mixArgb(0xFF000000.toInt(), 0xFFFFFFFF.toInt(), 0.5f))
        assertEquals(0xFF52525B.toInt(), GrokBotMotion.paletteInk("ink").first)
        assertEquals(0xFF3B93F0.toInt(), GrokBotMotion.paletteInk("blue").first)
    }

    @Test fun everyComplexMoveSettlesBackToTheRestingShape() {
        for (move in GrokBotMotion.ONE_SHOT_MOVES) {
            val dur = GrokBotMotion.duration(move)
            assertTrue(move, dur > 1f)
            val f = GrokBotMotion.frame(move, dur, rest("droplet"), faceEyes, "turquoise")!!
            for (i in f.body.indices) assertEquals(move, rest("droplet")[i], f.body[i], 1e-3f)
            assertEquals(move, 1f, f.eyeOpacity, eps)
        }
        assertEquals(null, GrokBotMotion.frame("wink", 0.5f, rest("circle"), faceEyes, "blue"))
    }

    @Test fun hiddenEyesSlideTogetherAndFadeWithoutSquashing() {
        val start = GrokBotMotion.frame("alert", 0f, rest("droplet"), faceEyes, "blue")!!
        val mid = GrokBotMotion.frame("alert", 0.16f, rest("droplet"), faceEyes, "blue")!!
        assertTrue(mid.eyeOpacity > 0f && mid.eyeOpacity < 1f)
        assertEquals(start.eyes.first.sy, mid.eyes.first.sy, eps)
        assertTrue(abs(mid.eyes.first.x) < abs(start.eyes.first.x))
        // Fully in, the eyes sit at 22 % of their distance from the centre.
        val inFrame = GrokBotMotion.frame("sleep", 1.0f, rest("circle"), faceEyes, "blue")!!
        assertEquals(faceEyes.first.x * 0.22f, inFrame.eyes.first.x, 0.05f)
        assertEquals(0f, inFrame.eyeOpacity, 1e-3f)
    }

    @Test fun alertAndExclamationUseTheExactGlyphsOnEveryShape() {
        val a = GrokBotMotion.frame("alert", 0.9f, rest("droplet"), faceEyes, "blue")!!.body.copyOf()
        val b = GrokBotMotion.frame("alert", 0.9f, rest("circle"), faceEyes, "blue")!!.body
        for (i in a.indices) assertEquals(a[i], b[i], 1e-3f)
        // At 0.9 s the glyph is back at its captured centroid x (46.9).
        var sx = 0f
        for (i in 0 until 386 step 2) sx += a[i] - GrokBotMotion.GLYPH_ALERT[i]
        assertEquals(0f, sx / 193f, 0.6f)
        assertNotNull(GrokBotMotion.frame("alert", 0.9f, rest("circle"), faceEyes, "blue")!!.alertDot)
        val ex = GrokBotMotion.frame("exclamation", 0.9f, rest("droplet"), faceEyes, "blue")!!
        assertEquals(11.3f, ex.dotsFront.single().r, eps)
    }

    @Test fun ringsAndCometLayerBehindAndInFrontLikeTheWeb() {
        val orbit = GrokBotMotion.frame("orbit", 1.2f, rest("droplet"), faceEyes, "blue")!!
        for (ring in GrokBotMotion.RINGS) {
            assertTrue(orbit.backStrokes.any { it.ellipse === ring })
            assertTrue(orbit.frontStrokes.any { it.ellipse === ring })
        }
        val swirl = GrokBotMotion.frame("swirl", 0.6f, rest("droplet"), faceEyes, "blue")!!
        assertTrue(swirl.backStrokes.any { it.ellipse === GrokBotMotion.RINGS[2] })
        assertTrue(swirl.backStrokes.none { it.ellipse === GrokBotMotion.RINGS[3] })

        val out = GrokBotMotion.frame("comet", 0.95f, rest("droplet"), faceEyes, "blue")!!
        assertTrue(out.backStrokes.any { it.ellipse === GrokBotMotion.COMET[0] })
        assertTrue(out.frontStrokes.isEmpty())
        val back = GrokBotMotion.frame("comet", 1.55f, rest("droplet"), faceEyes, "blue")!!
        assertTrue(back.frontStrokes.any { it.ellipse === GrokBotMotion.COMET[3] })
        val done = GrokBotMotion.frame("comet", 2.1f, rest("droplet"), faceEyes, "blue")!!
        assertTrue(done.backStrokes.isEmpty() && done.frontStrokes.isEmpty())
    }

    @Test fun orbitEyesResizeIntoTallerPillsAsTheTriangleRounds() {
        val spin = GrokBotMotion.frame("orbit", 1.0f, rest("circle"), faceEyes, "blue")!!
        assertEquals(9f + 8f, spin.eyes.first.pill.halfH, 1e-3f)
        val round = GrokBotMotion.frame("orbit", 2.4f, rest("circle"), faceEyes, "blue")!!
        assertEquals(9f + 11.5f, round.eyes.first.pill.halfH, 1e-3f)
        // Rounded fully, the body is the unit circle.
        val circle = rest("circle")
        val r = hypot(round.body[0], round.body[1])
        assertEquals(hypot(circle[0], circle[1]), r, 0.5f)
    }

    @Test fun moveTilesAreStrokeFreeStills() {
        for (move in GrokBotMotion.ONE_SHOT_MOVES) {
            val f = GrokBotMotion.frame(move, GrokBotMotion.previewTime(move), rest("circle"), faceEyes, "blue", tile = true)!!
            assertTrue(move, f.backStrokes.isEmpty() && f.frontStrokes.isEmpty())
        }
        val think = GrokBotMotion.frame("thinking", GrokBotMotion.previewTime("thinking"), rest("circle"), faceEyes, "blue", tile = true)!!
        assertEquals(2, think.dotsFront.size)
        assertEquals(0f, think.eyeOpacity, eps)
        // Burst particles sit behind the body, three of them mid-move, as on the reference tile.
        val burst = GrokBotMotion.frame("burst", GrokBotMotion.previewTime("burst"), rest("droplet"), faceEyes, "blue", tile = true)!!
        assertEquals(3, burst.dotsBack.size)
    }
}
