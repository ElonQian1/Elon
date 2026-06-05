package com.elon.app

import android.Manifest
import android.app.AlertDialog
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.io.File
import java.util.Locale

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
    private val isDirectSocialAiChatActive: () -> Boolean = { false },
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
    // 语音消息模式：并行 ASR 采集原文（使用 AgentVoiceBridge 多引擎轮换）
    private var voiceMessageBridge: AgentVoiceBridge? = null
    private var voiceMessageTranscription: String? = null
    private var voiceMessagePartialText: String? = null
    /** 所有本地 ASR 引擎都失败时为 true，触发服务器 Whisper fallback */
    private var voiceMessageAsrAllFailed = false
    // 方案 B：实时语音 → 转写 → AI 投递
    private var realtimeController: RealtimeVoiceController? = null
    private var realtimeVoiceSpeaker: VoiceSpeaker? = null
    // 端上 Agent 流式识别（默认主聊天语音管线，由 VoiceInputModeSettings 控制）
    private var agentBridge: AgentVoiceBridge? = null
    private var agentVoiceActive = false
    private var agentLastFinalText: String = ""
    private var agentLastPartialText: String = ""
    // 仿微信全屏麦克风遮罩（在端上模式下启用）
    private var voiceOverlay: VoiceRecordingOverlay? = null
    // 噪音检测——连续高音量且无识别结果时提示
    private var highVolumeStartMs: Long = 0L
    private var noiseWarningShown: Boolean = false

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
        // 好友/群聊/频道：无论设置里选了哪种语音模式，一律以语音气泡发送。
        // 一龙AI 私聊的实时语音通话由顶部电话按钮进入，底部恢复为按住说话。
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
        realtimeVoiceSpeaker?.release()
        realtimeVoiceSpeaker = null
        agentBridge?.destroy()
        agentBridge = null
        agentVoiceActive = false
        voiceOverlay?.hide()
        voiceOverlay = null
        voiceMessageBridge?.destroy()
        voiceMessageBridge = null
        voiceMessageTranscription = null
        voiceMessagePartialText = null
        voiceMessageAsrAllFailed = false
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
        bridge.onReady = {
            // 引擎绑定完成，开始收音
            voiceHoldButton().text = "正在听..."
            voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.LISTENING)
            voiceOverlay?.startCountdown(MAX_RECOGNITION_MS)
        }
        bridge.onStart = {
            // VAD 检测到声音开始：震动反馈确认
            vibrateOnce(30L)
            voiceHoldButton().text = "正在听..."
            voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.LISTENING)
        }
        bridge.onVolume = { v ->
            voiceOverlay?.setVolume(v)
            // 噪音检测：连续高音量(>0.70)且 2.5s 内无任何识别内容
            if (v > 0.70f) {
                if (highVolumeStartMs == 0L) highVolumeStartMs = System.currentTimeMillis()
                else if (!noiseWarningShown
                    && agentLastPartialText.isBlank()
                    && System.currentTimeMillis() - highVolumeStartMs > 2500L
                ) {
                    noiseWarningShown = true
                    voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.NOISE)
                }
            } else {
                highVolumeStartMs = 0L
            }
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
            } else if (agentLastFinalText.isNotBlank() || agentLastPartialText.isNotBlank()) {
                // SmartVAD 判断说完，已有识别内容，等用户松手
                voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.HEARD)
            } else {
                // VAD 静音超时但无任何内容，提示用户没听到
                voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.SILENCE)
            }
        }
        bridge.onError = { msg ->
            agentVoiceActive = false
            voiceOverlay?.stopCountdown()
            voiceOverlay?.hide()
            // SpeechRecognizer 所有引擎失败时（如 Honor MagicVoice 常驻占用 session），
            // 静默 fallback 到云端语音（AudioRecord PCM，不调 RecognitionService）。
            // 用户仍在按住按钮时无感知切换，松手后走 stopRealtimeVoice 正常提交。
            val cloudFallback = !isSpeechCanceled && isHoldActive && startRealtimeVoice()
            if (!cloudFallback) {
                voiceHoldButton().text = "按住 说话"
                if (!isSpeechCanceled) {
                    Toast.makeText(activity, "语音识别失败：${msg.take(60)}", Toast.LENGTH_SHORT).show()
                }
            }
        }
        voiceHoldButton().text = "准备中..."
        highVolumeStartMs = 0L
        noiseWarningShown = false
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
        // 松手后立即切到"识别中..."状态，让用户知道在等待引擎返回结果
        voiceOverlay?.setListeningState(VoiceRecordingOverlay.ListeningState.PROCESSING)
        val deadline = activity.window?.decorView
        deadline?.postDelayed({
            if (!agentVoiceActive) return@postDelayed
            // 只在 final 已到达时才提早提交。
            // partial 代表识别中间片段（可能只有一个字），用它提交会丢弃引擎
            // 随后返回的完整 final 结果（mibrain 等引擎约 600ms 后才返回 final）。
            if (agentLastFinalText.isNotBlank()) {
                commitAgentVoiceFinal(targetZone)
            }
        }, 250L)
        deadline?.postDelayed({
            if (!agentVoiceActive) return@postDelayed
            // 安全网（1500ms）：final 仍未到时用 partial 兜底
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
        voiceOverlay?.stopCountdown()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        return true
    }

    /** 短震动反馈（VAD 说话开始确认）。屏蔽异常，不影响主流程。 */
    private fun vibrateOnce(ms: Long) {
        runCatching {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val vm = activity.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager
                vm?.defaultVibrator?.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                val v = activity.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    v?.vibrate(VibrationEffect.createOneShot(ms, VibrationEffect.DEFAULT_AMPLITUDE))
                } else {
                    @Suppress("DEPRECATION")
                    v?.vibrate(ms)
                }
            }
        }
    }

    // ─── 方案 B：实时语音 → OpenAI 转写 → 投递 AI ────────────────────────────

    /** 按下按钮时尝试启动 Realtime 语音：项目页投递 CLI，一龙AI 私聊投递社交 AI。 */
    private fun startRealtimeVoice(): Boolean {
        val directSocialAi = isDirectSocialAiChatActive()
        val projectId: String?
        val conversationId: String?
        val target: String?
        if (directSocialAi) {
            projectId = null
            conversationId = null
            target = RealtimeVoiceWsClient.Target.SocialAiDirect
        } else {
            val project = activeProject()
            projectId = project.id.takeIf { it.isNotBlank() } ?: return false
            conversationId = activeConversation().id
            target = null
        }
        realtimeVoiceSpeaker?.stop()

        isHoldActive = true
        isSpeechCanceled = false
        voiceHoldButton().text = "连接中..."

        val ctrl = RealtimeVoiceController(
            context = activity,
            baseHttpUrl = serverUrl,
            userId = userId(),
            mode = RealtimeVoiceWsClient.Mode.Transcribe,
            target = target,
            projectId = projectId,
            conversationId = conversationId,
            onTranscriptDelta = { text ->
                activity.runOnUiThread { voiceHoldButton().text = text.take(24) }
            },
            onTranscriptFinal = { text ->
                activity.runOnUiThread { voiceHoldButton().text = "识别：${text.take(20)}" }
            },
            onCliDispatched = { ok, _ ->
                activity.runOnUiThread {
                    voiceHoldButton().text = if (ok) {
                        if (directSocialAi) "一龙AI 思考中…" else "AI 处理中…"
                    } else {
                        "按住 说话"
                    }
                }
            },
            onAiProgress = { text ->
                activity.runOnUiThread { voiceHoldButton().text = "AI: ${text.take(22)}" }
            },
            onAiDone = { message, _ ->
                activity.runOnUiThread {
                    voiceHoldButton().text = "按住 说话"
                    if (directSocialAi) {
                        realtimeSpeaker().speak(message)
                    } else {
                        Toast.makeText(activity, message.take(80), Toast.LENGTH_SHORT).show()
                        realtimeSpeaker().speak(message)
                    }
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

    private fun realtimeSpeaker(): VoiceSpeaker {
        return realtimeVoiceSpeaker ?: VoiceSpeaker(activity).also {
            realtimeVoiceSpeaker = it
        }
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
        // 乐观并行：同时启动 MediaRecorder 录音文件 + AgentVoiceBridge 实时 ASR。
        // 小米等设备两者可共存，录音期间有实时转写文字显示。
        // Honor/华为部分机型 HAL 更容易出现麦克风独占，双录音会额外放大发热。
        // 这些机型直接跳过并行本地 ASR，统一走录音完成后的服务器 Whisper 兜底。
        voiceMessageTranscription = null
        voiceMessagePartialText = null
        voiceMessageAsrAllFailed = false
        // 好友/群聊使用 4 区域模式：发送/取消/转文字/@AI回复
        val overlayMode = if (isFriendChatActive()) VoiceRecordingOverlay.Mode.FRIEND_CHAT else VoiceRecordingOverlay.Mode.AGENT
        val overlay = voiceOverlay?.takeIf { it.mode == overlayMode }
            ?: VoiceRecordingOverlay(activity, overlayMode).also { voiceOverlay = it }
        overlay.show()
        if (shouldRunParallelVoiceMessageAsr()) {
            // 启动 AgentVoiceBridge 多引擎并行 ASR（支持自动引擎切换）
            runCatching {
                voiceMessageBridge?.destroy()
                voiceMessageBridge = AgentVoiceBridge(activity).also { bridge ->
                    bridge.onPartial = { text ->
                        voiceMessagePartialText = text
                        voiceOverlay?.updatePartial(text)
                    }
                    bridge.onFinal = { text ->
                        if (text.isNotBlank()) voiceMessageTranscription = text
                        voiceOverlay?.updatePartial(text)
                        DebugTraceStore.record("voice_message_asr_result", mapOf("text" to text))
                    }
                    bridge.onError = { msg ->
                        // 所有本地引擎失败（Honor 抢麦冲突等）；finishVoiceMessageRecording
                        // 通过 transcription==null 判断需要服务器兜底，此标志仅供调试日志
                        voiceMessageAsrAllFailed = true
                        DebugTraceStore.record("voice_message_asr_all_failed", mapOf("msg" to msg))
                    }
                    bridge.start()
                }
            }.onFailure {
                DebugTraceStore.record("voice_message_asr_start_failed", mapOf("error" to it.message))
                voiceMessageBridge?.destroy()
                voiceMessageBridge = null
            }
        } else {
            DebugTraceStore.record(
                "voice_message_asr_skipped_for_device",
                mapOf("manufacturer" to Build.MANUFACTURER, "brand" to Build.BRAND)
            )
            voiceMessageBridge?.destroy()
            voiceMessageBridge = null
        }
        DebugTraceStore.record("voice_message_record_start", emptyMap())
        // 好友/群聊/频道模式已有浮层提示；清空底层按钮文字，避免透过半透明托盘重复显示。
        voiceHoldButton().text = if (isFriendChatActive()) "" else "松开 AI回复"
        return true
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
        // 停止 ASR 采集，最终结果已由 bridge 回调写入 voiceMessageTranscription
        runCatching { voiceMessageBridge?.stop() }
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
        // 不依赖 asrAllFailed（Honor 等设备上 onError 在松手后才异步到达，届时已为 false）。
        // 改为：只要没拿到文字就走服务器兜底——小米/华为通吃。
        val serverFallbackNeeded = transcription == null
        cleanupVoiceMessageRecognition()
        voiceOverlay?.hide()
        voiceHoldButton().text = "按住 说话"
        DebugTraceStore.record(
            "voice_message_record_done",
            mapOf(
                "bytes" to attachment.file.length(),
                "duration_sec" to (attachment.durationSeconds ?: 0),
                "has_transcription" to (transcription != null),
                "server_fallback" to serverFallbackNeeded,
                "zone" to zone.name
            )
        )
        // SEND / CANCEL 不需要转写文字，直接处理
        if (zone == VoiceRecordingOverlay.Zone.SEND) {
            sendVoiceAttachment(attachment.copy(transcription = transcription), "")
            return
        }
        if (zone == VoiceRecordingOverlay.Zone.CANCEL) {
            discardVoiceAttachment(attachment)
            return
        }
        // AI_REPLY / TRANSCRIBE：本地 ASR 全失败时尝试服务器 Whisper（若用户未关闭云端兜底）
        if (serverFallbackNeeded) {
            if (AsrFallbackSettings.isServerFallbackEnabled(activity)) {
                voiceHoldButton().text = "识别中..."
                uploadAudioForTranscription(attachment.file) { text ->
                    activity.runOnUiThread {
                        voiceHoldButton().text = "按住 说话"
                        applyVoiceZoneAction(zone, attachment, text)
                    }
                }
            } else {
                // 用户关闭了云端兜底：直接用空转写走后续逻辑（语音消息仍可发送，只是没文字）
                applyVoiceZoneAction(zone, attachment, null)
            }
            return
        }
        applyVoiceZoneAction(zone, attachment, transcription)
    }

    private fun shouldRunParallelVoiceMessageAsr(): Boolean {
        val manufacturer = Build.MANUFACTURER.orEmpty().lowercase(Locale.ROOT)
        val brand = Build.BRAND.orEmpty().lowercase(Locale.ROOT)
        val model = Build.MODEL.orEmpty().lowercase(Locale.ROOT)
        val isHonorOrHuawei =
            manufacturer.contains("honor") || manufacturer.contains("huawei") ||
                brand.contains("honor") || brand.contains("huawei") ||
                model.contains("honor") || model.contains("huawei")
        return !isHonorOrHuawei
    }

    /** 根据区域决定最终行为（在获得转写文字后调用）。 */
    private fun applyVoiceZoneAction(
        zone: VoiceRecordingOverlay.Zone,
        attachment: PendingAttachment,
        transcription: String?
    ) {
        when (zone) {
            VoiceRecordingOverlay.Zone.AI_REPLY -> {
                val sender = sendTextDirect
                if (sender != null && transcription != null) {
                    discardVoiceAttachment(attachment)
                    setVoiceMode(false)
                    applyVoiceMode()
                    showVoiceAiPreviewAndSend(transcription, sender)
                    return
                }
                // 还是没拿到文字：把语音气泡发出去
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
                sendVoiceAttachment(attachment.copy(transcription = transcription), "")
            }
            VoiceRecordingOverlay.Zone.CANCEL -> discardVoiceAttachment(attachment)
        }
    }

    /**
     * 将录音文件上传到服务器 `/api/voice/asr`，回调返回识别文字（失败时为 null）。
     * 在后台线程执行 HTTP，结果通过 callback 传回（调用方负责切换到主线程）。
     */
    private fun uploadAudioForTranscription(file: File, callback: (String?) -> Unit) {
        activity.lifecycleScope.launch(Dispatchers.IO) {
            runCatching {
                val bytes = file.readBytes()
                val ext = file.extension.lowercase().let { if (it.isEmpty()) "m4a" else it }
                val mime = when (ext) {
                    "wav" -> "audio/wav"
                    "mp3" -> "audio/mpeg"
                    "ogg", "oga" -> "audio/ogg"
                    "webm" -> "audio/webm"
                    "aac" -> "audio/aac"
                    else -> "audio/m4a"
                }
                val requestBody = okhttp3.MultipartBody.Builder()
                    .setType(okhttp3.MultipartBody.FORM)
                    .addFormDataPart("audio", file.name,
                        bytes.toRequestBody(mime.toMediaType()))
                    .addFormDataPart("format", mime)
                    .addFormDataPart("language", AsrFallbackSettings.getWhisperLanguage(activity))
                    .addFormDataPart("beam_size", AsrFallbackSettings.getWhisperBeamSize(activity).toString())
                    .addFormDataPart("vad_filter", AsrFallbackSettings.getWhisperVadFilter(activity).toString())
                    .addFormDataPart("condition_on_previous_text", AsrFallbackSettings.getWhisperConditionOnPrevious(activity).toString())
                    .build()
                val authToken = AuthManager.token(activity) ?: ""
                val request = Request.Builder()
                    .url("$serverUrl/api/voice/asr")
                    .addHeader("Authorization", "Bearer $authToken")
                    .post(requestBody)
                    .build()
                http.newCall(request).execute().use { resp ->
                    val body = resp.body?.string() ?: ""
                    if (resp.isSuccessful) {
                        val text = JSONObject(body).optString("text", "").trim()
                        DebugTraceStore.record("voice_asr_server_result", mapOf("text" to text))
                        callback(text.ifBlank { null })
                    } else {
                        DebugTraceStore.record("voice_asr_server_error", mapOf("code" to resp.code, "body" to body))
                        callback(null)
                    }
                }
            }.onFailure { e ->
                DebugTraceStore.record("voice_asr_server_exception", mapOf("error" to e.message))
                callback(null)
            }
        }
    }

    /** 语音消息模式：取消录音。 */
    private fun cancelVoiceMessageRecording() {
        isHoldActive = false
        translationGeneration += 1
        isSpeechCanceled = true
        runCatching { voiceMessageBridge?.cancel() }
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
        voiceMessageBridge?.destroy()
        voiceMessageBridge = null
        voiceMessageTranscription = null
        voiceMessagePartialText = null
        voiceMessageAsrAllFailed = false
    }

    private fun discardVoiceAttachment(attachment: PendingAttachment) {
        runCatching { attachment.file.delete() }
    }

    /**
     * 语音 @AI 回复预览气泡：底部展示识别结果 2 秒后自动发送。
     * 用户点「取消」可将文字回填到输入框自行修改，不发送。
     */
    private fun showVoiceAiPreviewAndSend(transcription: String, sender: (String) -> Unit) {
        val handler = Handler(Looper.getMainLooper())
        val textToSend = voiceAiReplyText(transcription)

        // 用 Snackbar 做底部气泡
        val rootView = activity.window.decorView.findViewById<android.view.View>(android.R.id.content)
        val snackbar = com.google.android.material.snackbar.Snackbar.make(
            rootView,
            textToSend,
            com.google.android.material.snackbar.Snackbar.LENGTH_INDEFINITE
        )
        snackbar.setAction("取消") {
            handler.removeCallbacksAndMessages(null)
            // 取消：把识别文字填到输入框，不发送
            setInputText(transcription.trim())
        }
        snackbar.show()

        // 2 秒后自动发送
        handler.postDelayed({
            if (snackbar.isShown) {
                snackbar.dismiss()
                sender(textToSend)
            }
        }, VOICE_AI_PREVIEW_DELAY_MS)
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

        private const val VOICE_AI_PREVIEW_DELAY_MS = 2000L
        // 识别时长上限提示（20s——大多数引擎不超过此时间）
        private const val MAX_RECOGNITION_MS = 20_000L
    }

    private data class SpeechAttempt(
        val preferLanguage: Boolean,
        val preferOffline: Boolean
    )
}
