package com.elon.app.sociallinks

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class BilibiliPublicPreviewTest {
    private val item = SocialLinkPolicy.link("https://www.bilibili.com/video/BV19eYH6NEsC/?t=80&p=2")!!
    private fun value() = JSONObject().put("code", 0).put("data", JSONObject()
        .put("bvid", "BV19eYH6NEsC").put("title", "Public video")
        .put("pic", "http://i0.hdslb.com/bfs/archive/poster.jpg")
        .put("owner", JSONObject().put("name", "Public author")))

    @Test fun keepsOriginalUrlAndPlayerWhileAddingValidatedMetadata() {
        val result = BilibiliPublicPreview.parse(item, value())!!
        assertEquals(item.url, result.url); assertEquals(item.player, result.player)
        assertEquals("https://i0.hdslb.com/bfs/archive/poster.jpg", result.image)
        assertEquals("Public author", result.author); assertTrue(result.ready)
    }
    @Test fun rejectsBusinessErrorsAndDifferentVideo() {
        assertNull(BilibiliPublicPreview.parse(item, value().put("code", -412)))
        val wrong = value(); wrong.getJSONObject("data").put("bvid", "BV1BEY96vEjJ")
        assertNull(BilibiliPublicPreview.parse(item, wrong))
        val blank = value(); blank.getJSONObject("data").put("title", " ")
        assertNull(BilibiliPublicPreview.parse(item, blank))
    }
    @Test fun rejectsUnsafeImagesAndUnrelatedProviders() {
        for (image in listOf("https://hdslb.com.evil.example/bfs/archive/x.jpg", "https://i0.hdslb.com/bfs/face/avatar.jpg", "http://127.0.0.1/x.jpg")) {
            val response = value(); response.getJSONObject("data").put("pic", image)
            assertNull(BilibiliPublicPreview.parse(item, response))
        }
        for (url in listOf("https://www.bilibili.com.evil.example/video/BV19eYH6NEsC/", "https://b23.tv/example", "https://www.bilibili.com/")) {
            assertNull(BilibiliPublicPreview.videoId(SocialLinkPolicy.link(url)!!))
        }
    }
}
