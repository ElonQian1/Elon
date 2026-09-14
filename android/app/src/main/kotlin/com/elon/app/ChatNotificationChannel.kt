package com.elon.app

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.net.Uri
import android.os.Build

internal object ChatNotificationChannel {
    const val ID = "chat_messages_v8_soft_single"
    const val LEGACY_ID = "chat_messages_v7_heads_up_loud"

    fun soundUri(context: Context): Uri =
        Uri.parse("android.resource://${context.packageName}/raw/chat_message_soft")

    fun create(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        if (manager.getNotificationChannel(ID) != null) return
        val legacy = manager.getNotificationChannel(LEGACY_ID)
        val attributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_NOTIFICATION)
            .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
            .build()
        val channel = NotificationChannel(
            ID, "聊天消息提醒", legacy?.importance ?: NotificationManager.IMPORTANCE_HIGH
        ).apply {
            description = "好友和群聊新消息，柔和单音提醒"
            setShowBadge(legacy?.canShowBadge() ?: true)
            vibrationPattern = legacy?.vibrationPattern ?: longArrayOf(0L, 60L)
            enableVibration(legacy?.shouldVibrate() ?: true)
            // A new channel is needed for a new default sound. Carry forward user
            // choices, including a silent/blocked legacy channel, instead of resetting them.
            val preserveSound = legacy != null && (
                legacy.sound != RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION) ||
                    (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && legacy.hasUserSetSound())
                )
            setSound(if (preserveSound) legacy?.sound else soundUri(context),
                if (preserveSound) legacy?.audioAttributes ?: attributes else attributes)
            if (legacy != null) {
                lockscreenVisibility = legacy.lockscreenVisibility
                enableLights(legacy.shouldShowLights())
                lightColor = legacy.lightColor
                setBypassDnd(legacy.canBypassDnd())
            }
        }
        manager.createNotificationChannel(channel)
    }
}
