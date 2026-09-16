package com.elon.app.sociallinks

/** Host-side validation of untrusted original-page observations. */
internal object SocialLinkReadIdentity {
    fun readingUrl(original: String): String {
        val u = SocialLinkPolicy.safeUrl(original) ?: return original
        val id = identity(original) ?: return original
        if (u.host != "app.binance.com" || !id.startsWith("binance:")) return original
        val language = u.rawQuery.orEmpty().split('&').firstOrNull { it.startsWith("l=") }?.removePrefix("l=")
            ?.takeIf { Regex("[a-z]{2}(?:-[A-Z]{2})?").matches(it) } ?: "en"
        return "https://www.binance.com/$language/square/post/${id.substringAfter(':')}" + (u.rawQuery?.let { "?$it" } ?: "")
    }
    fun identity(value: String): String? {
        val u = SocialLinkPolicy.safeUrl(value) ?: return null
        val path = u.path.trimEnd('/')
        if (u.host == "mp.weixin.qq.com" && Regex("/s(?:/[^/]+)?").matches(path)) return "wechat:$path" + if (path == "/s") "?${u.rawQuery.orEmpty()}" else ""
        if (u.host in setOf("binance.com", "www.binance.com", "app.binance.com")) {
            val m = Regex("(?:/[a-z]{2}(?:-[A-Z]{2})?)?/square/(?:post|article)/([0-9]{5,24})").matchEntire(path)
                ?: Regex("/uni-qr/cpos/([0-9]{5,24})").matchEntire(path)
            return m?.let { "binance:${it.groupValues[1]}" }
        }
        if (u.host in setOf("x.com", "www.x.com", "twitter.com", "www.twitter.com", "mobile.twitter.com")) {
            Regex("/(?:[A-Za-z0-9_]{1,15}|i/web)/status/([0-9]{5,24})").matchEntire(path)?.let { return "x-post:${it.groupValues[1]}" }
            Regex("/i/article/([0-9]{5,24})").matchEntire(path)?.let { return "x-article:${it.groupValues[1]}" }
        }
        return null
    }
    fun image(value: String, kind: String): String? {
        val u = SocialLinkPolicy.safeUrl(value) ?: return null
        val allowed = when {
            kind == "wechat" -> u.host == "qpic.cn" || u.host.endsWith(".qpic.cn")
            kind == "binance" -> (u.host == "bnbstatic.com" || u.host.endsWith(".bnbstatic.com")) && !Regex("logo|avatar|icon", RegexOption.IGNORE_CASE).containsMatchIn(u.path)
            kind.startsWith("x-") -> u.host == "pbs.twimg.com" && Regex("^/(?:media|card_img|amplify_video_thumb|ext_tw_video_thumb|tweet_video_thumb)/").containsMatchIn(u.path)
            else -> false
        }
        return u.toString().takeIf { allowed }
    }
}
