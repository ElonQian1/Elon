package com.elon.app

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.os.Build
import android.provider.Settings
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import kotlin.math.abs

private const val CHAT_MESSAGE_CHANNEL_ID = "chat_messages_v3_sound"
private const val MAX_DEDUPED_CHAT_MESSAGES = 160

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
            setSound(Settings.System.DEFAULT_NOTIFICATION_URI, soundAttributes)
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
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(100_000, fromUserId),
            title = senderName?.takeIf { it.isNotBlank() } ?: "好友消息",
            text = messagePreview(content),
            summary = "收到一条好友消息",
            requestKey = "friend:$fromUserId"
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
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(200_000, groupId),
            title = groupName?.takeIf { it.isNotBlank() } ?: "群聊消息",
            text = senderName
                ?.takeIf { it.isNotBlank() }
                ?.let { "$it：${messagePreview(content)}" }
                ?: messagePreview(content),
            summary = "收到一条群聊消息",
            requestKey = "group:$groupId"
        )
    }

    private fun showMessageNotification(
        context: Context,
        notificationId: Int,
        title: String,
        text: String,
        summary: String,
        requestKey: String
    ) {
        if (!canPostNotifications(context)) return
        createChannel(context)
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
            .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
            .setCategory(NotificationCompat.CATEGORY_MESSAGE)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setDefaults(NotificationCompat.DEFAULT_ALL)
            .setSound(Settings.System.DEFAULT_NOTIFICATION_URI)
            .setVibrate(longArrayOf(0L, 220L, 120L, 220L))
            .build()
        runCatching {
            NotificationManagerCompat.from(context).notify(notificationId, notification)
        }
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
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
    }
}
