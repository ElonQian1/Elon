package com.elon.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

private const val CHAT_REALTIME_CHANNEL_ID = "chat_realtime_keepalive"
private const val CHAT_REALTIME_NOTIFICATION_ID = 2303

class ChatRealtimeService : Service() {
    private val summaryPoller by lazy { SocialSummaryPoller(this) }

    override fun onCreate() {
        super.onCreate()
        createChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        startForeground(CHAT_REALTIME_NOTIFICATION_ID, buildNotification())
        (application as? ElonApplication)?.globalWs?.start(this)
        summaryPoller.start()
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        summaryPoller.stop()
        super.onDestroy()
    }

    private fun createChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = getSystemService(NotificationManager::class.java) ?: return
        manager.createNotificationChannel(
            NotificationChannel(
                CHAT_REALTIME_CHANNEL_ID,
                "\u804a\u5929\u5b9e\u65f6\u540c\u6b65",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "\u4fdd\u6301\u597d\u53cb\u548c\u7fa4\u804a\u6d88\u606f\u5b9e\u65f6\u540c\u6b65"
                setShowBadge(false)
            }
        )
    }

    private fun buildNotification(): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        return NotificationCompat.Builder(this, CHAT_REALTIME_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("\u4e00\u9f99\u804a\u5929\u540c\u6b65\u4e2d")
            .setContentText("\u540e\u53f0\u63a5\u6536\u597d\u53cb\u548c\u7fa4\u804a\u65b0\u6d88\u606f")
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setShowWhen(false)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    companion object {
        fun ensureRunning(context: Context) {
            val appContext = context.applicationContext
            runCatching {
                ContextCompat.startForegroundService(
                    appContext,
                    Intent(appContext, ChatRealtimeService::class.java)
                )
            }
        }
    }
}
