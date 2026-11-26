package org.pjstatecm.graceride.data.media

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ImageDecoder
import android.net.Uri
import androidx.core.graphics.createBitmap
import java.io.ByteArrayOutputStream
import java.io.IOException

data class PreparedUpload(
    val filename: String,
    val mime: String,
    val bytes: ByteArray,
)

const val MAX_UPLOAD_BYTES = 8 * 1024 * 1024

class FileTooLargeException : IllegalStateException("File exceeds 8 MB limit")

private fun readHeader(stream: java.io.InputStream, len: Int): ByteArray {
    val buf = ByteArray(len)
    var off = 0
    while (off < len) {
        val n = stream.read(buf, off, len - off)
        if (n <= 0) break
        off += n
    }
    return buf.copyOf(off)
}

private fun isGifMagic(bytes: ByteArray): Boolean {
    if (bytes.size < 6) return false
    val sig = String(bytes, 0, 6, Charsets.US_ASCII)
    return sig == "GIF87a" || sig == "GIF89a"
}

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
    require(maxDimensionPx > 0) { "maxDimensionPx must be positive" }
    val resolver = context.contentResolver

    // Preserve verified animated GIFs verbatim, bounded at 8 MiB.
    val header = resolver.openInputStream(uri)?.use { readHeader(it, 6) }
        ?: throw IllegalStateException("Cannot read selected image")
    if (isGifMagic(header)) {
        val raw = resolver.openInputStream(uri)?.use { readBounded(it, MAX_UPLOAD_BYTES) }
            ?: throw IllegalStateException("Cannot read selected image")
        if (isGifMagic(raw)) {
            return PreparedUpload(filename = "$prefix.gif", mime = "image/gif", bytes = raw)
        }
    }

    val decoded = try {
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
    } catch (e: Exception) {
        throw IOException("Unable to sanitize image upload without EXIF metadata stripping", e)
    }

    try {
        val hasAlpha = decoded.hasAlpha()
        if (hasAlpha) {
            val out = ByteArrayOutputStream(32 * 1024)
            if (!decoded.compress(Bitmap.CompressFormat.PNG, 100, out)) {
                throw IllegalStateException("Image compression failed")
            }
            val bytes = out.toByteArray()
            if (bytes.size <= MAX_UPLOAD_BYTES) {
                return PreparedUpload(filename = "$prefix.png", mime = "image/png", bytes = bytes)
            }
        }
        val jpegSource = if (hasAlpha) {
            createBitmap(decoded.width, decoded.height).also { flat ->
                val canvas = Canvas(flat)
                canvas.drawColor(Color.WHITE)
                canvas.drawBitmap(decoded, 0f, 0f, null)
            }
        } else {
            decoded
        }
        try {
            var q = quality.coerceIn(45, 95)
            while (q >= 45) {
                val out = ByteArrayOutputStream(32 * 1024)
                if (!jpegSource.compress(Bitmap.CompressFormat.JPEG, q, out)) {
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
            if (jpegSource !== decoded) {
                jpegSource.recycle()
            }
        }
    } finally {
        decoded.recycle()
    }
}
