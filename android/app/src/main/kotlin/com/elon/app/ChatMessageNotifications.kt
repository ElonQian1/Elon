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
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import kotlin.math.abs

private const val CHAT_MESSAGE_CHANNEL_ID = "chat_messages_v7_heads_up_loud"
private const val CHAT_MESSAGE_GROUP_KEY = "com.elon.app.CHAT_MESSAGES"
private const val MAX_DEDUPED_CHAT_MESSAGES = 160
private const val FALLBACK_RING_MS = 1600L
private const val FOREGROUND_HINT_VIBRATE_MS = 60L
private const val FOREGROUND_SOUND_MIN_INTERVAL_MS = 1500L

internal object ChatMessageNotifications {
    private val shownMessageKeys = LinkedHashSet<String>()
    @Volatile private var appInForeground = false
    @Volatile private var visibleFriendId: String? = null
    @Volatile private var visibleGroupId: String? = null
    @Volatile private var lastForegroundHintAtMs = 0L

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
        playForegroundHintIfNeeded(context)
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
        playForegroundHintIfNeeded(context)
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
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_ALL)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setPriority(NotificationCompat.PRIORITY_MAX)
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

    /**
     * 前台收到非当前会话的消息时，主动播放一次系统提示音 + 短震动。
     *
     * 解决问题：通知通道虽然是 IMPORTANCE_HIGH，但部分厂商在前台时会把声音压制得很弱、
     * 甚至直接静默，用户感受不到"叮"的提醒。这里像微信一样在前台主动响一下。
     *
     * 注意：仅在前台 + 不在静音模式时触发；与系统通知音叠加时间隔 1.5 秒去重。
     */
    private fun playForegroundHintIfNeeded(context: Context) {
        if (!appInForeground) return
        val now = System.currentTimeMillis()
        if (now - lastForegroundHintAtMs < FOREGROUND_SOUND_MIN_INTERVAL_MS) return
        lastForegroundHintAtMs = now
        playForegroundSound(context)
        triggerForegroundVibration(context)
    }

    private fun playForegroundSound(context: Context) {
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

    private fun triggerForegroundVibration(context: Context) {
        val appContext = context.applicationContext
        val vibrator = obtainVibrator(appContext) ?: return
        runCatching {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                vibrator.vibrate(VibrationEffect.createOneShot(FOREGROUND_HINT_VIBRATE_MS, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                vibrator.vibrate(FOREGROUND_HINT_VIBRATE_MS)
            }
        }
    }

    private fun obtainVibrator(context: Context): Vibrator? {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            (context.getSystemService(VibratorManager::class.java))?.defaultVibrator
        } else {
            @Suppress("DEPRECATION")
            context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
        }
    }
}
