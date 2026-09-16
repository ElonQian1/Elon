package com.elon.app.sociallinks

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class SocialLinkShareTextTest {
    private val url = "https://www.xiaohongshu.com/discovery/item/example?xsec_token=synthetic"
    private val share = "45 【一次看完多位画师卡位实拍，哪张击中你？✨ - OC猫 | 小红书 - 你的生活兴趣社区】 😆 vEtl5AZaiF1Cpm2 😆 $url"
    @Test fun preservesSpecificShareTitleWhenProviderReturnsGenericSiteTitle() {
        val item = SocialLinkPolicy.extract(share).single()
        val result = SocialLinkPolicy.merge(JSONObject().put("schema", 1).put("url", item.url).put("title", "小红书 - 你的生活兴趣社区").put("status", "ready"), item)
        assertEquals("一次看完多位画师卡位实拍，哪张击中你？✨", result.title)
        assertEquals("OC猫", result.author); assertEquals(url, result.url)
    }
    @Test fun onlyKnownShareTemplatesAreCollapsed() {
        assertTrue(SocialLinkShareText.compact(share))
        assertTrue(SocialLinkShareText.compact("3.53 复制打开抖音，看看【作者的作品】《世界》 https://v.douyin.com/_XMEsxVKKOY/ aNW:/ i@p.QX :9pm 02/07"))
        assertTrue(SocialLinkShareText.compact("https://mp.weixin.qq.com/s/example"))
        assertFalse(SocialLinkShareText.compact("我的评论 $share"))
        assertFalse(SocialLinkShareText.compact("$share 我还有话要说"))
        assertFalse(SocialLinkShareText.compact("$share https://x.com/i/status/123456"))
    }
    @Test fun readerRequiresSameArticleAndRejectsChallengeAndUnsafeCover() {
        val original = "https://mp.weixin.qq.com/s/example"
        fun value() = JSONObject().put("url", original).put("article", true).put("title", "公开文章").put("author", "公众号").put("image", "https://mmbiz.qpic.cn/image.jpg")
        assertEquals("公开文章", SocialLinkReadPreview.parse(original, value())?.title)
        assertNull(SocialLinkReadPreview.parse(original, value().put("url", "https://mp.weixin.qq.com/s/other")))
        assertNull(SocialLinkReadPreview.parse(original, value().put("title", "环境异常")))
        assertNull(SocialLinkReadPreview.parse(original, value().put("article", false)))
        assertNull(SocialLinkReadPreview.parse(original, value().put("image", "https://qpic.cn.evil.example/x"))?.image)
    }
    @Test fun readPreviewCacheIsBoundedExpiringAndAccountIsolated() {
        for (i in 0..128) SocialLinkReadPreview.remember("cache-test", "alice", SocialLink("https://mp.weixin.qq.com/s/$i", "微信公众号", "标题"), 100)
        assertNull(SocialLinkReadPreview.cached("cache-test", "alice", "https://mp.weixin.qq.com/s/0", 101))
        assertNotNull(SocialLinkReadPreview.cached("cache-test", "alice", "https://mp.weixin.qq.com/s/128", 101))
        assertNull(SocialLinkReadPreview.cached("cache-test", "bob", "https://mp.weixin.qq.com/s/128", 101))
        assertNull(SocialLinkReadPreview.cached("cache-test", "alice", "https://mp.weixin.qq.com/s/128", 86400101))
    }
}
