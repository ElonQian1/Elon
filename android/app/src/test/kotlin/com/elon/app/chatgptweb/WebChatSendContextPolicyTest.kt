package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSendContextPolicyTest {
    @Test
    fun cachedTargetRemainsReadOnlyUntilOfficialNavigationCompletes() {
        assertFalse(allows(navigationPending = true, selected = "/c/target", observed = "/c/target"))
    }

    @Test
    fun mismatchedOfficialConversationRemainsReadOnly() {
        assertFalse(allows(navigationPending = false, selected = "/c/target", observed = "/c/old"))
    }

    @Test
    fun matchingLiveConversationCanSend() {
        assertTrue(allows(navigationPending = false, selected = "/c/target", observed = "/c/target"))
    }

    @Test
    fun emptyOfficialHomeCanSendTheFirstTurn() {
        assertTrue(allows(navigationPending = false, selected = null, observed = null))
    }

    @Test
    fun streamingConversationCannotStartAnotherTurn() {
        assertFalse(allows(navigationPending = false, selected = null, observed = null, streaming = true))
    }

    private fun allows(
        navigationPending: Boolean,
        selected: String?,
        observed: String?,
        streaming: Boolean = false,
    ) = WebChatSendContextPolicy.allows(
        sessionReady = true,
        snapshot = snapshot(streaming),
        navigationPending = navigationPending,
        selectedConversationPath = selected,
        observedConversationPath = observed,
    )

    private fun snapshot(streaming: Boolean) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com/",
        draft = "",
        messages = emptyList(),
        authenticated = false,
        composerReady = true,
        streaming = streaming,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        pageKind = "home",
    )
}
