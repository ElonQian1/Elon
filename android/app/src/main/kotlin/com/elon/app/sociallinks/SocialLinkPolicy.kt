package com.elon.app.sociallinks

import java.net.URI
import java.net.URLDecoder
import org.json.JSONObject

internal data class SocialLink(val url: String, val site: String, val title: String = "", val author: String = "", val image: String? = null, val player: String? = null, val xId: String? = null, val ready: Boolean = false, val summary: String = "", val member: Boolean = false)

internal object SocialLinkPolicy {
    private val sites = mapOf(
        "mp.weixin.qq.com" to "微信公众号", "douyin.com" to "抖音", "www.douyin.com" to "抖音", "v.douyin.com" to "抖音", "www.iesdouyin.com" to "抖音",
        "xiaohongshu.com" to "小红书", "www.xiaohongshu.com" to "小红书", "xhslink.com" to "小红书", "www.xhslink.com" to "小红书",
        "bilibili.com" to "哔哩哔哩", "www.bilibili.com" to "哔哩哔哩", "m.bilibili.com" to "哔哩哔哩", "b23.tv" to "哔哩哔哩",
        "binance.com" to "币安广场", "www.binance.com" to "币安广场", "app.binance.com" to "币安广场", "x.com" to "X", "www.x.com" to "X", "twitter.com" to "X", "www.twitter.com" to "X", "mobile.twitter.com" to "X", "t.co" to "X"
    )
    fun safeUrl(value: String?): URI? = runCatching {
        if (value == null || value.length > 4096 || value.any { it.code < 32 || it.code == 127 }) return null
        URI(value).takeIf { it.scheme == "https" && !it.host.isNullOrBlank() && it.rawUserInfo == null && it.port in listOf(-1, 443) }
    }.getOrNull()
    /** Server-copied thumbnail; bounded so one response cannot inflate the preview cache. */
    fun inlineCover(value: String?): String? =
        value?.takeIf { it.length <= 98304 && Regex("^data:image/(?:jpeg|png|webp);base64,[A-Za-z0-9+/]+=*$").matches(it) }
    fun link(value: String, title: String = ""): SocialLink? {
        val uri = safeUrl(value) ?: return null
        val site = sites[uri.host.lowercase()] ?: return null
        val parts = uri.path.split('/').filter { it.isNotBlank() }
        fun id(segment: String): String? = parts.indexOf(segment).takeIf { it >= 0 }?.let { parts.getOrNull(it + 1) }
        val params = uri.rawQuery.orEmpty().split('&').mapNotNull {
            val pair = it.split('=', limit = 2)
            if (pair.size != 2) null else runCatching { URLDecoder.decode(pair[0], "UTF-8") to URLDecoder.decode(pair[1], "UTF-8") }.getOrNull()
        }.toMap()
        val bv = id("video")?.takeIf { site == "哔哩哔哩" && it.matches(Regex("BV[A-Za-z0-9]{10}")) }
        val video = id("video")?.takeIf { site == "抖音" && it.matches(Regex("[0-9]{5,24}")) }
        val x = id("status")?.takeIf { site == "X" && it.matches(Regex("[0-9]{5,24}")) }
        val player = when {
            bv != null -> buildString {
                append("https://player.bilibili.com/player.html?bvid=$bv&autoplay=0&poster=1")
                for (key in listOf("t", "p")) {
                    val n = params[key]?.takeIf { it.matches(Regex("[0-9]+")) }?.toIntOrNull()
                    if (n != null && n in (if (key == "p") 1 else 0)..(if (key == "p") 10000 else 604800)) append("&$key=$n")
                }
            }
            video != null -> "https://open.douyin.com/player/video?vid=$video&autoplay=0"
            else -> null
        }
        return SocialLink(value, site, title.take(160), player = player, xId = x)
    }
    fun extract(text: String): List<SocialLink> {
        if (Regex("^【一龙(?:文章|项目|AI)").containsMatchIn(text)) return emptyList()
        val result = mutableListOf<SocialLink>()
        for (match in Regex("https://[^\\s<>\"'\\]\\)]+").findAll(text.take(50000))) {
            val value = match.value.trimEnd('，', '。', '！', '？', '；', '：', '、', '）', '】', '》', '”', '’', '.', ',', '!', ';')
            val nearby = text.substring((match.range.first - 300).coerceAtLeast(0), match.range.first)
            val title = Regex("【([^】]+)】").findAll(nearby).map { it.groupValues[1] }.firstOrNull { !it.startsWith("精准空降") }.orEmpty()
            val linked = link(value, title) ?: continue
            val (headline, author) = SocialLinkShareText.title(title, linked.site, nearby)
            val item = linked.copy(title = headline, author = author)
            if (result.none { it.url == item.url }) result.add(item)
            if (result.size == 2) break
        }
        return result
    }
    fun merge(value: JSONObject, fallback: SocialLink): SocialLink {
        if (value.optInt("schema") != 1 || value.optString("url") != fallback.url) return fallback
        val embed = value.optJSONObject("embed")
        val id = embed?.optString("id").orEmpty()
        val resolved = when (embed?.optString("kind")) {
            "x" -> if (id.matches(Regex("[0-9]{5,24}"))) link("https://x.com/i/status/$id") else null
            "douyin" -> if (id.matches(Regex("[0-9]{5,24}"))) link("https://www.douyin.com/video/$id") else null
            "bilibili" -> {
                val player = safeUrl(embed.optString("url"))
                if (id.matches(Regex("BV[A-Za-z0-9]{10}")) && player?.host == "player.bilibili.com" && player.path == "/player.html") link("https://www.bilibili.com/video/$id?${player.rawQuery}") else null
            }
            else -> null
        }
        val title = value.optString("title").trim().take(160).takeUnless { SocialLinkShareText.isGeneric(it, fallback.site) } ?: fallback.title
        return fallback.copy(title = title, author = value.optString("author").take(80).ifBlank { fallback.author },
            image = inlineCover(value.optString("cover_data_url")) ?: safeUrl(value.optString("image"))?.toString(), player = resolved?.player ?: fallback.player, xId = resolved?.xId ?: fallback.xId, ready = value.optString("status") == "ready" && title.isNotBlank(),
            summary = value.optString("description").trim().take(300), member = value.optString("source") == "member")
    }
}
