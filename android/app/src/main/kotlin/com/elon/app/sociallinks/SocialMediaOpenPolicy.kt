package com.elon.app.sociallinks

internal enum class SocialMediaOpenMode { APP, READER }

internal enum class SocialMediaPlatform(val key: String, val appLabel: String, val packageName: String) {
    WECHAT("wechat_channels", "微信 App", "com.tencent.mm"),
    DOUYIN("douyin", "抖音 App", "com.ss.android.ugc.aweme"),
    XIAOHONGSHU("xiaohongshu", "小红书 App", "com.xingin.xhs"),
    BILIBILI("bilibili", "B站 App", "tv.danmaku.bili");
}

/** Routing derives only from validated source hosts, never from server-provided labels. */
internal object SocialMediaOpenPolicy {
    fun platform(item: SocialLink): SocialMediaPlatform? {
        val parsed = SocialLinkPolicy.link(item.url) ?: return null
        if (WechatChannelsPolicy.isChannels(parsed.url)) return SocialMediaPlatform.WECHAT
        return when (parsed.site) {
            "抖音" -> SocialMediaPlatform.DOUYIN
            "小红书" -> SocialMediaPlatform.XIAOHONGSHU
            "哔哩哔哩" -> SocialMediaPlatform.BILIBILI
            else -> null
        }
    }
    fun candidates(item: SocialLink): List<String> {
        val platform = platform(item) ?: return emptyList()
        if (platform == SocialMediaPlatform.WECHAT) return emptyList()
        val url = SocialLinkPolicy.safeUrl(item.url) ?: return emptyList()
        val identity = SocialLinkReadIdentity.identity(item.url)
        val share = if (platform == SocialMediaPlatform.DOUYIN && identity?.startsWith("douyin:") == true)
            "https://www.iesdouyin.com/share/video/${identity.substringAfter(':')}/" +
                (url.rawQuery?.let { "?$it" } ?: "") else null
        return listOfNotNull(share, item.url).distinct()
    }
    fun isShort(item: SocialLink): Boolean = SocialLinkPolicy.safeUrl(item.url)?.host in
        setOf("v.douyin.com", "xhslink.com", "www.xhslink.com", "b23.tv")
    fun redirect(item: SocialLink, current: String, location: String): String? {
        val expected = platform(item) ?: return null
        val next = runCatching { SocialLinkPolicy.safeUrl(current)?.resolve(location)?.toString() }.getOrNull() ?: return null
        val parsed = SocialLinkPolicy.link(next) ?: return null
        if (platform(parsed) != expected) return null
        val originalId = SocialLinkReadIdentity.identity(item.url)
        if (originalId != null && SocialLinkReadIdentity.identity(next) != originalId) return null
        return parsed.url
    }
    fun destination(item: SocialLink, resolved: String): SocialLink? {
        val parsed = SocialLinkPolicy.link(resolved) ?: return null
        return parsed.takeIf { platform(it) == platform(item) && SocialLinkReadIdentity.identity(it.url) != null }
    }
    fun effective(preferred: SocialMediaOpenMode, appAvailable: Boolean) =
        if (preferred == SocialMediaOpenMode.APP && !appAvailable) SocialMediaOpenMode.READER else preferred
}
