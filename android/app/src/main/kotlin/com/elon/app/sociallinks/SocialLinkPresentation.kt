package com.elon.app.sociallinks

/** Honest provider fallback; titles/covers are never inferred from unrelated articles. */
internal object SocialLinkPresentation {
    fun title(item: SocialLink) = item.title.ifBlank { when (item.site) {
        "微信公众号" -> "微信公众号文章"
        "小红书" -> "小红书笔记"
        "抖音" -> "抖音视频"
        "哔哩哔哩" -> "哔哩哔哩视频"
        "币安广场" -> "币安广场帖子"
        "X" -> "X 帖子"
        else -> "网页链接"
    } }
    fun badge(site: String) = when (site) {
        "微信公众号" -> "文"
        "抖音" -> "抖"
        "哔哩哔哩" -> "B站"
        "币安广场" -> "币安"
        "小红书" -> "小红书"
        "X" -> "X"
        else -> "↗"
    }
    fun colors(site: String) = when (site) {
        "微信公众号" -> "#263E35" to "#BDE6CA"
        "小红书" -> "#44313A" to "#FFD1DC"
        "币安广场" -> "#403B2C" to "#F2DC94"
        "哔哩哔哩" -> "#2B3D49" to "#BDE8FA"
        else -> "#343E4A" to "#E4EBF4"
    }
    fun source(item: SocialLink): String {
        val parts = SocialLinkPolicy.safeUrl(item.url)?.path.orEmpty().split('/').filter { it.isNotBlank() }
        val handle = parts.firstOrNull().orEmpty()
        val author = item.author.trim().ifBlank {
            if (item.site == "X" && parts.getOrNull(1) == "status" && handle != "i" && handle.matches(Regex("[A-Za-z0-9_]{1,15}"))) "@$handle" else ""
        }
        return item.site + (if (author.isNotBlank() && author != item.site) " · $author" else "") + (if (item.member) " · 成员回填" else "")
    }
    fun time(item: SocialLink): String {
        if (item.site != "哔哩哔哩") return ""
        val query = SocialLinkPolicy.safeUrl(item.player)?.rawQuery.orEmpty()
        val n = Regex("(?:^|&)t=([0-9]+)(?:&|$)").find(query)?.groupValues?.get(1)?.toIntOrNull() ?: return ""
        if (n !in 0..604800) return ""
        val minutes = (if (n >= 3600) n / 60 % 60 else n / 60).toString().padStart(2, '0')
        val stamp = (if (n >= 3600) "${n / 3600}:" else "") + minutes + ":" + (n % 60).toString().padStart(2, '0')
        return "从 $stamp 开始"
    }
}
