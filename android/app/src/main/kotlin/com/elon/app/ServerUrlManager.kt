package com.elon.app

import android.content.Context
import android.util.Log

/**
 * 主/备服务器 URL 自动切换管理器。
 *
 * - 主服务器：BuildConfig.SERVER_URL（云端）
 * - 备用服务器：用户在代理配置中填写的 "fallback_server_url"（本地开发机）
 *
 * 切换规则：
 *  1. WS 连续失败 [FAILURE_THRESHOLD] 次 → 切换到备用服务器
 *  2. 备用连接成功后保持使用备用
 *  3. 每隔 [RETRY_PRIMARY_INTERVAL_MS] 毫秒，[getActive] 让 GlobalWsManager
 *     尝试切回主服务器；主服务器恢复则自然保持，否则再次切回备用
 *
 * 状态存储在 SharedPreferences "server_url_mgr"，跨进程重启持久化。
 */
object ServerUrlManager {

    private const val TAG = "ServerUrlMgr"
    private const val PREF_NAME = "server_url_mgr"
    private const val KEY_USE_FALLBACK = "use_fallback"
    private const val KEY_FAILURE_COUNT = "failure_count"
    private const val KEY_FALLBACK_SINCE = "fallback_since_ms"

    /** 连续失败几次后切换备用 */
    private const val FAILURE_THRESHOLD = 3

    /** 切换到备用后，每隔多久尝试一次切回主服务器 */
    private const val RETRY_PRIMARY_INTERVAL_MS = 5 * 60 * 1000L  // 5 分钟

    // ── 公开 API ──────────────────────────────────────────────────────────────

    /**
     * 返回当前应连接的服务器 URL。
     * - 未配置备用 URL → 始终返回主服务器
     * - 已切换到备用且未到重试间隔 → 返回备用
     * - 已切换到备用且超过重试间隔 → 重置为主服务器（让 WS 尝试恢复）
     */
    fun getActive(ctx: Context): String {
        val fallback = fallbackUrl(ctx) ?: return primary()
        val p = ctx.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        if (!p.getBoolean(KEY_USE_FALLBACK, false)) return primary()

        // 超过重试间隔，尝试切回主服务器
        val since = p.getLong(KEY_FALLBACK_SINCE, 0L)
        if (System.currentTimeMillis() - since > RETRY_PRIMARY_INTERVAL_MS) {
            Log.i(TAG, "5 分钟已到，尝试切回主服务器")
            p.edit()
                .putBoolean(KEY_USE_FALLBACK, false)
                .putInt(KEY_FAILURE_COUNT, 0)
                .apply()
            return primary()
        }
        return fallback
    }

    /** WS 连接成功时调用——重置连续失败计数。 */
    fun reportSuccess(ctx: Context) {
        val p = ctx.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        val prev = p.getInt(KEY_FAILURE_COUNT, 0)
        if (prev > 0) Log.i(TAG, "连接成功，失败计数归零（之前=$prev）")
        p.edit().putInt(KEY_FAILURE_COUNT, 0).apply()
    }

    /**
     * WS 连接失败时调用。
     * 仅在连接主服务器时累计失败次数；已切换到备用时不做切换（让 GlobalWsManager 自行重试备用）。
     */
    fun reportFailure(ctx: Context) {
        if (fallbackUrl(ctx) == null) return          // 未配置备用，不做切换
        val p = ctx.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        if (p.getBoolean(KEY_USE_FALLBACK, false)) return  // 已在备用，跳过

        val count = p.getInt(KEY_FAILURE_COUNT, 0) + 1
        p.edit().putInt(KEY_FAILURE_COUNT, count).apply()
        Log.w(TAG, "主服务器连续失败 $count/$FAILURE_THRESHOLD")

        if (count >= FAILURE_THRESHOLD) {
            val fallback = fallbackUrl(ctx)!!
            Log.w(TAG, "切换到备用服务器: $fallback")
            p.edit()
                .putBoolean(KEY_USE_FALLBACK, true)
                .putLong(KEY_FALLBACK_SINCE, System.currentTimeMillis())
                .apply()
        }
    }

    /** 手动强制切回主服务器（供设置页调用）。 */
    fun forcePrimary(ctx: Context) {
        ctx.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_USE_FALLBACK, false)
            .putInt(KEY_FAILURE_COUNT, 0)
            .apply()
        Log.i(TAG, "手动切回主服务器")
    }

    /** 当前服务器状态描述，供 UI 显示。 */
    fun statusLabel(ctx: Context): String {
        val p = ctx.getSharedPreferences(PREF_NAME, Context.MODE_PRIVATE)
        return if (p.getBoolean(KEY_USE_FALLBACK, false)) {
            "备用服务器（${fallbackUrl(ctx) ?: "未配置"}）"
        } else {
            "主服务器（${primary()}）"
        }
    }

    // ── 私有辅助 ──────────────────────────────────────────────────────────────

    private fun primary(): String = BuildConfig.SERVER_URL

    /** 从 agent_config 偏好读取用户配置的备用服务器 URL；未配置时返回 null。 */
    private fun fallbackUrl(ctx: Context): String? =
        ctx.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
            .getString("fallback_server_url", "")
            .takeIf { !it.isNullOrBlank() }
}
