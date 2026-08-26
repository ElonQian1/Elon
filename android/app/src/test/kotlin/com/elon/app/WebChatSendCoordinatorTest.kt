package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebMessage
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSendCoordinatorTest {
    @Test
    fun notReadyTransportDoesNotCreatePendingSend() {
        val fixture = Fixture(ready = false)

        val result = fixture.coordinator.dispatch("hello", null) { fixture.optimisticRenders += 1 }

        assertEquals(WebChatSendCoordinator.DispatchOutcome.NOT_READY, result.outcome)
        assertNull(fixture.coordinator.prompt())
        assertEquals(0, fixture.optimisticRenders)
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun rejectedDispatchRestoresPromptWithoutLeavingWatchdog() {
        val fixture = Fixture(dispatchAccepted = false)

        val result = fixture.coordinator.dispatch("hello", null) { fixture.optimisticRenders += 1 }

        assertEquals(WebChatSendCoordinator.DispatchOutcome.REJECTED, result.outcome)
        assertEquals("hello", result.prompt)
        assertEquals(1, fixture.optimisticRenders)
        assertNull(fixture.coordinator.prompt())
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun acceptedOfficialDispatchIsSingleFlightAndSchedulesConfirmation() {
        val fixture = Fixture()

        val first = fixture.coordinator.dispatch("hello", null) { fixture.optimisticRenders += 1 }
        val second = fixture.coordinator.dispatch("again", null) { fixture.optimisticRenders += 1 }

        assertEquals(WebChatSendAuthority.OFFICIAL_PAGE, fixture.coordinator.authority())
        assertEquals(WebChatSendCoordinator.DispatchOutcome.DISPATCHED, first.outcome)
        assertEquals(WebChatSendCoordinator.DispatchOutcome.BUSY, second.outcome)
        assertEquals(listOf("hello"), fixture.transport.prompts)
        assertEquals(1, fixture.optimisticRenders)
        assertTrue(fixture.scheduler.hasTask())
        assertEquals(WebChatPendingSendState.Phase.SUBMITTING, fixture.coordinator.phase())
    }

    @Test
    fun observedSubmissionAndCompletedTurnSettleTheSamePendingSend() {
        val fixture = Fixture()
        val baseline = snapshot(
            message("old-user", "user", "hello world"),
            message("old-assistant", "assistant", "old reply"),
        )
        fixture.coordinator.dispatch("hello world", baseline) {}

        assertEquals(
            WebChatSendCoordinator.Observation.NONE,
            fixture.coordinator.observeSnapshot(baseline),
        )
        assertEquals(WebChatPendingSendState.Phase.SUBMITTING, fixture.coordinator.phase())
        assertEquals(
            WebChatSendCoordinator.Observation.SUBMISSION_CONFIRMED,
            fixture.coordinator.observeSnapshot(
                snapshot(
                    message("old-user", "user", "hello world"),
                    message("old-assistant", "assistant", "old reply"),
                    message("new-user", "user", " hello   world "),
                ),
            ),
        )
        assertEquals(WebChatPendingSendState.Phase.AWAITING_RESPONSE, fixture.coordinator.phase())
        assertEquals("已发送 · 等待回复", fixture.coordinator.status())
        assertEquals(
            WebChatSendCoordinator.Observation.TURN_COMPLETED,
            fixture.coordinator.observeSnapshot(
                snapshot(
                    message("old-user", "user", "hello world"),
                    message("old-assistant", "assistant", "old reply"),
                    message("new-user", "user", "hello world"),
                    message("new-assistant", "assistant", "new reply"),
                ),
            ),
        )
        assertNull(fixture.coordinator.prompt())
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun repeatedPromptCannotBeConfirmedByThePreDispatchTurn() {
        val fixture = Fixture()
        val baseline = snapshot(
            message("old-user", "user", "same prompt"),
            message("old-assistant", "assistant", "old reply"),
        )
        fixture.coordinator.dispatch("same prompt", baseline) {}

        assertEquals(
            WebChatSendCoordinator.Observation.NONE,
            fixture.coordinator.observeSnapshot(baseline),
        )
        assertEquals(WebChatPendingSendState.Phase.SUBMITTING, fixture.coordinator.phase())

        val submitted = snapshot(
            message("old-user", "user", "same prompt"),
            message("old-assistant", "assistant", "old reply"),
            message("new-user", "user", "same prompt"),
        )
        assertEquals(
            WebChatSendCoordinator.Observation.SUBMISSION_CONFIRMED,
            fixture.coordinator.observeSnapshot(submitted),
        )
        assertEquals(WebChatPendingSendState.Phase.AWAITING_RESPONSE, fixture.coordinator.phase())

        assertEquals(
            WebChatSendCoordinator.Observation.TURN_COMPLETED,
            fixture.coordinator.observeSnapshot(
                snapshot(
                    message("old-user", "user", "same prompt"),
                    message("old-assistant", "assistant", "old reply"),
                    message("new-user", "user", "same prompt"),
                    message("new-assistant", "assistant", "new reply"),
                ),
            ),
        )
        assertNull(fixture.coordinator.prompt())
    }

    @Test
    fun confirmedSendReconcilesTwiceThenRequiresOfficialConfirmation() {
        val fixture = Fixture()
        fixture.coordinator.dispatch("hello", null) {}
        fixture.coordinator.acceptCommandResult(ok = true)

        fixture.scheduler.runNext()
        fixture.scheduler.runNext()
        fixture.scheduler.runNext()

        assertEquals(3, fixture.transport.reconciliations)
        assertEquals(
            listOf(WebChatPendingSendState.TimeoutAction.REQUIRE_OFFICIAL_CONFIRMATION),
            fixture.terminalActions.map(WebChatPendingSendState.TimeoutResult::action),
        )
        assertTrue(fixture.coordinator.requiresOfficialConfirmation())
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun unconfirmedSendTimesOutToRestorablePrompt() {
        val fixture = Fixture()
        fixture.coordinator.dispatch("keep this", null) {}

        fixture.scheduler.runNext()

        assertEquals(0, fixture.transport.reconciliations)
        assertEquals(WebChatPendingSendState.TimeoutAction.RESTORE, fixture.terminalActions.single().action)
        assertEquals("keep this", fixture.terminalActions.single().prompt)
        assertNull(fixture.coordinator.prompt())
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun failedCommandReceiptCancelsPendingSendAndReturnsDraft() {
        val fixture = Fixture()
        fixture.coordinator.dispatch("retry me", null) {}

        val prompt = fixture.coordinator.acceptCommandResult(ok = false)

        assertEquals("retry me", prompt)
        assertNull(fixture.coordinator.prompt())
        assertFalse(fixture.scheduler.hasTask())
    }

    @Test
    fun clearingCoordinatorInvalidatesAnAlreadyQueuedTimeout() {
        val fixture = Fixture()
        fixture.coordinator.dispatch("hello", null) {}
        val stale = fixture.scheduler.currentTask()

        fixture.coordinator.clear()
        stale?.run()

        assertTrue(fixture.terminalActions.isEmpty())
        assertEquals(0, fixture.transport.reconciliations)
        assertNull(fixture.coordinator.prompt())
    }

    private class Fixture(
        ready: Boolean = true,
        dispatchAccepted: Boolean = true,
    ) {
        val scheduler = FakeScheduler()
        val transport = FakeTransport(ready, dispatchAccepted)
        val terminalActions = mutableListOf<WebChatPendingSendState.TimeoutResult>()
        var optimisticRenders = 0
        val coordinator = WebChatSendCoordinator(
            transport = transport,
            postDelayed = scheduler::postDelayed,
            removeCallbacks = scheduler::removeCallbacks,
            onTerminalTimeout = terminalActions::add,
            confirmationTimeoutMs = 10L,
        )
    }

    private fun snapshot(vararg messages: ChatGptWebMessage) = ChatGptWebSnapshot(
        title = "",
        url = "https://example.test/conversation",
        draft = "",
        messages = messages.toList(),
        authenticated = true,
        composerReady = true,
        streaming = false,
        currentModel = "",
        attachments = emptyList(),
        dictationActive = false,
        capabilities = ChatGptWebCapabilities.EMPTY,
        observedMessageCount = messages.size,
    )

    private fun message(id: String, role: String, content: String) = ChatGptWebMessage(
        id = id,
        role = role,
        content = content,
        state = "complete",
        parts = emptyList(),
    )

    private class FakeTransport(
        var ready: Boolean,
        var dispatchAccepted: Boolean,
    ) : WebChatSendTransport {
        override val authority: WebChatSendAuthority = WebChatSendAuthority.OFFICIAL_PAGE
        val prompts = mutableListOf<String>()
        var reconciliations = 0

        override fun isReady(): Boolean = ready

        override fun dispatch(prompt: String): Boolean {
            prompts += prompt
            return dispatchAccepted
        }

        override fun reconcile() {
            reconciliations += 1
        }
    }

    private class FakeScheduler {
        private var task: Runnable? = null

        fun postDelayed(next: Runnable, @Suppress("UNUSED_PARAMETER") delayMs: Long) {
            task = next
        }

        fun removeCallbacks(expected: Runnable) {
            if (task === expected) task = null
        }

        fun hasTask(): Boolean = task != null

        fun currentTask(): Runnable? = task

        fun runNext() {
            val next = task
            task = null
            requireNotNull(next).run()
        }
    }
}
