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
        assertEquals(WebChatRealtimeVoiceStage.PREPARING, fixture.surface.stage)

        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.scheduler.runNext()
        fixture.scheduler.runNext()
        assertEquals(2, fixture.port.executeCount)
        assertEquals(WebChatRealtimeVoiceStage.STARTING, fixture.surface.stage)

        fixture.port.voiceStatus = WebChatConsumerCommandStatus.SUCCEEDED
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

    @Test
    fun guestVoiceShowsOfficialLoginMethodsWithoutBlockingAnonymousTextChat() {
        val fixture = Fixture(authenticated = false)

        assertTrue(fixture.coordinator.start(fixture.provider))

        assertTrue(fixture.loginGate.visible)
        assertFalse(fixture.surface.visible)
        assertEquals(0, fixture.beginBackingCount)
        assertEquals(0, fixture.officialLoginCount)
    }

    @Test
    fun loginReturnRefreshesTheSharedSessionAndRetriesVoiceOnceAuthenticated() {
        val fixture = Fixture(authenticated = false)
        fixture.coordinator.start(fixture.provider)

        fixture.loginGate.openOfficialLogin()
        assertEquals(1, fixture.officialLoginCount)
        assertFalse(fixture.loginGate.visible)

        fixture.authenticated = true
        fixture.coordinator.onHostResumed()
        fixture.scheduler.runNext()

        assertTrue(fixture.surface.visible)
        assertEquals(WebChatRealtimeVoiceStage.PREPARING, fixture.surface.stage)
        assertEquals(1, fixture.beginBackingCount)
    }

    @Test
    fun unknownAuthenticationWaitsForTheSessionInsteadOfClaimingLoginIsRequired() {
        val fixture = Fixture(authenticated = false, sessionState = "loading", sessionReady = false)

        assertTrue(fixture.coordinator.start(fixture.provider))

        assertTrue(fixture.surface.visible)
        assertFalse(fixture.loginGate.visible)
        assertEquals(1, fixture.beginBackingCount)

        fixture.sessionState = "login_required"
        fixture.scheduler.runNext()

        assertFalse(fixture.surface.visible)
        assertTrue(fixture.loginGate.visible)
        assertEquals(1, fixture.endBackingCount)
    }

    @Test
    fun closesOfficialVoiceBeforeRevealingThePreservedNativeConversation() {
        val fixture = Fixture()
        fixture.completeVoiceStart()
        fixture.port.endControlAvailable = true

        fixture.surface.closeVoice()

        assertTrue(fixture.surface.visible)
        assertEquals("control_voice_end", fixture.port.invokedControlId)
        fixture.scheduler.runNext()
        assertTrue(fixture.surface.visible)

        fixture.port.endControlAvailable = false
        fixture.scheduler.runNext()
        assertFalse(fixture.surface.visible)
        assertEquals(listOf(true), fixture.endBackingGraceful)
    }

    @Test
    fun startsAgainAfterTheOfficialVoicePageHasFinishedClosing() {
        val fixture = Fixture()
        fixture.completeVoiceStart()
        fixture.port.endControlAvailable = true

        fixture.surface.closeVoice()
        fixture.scheduler.runNext()
        fixture.port.endControlAvailable = false
        fixture.scheduler.runNext()

        fixture.completeVoiceStart()

        assertTrue(fixture.surface.visible)
        assertEquals(WebChatRealtimeVoiceStage.ACTIVE, fixture.surface.stage)
        assertEquals(2, fixture.beginBackingCount)
        assertEquals(listOf(true), fixture.endBackingGraceful)
    }

    @Test
    fun fallsBackOnceWhenTheOfficialVoicePageNeverFinishesClosing() {
        val fixture = Fixture()
        fixture.completeVoiceStart()
        fixture.port.endControlAvailable = true

        fixture.surface.closeVoice()
        fixture.scheduler.runAll()

        assertFalse(fixture.surface.visible)
        assertEquals(listOf(false), fixture.endBackingGraceful)
        assertFalse(fixture.scheduler.hasPendingTasks())
    }

    private class Fixture(
        sessionReady: Boolean = true,
        authenticated: Boolean = true,
        sessionState: String = "ready",
    ) {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val surface = FakeSurface()
        val loginGate = FakeLoginGate()
        val scheduler = Scheduler()
        val port = FakePort()
        val back = FakeBackControl()
        var authenticated = authenticated
        var sessionState = sessionState
        var beginBackingCount = 0
        val endBackingGraceful = mutableListOf<Boolean>()
        val endBackingCount: Int get() = endBackingGraceful.size
        var officialFallbackCount = 0
        var officialLoginCount = 0
        val coordinator = WebChatRealtimeVoiceCoordinator(
            surface = surface,
            activeProvider = { WebChatProviderId.CHATGPT_WEB },
            consumerPort = { port },
            sessionReady = { sessionReady },
            authenticationState = {
                WebChatRealtimeVoiceAuthenticationPolicy.resolve(
                    this.authenticated,
                    this.sessionState,
                )
            },
            beginWebBacking = { beginBackingCount += 1; true },
            endWebBacking = { graceful -> endBackingGraceful += graceful },
            requestSessionRecovery = {},
            loginGate = loginGate,
            openOfficialLogin = { officialLoginCount += 1 },
            openOfficialFallback = { officialFallbackCount += 1 },
            schedule = scheduler::schedule,
            backControl = back,
        )

        fun completeVoiceStart() {
            coordinator.start(provider)
            scheduler.runNext()
            port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
            scheduler.runNext()
            scheduler.runNext()
            port.voiceStatus = WebChatConsumerCommandStatus.SUCCEEDED
            scheduler.runNext()
        }
    }

    private class FakeLoginGate : WebChatRealtimeVoiceLoginGate {
        var visible = false
        private var officialLogin: () -> Unit = {}

        override fun show(onOfficialLogin: () -> Unit, onCancel: () -> Unit) {
            visible = true
            officialLogin = onOfficialLogin
        }

        override fun dismiss() {
            visible = false
        }

        override fun isVisible(): Boolean = visible

        fun openOfficialLogin() {
            officialLogin()
        }
    }

    private class FakeSurface : WebChatRealtimeVoiceSurface {
        var visible = false
        var stage: WebChatRealtimeVoiceStage? = null
        private var fallback: () -> Unit = {}
        private var close: () -> Unit = {}

        override fun show(
            onClose: () -> Unit,
            onRetry: () -> Unit,
            onOfficialFallback: () -> Unit,
        ) {
            visible = true
            close = onClose
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
        fun closeVoice() = close()
    }

    private class FakePort : WebChatConsumerPort {
        var executeCount = 0
        var prepareStatus = WebChatConsumerCommandStatus.PENDING
        var voiceStatus = WebChatConsumerCommandStatus.PENDING
        var endControlAvailable = false
        var invokedControlId: String? = null

        override fun state() = WebChatConsumerState(
            streaming = false,
            dictationActive = false,
            composerSections = emptyMap(),
            pageKind = "conversation",
            pageUrl = "https://chatgpt.com/c/test",
            features = emptyList(),
            controls = if (endControlAvailable) {
                listOf(WebChatConsumerControlDescriptor(
                    control = VoiceEndControl,
                    requiresUserConfirmation = true,
                    presentation = WebChatConsumerControlPresentation.DIRECT,
                    nativeSelector = WebChatProductionSelectors.REALTIME_VOICE_CLOSE,
                ))
            } else if (prepareStatus == WebChatConsumerCommandStatus.SUCCEEDED) {
                listOf(WebChatConsumerControlDescriptor(
                    control = VoiceControl,
                    requiresUserConfirmation = false,
                    presentation = WebChatConsumerControlPresentation.DIRECT,
                    nativeSelector = WebChatProductionSelectors.REALTIME_VOICE_SURFACE,
                ))
            } else {
                emptyList()
            },
            commandRequests = listOf(
                WebChatConsumerCommandRequest("prepare_1", prepareStatus),
                WebChatConsumerCommandRequest("voice_1", voiceStatus),
            ),
        )

        override fun requestComposerOptions(section: String) = rejected()
        override fun dismissComposerOptions() = rejected()
        override fun selectComposerOption(section: String, optionId: String) = rejected()
        override fun requestFeatures() = rejected()
        override fun selectFeature(featureId: String, userConfirmed: Boolean) = rejected()
        override fun requestControls() = accepted()
        override fun invokeControl(controlId: String, userConfirmed: Boolean): WebChatConsumerCommandResult {
            invokedControlId = controlId
            return if (controlId == VoiceEndControl.id && userConfirmed) accepted() else rejected()
        }
        override fun updateControl(controlId: String, mutation: WebChatConsumerControlMutation) = rejected()

        override fun executeSessionCommand(action: String): WebChatConsumerCommandResult {
            executeCount += 1
            return when (action) {
                "chatgpt_prepare_realtime_voice" -> accepted("prepare_1")
                "chatgpt_start_realtime_voice" -> accepted("voice_1")
                else -> rejected()
            }
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

    private object VoiceEndControl : WebChatConsumerControl by VoiceControl {
        override val id = "control_voice_end"
        override val semantic = "close"
        override val label = "结束语音"
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

        fun hasPendingTasks(): Boolean = tasks.isNotEmpty()
    }

    private class FakeBackControl : WebChatRealtimeVoiceBackControl {
        var backEnabled = false
        override fun setEnabled(enabled: Boolean) {
            backEnabled = enabled
        }
        override fun dispose() = Unit
    }
}
