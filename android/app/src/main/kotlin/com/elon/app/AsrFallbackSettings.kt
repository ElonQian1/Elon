// AsrFallbackSettings.kt — 用户可配置的 ASR 回退链偏好
// 职责：持久化"哪些本地引擎被用户主动排除" + "是否启用服务器 Whisper 兜底"

package com.elon.app

import android.content.Context

/**
 * 用户可控制的 ASR 回退链偏好：
 *
 *  - [disabledEngineKeys]：被用户手动排除的本地引擎 key 集合。
 *    排除的引擎不会出现在 [AgentVoiceBridge] 的候选列表里，但仍在
 *    [VoiceEngineActivity] 里显示（标灰+可重新启用）。
 *
 *  - [serverFallbackEnabled]：本地所有（未被排除的）引擎全部失败后，
 *    是否把录音上传到服务器 Whisper 转写（默认开启）。
 *    仅对"语音消息"模式的转写路径有效，对"端上识别"模式无影响。
 */
object AsrFallbackSettings {

    private const val PREFS = "elon_asr_fallback"
    private const val KEY_SERVER_ENABLED = "server_fallback_enabled"
    private const val KEY_DISABLED_ENGINES = "disabled_engine_keys"
    private const val KEY_WHISPER_LANGUAGE = "whisper_language"
    private const val KEY_WHISPER_BEAM_SIZE = "whisper_beam_size"
    private const val KEY_WHISPER_VAD_FILTER = "whisper_vad_filter"
    private const val KEY_WHISPER_CONDITION_PREV = "whisper_condition_on_previous"

    // ──────────────── 服务器 Whisper 兜底 ────────────────

    /** 是否允许在本地引擎全败后上传录音到服务器识别，默认 true。 */
    fun isServerFallbackEnabled(context: Context): Boolean =
        prefs(context).getBoolean(KEY_SERVER_ENABLED, true)

    fun setServerFallbackEnabled(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_SERVER_ENABLED, enabled).apply()
    }

    // ──────────────── 云端 Whisper 转写语言 ────────────────

    /**
     * 云端 Whisper 转写时使用的语言代码：
     *   "zh"    = 简体中文（默认，输出简体字）
     *   "zh-TW" = 繁体中文
     *   "en"    = 英文
     *   "auto"  = 自动检测
     */
    fun getWhisperLanguage(context: Context): String =
        prefs(context).getString(KEY_WHISPER_LANGUAGE, "auto") ?: "auto"

    fun setWhisperLanguage(context: Context, lang: String) {
        prefs(context).edit().putString(KEY_WHISPER_LANGUAGE, lang).apply()
    }

    // ──────────────── 云端 Whisper 转写高级参数 ────────────────

    /**
     * beam_size：解码宽度，越大越准但越慢。
     *   1 = 最快（贪心解码）  5 = 平衡（默认）  10 = 最准
     */
    fun getWhisperBeamSize(context: Context): Int =
        prefs(context).getInt(KEY_WHISPER_BEAM_SIZE, 5)

    fun setWhisperBeamSize(context: Context, size: Int) {
        prefs(context).edit().putInt(KEY_WHISPER_BEAM_SIZE, size).apply()
    }

    /** vad_filter：是否启用静音过滤，开启后自动跳过无声片段（默认开启）。 */
    fun getWhisperVadFilter(context: Context): Boolean =
        prefs(context).getBoolean(KEY_WHISPER_VAD_FILTER, true)

    fun setWhisperVadFilter(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_WHISPER_VAD_FILTER, enabled).apply()
    }

    /**
     * condition_on_previous_text：是否让模型参考上一句识别结果（默认关闭）。
     * 关闭可避免一句识别错误"传染"到下一句。
     */
    fun getWhisperConditionOnPrevious(context: Context): Boolean =
        prefs(context).getBoolean(KEY_WHISPER_CONDITION_PREV, false)

    fun setWhisperConditionOnPrevious(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_WHISPER_CONDITION_PREV, enabled).apply()
    }

    // ──────────────── 本地引擎禁用列表 ────────────────

    /** 返回被用户排除的引擎 key 集合（不可变副本）。 */
    fun getDisabledEngineKeys(context: Context): Set<String> =
        prefs(context).getStringSet(KEY_DISABLED_ENGINES, emptySet())?.toSet() ?: emptySet()

    /** 某引擎是否被用户排除。 */
    fun isEngineDisabled(context: Context, key: String): Boolean =
        getDisabledEngineKeys(context).contains(key)

    /**
     * 设置某引擎的排除状态。
     * 注意：不允许把所有引擎全部排除——若操作后列表等于全量引擎列表则拒绝。
     * 调用方自行保证至少留一个引擎可用。
     */
    fun setEngineDisabled(context: Context, key: String, disabled: Boolean) {
        val p = prefs(context)
        val current = p.getStringSet(KEY_DISABLED_ENGINES, mutableSetOf())?.toMutableSet()
            ?: mutableSetOf()
        if (disabled) current.add(key) else current.remove(key)
        p.edit().putStringSet(KEY_DISABLED_ENGINES, current).apply()
    }

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
