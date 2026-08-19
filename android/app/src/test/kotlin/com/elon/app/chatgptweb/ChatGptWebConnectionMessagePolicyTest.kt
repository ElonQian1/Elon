package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebConnectionMessagePolicyTest {
    @Test
    fun firstConnectionShowsStatusWhenTheTranscriptIsEmpty() {
        assertTrue(ChatGptWebConnectionMessagePolicy.shouldShow(
            state = ChatGptBackgroundSession.State.LOADING,
            hasMessages = false,
            conversationNavigationActive = false,
        ))
    }

    @Test
    fun newConversationDoesNotPretendToReconnect() {
        assertFalse(ChatGptWebConnectionMessagePolicy.shouldShow(
            state = ChatGptBackgroundSession.State.LOADING,
            hasMessages = false,
            conversationNavigationActive = true,
        ))
    }

    @Test
    fun cachedConversationDoesNotReceiveAConnectingMessage() {
        assertFalse(ChatGptWebConnectionMessagePolicy.shouldShow(
            state = ChatGptBackgroundSession.State.LOADING,
            hasMessages = true,
            conversationNavigationActive = false,
        ))
    }
}
