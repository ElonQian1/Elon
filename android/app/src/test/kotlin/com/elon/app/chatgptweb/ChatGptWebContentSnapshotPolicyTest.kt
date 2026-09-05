package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Test

class ChatGptWebContentSnapshotPolicyTest {
    @Test
    fun privateHistoryOnlyUpdatesContentNotTheLiveInteractionState() {
        val previous = snapshot().copy(
            draft = "Unsent draft",
            attachments = listOf(ChatGptWebAttachment("file-1", "fixture.txt", "uploading", true)),
            dictationActive = true,
            dictationCaptureActive = true,
            privateReadAloudReady = true,
            privateReadAloudState = "playing",
            privateReadAloudContextId = "context-1",
        )
        val incoming = history().copy(
            title = "Updated title",
            url = "https://chatgpt.com/g/g-p-test/c/example",
        )
        val result = ChatGptWebContentSnapshotPolicy.reconcile(previous, incoming)
        assertEquals(previous.copy(
            title = incoming.title,
            url = incoming.url,
            messages = incoming.messages,
            observedMessageCount = incoming.observedMessageCount,
        ), result)
    }

    @Test
    fun aHistoryResponseCannotReplaceTheCurrentStreamingAnswer() {
        val previous = snapshot().copy(streaming = true)
        assertSame(previous, ChatGptWebContentSnapshotPolicy.reconcile(previous, history()))
    }

    @Test
    fun anotherConversationDoesNotInheritComposerOrVoiceState() {
        val incoming = history().copy(url = "https://chatgpt.com/c/another")
        assertSame(incoming, ChatGptWebContentSnapshotPolicy.reconcile(snapshot(), incoming))
    }

    @Test
    fun aRealOfficialSnapshotStillUpdatesInteractionState() {
        val incoming = history().copy(contentOnly = false, loginRequired = true)
        assertSame(incoming, ChatGptWebContentSnapshotPolicy.reconcile(snapshot(), incoming))
    }

    @Test
    fun firstHistoryResultIsRenderableButDoesNotClaimTheComposerIsReady() {
        val incoming = history()
        assertSame(incoming, ChatGptWebContentSnapshotPolicy.reconcile(null, incoming))
        assertEquals(false, incoming.composerReady)
    }

    @Test
    fun unknownOrExternalRoutesCannotBorrowReadyState() {
        for (url in listOf("https://example.com/c/example", "https://chatgpt.com/images")) {
            val incoming = history().copy(url = url)
            assertSame(incoming, ChatGptWebContentSnapshotPolicy.reconcile(snapshot(), incoming))
        }
    }

    private fun history() = snapshot().copy(
        contentOnly = true,
        composerReady = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        currentModel = "",
        messages = listOf(ChatGptWebMessage("reply", "assistant", "History reply", "completed", emptyList())),
        observedMessageCount = 1,
    )

    private fun snapshot() = ChatGptWebSnapshot(
        title = "Test",
        url = "https://chatgpt.com/c/example",
        draft = "",
        messages = emptyList(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "Auto",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities(setOf(ChatGptWebCapabilityId.COMPOSER_TOOLS)),
        pageKind = "conversation",
    )
}
