package com.elon.app.sociallinks

import android.content.Context
import org.robolectric.RuntimeEnvironment
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [28], application = android.app.Application::class)
class SocialLinkReadStoreTest {
    private val context: Context = RuntimeEnvironment.getApplication()
    private val original = "https://app.binance.com/uni-qr/cpos/123456?r=synthetic"
    private fun value(url: String = original) = JSONObject().put("schema", 1).put("original", url)
        .put("url", "https://www.binance.com/zh-CN/square/post/123456").put("article", true)
        .put("title", "一段公开内容的开头").put("author", "示例作者").put("image", "https://public.bnbstatic.com/image/cms/content.png")
    @Before fun clear() { context.getSharedPreferences("social-link-read-preview-v1", Context.MODE_PRIVATE).edit().clear().commit() }
    @Test fun aliasesMustKeepSameContentIdentityAndAllowedImage() {
        assertNotNull(SocialLinkReadPreview.parse(original, value()))
        assertEquals("https://www.binance.com/en/square/post/123456?r=synthetic", SocialLinkReadIdentity.readingUrl(original))
        assertNull(SocialLinkReadPreview.parse(original, value().put("url", "https://www.binance.com/en/square/post/999999")))
        assertNull(SocialLinkReadPreview.parse(original, value().put("url", "https://app.binance.com.evil.example/uni-qr/cpos/123456")))
        assertNull(SocialLinkReadPreview.parse(original, value().put("image", "https://public.bnbstatic.com/image/avatar.png"))?.image)
        assertNull(SocialLinkReadPreview.parse(original, value().put("title", "Sign in to Binance")))
        assertEquals("x-post:123456", SocialLinkReadIdentity.identity("https://twitter.com/author/status/123456?s=20"))
        assertEquals("x-article:123456", SocialLinkReadIdentity.identity("https://x.com/i/article/123456"))
        assertNull(SocialLinkReadIdentity.identity("https://x.com/author/status/123456/photo/1"))
        assertNull(SocialLinkReadIdentity.image("https://pbs.twimg.com/profile_images/123456/pic.jpg", "x-post"))
    }
    @Test fun diskRecordIsAccountScopedAndReadDoesNotExtendExpiry() {
        SocialLinkReadStore.put(context, "server-a", "alice", value(), 1000)
        assertEquals("一段公开内容的开头", SocialLinkReadStore.get(context, "server-a", "alice", original, 1001)?.title)
        assertNull(SocialLinkReadStore.get(context, "server-a", "bob", original, 1001))
        assertNull(SocialLinkReadStore.get(context, "server-b", "alice", original, 1001))
        assertNotNull(SocialLinkReadStore.get(context, "server-a", "alice", original, 86400999))
        assertNull(SocialLinkReadStore.get(context, "server-a", "alice", original, 86401000))
    }
    @Test fun capacityAndPersistenceOnlyKeepMetadata() {
        for (i in 100000..100128) {
            val url = "https://www.binance.com/en/square/post/$i"
            SocialLinkReadStore.put(context, "server", "alice", value(url).put("url", url).put("cookie", "must-not-be-stored"), i.toLong())
        }
        val all = context.getSharedPreferences("social-link-read-preview-v1", Context.MODE_PRIVATE).all
        assertEquals(128, all.size)
        assertFalse(all.values.any { it.toString().contains("must-not-be-stored") })
        assertNull(SocialLinkReadStore.get(context, "server", "alice", "https://www.binance.com/en/square/post/100000", 100129))
        assertNotNull(SocialLinkReadStore.get(context, "server", "alice", "https://www.binance.com/en/square/post/100128", 100129))
    }
}
