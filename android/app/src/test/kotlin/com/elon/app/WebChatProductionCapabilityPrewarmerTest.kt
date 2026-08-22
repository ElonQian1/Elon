package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionCapabilityPrewarmerTest {
    @Test
    fun prewarmsEveryDeclaredCapabilityOnceAndThenUsesTheWarmSnapshot() {
        val scheduler = Scheduler()
        val port = FakePort()
        var active = WebChatProviderId.CHATGPT_WEB
        val prewarmer = prewarmer(port, scheduler) { active }

        prewarmer.schedule(WebChatProviderRegistry.get(active))
        assertTrue(port.requests.isEmpty())
        scheduler.runNext()
        assertEquals(listOf("model"), port.requests)
        scheduler.drain()

        assertEquals(listOf("model", "tools", "features", "controls"), port.requests)
        prewarmer.schedule(WebChatProviderRegistry.get(active))
        scheduler.drain()
        assertEquals(4, port.requests.size)
    }

    @Test
    fun providerSwitchCancelsQueuedWarmupWithoutTouchingTheNewProvider() {
        val scheduler = Scheduler()
        val port = FakePort()
        var active = WebChatProviderId.CHATGPT_WEB
        val prewarmer = prewarmer(port, scheduler) { active }

        prewarmer.schedule(WebChatProviderRegistry.get(active))
        active = WebChatProviderId.GOOGLE_WEB
        scheduler.drain()

        assertTrue(port.requests.isEmpty())
    }

    @Test
    fun unavailableBridgeGetsOneBoundedRetryInsteadOfContinuousPolling() {
        val scheduler = Scheduler()
        val port = FakePort(acceptRequests = false)
        val prewarmer = prewarmer(port, scheduler) { WebChatProviderId.CHATGPT_WEB }

        prewarmer.schedule(WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB))
        scheduler.drain()

        assertEquals(8, port.requests.size)
        assertEquals(2, port.requests.count { it == "model" })
        assertEquals(2, port.requests.count { it == "tools" })
        assertEquals(2, port.requests.count { it == "features" })
        assertEquals(2, port.requests.count { it == "controls" })
    }

    @Test
    fun confirmedCacheSnapshotsSkipRedundantOfficialMenuReads() {
        val scheduler = Scheduler()
        val port = FakePort()
        val cache = WebChatProductionInteractionCache().apply {
            replaceComposerOptions(
                WebChatProviderId.CHATGPT_WEB,
                "model",
                listOf(option("auto")),
            )
            composerOptions(WebChatProviderId.CHATGPT_WEB, "tools", listOf(option("search")))
            features(WebChatProviderId.CHATGPT_WEB, listOf(feature("projects")))
            controls(
                WebChatProviderId.CHATGPT_WEB,
                emptyState().copy(controls = listOf(control("more"))),
            )
        }
        val prewarmer = prewarmer(port, scheduler, cache = cache) {
            WebChatProviderId.CHATGPT_WEB
        }

        prewarmer.schedule(WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB))
        scheduler.drain()

        assertTrue(port.requests.isEmpty())
    }

    @Test
    fun switchingConversationPrewarmsItsControlsDuringProviderCooldown() {
        val scheduler = Scheduler()
        val port = FakePort()
        val prewarmer = prewarmer(port, scheduler) { WebChatProviderId.CHATGPT_WEB }
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)

        prewarmer.schedule(provider)
        scheduler.drain()
        assertEquals(listOf("model", "tools", "features", "controls"), port.requests)

        port.openConversation("second")
        prewarmer.schedule(provider)
        scheduler.drain()

        assertEquals(
            listOf("model", "tools", "features", "controls", "controls"),
            port.requests,
        )
    }

    private fun prewarmer(
        port: FakePort,
        scheduler: Scheduler,
        cache: WebChatProductionInteractionCache = WebChatProductionInteractionCache(),
        activeProvider: () -> WebChatProviderId,
    ) = WebChatProductionCapabilityPrewarmer(
        consumerPort = { port },
        activeProvider = activeProvider,
        interactionCache = cache,
        scheduleAction = scheduler::schedule,
        nowMs = scheduler::now,
    )

    private class Scheduler {
        private var nowMs = 0L
        private val tasks = mutableListOf<Scheduled>()

        fun now(): Long = nowMs

        fun schedule(delayMs: Long, action: () -> Unit) {
            tasks += Scheduled(nowMs + delayMs, action)
        }

        fun drain() {
            while (tasks.isNotEmpty()) {
                runNext()
            }
        }

        fun runNext() {
            val next = tasks.minByOrNull(Scheduled::atMs) ?: return
            tasks.remove(next)
            nowMs = next.atMs
            next.action()
        }

        private data class Scheduled(val atMs: Long, val action: () -> Unit)
    }

    private class FakePort(
        private val acceptRequests: Boolean = true,
    ) : WebChatConsumerPort {
        val requests = mutableListOf<String>()
        private var current = emptyState()

        override fun state(): WebChatConsumerState = current

        override fun requestComposerOptions(section: String): WebChatConsumerCommandResult =
            respond(section) {
                current = current.copy(
                    composerSections = current.composerSections + (section to listOf(option(section))),
                )
            }

        override fun requestFeatures(): WebChatConsumerCommandResult = respond("features") {
            current = current.copy(features = listOf(feature("feature")))
        }

        override fun requestControls(): WebChatConsumerCommandResult = respond("controls") {
            current = current.copy(controls = listOf(control("control")))
        }

        override fun dismissComposerOptions() = accepted()
        override fun selectComposerOption(section: String, optionId: String) = accepted()
        override fun selectFeature(featureId: String, userConfirmed: Boolean) = accepted()
        override fun invokeControl(controlId: String, userConfirmed: Boolean) = accepted()
        override fun updateControl(controlId: String, mutation: WebChatConsumerControlMutation) = accepted()
        override fun executeSessionCommand(action: String) = accepted()

        fun openConversation(id: String) {
            current = current.copy(
                pageUrl = "https://chatgpt.com/c/$id",
                controls = emptyList(),
            )
        }

        private fun respond(label: String, update: () -> Unit): WebChatConsumerCommandResult {
            requests += label
            if (acceptRequests) update()
            return WebChatConsumerCommandResult(accepted = acceptRequests)
        }

        private fun accepted() = WebChatConsumerCommandResult(accepted = true)
    }

    private companion object {
        fun emptyState() = WebChatConsumerState(
            streaming = false,
            dictationActive = false,
            composerSections = emptyMap(),
            pageKind = "conversation",
            pageUrl = "https://example.invalid/",
            features = emptyList(),
            controls = emptyList(),
            commandRequests = emptyList(),
        )

        fun option(id: String) = WebChatConsumerOption(
            id = id,
            label = id,
            selected = false,
            semantic = id,
            opensSubmenu = false,
            nativeSelector = "option:$id",
        )

        fun feature(id: String) = WebChatConsumerFeature(
            id = id,
            label = id,
            kind = id,
            selected = false,
            requiresUserConfirmation = false,
            nativeSelector = "feature:$id",
        )

        fun control(id: String) = WebChatConsumerControlDescriptor(
            control = ChatGptWebUiControl(
                id = id,
                label = id,
                semantic = id,
                region = "header",
                role = "button",
                enabled = true,
                selected = false,
            ),
            requiresUserConfirmation = false,
            presentation = WebChatConsumerControlPresentation.DIRECT,
            nativeSelector = "control:$id",
        )
    }
}
