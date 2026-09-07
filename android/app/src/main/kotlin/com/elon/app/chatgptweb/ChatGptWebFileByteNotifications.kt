package com.elon.app.chatgptweb

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.SystemClock
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.elon.app.R

internal class ChatGptWebFileByteNotifications(
    private val context: Context,
    private val lease: ChatGptWebFileDownloadLease.Value,
    private val onCancel: () -> Unit,
) {
    private val manager = context.getSystemService(NotificationManager::class.java)
    private val action = context.packageName + ".CHATGPT_CANCEL_FILE_DOWNLOAD." + lease.id
    private var registered = false
    private var lastProgressAt = 0L
    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == action && intent.getStringExtra("leaseId") == lease.id) onCancel()
        }
    }

    fun start() {
        runCatching {
            manager?.createNotificationChannel(NotificationChannel(CHANNEL, "文件下载", NotificationManager.IMPORTANCE_LOW))
            ContextCompat.registerReceiver(context, receiver, IntentFilter(action), ContextCompat.RECEIVER_NOT_EXPORTED)
            registered = true
        }
        progress(0, -1)
    }

    fun progress(received: Long, expected: Long) {
        val now = SystemClock.elapsedRealtime()
        if (lastProgressAt != 0L && now - lastProgressAt < 1_000) return
        lastProgressAt = now
        val builder = base().setOngoing(true).setOnlyAlertOnce(true).setContentText("正在下载")
            .setProgress(100, if (expected > 0) (received * 100 / expected).coerceIn(0, 100).toInt() else 0, expected < 0)
        if (registered) {
            val intent = Intent(action).setPackage(context.packageName).putExtra("leaseId", lease.id)
            val cancel = PendingIntent.getBroadcast(context, lease.id.hashCode(), intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            builder.addAction(android.R.drawable.ic_menu_close_clear_cancel, "取消", cancel)
        }
        show(builder)
    }

    fun saved(uri: Uri?) {
        release()
        val builder = base().setContentText("已保存到下载目录").setAutoCancel(true)
        if (uri != null) {
            val intent = Intent(Intent.ACTION_VIEW).setDataAndType(uri, lease.mediaType.ifBlank { "application/octet-stream" })
                .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            builder.setContentIntent(PendingIntent.getActivity(context, lease.id.hashCode(), intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE))
        }
        show(builder)
    }

    fun failed() { release(); show(base().setContentText("下载未完成，请重试").setAutoCancel(true)) }
    fun cancel() { release(); runCatching { manager?.cancel(lease.id, NOTIFICATION_ID) } }

    private fun release() {
        if (registered) runCatching { context.unregisterReceiver(receiver) }
        registered = false
    }
    private fun base() = NotificationCompat.Builder(context, CHANNEL)
        .setSmallIcon(R.drawable.ic_notification_task_done)
        .setContentTitle(lease.name).setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
    private fun show(builder: NotificationCompat.Builder) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) return
        runCatching { manager?.notify(lease.id, NOTIFICATION_ID, builder.build()) }
    }

    private companion object {
        const val CHANNEL = "chatgpt_file_downloads"
        const val NOTIFICATION_ID = 4187
    }
}
