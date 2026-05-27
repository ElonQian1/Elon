package com.elon.app

import android.app.Activity
import android.content.Intent
import android.content.SharedPreferences
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import androidx.appcompat.app.AlertDialog

internal object TaskBackgroundKeepAlive {
    private const val PREF_BATTERY_PROMPT_LAST_SHOWN_AT = "task_battery_prompt_last_shown_at"
    private const val PROMPT_INTERVAL_MS = 7L * 24 * 60 * 60 * 1000

    fun maybePromptForDevelopmentTask(
        activity: Activity,
        prefs: SharedPreferences,
        isDevelopment: Boolean
    ) {
        if (!isDevelopment || Build.VERSION.SDK_INT < Build.VERSION_CODES.M) return
        if (activity.isFinishing || activity.isDestroyed) return
        if (isIgnoringBatteryOptimizations(activity)) return

        val now = System.currentTimeMillis()
        val lastShown = prefs.getLong(PREF_BATTERY_PROMPT_LAST_SHOWN_AT, 0L)
        if (now - lastShown < PROMPT_INTERVAL_MS) return

        prefs.edit().putLong(PREF_BATTERY_PROMPT_LAST_SHOWN_AT, now).apply()
        DebugTraceStore.record("task_battery_prompt_shown")
        AlertDialog.Builder(activity)
            .setTitle("允许后台开发保活")
            .setMessage("开发任务会通过常驻通知继续连接服务器。为减少切到微信等应用后被系统暂停，请把一龙的电池使用设置为不受限制或允许后台运行。")
            .setPositiveButton("去设置") { _, _ -> openBatterySettings(activity) }
            .setNegativeButton("稍后", null)
            .show()
    }

    private fun isIgnoringBatteryOptimizations(activity: Activity): Boolean {
        val powerManager = activity.getSystemService(PowerManager::class.java) ?: return true
        return powerManager.isIgnoringBatteryOptimizations(activity.packageName)
    }

    private fun openBatterySettings(activity: Activity) {
        val packageName = activity.packageName
        val requestIntent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
            data = Uri.parse("package:$packageName")
        }
        val fallbackIntent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).apply {
            data = Uri.parse("package:$packageName")
        }
        runCatching {
            activity.startActivity(requestIntent)
        }.recoverCatching {
            activity.startActivity(fallbackIntent)
        }.onFailure { error ->
            DebugTraceStore.record(
                "task_battery_settings_open_failed",
                mapOf("error" to (error.message ?: error.javaClass.simpleName))
            )
        }
    }
}
