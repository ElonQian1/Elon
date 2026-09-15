package com.elon.app.sharing

import org.junit.Assert.*
import org.junit.Test
import org.json.JSONObject

class SourceLinkTest {
    @Test fun preservesArticleQueryAndClassifiesExactHost() {
        val url = "https://mp.weixin.qq.com/s/test?start=1&end=2&scene=90"
        assertEquals(url, SourceLink.webUrl(url))
        assertEquals("阅读原文", SourceLink(url).label)
        assertEquals("打开链接", SourceLink("https://mp.weixin.qq.com.evil.test/s/test").label)
        assertEquals("打开链接", SourceLink("https://mp.weixin.qq.com/account").label)
        assertEquals(SourceLink(url), SourceLink.fromJson(SourceLink(url).json()))
    }
    @Test fun rejectsUntrustedSchemesCredentialsControlsAndOversizedMetadata() {
        listOf("javascript:alert(1)", "file:///sdcard/test", "intent://open", "/article", "https://a.test\\@b.test", "https://user:password@a.test/", "https://a.test/\nnext", "https://a.test/" + "x".repeat(4096)).forEach { assertNull(it, SourceLink.webUrl(it)) }
        assertNull(SourceLink.fromJson(JSONObject("""{"version":2,"url":"https://a.test","method":"qr"}""")))
        assertNull(SourceLink.fromJson(JSONObject("""{"version":1,"url":"https://a.test","method":"server"}""")))
        assertNull(SourceLink.fromJson(null))
    }
    @Test fun plainTextRemainsText() {
        assertNull(SourceLink.fromText("这是一段普通聊天"))
        assertNull(SourceLink.fromText("https://a.test https://b.test"))
        assertEquals("share", SourceLink.fromText(" https://a.test/a ")?.method)
    }
}
