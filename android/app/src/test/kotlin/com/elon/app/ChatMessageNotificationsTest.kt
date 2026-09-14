package com.elon.app

import android.Manifest
import android.app.Application
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.net.Uri
import android.media.AudioAttributes
import android.media.RingtoneManager
import androidx.core.app.NotificationCompat
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.Config

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34], manifest = Config.NONE, application = Application::class)
class ChatMessageNotificationsTest {
    private val context get() = RuntimeEnvironment.getApplication()
    private val manager get() = context.getSystemService(NotificationManager::class.java)

    @Before fun reset() {
        shadowOf(context).grantPermissions(Manifest.permission.POST_NOTIFICATIONS)
        ChatMessageNotifications.setVisibleConversation(false, null, null)
        manager.cancelAll()
    }

    @Test fun messagesInOneGroupReplaceOneCardAndBurstIsSilent() {
        ChatMessageNotifications.showGroupMessage(context, "group-a", "sender", "g1", "first", createdAt = "t1")
        val first = manager.activeNotifications.single()
        assertEquals(ChatNotificationChannel.ID, first.notification.channelId)
        assertEquals(ChatNotificationChannel.soundUri(context), manager.getNotificationChannel(first.notification.channelId).sound)
        ChatMessageNotifications.showGroupMessage(context, "group-a", "sender", "g2", "second", createdAt = "t2")
        val next = manager.activeNotifications.single()
        assertEquals(first.tag, next.tag)
        assertEquals("second", next.notification.extras.getString(Notification.EXTRA_TEXT))
        assertNull(next.notification.sound)
        assertNull(next.notification.vibrate)
        assertEquals(NotificationCompat.GROUP_ALERT_SUMMARY, next.notification.groupAlertBehavior)
    }

    @Test fun separateConversationsKeepSeparateCards() {
        ChatMessageNotifications.showFriendMessage(context, "friend-a", "f1", "first")
        ChatMessageNotifications.showFriendMessage(context, "friend-b", "f2", "second")
        assertEquals(2, manager.activeNotifications.size)
    }

    @Test fun summaryAfterRealtimeDoesNotReplaceOrAddAnotherNotification() {
        ChatMessageNotifications.showFriendMessage(context, "friend-c", "f3", "full content", createdAt = "2026-09-15T00:00:00Z")
        ChatMessageNotifications.showFriendMessage(context, "friend-c", "", "short preview", createdAt = "2026-09-15T00:00:00Z")
        assertEquals("full content", manager.activeNotifications.single().notification.extras.getString(Notification.EXTRA_TEXT))
    }

    @Test fun visibleConversationStaysQuietEvenAfterSwitchingAwayAndPolling() {
        ChatMessageNotifications.setVisibleConversation(true, "friend-d", null)
        ChatMessageNotifications.showFriendMessage(context, "friend-d", "f4", "read", createdAt = "t4")
        ChatMessageNotifications.setVisibleConversation(false, null, null)
        ChatMessageNotifications.showFriendMessage(context, "friend-d", "", "read", createdAt = "t4")
        assertTrue(manager.activeNotifications.isEmpty())
    }

    @Test fun cachedGroupNameIsUsedWhenRealtimeEventOmitsIt() {
        ChatMessageNotifications.rememberConversationName(context, "group", "group-b", "Test group")
        ChatMessageNotifications.showGroupMessage(context, "group-b", "sender", "g3", "hello")
        assertEquals("Test group", manager.activeNotifications.single().notification.extras.getString(Notification.EXTRA_TITLE))
    }

    @Test fun defaultLegacySoundMigratesToSoftSingleTone() {
        legacyChannel(NotificationManager.IMPORTANCE_HIGH, RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION))
        ChatNotificationChannel.create(context)
        assertEquals(ChatNotificationChannel.soundUri(context), manager.getNotificationChannel(ChatNotificationChannel.ID).sound)
    }

    @Test fun blockedAndSilentLegacySettingsArePreserved() {
        legacyChannel(NotificationManager.IMPORTANCE_NONE, null, vibrate = false)
        ChatNotificationChannel.create(context)
        val migrated = manager.getNotificationChannel(ChatNotificationChannel.ID)
        assertEquals(NotificationManager.IMPORTANCE_NONE, migrated.importance)
        assertNull(migrated.sound)
        assertFalse(migrated.shouldVibrate())
    }

    @Test fun customSoundAndNewChannelSettingsAreNeverReset() {
        val selected = Uri.parse("content://media/internal/audio/media/42")
        legacyChannel(NotificationManager.IMPORTANCE_LOW, selected, vibrate = false)
        ChatNotificationChannel.create(context)
        ChatNotificationChannel.create(context)
        val migrated = manager.getNotificationChannel(ChatNotificationChannel.ID)
        assertEquals(selected, migrated.sound)
        assertEquals(NotificationManager.IMPORTANCE_LOW, migrated.importance)
        assertFalse(migrated.shouldVibrate())
    }

    private fun legacyChannel(importance: Int, sound: Uri?, vibrate: Boolean = true) {
        manager.createNotificationChannel(NotificationChannel(ChatNotificationChannel.LEGACY_ID, "Chat", importance).apply {
            setSound(sound, AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION).build())
            enableVibration(vibrate)
        })
    }
}
