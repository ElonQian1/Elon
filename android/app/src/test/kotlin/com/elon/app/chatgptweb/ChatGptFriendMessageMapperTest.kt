package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import com.elon.app.WebChatProviderRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptFriendMessageMapperTest {
    @Test
    fun mapsAssistantIntoTheExistingFriendChatUiIdentity() {
        val snapshot = snapshot(
            messages = listOf(
                ChatGptWebMessage("u1", "user", "你好", "completed", emptyList()),
                ChatGptWebMessage("a1", "assistant", "你好，我在。", "completed", emptyList()),
            ),
        )

        val result = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = null,
            timestampFor = { 42L },
        )

        assertEquals(listOf("user", "friend"), result.map { it.role })
        assertEquals("ChatGPT 网页 AI", result.last().senderLabel)
        assertTrue(result.last().senderAvatarResId != null)
        assertEquals("GPT-5", result.last().modelUsed)
        assertNull(result.first().senderAvatarResId)
    }

    @Test
    fun keepsAnUnobservedPromptVisibleWhileTheOfficialPageAcceptsIt() {
        val result = ChatGptFriendMessageMapper.map(
            snapshot = snapshot(messages = emptyList()),
            provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = "待发送消息",
            timestampFor = { 42L },
        )

        assertEquals(1, result.size)
        assertEquals("user", result.single().role)
        assertEquals("发送中…", result.single().sendStatus)
    }

    private fun snapshot(messages: List<ChatGptWebMessage>) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = messages,
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "GPT-5",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
    )
}
