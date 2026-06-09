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

private const val CHAT_MESSAGE_CHANNEL_ID = "chat_messages_v6_loud_badge"
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
            "\u804a\u5929\u6d88\u606f\u63d0\u9192",
            NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = "\u597d\u53cb\u548c\u7fa4\u804a\u65b0\u6d88\u606f\u63d0\u9192"
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
        senderName: String? = null,
        createdAt: String? = null
    ) {
        if (fromUserId.isBlank()) return
        if (appInForeground && visibleFriendId == fromUserId) return
        val primaryKey = "friend:${messageId.ifBlank { "$fromUserId:${content.hashCode()}" }}"
        val fingerprintKey = "friend:fingerprint:$fromUserId:${createdAt.orEmpty()}:${content.hashCode()}"
        if (!markMessageShown(primaryKey, fingerprintKey)) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(100_000, primaryKey),
            title = senderName?.takeIf { it.isNotBlank() } ?: "\u597d\u53cb\u6d88\u606f",
            text = messagePreview(content),
            summary = "\u6536\u5230\u4e00\u6761\u597d\u53cb\u6d88\u606f",
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
        groupName: String? = null,
        createdAt: String? = null
    ) {
        if (groupId.isBlank()) return
        if (fromUserId == AuthManager.userId(context)) return
        if (appInForeground && visibleGroupId == groupId) return
        val primaryKey = "group:${messageId.ifBlank { "$groupId:$fromUserId:${content.hashCode()}" }}"
        val fingerprintKey = "group:fingerprint:$groupId:${createdAt.orEmpty()}:${content.hashCode()}"
        if (!markMessageShown(primaryKey, fingerprintKey)) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            notificationId = stableNotificationId(200_000, primaryKey),
            title = groupName?.takeIf { it.isNotBlank() } ?: "\u7fa4\u804a\u6d88\u606f",
            text = senderName
                ?.takeIf { it.isNotBlank() }
                ?.let { "$it\uff1a${messagePreview(content)}" }
                ?: messagePreview(content),
            summary = "\u6536\u5230\u4e00\u6761\u7fa4\u804a\u6d88\u606f",
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
        if (text.isBlank()) return "\u6536\u5230\u4e00\u6761\u65b0\u6d88\u606f"
        return text.take(80)
    }

    @Synchronized
    private fun markMessageShown(vararg keys: String): Boolean {
        val normalized = keys.filter { it.isNotBlank() }
        if (normalized.any { shownMessageKeys.contains(it) }) return false
        normalized.forEach { shownMessageKeys.add(it) }
        while (shownMessageKeys.size > MAX_DEDUPED_CHAT_MESSAGES) {
            shownMessageKeys.remove(shownMessageKeys.first())
        }
        return true
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
