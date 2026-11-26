package org.pjstatecm.graceride.data.media

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.net.Uri
import java.io.ByteArrayOutputStream

data class PreparedUpload(
    val filename: String,
    val mime: String,
    val bytes: ByteArray,
)

const val MAX_UPLOAD_BYTES = 8 * 1024 * 1024

class FileTooLargeException : IllegalStateException("File exceeds 8 MB limit")

private fun readBounded(stream: java.io.InputStream, maxBytes: Int): ByteArray {
    val out = ByteArrayOutputStream()
    val buf = ByteArray(16 * 1024)
    var total = 0
    while (true) {
        val n = stream.read(buf)
        if (n <= 0) break
        total += n
        if (total > maxBytes) throw FileTooLargeException()
        out.write(buf, 0, n)
    }
    return out.toByteArray()
}

/**
 * Decodes, EXIF-orients, and downscales user-selected images via native `ImageDecoder`
 * (API 28+) in memory before streaming to OkHttp, avoiding multi-megabyte temp-file copies
 * and enforcing the backend 8 MiB upload limit.
 */
fun prepareImageUpload(
    context: Context,
    uri: Uri,
    prefix: String,
    maxDimensionPx: Int,
    quality: Int = 85,
): PreparedUpload {
    val resolver = context.contentResolver
    val resolvedMime = resolver.getType(uri) ?: "image/jpeg"

    // Preserve animated GIFs verbatim, bounded at 8 MiB.
    if (resolvedMime.equals("image/gif", ignoreCase = true)) {
        val raw = resolver.openInputStream(uri)?.use { readBounded(it, MAX_UPLOAD_BYTES) }
            ?: throw IllegalStateException("Cannot read selected image")
        return PreparedUpload(filename = "$prefix.gif", mime = "image/gif", bytes = raw)
    }

    val decoded = runCatching {
        val source = ImageDecoder.createSource(resolver, uri)
        ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
            // Software bitmap is required for Bitmap.compress(...) across all devices.
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            val srcW = info.size.width
            val srcH = info.size.height
            val longest = maxOf(srcW, srcH)
            if (longest > maxDimensionPx && longest > 0) {
                val scale = maxDimensionPx.toFloat() / longest.toFloat()
                val targetW = (srcW * scale).toInt().coerceAtLeast(1)
                val targetH = (srcH * scale).toInt().coerceAtLeast(1)
                decoder.setTargetSize(targetW, targetH)
            }
        }
    }.getOrNull()

    if (decoded != null) {
        try {
            val keepPng = decoded.hasAlpha() && (
                resolvedMime.equals("image/png", ignoreCase = true) ||
                    resolvedMime.equals("image/webp", ignoreCase = true)
                )
            if (keepPng) {
                val out = ByteArrayOutputStream(32 * 1024)
                if (!decoded.compress(Bitmap.CompressFormat.PNG, 100, out)) {
                    throw IllegalStateException("Image compression failed")
                }
                val bytes = out.toByteArray()
                if (bytes.size <= MAX_UPLOAD_BYTES) {
                    return PreparedUpload(filename = "$prefix.png", mime = "image/png", bytes = bytes)
                }
            }
            var q = quality.coerceIn(45, 95)
            while (q >= 45) {
                val out = ByteArrayOutputStream(32 * 1024)
                if (!decoded.compress(Bitmap.CompressFormat.JPEG, q, out)) {
                    throw IllegalStateException("Image compression failed")
                }
                val bytes = out.toByteArray()
                if (bytes.size <= MAX_UPLOAD_BYTES) {
                    return PreparedUpload(filename = "$prefix.jpg", mime = "image/jpeg", bytes = bytes)
                }
                q -= 15
            }
            throw FileTooLargeException()
        } finally {
            decoded.recycle()
        }
    }

    throw java.io.IOException("Unable to sanitize image upload without EXIF metadata stripping")
}
