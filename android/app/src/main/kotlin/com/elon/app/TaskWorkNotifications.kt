package com.elon.app

import android.Manifest
import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioManager
import android.media.RingtoneManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

private const val PREF_NOTIFICATION_PERMISSION_ASKED = "notification_permission_asked_v4_heads_up_alerts"
private const val PREF_RECENT_TASK_COMPLETION_KEY = "recent_task_completion_key"
private const val PREF_RECENT_TASK_COMPLETION_AT = "recent_task_completion_at"
private const val RECENT_TASK_COMPLETION_WINDOW_MS = 2 * 60 * 1000L
private const val TASK_COMPLETE_FALLBACK_RING_MS = 1500L

internal fun setupTaskCompletionAlerts(activity: Activity, prefs: SharedPreferences, requestCode: Int) {
    ChatMessageNotifications.createChannel(activity)
    createTaskWorkNotificationChannels(activity)
    requestTaskNotificationPermissionIfNeeded(activity, prefs, requestCode)
}

internal fun clearCompletedTaskBadge(context: Context, prefs: SharedPreferences) {
    prefs.edit().putInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, 0).apply()
    NotificationManagerCompat.from(context).cancel(TaskWorkService.TASK_COMPLETE_NOTIFICATION_ID)
    setTaskLauncherBadgeCount(context, 0)
}

internal fun createTaskWorkNotificationChannels(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val notificationManager = context.getSystemService(NotificationManager::class.java)
    val soundUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
    val soundAttributes = AudioAttributes.Builder()
        .setUsage(AudioAttributes.USAGE_NOTIFICATION)
        .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
        .build()
    notificationManager.createNotificationChannel(
        NotificationChannel(
            TaskWorkService.ACTIVE_WORK_CHANNEL_ID,
            "后台任务运行",
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = "任务执行中保持后台连接"
            setShowBadge(false)
        }
    )
    notificationManager.createNotificationChannel(
        NotificationChannel(
            TaskWorkService.TASK_COMPLETE_CHANNEL_ID,
            "任务完成提醒",
            NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = "后台任务或项目会话完成后发出声音并显示桌面角标"
            setShowBadge(true)
            setSound(soundUri, soundAttributes)
            enableVibration(true)
        }
    )
    notificationManager.createNotificationChannel(
        NotificationChannel(
            TaskWorkService.APP_UPDATE_CHANNEL_ID,
            "应用更新提醒",
            NotificationManager.IMPORTANCE_DEFAULT
        ).apply {
            description = "一龙 APP 有新版本时提醒"
            setShowBadge(true)
        }
    )
}

private fun requestTaskNotificationPermissionIfNeeded(activity: Activity, prefs: SharedPreferences, requestCode: Int) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
    if (ContextCompat.checkSelfPermission(activity, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
    ) {
        return
    }
    if (prefs.getBoolean(PREF_NOTIFICATION_PERMISSION_ASKED, false)) return
    prefs.edit().putBoolean(PREF_NOTIFICATION_PERMISSION_ASKED, true).apply()
    ActivityCompat.requestPermissions(
        activity,
        arrayOf(Manifest.permission.POST_NOTIFICATIONS),
        requestCode
    )
}

internal fun activeTaskNotification(context: Context): Notification {
    return NotificationCompat.Builder(context, TaskWorkService.ACTIVE_WORK_CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle("一龙正在处理任务")
        .setContentText("切到其他应用时，任务会继续在后台运行。")
        .setContentIntent(mainActivityPendingIntent(context))
        .setOngoing(true)
        .setOnlyAlertOnce(true)
        .setSilent(true)
        .setCategory(NotificationCompat.CATEGORY_SERVICE)
        .setPriority(NotificationCompat.PRIORITY_LOW)
        .build()
}

internal fun updateProgressNotification(
    context: Context,
    step: Int,
    total: Int,
    phase: String,
    etaText: String?
): Notification {
    val phaseLabel = when (phase) {
        "ai_thinking" -> "理解需求"
        "code_editing" -> "修改代码"
        "code_committing" -> "提交代码"
        "building" -> "编译打包"
        "deploying" -> "部署发布"
        else -> "处理中"
    }
    return NotificationCompat.Builder(context, TaskWorkService.ACTIVE_WORK_CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle("第 $step/$total 步：$phaseLabel")
        .setContentText(etaText?.let { "预计还有 $it" } ?: "一龙正在后台工作")
        .setProgress(total, step, false)
        .setContentIntent(mainActivityPendingIntent(context))
        .setOngoing(true)
        .setOnlyAlertOnce(true)
        .setSilent(true)
        .setPriority(NotificationCompat.PRIORITY_LOW)
        .build()
}

internal fun notifyBackgroundTaskCompleted(
    context: Context,
    prefs: SharedPreferences,
    wasDevelopment: Boolean,
    apkUrl: String?,
    success: Boolean,
    projectId: String? = null,
    conversationId: String? = null
) {
    markTaskCompletionNotified(prefs, projectId, conversationId)
    if (success) {
        markProjectTaskCompletionBadge(prefs, projectId)
    }
    val count = prefs.getInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, 0).coerceAtLeast(0) + 1
    prefs.edit().putInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, count).apply()
    setTaskLauncherBadgeCount(context, count)
    showTaskCompletedNotification(context, count, wasDevelopment, apkUrl, success)
}

internal fun notifyProjectTaskDoneFromGlobalWs(
    context: Context,
    prefs: SharedPreferences,
    projectId: String?,
    conversationId: String?,
    message: String,
    apkUrl: String?
) {
    if (hasPendingLocalTask(prefs, projectId, conversationId)) return
    if (wasTaskCompletionRecentlyNotified(prefs, projectId, conversationId)) return

    markTaskCompletionNotified(prefs, projectId, conversationId)
    markProjectTaskCompletionBadge(prefs, projectId)
    val count = prefs.getInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, 0).coerceAtLeast(0) + 1
    prefs.edit().putInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, count).apply()
    setTaskLauncherBadgeCount(context, count)
    showTaskCompletedNotification(
        context = context,
        count = count,
        wasDevelopment = true,
        apkUrl = apkUrl,
        success = true,
        titleOverride = "会话已完成",
        textOverride = taskCompletionNotificationText(message, apkUrl)
    )
}

internal fun showAppUpdateNotification(context: Context, json: JSONObject) {
    val versionCode = json.optInt("versionCode", 0)
    if (versionCode <= BuildConfig.VERSION_CODE) return
    if (!canPostNotifications(context)) return

    val versionName = json.optString("versionName").takeIf { it.isNotBlank() } ?: "新版"
    val changelog = json.optString("changelog").takeIf { it.isNotBlank() }
    val intent = Intent(context, MainActivity::class.java).apply {
        flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        putExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE, true)
    }
    val pendingIntent = PendingIntent.getActivity(
        context,
        2,
        intent,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
    val notification = NotificationCompat.Builder(context, TaskWorkService.APP_UPDATE_CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle("一龙有新版本 v$versionName")
        .setContentText(changelog ?: "点击查看并安装更新")
        .setContentIntent(pendingIntent)
        .setAutoCancel(true)
        .setOnlyAlertOnce(true)
        .setCategory(NotificationCompat.CATEGORY_STATUS)
        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        .build()
    runCatching {
        NotificationManagerCompat.from(context).notify(TaskWorkService.APP_UPDATE_NOTIFICATION_ID, notification)
    }
}

private fun showTaskCompletedNotification(
    context: Context,
    count: Int,
    wasDevelopment: Boolean,
    apkUrl: String?,
    success: Boolean,
    titleOverride: String? = null,
    textOverride: String? = null
) {
    if (!canPostNotifications(context)) return

    val title = titleOverride ?: when {
        !success -> "任务需要处理"
        wasDevelopment -> "开发任务已完成"
        else -> "任务已完成"
    }
    val text = textOverride ?: if (apkUrl != null) {
        "已有 $count 个任务完成，APK 可以下载测试。"
    } else {
        "已有 $count 个任务完成，点击查看结果。"
    }
    val soundUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
    val notification = NotificationCompat.Builder(context, TaskWorkService.TASK_COMPLETE_CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle(title)
        .setContentText(text)
        .setTicker(title)
        .setStyle(NotificationCompat.BigTextStyle().bigText(text))
        .setNumber(count)
        .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
        .setContentIntent(mainActivityPendingIntent(context))
        .setAutoCancel(true)
        .setOnlyAlertOnce(false)
        .setDefaults(NotificationCompat.DEFAULT_SOUND or NotificationCompat.DEFAULT_VIBRATE)
        .setSound(soundUri)
        .setVibrate(longArrayOf(0L, 260L, 120L, 260L))
        .setCategory(NotificationCompat.CATEGORY_STATUS)
        .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
        .setPriority(NotificationCompat.PRIORITY_HIGH)
        .build()
    runCatching {
        NotificationManagerCompat.from(context).notify(TaskWorkService.TASK_COMPLETE_NOTIFICATION_ID, notification)
    }
    playFallbackTaskCompletionSound(context)
}

private fun mainActivityPendingIntent(context: Context): PendingIntent {
    val intent = Intent(context, MainActivity::class.java).apply {
        flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
    }
    return PendingIntent.getActivity(
        context,
        0,
        intent,
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
}

private fun canPostNotifications(context: Context): Boolean {
    if (!NotificationManagerCompat.from(context).areNotificationsEnabled()) return false
    return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
}

private fun playFallbackTaskCompletionSound(context: Context) {
    val appContext = context.applicationContext
    val audioManager = appContext.getSystemService(AudioManager::class.java) ?: return
    if (audioManager.ringerMode != AudioManager.RINGER_MODE_NORMAL) return
    if (audioManager.getStreamVolume(AudioManager.STREAM_NOTIFICATION) <= 0) return
    val soundUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION) ?: return
    val ringtone = runCatching { RingtoneManager.getRingtone(appContext, soundUri) }.getOrNull() ?: return
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
        ringtone.audioAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_NOTIFICATION)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()
    }
    runCatching { ringtone.play() }
    Handler(Looper.getMainLooper()).postDelayed({
        runCatching {
            if (ringtone.isPlaying) ringtone.stop()
        }
    }, TASK_COMPLETE_FALLBACK_RING_MS)
}

private fun hasPendingLocalTask(
    prefs: SharedPreferences,
    projectId: String?,
    conversationId: String?
): Boolean {
    val tasksJson = prefs.getString(TaskWorkService.PREF_PENDING_WORK_TASKS, null)?.takeIf { it.isNotBlank() }
    if (tasksJson != null) {
        val array = runCatching { JSONArray(tasksJson) }.getOrNull() ?: return false
        for (index in 0 until array.length()) {
            val payload = array.optJSONObject(index)?.optString("payload") ?: continue
            if (isSameTaskPayload(payload, projectId, conversationId)) return true
        }
    }

    val payload = prefs.getString(TaskWorkService.PREF_PENDING_WORK_PAYLOAD, null)
    return isSameTaskPayload(payload, projectId, conversationId)
}

private fun isSameTaskPayload(payload: String?, projectId: String?, conversationId: String?): Boolean {
    if (payload.isNullOrBlank()) return false
    val payloadProjectId = taskPayloadString(payload, "project_id")
    val payloadConversationId = taskPayloadString(payload, "conversation_id")
    return !projectId.isNullOrBlank() &&
        !conversationId.isNullOrBlank() &&
        payloadProjectId == projectId &&
        payloadConversationId == conversationId
}

private fun completionKey(projectId: String?, conversationId: String?): String? {
    if (projectId.isNullOrBlank() || conversationId.isNullOrBlank()) return null
    return "$projectId:$conversationId"
}

private fun markTaskCompletionNotified(
    prefs: SharedPreferences,
    projectId: String?,
    conversationId: String?
) {
    val key = completionKey(projectId, conversationId) ?: return
    prefs.edit()
        .putString(PREF_RECENT_TASK_COMPLETION_KEY, key)
        .putLong(PREF_RECENT_TASK_COMPLETION_AT, System.currentTimeMillis())
        .apply()
}

private fun wasTaskCompletionRecentlyNotified(
    prefs: SharedPreferences,
    projectId: String?,
    conversationId: String?
): Boolean {
    val key = completionKey(projectId, conversationId) ?: return false
    if (prefs.getString(PREF_RECENT_TASK_COMPLETION_KEY, null) != key) return false
    val notifiedAt = prefs.getLong(PREF_RECENT_TASK_COMPLETION_AT, 0L)
    return notifiedAt > 0 && System.currentTimeMillis() - notifiedAt <= RECENT_TASK_COMPLETION_WINDOW_MS
}

private fun taskCompletionNotificationText(message: String, apkUrl: String?): String {
    if (!apkUrl.isNullOrBlank()) return "会话已完成，APK 可以下载测试。"
    val text = message.replace('\n', ' ').trim()
    return if (text.length <= 80) text.ifBlank { "点击查看会话结果。" } else text.take(80) + "..."
}
