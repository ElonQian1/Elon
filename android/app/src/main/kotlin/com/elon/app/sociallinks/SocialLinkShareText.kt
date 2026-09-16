package com.elon.app.sociallinks

/** Presentation only: never mutate the stored message or copied/edited source. */
internal object SocialLinkShareText {
    fun isGeneric(title: String, site: String): Boolean {
        val clean = title.replace(Regex("\\s+"), "").lowercase()
        return clean.isBlank() || clean == site.lowercase() || clean in setOf(
            "小红书-你的生活兴趣社区", "小红书–你的生活兴趣社区", "微信公众平台", "微信公众号", "环境异常", "安全验证", "访问验证", "抖音-记录美好生活")
    }
    fun title(raw: String, site: String, nearby: String = ""): Pair<String, String> {
        if (site == "抖音") {
            val match = Regex("看看【([^】]+)的作品】\\s*(\\S[\\s\\S]*)$").find(nearby)
            if (match != null) return match.groupValues[2].trim().take(160) to match.groupValues[1].take(80)
        }
        if (site != "小红书") return raw.take(160) to ""
        val split = raw.substringBefore(" | 小红书").split(" - ")
        return if (split.size > 1) split.dropLast(1).joinToString(" - ").take(160) to split.last().take(80) else raw.take(160) to ""
    }
    fun compact(content: String, items: List<SocialLink> = SocialLinkPolicy.extract(content)): Boolean {
        if (items.size != 1) return false
        val item = items.single(); val text = content.trim()
        if (text == item.url || text == "[${item.url}](${item.url})") return true
        val index = text.indexOf(item.url)
        if (index < 0) return false
        val before = text.substring(0, index).trim(); val after = text.substring(index + item.url.length).trim()
        return when (item.site) {
            "小红书" -> after.isEmpty() && Regex("^\\d{1,3}\\s+【[^】]+】\\s+.{1,8}\\s+[A-Za-z0-9]{6,32}\\s+.{1,8}$").matches(before)
            "抖音" -> Regex("^\\d+(?:\\.\\d+)?\\s+复制打开抖音[，,].+$").matches(before) &&
                Regex("^[A-Za-z0-9]{3}:/\\s+[A-Za-z0-9@.]+\\s+:[A-Za-z0-9]+\\s+\\d{2}/\\d{2}$").matches(after)
            "哔哩哔哩" -> after.isEmpty() && Regex("^(?:【[^】]+】\\s*){1,2}$").matches(before)
            else -> false
        }
    }
}
