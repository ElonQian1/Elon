package com.elon.app

import android.content.Context
import android.net.Uri
import android.os.Bundle
import me.leolin.shortcutbadger.ShortcutBadger

private const val LAUNCHER_BADGE_PREFS = "launcher_badges"
private const val KEY_CHAT_UNREAD_BADGE_COUNT = "chat_unread_badge_count"
private const val KEY_TASK_BADGE_COUNT = "task_badge_count"

@Synchronized
internal fun incrementChatLauncherBadgeCount(context: Context): Int {
    val prefs = launcherBadgePrefs(context)
    val next = (prefs.getInt(KEY_CHAT_UNREAD_BADGE_COUNT, 0).coerceAtLeast(0) + 1)
        .coerceAtMost(999)
    prefs.edit().putInt(KEY_CHAT_UNREAD_BADGE_COUNT, next).apply()
    applyCombinedLauncherBadgeCount(context)
    return next
}

@Synchronized
internal fun setChatLauncherBadgeCount(context: Context, count: Int) {
    launcherBadgePrefs(context)
        .edit()
        .putInt(KEY_CHAT_UNREAD_BADGE_COUNT, count.coerceAtLeast(0).coerceAtMost(999))
        .apply()
    applyCombinedLauncherBadgeCount(context)
}

@Synchronized
internal fun setTaskLauncherBadgeCount(context: Context, count: Int) {
    launcherBadgePrefs(context)
        .edit()
        .putInt(KEY_TASK_BADGE_COUNT, count.coerceAtLeast(0).coerceAtMost(999))
        .apply()
    applyCombinedLauncherBadgeCount(context)
}

private fun launcherBadgePrefs(context: Context) =
    context.applicationContext.getSharedPreferences(LAUNCHER_BADGE_PREFS, Context.MODE_PRIVATE)

private fun applyCombinedLauncherBadgeCount(context: Context) {
    val appContext = context.applicationContext
    val prefs = launcherBadgePrefs(appContext)
    val count = (
        prefs.getInt(KEY_CHAT_UNREAD_BADGE_COUNT, 0).coerceAtLeast(0) +
            prefs.getInt(KEY_TASK_BADGE_COUNT, 0).coerceAtLeast(0)
        ).coerceAtMost(999)

    runCatching {
        if (count > 0) {
            ShortcutBadger.applyCount(appContext, count)
        } else {
            ShortcutBadger.removeCount(appContext)
        }
    }
    applyHuaweiHonorBadge(appContext, count)
}

private fun applyHuaweiHonorBadge(context: Context, count: Int) {
    val payload = Bundle().apply {
        putString("package", context.packageName)
        putString("class", MainActivity::class.java.name)
        putInt("badgenumber", count)
    }
    listOf(
        "content://com.huawei.android.launcher.settings/badge/",
        "content://com.hihonor.android.launcher.settings/badge/"
    ).forEach { badgeUri ->
        runCatching {
            context.contentResolver.call(Uri.parse(badgeUri), "change_badge", null, payload)
        }
    }
}
