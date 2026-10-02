package com.elon.app.sociallinks

import org.junit.Assert.*
import org.junit.Test

class XiaohongshuShortLinkTest {
    @Test fun cnSharesKeepTitleAndUseNoteCardWithoutHidingComments() {
        for ((title, url, suffix) in listOf(
            Triple("游戏创作的天塌了，AI 黑科技它来了！", "https://xhslink.cn/o/7QvqZwsx2Ch", "跳转【小红书】看看这篇分享！"),
            Triple("帮助你做游戏的项目，将做游戏的门槛拉低", "https://xhslink.cn/o/4Oatsa1J8RD", "带上口令，来【小红书】看笔记全文~")
        )) {
            val share = "$title $url $suffix"
            val item = SocialLinkPolicy.extract(share).single()
            assertEquals("小红书", item.site)
            assertEquals(title, item.title)
            assertEquals("", item.author)
            assertTrue(SocialLinkPresentation.mediaCard(item))
            assertNull(item.player)
            assertFalse(SocialLinkShareText.compact(share))
            assertEquals("", SocialLinkPolicy.extract("My comment\n$share").single().title)
            assertEquals("", SocialLinkPolicy.extract("$share extra comment").single().title)
        }
    }
    @Test fun shortLinksResolveOnlyToSameProviderAndKeepAccessParameters() {
        val note = "https://www.xiaohongshu.com/discovery/item/0123456789abcdef01234567?xsec_token=fixture%2Btoken"
        for (host in listOf("xhslink.cn", "www.xhslink.cn", "xhslink.com", "www.xhslink.com")) {
            val item = SocialLinkPolicy.link("https://$host/o/example")!!
            assertTrue(SocialLinkPresentation.mediaCard(item))
            assertTrue(SocialMediaOpenPolicy.isShort(item))
            assertEquals(SocialMediaPlatform.XIAOHONGSHU, SocialMediaOpenPolicy.platform(item))
            assertEquals(note, SocialMediaOpenPolicy.redirect(item, item.url, note))
            assertEquals(note, SocialMediaOpenPolicy.destination(item, note)?.url)
            assertNull(SocialMediaOpenPolicy.redirect(item, item.url, "https://xhslink.cn.evil.example/o/test"))
            assertNull(SocialMediaOpenPolicy.destination(item, "https://www.xiaohongshu.com/login"))
        }
        val spoof = SocialLinkPolicy.link("https://xhslink.cn.evil.example/o/test")!!
        assertFalse(SocialLinkPresentation.mediaCard(spoof))
        assertNull(SocialMediaOpenPolicy.platform(spoof))
    }
}
