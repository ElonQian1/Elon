package com.elon.app.articles.square

import android.content.Context
import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.util.Base64
import com.elon.app.articles.articleImageBase64
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.ByteArrayOutputStream

internal fun squareImage(context: Context, uri: Uri, api: SquareApi): JSONObject = api.request("/media", "POST", JSONObject().put("base64", articleImageBase64(context, uri)))

internal fun squareVideo(context: Context, uri: Uri, api: SquareApi): JSONObject {
    val mime = context.contentResolver.getType(uri).orEmpty()
    check(mime in setOf("video/mp4", "video/webm")) { "请选择MP4或WebM视频" }
    val bytes = context.contentResolver.openInputStream(uri)?.use { input ->
        val output = ByteArrayOutputStream(); val chunk = ByteArray(8192)
        while (true) { val n = input.read(chunk); if (n < 0) break; output.write(chunk, 0, n); check(output.size() <= 32 * 1024 * 1024) { "视频不能超过32MB" } }
        output.toByteArray()
    } ?: error("无法读取视频")
    val retriever = MediaMetadataRetriever()
    var frame: Bitmap? = null
    try {
        retriever.setDataSource(context, uri)
        val duration = (retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toDoubleOrNull() ?: 0.0) / 1000
        check(duration > 0 && duration <= 600) { "视频需在10分钟以内" }
        frame = retriever.getFrameAtTime(0, MediaMetadataRetriever.OPTION_CLOSEST_SYNC) ?: error("无法生成视频首帧，请重新选择视频")
        val original = frame!!; val scale = minOf(1.0, 1280.0 / maxOf(original.width, original.height))
        if (scale < 1) frame = Bitmap.createScaledBitmap(original, (original.width * scale).toInt().coerceAtLeast(1), (original.height * scale).toInt().coerceAtLeast(1), true).also { if (it !== original) original.recycle() }
        val jpeg = ByteArrayOutputStream().use { frame!!.compress(Bitmap.CompressFormat.JPEG, 75, it); it.toByteArray() }
        check(jpeg.size <= 512 * 1024) { "视频封面过大，请重新选择视频" }
        val cover = api.request("/media", "POST", JSONObject().put("base64", Base64.encodeToString(jpeg, Base64.NO_WRAP)))
        val data = MultipartBody.Builder().setType(MultipartBody.FORM)
            .addFormDataPart("cover_id", cover.getString("id")).addFormDataPart("duration", duration.toString())
            .addFormDataPart("video", if (mime == "video/mp4") "video.mp4" else "video.webm", bytes.toRequestBody(mime.toMediaType())).build()
        return api.send("/videos", "POST", data).put("cover", cover.getString("data_url"))
    } finally { frame?.recycle(); retriever.release() }
}
