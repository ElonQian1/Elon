package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatSendSnapshotIdentityTest {
    @Test
    fun anotherConversationCannotConfirmTheSamePrompt() {
        val fixture = Fixture(snapshot("/c/first", "old-user"))
        fixture.observeIgnored(snapshot("/c/second", "other-user", count = 8))
        fixture.complete(snapshot("/c/first", "new-user"))
    }

    @Test
    fun historyExpansionCannotConfirmTheOldUserMessageAgain() {
        val fixture = Fixture(snapshot("/c/first", "old-user", count = 2))
        fixture.observeIgnored(snapshot("/c/first", "old-user", count = 20))
        fixture.complete(snapshot("/c/first", "new-user", count = 22))
    }

    @Test
    fun temporaryMissingRouteCannotReleaseThePendingCommand() {
        val fixture = Fixture(snapshot("/c/first", "old-user"))
        fixture.observeIgnored(snapshot("/", "new-user"))
        fixture.observeIgnored(snapshot("/c/first", "new-user").copy(url = "https://example.test/c/first"))
        fixture.complete(snapshot("/c/first", "new-user"))
    }

    @Test
    fun missingUserIdentityCannotUseMessageCountAsProof() {
        val fixture = Fixture(snapshot("/c/first", "old-user"))
        fixture.observeIgnored(snapshot("/c/first", "", count = 20))
        fixture.complete(snapshot("/c/first", "new-user"))
    }

    @Test
    fun projectAndPersonalRoutesForTheSameConversationAreEquivalent() {
        val fixture = Fixture(snapshot("/g/g-p-project/c/first", "old-user"))
        fixture.complete(snapshot("/c/first?model=auto", "new-user"))
    }

    @Test
    fun aNewUserMessageCanConfirmInASmallerWindow() {
        val fixture = Fixture(snapshot("/c/first", "old-user", count = 40))
        fixture.complete(snapshot("/c/first", "new-user", count = 2))
    }

    @Test
    fun newConversationCanReceiveItsFirstAssignedRoute() {
        val fixture = Fixture(snapshot("/", "").copy(messages = emptyList(), observedMessageCount = 0))
        fixture.complete(snapshot("/c/created", "new-user"))
    }

    @Test
    fun providerWithoutStableConversationIdentityKeepsItsExistingPolicy() {
        val baseline = snapshot("/", "same-user").copy(url = "https://www.google.com/search?udm=50")
        val fixture = Fixture(baseline)
        fixture.complete(baseline.copy(observedMessageCount = 4))
    }

    private class Fixture(baseline: ChatGptWebSnapshot) {
        var dispatches = 0
        var reconciliations = 0
        private val tasks = mutableListOf<Runnable>()
        private val coordinator = WebChatSendCoordinator(
            transport = object : WebChatSendTransport {
                override val authority = WebChatSendAuthority.SAME_ORIGIN_PRIVATE
                override fun isReady() = true
                override fun dispatch(command: WebChatSendCommand): WebChatTransportDispatchResult {
                    dispatches += 1
                    return WebChatTransportDispatchResult.QUEUED
                }
                override fun reconcile() { reconciliations += 1 }
            },
            postDelayed = { task, _ -> tasks.add(task) },
            removeCallbacks = { tasks.remove(it) },
            onTerminalTimeout = {},
        )

        init {
            assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED,
                coordinator.dispatch("fixture prompt", baseline) {}.outcome)
        }

        fun observeIgnored(snapshot: ChatGptWebSnapshot) {
            assertEquals(WebChatSendCoordinator.Observation.NONE, coordinator.observeSnapshot(snapshot))
            assertNotNull(coordinator.prompt())
            assertEquals(WebChatSendCoordinator.DispatchOutcome.BUSY,
                coordinator.dispatch("fixture prompt", snapshot) {}.outcome)
            assertEquals(1, tasks.size)
            assertEquals(1, dispatches)
            assertEquals(0, reconciliations)
        }

        fun complete(snapshot: ChatGptWebSnapshot) {
            assertEquals(WebChatSendCoordinator.Observation.TURN_COMPLETED, coordinator.observeSnapshot(snapshot))
            assertNull(coordinator.prompt())
            assertEquals(0, tasks.size)
            assertEquals(1, dispatches)
            assertEquals(0, reconciliations)
        }
    }

    private fun snapshot(path: String, userId: String, count: Int = 2) = ChatGptWebSnapshot(
        title = "",
        url = "https://chatgpt.com$path",
        draft = "",
        messages = listOf(
            ChatGptWebMessage(userId, "user", "fixture prompt", "completed", emptyList()),
            ChatGptWebMessage("answer-$userId", "assistant", "fixture answer", "completed", emptyList()),
        ),
        authenticated = true,
        composerReady = false,
        privateSendReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        observedMessageCount = count,
    )
}
