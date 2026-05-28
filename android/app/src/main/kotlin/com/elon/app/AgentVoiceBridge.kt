package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.speech.SpeechRecognizer
import android.util.Log
import com.elon.app.agent.infrastructure.voice.EngineHealth
import com.elon.app.agent.infrastructure.voice.EnginePreference
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
    /** 本次会话已切换/重试的总次数，防止旧 recognizer 残留事件触发死循环 */
    private var transitionSeq: Int = 0
    /** 同一引擎因 RECOGNIZER_BUSY 已经短延迟重试过几次（最多 2 次） */
    private var busyRetryOnSame: Int = 0
    /** 同一引擎因冷启动失败已经短延迟重试过几次（最多 1 次） */
    private var coldStartRetryOnSame: Int = 0
    /** 本次 startListening 的开始时间（毫秒），用于判定"冷启动瞬时失败" */
    private var listeningStartedAt: Long = 0L

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
                // 任何识别成功都记录当前引擎为 OK
                candidates.getOrNull(candidateIndex)?.let { eng ->
                    EnginePreference.setHealth(appContext, eng.key(), EngineHealth.OK)
                }
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
        candidates = RecognitionEngineSelector.listForUse(appContext)
        candidateIndex = 0
        transitionSeq += 1
        busyRetryOnSame = 0
        coldStartRetryOnSame = 0
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

    /**
     * 预热：提前创建 SpeechRecognizer 实例，让 mibrain/厂商 ASR 服务在用户按麦克风之前完成
     * 服务绑定。Android 文档：createSpeechRecognizer 是异步绑定，第一次 startListening 前
     * 若绑定未完成会返回 ERROR_SERVER_DISCONNECTED(11)。
     * 幂等，可多次调用；不会打断进行中的识别。
     */
    fun prewarm() {
        if (isRunning) return
        main.post {
            if (isRunning) return@post
            val engines = RecognitionEngineSelector.list(appContext)
            val first = engines.firstOrNull() ?: return@post
            if (asr.isInitialized && asr.engineComponent == first.component) return@post
            asr.engineComponent = first.component
            asr.initialize()   // 创建 SpeechRecognizer，开始服务绑定，不 startListening
            Log.i(TAG, "预热: 已为 ${first.label}(${first.packageName}) 启动服务绑定")
        }
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
        val engineChanged = asr.engineComponent != engine.component
        asr.engineComponent = engine.component
        if (engineChanged) {
            // 只在切换引擎时销毁重建；同一引擎保留实例（服务绑定已建立，无需重建）
            Log.i(TAG, "引擎切换，重置 recognizer")
            asr.resetEngine()
        }
        listeningStartedAt = System.currentTimeMillis()
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
        // 关键防护：若当前 isRunning=false（说明已经报错收尾或已切换引擎完成），
        // 直接忽略旧 recognizer 残留事件，避免 candidateIndex 被多冲一次导致越界。
        if (!isRunning) {
            Log.d(TAG, "忽略残留错误 code=$code（isRunning=false）")
            return
        }
        Log.w(TAG, "引擎错误 code=$code msg=$message engineIdx=$candidateIndex sawPartial=$sawAnyPartial busyRetry=$busyRetryOnSame coldRetry=$coldStartRetryOnSame")
        val current = candidates.getOrNull(candidateIndex)

        // RECOGNIZER_BUSY：直接重试 startListening（不销毁 recognizer），最多 2 次。
        // 注意：以前这里调用了 resetEngine()，会导致重建的 recognizer 触发新一轮冷启动，已修复。
        if (code == SpeechRecognizer.ERROR_RECOGNIZER_BUSY && current != null && busyRetryOnSame < 2 && !sawAnyPartial) {
            busyRetryOnSame += 1
            Log.i(TAG, "RECOGNIZER_BUSY，延迟 250ms 重试同引擎 ${current.label}（不销毁 recognizer）(#$busyRetryOnSame)")
            val mySeq = ++transitionSeq
            main.postDelayed({
                if (isRunning && mySeq == transitionSeq) {
                    listeningStartedAt = System.currentTimeMillis()
                    asr.startListening()   // 不 resetEngine：服务绑定已建立，直接复用
                }
            }, 250L)
            return
        }

        // 冷启动瞬时失败：startListening 后 200ms 内就报错（mibrain 服务绑定尚未完成）。
        // 不销毁 recognizer，直接重试 startListening，绑定通常在 50~100ms 内完成。
        // 允许重试 2 次，每次递增延迟。
        val sinceStart = System.currentTimeMillis() - listeningStartedAt
        val isColdStartGlitch = sinceStart in 0..200 && coldStartRetryOnSame < 2 && code == 11
        if (isColdStartGlitch && current != null && !sawAnyPartial) {
            coldStartRetryOnSame += 1
            val delay = if (coldStartRetryOnSame == 1) 100L else 300L
            Log.i(TAG, "冷启动失败(${sinceStart}ms<200ms, code=11)，${delay}ms后重试（不销毁 recognizer）#$coldStartRetryOnSame")
            val mySeq = ++transitionSeq
            main.postDelayed({
                if (isRunning && mySeq == transitionSeq) {
                    listeningStartedAt = System.currentTimeMillis()
                    asr.startListening()   // 不 resetEngine：等服务绑定完成后直接复用
                }
            }, delay)
            return
        }

        val retryable = isEngineFailure(code) && !sawAnyPartial && current != null
        if (!retryable) {
            isRunning = false
            // 当前引擎确认失败：记录健康状态供 UI 显示
            if (current != null) {
                EnginePreference.setHealth(appContext, current.key(), EngineHealth.FAILED, code, message)
            }
            val friendly = if (candidateIndex >= candidates.size - 1)
                "本机所有语音引擎暂时都不可用，请稍后再试或在设置切换为云端识别"
            else message
            main.post {
                onError(friendly)
                onEnd()
            }
            return
        }

        // 当前引擎放弃了，标记 FAILED，然后切下一个
        if (current != null) {
            EnginePreference.setHealth(appContext, current.key(), EngineHealth.FAILED, code, message)
        }
        candidateIndex += 1
        busyRetryOnSame = 0
        coldStartRetryOnSame = 0
        transitionSeq += 1
        val nextLabel = candidates.getOrNull(candidateIndex)?.label ?: "<无>"
        Log.i(TAG, "回退到下一个引擎: $nextLabel (上一个: ${current!!.label}, code=$code)")
        main.post { startWithCurrentCandidate() }
    }
}
