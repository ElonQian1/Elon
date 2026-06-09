package com.elon.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat

/**
 * 聊天后台保活服务。
 *
 * 目的：让 APK 切到后台后仍能稳定收到好友/群消息推送。
 *
 * 解决的问题：
 *  - 仅靠 [ElonApplication] 持有的 [GlobalWsManager] 在 Doze/省电模式下会被系统冻结，
 *    导致用户切到微信、锁屏一段时间后好友消息收不到。
 *  - 提供前台 dataSync 类型的常驻通知（IMPORTANCE_LOW、静默）后，进程优先级提升，
 *    WebSocket 长连接基本可以一直保持，让用户像微信一样能在后台收到消息提醒。
 *
 * 使用方式：
 *  - 用户登录成功后由 [ElonApplication] / [LoginActivity] 调用 [start]；
 *  - 用户登出时调用 [stop]；
 *  - 用户从设置页关闭"后台收消息"开关时调用 [stop]，开启时调用 [start]。
 *
 * 行为：
 *  - 启动时通过 [GlobalWsManager.start] 保证 WS 连上；
 *  - onDestroy 时不主动断 WS（应用本身仍可能在前台），仅释放服务自身的常驻通知。
 */
class ChatBackgroundService : Service() {

    override fun onCreate() {
        super.onCreate()
        ensureChannel(this)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        runCatching {
            startForeground(NOTIFICATION_ID, buildNotification(this), foregroundServiceTypeOrZero())
        }.onFailure { Log.w(TAG, "startForeground 失败: ${it.message}") }
        // 启动 WS（应用进程内已有则等价于 no-op，token 变化时会重连）
        runCatching { (application as? ElonApplication)?.globalWs?.start(this) }
        return START_STICKY
    }

    override fun onDestroy() {
        // 不主动 stop globalWs，让前台 Activity 决定连接生命周期
        super.onDestroy()
    }

    private fun foregroundServiceTypeOrZero(): Int {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
        } else {
            0
        }
    }

    companion object {
        private const val TAG = "ChatBgService"
        private const val NOTIFICATION_ID = 91_001
        const val CHANNEL_ID = "chat_background_keepalive_v1"

        /** 启动保活服务（已运行则立即返回）。仅在用户允许后台接收消息且已登录时调用。 */
        fun start(context: Context) {
            val ctx = context.applicationContext
            val intent = Intent(ctx, ChatBackgroundService::class.java)
            runCatching {
                ContextCompat.startForegroundService(ctx, intent)
            }.onFailure { Log.w(TAG, "启动保活服务失败: ${it.message}") }
        }

        /** 停止保活服务。 */
        fun stop(context: Context) {
            val ctx = context.applicationContext
            runCatching { ctx.stopService(Intent(ctx, ChatBackgroundService::class.java)) }
        }

        /** 创建保活通知通道（IMPORTANCE_LOW，无声音、无震动，避免打扰）。 */
        fun ensureChannel(context: Context) {
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
            val manager = context.getSystemService(NotificationManager::class.java) ?: return
            val existing = manager.getNotificationChannel(CHANNEL_ID)
            if (existing != null) return
            val channel = NotificationChannel(
                CHANNEL_ID,
                "保持消息在线",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "用于在后台保持与服务器的连接，及时接收好友消息"
                setShowBadge(false)
                enableVibration(false)
                setSound(null, null)
            }
            manager.createNotificationChannel(channel)
        }

        private fun buildNotification(context: Context): Notification {
            ensureChannel(context)
            val pendingIntent = PendingIntent.getActivity(
                context,
                NOTIFICATION_ID,
                Intent(context, MainActivity::class.java).apply {
                    flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
                },
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            return NotificationCompat.Builder(context, CHANNEL_ID)
                .setSmallIcon(R.drawable.ic_notification_task_done)
                .setContentTitle("一龙正在为你保持消息在线")
                .setContentText("点击打开应用 · 可在设置中关闭后台保活")
                .setContentIntent(pendingIntent)
                .setOngoing(true)
                .setOnlyAlertOnce(true)
                .setSilent(true)
                .setShowWhen(false)
                .setCategory(NotificationCompat.CATEGORY_SERVICE)
                .setPriority(NotificationCompat.PRIORITY_LOW)
                .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
                .build()
        }
    }
}
