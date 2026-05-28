package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.speech.SpeechRecognizer
import android.util.Log
import com.elon.app.agent.infrastructure.voice.RecognitionEngine
import com.elon.app.agent.infrastructure.voice.RecognitionEngineSelector
import com.elon.app.agent.infrastructure.voice.StreamingASR
import com.elon.app.agent.infrastructure.voice.StreamingASRCallback

/**
 * elon 主聊天用语音桥接层。
 *
 * 把 Agent 子系统的 [StreamingASR]（端上 `SpeechRecognizer` + `SmartVAD`）封装为
 * 主聊天可直接调用的"按一下开始 → 系统判定结束 → 拿到文字"流程，并提供：
 *   - `onPartial`：UI 实时显示听到的内容
 *   - `onFinal`：最终文字，由调用方决定是填输入框还是直接发
 *   - `onError`：失败提示
 *
 * **引擎自动回退（仅本次会话内有效，不持久化）**：每次 [start] 都从
 * [RecognitionEngineSelector] 拿到按优先级排序的候选引擎列表（系统默认 +
 * 各品牌厂商 + Google）。任一引擎遇到 `ERROR_NETWORK` / `ERROR_CLIENT` /
 * `ERROR_SERVER` 等"引擎本身不可用"的错误且尚未拿到任何部分结果时，
 * 直接切换到下一个候选并重启识别。所有候选都失败后才把错误回调给上层。
 * 下次 [start]（包括下次按麦克风）会重新从首选开始尝试 —— 用户可能切换了
 * 网络或在系统设置里换了引擎，没必要永久封禁。
 *
 * 不依赖 Agent 的对话状态机 / TTS / 任务执行，只复用 ASR + VAD 这一层。
 */
internal class AgentVoiceBridge(context: Context) {

    companion object {
        private const val TAG = "AgentVoiceBridge"
    }

    private val appContext: Context = context.applicationContext
    private val main = Handler(Looper.getMainLooper())
    private val asr = StreamingASR(appContext).apply {
        useSmartVAD = true
        language = "zh-CN"
    }

    var onPartial: (String) -> Unit = {}
    var onFinal: (String) -> Unit = {}
    var onError: (String) -> Unit = {}
    var onStart: () -> Unit = {}
    var onEnd: () -> Unit = {}

    @Volatile
    var isRunning: Boolean = false
        private set

    /** 当前会话使用的候选引擎列表（按优先级） */
    private var candidates: List<RecognitionEngine> = emptyList()
    /** 当前正在用第几个候选 */
    private var candidateIndex: Int = 0
    /** 本次会话期间收到过至少一个非空部分结果（用于判断"引擎已经在工作") */
    private var sawAnyPartial: Boolean = false

    init {
        asr.callback = object : StreamingASRCallback {
            override fun onReady() = Unit
            override fun onSpeechStart() {
                main.post { onStart() }
            }
            override fun onPartialResult(text: String, confidence: Float) {
                if (text.isNotBlank()) sawAnyPartial = true
                main.post { onPartial(text) }
            }
            override fun onFinalResult(text: String) {
                isRunning = false
                val clean = text.trim()
                if (clean.isEmpty()) {
                    main.post { onEnd() }
                    return
                }
                main.post {
                    onFinal(clean)
                    onEnd()
                }
            }
            override fun onSpeechEnd() {
                main.post { onEnd() }
            }
            override fun onError(message: String) {
                // 真正决定是否回退的逻辑在 onErrorCode 里
            }
            override fun onErrorCode(code: Int, message: String) {
                handleEngineError(code, message)
            }
        }
    }

    /** 启动流式识别（系统 VAD/SmartVAD 自行判定结束）。 */
    fun start() {
        if (isRunning) return
        isRunning = true
        sawAnyPartial = false
        candidates = RecognitionEngineSelector.list(appContext)
        candidateIndex = 0
        main.post { startWithCurrentCandidate() }
    }

    /** 用户主动结束（松手）。 */
    fun stop() {
        if (!isRunning) return
        isRunning = false
        main.post { asr.stopListening() }
    }

    /** 用户取消（不要文字）。 */
    fun cancel() {
        isRunning = false
        main.post { asr.cancel() }
    }

    /** 释放资源（Activity 销毁时调用）。 */
    fun destroy() {
        isRunning = false
        main.post { asr.destroy() }
    }

    // ─────────────────────── 内部 ───────────────────────

    private fun startWithCurrentCandidate() {
        val engine = candidates.getOrNull(candidateIndex)
        if (engine == null) {
            isRunning = false
            main.post {
                onError("没有可用的语音识别引擎，请在设置里切换为云端识别")
                onEnd()
            }
            return
        }
        Log.i(TAG, "尝试引擎[$candidateIndex/${candidates.size}]: ${engine.label}(${engine.packageName})")
        asr.engineComponent = engine.component
        asr.resetEngine()
        asr.startListening()
    }

    /**
     * 是否属于"引擎本身不可用，应该切换下一个"的错误码。
     * 反向白名单：以下错误码切引擎也没用，其它一律当引擎故障重试。
     *   - NO_MATCH(7) / SPEECH_TIMEOUT(6)：用户没说话或没说清
     *   - INSUFFICIENT_PERMISSIONS(9)：权限问题
     *   - AUDIO(3)：录音硬件问题
     * 已知会被覆盖的引擎故障码：
     *   2=NETWORK, 4=SERVER, 5=CLIENT, 8=RECOGNIZER_BUSY,
     *   11=SERVER_DISCONNECTED, 12=LANGUAGE_NOT_SUPPORTED,
     *   13=LANGUAGE_UNAVAILABLE, 14=CANNOT_CHECK_SUPPORT, ...
     */
    private fun isEngineFailure(code: Int): Boolean = when (code) {
        SpeechRecognizer.ERROR_NO_MATCH,
        SpeechRecognizer.ERROR_SPEECH_TIMEOUT,
        SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS,
        SpeechRecognizer.ERROR_AUDIO -> false
        else -> true
    }

    private fun handleEngineError(code: Int, message: String) {
        Log.w(TAG, "引擎错误 code=$code msg=$message engineIdx=$candidateIndex sawPartial=$sawAnyPartial")
        val current = candidates.getOrNull(candidateIndex)

        val retryable = isEngineFailure(code) && !sawAnyPartial && current != null
        if (!retryable) {
            isRunning = false
            main.post {
                onError(message)
                onEnd()
            }
            return
        }

        candidateIndex += 1
        val nextLabel = candidates.getOrNull(candidateIndex)?.label ?: "<无>"
        Log.i(TAG, "回退到下一个引擎: $nextLabel (上一个: ${current!!.label}, code=$code)")
        main.post { startWithCurrentCandidate() }
    }
}
