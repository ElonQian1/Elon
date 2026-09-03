package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptConversationOpenRecoveryCoordinatorTest {
    private var currentUrl = "https://chatgpt.com/c/origin"
    private var pending = true
    private val scheduled = mutableListOf<Runnable>()
    private val recoveries = mutableListOf<Boolean>()
    private val loaded = mutableListOf<String>()
    private val coordinator = ChatGptConversationOpenRecoveryCoordinator(
        currentUrl = { currentUrl },
        navigationPending = { pending },
        schedule = { task, _ -> scheduled += task },
        cancelTask = { task -> scheduled.remove(task) },
        onRecovery = recoveries::add,
        loadUrl = loaded::add,
    )

    @Test
    fun loadsTheValidatedTargetWhenThePageCommandDidNotNavigate() {
        coordinator.schedule("/g/g-p-project/c/target")

        scheduled.removeAt(0).run()

        assertEquals(listOf("https://chatgpt.com/g/g-p-project/c/target"), loaded)
        assertEquals(listOf(false), recoveries)
    }

    @Test
    fun reloadsATargetPathThatStillHasNotProducedAReadySnapshot() {
        currentUrl = "https://chatgpt.com/c/target"
        coordinator.schedule("/c/target")
        scheduled.removeAt(0).run()

        assertEquals(listOf("https://chatgpt.com/c/target"), loaded)
        assertEquals(listOf(true), recoveries)
    }

    @Test
    fun leavesACompletedNavigationAlone() {
        pending = false

        coordinator.schedule("/c/other")
        scheduled.removeAt(0).run()

        assertTrue(loaded.isEmpty())
        assertTrue(recoveries.isEmpty())
    }

    @Test
    fun aNewRequestCancelsTheOlderRecovery() {
        coordinator.schedule("/c/first")
        coordinator.schedule("/c/second")

        assertEquals(1, scheduled.size)
        scheduled.removeAt(0).run()

        assertEquals(listOf("https://chatgpt.com/c/second"), loaded)
    }
}
