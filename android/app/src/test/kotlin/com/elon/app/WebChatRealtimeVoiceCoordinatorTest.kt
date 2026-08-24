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
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)
        assertEquals(1, fixture.beginBackingCount)

        fixture.scheduler.runNext()
        assertEquals(1, fixture.port.executeCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)

        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.scheduler.runNext()
        fixture.scheduler.runNext()
        assertEquals(2, fixture.port.executeCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)

        fixture.port.voiceStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.scheduler.runNext()
        assertEquals(WebChatRealtimeVoiceLifecycle.ACTIVE, fixture.surface.state?.lifecycle)
        assertEquals(WebChatRealtimeVoiceTurn.IDLE, fixture.surface.state?.turn)
        assertEquals(0, fixture.officialFallbackCount)
        assertFalse(fixture.back.backEnabled)
    }

    @Test
    fun readyConversationSkipsFullSessionRecovery() {
        val fixture = Fixture()
        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED

        fixture.coordinator.start(fixture.provider)

        assertEquals(0, fixture.sessionRecoveryCount)
        fixture.scheduler.runNext()
        assertEquals(1, fixture.port.executeCount)
    }

    @Test
    fun directReadyVoiceStartsWithoutRefreshingControlsAgain() {
        val fixture = Fixture()
        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.port.prepareReturnsReceipt = false

        fixture.coordinator.start(fixture.provider)
        fixture.scheduler.runNext()

        assertEquals(2, fixture.port.executeCount)
        assertEquals(0, fixture.port.requestControlsCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)
    }

    @Test
    fun coldConversationStillRequestsSessionRecovery() {
        val fixture = Fixture(sessionReady = false)

        fixture.coordinator.start(fixture.provider)

        assertTrue(fixture.sessionRecoveryCount >= 1)
    }

    @Test
    fun cachedConversationRefreshesOnlyControlsBeforeStarting() {
        val fixture = Fixture()
        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.launchCache.observe(WebChatProviderId.CHATGPT_WEB, fixture.port.state())
        fixture.port.prepareStatus = WebChatConsumerCommandStatus.PENDING

        fixture.coordinator.start(fixture.provider)

        assertEquals(0, fixture.sessionRecoveryCount)
        assertEquals(1, fixture.port.requestControlsCount)
    }

    @Test
    fun failureStaysNativeUntilTheUserExplicitlyChoosesOfficialFallback() {
        val fixture = Fixture(sessionReady = false)

        assertTrue(fixture.coordinator.start(fixture.provider))
        fixture.scheduler.runAll()

        assertEquals(WebChatRealtimeVoiceLifecycle.FAILED, fixture.surface.state?.lifecycle)
        assertTrue(fixture.surface.visible)
        assertEquals(0, fixture.officialFallbackCount)
        assertTrue(fixture.endBackingCount >= 1)

        fixture.surface.openOfficialFallback()
        assertEquals(1, fixture.officialFallbackCount)
        assertFalse(fixture.surface.visible)
    }

    @Test
    fun tappingVoiceAgainAfterFailureRetriesTheReadySessionWithoutRecovery() {
        val fixture = Fixture(sessionReady = false)
        fixture.coordinator.start(fixture.provider)
        fixture.scheduler.runAll()
        val recoveryCountAfterFailure = fixture.sessionRecoveryCount

        fixture.sessionReady = true
        fixture.port.prepareStatus = WebChatConsumerCommandStatus.SUCCEEDED
        fixture.port.prepareReturnsReceipt = false
        fixture.coordinator.start(fixture.provider)

        assertEquals(2, fixture.beginBackingCount)
        assertEquals(recoveryCountAfterFailure, fixture.sessionRecoveryCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)
        fixture.scheduler.runNext()
        assertEquals(2, fixture.port.executeCount)
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
        assertEquals(WebChatRealtimeVoiceLifecycle.CONNECTING, fixture.surface.state?.lifecycle)
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
        assertEquals(WebChatRealtimeVoiceLifecycle.ENDING, fixture.surface.state?.lifecycle)
        fixture.scheduler.runNext()
        assertTrue(fixture.surface.visible)

        fixture.port.endControlAvailable = false
        fixture.scheduler.runNext()
        assertFalse(fixture.surface.visible)
        assertEquals(listOf(true), fixture.endBackingGraceful)
    }

    @Test
    fun endingVoiceIgnoresRepeatedHangupRequests() {
        val fixture = Fixture()
        fixture.completeVoiceStart()
        fixture.port.endControlAvailable = true

        fixture.surface.closeVoice()
        fixture.surface.closeVoice()

        assertEquals(1, fixture.port.invokedControlCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.ENDING, fixture.surface.state?.lifecycle)
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
        assertEquals(WebChatRealtimeVoiceLifecycle.ACTIVE, fixture.surface.state?.lifecycle)
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

    @Test
    fun hostResumeRestoresTheFloatingControlAboveTheCurrentActivitySurface() {
        val fixture = Fixture()
        fixture.completeVoiceStart()
        fixture.surface.visible = false

        fixture.coordinator.onHostResumed()

        assertTrue(fixture.surface.visible)
        assertEquals(1, fixture.surface.ensureVisibleCount)
        assertEquals(WebChatRealtimeVoiceLifecycle.ACTIVE, fixture.surface.state?.lifecycle)
    }

    @Test
    fun expandedVoiceControlOpensTheConversationCapturedAtVoiceStart() {
        val fixture = Fixture()
        fixture.completeVoiceStart()

        fixture.surface.openConversation()

        assertEquals("/c/test", fixture.openedContext?.conversationPath)
        assertEquals("测试会话", fixture.openedContext?.label)
    }

    @Test
    fun newVoiceConversationUpdatesItsAttributionWhenTheOfficialPathAppears() {
        val fixture = Fixture()
        fixture.context = WebChatRealtimeVoiceContext(
            conversationPath = null,
            label = "新会话（发送后自动归档）",
            savedToHistory = true,
        )
        fixture.completeVoiceStart()

        fixture.context = WebChatRealtimeVoiceContext(
            conversationPath = "/c/generated",
            label = "语音生成的会话",
            savedToHistory = true,
        )
        fixture.scheduler.runNext()

        assertEquals("/c/generated", fixture.surface.state?.context?.conversationPath)
        assertEquals("语音生成的会话", fixture.surface.state?.context?.label)
        fixture.surface.openConversation()
        assertEquals("/c/generated", fixture.openedContext?.conversationPath)
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
        var sessionReady = sessionReady
        var authenticated = authenticated
        var sessionState = sessionState
        var beginBackingCount = 0
        val endBackingGraceful = mutableListOf<Boolean>()
        val endBackingCount: Int get() = endBackingGraceful.size
        var officialFallbackCount = 0
        var officialLoginCount = 0
        var sessionRecoveryCount = 0
        var openedContext: WebChatRealtimeVoiceContext? = null
        var context = WebChatRealtimeVoiceContext(
            "/c/test",
            "测试会话",
            savedToHistory = true,
        )
        val launchCache = WebChatRealtimeVoiceLaunchCache()
        val coordinator = WebChatRealtimeVoiceCoordinator(
            surface = surface,
            activeProvider = { WebChatProviderId.CHATGPT_WEB },
            consumerPort = { port },
            sessionReady = { this.sessionReady },
            authenticationState = {
                WebChatRealtimeVoiceAuthenticationPolicy.resolve(
                    this.authenticated,
                    this.sessionState,
                )
            },
            beginWebBacking = { beginBackingCount += 1; true },
            endWebBacking = { graceful -> endBackingGraceful += graceful },
            requestSessionRecovery = { sessionRecoveryCount += 1 },
            loginGate = loginGate,
            openOfficialLogin = { officialLoginCount += 1 },
            openOfficialFallback = { officialFallbackCount += 1 },
            resolveConversationContext = { context },
            openConversation = { openedContext = it },
            schedule = scheduler::schedule,
            backControl = back,
            launchCache = launchCache,
            log = {},
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
        var state: WebChatRealtimeVoiceState? = null
        var ensureVisibleCount = 0
        private var fallback: () -> Unit = {}
        private var close: () -> Unit = {}
        private var openConversation: () -> Unit = {}

        override fun show(
            onClose: () -> Unit,
            onRetry: () -> Unit,
            onOfficialFallback: () -> Unit,
            onOpenConversation: () -> Unit,
        ) {
            visible = true
            close = onClose
            fallback = onOfficialFallback
            openConversation = onOpenConversation
        }

        override fun render(state: WebChatRealtimeVoiceState) {
            this.state = state
        }

        override fun hide() {
            visible = false
        }

        override fun ensureVisibleOnTop() {
            ensureVisibleCount += 1
            visible = true
        }

        override fun isVisible(): Boolean = visible

        fun openOfficialFallback() = fallback()
        fun closeVoice() = close()
        fun openConversation() = openConversation.invoke()
    }

    private class FakePort : WebChatConsumerPort {
        var executeCount = 0
        var prepareStatus = WebChatConsumerCommandStatus.PENDING
        var prepareReturnsReceipt = true
        var voiceStatus = WebChatConsumerCommandStatus.PENDING
        var endControlAvailable = false
        var invokedControlId: String? = null
        var invokedControlCount = 0
        var requestControlsCount = 0

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
        override fun requestControls(): WebChatConsumerCommandResult {
            requestControlsCount += 1
            return accepted()
        }
        override fun invokeControl(controlId: String, userConfirmed: Boolean): WebChatConsumerCommandResult {
            invokedControlId = controlId
            invokedControlCount += 1
            return if (controlId == VoiceEndControl.id && userConfirmed) accepted() else rejected()
        }
        override fun updateControl(controlId: String, mutation: WebChatConsumerControlMutation) = rejected()

        override fun executeSessionCommand(action: String): WebChatConsumerCommandResult {
            executeCount += 1
            return when (action) {
                "chatgpt_prepare_realtime_voice" ->
                    accepted("prepare_1".takeIf { prepareReturnsReceipt })
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
