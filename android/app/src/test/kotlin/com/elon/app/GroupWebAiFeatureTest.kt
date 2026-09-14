package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.GroupWebAiExecutor

class GroupWebAiFeatureTest {
    @Test fun mentionsMatchLegacyWithoutWhitespaceRequirement() {
        assertTrue(GroupWebAiFeature.mentionsAi("@EL help"))
        assertTrue(GroupWebAiFeature.mentionsAi("＠el帮我分析"))
        assertTrue(GroupWebAiFeature.mentionsAi("@AI help"))
        assertFalse(GroupWebAiFeature.mentionsAi("hello everyone"))
    }

    @Test fun onlyCompletedMatchingReplyCanBePublished() {
        val messages = listOf(
            ChatGptWebMessage("u", "user", "group prompt", "completed", emptyList()),
            ChatGptWebMessage("a", "assistant", "answer", "streaming", emptyList()),
        )
        assertNull(GroupWebAiExecutor.completedReply(messages, "other prompt", false, "completed"))
        assertNull(GroupWebAiExecutor.completedReply(messages, "group prompt", true, "completed"))
        assertNull(GroupWebAiExecutor.completedReply(messages, "group prompt", false, "idle"))
        assertEquals("answer", GroupWebAiExecutor.completedReply(messages, "group prompt", false, "completed"))
        assertEquals("answer", GroupWebAiExecutor.completedReply(messages, "group\n\nprompt", false, "completed"))
    }

    @Test fun oldReplyBeforeSourceIsNeverPublished() {
        val messages = listOf(
            ChatGptWebMessage("a", "assistant", "old answer", "completed", emptyList()),
            ChatGptWebMessage("u", "user", "group prompt", "completed", emptyList()),
        )
        assertNull(GroupWebAiExecutor.completedReply(messages, "group prompt", false, "completed"))
    }
}
