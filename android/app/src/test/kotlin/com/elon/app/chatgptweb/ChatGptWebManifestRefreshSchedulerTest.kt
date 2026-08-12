package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Test

class ChatGptWebManifestRefreshSchedulerTest {
    @Test
    fun refreshesAcrossTheOfficialOverlayRenderWindow() {
        val scheduled = mutableListOf<Pair<Long, () -> Unit>>()
        var refreshes = 0
        val scheduler = ChatGptWebManifestRefreshScheduler(
            schedule = { delay, action -> scheduled += delay to action },
            refresh = { refreshes += 1 },
        )

        scheduler.afterAdaptiveTouch()
        scheduled.forEach { it.second() }

        assertEquals(ChatGptWebManifestRefreshScheduler.SETTLE_DELAYS_MS, scheduled.map { it.first })
        assertEquals(4, refreshes)
    }

    @Test
    fun newerTouchAndDisposeInvalidateOlderRefreshes() {
        val scheduled = mutableListOf<() -> Unit>()
        var refreshes = 0
        val scheduler = ChatGptWebManifestRefreshScheduler(
            schedule = { _, action -> scheduled += action },
            refresh = { refreshes += 1 },
        )

        scheduler.afterAdaptiveTouch()
        scheduler.afterAdaptiveTouch()
        scheduled.take(4).forEach { it() }
        scheduled.drop(4).forEach { it() }
        assertEquals(4, refreshes)

        scheduler.afterAdaptiveTouch()
        scheduler.dispose()
        scheduled.takeLast(4).forEach { it() }
        assertEquals(4, refreshes)
    }
}
