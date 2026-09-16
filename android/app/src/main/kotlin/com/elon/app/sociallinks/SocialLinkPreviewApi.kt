package com.elon.app.sociallinks

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.util.LruCache
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import okhttp3.Cache
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.File
import java.io.InputStream
import java.io.ByteArrayOutputStream
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

internal object SocialLinkPreviewApi {
    val loader = ThreadPoolExecutor(3, 3, 30, TimeUnit.SECONDS, ArrayBlockingQueue(32), ThreadPoolExecutor.DiscardPolicy())
    private val http = OkHttpClient.Builder().callTimeout(12, TimeUnit.SECONDS).build()
    private var images: OkHttpClient? = null
    private val previews = object : LruCache<String, Pair<Long, SocialLink>>(128) {}
    private val bitmaps = object : LruCache<String, Bitmap>(8 * 1024 * 1024) {
        override fun sizeOf(key: String, value: Bitmap) = value.allocationByteCount
    }
    fun load(context: Context, item: SocialLink, refresh: Boolean = false): SocialLink {
        val owner = AuthManager.userId(context).orEmpty()
        SocialLinkReadPreview.cached(ServerUrlManager.getActive(context), AuthManager.userId(context), item.url)?.let { return it }
        val key = ServerUrlManager.getActive(context) + ":" + owner + ":" + item.url
        val old = previews.get(key)
        if (!refresh && old != null && old.first > System.currentTimeMillis()) return old.second
        val request = Request.Builder().url(ServerUrlManager.getActive(context).trimEnd('/') + "/api/me/link-preview")
            .post(JSONObject().put("url", item.url).toString().toRequestBody("application/json".toMediaType()))
        val preview = runCatching {
            http.newCall(AuthManager.applyAuth(context, request).build()).execute().use { response ->
                if (!response.isSuccessful) return@use item
                val bytes = response.body?.byteStream()?.use { readBounded(it, 16384) } ?: return@use item
                SocialLinkPolicy.merge(JSONObject(String(bytes, Charsets.UTF_8)), item)
            }
        }.getOrDefault(item)
        if (AuthManager.userId(context).orEmpty() == owner) previews.put(key, (System.currentTimeMillis() + if (preview.ready) 3600000 else 30000) to preview)
        return preview
    }
    fun cover(context: Context, url: String): Bitmap? {
        bitmaps.get(url)?.let { return it }
        if (SocialLinkPolicy.safeUrl(url) == null) return null
        return runCatching {
            val client = synchronized(this) { images ?: OkHttpClient.Builder().callTimeout(10, TimeUnit.SECONDS)
                .cache(Cache(File(context.cacheDir, "social-link-covers"), 24L * 1024 * 1024)).build().also { images = it } }
            client.newCall(Request.Builder().url(url).build()).execute().use { response ->
                if (!response.isSuccessful || response.body?.contentType()?.type != "image") return@use null
                val data = response.body?.byteStream()?.use { readBounded(it, 1024 * 1024 + 1) } ?: return@use null
                if (data.size > 1024 * 1024) return@use null
                val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
                BitmapFactory.decodeByteArray(data, 0, data.size, options)
                if (options.outWidth <= 0 || options.outHeight <= 0) return@use null
                options.inSampleSize = 1
                while (maxOf(options.outWidth, options.outHeight) / options.inSampleSize > 256) options.inSampleSize *= 2
                options.inJustDecodeBounds = false
                BitmapFactory.decodeByteArray(data, 0, data.size, options)?.also { bitmaps.put(url, it) }
            }
        }.getOrNull()
    }
    private fun readBounded(input: InputStream, limit: Int): ByteArray {
        val out = ByteArrayOutputStream(); val buffer = ByteArray(8192)
        while (out.size() < limit) {
            val count = input.read(buffer, 0, minOf(buffer.size, limit - out.size()))
            if (count < 0) break
            out.write(buffer, 0, count)
        }
        return out.toByteArray()
    }
}
