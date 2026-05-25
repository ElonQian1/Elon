package com.elon.app

import android.content.Context
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
    val mimeType = context.contentResolver.getType(uri) ?: guessMimeType(displayName)
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
                    if (isPhotoAttachment(mimeType)) {
                        return compressPhotoAttachmentToCache(
                            context,
                            displayLabel,
                            uri,
                            displayName,
                            attachmentIndex
                        )
                    }
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

private fun isPhotoAttachment(mimeType: String): Boolean {
    return mimeType.startsWith("image/")
}

private fun compressPhotoAttachmentToCache(
    context: Context,
    displayLabel: String,
    uri: Uri,
    displayName: String,
    attachmentIndex: Int
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

    val bytes = ByteArrayOutputStream()
    var selectedBytes: ByteArray? = null
    for (quality in PHOTO_COMPRESS_QUALITIES) {
        bytes.reset()
        bitmap.compress(android.graphics.Bitmap.CompressFormat.JPEG, quality, bytes)
        if (bytes.size() <= MAX_ATTACHMENT_BYTES) {
            selectedBytes = bytes.toByteArray()
            break
        }
    }
    val finalBytes = selectedBytes ?: bytes.toByteArray()
    bitmap.recycle()
    require(finalBytes.size <= MAX_ATTACHMENT_BYTES) { "Compressed photo is still too large" }

    val safeName = displayName.substringBeforeLast('.', displayName).ifBlank { "photo" }
    val attachmentDir = File(context.cacheDir, "pending_attachments").apply { mkdirs() }
    val fileName = "attachment_${System.currentTimeMillis()}_$attachmentIndex.jpg"
    val target = File(attachmentDir, fileName)
    target.writeBytes(finalBytes)
    return PendingAttachment(
        kind = "image",
        displayLabel = displayLabel,
        displayName = "$safeName.jpg",
        fileName = fileName,
        mimeType = "image/jpeg",
        file = target
    )
}

private fun normalizedAttachmentKind(mimeType: String): String {
    return if (mimeType.startsWith("image/")) "image" else "file"
}

private fun photoSampleSize(width: Int, height: Int): Int {
    var sample = 1
    while ((width / sample) * (height / sample) > PHOTO_MAX_PIXELS) {
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
