package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceContextTest {
    @Test
    fun projectConversationShowsItsProjectAndConversationWithoutExposingTheRawPath() {
        val context = WebChatRealtimeVoiceContextPolicy.resolve(
            conversationPath = "/g/g-p-project/c/conversation-id",
            conversations = listOf(ChatGptWebConversation(
                id = "conversation-id",
                title = "父母体检记录",
                path = "/g/g-p-project/c/conversation-id",
                active = true,
                projectId = "g-p-project",
                projectTitle = "家庭成员健康",
            )),
            temporaryChat = false,
        )

        assertEquals("家庭成员健康 / 父母体检记录", context.label)
        assertEquals("/g/g-p-project/c/conversation-id", context.conversationPath)
        assertTrue(context.savedToHistory)
    }

    @Test
    fun temporaryVoiceExplainsThatItsTranscriptIsNotSavedToHistory() {
        val context = WebChatRealtimeVoiceContextPolicy.resolve(
            conversationPath = "/c/private-id",
            conversations = emptyList(),
            temporaryChat = true,
        )

        assertEquals("临时聊天（不保存到历史）", context.label)
        assertNull(context.conversationPath)
        assertFalse(context.savedToHistory)
    }

    @Test
    fun unknownNewConversationUsesAStableLocalLabel() {
        val context = WebChatRealtimeVoiceContextPolicy.resolve(
            conversationPath = null,
            conversations = emptyList(),
            temporaryChat = false,
        )

        assertEquals("新会话（发送后自动归档）", context.label)
        assertTrue(context.savedToHistory)
    }
}
