package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.provider.OpenableColumns
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.Locale

internal const val MAX_ATTACHMENT_BYTES = 8 * 1024 * 1024
private const val PHOTO_MAX_PIXELS = 4_000_000
private val PHOTO_COMPRESS_QUALITIES = intArrayOf(88, 78, 68, 58, 48, 38)

internal fun displayNameForUri(context: Context, uri: Uri): String? {
    return runCatching {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0 && cursor.moveToFirst()) cursor.getString(index) else null
        }
    }.getOrNull()?.takeIf { it.isNotBlank() }
}

internal fun copyAttachmentToCache(
    context: Context,
    displayLabel: String,
    uri: Uri,
    displayName: String,
    attachmentIndex: Int,
    maxBytes: Int = MAX_ATTACHMENT_BYTES
): PendingAttachment {
    require(maxBytes in 1..ChatImageDiskCache.MAX_IMAGE_BYTES)
    val mimeType = (context.contentResolver.getType(uri) ?: guessMimeType(displayName))
        .lowercase(Locale.CHINA)
    val extension = extensionForAttachment(displayName, mimeType)
    val attachmentDir = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
    val target = File.createTempFile("attachment_${attachmentIndex}_", ".$extension", attachmentDir)
    var total = 0L
    try {
        context.contentResolver.openInputStream(uri).use { input ->
            requireNotNull(input) { "Cannot open selected file" }
            target.outputStream().use { output ->
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                while (true) {
                    val read = input.read(buffer)
                    if (read <= 0) break
                    total += read
                    if (total > maxBytes) throw AttachmentSizeLimitException()
                    output.write(buffer, 0, read)
                }
            }
        }
        require(total > 0) { "Empty attachment" }
    } catch (error: Exception) {
        target.delete()
        if (error is AttachmentSizeLimitException && isStaticPhotoAttachment(mimeType)) {
            return normalizePhotoAttachmentToCache(context, "图片（已压缩）", uri, displayName, attachmentIndex, mimeType, maxBytes)
        }
        throw error
    }
    // The upload retains the encoded original, including PNG text edges and JPEG/EXIF metadata.
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    if (mimeType.startsWith("image/")) {
        BitmapFactory.decodeFile(target.path, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) {
            target.delete()
            throw IllegalArgumentException("Cannot decode selected photo")
        }
    }
    return PendingAttachment(
        kind = normalizedAttachmentKind(mimeType),
        displayLabel = displayLabel,
        displayName = displayName,
        fileName = target.name,
        mimeType = mimeType,
        file = target,
        imageWidth = bounds.outWidth.takeIf { it > 0 },
        imageHeight = bounds.outHeight.takeIf { it > 0 }
    )
}

private class AttachmentSizeLimitException : IllegalArgumentException("Attachment too large")

private fun isStaticPhotoAttachment(mimeType: String): Boolean {
    return mimeType in setOf("image/jpeg", "image/png", "image/webp")
}

private fun normalizePhotoAttachmentToCache(
    context: Context,
    displayLabel: String,
    uri: Uri,
    displayName: String,
    attachmentIndex: Int,
    sourceMimeType: String,
    maxBytes: Int
): PendingAttachment {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "Cannot open selected photo" }
        BitmapFactory.decodeStream(input, null, bounds)
    }
    require(bounds.outWidth > 0 && bounds.outHeight > 0) { "Cannot decode selected photo" }

    val decodeOptions = BitmapFactory.Options().apply {
        inSampleSize = photoSampleSize(bounds.outWidth, bounds.outHeight)
    }
    val bitmap = context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "Cannot open selected photo" }
        BitmapFactory.decodeStream(input, null, decodeOptions)
    } ?: throw IllegalArgumentException("Cannot decode selected photo")

    val normalized = try { normalizedPhotoBytes(bitmap, sourceMimeType, maxBytes) }
        catch (error: Exception) { bitmap.recycle(); throw error }
    val finalBytes = normalized.bytes
    val width = bitmap.width
    val height = bitmap.height
    bitmap.recycle()
    require(finalBytes.size <= maxBytes) { "Compressed photo is still too large" }

    val safeName = displayName.substringBeforeLast('.', displayName).ifBlank { "photo" }
    val attachmentDir = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
    val fileName = "attachment_${System.currentTimeMillis()}_$attachmentIndex.${normalized.extension}"
    val target = File(attachmentDir, fileName)
    target.writeBytes(finalBytes)
    return PendingAttachment(
        kind = "image",
        displayLabel = displayLabel,
        displayName = "$safeName.${normalized.extension}",
        fileName = fileName,
        mimeType = normalized.mimeType,
        file = target,
        imageWidth = width,
        imageHeight = height
    )
}

private data class NormalizedPhotoBytes(
    val bytes: ByteArray,
    val mimeType: String,
    val extension: String
)

private fun normalizedPhotoBytes(bitmap: Bitmap, sourceMimeType: String, maxBytes: Int): NormalizedPhotoBytes {
    val pngBytes = if (sourceMimeType == "image/png") {
        ByteArrayOutputStream().use { output ->
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)
            output.toByteArray().takeIf { it.size <= maxBytes }
        }
    } else {
        null
    }
    if (pngBytes != null) {
        return NormalizedPhotoBytes(pngBytes, "image/png", "png")
    }

    val bytes = ByteArrayOutputStream()
    for (quality in PHOTO_COMPRESS_QUALITIES) {
        bytes.reset()
        bitmap.compress(Bitmap.CompressFormat.JPEG, quality, bytes)
        if (bytes.size() <= maxBytes) {
            return NormalizedPhotoBytes(bytes.toByteArray(), "image/jpeg", "jpg")
        }
    }
    return NormalizedPhotoBytes(bytes.toByteArray(), "image/jpeg", "jpg")
}

private fun normalizedAttachmentKind(mimeType: String): String {
    return if (mimeType.startsWith("image/")) "image" else "file"
}

private fun photoSampleSize(width: Int, height: Int): Int {
    var sample = 1
    while ((width / sample).toLong() * (height / sample).toLong() > PHOTO_MAX_PIXELS) {
        sample *= 2
    }
    return sample
}

internal fun guessMimeType(name: String): String {
    return when (name.substringAfterLast('.', "").lowercase(Locale.CHINA)) {
        "jpg", "jpeg" -> "image/jpeg"
        "png" -> "image/png"
        "webp" -> "image/webp"
        "gif" -> "image/gif"
        "pdf" -> "application/pdf"
        "txt" -> "text/plain"
        "m4a", "mp4" -> "audio/mp4"
        "aac" -> "audio/aac"
        "wav" -> "audio/wav"
        "mp3" -> "audio/mpeg"
        else -> "application/octet-stream"
    }
}

internal fun extensionForAttachment(name: String, mimeType: String): String {
    val fromName = name.substringAfterLast('.', "").lowercase(Locale.CHINA)
        .filter { it.isLetterOrDigit() }
        .take(8)
    if (fromName.isNotBlank()) return fromName
    return when (mimeType) {
        "image/jpeg" -> "jpg"
        "image/png" -> "png"
        "image/webp" -> "webp"
        "image/gif" -> "gif"
        "application/pdf" -> "pdf"
        "text/plain" -> "txt"
        "audio/mp4" -> "m4a"
        "audio/aac" -> "aac"
        "audio/wav" -> "wav"
        "audio/mpeg" -> "mp3"
        else -> "bin"
    }
}
