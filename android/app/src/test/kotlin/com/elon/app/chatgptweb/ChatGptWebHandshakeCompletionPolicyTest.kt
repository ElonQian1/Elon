package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebHandshakeCompletionPolicyTest {
    @Test
    fun adapterReadyDoesNotStopSnapshotRetries() {
        val event = ChatGptWebEvent.AdapterReady(ChatGptWebCapabilities.EMPTY)

        assertFalse(ChatGptWebHandshakeCompletionPolicy.completes(event))
    }

    @Test
    fun snapshotStopsRetriesEvenWhenThePageRequiresLogin() {
        val event = ChatGptWebEvent.Snapshot(
            ChatGptWebSnapshot(
                title = "",
                url = "https://chatgpt.com/",
                draft = "",
                messages = emptyList(),
                authenticated = false,
                composerReady = false,
                streaming = false,
                currentModel = "",
                attachments = emptyList(),
                dictationActive = false,
                capabilities = ChatGptWebCapabilities.EMPTY,
                pageKind = "login",
                loginRequired = true,
            ),
        )

        assertTrue(ChatGptWebHandshakeCompletionPolicy.completes(event))
    }
}
