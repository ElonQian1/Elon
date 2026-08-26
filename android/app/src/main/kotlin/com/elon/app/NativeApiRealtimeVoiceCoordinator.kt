package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.elon.app.chatgptweb.ChatGptWebAudioPermissionController

/**
 * Owns the native microphone -> app WebSocket -> Realtime API -> PCM path.
 * This transport always writes to a new local 一龙 AI conversation and never
 * claims ownership of the user's ChatGPT web-account conversation.
 */
internal class NativeApiRealtimeVoiceCoordinator(
    private val activity: AppCompatActivity,
    private val surface: WebChatRealtimeVoiceSurface,
    private val audioPermissionController: ChatGptWebAudioPermissionController,
    private val serverUrl: () -> String,
    private val userId: () -> String,
    private val openLocalConversation: () -> Unit,
    private val openOfficialFallback: () -> Unit,
    private val backgroundBridge: WebChatRealtimeVoiceBackgroundBridge =
        WebChatRealtimeVoiceBackgroundBridge(activity),
) : WebChatRealtimeVoiceBackgroundControlSink {
    private val transport = RealtimeVoiceTransportCatalog.serverApiExperiment
    private val context = RealtimeVoiceTransportPolicy.contextFor(transport)
    private var controller: RealtimeVoiceController? = null
    private var player: RealtimePcmPlayer? = null
    private var generation = 0
    private var running = false
    private var paused = false
    private var hostResumed = true
    private var turn = WebChatRealtimeVoiceTurn.UNKNOWN
    private var state: WebChatRealtimeVoiceState? = null

    init {
        backgroundBridge.attach(this)
    }

    fun start(): Boolean {
        if (running) {
            surface.ensureVisibleOnTop()
            return true
        }
        running = true
        paused = false
        turn = WebChatRealtimeVoiceTurn.UNKNOWN
        surface.show(::close, ::retry, ::openOfficialVoice, ::openConversation)
        render(WebChatRealtimeVoiceLifecycle.CONNECTING, "正在连接${transport.label}")
        audioPermissionController.runWithMicrophone(
            action = ::startGrantedSession,
            onPermissionDenied = { fail("需要麦克风权限才能开始实时语音") },
        )
        return true
    }

    fun isActive(): Boolean = running

    fun onHostResumed() {
        hostResumed = true
        surface.setHostVisible(true)
        backgroundBridge.setHostVisible(true)
    }

    fun onHostPaused() {
        hostResumed = false
        surface.setHostVisible(false)
        backgroundBridge.setHostVisible(false)
    }

    fun destroy() {
        stopSession(hideSurface = true)
        backgroundBridge.dispose()
    }

    override fun pauseFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource) {
        if (!running || paused) return
        controller?.pauseInput()
        player?.clear()
        player?.outputEnabled = false
        paused = true
        val detail = if (source == WebChatRealtimeVoiceBackgroundControlSource.MEDIA) {
            "其他媒体正在播放，语音已自动暂停"
        } else {
            "实时语音已暂停"
        }
        render(WebChatRealtimeVoiceLifecycle.ACTIVE, detail, turn)
        backgroundBridge.setPaused(true, detail)
    }

    override fun resumeFromBackground(source: WebChatRealtimeVoiceBackgroundControlSource) {
        if (!running || !paused) return
        val resumed = controller?.resumeInput() == true
        if (!resumed) {
            backgroundBridge.reportControlFailure("麦克风恢复失败，请重试")
            fail("麦克风恢复失败，请重试")
            return
        }
        player?.outputEnabled = true
        paused = false
        turn = WebChatRealtimeVoiceTurn.LISTENING
        render(WebChatRealtimeVoiceLifecycle.ACTIVE, "正在聆听", turn)
        backgroundBridge.setPaused(false, "正在聆听")
    }

    override fun hangUpFromBackground() = close()

    private fun startGrantedSession() {
        if (!running) return
        val baseUrl = serverUrl().trim()
        val owner = userId().trim()
        if (baseUrl.isBlank() || owner.isBlank()) {
            fail("一龙账号会话尚未就绪")
            return
        }
        val expectedGeneration = ++generation
        controller?.shutdown()
        val output = RealtimePcmPlayer()
        if (!output.start()) {
            fail("无法启动实时语音播放")
            return
        }
        player = output
        controller = RealtimeVoiceController(
            context = activity,
            baseHttpUrl = baseUrl,
            userId = owner,
            mode = RealtimeVoiceWsClient.Mode.RealtimeChat,
            target = RealtimeVoiceWsClient.Target.SocialAiDirect,
            onTranscriptFinal = {
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.THINKING, "正在理解")
                }
            },
            onAiProgress = {
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.THINKING, "正在生成回答")
                }
            },
            onAiDone = { _, _ ->
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.LISTENING, "正在聆听")
                }
            },
            onAiError = { message ->
                onCurrent(expectedGeneration) { fail("实时回答失败：${message.take(48)}") }
            },
            onRealtimeAudio = { chunk ->
                player?.play(chunk)
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.SPEAKING, "正在回答")
                }
            },
            onRealtimeSpeechStarted = {
                onCurrent(expectedGeneration) {
                    player?.clear()
                    updateTurn(WebChatRealtimeVoiceTurn.LISTENING, "正在聆听")
                }
            },
            onRealtimeSpeechStopped = {
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.THINKING, "正在理解")
                }
            },
            onRealtimeResponseDone = {
                onCurrent(expectedGeneration) {
                    updateTurn(WebChatRealtimeVoiceTurn.LISTENING, "正在聆听")
                }
            },
            onReady = {
                onCurrent(expectedGeneration) {
                    turn = WebChatRealtimeVoiceTurn.LISTENING
                    val activeState = render(
                        WebChatRealtimeVoiceLifecycle.ACTIVE,
                        "正在聆听",
                        turn,
                    )
                    backgroundBridge.start(activeState)
                }
            },
            onClosed = {
                onCurrent(expectedGeneration) { fail("实时语音连接已断开，请重试") }
            },
            onError = { message ->
                onCurrent(expectedGeneration) { fail(message.take(72)) }
            },
        ).also { it.start(activity.lifecycleScope) }
    }

    private fun updateTurn(next: WebChatRealtimeVoiceTurn, detail: String) {
        if (!running || paused || turn == next) return
        turn = next
        render(WebChatRealtimeVoiceLifecycle.ACTIVE, detail, next)
    }

    private fun retry() {
        stopSession(hideSurface = false)
        start()
    }

    private fun close() {
        if (!running) return
        render(WebChatRealtimeVoiceLifecycle.ENDING, "正在结束原生实时语音", turn)
        stopSession(hideSurface = true)
    }

    private fun openOfficialVoice() {
        stopSession(hideSurface = true)
        openOfficialFallback()
    }

    private fun openConversation() {
        openLocalConversation()
        if (hostResumed) surface.ensureVisibleOnTop()
    }

    private fun fail(detail: String) {
        generation += 1
        controller?.shutdown()
        controller = null
        player?.release()
        player = null
        paused = false
        render(WebChatRealtimeVoiceLifecycle.FAILED, detail, turn)
        backgroundBridge.stop()
    }

    private fun stopSession(hideSurface: Boolean) {
        generation += 1
        running = false
        paused = false
        turn = WebChatRealtimeVoiceTurn.UNKNOWN
        controller?.shutdown()
        controller = null
        player?.release()
        player = null
        backgroundBridge.stop()
        state = null
        if (hideSurface) surface.hide()
    }

    private fun render(
        lifecycle: WebChatRealtimeVoiceLifecycle,
        detail: String,
        currentTurn: WebChatRealtimeVoiceTurn = turn,
    ): WebChatRealtimeVoiceState = WebChatRealtimeVoiceState(
        lifecycle = lifecycle,
        detail = detail,
        turn = currentTurn,
        context = context,
        paused = paused,
    ).also {
        state = it
        surface.render(it)
        backgroundBridge.update(it)
    }

    private fun onCurrent(expectedGeneration: Int, action: () -> Unit) {
        activity.runOnUiThread {
            if (running && expectedGeneration == generation) action()
        }
    }
}
