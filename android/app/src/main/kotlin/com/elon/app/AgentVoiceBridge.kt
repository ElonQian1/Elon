package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
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
 * 不依赖 Agent 的对话状态机 / TTS / 任务执行，只复用 ASR + VAD 这一层。
 */
internal class AgentVoiceBridge(context: Context) {

    private val main = Handler(Looper.getMainLooper())
    private val asr = StreamingASR(context.applicationContext).apply {
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

    init {
        asr.callback = object : StreamingASRCallback {
            override fun onReady() = Unit
            override fun onSpeechStart() {
                main.post { onStart() }
            }
            override fun onPartialResult(text: String, confidence: Float) {
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
                isRunning = false
                main.post {
                    onError(message)
                    onEnd()
                }
            }
        }
    }

    /** 启动流式识别（系统 VAD/SmartVAD 自行判定结束）。 */
    fun start() {
        if (isRunning) return
        isRunning = true
        // ASR.startListening 必须在主线程
        main.post { asr.startListening() }
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
}
