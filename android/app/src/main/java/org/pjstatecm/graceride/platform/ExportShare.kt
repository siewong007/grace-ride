package org.pjstatecm.graceride.platform

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Shared FileProvider export + RFC 4180 CSV builder matching `frontend/src/fareSheet.ts`
 * formula-injection neutralization and UTF-8 BOM output.
 */
object ExportShare {

    suspend fun writeAndShareFile(
        context: Context,
        filename: String,
        mimeType: String,
        bytes: ByteArray,
        chooserTitle: String? = null,
        subdir: String = "shared_exports",
    ): Boolean {
        val safeName = filename.replace(Regex("[^A-Za-z0-9._-]"), "_").ifBlank { "export.dat" }
        val file = withContext(Dispatchers.IO) {
            val dir = File(context.cacheDir, subdir).apply { mkdirs() }
            File(dir, safeName).apply { writeBytes(bytes) }
        }
        val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
        val send = Intent(Intent.ACTION_SEND).apply {
            type = mimeType
            putExtra(Intent.EXTRA_STREAM, uri)
            putExtra(Intent.EXTRA_SUBJECT, safeName)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (context !is android.app.Activity) {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
        }
        val chooser = Intent.createChooser(send, chooserTitle ?: safeName).apply {
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            if (context !is android.app.Activity) {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
        }
        return try {
            context.startActivity(chooser)
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }
}

object CsvBuilder {

    fun escapeCell(raw: String): String {
        val trimmed = raw.trimStart()
        val neutralized = if (
            raw.startsWith('\t') ||
            raw.startsWith('\r') ||
            trimmed.startsWith('=') ||
            trimmed.startsWith('+') ||
            trimmed.startsWith('-') ||
            trimmed.startsWith('@') ||
            trimmed.startsWith('|') ||
            trimmed.startsWith('%')
        ) {
            "'$raw"
        } else {
            raw
        }
        return if (
            neutralized.contains(',') ||
            neutralized.contains('"') ||
            neutralized.contains('\n') ||
            neutralized.contains('\r')
        ) {
            "\"${neutralized.replace("\"", "\"\"")}\""
        } else {
            neutralized
        }
    }

    fun build(headers: List<String>, rows: List<List<String>>): String = buildString {
        append('\uFEFF')
        append(headers.joinToString(",") { escapeCell(it) })
        append("\r\n")
        for (row in rows) {
            append(row.joinToString(",") { escapeCell(it) })
            append("\r\n")
        }
    }

    fun buildBytes(headers: List<String>, rows: List<List<String>>): ByteArray =
        build(headers, rows).toByteArray(Charsets.UTF_8)
}
