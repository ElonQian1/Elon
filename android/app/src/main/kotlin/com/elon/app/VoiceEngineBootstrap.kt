// VoiceEngineBootstrap.kt — 应用启动时静默探测语音引擎
// 策略：
//  - 若已存在至少一个 OK 引擎，且用户已设过偏好，则跳过
//  - 否则延迟 4 秒后台跑一次全量探测，把结果写入 EnginePreference
//  - 不弹任何 UI；用户进入"管理引擎"页时可看到结果

package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.speech.SpeechRecognizer
import android.util.Log
import com.elon.app.agent.infrastructure.voice.EngineHealth
import com.elon.app.agent.infrastructure.voice.EnginePreference
import com.elon.app.agent.infrastructure.voice.EngineProbe
import com.elon.app.agent.infrastructure.voice.RecognitionEngineSelector

object VoiceEngineBootstrap {
    private const val TAG = "VoiceEngineBootstrap"

    @Volatile private var scheduled = false

    fun scheduleSilentProbeIfNeeded(context: Context) {
        if (scheduled) return
        scheduled = true
        Handler(Looper.getMainLooper()).postDelayed({
            try {
                doProbeIfNeeded(context.applicationContext)
            } catch (t: Throwable) {
                Log.w(TAG, "silent probe 失败: ${t.message}")
            }
        }, 4000L)
    }

    private fun doProbeIfNeeded(context: Context) {
        val engines = RecognitionEngineSelector.list(context)
        if (engines.isEmpty()) {
            Log.i(TAG, "本机无候选引擎，跳过静默探测")
            return
        }
        val hasOk = engines.any { EnginePreference.getHealth(context, it.key()) == EngineHealth.OK }
        val hasPreferred = EnginePreference.getPreferredKey(context) != null
        if (hasOk && hasPreferred) {
            Log.i(TAG, "已有 OK 引擎和偏好，跳过静默探测")
            return
        }
        Log.i(TAG, "启动静默探测 ${engines.size} 个引擎")
        EngineProbe.probeAll(context, engines,
            onEach = { result ->
                EnginePreference.setHealth(context, result.key, result.health, result.errorCode, result.errorMessage)
                Log.i(TAG, "  ${result.key} → ${result.health} (${result.errorMessage ?: "-"})")
            },
            onDone = {
                // 自动排除"系统常驻语音助手"引擎（ERROR_RECOGNIZER_BUSY=8）
                // 条件：至少还剩一个非 BUSY 引擎可用，且用户未手动配置过
                val busyKeys = engines.filter { engine ->
                    val err = EnginePreference.getLastError(context, engine.key())
                    err?.first == SpeechRecognizer.ERROR_RECOGNIZER_BUSY
                }.map { it.key() }
                val nonBusyCount = engines.count { it.key() !in busyKeys }
                if (busyKeys.isNotEmpty() && nonBusyCount > 0) {
                    busyKeys.forEach { busyKey ->
                        if (!AsrFallbackSettings.isEngineDisabled(context, busyKey)) {
                            AsrFallbackSettings.setEngineDisabled(context, busyKey, true)
                            Log.i(TAG, "自动排除系统常驻引擎（RECOGNIZER_BUSY）: $busyKey")
                        }
                    }
                }

                // 若用户未设偏好，且至少有一个 OK 引擎，自动把第一个 OK 引擎设为偏好
                if (EnginePreference.getPreferredKey(context) == null) {
                    val firstOk = engines.firstOrNull { EnginePreference.getHealth(context, it.key()) == EngineHealth.OK }
                    if (firstOk != null) {
                        EnginePreference.setPreferredKey(context, firstOk.key())
                        Log.i(TAG, "自动设定偏好引擎: ${firstOk.label}")
                    }
                }
                Log.i(TAG, "静默探测完成")
            }
        )
    }
}
