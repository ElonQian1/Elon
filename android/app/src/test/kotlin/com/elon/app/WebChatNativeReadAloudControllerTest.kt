package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotSame
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatNativeReadAloudControllerTest {
    @Test
    fun advancesEveryChunkAndIgnoresDuplicateCompletion() {
        val speakers = mutableListOf<FakeSpeaker>()
        val controller = controller(speakers = speakers)

        assertEquals(
            WebChatNativeReadAloudResult.STARTED,
            controller.toggle("message", "一".repeat(400)),
        )
        val speaker = speakers.single()
        assertEquals(1, speaker.calls.size)
        val staleDone = speaker.calls.first().onDone

        staleDone()
        assertEquals(2, speaker.calls.size)
        staleDone()
        assertEquals(2, speaker.calls.size)
        speaker.calls[1].onDone()
        assertEquals(3, speaker.calls.size)
        speaker.calls[2].onDone()

        assertFalse(controller.isActive("message"))
    }

    @Test
    fun stopInvalidatesPendingCompletion() {
        val speakers = mutableListOf<FakeSpeaker>()
        val controller = controller(speakers = speakers)
        controller.toggle("message", "需要停止的朗读")
        val staleDone = speakers.single().calls.single().onDone

        controller.stop()
        staleDone()

        assertFalse(controller.isActive("message"))
        assertEquals(1, speakers.single().stopCount)
        assertEquals(1, speakers.single().calls.size)
    }

    @Test
    fun failureClearsStateAndNextToggleCreatesFreshSpeaker() {
        val speakers = mutableListOf<FakeSpeaker>()
        var failures = 0
        val controller = controller(speakers = speakers, onFailure = { failures += 1 })
        controller.toggle("first", "第一次")

        speakers.single().calls.single().onError()

        assertFalse(controller.isActive("first"))
        assertEquals(1, failures)
        assertEquals(1, speakers.single().releaseCount)

        controller.toggle("second", "第二次")
        assertEquals(2, speakers.size)
        assertNotSame(speakers[0], speakers[1])
        assertTrue(controller.isActive("second"))
    }

    @Test
    fun watchdogFailureDoesNotLeaveMessageActive() {
        val speakers = mutableListOf<FakeSpeaker>()
        val scheduler = FakeScheduler()
        var failures = 0
        val controller = controller(speakers, scheduler) { failures += 1 }
        controller.toggle("message", "等待超时")

        scheduler.fireLatest()

        assertFalse(controller.isActive("message"))
        assertEquals(1, failures)
        assertEquals(1, speakers.single().releaseCount)
    }

    @Test
    fun synchronousPlaybackFailureSettlesAndRebuildsTheSpeaker() {
        val first = FakeSpeaker(throwOnNextSpeak = true)
        val second = FakeSpeaker()
        val speakers = ArrayDeque(listOf(first, second))
        var failures = 0
        val controller = WebChatNativeReadAloudController(
            speakerFactory = { speakers.removeFirst() },
            scheduler = FakeScheduler(),
            onFailure = { failures += 1 },
        )

        controller.toggle("first", "第一次")

        assertFalse(controller.isActive("first"))
        assertEquals(1, failures)
        assertEquals(1, first.releaseCount)

        controller.toggle("second", "第二次")
        assertTrue(controller.isActive("second"))
        assertEquals(1, second.calls.size)
    }

    @Test
    fun secondToggleStopsTheSameMessage() {
        val speakers = mutableListOf<FakeSpeaker>()
        val controller = controller(speakers = speakers)
        controller.toggle("message", "正在朗读")

        assertEquals(WebChatNativeReadAloudResult.STOPPED, controller.toggle("message", "正在朗读"))
        assertFalse(controller.isActive("message"))
        assertEquals(1, speakers.single().stopCount)
    }

    private fun controller(
        speakers: MutableList<FakeSpeaker>,
        scheduler: FakeScheduler = FakeScheduler(),
        onFailure: () -> Unit = {},
    ) = WebChatNativeReadAloudController(
        speakerFactory = { FakeSpeaker().also(speakers::add) },
        scheduler = scheduler,
        onFailure = onFailure,
    )

    private class FakeSpeaker(
        private var throwOnNextSpeak: Boolean = false,
    ) : WebChatReadAloudSpeaker {
        val calls = mutableListOf<Call>()
        var stopCount = 0
        var releaseCount = 0

        override fun speak(text: String, onDone: () -> Unit, onError: () -> Unit) {
            if (throwOnNextSpeak) {
                throwOnNextSpeak = false
                throw IllegalStateException("synthetic playback failure")
            }
            calls += Call(text, onDone, onError)
        }

        override fun stop() {
            stopCount += 1
        }

        override fun release() {
            releaseCount += 1
        }

        data class Call(val text: String, val onDone: () -> Unit, val onError: () -> Unit)
    }

    private class FakeScheduler : WebChatReadAloudScheduler {
        private val scheduled = mutableListOf<Scheduled>()

        override fun post(task: () -> Unit) = task()

        override fun postDelayed(delayMs: Long, task: () -> Unit): WebChatReadAloudCancellation {
            val scheduledTask = Scheduled(delayMs, task)
            scheduled += scheduledTask
            return WebChatReadAloudCancellation { scheduledTask.cancelled = true }
        }

        fun fireLatest() {
            scheduled.last { !it.cancelled }.task()
        }

        private data class Scheduled(
            val delayMs: Long,
            val task: () -> Unit,
            var cancelled: Boolean = false,
        )
    }
}
