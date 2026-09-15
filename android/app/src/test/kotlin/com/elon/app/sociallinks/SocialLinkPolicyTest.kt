package com.elon.app.sociallinks

import org.junit.Assert.*
import org.junit.Test

class SocialLinkPolicyTest {
    @Test fun bilibiliShareKeepsTimeAndPage() {
        val items = SocialLinkPolicy.extract("【高糖VS戒糖14天！真的差别很大吗？】 【精准空降到 00:02】 https://www.bilibili.com/video/BV1enYL6SEtU/?share_source=copy_web&t=2&p=3")
        assertEquals(1, items.size)
        assertEquals("高糖VS戒糖14天！真的差别很大吗？", items[0].title)
        assertEquals("https://player.bilibili.com/player.html?bvid=BV1enYL6SEtU&autoplay=0&poster=1&t=2&p=3", items[0].player)
    }
    @Test fun linksPreserveShareAccessAndRejectLookalikes() {
        val url = "https://www.xiaohongshu.com/discovery/item/123?xsec_token=synthetic&xsec_source=pc_share"
        assertEquals(url, SocialLinkPolicy.extract(url)[0].url)
        assertTrue(SocialLinkPolicy.extract("https://x.com.evil.example/a/status/123456").isEmpty())
        assertNull(SocialLinkPolicy.safeUrl("https://user:pass@x.com/a"))
        assertNull(SocialLinkPolicy.safeUrl("javascript:alert(1)"))
    }
    @Test fun xArticlesAreNotMisrepresentedAsEmbeddablePosts() {
        assertNull(SocialLinkPolicy.link("https://x.com/i/article/123456")!!.xId)
        assertEquals("463440424141459456", SocialLinkPolicy.link("https://twitter.com/Interior/status/463440424141459456?s=20")!!.xId)
        assertNull(SocialLinkPolicy.link("https://www.binance.com/en/square/post/123456")!!.player)
    }
    @Test fun repeatedAndInternalCardsDoNotExpandTwice() {
        val url = "https://mp.weixin.qq.com/s/example"
        assertEquals(1, SocialLinkPolicy.extract("$url $url").size)
        assertTrue(SocialLinkPolicy.extract("【一龙文章】\n{\"url\":\"$url\"}").isEmpty())
    }
}
