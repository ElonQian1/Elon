package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatNativeDictationSessionTest {
    @Test
    fun partialAndFinalTextExtendTheCurrentDraftWithoutSending() {
        val engine = FakeEngine()
        var draft = "已有内容"
        val states = mutableListOf<WebChatNativeDictationPhase>()
        val unavailable = mutableListOf<String>()
        val session = WebChatNativeDictationSession(
            bridge = { engine },
            readDraft = { draft },
            writeDraft = { draft = it },
            onStateChanged = { states += it.phase },
            onUnavailable = unavailable::add,
            scheduler = FakeScheduler(),
        )

        assertTrue(session.start())
        engine.onReady()
        engine.onPartial("你好")
        assertEquals("已有内容 你好", draft)

        engine.deliverFinal("你好世界")

        assertEquals("已有内容 你好世界", draft)
        assertEquals(WebChatNativeDictationPhase.IDLE, session.state().phase)
        assertEquals(1, engine.prewarmCount)
        assertTrue(unavailable.isEmpty())
        assertTrue(WebChatNativeDictationPhase.LISTENING in states)
    }

    @Test
    fun cancelRestoresTheExactDraftAndDoesNotCommitPartialText() {
        val engine = FakeEngine()
        var draft = "保留  空格 "
        val session = session(engine, { draft }, { draft = it })

        session.start()
        engine.onPartial("临时内容")
        assertTrue(session.cancel())

        assertEquals("保留  空格 ", draft)
        assertFalse(session.state().active)
        assertEquals(1, engine.cancelCount)
    }

    @Test
    fun submitSettlesOnceAndReportsAnEmptyRecognition() {
        val engine = FakeEngine()
        val scheduler = FakeScheduler()
        val unavailable = mutableListOf<String>()
        val session = WebChatNativeDictationSession(
            bridge = { engine },
            readDraft = { "" },
            writeDraft = {},
            onStateChanged = {},
            onUnavailable = unavailable::add,
            scheduler = scheduler,
        )

        session.start()
        assertTrue(session.submit())
        scheduler.runPending()

        assertFalse(session.state().active)
        assertEquals(listOf("没有识别到语音"), unavailable)
        assertEquals(1, engine.stopCount)
    }

    @Test
    fun submitUsesTheActiveEngineResultTimeoutForServerFallback() {
        val engine = FakeEngine().apply { resultTimeoutMs = 65_000L }
        val scheduler = FakeScheduler()
        val session = WebChatNativeDictationSession(
            bridge = { engine },
            readDraft = { "" },
            writeDraft = {},
            onStateChanged = {},
            onUnavailable = {},
            scheduler = scheduler,
        )

        session.start()
        session.submit()

        assertEquals(65_000L, scheduler.lastDelayMs)
    }

    @Test
    fun speechEndWaitsForTheFinalResultWithoutHidingThePartialDraft() {
        val engine = FakeEngine()
        val scheduler = FakeScheduler()
        var draft = ""
        val session = WebChatNativeDictationSession(
            bridge = { engine },
            readDraft = { draft },
            writeDraft = { draft = it },
            onStateChanged = {},
            onUnavailable = {},
            scheduler = scheduler,
        )

        assertTrue(session.start())
        engine.onPartial("部分")
        engine.onEnd()

        assertEquals("部分", draft)
        assertEquals(WebChatNativeDictationPhase.PROCESSING, session.state().phase)

        engine.deliverFinal("完整结果")

        assertEquals("完整结果", draft)
        assertEquals(WebChatNativeDictationPhase.IDLE, session.state().phase)
        assertEquals(0, engine.cancelCount)
    }

    @Test
    fun settlementCancelsAnOrphanedEngineAndAllowsTheNextStart() {
        val engine = FakeEngine()
        val scheduler = FakeScheduler()
        val session = WebChatNativeDictationSession(
            bridge = { engine },
            readDraft = { "" },
            writeDraft = {},
            onStateChanged = {},
            onUnavailable = {},
            scheduler = scheduler,
        )

        assertTrue(session.start())
        engine.onEnd()
        scheduler.runPending()

        assertEquals(WebChatNativeDictationPhase.IDLE, session.state().phase)
        assertEquals(1, engine.cancelCount)
        assertTrue(session.start())
        assertEquals(2, engine.startCount)
    }

    @Test
    fun destroyReleasesTheCapturedEngineEvenAfterItBecameIdle() {
        val engine = FakeEngine()
        val session = session(engine, { "" }, {})

        session.start()
        engine.deliverFinal("完成")
        session.destroy()

        assertEquals(1, engine.destroyCount)
    }

    private fun session(
        engine: FakeEngine,
        readDraft: () -> String,
        writeDraft: (String) -> Unit,
    ) = WebChatNativeDictationSession(
        bridge = { engine },
        readDraft = readDraft,
        writeDraft = writeDraft,
        onStateChanged = {},
        onUnavailable = {},
        scheduler = FakeScheduler(),
    )

    private class FakeScheduler : WebChatNativeDictationScheduler {
        private val tasks = linkedSetOf<Runnable>()
        var lastDelayMs: Long? = null
        override fun postDelayed(task: Runnable, delayMs: Long) {
            tasks += task
            lastDelayMs = delayMs
        }
        override fun remove(task: Runnable) {
            tasks -= task
        }
        fun runPending() {
            tasks.toList().also(tasks::removeAll).forEach(Runnable::run)
        }
    }

    private class FakeEngine : WebChatNativeDictationEngine {
        override var onReady: () -> Unit = {}
        override var onStart: () -> Unit = {}
        override var onPartial: (String) -> Unit = {}
        override var onFinal: (String) -> Unit = {}
        override var onEnd: () -> Unit = {}
        override var onError: (String) -> Unit = {}
        override var onVolume: (Float) -> Unit = {}
        override var isRunning: Boolean = false
        override var resultTimeoutMs: Long = WebChatNativeDictationEngine.DEFAULT_RESULT_TIMEOUT_MS
        var stopCount = 0
        var cancelCount = 0
        var prewarmCount = 0
        var startCount = 0
        var destroyCount = 0

        override fun start() {
            isRunning = true
            startCount += 1
        }
        override fun stop() {
            isRunning = false
            stopCount += 1
        }
        override fun cancel() {
            isRunning = false
            cancelCount += 1
        }
        override fun prewarm() {
            prewarmCount += 1
        }
        override fun destroy() {
            isRunning = false
            destroyCount += 1
        }

        fun deliverFinal(value: String) {
            isRunning = false
            onFinal(value)
        }
    }
}
