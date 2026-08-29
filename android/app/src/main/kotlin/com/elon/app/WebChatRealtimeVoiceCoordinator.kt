package com.elon.app

import android.util.Log

internal class WebChatRealtimeVoiceCoordinator(
    private val surface: WebChatRealtimeVoiceSurface,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val sessionReady: () -> Boolean,
    private val audioActivationEvidence: () -> WebChatRealtimeVoiceActivationEvidence,
    private val authenticationState: () -> WebChatRealtimeVoiceAuthenticationState,
    private val beginWebBacking: () -> Boolean,
    private val endWebBacking: (Boolean) -> Unit,
    private val startManagedWebRtc: () -> Boolean,
    private val managedWebRtcState: () -> WebChatManagedRealtimeVoiceState,
    private val setManagedWebRtcMuted: (Boolean) -> Boolean,
    private val showInteractiveActivation: () -> Boolean,
    private val restoreNativeSurface: () -> Unit,
    private val requestSessionRecovery: () -> Unit,
    private val loginGate: WebChatRealtimeVoiceLoginGate,
    private val openOfficialLogin: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val resolveConversationContext: () -> WebChatRealtimeVoiceContext,
    private val openConversation: (WebChatRealtimeVoiceContext) -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val backControl: WebChatRealtimeVoiceBackControl,
    private val backgroundBridge: WebChatRealtimeVoiceBackgroundPort,
    private val launchCache: WebChatRealtimeVoiceLaunchCache = WebChatRealtimeVoiceLaunchCache(),
    private val monotonicClockMs: () -> Long = { System.nanoTime() / 1_000_000L },
    private val log: (String) -> Unit = { message -> Log.i(TAG, message) },
) : WebChatRealtimeVoiceBackgroundControlSink {
    private var generation = 0
    private var provider: WebChatProviderIdentity? = null
    private var prepareRequestId: String? = null
    private var preparedGeneration: Int? = null
    private var commandRequestId: String? = null
    private var closePending = false
    private var closeFailed = false
    private var closeCommandAccepted = false
    private var automaticCloseRetries = 0
    private var interactiveActivation = false
    private var pendingLoginProvider: WebChatProviderIdentity? = null
    private var waitingForLoginReturn = false
    private var startedAtElapsedMs = 0L
    private var lastState: WebChatRealtimeVoiceState? = null
    private var hostResumed = true
    private var backingEndedForClose = false
    private val closeSettlement = WebChatRealtimeVoiceCloseSettlement()
    private val delayedCloseMonitor = WebChatRealtimeVoiceDelayedCloseMonitor(
        schedule = schedule,
        currentState = { consumerPort()?.state() },
        requestControls = { consumerPort()?.requestControls() },
        nowMs = monotonicClockMs,
        onConfirmed = { state ->
            log("voice_close_delayed_confirmation")
            state?.let { current -> provider?.let { launchCache.observe(it.id, current) } }
            finishClose(gracefulExit = true)
        },
        onExpired = { log("voice_close_delayed_confirmation_expired") },
    )
    private val activationGate = WebChatRealtimeVoiceActivationGate()
    private val activationMonitor = WebChatRealtimeVoiceActivationMonitor(
        schedule = schedule,
        observeActivation = {
            activationGate.observe(
                currentActivationEvidence(),
                attempt = 0,
                pollLimit = Int.MAX_VALUE,
            )
        },
        requestControls = { consumerPort()?.requestControls() },
        onConfirmed = { markActive(generation) },
        onRejected = ::fail,
        onWatchdogExhausted = { log("voice_activation_observation_still_stale") },
    )
    private val pauseController = WebChatRealtimeVoicePauseController(
        consumerPort = consumerPort,
        schedule = schedule,
        onCompleted = { paused, detail ->
            backgroundBridge.setPaused(paused, detail)
            lastState?.takeIf { it.lifecycle.isVoiceOngoing() }?.let {
                render(it.lifecycle, detail, it.turn, paused, it.observation)
            }
        },
        onFailed = backgroundBridge::reportControlFailure,
    )
    private val managedVoiceLaunch = WebChatManagedRealtimeVoiceLaunchCoordinator(
        startTransport = startManagedWebRtc,
        transportState = managedWebRtcState,
        setMuted = setManagedWebRtcMuted,
        schedule = schedule,
        isCurrent = ::isCurrent,
    )
    private val pauseRouter = WebChatRealtimeVoicePauseRouter(
        managedVoice = managedVoiceLaunch,
        officialVoice = pauseController,
        onManagedApplied = { paused, detail ->
            backgroundBridge.setPaused(paused, detail)
            lastState?.takeIf { it.lifecycle.isVoiceOngoing() }
                ?.let { render(it.lifecycle, detail, it.turn, paused, it.observation) }
        },
    )
    private val contextTracker = WebChatRealtimeVoiceContextTracker(
        resolve = resolveConversationContext,
        schedule = schedule,
        isCurrent = ::isCurrent,
        onChanged = { context -> lastState = lastState?.copy(context = context).also { it?.let(surface::render) } },
    )

    fun start(candidate: WebChatProviderIdentity): Boolean {
        if (
            !candidate.supports(WebChatProviderCapability.REALTIME_VOICE) ||
            activeProvider() != candidate.id
        ) {
            return false
        }
        if (authenticationState() == WebChatRealtimeVoiceAuthenticationState.GUEST) {
            requireLogin(candidate, stopBacking = false)
            return true
        }
        if (surface.isVisible() && provider?.id == candidate.id) {
            if (WebChatRealtimeVoiceFastPath.shouldRetryVisibleSurface(lastState?.lifecycle)) {
                retry()
                return true
            }
            surface.ensureVisibleOnTop()
            return true
        }
        generation += 1
        startedAtElapsedMs = monotonicClockMs()
        provider = candidate
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        closeFailed = false
        closeCommandAccepted = false
        activationMonitor.cancel()
        delayedCloseMonitor.cancel()
        automaticCloseRetries = 0
        interactiveActivation = false
        managedVoiceLaunch.reset()
        backingEndedForClose = false
        pauseController.reset()
        backgroundBridge.stop()
        contextTracker.begin()
        surface.show(::close, ::retry, ::openFallback, ::openVoiceConversation)
        // The floating voice control owns hangup while back keeps normal app navigation.
        backControl.setEnabled(false)
        render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在恢复本机网页会话，不会跳转到官网页面")
        if (!beginWebBacking()) {
            fail("后台网页会话尚未建立，请重试")
            return true
        }
        applyLaunchPlan(candidate, isRetry = false)
        scheduleStart(generation, attempt = 0, delayMs = 0L)
        return true
    }

    fun isActive(): Boolean = provider != null || closePending
    fun close() {
        if (closePending) return
        if (closeFailed) {
            beginClose()
            return
        }
        if (
            lastState?.lifecycle == WebChatRealtimeVoiceLifecycle.FAILED ||
            lastState?.lifecycle == WebChatRealtimeVoiceLifecycle.CONNECTING && commandRequestId == null
        ) {
            finishClose(gracefulExit = false)
            return
        }
        beginClose()
    }
    private fun beginClose() {
        activationMonitor.cancel()
        delayedCloseMonitor.cancel()
        closePending = true
        closeFailed = false
        closeCommandAccepted = false
        automaticCloseRetries = 0
        pauseController.reset()
        finishInteractiveActivation()
        if (managedVoiceLaunch.ownsNativeMedia()) {
            endWebBacking(true)
            backingEndedForClose = true
            finishClose(gracefulExit = true)
            return
        }
        closeSettlement.begin()
        render(WebChatRealtimeVoiceLifecycle.ENDING, "正在结束语音并保留当前对话")
        advanceCloseSettlement(generation)
    }

    private fun scheduleCloseSettlement(expectedGeneration: Int) {
        schedule(
            Runnable { advanceCloseSettlement(expectedGeneration) },
            CLOSE_POLL_DELAY_MS,
        )
    }

    private fun advanceCloseSettlement(expectedGeneration: Int) {
        if (!closePending || expectedGeneration != generation) return
        val port = consumerPort()
        when (val decision = closeSettlement.observe(port?.state())) {
            is WebChatRealtimeVoiceCloseDecision.InvokeEnd -> {
                val result = port?.invokeControl(decision.control.id, userConfirmed = true)
                if (result?.accepted != true) {
                    finishClose(gracefulExit = false)
                    return
                }
                closeCommandAccepted = true
                closeSettlement.endInvocationAccepted()
                port.requestControls()
                scheduleCloseSettlement(expectedGeneration)
            }
            is WebChatRealtimeVoiceCloseDecision.Wait -> {
                if (decision.refreshControls) port?.requestControls()
                scheduleCloseSettlement(expectedGeneration)
            }
            WebChatRealtimeVoiceCloseDecision.CompleteGracefully -> {
                port?.state()?.let { state -> provider?.let { launchCache.observe(it.id, state) } }
                finishClose(gracefulExit = true)
            }
            WebChatRealtimeVoiceCloseDecision.CompleteInterrupted -> retryCloseOrReportStillActive()
        }
    }

    private fun retryCloseOrReportStillActive() {
        if (automaticCloseRetries < MAX_AUTOMATIC_CLOSE_RETRIES) {
            automaticCloseRetries += 1
            closeSettlement.begin()
            consumerPort()?.requestControls()
            render(
                WebChatRealtimeVoiceLifecycle.ENDING,
                "挂断暂未生效，正在再次尝试",
            )
            advanceCloseSettlement(generation)
            return
        }
        failClose()
    }

    private fun failClose() {
        log("voice_close_unconfirmed retries=$automaticCloseRetries")
        closePending = false
        closeFailed = true
        closeSettlement.reset()
        consumerPort()?.requestControls()
        render(
            WebChatRealtimeVoiceLifecycle.HANGUP_UNCONFIRMED,
            "挂断尚未生效，语音仍在进行。可再次挂断，或打开官网语音确认关闭",
        )
        // A missing end control is not proof that voice stopped. Only reconcile a
        // late page transition after the official hangup command was accepted.
        if (closeCommandAccepted) delayedCloseMonitor.begin()
    }

    fun onConsumerStateChanged(state: WebChatConsumerState) {
        provider?.let { launchCache.observe(it.id, state) }
        activationMonitor.observeEvent()
        delayedCloseMonitor.observe(state)
    }

    private fun finishClose(gracefulExit: Boolean) {
        log("voice_close graceful=$gracefulExit")
        generation += 1
        closePending = false
        closeFailed = false
        closeCommandAccepted = false
        delayedCloseMonitor.cancel()
        automaticCloseRetries = 0
        finishInteractiveActivation()
        provider = null
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        closeSettlement.reset()
        activationMonitor.cancel()
        backControl.setEnabled(false)
        surface.hide()
        backgroundBridge.stop()
        if (!backingEndedForClose) endWebBacking(gracefulExit)
        backingEndedForClose = false
        managedVoiceLaunch.reset()
        contextTracker.reset()
        lastState = null
    }

    fun destroy() {
        clearPendingLogin()
        loginGate.dismiss()
        finishClose(gracefulExit = false)
        backgroundBridge.dispose()
        backControl.dispose()
    }

    fun onHostResumed() {
        hostResumed = true
        if (provider != null) contextTracker.refresh()
        syncHostSurface()
        val candidate = pendingLoginProvider ?: return
        if (!waitingForLoginReturn) return
        generation += 1
        requestSessionRecovery()
        pollAuthentication(generation, candidate, attempt = 0)
    }

    fun onHostPaused() {
        hostResumed = false
        syncHostSurface()
    }

    fun onActiveSurfaceChanged() = syncHostSurface()

    private fun syncHostSurface() {
        val state = lastState
        val voiceProvider = provider?.id
        val nativeSurfaceVisible = hostResumed && state != null && voiceProvider == activeProvider()
        surface.setHostVisible(nativeSurfaceVisible)
        backgroundBridge.setHostVisible(nativeSurfaceVisible)
        if (nativeSurfaceVisible && state != null) surface.render(state)
    }

    override fun pauseFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource) =
        lastState?.takeIf { it.lifecycle.isVoiceOngoing() }?.let { pauseRouter.request(true, source) } ?: Unit

    override fun resumeFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource) =
        lastState?.takeIf { it.lifecycle.isVoiceOngoing() }?.let { pauseRouter.request(false, source) } ?: Unit

    override fun hangUpFromBackground() = close()

    private fun retry() {
        if (closeFailed) {
            beginClose()
            return
        }
        val current = provider ?: return
        generation += 1
        startedAtElapsedMs = monotonicClockMs()
        closePending = false
        closeFailed = false
        closeCommandAccepted = false
        activationMonitor.cancel()
        automaticCloseRetries = 0
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        finishInteractiveActivation()
        managedVoiceLaunch.reset()
        backingEndedForClose = false
        pauseController.reset()
        render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在重新连接后台网页会话")
        if (!beginWebBacking()) {
            fail("后台网页会话尚未建立，请稍后重试")
            return
        }
        applyLaunchPlan(current, isRetry = true)
        scheduleStart(generation, attempt = 0, delayMs = 0L)
    }

    private fun applyLaunchPlan(candidate: WebChatProviderIdentity, isRetry: Boolean) {
        val launchPlan = launchCache.plan(candidate.id, consumerPort()?.state(), sessionReady())
        log("voice_${if (isRetry) "retry" else "start"} provider=${candidate.id.wireValue} plan=$launchPlan")
        when (launchPlan) {
            WebChatRealtimeVoiceLaunchPlan.DIRECT ->
                render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在启动当前会话的实时语音")
            WebChatRealtimeVoiceLaunchPlan.REFRESH_CONTROLS -> {
                render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在刷新当前会话的语音入口")
                consumerPort()?.requestControls()
            }
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION -> requestSessionRecovery()
        }
    }

    private fun openFallback() {
        finishClose(gracefulExit = false)
        openOfficialFallback()
    }

    private fun scheduleStart(expectedGeneration: Int, attempt: Int, delayMs: Long) {
        schedule(Runnable { attemptStart(expectedGeneration, attempt) }, delayMs)
    }

    private fun attemptStart(expectedGeneration: Int, attempt: Int) {
        if (!isCurrent(expectedGeneration)) return
        if (authenticationState() == WebChatRealtimeVoiceAuthenticationState.GUEST) {
            requireLogin(provider ?: return, stopBacking = true)
            return
        }
        if (!sessionReady()) {
            if (attempt >= MAX_START_ATTEMPTS) {
                fail("连接网页会话超时，可重试或打开官网语音")
                return
            }
            requestSessionRecovery()
            scheduleStart(expectedGeneration, attempt + 1, RETRY_DELAY_MS)
            return
        }
        val port = consumerPort()
        if (port == null) {
            fail("本机网页会话接口未就绪")
            return
        }
        if (preparedGeneration != expectedGeneration) {
            prepareVoice(expectedGeneration, attempt, port)
            return
        }
        val voiceReady = port.state().controls.any { descriptor ->
            descriptor.control.semantic == REALTIME_VOICE_SEMANTIC && descriptor.control.enabled
        }
        if (!voiceReady) {
            if (attempt >= MAX_START_ATTEMPTS) {
                fail("官网语音入口暂未读到，可重试或打开官网语音")
                return
            }
            if (attempt % CONTROL_REFRESH_INTERVAL == 0) port.requestControls()
            scheduleStart(expectedGeneration, attempt + 1, RETRY_DELAY_MS)
            return
        }
        provider?.let { launchCache.observe(it.id, port.state()) }
        if (startManagedVoice(expectedGeneration, port)) return
        startOfficialVoice(expectedGeneration, attempt, port)
    }

    private fun startOfficialVoice(
        expectedGeneration: Int,
        attempt: Int,
        port: WebChatConsumerPort,
    ) {
        render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在由后台网页启动官方语音连接")
        activationGate.begin(currentActivationEvidence())
        val result = port.executeSessionCommand(REALTIME_VOICE_ACTION)
        if (!result.accepted) {
            if (attempt < MAX_START_ATTEMPTS && result.error in RETRYABLE_ERRORS) {
                port.requestControls()
                scheduleStart(expectedGeneration, attempt + 1, RETRY_DELAY_MS)
            } else {
                fail("官网语音入口没有响应，可重试或打开官网语音")
            }
            return
        }
        commandRequestId = result.requestId
        if (result.requestId == null) {
            schedule(Runnable { pollAudioActivation(expectedGeneration, attempt = 0) }, COMMAND_SETTLE_MS)
        } else {
            pollCommand(expectedGeneration, result.requestId, attempt = 0)
        }
    }

    private fun startManagedVoice(
        expectedGeneration: Int,
        port: WebChatConsumerPort,
    ): Boolean = managedVoiceLaunch.start(
        generation = expectedGeneration,
        onConnecting = {
            activationGate.begin(currentActivationEvidence())
            render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在建立原生语音连接")
        },
        onActive = { markActive(expectedGeneration) },
        onOfficialFallback = {
            render(
                WebChatRealtimeVoiceLifecycle.CONNECTING,
                "原生连接未完成，正在自动恢复官网语音",
            )
            pollAudioActivation(expectedGeneration, attempt = 0)
        },
        onUnavailable = { startOfficialVoice(expectedGeneration, attempt = 0, port = port) },
        onTimeout = { fail("原生语音连接超时，可重试或打开官网语音") },
    )

    private fun prepareVoice(
        expectedGeneration: Int,
        attempt: Int,
        port: WebChatConsumerPort,
    ) {
        if (prepareRequestId != null) return
        render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在同步输入状态并准备官方语音入口")
        val result = port.executeSessionCommand(PREPARE_REALTIME_VOICE_ACTION)
        if (!result.accepted) {
            if (attempt < MAX_START_ATTEMPTS && result.error in RETRYABLE_ERRORS) {
                requestSessionRecovery()
                scheduleStart(expectedGeneration, attempt + 1, RETRY_DELAY_MS)
            } else {
                val detail = if (result.error == "native_draft_not_empty") {
                    "请先发送或清空输入内容，再启动实时语音"
                } else {
                    "同步网页输入状态失败，可重试或打开官网语音"
                }
                fail(detail)
            }
            return
        }
        val requestId = result.requestId
        if (requestId == null) {
            finishPreparation(expectedGeneration, port, controlsAlreadyCurrent = true)
        } else {
            prepareRequestId = requestId
            pollPreparation(expectedGeneration, requestId, attempt = 0)
        }
    }

    private fun pollPreparation(expectedGeneration: Int, requestId: String, attempt: Int) {
        schedule(Runnable {
            if (!isCurrent(expectedGeneration) || prepareRequestId != requestId) return@Runnable
            val port = consumerPort() ?: run {
                fail("本机网页会话接口未就绪")
                return@Runnable
            }
            val request = port.state().commandRequests.lastOrNull { it.id == requestId }
            when (request?.status) {
                WebChatConsumerCommandStatus.SUCCEEDED -> finishPreparation(expectedGeneration, port)
                WebChatConsumerCommandStatus.FAILED,
                WebChatConsumerCommandStatus.TIMED_OUT ->
                    fail("同步网页输入状态失败，可重试或打开官网语音")
                WebChatConsumerCommandStatus.PENDING,
                WebChatConsumerCommandStatus.UNKNOWN,
                null -> if (attempt >= MAX_COMMAND_POLLS) {
                    fail("等待网页输入状态同步超时，可重试或打开官网语音")
                } else {
                    pollPreparation(expectedGeneration, requestId, attempt + 1)
                }
            }
        }, COMMAND_POLL_DELAY_MS)
    }

    private fun finishPreparation(
        expectedGeneration: Int,
        port: WebChatConsumerPort,
        controlsAlreadyCurrent: Boolean = false,
    ) {
        if (!isCurrent(expectedGeneration)) return
        prepareRequestId = null
        preparedGeneration = expectedGeneration
        if (WebChatRealtimeVoiceFastPath.canStartAfterPreparation(controlsAlreadyCurrent, port.state())) {
            attemptStart(expectedGeneration, attempt = 0)
            return
        }
        port.requestControls()
        scheduleStart(expectedGeneration, attempt = 0, delayMs = CONTROL_SETTLE_MS)
    }

    private fun pollCommand(expectedGeneration: Int, requestId: String, attempt: Int) {
        schedule(Runnable {
            if (!isCurrent(expectedGeneration) || commandRequestId != requestId) return@Runnable
            val request = consumerPort()?.state()?.commandRequests?.lastOrNull { it.id == requestId }
            when (request?.status) {
                WebChatConsumerCommandStatus.SUCCEEDED -> pollAudioActivation(expectedGeneration, attempt = 0)
                WebChatConsumerCommandStatus.FAILED ->
                    fail("官网未能启动实时语音，可重试或打开官网语音")
                WebChatConsumerCommandStatus.TIMED_OUT ->
                    markActivationUnconfirmed(expectedGeneration)
                WebChatConsumerCommandStatus.PENDING,
                WebChatConsumerCommandStatus.UNKNOWN,
                null -> if (attempt >= MAX_COMMAND_POLLS) {
                    markActivationUnconfirmed(expectedGeneration)
                } else {
                    pollCommand(expectedGeneration, requestId, attempt + 1)
                }
            }
        }, COMMAND_POLL_DELAY_MS)
    }

    private fun pollAudioActivation(expectedGeneration: Int, attempt: Int) {
        if (!isCurrent(expectedGeneration)) return
        val pollLimit = if (interactiveActivation) {
            MAX_INTERACTIVE_ACTIVATION_POLLS
        } else {
            MAX_BACKGROUND_ACTIVATION_POLLS
        }
        when (val decision = activationGate.observe(currentActivationEvidence(), attempt, pollLimit)) {
            WebChatRealtimeVoiceActivationDecision.Active -> markActive(expectedGeneration)
            is WebChatRealtimeVoiceActivationDecision.Rejected -> {
                if (!interactiveActivation && beginInteractiveActivation(expectedGeneration)) return
                fail(decision.detail)
            }
            WebChatRealtimeVoiceActivationDecision.Unconfirmed -> {
                if (!interactiveActivation && beginInteractiveActivation(expectedGeneration)) return
                markActivationUnconfirmed(expectedGeneration)
            }
            is WebChatRealtimeVoiceActivationDecision.Wait -> {
                render(WebChatRealtimeVoiceLifecycle.CONNECTING, decision.detail)
                schedule(
                    Runnable { pollAudioActivation(expectedGeneration, attempt + 1) },
                    AUDIO_ACTIVATION_POLL_DELAY_MS,
                )
            }
        }
    }

    private fun markActive(expectedGeneration: Int) {
        if (!isCurrent(expectedGeneration)) return
        val backgroundAlreadyStarted = lastState?.lifecycle.isVoiceOngoing()
        activationMonitor.cancel()
        finishInteractiveActivation()
        log("voice_active elapsed_ms=${monotonicClockMs() - startedAtElapsedMs}")
        render(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "语音已连接，可继续使用当前页面",
            turn = WebChatRealtimeVoiceTurn.IDLE,
        )
        lastState?.let { state ->
            if (!backgroundAlreadyStarted && !backgroundBridge.start(state)) {
                log("voice_background_service_unavailable")
            }
        }
        // Permission and official voice UI handoffs may temporarily pause the Activity.
        // Reconcile once activation settles so the correct native/system surface wins.
        syncHostSurface()
        contextTracker.scheduleRefresh(expectedGeneration)
    }

    private fun markActivationUnconfirmed(expectedGeneration: Int) {
        if (!isCurrent(expectedGeneration)) return
        finishInteractiveActivation()
        log("voice_activation_observation_stale elapsed_ms=${monotonicClockMs() - startedAtElapsedMs}")
        render(
            lifecycle = WebChatRealtimeVoiceLifecycle.ACTIVE,
            detail = "语音仍可继续，正在后台同步通话状态",
            turn = WebChatRealtimeVoiceTurn.UNKNOWN,
            observation = WebChatRealtimeVoiceObservation.STALE,
        )
        lastState?.let { state ->
            if (!backgroundBridge.start(state)) log("voice_background_service_unavailable")
        }
        syncHostSurface()
        contextTracker.scheduleRefresh(expectedGeneration)
        activationMonitor.begin()
    }

    private fun fail(detail: String) {
        log("voice_failed detail=$detail")
        generation += 1
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        managedVoiceLaunch.reset()
        finishInteractiveActivation()
        activationMonitor.cancel()
        pauseController.reset()
        delayedCloseMonitor.cancel()
        backgroundBridge.stop()
        endWebBacking(false)
        render(WebChatRealtimeVoiceLifecycle.FAILED, detail)
    }

    private fun beginInteractiveActivation(expectedGeneration: Int): Boolean {
        if (!isCurrent(expectedGeneration) || !showInteractiveActivation()) return false
        interactiveActivation = true
        activationGate.begin(currentActivationEvidence())
        render(
            WebChatRealtimeVoiceLifecycle.CONNECTING,
            "请点页面右下角蓝色语音按钮，连接后自动返回原生聊天",
        )
        schedule(
            Runnable { pollAudioActivation(expectedGeneration, attempt = 0) },
            AUDIO_ACTIVATION_POLL_DELAY_MS,
        )
        return true
    }

    private fun finishInteractiveActivation() {
        if (!interactiveActivation) return
        interactiveActivation = false
        restoreNativeSurface()
    }

    private fun currentActivationEvidence() = WebChatRealtimeVoiceActivationEvidencePolicy.resolve(
        audioActivationEvidence(),
        consumerPort()?.state(),
    )

    private fun render(
        lifecycle: WebChatRealtimeVoiceLifecycle,
        detail: String,
        turn: WebChatRealtimeVoiceTurn = WebChatRealtimeVoiceTurn.UNKNOWN,
        paused: Boolean = lastState?.paused ?: false,
        observation: WebChatRealtimeVoiceObservation = WebChatRealtimeVoiceObservation.CONFIRMED,
    ) {
        val state = WebChatRealtimeVoiceState(
            lifecycle = lifecycle,
            detail = detail,
            turn = turn,
            context = contextTracker.value,
            paused = paused,
            observation = observation,
        )
        lastState = state
        surface.render(state)
        backgroundBridge.update(state)
    }

    private fun openVoiceConversation() {
        if (provider != null) contextTracker.refresh()
        contextTracker.value?.let(openConversation)
        surface.ensureVisibleOnTop()
    }

    private fun requireLogin(candidate: WebChatProviderIdentity, stopBacking: Boolean) {
        generation += 1
        provider = null
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        closePending = false
        closeFailed = false
        closeCommandAccepted = false
        managedVoiceLaunch.reset()
        backingEndedForClose = false
        activationMonitor.cancel()
        finishInteractiveActivation()
        pauseController.reset()
        backgroundBridge.stop()
        backControl.setEnabled(false)
        if (surface.isVisible()) surface.hide()
        if (stopBacking) endWebBacking(false)
        pendingLoginProvider = candidate
        waitingForLoginReturn = false
        loginGate.show(
            onOfficialLogin = {
                waitingForLoginReturn = true
                loginGate.dismiss()
                openOfficialLogin()
            },
            onCancel = ::clearPendingLogin,
        )
    }

    private fun pollAuthentication(
        expectedGeneration: Int,
        candidate: WebChatProviderIdentity,
        attempt: Int,
    ) {
        schedule(Runnable {
            if (
                expectedGeneration != generation ||
                !waitingForLoginReturn ||
                pendingLoginProvider?.id != candidate.id
            ) {
                return@Runnable
            }
            if (authenticationState() == WebChatRealtimeVoiceAuthenticationState.AUTHENTICATED) {
                clearPendingLogin()
                start(candidate)
                return@Runnable
            }
            if (attempt >= MAX_AUTHENTICATION_POLLS) {
                requireLogin(candidate, stopBacking = false)
                return@Runnable
            }
            requestSessionRecovery()
            pollAuthentication(expectedGeneration, candidate, attempt + 1)
        }, AUTHENTICATION_POLL_DELAY_MS)
    }

    private fun clearPendingLogin() {
        pendingLoginProvider = null
        waitingForLoginReturn = false
    }

    private fun isCurrent(expectedGeneration: Int): Boolean =
        expectedGeneration == generation && provider != null && surface.isVisible()

    private companion object {
        const val TAG = "WebChatRealtimeVoice"
        const val PREPARE_REALTIME_VOICE_ACTION = "chatgpt_prepare_realtime_voice"
        const val REALTIME_VOICE_ACTION = "chatgpt_start_realtime_voice"
        const val REALTIME_VOICE_SEMANTIC = "voice_mode"
        const val MAX_START_ATTEMPTS = 30
        const val MAX_COMMAND_POLLS = 40
        const val CONTROL_REFRESH_INTERVAL = 3
        const val RETRY_DELAY_MS = 400L
        const val COMMAND_POLL_DELAY_MS = 250L
        const val COMMAND_SETTLE_MS = 1_000L
        const val AUDIO_ACTIVATION_POLL_DELAY_MS = 250L
        const val MAX_BACKGROUND_ACTIVATION_POLLS = 12
        const val MAX_INTERACTIVE_ACTIVATION_POLLS = 60
        const val MAX_AUTOMATIC_CLOSE_RETRIES = 1
        const val CONTROL_SETTLE_MS = 400L
        const val CLOSE_POLL_DELAY_MS = 250L
        const val AUTHENTICATION_POLL_DELAY_MS = 400L
        const val MAX_AUTHENTICATION_POLLS = 30
        val RETRYABLE_ERRORS = setOf(
            "bridge_not_ready",
            "adapter_generation_not_ready",
            "draft_unavailable",
            "realtime_voice_unavailable",
        )

    }
}

private fun WebChatRealtimeVoiceLifecycle?.isVoiceOngoing(): Boolean =
    this == WebChatRealtimeVoiceLifecycle.ACTIVE ||
        this == WebChatRealtimeVoiceLifecycle.HANGUP_UNCONFIRMED

internal interface WebChatRealtimeVoiceBackControl {
    fun setEnabled(enabled: Boolean)
    fun dispose()
}
