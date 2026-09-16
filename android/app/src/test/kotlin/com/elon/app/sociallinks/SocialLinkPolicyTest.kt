package com.elon.app.sociallinks

import org.junit.Assert.*
import org.junit.Test

class SocialLinkPolicyTest {
    @Test fun platformAuthorTimeAndBinanceFallbackAreConsistent() {
        val douyin = SocialLinkPolicy.extract("3.53 复制打开抖音，看看【作者的作品】《世界》第一季合集 https://v.douyin.com/fixture/ aNW:/ i@p.QX :9pm 02/07").single()
        assertEquals("《世界》第一季合集", douyin.title); assertEquals("作者", douyin.author)
        val url = "https://app.binance.com/uni-qr/cpos/123456?r=synthetic&l=zh-CN"
        val item = SocialLinkPolicy.extract(url).single()
        assertEquals(url, item.url); assertEquals("币安广场", item.site)
        assertTrue(SocialLinkShareText.compact(url, listOf(item)))
        assertEquals("币安广场帖子", SocialLinkPresentation.title(item))
        assertNull(SocialLinkPolicy.link("https://app.binance.com.evil.example/a"))
        assertEquals("小红书 · 作者", SocialLinkPresentation.source(item.copy(site = "小红书", author = "作者")))
        assertEquals("X · @example", SocialLinkPresentation.source(SocialLinkPolicy.link("https://x.com/example/status/123456")!!))
        for ((seconds, label) in listOf(80 to "01:20", 3680 to "1:01:20", 0 to "00:00")) {
            val video = SocialLinkPolicy.link("https://www.bilibili.com/video/BV1enYL6SEtU/?t=$seconds&p=3")!!
            assertEquals("从 $label 开始", SocialLinkPresentation.time(video))
        }
    }
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
