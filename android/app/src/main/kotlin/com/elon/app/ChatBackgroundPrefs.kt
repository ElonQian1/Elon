package com.elon.app

import android.content.Context

/**
 * 后台消息保活偏好设置。
 *
 * - 默认开启：用户首次登录后自动启用，让 APK 在后台也能像微信一样收到好友消息提醒。
 * - 用户可在设置页关闭：关闭后服务停止，进程进入后台后将依赖系统调度，可能延迟到位。
 */
internal object ChatBackgroundPrefs {
    private const val PREFS = "chat_background_prefs"
    private const val KEY_KEEP_ALIVE_ENABLED = "keep_alive_enabled_v1"

    fun isKeepAliveEnabled(context: Context): Boolean {
        return prefs(context).getBoolean(KEY_KEEP_ALIVE_ENABLED, true)
    }

    fun setKeepAliveEnabled(context: Context, enabled: Boolean) {
        prefs(context).edit().putBoolean(KEY_KEEP_ALIVE_ENABLED, enabled).apply()
    }

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
