package com.elon.app

import android.Manifest
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.SystemClock
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import kotlin.math.abs

private const val CHAT_MESSAGE_GROUP_KEY = "com.elon.app.CHAT_MESSAGES"

internal object ChatMessageNotifications {
    private val policy = ChatNotificationPolicy()
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
        ChatNotificationChannel.create(context)
    }

    @Synchronized
    fun showFriendMessage(
        context: Context,
        fromUserId: String,
        messageId: String,
        content: String,
        senderName: String? = null,
        createdAt: String? = null
    ) {
        if (fromUserId.isBlank()) return
        if (fromUserId == AuthManager.userId(context)) return
        rememberConversationName(context, "friend", fromUserId, senderName)
        val conversation = conversationKey(context, "friend", fromUserId)
        val decision = policy.receive(conversation, messageId, createdAt, content,
            appInForeground && visibleFriendId == fromUserId, false, SystemClock.elapsedRealtime())
        if (!decision.show) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            conversation = conversation,
            title = conversationName(context, "friend", fromUserId) ?: "\u597d\u53cb\u6d88\u606f",
            text = messagePreview(content),
            summary = "\u6536\u5230\u4e00\u6761\u597d\u53cb\u6d88\u606f",
            requestKey = "friend:$fromUserId",
            badgeCount = badgeCount,
            alert = decision.alert
        )
    }

    @Synchronized
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
        rememberConversationName(context, "group", groupId, groupName)
        val conversation = conversationKey(context, "group", groupId)
        val decision = policy.receive(conversation, messageId, createdAt, content,
            appInForeground && visibleGroupId == groupId, true, SystemClock.elapsedRealtime())
        if (!decision.show) return
        val badgeCount = incrementChatLauncherBadgeCount(context)
        showMessageNotification(
            context = context,
            conversation = conversation,
            title = conversationName(context, "group", groupId) ?: "\u7fa4\u804a\u6d88\u606f",
            text = senderName
                ?.takeIf { it.isNotBlank() }
                ?.let { "$it\uff1a${messagePreview(content)}" }
                ?: messagePreview(content),
            summary = "\u6536\u5230\u4e00\u6761\u7fa4\u804a\u6d88\u606f",
            requestKey = "group:$groupId",
            badgeCount = badgeCount,
            alert = decision.alert
        )
    }

    private fun showMessageNotification(
        context: Context,
        conversation: String,
        title: String,
        text: String,
        summary: String,
        requestKey: String,
        badgeCount: Int,
        alert: Boolean
    ) {
        if (!canPostNotifications(context)) return
        createChannel(context)
        val soundUri = ChatNotificationChannel.soundUri(context)
        val pendingIntent = PendingIntent.getActivity(
            context,
            stableNotificationId(300_000, requestKey),
            Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        val notification = NotificationCompat.Builder(context, ChatNotificationChannel.ID)
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
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setPriority(NotificationCompat.PRIORITY_MAX)
            .setSound(soundUri)
            .setVibrate(longArrayOf(0L, 60L))
            .setSilent(!alert)
            .build()
        runCatching {
            // A tag per account/conversation avoids hash collisions and card-per-message spam.
            NotificationManagerCompat.from(context).notify(conversation, 1, notification)
        }
    }

    private fun messagePreview(content: String): String {
        com.elon.app.articles.ArticleApi.reference(content)?.let { return "[文章] ${it.optString("title").take(120)}" }
        val text = content.trim()
        if (text.isBlank()) return "\u6536\u5230\u4e00\u6761\u65b0\u6d88\u606f"
        return text.take(80)
    }

    private fun conversationKey(context: Context, kind: String, id: String): String =
        "chat:${AuthManager.userId(context).orEmpty()}:$kind:$id"

    fun rememberConversationName(context: Context, kind: String, id: String, name: String?) {
        if (id.isBlank() || name.isNullOrBlank()) return
        val prefs = namePrefs(context)
        val key = conversationKey(context, kind, id)
        val value = name.trim()
        if (prefs.getString(key, null) != value) prefs.edit().putString(key, value).apply()
    }

    private fun conversationName(context: Context, kind: String, id: String): String? =
        namePrefs(context).getString(conversationKey(context, kind, id), null)

    private fun namePrefs(context: Context) =
        context.applicationContext.getSharedPreferences("chat_notification_names", Context.MODE_PRIVATE)

    private fun stableNotificationId(base: Int, key: String): Int {
        return base + abs(key.hashCode() % 90_000)
    }

    private fun canPostNotifications(context: Context): Boolean {
        if (!NotificationManagerCompat.from(context).areNotificationsEnabled()) return false
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
    }

}
