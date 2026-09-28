package com.elon.app.chatrecords

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.MediaMetadataRetriever
import android.util.AtomicFile
import org.json.JSONObject
import java.io.File

/** File names inherit the account/record/content digest of the revalidated byte cache. */
internal object ChatRecordVideoPoster {
    data class Poster(val bitmap: Bitmap?, val duration: Long)
    fun read(file: File): Poster {
        val image = File(file.parentFile, file.name + ".poster.jpg")
        val metadata = File(file.parentFile, file.name + ".poster.json")
        runCatching {
            val info = JSONObject(metadata.readText())
            if (info.getLong("size") == file.length()) {
                val bitmap = BitmapFactory.decodeFile(image.path)
                if (bitmap != null) { image.setLastModified(System.currentTimeMillis()); metadata.setLastModified(System.currentTimeMillis()); return Poster(bitmap, info.optLong("duration")) }
            }
        }
        val retriever = MediaMetadataRetriever()
        try {
            retriever.setDataSource(file.path)
            val duration = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull() ?: 0L
            val original = if (android.os.Build.VERSION.SDK_INT >= 27) retriever.getScaledFrameAtTime(0, MediaMetadataRetriever.OPTION_CLOSEST_SYNC, 480, 480)
                else retriever.getFrameAtTime(0, MediaMetadataRetriever.OPTION_CLOSEST_SYNC)
            if (original == null) return Poster(null, duration)
            val scale = minOf(1f, 480f / maxOf(original.width, original.height))
            val bitmap = Bitmap.createScaledBitmap(original, (original.width * scale).toInt().coerceAtLeast(1), (original.height * scale).toInt().coerceAtLeast(1), true)
            if (bitmap !== original) original.recycle()
            runCatching {
                atomic(image) { bitmap.compress(Bitmap.CompressFormat.JPEG, 75, it) }
                atomic(metadata) { it.write(JSONObject().put("size", file.length()).put("duration", duration).toString().toByteArray()) }
            }
            return Poster(bitmap, duration)
        } finally { retriever.release() }
    }
    private fun atomic(file: File, write: (java.io.OutputStream) -> Unit) {
        val target = AtomicFile(file); val stream = target.startWrite()
        try { write(stream); target.finishWrite(stream) } catch (error: Exception) { target.failWrite(stream); throw error }
    }
}
