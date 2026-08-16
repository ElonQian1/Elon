package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Test

class WebChatProductionHistoryNoticeTest {
    private val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)

    @Test
    fun `complete history keeps mapped messages unchanged`() {
        val messages = listOf(ChatMessage(role = "friend", content = "answer", id = "answer"))

        val result = WebChatProductionHistoryNotice.prepend(
            snapshot = snapshot(windowStart = 0, messageCount = 1),
            provider = provider,
            messages = messages,
            timestampFor = { 1L },
        )

        assertSame(messages, result)
    }

    @Test
    fun `truncated history adds provider notice and official fallback part`() {
        val messages = listOf(ChatMessage(role = "friend", content = "answer", id = "answer"))

        val result = WebChatProductionHistoryNotice.prepend(
            snapshot = snapshot(windowStart = 18, messageCount = 32),
            provider = provider,
            messages = messages,
            timestampFor = { 42L },
        )

        assertEquals(2, result.size)
        assertEquals("当前显示最近 32 条消息，较早的 18 条消息未在本机加载。", result.first().content)
        assertEquals(provider.displayName, result.first().senderLabel)
        assertEquals(provider.avatarResId, result.first().senderAvatarResId)
        assertEquals("chatgpt_web:history-before-18", result.first().id)
        assertEquals(42L, result.first().createdAtMs)
        assertEquals("chatgpt_web", result.first().webChatMessage?.providerWireValue)
        assertEquals("history-before-18", result.first().webChatMessage?.sourceMessageId)
        assertEquals(emptySet<WebChatMessageAction>(), result.first().webChatMessage?.actions)
        assertEquals(
            WebChatProductionContentPart(
                type = "interactive",
                label = "在官网查看完整会话",
            ),
            result.first().webChatMessage?.contentParts?.single(),
        )
        assertSame(messages.first(), result.last())
    }

    @Test
    fun `empty local window still offers complete history fallback`() {
        val result = WebChatProductionHistoryNotice.prepend(
            snapshot = snapshot(windowStart = 8, messageCount = 0),
            provider = provider,
            messages = emptyList(),
            timestampFor = { 7L },
        )

        assertEquals("较早的 8 条消息暂未在本机加载。", result.single().content)
        assertEquals("在官网查看完整会话", result.single().webChatMessage?.contentParts?.single()?.label)
    }

    private fun snapshot(windowStart: Int, messageCount: Int) = ChatGptWebSnapshot(
        title = "Conversation",
        url = "https://chatgpt.com/c/test",
        draft = "",
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "Fast",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = com.elon.app.chatgptweb.ChatGptWebCapabilities.EMPTY,
        messages = List(messageCount) { index ->
            ChatGptWebMessage(
                id = "message-$index",
                role = if (index % 2 == 0) "user" else "assistant",
                content = "content-$index",
                state = "completed",
                parts = emptyList(),
            )
        },
        loginRequired = false,
        messageWindowStart = windowStart,
        observedMessageCount = windowStart + messageCount,
    )
}
