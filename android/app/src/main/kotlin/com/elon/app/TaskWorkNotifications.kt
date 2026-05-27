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
import android.net.Uri
import android.os.Build
import android.os.Bundle
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.json.JSONObject

private const val PREF_NOTIFICATION_PERMISSION_ASKED = "notification_permission_asked"

internal fun setupTaskCompletionAlerts(activity: Activity, prefs: SharedPreferences, requestCode: Int) {
    createTaskWorkNotificationChannels(activity)
    requestTaskNotificationPermissionIfNeeded(activity, prefs, requestCode)
}

internal fun clearCompletedTaskBadge(context: Context, prefs: SharedPreferences) {
    prefs.edit().putInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, 0).apply()
    NotificationManagerCompat.from(context).cancel(TaskWorkService.TASK_COMPLETE_NOTIFICATION_ID)
    updateLauncherBadgeCount(context, 0)
}

internal fun createTaskWorkNotificationChannels(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
    val notificationManager = context.getSystemService(NotificationManager::class.java)
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
            NotificationManager.IMPORTANCE_DEFAULT
        ).apply {
            description = "后台任务完成后显示桌面角标"
            setShowBadge(true)
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
        .setContentText("后台保活连接中，切到其他应用也会继续同步进度。")
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
        .setContentText(etaText?.let { "预计还有 $it，后台连接保持中" } ?: "后台连接保持中")
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
    success: Boolean
) {
    val count = prefs.getInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, 0).coerceAtLeast(0) + 1
    prefs.edit().putInt(TaskWorkService.PREF_COMPLETED_TASK_BADGE_COUNT, count).apply()
    updateLauncherBadgeCount(context, count)
    showTaskCompletedNotification(context, count, wasDevelopment, apkUrl, success)
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
    success: Boolean
) {
    if (!canPostNotifications(context)) return

    val title = when {
        !success -> "任务需要处理"
        wasDevelopment -> "开发任务已完成"
        else -> "任务已完成"
    }
    val text = if (apkUrl != null) {
        "已有 $count 个任务完成，APK 可以下载测试。"
    } else {
        "已有 $count 个任务完成，点击查看结果。"
    }
    val notification = NotificationCompat.Builder(context, TaskWorkService.TASK_COMPLETE_CHANNEL_ID)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle(title)
        .setContentText(text)
        .setNumber(count)
        .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
        .setContentIntent(mainActivityPendingIntent(context))
        .setAutoCancel(true)
        .setOnlyAlertOnce(true)
        .setSilent(true)
        .setCategory(NotificationCompat.CATEGORY_STATUS)
        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        .build()
    runCatching {
        NotificationManagerCompat.from(context).notify(TaskWorkService.TASK_COMPLETE_NOTIFICATION_ID, notification)
    }
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
    return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
        ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED
}

private fun updateLauncherBadgeCount(context: Context, count: Int) {
    val badge = count.coerceAtLeast(0)
    val payload = Bundle().apply {
        putString("package", context.packageName)
        putString("class", MainActivity::class.java.name)
        putInt("badgenumber", badge)
    }
    listOf(
        "content://com.huawei.android.launcher.settings/badge/",
        "content://com.hihonor.android.launcher.settings/badge/"
    ).forEach { badgeUri ->
        runCatching {
            context.contentResolver.call(Uri.parse(badgeUri), "change_badge", null, payload)
        }
    }
}
