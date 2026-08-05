package com.elon.app.update

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.pm.ServiceInfo
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.ForegroundInfo
import com.elon.app.MainActivity
import com.elon.app.R
import com.elon.app.TaskWorkService

internal object AppUpdateNotifications {
    private const val CHANNEL_ALERTS = "app_update_alerts"
    private const val CHANNEL_DOWNLOADS = "app_update_downloads"
    private const val ALERT_NOTIFICATION_ID = 2402
    private const val DOWNLOAD_NOTIFICATION_ID = 2403

    fun createChannels(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ALERTS,
                "应用更新提醒",
                NotificationManager.IMPORTANCE_DEFAULT,
            ).apply {
                description = "一龙 APP 有新版本时提醒"
                setShowBadge(true)
            }
        )
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_DOWNLOADS,
                "应用更新下载",
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = "在后台继续下载一龙 APP 更新"
                setShowBadge(false)
            }
        )
    }

    fun notifyAvailable(context: Context, version: AppUpdateVersion) {
        if (!canPostNotifications(context)) return
        createChannels(context)
        val text = version.changelog.ifBlank { "点击查看更新内容并在后台下载" }
        val notification = NotificationCompat.Builder(context, CHANNEL_ALERTS)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("一龙有新版本 v${version.versionName}")
            .setContentText(text)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setContentIntent(openUpdatePendingIntent(context))
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(context).notify(ALERT_NOTIFICATION_ID, notification)
        }
    }

    fun foregroundInfo(context: Context, snapshot: AppUpdateSnapshot): ForegroundInfo {
        createChannels(context)
        return ForegroundInfo(
            DOWNLOAD_NOTIFICATION_ID,
            downloadNotification(context, snapshot),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
        )
    }

    fun notifyProgress(context: Context, snapshot: AppUpdateSnapshot) {
        createChannels(context)
        runCatching {
            NotificationManagerCompat.from(context).notify(
                DOWNLOAD_NOTIFICATION_ID,
                downloadNotification(context, snapshot),
            )
        }
    }

    fun notifyReady(context: Context, snapshot: AppUpdateSnapshot) {
        createChannels(context)
        val notification = NotificationCompat.Builder(context, CHANNEL_DOWNLOADS)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("v${snapshot.versionName} 已下载并校验")
            .setContentText("点击返回一龙安装更新")
            .setContentIntent(openUpdatePendingIntent(context))
            .setAutoCancel(true)
            .setOnlyAlertOnce(false)
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(context).notify(DOWNLOAD_NOTIFICATION_ID, notification)
        }
    }

    fun notifyFailed(context: Context, snapshot: AppUpdateSnapshot) {
        if (!canPostNotifications(context)) return
        createChannels(context)
        val notification = NotificationCompat.Builder(context, CHANNEL_DOWNLOADS)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("v${snapshot.versionName} 下载未完成")
            .setContentText(snapshot.errorMessage.ifBlank { "点击重试" })
            .setContentIntent(openUpdatePendingIntent(context))
            .setAutoCancel(true)
            .setOnlyAlertOnce(false)
            .setCategory(NotificationCompat.CATEGORY_ERROR)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .build()
        runCatching {
            NotificationManagerCompat.from(context).notify(DOWNLOAD_NOTIFICATION_ID, notification)
        }
    }

    fun cancelDownloadNotification(context: Context) {
        NotificationManagerCompat.from(context).cancel(DOWNLOAD_NOTIFICATION_ID)
    }

    private fun downloadNotification(context: Context, snapshot: AppUpdateSnapshot): Notification {
        val indeterminate = snapshot.totalBytes <= 0L
        val detail = if (indeterminate) {
            snapshot.sourceName.ifBlank { "正在连接下载源" }
        } else {
            "${snapshot.progressPercent}% · ${formatUpdateBytes(snapshot.downloadedBytes)} / " +
                formatUpdateBytes(snapshot.totalBytes)
        }
        return NotificationCompat.Builder(context, CHANNEL_DOWNLOADS)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle("正在后台下载 v${snapshot.versionName}")
            .setContentText(detail)
            .setProgress(100, snapshot.progressPercent, indeterminate)
            .setContentIntent(openUpdatePendingIntent(context))
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun openUpdatePendingIntent(context: Context): PendingIntent {
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
            putExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE, true)
        }
        return PendingIntent.getActivity(
            context,
            2402,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    private fun canPostNotifications(context: Context): Boolean {
        if (!NotificationManagerCompat.from(context).areNotificationsEnabled()) return false
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
    }
}
