package com.elon.app

import android.content.Context
import android.content.SharedPreferences
import okhttp3.Request
import org.json.JSONObject
import java.util.UUID

/**
 * 账号会话管理。
 *
 * 设计原则：
 * - 兼容老用户：未登录时仍走原来的随机 user_id 匿名模式。
 * - 登录后：`effectiveUserId` 返回服务器分配的 user.id；服务端所有 `/api/user/:user_id/...`
 *   路由继续工作，相当于把本机身份升级成可跨设备的账号身份。
 * - Token 通过 `Authorization: Bearer <token>` 发到需要鉴权的接口（如 `/api/me`、
 *   `/api/me/projects`、`POST /api/projects`）。老接口不需要 token。
 */
object AuthManager {
    private const val PREFS_NAME = "elon"
    private const val KEY_AUTH_TOKEN = "auth_token"
    private const val KEY_AUTH_USER_ID = "auth_user_id"
    private const val KEY_AUTH_ACCOUNT = "auth_account"
    private const val KEY_AUTH_NICKNAME = "auth_nickname"
    private const val KEY_AUTH_EXPIRES_AT = "auth_expires_at"
    private const val KEY_LEGACY_USER_ID = "user_id"

    fun prefs(ctx: Context): SharedPreferences =
        ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    /**
     * 返回当前有效用户的数据存储（项目列表、会话等），按账号 ID 隔离。
     * 已登录账号使用 "elon_data_<userId>"；游客使用 "elon_data_<deviceUUID>"。
     * 认证凭据本身仍存储在 prefs()（"elon"），不受影响。
     */
    fun userDataPrefs(ctx: Context): SharedPreferences {
        val userId = effectiveUserId(ctx)
        return ctx.getSharedPreferences("elon_data_$userId", Context.MODE_PRIVATE)
    }

    /**
     * 返回游客（设备 UUID）专用的数据存储，始终以本机设备 UUID 命名，
     * 不随账号登录/登出而变化。用于登录后迁移游客历史记录。
     */
    fun guestDataPrefs(ctx: Context): SharedPreferences {
        val guestId = legacyAnonymousUserId(ctx)
        return ctx.getSharedPreferences("elon_data_$guestId", Context.MODE_PRIVATE)
    }

    fun isLoggedIn(ctx: Context): Boolean = !token(ctx).isNullOrBlank() && !userId(ctx).isNullOrBlank()

    fun token(ctx: Context): String? = prefs(ctx).getString(KEY_AUTH_TOKEN, null)?.takeIf { it.isNotBlank() }

    fun userId(ctx: Context): String? = prefs(ctx).getString(KEY_AUTH_USER_ID, null)?.takeIf { it.isNotBlank() }

    fun account(ctx: Context): String? = prefs(ctx).getString(KEY_AUTH_ACCOUNT, null)?.takeIf { it.isNotBlank() }

    fun nickname(ctx: Context): String? = prefs(ctx).getString(KEY_AUTH_NICKNAME, null)?.takeIf { it.isNotBlank() }

    fun displayName(ctx: Context): String =
        nickname(ctx) ?: account(ctx) ?: "未登录"

    fun updateNickname(ctx: Context, nickname: String) {
        prefs(ctx).edit().putString(KEY_AUTH_NICKNAME, nickname.trim()).apply()
    }

    /** 登录后用此 ID 调用服务端所有 /api/user/:user_id/... 路由；未登录时返回本机匿名 ID。 */
    fun effectiveUserId(ctx: Context): String {
        userId(ctx)?.let { return it }
        return legacyAnonymousUserId(ctx)
    }

    /** 老版本匿名 user_id（未登录用户继续用这一份本机随机 UUID）。 */
    fun legacyAnonymousUserId(ctx: Context): String {
        val p = prefs(ctx)
        p.getString(KEY_LEGACY_USER_ID, null)?.takeIf { it.isNotBlank() }?.let { return it }
        val generated = UUID.randomUUID().toString().replace("-", "")
        p.edit().putString(KEY_LEGACY_USER_ID, generated).apply()
        return generated
    }

    fun saveSession(ctx: Context, token: String, userId: String, account: String?, nickname: String?, expiresAt: Long?) {
        prefs(ctx).edit().apply {
            putString(KEY_AUTH_TOKEN, token)
            putString(KEY_AUTH_USER_ID, userId)
            putString(KEY_AUTH_ACCOUNT, account.orEmpty())
            putString(KEY_AUTH_NICKNAME, nickname.orEmpty())
            putLong(KEY_AUTH_EXPIRES_AT, expiresAt ?: 0L)
        }.apply()
        refreshGlobalWsAuth(ctx)
    }

    fun clear(ctx: Context) {
        prefs(ctx).edit().apply {
            remove(KEY_AUTH_TOKEN)
            remove(KEY_AUTH_USER_ID)
            remove(KEY_AUTH_ACCOUNT)
            remove(KEY_AUTH_NICKNAME)
            remove(KEY_AUTH_EXPIRES_AT)
        }.apply()
        refreshGlobalWsAuth(ctx)
    }

    /** 给 OkHttp Request.Builder 加 Bearer token；未登录时不加。 */
    fun applyAuth(ctx: Context, b: Request.Builder): Request.Builder {
        token(ctx)?.let { b.header("Authorization", "Bearer $it") }
        return b
    }

    /** 解析 /api/auth/login 或 /api/auth/register 的响应体并落地。 */
    fun handleAuthResponse(ctx: Context, body: String): String {
        val json = JSONObject(body)
        val token = json.optString("token", "").trim()
        val user = json.optJSONObject("user") ?: throw IllegalStateException("响应缺少 user 字段")
        val uid = user.optString("id", "").trim()
        val account = user.optString("account", "").trim().ifBlank { null }
        val nickname = user.optString("nickname", "").trim().ifBlank { null }
        val expiresAt = json.optLong("expires_at", 0L)
        if (token.isBlank() || uid.isBlank()) throw IllegalStateException("响应缺少 token 或 user.id")
        saveSession(ctx, token, uid, account, nickname, expiresAt.takeIf { it > 0 })
        return uid
    }

    private fun refreshGlobalWsAuth(ctx: Context) {
        val app = ctx.applicationContext
        (app as? ElonApplication)?.globalWs?.reconnectWithNewToken()
        // 登录后启动后台保活让用户能像微信一样后台收消息；登出后停止
        if (isLoggedIn(app) && ChatBackgroundPrefs.isKeepAliveEnabled(app)) {
            ChatBackgroundService.start(app)
        } else if (!isLoggedIn(app)) {
            ChatBackgroundService.stop(app)
        }
    }
}
