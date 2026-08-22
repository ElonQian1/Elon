package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebProjectTitlePolicyTest {
    @Test
    fun keepsStableCachedTitleWhenDomFallsBackToGenericChatLabel() {
        assertEquals(
            "投资 加密货币",
            ChatGptWebProjectTitlePolicy.prefer("投资 加密货币", "聊天"),
        )
        assertEquals(
            "投资 加密货币",
            ChatGptWebProjectTitlePolicy.prefer("投资 加密货币", "Chat"),
        )
    }

    @Test
    fun rejectsGenericLabelsForNewProjects() {
        assertNull(ChatGptWebProjectTitlePolicy.normalize("聊天"))
        assertNull(ChatGptWebProjectTitlePolicy.normalize("Projects"))
    }
}
