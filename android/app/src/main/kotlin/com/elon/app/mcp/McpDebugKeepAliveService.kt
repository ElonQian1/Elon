package com.elon.app.mcp

import com.elon.app.*
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
import org.json.JSONObject

class McpDebugKeepAliveService : Service() {
    private val prefs by lazy { getSharedPreferences("elon", MODE_PRIVATE) }
    private var foregroundStarted = false

    override fun onCreate() {
        super.onCreate()
        DebugTraceStore.init(this)
        createChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            saveEnabled(this, false)
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
            return START_NOT_STICKY
        }
        if (intent?.action == ACTION_START) saveEnabled(this, true)
        if (!shouldAutoStart(this)) { stopSelf(); return START_NOT_STICKY }
        if (foregroundStarted) return START_STICKY
        McpDebugServer.start(this)
        prefs.edit()
            .putBoolean(PREF_MANUAL_STOPPED, false)
            .putBoolean(PREF_ACTIVE, true)
            .putLong(PREF_STARTED_AT, System.currentTimeMillis())
            .apply()
        DebugTraceStore.record("mcp_keepalive_started")
        startForeground(NOTIFICATION_ID, buildNotification())
        foregroundStarted = true
        running = true
        return START_STICKY
    }

    override fun onDestroy() {
        running = false
        stopForeground(STOP_FOREGROUND_REMOVE)
        prefs.edit()
            .putBoolean(PREF_ACTIVE, false)
            .remove(PREF_STARTED_AT)
            .apply()
        DebugTraceStore.record("mcp_keepalive_stopped")
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun buildNotification() =
        NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle("一龙 MCP 调试已开启")
            .setContentText("后台调试连接可用 · 调试结束后可关闭")
            .setContentIntent(mainActivityPendingIntent())
            .addAction(0, "结束后台调试", PendingIntent.getService(this, NOTIFICATION_ID + 1,
                Intent(this, McpDebugKeepAliveService::class.java).setAction(ACTION_STOP),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE))
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setSilent(true)
            .setShowWhen(false)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

    private fun mainActivityPendingIntent(): PendingIntent {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        return PendingIntent.getActivity(
            this,
            3,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
    }

    private fun createChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_ID,
                "MCP 调试保活",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "调试 APK 时保持 MCP 服务在后台可访问"
                setShowBadge(false)
                enableVibration(false)
                setSound(null, null)
            }
        )
    }

    companion object {
        const val ACTION_START = "com.elon.app.mcp.START_KEEPALIVE"
        const val ACTION_STOP = "com.elon.app.mcp.STOP_KEEPALIVE"
        const val CHANNEL_ID = "mcp_debug_keepalive"
        const val NOTIFICATION_ID = 2500
        const val PREF_ACTIVE = "mcp_keepalive_active"
        const val PREF_STARTED_AT = "mcp_keepalive_started_at"
        const val PREF_MANUAL_STOPPED = "mcp_keepalive_manual_stopped"
        private const val PREF_ENABLED = "mcp_keepalive_enabled_v2"
        private const val ACTION_RESUME = "com.elon.app.mcp.RESUME_KEEPALIVE"
        @Volatile private var running = false

        fun shouldAutoStart(context: Context): Boolean {
            val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
            return prefs.getBoolean(PREF_ENABLED, false) && !prefs.getBoolean(PREF_MANUAL_STOPPED, false)
        }

        private fun saveEnabled(context: Context, enabled: Boolean) {
            context.getSharedPreferences("elon", Context.MODE_PRIVATE).edit()
                .putBoolean(PREF_ENABLED, enabled).putBoolean(PREF_MANUAL_STOPPED, !enabled).apply()
        }

        fun requestStart(context: Context) {
            saveEnabled(context, true)
            resumeIfEnabled(context)
        }

        fun resumeIfEnabled(context: Context) {
            if (shouldAutoStart(context) && !running) ContextCompat.startForegroundService(context,
                Intent(context, McpDebugKeepAliveService::class.java).setAction(ACTION_RESUME))
        }

        fun requestStop(context: Context) {
            saveEnabled(context, false)
            running = false
            context.getSharedPreferences("elon", Context.MODE_PRIVATE).edit()
                .putBoolean(PREF_ACTIVE, false).remove(PREF_STARTED_AT).apply()
            context.stopService(Intent(context, McpDebugKeepAliveService::class.java))
        }

        fun statusJson(context: Context): JSONObject {
            val prefs = context.getSharedPreferences("elon", Context.MODE_PRIVATE)
            val startedAt = prefs.getLong(PREF_STARTED_AT, 0L)
            return JSONObject()
                .put("active", prefs.getBoolean(PREF_ACTIVE, false))
                .put("auto_start_enabled", shouldAutoStart(context))
                .put("started_at_ms", if (startedAt > 0L) startedAt else JSONObject.NULL)
                .put(
                    "age_ms",
                    if (startedAt > 0L) System.currentTimeMillis() - startedAt else JSONObject.NULL
                )
        }
    }
}
