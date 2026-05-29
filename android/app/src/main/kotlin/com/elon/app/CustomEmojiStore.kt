package com.elon.app

import android.content.Context
import android.graphics.BitmapFactory
import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.Locale

internal data class CustomEmojiItem(
    val id: String,
    val displayName: String,
    val fileName: String,
    val mimeType: String,
    val imageWidth: Int?,
    val imageHeight: Int?,
    val addedAt: Long
)

internal object CustomEmojiStore {
    private const val KEY_CUSTOM_EMOJIS = "custom_emojis_json"
    private const val MAX_SAVED_EMOJIS = 120

    fun load(context: Context): List<CustomEmojiItem> {
        val raw = AuthManager.userDataPrefs(context).getString(KEY_CUSTOM_EMOJIS, null) ?: return emptyList()
        return runCatching {
            val array = JSONArray(raw)
            List(array.length()) { index -> array.optJSONObject(index) }
                .mapNotNull { it?.toCustomEmojiItem() }
                .filter { fileFor(context, it.fileName).isFile }
        }.getOrDefault(emptyList())
    }

    fun import(context: Context, uri: Uri, displayName: String): CustomEmojiItem {
        val mimeType = (context.contentResolver.getType(uri) ?: guessMimeType(displayName))
            .lowercase(Locale.CHINA)
        require(mimeType.startsWith("image/")) { "Only image custom emoji is supported" }
        val id = "emoji_${System.currentTimeMillis()}_${load(context).size}"
        val extension = extensionForAttachment(displayName, mimeType)
        val fileName = "$id.$extension"
        val target = fileFor(context, fileName)
        target.parentFile?.mkdirs()
        copyUriToFile(context, uri, target)
        val size = imageSize(target)
        val item = CustomEmojiItem(
            id = id,
            displayName = displayName.take(32).ifBlank { "自定义表情" },
            fileName = fileName,
            mimeType = mimeType,
            imageWidth = size.first,
            imageHeight = size.second,
            addedAt = System.currentTimeMillis()
        )
        save(context, (listOf(item) + load(context)).take(MAX_SAVED_EMOJIS))
        return item
    }

    fun remove(context: Context, id: String) {
        val current = load(context)
        current.firstOrNull { it.id == id }?.let { fileFor(context, it.fileName).delete() }
        save(context, current.filterNot { it.id == id })
    }

    fun toPendingAttachment(context: Context, item: CustomEmojiItem, index: Int): PendingAttachment? {
        val source = fileFor(context, item.fileName).takeIf { it.isFile } ?: return null
        val extension = extensionForAttachment(item.displayName, item.mimeType)
        val fileName = "emoji_${System.currentTimeMillis()}_$index.$extension"
        val target = File(context.cacheDir, "pending_attachments").apply { mkdirs() }.resolve(fileName)
        source.copyTo(target, overwrite = true)
        return PendingAttachment(
            kind = "image",
            displayLabel = "表情",
            displayName = item.displayName,
            fileName = fileName,
            mimeType = item.mimeType,
            file = target,
            imageWidth = item.imageWidth,
            imageHeight = item.imageHeight
        )
    }

    fun thumbnail(context: Context, item: CustomEmojiItem) = runCatching {
        val source = fileFor(context, item.fileName)
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(source.absolutePath, bounds)
        val target = (96 * context.resources.displayMetrics.density).toInt()
        val options = BitmapFactory.Options().apply {
            inSampleSize = thumbnailSampleSize(bounds.outWidth, bounds.outHeight, target)
        }
        BitmapFactory.decodeFile(source.absolutePath, options)
    }.getOrNull()

    private fun copyUriToFile(context: Context, uri: Uri, target: File) {
        var total = 0L
        context.contentResolver.openInputStream(uri).use { input ->
            requireNotNull(input) { "Cannot open custom emoji" }
            target.outputStream().use { output ->
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                while (true) {
                    val read = input.read(buffer)
                    if (read <= 0) break
                    total += read
                    if (total > MAX_ATTACHMENT_BYTES) {
                        target.delete()
                        throw IllegalArgumentException("Custom emoji too large")
                    }
                    output.write(buffer, 0, read)
                }
            }
        }
    }

    private fun save(context: Context, items: List<CustomEmojiItem>) {
        val array = JSONArray()
        items.forEach { array.put(it.toJson()) }
        AuthManager.userDataPrefs(context).edit().putString(KEY_CUSTOM_EMOJIS, array.toString()).apply()
    }

    private fun fileFor(context: Context, fileName: String): File {
        return File(context.filesDir, "custom_emojis/$fileName")
    }

    private fun imageSize(file: File): Pair<Int?, Int?> {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(file.absolutePath, bounds)
        return bounds.outWidth.takeIf { it > 0 } to bounds.outHeight.takeIf { it > 0 }
    }

    private fun thumbnailSampleSize(width: Int, height: Int, target: Int): Int {
        if (width <= 0 || height <= 0 || target <= 0) return 1
        var sample = 1
        while ((width / sample) > target || (height / sample) > target) {
            sample *= 2
        }
        return sample
    }

    private fun JSONObject.toCustomEmojiItem(): CustomEmojiItem? {
        val id = optString("id").takeIf { it.isNotBlank() } ?: return null
        val fileName = optString("file_name").takeIf { it.isNotBlank() } ?: return null
        return CustomEmojiItem(
            id = id,
            displayName = optString("display_name").takeIf { it.isNotBlank() } ?: "自定义表情",
            fileName = fileName,
            mimeType = optString("mime_type").takeIf { it.isNotBlank() } ?: guessMimeType(fileName),
            imageWidth = optInt("image_width", 0).takeIf { it > 0 },
            imageHeight = optInt("image_height", 0).takeIf { it > 0 },
            addedAt = optLong("added_at", 0L)
        )
    }

    private fun CustomEmojiItem.toJson(): JSONObject {
        return JSONObject()
            .put("id", id)
            .put("display_name", displayName)
            .put("file_name", fileName)
            .put("mime_type", mimeType)
            .put("image_width", imageWidth ?: 0)
            .put("image_height", imageHeight ?: 0)
            .put("added_at", addedAt)
    }
}
