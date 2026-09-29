package com.elon.app.socialquotes

import com.elon.app.ChatAttachment
import com.elon.app.ChatMessage
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
@org.robolectric.annotation.Config(sdk = [34], application = android.app.Application::class)
class SocialQuoteTest {
    @Test fun structuredQuoteNeverChangesTypedText() {
        val json = JSONObject().put("quote", JSONObject().put("message_id", "m1").put("sender_name", "Alice")
            .put("content", "https://example.com/a").put("revision", 2))
        val message = SocialQuoteCodec.bind(ChatMessage("friend", "可以吗"), json)
        assertEquals("可以吗", message.content)
        assertEquals("m1", message.quote?.messageId)
        assertEquals(2, message.quote?.source()?.getInt("revision"))
    }
    @Test fun legacyQuoteMovesOutOfBodyButAiMarkdownIsUntouched() {
        val content = "> 引用 Alice · 第 2 版\n> https://example.com/a\n\n可以吗"
        val message = SocialQuoteCodec.bind(ChatMessage("friend", content), JSONObject())
        assertEquals("可以吗", message.content)
        assertEquals("Alice", message.quote?.senderName)
        assertEquals(content, SocialQuoteCodec.bind(ChatMessage("ai", content), JSONObject()).content)
        assertNull(SocialQuoteCodec.split("> markdown only").second)
        assertEquals("单行回复", SocialQuoteCodec.split("> 引用摘要\n单行回复").first)
    }
    @Test fun replyToReplyUsesOnlyImmediateMessage() {
        val quote = SocialQuoteCodec.from(ChatMessage("user", "new body", id = "m2", quote = SocialQuote("m1", "Alice", "old body")), "我")
        assertEquals("new body", quote.content)
        assertEquals("m2", quote.messageId)
        assertEquals("原消息已撤回或不可用", SocialQuoteCodec.summary(quote.copy(unavailable = true)))
        assertTrue(SocialQuoteCodec.summary(quote.copy(content = "", attachments = listOf(ChatAttachment(kind="image")))).startsWith("[图片]"))
    }
}
