// infrastructure/voice/EnginePreference.kt
// module: infrastructure/voice | layer: infrastructure | role: persistence
// summary: 持久化"用户偏好引擎"和"已知失败引擎"，以及最近一次探测结果。

package com.elon.app.agent.infrastructure.voice

import android.content.ComponentName
import android.content.Context

/**
 * 引擎健康状态。
 */
enum class EngineHealth {
    /** 从未测试过 */
    UNKNOWN,
    /** 正在测试中 */
    PROBING,
    /** 测试通过（onReady 触发） */
    OK,
    /** 测试失败（startListening 后短时间内 onError） */
    FAILED,
}

/**
 * 单个引擎的可观测状态（给 UI 用）。
 */
data class EngineStatus(
    val component: ComponentName?,
    val packageName: String,
    val label: String,
    val health: EngineHealth,
    val lastErrorCode: Int?,
    val lastErrorMessage: String?,
    val isUserPreferred: Boolean,
) {
    fun key(): String = component?.flattenToShortString() ?: "<system-default>"
}

/**
 * 持久化：
 *  - `preferred_engine_key`：用户主动选定的引擎（key 形式），下次启动直接用
 *  - `engine_health_<key>`：上次探测结果（OK/FAILED + 错误码）。仅用作 UI 显示和
 *    "下次按麦克风时跳过已知坏引擎"的提示，不做永久封禁 —— 用户可以在 UI 里
 *    手动重新测试或清空。
 */
object EnginePreference {
    private const val PREFS = "elon_voice_engine"
    private const val KEY_PREFERRED = "preferred_engine_key"
    private const val KEY_HEALTH_PREFIX = "engine_health_"
    private const val KEY_ERROR_PREFIX = "engine_error_"

    /** 用户偏好的引擎 key（如 "com.xiaomi.mibrain.speech/.asr.AsrService" 或 "<system-default>"），null 表示未设置 */
    fun getPreferredKey(context: Context): String? {
        return prefs(context).getString(KEY_PREFERRED, null)
    }

    /** 设置用户偏好引擎；传 null 清空 */
    fun setPreferredKey(context: Context, key: String?) {
        prefs(context).edit().apply {
            if (key == null) remove(KEY_PREFERRED) else putString(KEY_PREFERRED, key)
        }.apply()
    }

    /** 读取某引擎的上次探测健康度（未探测过返回 UNKNOWN） */
    fun getHealth(context: Context, key: String): EngineHealth {
        val name = prefs(context).getString(KEY_HEALTH_PREFIX + key, null) ?: return EngineHealth.UNKNOWN
        return runCatching { EngineHealth.valueOf(name) }.getOrDefault(EngineHealth.UNKNOWN)
    }

    /** 记录探测结果。OK 时清空错误信息；FAILED 时存错误码+消息 */
    fun setHealth(context: Context, key: String, health: EngineHealth, errorCode: Int? = null, errorMessage: String? = null) {
        prefs(context).edit().apply {
            putString(KEY_HEALTH_PREFIX + key, health.name)
            if (health == EngineHealth.OK) {
                remove(KEY_ERROR_PREFIX + key)
            } else if (errorCode != null) {
                putString(KEY_ERROR_PREFIX + key, "$errorCode|${errorMessage ?: ""}")
            }
        }.apply()
    }

    /** 读取最近一次错误（仅 FAILED 时有值） */
    fun getLastError(context: Context, key: String): Pair<Int, String>? {
        val raw = prefs(context).getString(KEY_ERROR_PREFIX + key, null) ?: return null
        val idx = raw.indexOf('|')
        if (idx < 0) return null
        val code = raw.substring(0, idx).toIntOrNull() ?: return null
        val msg = raw.substring(idx + 1)
        return code to msg
    }

    /** 清空所有探测记录（用户可在 UI 触发重新测试） */
    fun clearAllHealth(context: Context) {
        val p = prefs(context)
        val toRemove = p.all.keys.filter { it.startsWith(KEY_HEALTH_PREFIX) || it.startsWith(KEY_ERROR_PREFIX) }
        p.edit().apply {
            toRemove.forEach { remove(it) }
        }.apply()
    }

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
