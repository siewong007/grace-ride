package org.pjstatecm.graceride.ui.screens.traffic

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.net.Uri
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.pjstatecm.graceride.data.api.PlanDataDto
import org.pjstatecm.graceride.domain.EffectiveVisitor
import org.pjstatecm.graceride.domain.KlTime
import java.io.File
import java.io.FileOutputStream

/**
 * Renders the event transport board into a clean shareable PNG matching web `planImageExport.ts`
 * and exposes it through the app's FileProvider (`shared_plans/`).
 */
object PlanImageExporter {

    suspend fun exportAndCreateShareUri(
        context: Context,
        plan: PlanDataDto,
        assign: Map<Long, Long?>,
        visitors: List<EffectiveVisitor>,
        outboundLabel: String,
        returnLabel: String,
        unassignedLabel: String,
    ): Uri = withContext(Dispatchers.Default) {
        val width = 1080
        val pad = 44f
        val lineH = 38f
        val cardGap = 24f

        val driversById = plan.drivers.associateBy { it.id }
        val activeVisitors = visitors.filter { it.draftOp != "delete" }
        val ridesByDriver = plan.rides.groupBy { assign[it.id] }
        val visitorsByDriver = activeVisitors.groupBy { it.driverId }

        val driverIds = (ridesByDriver.keys.filterNotNull() + visitorsByDriver.keys.filterNotNull()).distinct()
        val unassignedRides = ridesByDriver[null].orEmpty()
        val unassignedVisitors = visitorsByDriver[null].orEmpty()

        // Measure total height before allocating bitmap.
        var estHeight = 200f
        driverIds.forEach { dId ->
            val rCount = ridesByDriver[dId].orEmpty().size
            val vCount = visitorsByDriver[dId].orEmpty().size
            estHeight += 96f + (rCount + vCount).coerceAtLeast(1) * lineH + cardGap
        }
        if (unassignedRides.isNotEmpty() || unassignedVisitors.isNotEmpty()) {
            estHeight += 96f + (unassignedRides.size + unassignedVisitors.size) * lineH + cardGap
        }
        val height = estHeight.toInt().coerceIn(520, 6400)

        val bmp = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(bmp)
        canvas.drawColor(0xFFF6F7FB.toInt())

        val titlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFF0F172A.toInt()
            textSize = 42f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
        }
        val subPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFF475569.toInt()
            textSize = 26f
        }
        val cardFill = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFFFFFFFF.toInt()
            style = Paint.Style.FILL
        }
        val cardStroke = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFFE2E8F0.toInt()
            style = Paint.Style.STROKE
            strokeWidth = 2f
        }
        val hdrPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFF1E40AF.toInt()
            textSize = 30f
            typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
        }
        val rowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFF1E293B.toInt()
            textSize = 25f
        }

        var y = 64f
        canvas.drawText(plan.event.title, pad, y, titlePaint)
        y += 42f
        val subtitle = "${KlTime.display(plan.event.startsAt)}${if (plan.event.location.isNotBlank()) " · ${plan.event.location}" else ""}"
        canvas.drawText(subtitle, pad, y, subPaint)
        y += 38f

        fun drawGroup(header: String, rows: List<String>) {
            val boxTop = y
            val boxH = 68f + rows.size.coerceAtLeast(1) * lineH + 16f
            val rect = RectF(pad, boxTop, width - pad, boxTop + boxH)
            canvas.drawRoundRect(rect, 18f, 18f, cardFill)
            canvas.drawRoundRect(rect, 18f, 18f, cardStroke)
            canvas.drawText(header, pad + 24f, boxTop + 44f, hdrPaint)
            var ry = boxTop + 82f
            for (line in rows) {
                canvas.drawText(line.take(66), pad + 24f, ry, rowPaint)
                ry += lineH
            }
            y += boxH + cardGap
        }

        for (dId in driverIds) {
            val drv = driversById[dId]
            val rides = ridesByDriver[dId].orEmpty()
            val vis = visitorsByDriver[dId].orEmpty()
            val header = "${drv?.name ?: "#$dId"}${drv?.vehicle?.plate?.let { " · $it" } ?: ""}"
            val lines = buildList {
                rides.forEach { r ->
                    val legTag = if (r.leg == "return") returnLabel else outboundLabel
                    add("[$legTag] ${r.passenger.name} (${r.seats}) · ${r.area.name}")
                }
                vis.forEach { v ->
                    val legTag = if (v.leg == "return") returnLabel else outboundLabel
                    add("[$legTag] ${v.name} (${v.seats}) · ${v.areaName.orEmpty()}")
                }
            }
            drawGroup(header, lines)
        }

        if (unassignedRides.isNotEmpty() || unassignedVisitors.isNotEmpty()) {
            val lines = buildList {
                unassignedRides.forEach { r ->
                    val legTag = if (r.leg == "return") returnLabel else outboundLabel
                    add("[$legTag] ${r.passenger.name} (${r.seats}) · ${r.area.name}")
                }
                unassignedVisitors.forEach { v ->
                    val legTag = if (v.leg == "return") returnLabel else outboundLabel
                    add("[$legTag] ${v.name} (${v.seats}) · ${v.areaName.orEmpty()}")
                }
            }
            drawGroup(unassignedLabel, lines)
        }

        withContext(Dispatchers.IO) {
            val dir = File(context.cacheDir, "shared_plans").apply { mkdirs() }
            val outFile = File(dir, "plan-${plan.event.id}.png")
            FileOutputStream(outFile).use { stream ->
                bmp.compress(Bitmap.CompressFormat.PNG, 95, stream)
            }
            bmp.recycle()
            FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", outFile)
        }
    }

    fun launchShareIntent(context: Context, uri: Uri, title: String) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "image/png"
            putExtra(Intent.EXTRA_STREAM, uri)
            putExtra(Intent.EXTRA_SUBJECT, title)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        context.startActivity(Intent.createChooser(intent, title))
    }
}
