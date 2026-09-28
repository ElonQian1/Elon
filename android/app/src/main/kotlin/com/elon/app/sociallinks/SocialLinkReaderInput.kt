package com.elon.app.sociallinks

/** Preserve resolved card destinations across Activity recreation without accepting arbitrary embeds. */
internal object SocialLinkReaderInput {
    fun parse(url: String, title: String, player: String?, xId: String?): SocialLink? {
        val link = SocialLinkPolicy.link(url, title) ?: return null
        val embed = SocialLinkPolicy.safeUrl(player)
        val resolved = when {
            link.site == "抖音" && embed?.host == "open.douyin.com" && embed.path == "/player/video" -> {
                val id = Regex("(?:^|&)vid=([0-9]{5,24})(?:&|$)").find(embed.rawQuery.orEmpty())?.groupValues?.get(1)
                id?.let { SocialLinkPolicy.link("https://www.douyin.com/video/$it") }
                    ?.takeIf { SocialLinkReadIdentity.cacheAlias(link.url, it.url) }
            }
            link.site == "哔哩哔哩" && embed?.host == "player.bilibili.com" && embed.path == "/player.html" -> {
                val id = Regex("(?:^|&)bvid=(BV[A-Za-z0-9]{10})(?:&|$)").find(embed.rawQuery.orEmpty())?.groupValues?.get(1)
                id?.let { SocialLinkPolicy.link("https://www.bilibili.com/video/$it?${embed.rawQuery}") }
                    ?.takeIf { SocialLinkReadIdentity.identity(link.url) == SocialLinkReadIdentity.identity(it.url) || SocialLinkPolicy.safeUrl(link.url)?.host == "b23.tv" }
            }
            else -> null
        }
        val postId = xId?.takeIf { link.site == "X" && Regex("[0-9]{5,24}").matches(it) &&
            (SocialLinkReadIdentity.identity(link.url) == "x-post:$it" || SocialLinkPolicy.safeUrl(link.url)?.host == "t.co") }
        return link.copy(player = resolved?.player ?: link.player, xId = postId ?: link.xId)
    }
}
