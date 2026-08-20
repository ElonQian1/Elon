package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceCoordinatorTest {
    @Test
    fun startsOfficialVoiceBehindTheNativeSurfaceAndWaitsForReceipt() {
        val fixture = Fixture()

        assertTrue(fixture.coordinator.start(fixture.provider))
        assertTrue(fixture.surface.visible)
        assertEquals(WebChatRealtimeVoiceStage.PREPARING, fixture.surface.stage)
        assertEquals(1, fixture.beginBackingCount)

        fixture.scheduler.runNext()
        assertEquals(1, fixture.port.executeCount)
        assertEquals(WebChatRealtimeVoiceStage.STARTING, fixture.surface.stage)

        fixture.port.commandStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.scheduler.runNext()
        assertEquals(WebChatRealtimeVoiceStage.ACTIVE, fixture.surface.stage)
        assertEquals(0, fixture.officialFallbackCount)
        assertTrue(fixture.back.backEnabled)
    }

    @Test
    fun failureStaysNativeUntilTheUserExplicitlyChoosesOfficialFallback() {
        val fixture = Fixture(sessionReady = false)

        assertTrue(fixture.coordinator.start(fixture.provider))
        fixture.scheduler.runAll()

        assertEquals(WebChatRealtimeVoiceStage.FAILED, fixture.surface.stage)
        assertTrue(fixture.surface.visible)
        assertEquals(0, fixture.officialFallbackCount)
        assertTrue(fixture.endBackingCount >= 1)

        fixture.surface.openOfficialFallback()
        assertEquals(1, fixture.officialFallbackCount)
        assertFalse(fixture.surface.visible)
    }

    @Test
    fun unsupportedProviderDoesNotOpenAnyVoiceSurface() {
        val fixture = Fixture()

        assertFalse(
            fixture.coordinator.start(WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)),
        )
        assertFalse(fixture.surface.visible)
        assertEquals(0, fixture.beginBackingCount)
    }

    private class Fixture(sessionReady: Boolean = true) {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val surface = FakeSurface()
        val scheduler = Scheduler()
        val port = FakePort()
        val back = FakeBackControl()
        var beginBackingCount = 0
        var endBackingCount = 0
        var officialFallbackCount = 0
        val coordinator = WebChatRealtimeVoiceCoordinator(
            surface = surface,
            activeProvider = { WebChatProviderId.CHATGPT_WEB },
            consumerPort = { port },
            sessionReady = { sessionReady },
            beginWebBacking = { beginBackingCount += 1; true },
            endWebBacking = { endBackingCount += 1 },
            requestSessionRecovery = {},
            openOfficialFallback = { officialFallbackCount += 1 },
            schedule = scheduler::schedule,
            backControl = back,
        )
    }

    private class FakeSurface : WebChatRealtimeVoiceSurface {
        var visible = false
        var stage: WebChatRealtimeVoiceStage? = null
        private var fallback: () -> Unit = {}

        override fun show(
            onClose: () -> Unit,
            onRetry: () -> Unit,
            onOfficialFallback: () -> Unit,
        ) {
            visible = true
            fallback = onOfficialFallback
        }

        override fun render(stage: WebChatRealtimeVoiceStage, detail: String) {
            this.stage = stage
        }

        override fun hide() {
            visible = false
        }

        override fun isVisible(): Boolean = visible

        fun openOfficialFallback() = fallback()
    }

    private class FakePort : WebChatConsumerPort {
        var executeCount = 0
        var commandStatus = WebChatConsumerCommandStatus.PENDING

        override fun state() = WebChatConsumerState(
            streaming = false,
            dictationActive = false,
            composerSections = emptyMap(),
            pageKind = "conversation",
            pageUrl = "https://chatgpt.com/c/test",
            features = emptyList(),
            controls = listOf(WebChatConsumerControlDescriptor(
                control = VoiceControl,
                requiresUserConfirmation = false,
                presentation = WebChatConsumerControlPresentation.DIRECT,
                nativeSelector = WebChatProductionSelectors.REALTIME_VOICE_SURFACE,
            )),
            commandRequests = listOf(WebChatConsumerCommandRequest("voice_1", commandStatus)),
        )

        override fun requestComposerOptions(section: String) = rejected()
        override fun dismissComposerOptions() = rejected()
        override fun selectComposerOption(section: String, optionId: String) = rejected()
        override fun requestFeatures() = rejected()
        override fun selectFeature(featureId: String, userConfirmed: Boolean) = rejected()
        override fun requestControls() = accepted()
        override fun invokeControl(controlId: String, userConfirmed: Boolean) = rejected()
        override fun updateControl(controlId: String, mutation: WebChatConsumerControlMutation) = rejected()

        override fun executeSessionCommand(action: String): WebChatConsumerCommandResult {
            executeCount += 1
            return accepted("voice_1")
        }

        private fun accepted(requestId: String? = null) =
            WebChatConsumerCommandResult(true, requestId = requestId)

        private fun rejected() = WebChatConsumerCommandResult(false)
    }

    private object VoiceControl : WebChatConsumerControl {
        override val id = "control_voice"
        override val semantic = "voice_mode"
        override val label = "实时语音"
        override val region = "composer"
        override val role = "button"
        override val enabled = true
        override val selected = false
        override val inputKind: String? = null
        override val writable = false
        override val stateSettable = false
        override val choiceLabels = emptyList<String>()
        override val selectedChoiceIndex: Int? = null
        override val slider: WebChatConsumerSlider? = null
        override val expanded: Boolean? = null
        override val expandable = false
        override val contextId: String? = null
        override val inViewport = true
        override val webXRatio: Double? = 0.5
        override val webYRatio: Double? = 0.5
    }

    private class Scheduler {
        private val tasks = ArrayDeque<Runnable>()

        fun schedule(task: Runnable, @Suppress("UNUSED_PARAMETER") delayMs: Long) {
            tasks.addLast(task)
        }

        fun runNext() {
            tasks.removeFirstOrNull()?.run()
        }

        fun runAll() {
            repeat(100) {
                if (tasks.isEmpty()) return
                runNext()
            }
            error("Realtime voice scheduler did not settle")
        }
    }

    private class FakeBackControl : WebChatRealtimeVoiceBackControl {
        var backEnabled = false
        override fun setEnabled(enabled: Boolean) {
            backEnabled = enabled
        }
        override fun dispose() = Unit
    }
}
