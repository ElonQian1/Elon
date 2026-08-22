package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatSessionPrewarmCoordinatorTest {
    @Test
    fun cachedSelectedProviderIsPrewarmedAfterTheFirstFrame() {
        val fixture = Fixture()
        fixture.warm += WebChatProviderId.CHATGPT_WEB

        fixture.coordinator.onHostResumed()

        assertTrue(fixture.scheduler.hasTasks())
        fixture.scheduler.runNext()
        assertEquals(listOf(WebChatProviderId.CHATGPT_WEB), fixture.prewarmed)
    }

    @Test
    fun providerWithoutCacheDoesNotCreateAWebSession() {
        val fixture = Fixture()

        fixture.coordinator.onHostResumed()

        assertFalse(fixture.scheduler.hasTasks())
        assertTrue(fixture.prewarmed.isEmpty())
    }

    @Test
    fun pauseCancelsPendingPrewarm() {
        val fixture = Fixture()
        fixture.warm += WebChatProviderId.CHATGPT_WEB
        fixture.coordinator.onHostResumed()

        fixture.coordinator.cancel()

        assertFalse(fixture.scheduler.hasTasks())
        assertTrue(fixture.prewarmed.isEmpty())
    }

    @Test
    fun providerChangeUsesTheCurrentCachedProvider() {
        val fixture = Fixture()
        fixture.warm += WebChatProviderId.CHATGPT_WEB
        fixture.warm += WebChatProviderId.GOOGLE_WEB
        fixture.coordinator.onHostResumed()
        fixture.provider = WebChatProviderId.GOOGLE_WEB

        fixture.scheduler.runNext()

        assertEquals(listOf(WebChatProviderId.GOOGLE_WEB), fixture.prewarmed)
    }

    @Test
    fun activeProviderAndWorkModeDoNotPrewarm() {
        val fixture = Fixture()
        fixture.warm += WebChatProviderId.CHATGPT_WEB
        fixture.active += WebChatProviderId.CHATGPT_WEB
        fixture.coordinator.onHostResumed()
        assertFalse(fixture.scheduler.hasTasks())

        fixture.active.clear()
        fixture.mode = SocialAiInteractionMode.WORK
        fixture.coordinator.onHostResumed()
        assertFalse(fixture.scheduler.hasTasks())
    }

    private class Fixture {
        val scheduler = Scheduler()
        val warm = mutableSetOf<WebChatProviderId>()
        val active = mutableSetOf<WebChatProviderId>()
        val prewarmed = mutableListOf<WebChatProviderId>()
        var mode = SocialAiInteractionMode.CHAT
        var provider = WebChatProviderId.CHATGPT_WEB
        val coordinator = WebChatSessionPrewarmCoordinator(
            schedule = scheduler::schedule,
            cancel = scheduler::cancel,
            interactionMode = { mode },
            selectedProvider = { provider },
            hasWarmSession = warm::contains,
            isProviderActive = active::contains,
            prewarm = { prewarmed += it; true },
            delayMs = 1L,
        )
    }

    private class Scheduler {
        private val tasks = mutableListOf<Runnable>()

        fun schedule(task: Runnable, @Suppress("UNUSED_PARAMETER") delayMs: Long) {
            tasks += task
        }

        fun cancel(task: Runnable) {
            tasks.remove(task)
        }

        fun runNext() {
            tasks.removeFirstOrNull()?.run()
        }

        fun hasTasks(): Boolean = tasks.isNotEmpty()
    }
}
