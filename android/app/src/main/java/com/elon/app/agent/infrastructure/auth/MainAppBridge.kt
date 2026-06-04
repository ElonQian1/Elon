// infrastructure/auth/MainAppBridge.kt
// module: infrastructure/auth | layer: infrastructure | role: main-app-bridge
// summary: 让 agent 子系统读取主 UI 的登录态和当前活跃项目，避免 agent 与主 UI 配置割裂

package com.elon.app.agent.infrastructure.auth

import android.content.Context
import android.util.Log

/**
 * 🌉 主 UI 桥接器
 *
 * agent 子系统（无障碍 / 悬浮球 / 语音）原本和主 UI 是两套孤岛：
 *   - 主 UI 把 token 存在 SharedPreferences("elon") → "auth_token"
 *   - 主 UI 把当前活跃项目 id 存在 SharedPreferences("elon_data_<userId>") → "active_project_id"
 *   - agent 自己一份 SharedPreferences("agent_config") + 让用户手动填 cli_project_id
 *
 * 这个对象统一从主 UI 的存储读出登录态和活跃项目，
 * agent 端的所有 AI 调用（HunyuanAIClient / LLMClient / 服务器 CLI）都通过它来决定走哪条链路。
 *
 * **存储 key 必须和主 UI 保持一致**，详见：
 *   - [com.elon.app.AuthManager] PREFS_NAME="elon", KEY_AUTH_TOKEN="auth_token"
 *   - [com.elon.app.TaskWorkService.PREF_ACTIVE_PROJECT_ID]="active_project_id"
 */
object MainAppBridge {
    private const val TAG = "MainAppBridge"

    // 必须和 com.elon.app.AuthManager 完全一致
    private const val MAIN_PREFS = "elon"
    private const val KEY_AUTH_TOKEN = "auth_token"
    private const val KEY_AUTH_USER_ID = "auth_user_id"
    private const val KEY_LEGACY_USER_ID = "user_id"

    // 必须和 com.elon.app.TaskWorkService.PREF_ACTIVE_PROJECT_ID 一致
    private const val KEY_ACTIVE_PROJECT_ID = "active_project_id"

    /** 主 UI 的 token；未登录返回 null。 */
    fun authToken(ctx: Context): String? {
        val v = ctx.getSharedPreferences(MAIN_PREFS, Context.MODE_PRIVATE)
            .getString(KEY_AUTH_TOKEN, null)
        return v?.takeIf { it.isNotBlank() }
    }

    fun isLoggedIn(ctx: Context): Boolean = !authToken(ctx).isNullOrBlank()

    /** 当前有效用户 id：登录账号优先，否则游客 UUID（与 AuthManager 一致）。 */
    fun effectiveUserId(ctx: Context): String {
        val prefs = ctx.getSharedPreferences(MAIN_PREFS, Context.MODE_PRIVATE)
        prefs.getString(KEY_AUTH_USER_ID, null)?.takeIf { it.isNotBlank() }?.let { return it }
        return prefs.getString(KEY_LEGACY_USER_ID, "").orEmpty()
    }

    /**
     * 主 UI 当前活跃项目 id（[com.elon.app.MainProjectData] 写入）。
     * 没有则返回 null。
     */
    fun activeProjectId(ctx: Context): String? {
        val userId = effectiveUserId(ctx).ifBlank {
            Log.w(TAG, "无 effectiveUserId，无法读取 active project")
            return null
        }
        val userPrefs = ctx.getSharedPreferences("elon_data_$userId", Context.MODE_PRIVATE)
        return userPrefs.getString(KEY_ACTIVE_PROJECT_ID, null)?.takeIf { it.isNotBlank() }
    }

    /**
     * agent 实际要发往服务器 CLI 的 projectId：
     *   1. 优先：主 UI 当前活跃项目（用户在主 UI 选了哪个就用哪个）
     *   2. 兜底：AgentConfig 里手动填的 cli_project_id（兼容老用户）
     */
    fun effectiveCliProjectId(ctx: Context): String? {
        activeProjectId(ctx)?.let { return it }
        val agentPrefs = ctx.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
        return agentPrefs.getString("cli_project_id", null)?.takeIf { it.isNotBlank() }
    }

    /** agent 配置里的 elon 服务器地址（默认线上服务器）。 */
    fun serverUrl(ctx: Context): String {
        val agentPrefs = ctx.getSharedPreferences("agent_config", Context.MODE_PRIVATE)
        return agentPrefs.getString("cli_server_url", null)?.takeIf { it.isNotBlank() }
            ?: "http://43.139.149.158:8080"
    }
}
