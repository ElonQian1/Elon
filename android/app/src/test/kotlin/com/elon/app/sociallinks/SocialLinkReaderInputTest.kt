package com.elon.app.sociallinks

import org.junit.Assert.*
import org.junit.Test

class SocialLinkReaderInputTest {
    @Test fun restoredShortLinkKeepsResolvedVideo() {
        val link = SocialLinkReaderInput.parse("https://v.douyin.com/fixture/", "Title", "https://open.douyin.com/player/video?vid=12345678&autoplay=0", null)!!
        assertEquals("https://www.douyin.com/video/12345678", SocialLinkReadIdentity.readSource(link))
        assertEquals("https://v.douyin.com/fixture/", link.url)
        assertEquals("Title", link.title)
    }
    @Test fun unrelatedEmbedsCannotOverrideOriginal() {
        for (player in listOf("https://open.douyin.com.evil.example/player/video?vid=12345678", "https://open.douyin.com/player/other?vid=12345678", "https://open.douyin.com/player/video?vid=oops")) {
            assertNull(SocialLinkReaderInput.parse("https://v.douyin.com/fixture/", "", player, null)?.player)
        }
        assertEquals("https://www.douyin.com/video/99999999", SocialLinkReadIdentity.readSource(SocialLinkReaderInput.parse("https://www.douyin.com/video/99999999", "", "https://open.douyin.com/player/video?vid=12345678", null)!!))
        assertNull(SocialLinkReaderInput.parse("https://example.com/", "", "https://open.douyin.com/player/video?vid=12345678", "12345678")?.player)
    }
    @Test fun otherSupportedEmbedsKeepPositionAndIdentity() {
        val bili = SocialLinkReaderInput.parse("https://b23.tv/fixture", "", "https://player.bilibili.com/player.html?bvid=BV19eYH6NEsC&t=80&autoplay=1", null)!!
        assertTrue(bili.player!!.contains("t=80")); assertTrue(bili.player!!.contains("autoplay=0"))
        assertEquals("12345678", SocialLinkReaderInput.parse("https://t.co/fixture", "", null, "12345678")?.xId)
        assertNull(SocialLinkReaderInput.parse("https://example.com/", "", null, "12345678")?.xId)
    }
}
