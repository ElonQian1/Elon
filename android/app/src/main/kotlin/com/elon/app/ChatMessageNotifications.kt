package com.elon.app

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioManager
import android.media.RingtoneManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import kotlin.math.abs

private const val CHAT_MESSAGE_CHANNEL_ID = "chat_messages_v5_loud_badge"
private const val CHAT_MESSAGE_GROUP_KEY = "com.elon.app.CHAT_MESSAGES"
private const val MAX_DEDUPED_CHAT_MESSAGES = 160
private const val FALLBACK_RING_MS = 1600L

internal object ChatMessageNotifications {
    private val shownMessageKeys = LinkedHashSet<String>()
    @Volatile private var appInForeground = false
    @Volatile private var visibleFriendId: String? = null
    @Volatile private var visibleGroupId: String? = null

    fun setVisibleConversation(
        foreground: Boolean,
        friendId: String?,
        groupId: String?
    ) {
        appInForeground = foreground
        visibleFriendId = friendId?.takeIf { it.isNotBlank() }
        visibleGroupId = groupId?.takeIf { it.isNotBlank() }
    }

    fun createChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        val soundUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
        val soundAttributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_NOTIFICATION)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()
        val channel = NotificationChannel(
            CHAT_MESSAGE_CHANNEL_ID,
            "聊天消息提醒",
            NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = "好友和群聊新消息提醒"
            setShowBadge(true)
            enableVibration(true)
            setSound(soundUri, soundAttributes)
        }
        manager.createNotificationChannel(channel)
    }

    fun showFriendMessage(
        context: Context,
        fromUserId: String,
        messageId: String,
        content: String,
        senderName: String? = null
    ) {
        if (fromUserId.isBlank()) return
        if (appInForeground && visibleFriendId == fromUserId) return
        val key = "friend:${messageId.ifBlank { "${fromUserId}:${content.hashCode()}" }}"
        if (!markMessageShown(key)) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(100_000, key),
            title = senderName?.takeIf { it.isNotBlank() } ?: "好友消息",
            text = messagePreview(content),
            summary = "收到一条好友消息",
            requestKey = "friend:$fromUserId",
            badgeCount = badgeCount
        )
    }

    fun showGroupMessage(
        context: Context,
        groupId: String,
        fromUserId: String,
        messageId: String,
        content: String,
        senderName: String? = null,
        groupName: String? = null
    ) {
        if (groupId.isBlank()) return
        if (fromUserId == AuthManager.userId(context)) return
        if (appInForeground && visibleGroupId == groupId) return
        val key = "group:${messageId.ifBlank { "${groupId}:${fromUserId}:${content.hashCode()}" }}"
        if (!markMessageShown(key)) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(200_000, key),
            title = groupName?.takeIf { it.isNotBlank() } ?: "群聊消息",
            text = senderName
                ?.takeIf { it.isNotBlank() }
                ?.let { "$it：${messagePreview(content)}" }
                ?: messagePreview(content),
            summary = "收到一条群聊消息",
            requestKey = "group:$groupId",
            badgeCount = badgeCount
        )
    }

    private fun showMessageNotification(
        context: Context,
        notificationId: Int,
        title: String,
        text: String,
        summary: String,
        requestKey: String,
        badgeCount: Int
    ) {
        if (!canPostNotifications(context)) return
        createChannel(context)
        val soundUri = RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION)
        val pendingIntent = PendingIntent.getActivity(
            context,
            stableNotificationId(300_000, requestKey),
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification = NotificationCompat.Builder(context, CHAT_MESSAGE_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification_task_done)
            .setContentTitle(title)
            .setContentText(text)
            .setTicker(summary)
            .setStyle(NotificationCompat.BigTextStyle().bigText(text))
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .setOnlyAlertOnce(false)
            .setShowWhen(true)
            .setWhen(System.currentTimeMillis())
            .setNumber(badgeCount.coerceAtLeast(1))
            .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setGroup(CHAT_MESSAGE_GROUP_KEY)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setDefaults(NotificationCompat.DEFAULT_SOUND or NotificationCompat.DEFAULT_VIBRATE)
            .setSound(soundUri)
            .setVibrate(longArrayOf(0L, 220L, 120L, 220L))
            .build()
        runCatching {
            NotificationManagerCompat.from(context).notify(notificationId, notification)
        }
        playFallbackMessageSound(context)
    }

    private fun messagePreview(content: String): String {
        val text = content.trim()
        if (text.isBlank()) return "收到一条新消息"
        return text.take(80)
    }

    @Synchronized
    private fun markMessageShown(key: String): Boolean {
        val added = shownMessageKeys.add(key)
        while (shownMessageKeys.size > MAX_DEDUPED_CHAT_MESSAGES) {
            shownMessageKeys.remove(shownMessageKeys.first())
        }
        return added
    }

    private fun stableNotificationId(base: Int, key: String): Int {
        return base + abs(key.hashCode() % 90_000)
    }

    private fun canPostNotifications(context: Context): Boolean {
        if (!NotificationManagerCompat.from(context).areNotificationsEnabled()) return false
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
    }

    private fun playFallbackMessageSound(context: Context) {
        if (appInForeground) return
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
        }, FALLBACK_RING_MS)
    }
}
