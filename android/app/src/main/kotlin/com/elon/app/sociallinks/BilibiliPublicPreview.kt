package com.elon.app.sociallinks

import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/** Public video metadata only. No WebView, account credentials, redirects or playback requests. */
internal object BilibiliPublicPreview {
    private const val MAX_BYTES = 256 * 1024
    private val http = OkHttpClient.Builder().callTimeout(4, TimeUnit.SECONDS)
        .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false).build()

    internal fun videoId(item: SocialLink): String? = SocialLinkReadIdentity.identity(item.url)
        ?.takeIf { it.startsWith("bilibili:") }?.substringAfter(':')

    fun load(item: SocialLink): SocialLink? {
        val id = videoId(item) ?: return null
        return runCatching {
            val request = Request.Builder().url("https://api.bilibili.com/x/web-interface/view?bvid=$id").build()
            http.newCall(request).execute().use { response ->
                if (!response.isSuccessful || response.body?.contentType()?.subtype != "json") return@use null
                val body = response.body ?: return@use null
                if (body.contentLength() > MAX_BYTES) return@use null
                val bytes = body.byteStream().use { input ->
                    val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(8192)
                    while (out.size() <= MAX_BYTES) {
                        val count = input.read(buffer, 0, minOf(buffer.size, MAX_BYTES + 1 - out.size()))
                        if (count < 0) break
                        out.write(buffer, 0, count)
                    }
                    out.toByteArray()
                }
                if (bytes.size > MAX_BYTES) return@use null
                parse(item, JSONObject(String(bytes, Charsets.UTF_8)))
            }
        }.getOrNull()
    }

    internal fun parse(item: SocialLink, value: JSONObject): SocialLink? {
        val id = videoId(item) ?: return null
        if (value.optInt("code", -1) != 0) return null
        val data = value.optJSONObject("data") ?: return null
        if (data.optString("bvid") != id) return null
        val title = data.optString("title").trim().take(160)
        if (SocialLinkShareText.isGeneric(title, item.site)) return null
        val pic = data.optString("pic").let { if (it.startsWith("http://")) "https://" + it.removePrefix("http://") else it }
        val image = SocialLinkReadIdentity.image(pic, "bilibili") ?: return null
        return item.copy(title = title, image = image, ready = true,
            author = data.optJSONObject("owner")?.optString("name").orEmpty().trim().take(80))
    }
}
