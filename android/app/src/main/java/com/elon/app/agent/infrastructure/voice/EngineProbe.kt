// infrastructure/voice/EngineProbe.kt
// module: infrastructure/voice | layer: infrastructure | role: probe
// summary: 静默测试一个 ASR 引擎是否能就绪：startListening 后 1.5s 内若收到 onReadyForSpeech 视为 OK，
// 收到 onError 视为 FAILED，超时视为 FAILED。
//
// 设计原则：
//  - 必须在主线程（Android Looper）回调 SpeechRecognizer，所以这里用 Handler+Looper 而不是协程
//  - 探测过程不向用户播放任何提示音、不抢占麦克风焦点（系统 RecognitionService 自己处理）
//  - 单次探测最多占用 1.5s

package com.elon.app.agent.infrastructure.voice

import android.content.Context
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.util.Log

object EngineProbe {

    private const val TAG = "EngineProbe"
    private const val PROBE_TIMEOUT_MS = 1500L

    data class ProbeResult(
        val key: String,
        val health: EngineHealth,
        val errorCode: Int? = null,
        val errorMessage: String? = null,
    )

    /**
     * 异步探测单个引擎。结果回调在 [callback] 中触发（主线程）。
     *
     * @param engine 候选引擎；component=null 表示使用系统默认
     * @param callback 完成时回调，参数为 [ProbeResult]
     */
    fun probe(context: Context, engine: RecognitionEngine, callback: (ProbeResult) -> Unit) {
        val handler = Handler(Looper.getMainLooper())
        handler.post {
            doProbe(context, engine, handler, callback)
        }
    }

    /**
     * 顺序探测一批引擎，每个引擎完成后回调 [onEach]。全部完成后回调 [onDone]。
     */
    fun probeAll(
        context: Context,
        engines: List<RecognitionEngine>,
        onEach: (ProbeResult) -> Unit,
        onDone: () -> Unit,
    ) {
        val handler = Handler(Looper.getMainLooper())
        fun next(i: Int) {
            if (i >= engines.size) {
                onDone()
                return
            }
            doProbe(context, engines[i], handler) { result ->
                onEach(result)
                handler.postDelayed({ next(i + 1) }, 200L) // 给引擎释放时间
            }
        }
        handler.post { next(0) }
    }

    private fun doProbe(
        context: Context,
        engine: RecognitionEngine,
        handler: Handler,
        callback: (ProbeResult) -> Unit,
    ) {
        val key = engine.key()
        Log.i(TAG, "开始探测: ${engine.label} ($key)")
        val app = context.applicationContext
        val recognizer = try {
            if (engine.component != null) {
                SpeechRecognizer.createSpeechRecognizer(app, engine.component)
            } else {
                SpeechRecognizer.createSpeechRecognizer(app)
            }
        } catch (t: Throwable) {
            Log.w(TAG, "创建 recognizer 失败: ${t.message}")
            callback(ProbeResult(key, EngineHealth.FAILED, -1, "创建识别器失败: ${t.message}"))
            return
        }

        var finished = false
        val timeout = Runnable {
            if (!finished) {
                finished = true
                Log.w(TAG, "探测超时: ${engine.label}")
                try { recognizer.cancel() } catch (_: Throwable) {}
                try { recognizer.destroy() } catch (_: Throwable) {}
                callback(ProbeResult(key, EngineHealth.FAILED, -2, "探测超时（${PROBE_TIMEOUT_MS}ms 内未就绪）"))
            }
        }

        recognizer.setRecognitionListener(object : RecognitionListener {
            override fun onReadyForSpeech(params: Bundle?) {
                if (finished) return
                finished = true
                handler.removeCallbacks(timeout)
                Log.i(TAG, "✅ 探测通过: ${engine.label}")
                try { recognizer.cancel() } catch (_: Throwable) {}
                try { recognizer.destroy() } catch (_: Throwable) {}
                callback(ProbeResult(key, EngineHealth.OK))
            }

            override fun onError(error: Int) {
                if (finished) return
                finished = true
                handler.removeCallbacks(timeout)
                val msg = errorName(error)
                Log.w(TAG, "❌ 探测失败: ${engine.label} code=$error ($msg)")
                try { recognizer.cancel() } catch (_: Throwable) {}
                try { recognizer.destroy() } catch (_: Throwable) {}
                callback(ProbeResult(key, EngineHealth.FAILED, error, msg))
            }

            override fun onBeginningOfSpeech() = Unit
            override fun onRmsChanged(rmsdB: Float) = Unit
            override fun onBufferReceived(buffer: ByteArray?) = Unit
            override fun onEndOfSpeech() = Unit
            override fun onResults(results: Bundle?) = Unit
            override fun onPartialResults(partialResults: Bundle?) = Unit
            override fun onEvent(eventType: Int, params: Bundle?) = Unit
        })

        val intent = android.content.Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, "zh-CN")
            putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
            putExtra(RecognizerIntent.EXTRA_CALLING_PACKAGE, app.packageName)
        }

        handler.postDelayed(timeout, PROBE_TIMEOUT_MS)
        try {
            recognizer.startListening(intent)
        } catch (t: Throwable) {
            if (!finished) {
                finished = true
                handler.removeCallbacks(timeout)
                Log.w(TAG, "startListening 抛异常: ${t.message}")
                try { recognizer.destroy() } catch (_: Throwable) {}
                callback(ProbeResult(key, EngineHealth.FAILED, -3, "启动识别异常: ${t.message}"))
            }
        }
    }

    private fun errorName(code: Int): String = when (code) {
        SpeechRecognizer.ERROR_AUDIO -> "录音错误"
        SpeechRecognizer.ERROR_CLIENT -> "客户端错误"
        SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "权限不足"
        SpeechRecognizer.ERROR_NETWORK -> "网络错误"
        SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "网络超时"
        SpeechRecognizer.ERROR_NO_MATCH -> "未识别到语音"
        SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "识别器忙"
        SpeechRecognizer.ERROR_SERVER -> "服务器错误"
        SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "语音超时"
        11 -> "服务连接被断开"
        12 -> "语言不支持"
        13 -> "语言未下载"
        14 -> "无法检查语言支持"
        else -> "未知错误($code)"
    }
}
