package com.elon.app.sharing

import org.json.JSONObject
import java.net.URI

/** Sender-derived metadata, not a claim that the destination is trusted. */
data class SourceLink(val url: String, val method: String = "qr") {
    fun json() = JSONObject().put("version", 1).put("url", url).put("method", method)
    val label: String get() = if (isArticle(url)) "阅读原文" else "打开链接"
    companion object {
        fun webUrl(raw: String?): String? {
            if (raw == null || raw.toByteArray(Charsets.UTF_8).size > 4096 || raw.any { it.isWhitespace() || it.isISOControl() || it == '\\' }) return null
            if (!raw.startsWith("https://", true) && !raw.startsWith("http://", true)) return null
            return runCatching { URI(raw).takeIf { it.scheme?.lowercase() in setOf("https", "http") && !it.host.isNullOrBlank() && it.rawUserInfo == null }?.let { raw } }.getOrNull()
        }
        fun fromJson(value: JSONObject?): SourceLink? {
            if (value?.optInt("version") != 1 || value.optString("method") !in setOf("qr", "share")) return null
            return webUrl(value.optString("url"))?.let { SourceLink(it, value.optString("method")) }
        }
        fun isArticle(url: String): Boolean = runCatching {
            val uri = URI(url)
            uri.host.equals("mp.weixin.qq.com", true) && (uri.path == "/s" || uri.path.startsWith("/s/"))
        }.getOrDefault(false)
        fun fromText(text: String): SourceLink? = webUrl(text.trim())?.let { SourceLink(it, "share") }
    }
}
