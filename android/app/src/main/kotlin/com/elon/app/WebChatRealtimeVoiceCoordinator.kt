package com.elon.app

import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.AppCompatActivity

internal class WebChatRealtimeVoiceCoordinator(
    private val surface: WebChatRealtimeVoiceSurface,
    private val activeProvider: () -> WebChatProviderId?,
    private val consumerPort: () -> WebChatConsumerPort?,
    private val sessionReady: () -> Boolean,
    private val beginWebBacking: () -> Boolean,
    private val endWebBacking: (Boolean) -> Unit,
    private val requestSessionRecovery: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val backControl: WebChatRealtimeVoiceBackControl,
) {
    private var generation = 0
    private var provider: WebChatProviderIdentity? = null
    private var prepareRequestId: String? = null
    private var preparedGeneration: Int? = null
    private var commandRequestId: String? = null
    private var closePending = false

    fun start(candidate: WebChatProviderIdentity): Boolean {
        if (
            !candidate.supports(WebChatProviderCapability.REALTIME_VOICE) ||
            activeProvider() != candidate.id
        ) {
            return false
        }
        if (surface.isVisible() && provider?.id == candidate.id) return true
        generation += 1
        provider = candidate
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        surface.show(::close, ::retry, ::openFallback)
        backControl.setEnabled(true)
        surface.render(
            WebChatRealtimeVoiceStage.PREPARING,
            "正在恢复本机网页会话，不会跳转到官网页面",
        )
        if (!beginWebBacking()) {
            fail("后台网页会话尚未建立，请重试")
            return true
        }
        requestSessionRecovery()
        scheduleStart(generation, attempt = 0, delayMs = 0L)
        return true
    }

    fun close() {
        if (closePending) return
        val port = consumerPort()
        val endControl = WebChatRealtimeVoiceEndPolicy.resolve(port?.state()?.controls.orEmpty())
        if (endControl != null) {
            val result = port?.invokeControl(endControl.id, userConfirmed = true)
            if (result?.accepted == true) {
                closePending = true
                surface.render(WebChatRealtimeVoiceStage.STARTING, "正在结束语音并返回对话")
                schedule(Runnable { finishClose(gracefulExit = true) }, END_VOICE_SETTLE_MS)
                return
            }
        }
        finishClose(gracefulExit = false)
    }

    private fun finishClose(gracefulExit: Boolean) {
        generation += 1
        closePending = false
        provider = null
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        backControl.setEnabled(false)
        surface.hide()
        endWebBacking(gracefulExit)
    }

    fun destroy() {
        finishClose(gracefulExit = false)
        backControl.dispose()
    }

    private fun retry() {
        val current = provider ?: return
        generation += 1
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        surface.render(WebChatRealtimeVoiceStage.PREPARING, "正在重新连接后台网页会话")
        if (!beginWebBacking()) {
            fail("后台网页会话尚未建立，请稍后重试")
            return
        }
        requestSessionRecovery()
        scheduleStart(generation, attempt = 0, delayMs = 0L)
        provider = current
    }

    private fun openFallback() {
        close()
        openOfficialFallback()
    }

    private fun scheduleStart(expectedGeneration: Int, attempt: Int, delayMs: Long) {
        schedule(Runnable { attemptStart(expectedGeneration, attempt) }, delayMs)
    }

    private fun attemptStart(expectedGeneration: Int, attempt: Int) {
        if (!isCurrent(expectedGeneration)) return
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
        surface.render(WebChatRealtimeVoiceStage.STARTING, "正在由后台网页启动官方语音连接")
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
            schedule(Runnable { markActive(expectedGeneration) }, COMMAND_SETTLE_MS)
        } else {
            pollCommand(expectedGeneration, result.requestId, attempt = 0)
        }
    }

    private fun prepareVoice(
        expectedGeneration: Int,
        attempt: Int,
        port: WebChatConsumerPort,
    ) {
        if (prepareRequestId != null) return
        surface.render(WebChatRealtimeVoiceStage.PREPARING, "正在同步输入状态并准备官方语音入口")
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
            finishPreparation(expectedGeneration, port)
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

    private fun finishPreparation(expectedGeneration: Int, port: WebChatConsumerPort) {
        if (!isCurrent(expectedGeneration)) return
        prepareRequestId = null
        preparedGeneration = expectedGeneration
        port.requestControls()
        scheduleStart(expectedGeneration, attempt = 0, delayMs = CONTROL_SETTLE_MS)
    }

    private fun pollCommand(expectedGeneration: Int, requestId: String, attempt: Int) {
        schedule(Runnable {
            if (!isCurrent(expectedGeneration) || commandRequestId != requestId) return@Runnable
            val request = consumerPort()?.state()?.commandRequests?.lastOrNull { it.id == requestId }
            when (request?.status) {
                WebChatConsumerCommandStatus.SUCCEEDED -> markActive(expectedGeneration)
                WebChatConsumerCommandStatus.FAILED,
                WebChatConsumerCommandStatus.TIMED_OUT ->
                    fail("官网未能启动实时语音，可重试或打开官网语音")
                WebChatConsumerCommandStatus.PENDING,
                WebChatConsumerCommandStatus.UNKNOWN,
                null -> if (attempt >= MAX_COMMAND_POLLS) {
                    fail("等待官网语音响应超时，可重试或打开官网语音")
                } else {
                    pollCommand(expectedGeneration, requestId, attempt + 1)
                }
            }
        }, COMMAND_POLL_DELAY_MS)
    }

    private fun markActive(expectedGeneration: Int) {
        if (!isCurrent(expectedGeneration)) return
        surface.render(
            WebChatRealtimeVoiceStage.ACTIVE,
            "官方语音界面已在后台启动，连接成功后可以直接说话",
        )
    }

    private fun fail(detail: String) {
        generation += 1
        prepareRequestId = null
        preparedGeneration = null
        commandRequestId = null
        endWebBacking(false)
        surface.render(WebChatRealtimeVoiceStage.FAILED, detail)
    }

    private fun isCurrent(expectedGeneration: Int): Boolean =
        expectedGeneration == generation && provider != null && surface.isVisible()

    private companion object {
        const val PREPARE_REALTIME_VOICE_ACTION = "chatgpt_prepare_realtime_voice"
        const val REALTIME_VOICE_ACTION = "chatgpt_start_realtime_voice"
        const val REALTIME_VOICE_SEMANTIC = "voice_mode"
        const val MAX_START_ATTEMPTS = 30
        const val MAX_COMMAND_POLLS = 40
        const val CONTROL_REFRESH_INTERVAL = 3
        const val RETRY_DELAY_MS = 400L
        const val COMMAND_POLL_DELAY_MS = 250L
        const val COMMAND_SETTLE_MS = 1_000L
        const val CONTROL_SETTLE_MS = 400L
        const val END_VOICE_SETTLE_MS = 350L
        val RETRYABLE_ERRORS = setOf(
            "bridge_not_ready",
            "adapter_generation_not_ready",
            "draft_unavailable",
            "realtime_voice_unavailable",
        )
    }
}

internal interface WebChatRealtimeVoiceBackControl {
    fun setEnabled(enabled: Boolean)
    fun dispose()
}

internal fun createWebChatRealtimeVoiceCoordinator(
    activity: AppCompatActivity,
    surface: WebChatRealtimeVoiceSurface,
    activeProvider: () -> WebChatProviderId?,
    consumerPort: () -> WebChatConsumerPort?,
    sessionReady: () -> Boolean,
    beginWebBacking: () -> Boolean,
    endWebBacking: (Boolean) -> Unit,
    requestSessionRecovery: () -> Unit,
    openOfficialFallback: () -> Unit,
): WebChatRealtimeVoiceCoordinator {
    lateinit var coordinator: WebChatRealtimeVoiceCoordinator
    val callback = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() = coordinator.close()
    }
    activity.onBackPressedDispatcher.addCallback(activity, callback)
    coordinator = WebChatRealtimeVoiceCoordinator(
        surface = surface,
        activeProvider = activeProvider,
        consumerPort = consumerPort,
        sessionReady = sessionReady,
        beginWebBacking = beginWebBacking,
        endWebBacking = endWebBacking,
        requestSessionRecovery = requestSessionRecovery,
        openOfficialFallback = openOfficialFallback,
        schedule = { task, delayMs -> activity.window.decorView.postDelayed(task, delayMs) },
        backControl = object : WebChatRealtimeVoiceBackControl {
            override fun setEnabled(enabled: Boolean) {
                callback.isEnabled = enabled
            }

            override fun dispose() = callback.remove()
        },
    )
    return coordinator
}
