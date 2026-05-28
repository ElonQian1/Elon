package com.elon.app

import android.content.Context

/**
 * 语音输入模式：用户可在「设置」中切换。
 *
 * - [LOCAL_AGENT_ASR]：使用 Agent 子系统的端上流式识别（`StreamingASR` + `SmartVAD`）。
 *   麦克风→端上系统 ASR→文字→填入输入框→走正常文字发送链路→后端。
 *   优点：低延迟、可离线（取决于厂商引擎）、和键盘输入完全同一条链路。
 *
 * - [CLOUD_REALTIME]：使用云端 PCM 直连（`RealtimeVoiceController`）。
 *   麦克风→PCM 流→服务器 WS→转写→AI 自动派发 CLI。
 *   优点：服务器自有 ASR 模型质量可控；用于直接驱动 AI 流程。
 *
 * 默认值：[LOCAL_AGENT_ASR]（按用户要求，2026-05 起作为主聊天语音管线）。
 */
enum class VoiceInputMode {
    LOCAL_AGENT_ASR,
    CLOUD_REALTIME,

    /**
     * 语音消息模式：长按麦克风录音，松开后以语音气泡发送到聊天，可点击收听。
     * 不经过 ASR，音频原始文件上传到服务器。
     */
    VOICE_MESSAGE;

    companion object {
        fun fromKey(key: String?): VoiceInputMode = when (key) {
            CLOUD_REALTIME.name -> CLOUD_REALTIME
            VOICE_MESSAGE.name -> VOICE_MESSAGE
            else -> LOCAL_AGENT_ASR
        }
    }
}

object VoiceInputModeSettings {
    private const val PREFS_NAME = "elon"
    private const val KEY_MODE = "voice_input_mode"

    fun get(context: Context): VoiceInputMode {
        val prefs = context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        return VoiceInputMode.fromKey(prefs.getString(KEY_MODE, null))
    }

    fun set(context: Context, mode: VoiceInputMode) {
        context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_MODE, mode.name)
            .apply()
    }
}
