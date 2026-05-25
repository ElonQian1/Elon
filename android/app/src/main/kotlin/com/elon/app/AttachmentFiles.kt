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
    attachmentIndex: Int
): PendingAttachment {
    val mimeType = (context.contentResolver.getType(uri) ?: guessMimeType(displayName))
        .lowercase(Locale.CHINA)
    if (isStaticPhotoAttachment(mimeType)) {
        return normalizePhotoAttachmentToCache(
            context,
            displayLabel,
            uri,
            displayName,
            attachmentIndex,
            mimeType
        )
    }

    val extension = extensionForAttachment(displayName, mimeType)
    val fileName = "attachment_${System.currentTimeMillis()}_$attachmentIndex.$extension"
    val attachmentDir = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
    val target = File(attachmentDir, fileName)
    var total = 0L
    context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "Cannot open selected file" }
        target.outputStream().use { output ->
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
            while (true) {
                val read = input.read(buffer)
                if (read <= 0) break
                total += read
                if (total > MAX_ATTACHMENT_BYTES) {
                    target.delete()
                    throw IllegalArgumentException("Attachment too large")
                }
                output.write(buffer, 0, read)
            }
        }
    }
    return PendingAttachment(
        kind = normalizedAttachmentKind(mimeType),
        displayLabel = displayLabel,
        displayName = displayName,
        fileName = fileName,
        mimeType = mimeType,
        file = target
    )
}

private fun isStaticPhotoAttachment(mimeType: String): Boolean {
    return mimeType in setOf("image/jpeg", "image/png", "image/webp")
}

private fun normalizePhotoAttachmentToCache(
    context: Context,
    displayLabel: String,
    uri: Uri,
    displayName: String,
    attachmentIndex: Int,
    sourceMimeType: String
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

    val normalized = normalizedPhotoBytes(bitmap, sourceMimeType)
    val finalBytes = normalized.bytes
    val width = bitmap.width
    val height = bitmap.height
    bitmap.recycle()
    require(finalBytes.size <= MAX_ATTACHMENT_BYTES) { "Compressed photo is still too large" }

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

private fun normalizedPhotoBytes(bitmap: Bitmap, sourceMimeType: String): NormalizedPhotoBytes {
    val pngBytes = if (sourceMimeType == "image/png") {
        ByteArrayOutputStream().use { output ->
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, output)
            output.toByteArray().takeIf { it.size <= MAX_ATTACHMENT_BYTES }
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
        if (bytes.size() <= MAX_ATTACHMENT_BYTES) {
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
        else -> "bin"
    }
}
