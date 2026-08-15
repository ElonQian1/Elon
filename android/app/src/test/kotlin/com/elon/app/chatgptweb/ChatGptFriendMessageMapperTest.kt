package com.elon.app.chatgptweb

import com.elon.app.ChatAttachment
import com.elon.app.WebChatProviderId
import com.elon.app.WebChatProviderRegistry
import com.elon.app.WebChatMessageAction
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
        assertEquals(setOf(WebChatMessageAction.COPY), result.first().webChatMessage?.actions)
        assertEquals(setOf(WebChatMessageAction.COPY), result.last().webChatMessage?.actions)
    }

    @Test
    fun exposesRegenerateAndContextActionsOnlyWhenProductionAndPageCapabilitiesAgree() {
        val snapshot = snapshot(
            messages = listOf(
                ChatGptWebMessage("a-old", "assistant", "旧回答", "completed", emptyList()),
                ChatGptWebMessage("a-latest", "assistant", "新回答", "completed", emptyList()),
            ),
            capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.MESSAGE_REGENERATE)),
        )

        val result = ChatGptFriendMessageMapper.map(
            snapshot = snapshot,
            provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = null,
            messageActionContextIds = setOf("a-latest"),
            timestampFor = { 42L },
        )

        assertEquals(setOf(WebChatMessageAction.COPY), result.first().webChatMessage?.actions)
        assertEquals(
            setOf(WebChatMessageAction.COPY, WebChatMessageAction.REGENERATE, WebChatMessageAction.MORE),
            result.last().webChatMessage?.actions,
        )
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

    @Test
    fun keepsNativeAttachmentsOnTheOptimisticFriendChatBubble() {
        val attachment = ChatAttachment(
            kind = "file",
            displayName = "fixture.txt",
            localPath = "fixture.txt",
        )

        val result = ChatGptFriendMessageMapper.map(
            snapshot = snapshot(messages = emptyList()),
            provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = "",
            pendingAttachments = listOf(attachment),
            pendingSendStatus = "上传中…",
            timestampFor = { 42L },
        )

        assertEquals("user", result.single().role)
        assertEquals("上传中…", result.single().sendStatus)
        assertEquals("fixture.txt", result.single().attachments?.single()?.displayName)
    }

    @Test
    fun restoresOfficialFilePartsIntoTheExistingAttachmentBubble() {
        val message = ChatGptWebMessage(
            id = "u-file",
            role = "user",
            content = "查看文件",
            state = "completed",
            parts = listOf(
                ChatGptWebMessagePart(
                    type = "file",
                    label = "fixture.pdf",
                    metadata = ChatGptWebMessagePartMetadata(mediaType = "application/pdf"),
                ),
            ),
        )

        val result = ChatGptFriendMessageMapper.map(
            snapshot = snapshot(messages = listOf(message)),
            provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB),
            pendingPrompt = null,
            timestampFor = { 42L },
        )

        assertEquals("fixture.pdf", result.single().attachments?.single()?.displayName)
        assertEquals("application/pdf", result.single().attachments?.single()?.mimeType)
    }

    private fun snapshot(
        messages: List<ChatGptWebMessage>,
        capabilities: ChatGptWebCapabilities = ChatGptWebCapabilities.EMPTY,
    ) = ChatGptWebSnapshot(
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
        capabilities = capabilities,
    )
}
