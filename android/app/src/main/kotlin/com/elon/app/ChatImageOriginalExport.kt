package com.elon.app

import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.os.Build
import android.os.Environment
import android.provider.MediaStore

internal object ChatImageOriginalExport {
    /** Copy encoded bytes, never save the decoded screen/thumbnail bitmap. */
    fun save(context: Context, source: String, attachment: ChatAttachment): Intent? {
        if (Build.VERSION.SDK_INT < 29) {
            val files = ChatAttachmentExport.prepare(context, listOf(attachment))
            require(files.isNotEmpty())
            return Intent.createChooser(ChatAttachmentExport.intent(context, files, ""), "保存原图")
        }
        ChatImageDiskCache.acquire(context, source).use { lease ->
            val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeFile(lease.file.path, bounds)
            require(bounds.outWidth > 0 && bounds.outHeight > 0)
            val mime = requireNotNull(bounds.outMimeType)
            val extension = when (mime) {
                "image/png" -> "png"
                "image/webp" -> "webp"
                "image/gif" -> "gif"
                else -> "jpg"
            }
            val name = attachment.displayName.orEmpty().substringBeforeLast('.').take(80)
                .replace(Regex("[^\\p{L}\\p{N}._ -]"), "_").ifBlank { "image_${System.currentTimeMillis()}" }
            val values = ContentValues().apply {
                put(MediaStore.Images.Media.DISPLAY_NAME, "$name.$extension")
                put(MediaStore.Images.Media.MIME_TYPE, mime)
                put(MediaStore.Images.Media.RELATIVE_PATH, "${Environment.DIRECTORY_PICTURES}/Yilong")
                put(MediaStore.Images.Media.IS_PENDING, 1)
            }
            val resolver = context.contentResolver
            val uri = requireNotNull(resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, values))
            try {
                requireNotNull(resolver.openOutputStream(uri)).use { output ->
                    lease.file.inputStream().use { it.copyTo(output) }
                }
                resolver.update(uri, ContentValues().apply { put(MediaStore.Images.Media.IS_PENDING, 0) }, null, null)
            } catch (error: Exception) {
                resolver.delete(uri, null, null)
                throw error
            }
        }
        return null
    }
}
