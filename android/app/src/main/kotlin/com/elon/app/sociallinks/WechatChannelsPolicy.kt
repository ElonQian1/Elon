package com.elon.app.sociallinks

import java.net.URI
import java.net.URLDecoder

/** Only the preview and feed handoff observed in the official Channels page. */
internal object WechatChannelsPolicy {
    private fun https(value: String): URI? = runCatching { URI(value) }.getOrNull()?.takeIf {
        it.scheme == "https" && it.rawUserInfo == null && it.port == -1 && it.rawFragment == null
    }

    fun isChannels(value: String): Boolean {
        val uri = https(value) ?: return false
        return (uri.host == "weixin.qq.com" && Regex("/sph/[A-Za-z0-9_-]{1,128}").matches(uri.path.orEmpty())) || isPreview(value)
    }

    fun isPreview(value: String): Boolean {
        val uri = https(value) ?: return false
        if (uri.host != "channels.weixin.qq.com" || uri.path != "/finder-preview/pages/sph") return false
        val ids = uri.rawQuery.orEmpty().split('&').filter { it.substringBefore('=') == "id" }
        return ids.size == 1 && Regex("[A-Za-z0-9_-]{1,128}").matches(ids[0].substringAfter('=', ""))
    }

    fun sameContent(first: String, second: String): Boolean {
        fun id(value: String): String? {
            if (!isChannels(value)) return null
            val uri = https(value) ?: return null
            return if (uri.host == "weixin.qq.com") uri.path.substringAfter("/sph/") else
                uri.rawQuery.orEmpty().split('&').single { it.substringBefore('=') == "id" }.substringAfter('=')
        }
        return id(first)?.let { it == id(second) } == true
    }

    fun allows(source: String, target: String): Boolean {
        if (!isPreview(source) || target.length > 4096) return false
        val uri = runCatching { URI(target) }.getOrNull() ?: return false
        if (uri.scheme != "weixin" || uri.host != "biz" || uri.rawUserInfo != null || uri.port != -1 || uri.rawQuery != null || uri.rawFragment != null) return false
        val prefix = "/finder/openFinderFeed/"
        if (!uri.rawPath.orEmpty().startsWith(prefix)) return false
        val decoded = runCatching { URLDecoder.decode(uri.rawPath.removePrefix(prefix), "UTF-8") }.getOrNull() ?: return false
        val seen = mutableSetOf<String>()
        for (pair in decoded.split('&')) {
            if (!pair.contains('=')) return false
            val key = pair.substringBefore('='); val value = pair.substringAfter('=')
            if (!seen.add(key)) return false
            val valid = when (key) {
                "exportId" -> Regex("export/[A-Za-z0-9_-]{8,2048}").matches(value)
                "actionType" -> value == "0"
                "commentScene", "entryScene", "entryCardType", "requestScene" -> Regex("[0-9]{1,7}").matches(value) && (value.toIntOrNull() ?: -1) in 0..1_000_000
                else -> false
            }
            if (!valid) return false
        }
        return seen.containsAll(listOf("exportId", "actionType"))
    }
}
