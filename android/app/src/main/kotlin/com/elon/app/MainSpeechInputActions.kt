package com.elon.app

import android.Manifest
import android.app.AlertDialog
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.File

internal class MainSpeechInputActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val speechPermissionRequest: Int,
    private val userId: () -> String,
    private val selectedAgent: () -> String?,
    private val activeConversation: () -> AppConversation,
    private val activeProject: () -> AppProject,
    private val voiceHoldButton: () -> TextView,
    private val sendVoiceAttachment: (PendingAttachment, String) -> Unit,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit,
    private val isFriendChatActive: () -> Boolean = { false },
    private val isSocialAiChatActive: () -> Boolean = { false },
    private val sendTextDirect: ((String) -> Unit)? = null
) {
    private val voiceRecorder = VoiceAudioRecorder(activity)
    private var speechRecognizer: SpeechRecognizer? = null
    private var isListeningForSpeech = false
    private var isHoldActive = false
    private var isSpeechCanceled = false
    private var speechSessionId = 0
    private var translationGeneration = 0
    // 语音消息模式：并行 ASR 采集原文
    private var voiceMessageAsrRecognizer: SpeechRecognizer? = null
    private var voiceMessageTranscription: String? = null
    private var voiceMessagePartialText: String? = null
    // 方案 B：实时语音 → 转写 → AI 投递
    private var realtimeController: RealtimeVoiceController? = null
    // 端上 Agent 流式识别（默认主聊天语音管线，由 VoiceInputModeSettings 控制）
    private var agentBridge: AgentVoiceBridge? = null
    private var agentVoiceActive = false
    private var agentLastFinalText: String = ""
    private var agentLastPartialText: String = ""
    // 仿微信全屏麦克风遮罩（在端上模式下启用）
    private var voiceOverlay: VoiceRecordingOverlay? = null

    init {
        // 预热端上 ASR 引擎：提前创建 SpeechRecognizer 并连接服务，
        // 让 mibrain 等厂商服务在用户按下麦克风之前完成绑定（约 70ms）。
        if (VoiceInputModeSettings.get(activity) == VoiceInputMode.LOCAL_AGENT_ASR) {
            agentBridge = AgentVoiceBridge(activity).also { it.prewarm() }
        }
    }

    fun startSpeechToText() {
        if (activeConversation().ended) return
        if (ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.RECORD_AUDIO), speechPermissionRequest)
            return
        }
        // 好友/群聊/频道：无论设置里选了哪种语音模式，一律以语音气泡发送
        if (isFriendChatActive()) {
            if (startVoiceMessageRecording()) return
            return
        }
        when (VoiceInputModeSettings.get(activity)) {
            VoiceInputMode.LOCAL_AGENT_ASR -> {
                if (startAgentVoice()) return
                // Agent 桥接启动失败时退回系统 SpeechRecognizer
            }
            VoiceInputMode.CLOUD_REALTIME -> {
                if (startRealtimeVoice()) return
                // 云端模式下无 projectId 时也退回系统 SpeechRecognizer
            }
            VoiceInputMode.VOICE_MESSAGE -> {
                // 语音消息模式：录音并以语音气泡发送，不走 ASR 流程
                if (startVoiceMessageRecording()) return
                // 录音无法启动时（权限问题等）不决策
                return
            }
        }
        if (!SpeechRecognizer.isRecognitionAvailable(activity)) {
            Toast.makeText(activity, "当前设备不可用语音识别", Toast.LENGTH_SHORT).show()
            return
        }
        translationGeneration += 1
        isHoldActive = true
        isSpeechCanceled = false
        startSpeechSession(SpeechAttempt(preferLanguage = true, preferOffline = false))
    }

    fun stopSpeechToText() {
        if (stopAgentVoice()) return
        if (stopRealtimeVoice()) return
        if (voiceRecorder.isRecording) {
            // 好友/群聊/频道 或 VOICE_MESSAGE 模式：发送语音气泡
            if (isFriendChatActive() || VoiceInputModeSettings.get(activity) == VoiceInputMode.VOICE_MESSAGE) {
                stopVoiceMessageRecording()
            } else {
                stopDirectVoiceRecording()
            }
            return
        }
        isHoldActive = false
        if (!isListeningForSpeech) {
            speechSessionId += 1
            voiceHoldButton().text = "按住 说话"
            return
        }
        isListeningForSpeech = false
        voiceHoldButton().text = "识别中..."
        runCatching {
            speechRecognizer?.stopListening()
        }.onFailure { error ->
            DebugTraceStore.record("speech_stop_failed", mapOf("error" to error.message))
            voiceHoldButton().text = "按住 说话"
            resetSpeechRecognizer()
            showSpeechFailureToast("停止识别失败")
        }
    }

    fun cancelSpeechToText() {
        if (cancelAgentVoice()) return
        if (cancelRealtimeVoice()) return
        if (voiceRecorder.isRecording) {
            // 好友/群聊/频道 或 VOICE_MESSAGE 模式：取消语音气泡录制
            if (isFriendChatActive() || VoiceInputModeSettings.get(activity) == VoiceInputMode.VOICE_MESSAGE) {
                cancelVoiceMessageRecording()
            } else {
                cancelDirectVoiceRecording()
            }
            return
        }
        if (!isListeningForSpeech && speechRecognizer == null) return
        isHoldActive = false
        speechSessionId += 1
        translationGeneration += 1
        isSpeechCanceled = true
        isListeningForSpeech = false
        voiceHoldButton().text = "按住 说话"
        runCatching { speechRecognizer?.cancel() }
        resetSpeechRecognizer()
    }

    fun destroy() {
        speechSessionId += 1
        translationGeneration += 1
        isHoldActive = false
        voiceRecorder.cancel()
        resetSpeechRecognizer()
        isListeningForSpeech = false
        realtimeController?.shutdown()
        realtimeController = null
        agentBridge?.destroy()
        agentBridge = null
        agentVoiceActive = false
        voiceOverlay?.hide()
        voiceOverlay = null
        voiceMessageAsrRecognizer?.destroy()
        voiceMessageAsrRecognizer = null
        voiceMessageTranscription = null
        voiceMessagePartialText = null
    }

    // ─── 手指拖动的 zone 反馈（仅端上模式生效） ─────────────────────

    /** 手指屏幕坐标。仅在 agent 语音中生效，用于选择 AI回复 / 转文字 / 取消。 */
    fun onVoiceTouchMove(rawX: Float, rawY: Float) {
        voiceOverlay?.updateTouch(rawX, rawY)
    }

    // ─── 方案 A：端上 Agent 流式识别 → 文字 → 走 elon 正常发送链路 ─────────────

    /** 启动端上流式识别。成功接管返回 true。 */
    private fun startAgentVoice(): Boolean {
        val bridge = agentBridge ?: AgentVoiceBridge(activity).also { agentBridge = it }
        if (bridge.isRunning) return true
        isHoldActive = true
        isSpeechCanceled = false
        agentVoiceActive = true
        agentLastFinalText = ""
        agentLastPartialText = ""
        translationGeneration += 1
        val overlay = voiceOverlay?.takeIf { it.mode == VoiceRecordingOverlay.Mode.AGENT }
            ?: VoiceRecordingOverlay(activity, VoiceRecordingOverlay.Mode.AGENT).also { voiceOverlay = it }
        overlay.show()
        bridge.onStart = {
            voiceHoldButton().text = "正在听..."
        }
        bridge.onPartial = { text ->
            if (text.isNotBlank()) {
                agentLastPartialText = text
                voiceHoldButton().text = text.take(24)
                voiceOverlay?.updatePartial(text)
            }
        }
        bridge.onFinal = { text ->
            agentLastFinalText = text
            voiceOverlay?.updatePartial(text)
            // 不在这里自动处理 AI回复/转文字，等 ACTION_UP 手势决定跳转。
            // 如果用户已经松开，stopAgentVoice 会取走最终结果。
        }
        bridge.onEnd = {
            if (!agentVoiceActive) {
                voiceHoldButton().text = "按住 说话"
                agentBridge?.prewarm()  // 立刻重新预热，准备下次按键（消除 mibrain 冷启动延迟）
            }
        }
        bridge.onError = { msg ->
            agentVoiceActive = false
            voiceOverlay?.hide()
            voiceHoldButton().text = "按住 说话"
            if (!isSpeechCanceled) {
                Toast.makeText(activity, "语音识别失败：${msg.take(60)}", Toast.LENGTH_SHORT).show()
            }
        }
        voiceHoldButton().text = "准备识别..."
        bridge.start()
        return true
    }

    /**
     * 松开按钮：根据遵照 [voiceOverlay] 当前区域决定动作。
     *  - AI_REPLY: 直接把转写文字交给 AI 回复
     *  - TRANSCRIBE: 回填输入框供用户查看/编辑
     *  - CANCEL: 丢弃
     */
    private fun stopAgentVoice(): Boolean {
        val bridge = agentBridge ?: return false
        if (!agentVoiceActive && !bridge.isRunning) return false
        val zone = voiceOverlay?.currentZone ?: VoiceRecordingOverlay.Zone.AI_REPLY
        isHoldActive = false
        if (zone == VoiceRecordingOverlay.Zone.CANCEL) {
            isSpeechCanceled = true
            agentVoiceActive = false
            bridge.cancel()
            voiceOverlay?.hide()
            voiceHoldButton().text = "按住 说话"
            return true
        }
        // 让 ASR 赶紧出最终结果。bridge.onFinal 可能同步或异步回调。
        bridge.stop()
        val targetZone = zone
        // 如果 onFinal 已提前到达（agentLastFinalText 非空）则立即处理，
        // 否则等待一个上限 1500ms 的窗口后用 partial 兑现。
        val deadline = activity.window?.decorView
        deadline?.postDelayed({
            if (!agentVoiceActive) return@postDelayed
            // 只在已有文字时提早提交。
            // mibrain 云端 ASR 在 stopListening 后约 600ms 才返回结果，
            // 250ms 抢先提交会导致 finalText="" → "没听清"，真实结果被丢弃。
            // 无文字时什么都不做，等 onFinal 回调（~600ms）或 1500ms 安全网。
            if (agentLastFinalText.isNotBlank() || agentLastPartialText.isNotBlank()) {
                commitAgentVoiceFinal(targetZone)
            }
        }, 250L)
        deadline?.postDelayed({
            if (!agentVoiceActive) return@postDelayed
            // 安全网：其他路径仍未完成时兑现 partial
            commitAgentVoiceFinal(targetZone)
        }, 1500L)
        // 同步绑定 onFinal，立即拿到结果就立即提交，跳过安全网调度
        bridge.onFinal = { text ->
            agentLastFinalText = text
            voiceOverlay?.updatePartial(text)
            if (agentVoiceActive) commitAgentVoiceFinal(targetZone)
        }
        return true
    }

    private fun commitAgentVoiceFinal(zone: VoiceRecordingOverlay.Zone) {
        if (!agentVoiceActive) return
        agentVoiceActive = false
        val finalText = agentLastFinalText.ifBlank { agentLastPartialText }.trim()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        // 会话结束时立即预热，确保下次按键准备就绪。
        // bridge.onEnd 在网络超时被忽略时不会触发，此处兜底保证预热不被遗漏。
        agentBridge?.prewarm()
        if (finalText.isBlank()) {
            Toast.makeText(activity, "没听清，请重试", Toast.LENGTH_SHORT).show()
            return
        }
        when (zone) {
            VoiceRecordingOverlay.Zone.AI_REPLY -> {
                // AI回复：走现有文字发送链路（后台只看到文字）
                val sender = sendTextDirect
                if (sender != null) {
                    setVoiceMode(false)
                    applyVoiceMode()
                    sender(finalText)
                } else {
                    handleRecognizedSpeech(finalText)
                }
            }
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> {
                // 转文字：只回填输入框，交给用户继续编辑或手动发送。
                setVoiceMode(false)
                applyVoiceMode()
                setInputText(finalText)
            }
            VoiceRecordingOverlay.Zone.CANCEL -> Unit
            VoiceRecordingOverlay.Zone.SEND -> Unit  // Agent 模式不会出现，忽略
        }
    }

    /** 取消：立即关闭。 */
    private fun cancelAgentVoice(): Boolean {
        val bridge = agentBridge ?: return false
        if (!agentVoiceActive && !bridge.isRunning) return false
        isHoldActive = false
        isSpeechCanceled = true
        agentVoiceActive = false
        bridge.cancel()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        return true
    }

    // ─── 方案 B：实时语音 → OpenAI 转写 → 投递 AI ────────────────────────────

    /** 按下按钮时尝试启动 Realtime 语音。有 projectId 时返回 true 并接管流程。 */
    private fun startRealtimeVoice(): Boolean {
        val project = activeProject()
        val projectId = project.id.takeIf { it.isNotBlank() } ?: return false
        val conversation = activeConversation()

        isHoldActive = true
        isSpeechCanceled = false
        voiceHoldButton().text = "连接中..."

        val ctrl = RealtimeVoiceController(
            context = activity,
            baseHttpUrl = serverUrl,
            userId = userId(),
            mode = RealtimeVoiceWsClient.Mode.Transcribe,
            projectId = projectId,
            conversationId = conversation.id,
            onTranscriptDelta = { text ->
                activity.runOnUiThread { voiceHoldButton().text = text.take(24) }
            },
            onTranscriptFinal = { text ->
                activity.runOnUiThread { voiceHoldButton().text = "识别：${text.take(20)}" }
            },
            onCliDispatched = { ok, _ ->
                activity.runOnUiThread {
                    voiceHoldButton().text = if (ok) "AI 处理中…" else "按住 说话"
                }
            },
            onAiProgress = { text ->
                activity.runOnUiThread { voiceHoldButton().text = "AI: ${text.take(22)}" }
            },
            onAiDone = { message, _ ->
                activity.runOnUiThread {
                    voiceHoldButton().text = "按住 说话"
                    Toast.makeText(activity, message.take(80), Toast.LENGTH_SHORT).show()
                }
                realtimeController = null
            },
            onAiError = { msg ->
                activity.runOnUiThread {
                    voiceHoldButton().text = "按住 说话"
                    Toast.makeText(activity, "AI 出错：${msg.take(60)}", Toast.LENGTH_SHORT).show()
                }
                realtimeController = null
            },
            onError = { msg ->
                activity.runOnUiThread {
                    voiceHoldButton().text = "按住 说话"
                    Toast.makeText(activity, "语音失败：${msg.take(60)}", Toast.LENGTH_SHORT).show()
                }
                realtimeController = null
            },
        )
        realtimeController = ctrl
        ctrl.start(activity.lifecycleScope)
        return true
    }

    /** 松开按钮：发 commit，等待转写完成。若不在 Realtime 模式返回 false。 */
    private fun stopRealtimeVoice(): Boolean {
        val ctrl = realtimeController ?: return false
        ctrl.commitUtterance()
        isHoldActive = false
        voiceHoldButton().text = "识别中…"
        return true
    }

    /** 取消：立即关闭 Realtime 会话。若不在 Realtime 模式返回 false。 */
    private fun cancelRealtimeVoice(): Boolean {
        val ctrl = realtimeController ?: return false
        ctrl.shutdown()
        realtimeController = null
        isHoldActive = false
        isSpeechCanceled = true
        voiceHoldButton().text = "按住 说话"
        return true
    }

    // ──────────────────────────────────────────────────────────────────────────

    /** 语音消息模式：开始录音。成功返回 true。同时启动 ASR 采集原文（并行录音+识别）。 */
    private fun startVoiceMessageRecording(): Boolean {
        translationGeneration += 1
        isHoldActive = true
        isSpeechCanceled = false
        val started = voiceRecorder.start()
        if (!started) {
            Toast.makeText(activity, "麦克风启动失败，请重试", Toast.LENGTH_SHORT).show()
            DebugTraceStore.record("voice_message_record_start_failed", emptyMap())
            return false
        }
        // 并行启动 ASR：MediaRecorder 录音文件 + SpeechRecognizer 识别原文
        voiceMessageTranscription = null
        voiceMessagePartialText = null
        // 好友/群聊使用 4 区域模式：发送/取消/转文字/@AI回复
        val overlayMode = if (isFriendChatActive()) VoiceRecordingOverlay.Mode.FRIEND_CHAT else VoiceRecordingOverlay.Mode.AGENT
        val overlay = voiceOverlay?.takeIf { it.mode == overlayMode }
            ?: VoiceRecordingOverlay(activity, overlayMode).also { voiceOverlay = it }
        overlay.show()
        runCatching {
            voiceMessageAsrRecognizer?.destroy()
            voiceMessageAsrRecognizer = SpeechRecognizer.createSpeechRecognizer(activity).apply {
                setRecognitionListener(createVoiceMessageAsrListener())
            }
            voiceMessageAsrRecognizer?.startListening(
                recognizerIntent(SpeechAttempt(preferLanguage = true, preferOffline = true))
            )
        }.onFailure {
            DebugTraceStore.record("voice_message_asr_start_failed", mapOf("error" to it.message))
            voiceMessageAsrRecognizer?.destroy()
            voiceMessageAsrRecognizer = null
        }
        DebugTraceStore.record("voice_message_record_start", emptyMap())
        voiceHoldButton().text = if (isFriendChatActive()) "松开 发送" else "松开 AI回复"
        return true
    }

    private fun createVoiceMessageAsrListener(): RecognitionListener = object : RecognitionListener {
        override fun onReadyForSpeech(params: Bundle?) = Unit
        override fun onBeginningOfSpeech() = Unit
        override fun onRmsChanged(rmsdB: Float) = Unit
        override fun onBufferReceived(buffer: ByteArray?) = Unit
        override fun onEndOfSpeech() = Unit
        override fun onError(error: Int) {
            DebugTraceStore.record("voice_message_asr_error", mapOf("code" to error))
            voiceMessageAsrRecognizer?.destroy()
            voiceMessageAsrRecognizer = null
        }
        override fun onResults(results: Bundle?) {
            val text = results
                ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                ?.firstOrNull()?.trim()
                ?.takeIf { it.isNotBlank() }
            voiceMessageTranscription = text
            text?.let { voiceOverlay?.updatePartial(it) }
            DebugTraceStore.record("voice_message_asr_result", mapOf("text" to (text ?: "")))
            voiceMessageAsrRecognizer?.destroy()
            voiceMessageAsrRecognizer = null
        }
        override fun onPartialResults(partialResults: Bundle?) {
            val text = partialResults
                ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                ?.firstOrNull()?.trim()
                ?.takeIf { it.isNotBlank() }
            voiceMessagePartialText = text
            text?.let { voiceOverlay?.updatePartial(it) }
        }
        override fun onEvent(eventType: Int, params: Bundle?) = Unit
    }

    /** 语音消息模式：录音结束，将音频以语音气泡发送（携带 ASR 原文）。 */
    private fun stopVoiceMessageRecording() {
        val zone = voiceOverlay?.currentZone
            ?: if (isFriendChatActive()) VoiceRecordingOverlay.Zone.SEND else VoiceRecordingOverlay.Zone.AI_REPLY
        if (zone == VoiceRecordingOverlay.Zone.CANCEL) {
            cancelVoiceMessageRecording()
            return
        }
        isHoldActive = false
        // 停止 ASR 采集，等候最终结果已由 onResults/onError 回调写入 voiceMessageTranscription
        runCatching { voiceMessageAsrRecognizer?.stopListening() }
        voiceHoldButton().text = when (zone) {
            VoiceRecordingOverlay.Zone.AI_REPLY -> "AI回复中..."
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> "转文字中..."
            VoiceRecordingOverlay.Zone.CANCEL -> "按住 说话"
            VoiceRecordingOverlay.Zone.SEND -> "发送中..."
        }
        val attachment = voiceRecorder.stopToAttachment()
        if (attachment == null) {
            cleanupVoiceMessageRecognition()
            voiceOverlay?.hide()
            voiceHoldButton().text = "按住 说话"
            DebugTraceStore.record("voice_message_record_empty", emptyMap())
            Toast.makeText(activity, "语音太短，请轻触再试", Toast.LENGTH_SHORT).show()
            return
        }
        val finishGeneration = ++translationGeneration
        val posted = activity.window?.decorView?.postDelayed({
            if (finishGeneration != translationGeneration) return@postDelayed
            finishVoiceMessageRecording(zone, attachment)
        }, VOICE_MESSAGE_ASR_SETTLE_MS) ?: false
        if (!posted) finishVoiceMessageRecording(zone, attachment)
    }

    private fun finishVoiceMessageRecording(
        zone: VoiceRecordingOverlay.Zone,
        attachment: PendingAttachment
    ) {
        val transcription = currentVoiceMessageText()
        cleanupVoiceMessageRecognition()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        DebugTraceStore.record(
            "voice_message_record_done",
            mapOf(
                "bytes" to attachment.file.length(),
                "duration_sec" to (attachment.durationSeconds ?: 0),
                "has_transcription" to (transcription != null),
                "zone" to zone.name
            )
        )
        when (zone) {
            VoiceRecordingOverlay.Zone.AI_REPLY -> {
                val sender = sendTextDirect
                if (sender != null && transcription != null) {
                    discardVoiceAttachment(attachment)
                    setVoiceMode(false)
                    applyVoiceMode()
                    sender(voiceAiReplyText(transcription))
                    return
                }
                // ASR 没拿到文字时，保留老逻辑：仍然把语音气泡发出去。
                val messageText = if (!isFriendChatActive() && transcription != null) transcription else ""
                sendVoiceAttachment(attachment.copy(transcription = transcription), messageText)
            }
            VoiceRecordingOverlay.Zone.TRANSCRIBE -> {
                discardVoiceAttachment(attachment)
                if (transcription == null) {
                    Toast.makeText(activity, "没听清，请重试", Toast.LENGTH_SHORT).show()
                    return
                }
                setVoiceMode(false)
                applyVoiceMode()
                setInputText(transcription)
            }
            VoiceRecordingOverlay.Zone.SEND -> {
                // 直接发送语音气泡给对方，不经 AI
                val messageText = if (transcription != null) transcription else ""
                sendVoiceAttachment(attachment.copy(transcription = transcription), messageText)
            }
            VoiceRecordingOverlay.Zone.CANCEL -> discardVoiceAttachment(attachment)
        }
    }

    /** 语音消息模式：取消录音。 */
    private fun cancelVoiceMessageRecording() {
        isHoldActive = false
        translationGeneration += 1
        isSpeechCanceled = true
        runCatching { voiceMessageAsrRecognizer?.cancel() }
        cleanupVoiceMessageRecognition()
        voiceRecorder.cancel()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        DebugTraceStore.record("voice_message_record_canceled", emptyMap())
    }

    private fun currentVoiceMessageText(): String? =
        voiceMessageTranscription?.trim()?.takeIf { it.isNotBlank() }
            ?: voiceMessagePartialText?.trim()?.takeIf { it.isNotBlank() }

    private fun cleanupVoiceMessageRecognition() {
        voiceMessageAsrRecognizer?.destroy()
        voiceMessageAsrRecognizer = null
        voiceMessageTranscription = null
        voiceMessagePartialText = null
    }

    private fun discardVoiceAttachment(attachment: PendingAttachment) {
        runCatching { attachment.file.delete() }
    }

    private fun voiceAiReplyText(text: String): String {
        val trimmed = text.trim()
        return if (isSocialAiChatActive() && !containsElMention(trimmed)) "@EL $trimmed" else trimmed
    }

    private fun containsElMention(text: String): Boolean =
        text.replace('＠', '@').contains("@EL", ignoreCase = true)

    /**
     * 长按语音气泡时展示原文（录音时 ASR 采集的识别结果）和翻译选项。
     * 由 Activity 通过 [ChatAdapter.onVoiceAttachmentLongPress] 调用。
     */
    fun showVoiceAttachmentActions(message: ChatMessage, attachment: ChatAttachment) {
        val transcription = attachment.transcription?.trim()?.takeIf { it.isNotBlank() }
        if (transcription != null) {
            AlertDialog.Builder(activity)
                .setTitle("原文")
                .setMessage(transcription)
                .setPositiveButton("关闭", null)
                .setNeutralButton("翻译") { _, _ ->
                    sendTextDirect?.invoke("请把以下内容翻译成中文（如果已是中文则翻译成英文）：\n\n$transcription")
                }
                .show()
        } else {
            AlertDialog.Builder(activity)
                .setTitle("语音消息")
                .setMessage("暂无原文（录音时语音识别未成功，可切换到[云端直连]模式后重试）")
                .setPositiveButton("关闭", null)
                .show()
        }
    }

    private fun startDirectVoiceRecording(): Boolean {
        translationGeneration += 1
        isHoldActive = true
        isSpeechCanceled = false
        val started = voiceRecorder.start()
        if (!started) {
            DebugTraceStore.record("voice_direct_record_start_failed", emptyMap())
            return false
        }
        DebugTraceStore.record("voice_direct_record_start", emptyMap())
        voiceHoldButton().text = "松开 发送语音"
        return true
    }

    private fun stopDirectVoiceRecording() {
        isHoldActive = false
        voiceHoldButton().text = "上传语音..."
        val attachment = voiceRecorder.stopToAttachment()
        voiceHoldButton().text = "按住 说话"
        if (attachment == null) {
            DebugTraceStore.record("voice_direct_record_empty", emptyMap())
            Toast.makeText(activity, "语音太短或录音失败，请重试", Toast.LENGTH_SHORT).show()
            return
        }
        DebugTraceStore.record(
            "voice_direct_record_done",
            mapOf("bytes" to attachment.file.length(), "mime_type" to attachment.mimeType)
        )
        sendVoiceAttachment(attachment, DIRECT_VOICE_MESSAGE)
    }

    private fun cancelDirectVoiceRecording() {
        isHoldActive = false
        translationGeneration += 1
        isSpeechCanceled = true
        voiceRecorder.cancel()
        voiceHoldButton().text = "按住 说话"
        DebugTraceStore.record("voice_direct_record_canceled", emptyMap())
    }

    private fun startSpeechSession(attempt: SpeechAttempt) {
        val sessionId = ++speechSessionId
        resetSpeechRecognizer()
        isListeningForSpeech = true
        voiceHoldButton().text = if (attempt.preferOffline) "离线识别中..." else "松开 转文字"
        speechRecognizer = SpeechRecognizer.createSpeechRecognizer(activity).apply {
            setRecognitionListener(createSpeechRecognitionListener(sessionId, attempt))
        }
        runCatching {
            speechRecognizer?.startListening(recognizerIntent(attempt))
        }.onFailure { error ->
            if (sessionId != speechSessionId) return
            DebugTraceStore.record(
                "speech_start_failed",
                mapOf(
                    "error" to error.message,
                    "prefer_language" to attempt.preferLanguage,
                    "prefer_offline" to attempt.preferOffline
                )
            )
            isListeningForSpeech = false
            resetSpeechRecognizer()
            val nextAttempt = nextSpeechAttempt(attempt, null)
            if (isHoldActive && nextAttempt != null) {
                retrySpeech(sessionId, nextAttempt, "start_exception")
            } else {
                voiceHoldButton().text = "按住 说话"
                showSpeechFailureToast("启动失败")
            }
        }
    }

    private fun createSpeechRecognitionListener(sessionId: Int, attempt: SpeechAttempt): RecognitionListener {
        return object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                if (!isCurrentSpeechSession(sessionId)) return
                voiceHoldButton().text = "正在听..."
            }
            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() {
                if (!isCurrentSpeechSession(sessionId)) return
                voiceHoldButton().text = "识别中..."
            }
            override fun onError(error: Int) {
                if (!isCurrentSpeechSession(sessionId)) return
                DebugTraceStore.record(
                    "speech_error",
                    mapOf(
                        "code" to error,
                        "message" to speechErrorMessage(error),
                        "prefer_language" to attempt.preferLanguage,
                        "prefer_offline" to attempt.preferOffline
                    )
                )
                isListeningForSpeech = false
                resetSpeechRecognizer()
                if (isSpeechCanceled) {
                    voiceHoldButton().text = "按住 说话"
                    return
                }
                val nextAttempt = nextSpeechAttempt(attempt, error)
                if (isHoldActive && nextAttempt != null) {
                    retrySpeech(sessionId, nextAttempt, "error_$error")
                    return
                }
                voiceHoldButton().text = "按住 说话"
                showSpeechFailureToast(speechErrorMessage(error), error)
            }
            override fun onResults(results: Bundle?) {
                if (!isCurrentSpeechSession(sessionId)) return
                isListeningForSpeech = false
                voiceHoldButton().text = "按住 说话"
                resetSpeechRecognizer()
                if (isSpeechCanceled) return
                val spoken = results
                    ?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                    ?.firstOrNull()
                    .orEmpty()
                    .trim()
                if (spoken.isNotBlank()) {
                    handleRecognizedSpeech(spoken)
                }
            }
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        }
    }

    private fun recognizerIntent(attempt: SpeechAttempt): Intent {
        return Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_CALLING_PACKAGE, activity.packageName)
            putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, attempt.preferOffline)
            if (attempt.preferLanguage) {
                putExtra(RecognizerIntent.EXTRA_LANGUAGE, SPEECH_LANGUAGE_TAG)
            }
        }
    }

    private fun handleRecognizedSpeech(spoken: String) {
        setVoiceMode(false)
        applyVoiceMode()
        setInputText(spoken)
        translateSpeechText(spoken)
    }

    private fun translateSpeechText(source: String) {
        val generation = ++translationGeneration
        Thread {
            val result = runCatching { requestSimplifiedChinese(source) }
            activity.runOnUiThread {
                if (generation != translationGeneration) return@runOnUiThread
                result.onSuccess { translated ->
                    val clean = translated.trim()
                    if (clean.isNotBlank() && binding.inputEdit.text.toString() == source) {
                        setInputText(clean)
                    }
                }.onFailure { error ->
                    DebugTraceStore.record("speech_translate_failed", mapOf("error" to error.message))
                    Toast.makeText(activity, "翻译暂不可用，已保留识别文字", Toast.LENGTH_SHORT).show()
                }
            }
        }.start()
    }

    private fun requestSimplifiedChinese(source: String): String {
        val payload = JSONObject().apply {
            put("text", source)
            selectedAgent()?.takeIf { it.isNotBlank() }?.let { put("agent_name", it) }
        }
        val body = payload.toString().toRequestBody("application/json; charset=utf-8".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/user/${urlPart(userId())}/speech/translate")
                .post(body)
        ).build()
        http.newCall(request).execute().use { response ->
            val responseBody = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })
            return JSONObject(responseBody).optString("text", source).ifBlank { source }
        }
    }

    private fun setInputText(text: String) {
        binding.inputEdit.setText(text)
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
    }

    private fun isCurrentSpeechSession(sessionId: Int): Boolean {
        return sessionId == speechSessionId
    }

    private fun nextSpeechAttempt(attempt: SpeechAttempt, error: Int?): SpeechAttempt? {
        if (error == null || isNetworkSpeechError(error)) {
            if (!attempt.preferOffline) {
                return attempt.copy(preferOffline = true)
            }
            if (attempt.preferLanguage) {
                return SpeechAttempt(preferLanguage = false, preferOffline = true)
            }
            return null
        }

        if (isLanguageOrServiceSpeechError(error) && attempt.preferLanguage) {
            return attempt.copy(preferLanguage = false)
        }

        if (isLanguageOrServiceSpeechError(error) && !attempt.preferOffline) {
            return attempt.copy(preferOffline = true)
        }

        return null
    }

    private fun retrySpeech(previousSessionId: Int, attempt: SpeechAttempt, reason: String) {
        DebugTraceStore.record(
            "speech_retry",
            mapOf(
                "reason" to reason,
                "prefer_language" to attempt.preferLanguage,
                "prefer_offline" to attempt.preferOffline
            )
        )
        voiceHoldButton().text = if (attempt.preferOffline) "切换离线识别..." else "正在重试..."
        binding.root.postDelayed({
            if (previousSessionId != speechSessionId || !isHoldActive || isSpeechCanceled) return@postDelayed
            startSpeechSession(attempt)
        }, SPEECH_RETRY_DELAY_MS)
    }

    private fun isNetworkSpeechError(error: Int): Boolean {
        return error == SpeechRecognizer.ERROR_NETWORK ||
            error == SpeechRecognizer.ERROR_NETWORK_TIMEOUT
    }

    private fun isLanguageOrServiceSpeechError(error: Int): Boolean {
        return error == SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED ||
            error == SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE ||
            error == SpeechRecognizer.ERROR_CANNOT_CHECK_SUPPORT ||
            error == SpeechRecognizer.ERROR_SERVER ||
            error == SpeechRecognizer.ERROR_SERVER_DISCONNECTED ||
            error == SpeechRecognizer.ERROR_CLIENT
    }

    private fun showSpeechFailureToast(message: String, error: Int? = null) {
        val suffix = error?.let { "（$it）" }.orEmpty()
        Toast.makeText(activity, "语音识别失败：$message$suffix", Toast.LENGTH_LONG).show()
    }

    private fun speechErrorMessage(error: Int): String {
        return when (error) {
            SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "网络超时"
            SpeechRecognizer.ERROR_NETWORK -> "网络不可用"
            SpeechRecognizer.ERROR_AUDIO -> "麦克风录音失败"
            SpeechRecognizer.ERROR_SERVER -> "系统语音服务异常"
            SpeechRecognizer.ERROR_CLIENT -> "识别服务客户端异常"
            SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "没有检测到语音"
            SpeechRecognizer.ERROR_NO_MATCH -> "没有听清"
            SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "识别服务正忙"
            SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "麦克风权限不足"
            SpeechRecognizer.ERROR_TOO_MANY_REQUESTS -> "请求过于频繁"
            SpeechRecognizer.ERROR_SERVER_DISCONNECTED -> "语音服务断开"
            SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED -> "系统不支持当前识别语言"
            SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE -> "当前识别语言不可用"
            SpeechRecognizer.ERROR_CANNOT_CHECK_SUPPORT -> "无法检查系统识别能力"
            SpeechRecognizer.ERROR_CANNOT_LISTEN_TO_DOWNLOAD_EVENTS -> "无法监听语言包下载"
            else -> "未知错误"
        }
    }

    private fun resetSpeechRecognizer() {
        speechRecognizer?.let { recognizer ->
            runCatching { recognizer.destroy() }
        }
        speechRecognizer = null
    }

    private companion object {
        // Android speech engines are much more consistent with zh-CN than zh-Hans-CN.
        private const val SPEECH_LANGUAGE_TAG = "zh-CN"
        private const val SPEECH_RETRY_DELAY_MS = 180L
        private const val VOICE_MESSAGE_ASR_SETTLE_MS = 900L
        private const val DIRECT_VOICE_MESSAGE =
            "我上传了一段原始语音，请优先根据语音附件理解我的需求。"

        // 语音消息模式：AI 可以对语音内容做出回应，但不强制执行命令
        private const val VOICE_MESSAGE_TEXT =
            "我发送了一条语音消息，请根据语音内容做出回应。"
    }

    private data class SpeechAttempt(
        val preferLanguage: Boolean,
        val preferOffline: Boolean
    )
}
