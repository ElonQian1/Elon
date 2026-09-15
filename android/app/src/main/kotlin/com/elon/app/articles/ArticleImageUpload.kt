package com.elon.app.articles

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.util.Base64
import org.json.JSONObject
import java.io.ByteArrayOutputStream

internal fun uploadArticleImage(context: Context, uri: Uri, api: ArticleApi): JSONObject {
    check(context.contentResolver.getType(uri) in setOf("image/png", "image/jpeg", "image/webp")) { "请选择PNG、JPEG或WebP图片" }
    val bytes = context.contentResolver.openInputStream(uri)?.use { stream ->
        val buffer = ByteArrayOutputStream(); val chunk = ByteArray(8192)
        while (true) { val count = stream.read(chunk); if (count < 0) break; buffer.write(chunk, 0, count); check(buffer.size() <= 20 * 1024 * 1024) { "请选择小于20MB的图片" } }
        buffer.toByteArray()
    } ?: error("无法读取图片")
    val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options)
    check(options.outWidth > 0 && options.outHeight > 0) { "图片格式无法读取" }
    var sample = 1
    while (maxOf(options.outWidth, options.outHeight) / sample > 1800) sample *= 2
    val source = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = sample }) ?: error("图片无法解码")
    val orientation = runCatching { android.media.ExifInterface(java.io.ByteArrayInputStream(bytes)).getAttributeInt(android.media.ExifInterface.TAG_ORIENTATION, 1) }.getOrDefault(1)
    val matrix = android.graphics.Matrix().apply {
        when (orientation) {
            2 -> setScale(-1f, 1f); 3 -> setRotate(180f); 4 -> { setRotate(180f); postScale(-1f, 1f) }
            5 -> { setRotate(90f); postScale(-1f, 1f) }; 6 -> setRotate(90f); 7 -> { setRotate(-90f); postScale(-1f, 1f) }; 8 -> setRotate(-90f)
        }
    }
    val bitmap = if (matrix.isIdentity) source else Bitmap.createBitmap(source, 0, 0, source.width, source.height, matrix, true).also { if (it !== source) source.recycle() }
    try {
        var compressed = ByteArray(0)
        for (quality in listOf(85, 70, 55, 40)) {
            compressed = ByteArrayOutputStream().use { bitmap.compress(Bitmap.CompressFormat.JPEG, quality, it); it.toByteArray() }
            if (compressed.size <= 512 * 1024) break
        }
        check(compressed.size <= 512 * 1024) { "图片仍然过大，请裁剪后重试" }
        return api.request("/api/me/articles/media", "POST", JSONObject().put("base64", Base64.encodeToString(compressed, Base64.NO_WRAP)))
    } finally { bitmap.recycle() }
}
