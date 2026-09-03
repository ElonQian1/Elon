package com.elon.app.chatgptweb

import com.elon.app.WebChatNativeDictationPhase
import com.elon.app.WebChatNativeDictationScheduler
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateDictationTransportTest {
    @Test
    fun confirmedCaptureAndTranscriptUpdateNativeDraft() {
        var nativeDraft = "hello"
        var started = false
        var submitted = false
        val transport = transport(
            readDraft = { nativeDraft },
            writeDraft = { nativeDraft = it },
            dispatchStart = { _, _, _ -> started = true; true },
            dispatchSubmit = { submitted = true; true },
        )

        assertTrue(transport.ready())
        assertTrue(transport.start({}, {}))
        assertTrue(started)
        assertEquals(WebChatNativeDictationPhase.STARTING, transport.state().phase)

        transport.onCommandResult(
            ChatGptWebPrivateDictationTransport.START_ACTION,
            true,
            "capture_started",
        )
        assertEquals(WebChatNativeDictationPhase.LISTENING, transport.state().phase)
        assertTrue(transport.submit())
        assertTrue(submitted)
        transport.onCommandResult(
            ChatGptWebPrivateDictationTransport.SUBMIT_ACTION,
            true,
            "transcript_ready:5",
        )
        assertEquals(WebChatNativeDictationPhase.PROCESSING, transport.state().phase)

        transport.observeOfficialDraft("hello world")
        assertEquals("hello world", nativeDraft)
        assertEquals(WebChatNativeDictationPhase.IDLE, transport.state().phase)
    }

    @Test
    fun failureBeforeCaptureFallsBackButCapturedFailureDoesNot() {
        var fallbackCount = 0
        val failures = mutableListOf<String>()
        val transport = transport(onFailure = failures::add)

        transport.start({}, { fallbackCount += 1 })
        transport.onCommandResult(
            ChatGptWebPrivateDictationTransport.START_ACTION,
            false,
            "before_capture:auth_missing",
        )
        assertEquals(1, fallbackCount)
        assertTrue(failures.isEmpty())

        transport.start({}, { fallbackCount += 1 })
        transport.onCommandResult(
            ChatGptWebPrivateDictationTransport.START_ACTION,
            false,
            "capture:start_failed",
        )
        assertEquals(1, fallbackCount)
        assertEquals(1, failures.size)
        assertEquals(WebChatNativeDictationPhase.IDLE, transport.state().phase)
    }

    @Test
    fun touchDispatchCannotMovePrivateSessionToListening() {
        val transport = transport()

        assertTrue(transport.start({}, {}))

        assertEquals(WebChatNativeDictationPhase.STARTING, transport.state().phase)
        assertFalse(transport.state().phase == WebChatNativeDictationPhase.LISTENING)
    }

    private fun transport(
        readDraft: () -> String = { "" },
        writeDraft: (String) -> Unit = {},
        dispatchStart: (String, String, () -> Unit) -> Boolean = { _, _, _ -> true },
        dispatchSubmit: () -> Boolean = { true },
        onFailure: (String) -> Unit = {},
    ) = ChatGptWebPrivateDictationTransport(
        enabled = true,
        readyCheck = { true },
        currentOfficialDraft = { "" },
        readDraft = readDraft,
        writeDraft = writeDraft,
        dispatchStart = dispatchStart,
        dispatchSubmit = dispatchSubmit,
        dispatchCancel = { true },
        onFailure = onFailure,
        scheduler = NoOpScheduler,
        trace = { _, _ -> },
    )

    private object NoOpScheduler : WebChatNativeDictationScheduler {
        override fun postDelayed(task: Runnable, delayMs: Long) = Unit
        override fun remove(task: Runnable) = Unit
    }
}
