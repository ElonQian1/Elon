package com.elon.app

import android.content.Context
import android.util.Log
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

private const val SOCIAL_SUMMARY_POLL_MS = 15_000L
private const val FIRST_POLL_NOTIFY_WINDOW_MS = 10 * 60 * 1000L

internal class SocialSummaryPoller(
    context: Context,
    private val http: OkHttpClient = OkHttpClient.Builder().build()
) {
    private val appContext = context.applicationContext
    private val prefs = appContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private val executor: ExecutorService = Executors.newSingleThreadExecutor()
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    private val pollRunnable = object : Runnable {
        override fun run() {
            pollNow()
            if (running) handler.postDelayed(this, SOCIAL_SUMMARY_POLL_MS)
        }
    }

    @Volatile private var running = false
    @Volatile private var inFlight = false

    fun start() {
        if (running) return
        running = true
        pollNow()
        handler.postDelayed(pollRunnable, SOCIAL_SUMMARY_POLL_MS)
    }

    fun stop() {
        running = false
        handler.removeCallbacks(pollRunnable)
        executor.shutdownNow()
    }

    private fun pollNow() {
        if (inFlight || !running) return
        if (!AuthManager.isLoggedIn(appContext)) {
            prefs.edit().clear().apply()
            setChatLauncherBadgeCount(appContext, 0)
            return
        }
        inFlight = true
        runCatching {
            executor.execute {
                try {
                    val friends = fetchFriends()
                    val groups = fetchGroups()
                    handleSnapshot(friends, groups)
                } catch (t: Throwable) {
                    Log.w(TAG, "poll failed: ${t.message}")
                } finally {
                    inFlight = false
                }
            }
        }.onFailure {
            inFlight = false
        }
    }

    private fun fetchFriends(): List<FriendSummary> {
        val request = AuthManager.applyAuth(
            appContext,
            Request.Builder().url("${ServerUrlManager.getActive(appContext)}/api/me/friends").get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error("friends HTTP ${response.code}")
            val array = JSONObject(body).optJSONArray("friends") ?: JSONArray()
            return List(array.length()) { index ->
                parseFriend(array.optJSONObject(index) ?: JSONObject())
            }
        }
    }

    private fun fetchGroups(): List<GroupSummary> {
        val request = AuthManager.applyAuth(
            appContext,
            Request.Builder().url("${ServerUrlManager.getActive(appContext)}/api/me/groups").get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error("groups HTTP ${response.code}")
            val array = JSONObject(body).optJSONArray("groups") ?: JSONArray()
            return List(array.length()) { index ->
                parseGroup(array.optJSONObject(index) ?: JSONObject())
            }
        }
    }

    private fun handleSnapshot(
        friends: List<FriendSummary>,
        groups: List<GroupSummary>
    ) {
        val initialized = prefs.getBoolean(KEY_INITIALIZED, false)
        val now = System.currentTimeMillis()
        val editor = prefs.edit()

        friends.forEach { summary ->
            if (shouldNotifyFriend(summary, initialized, now)) {
                ChatMessageNotifications.showFriendMessage(
                    context = appContext,
                    fromUserId = summary.id,
                    messageId = "",
                    content = summary.lastMessage.orEmpty(),
                    senderName = summary.name,
                    createdAt = summary.lastMessageAtRaw
                )
            }
            editor.putString(friendFingerprintKey(summary.id), summary.fingerprint)
            editor.putInt(friendUnreadKey(summary.id), summary.unreadCount)
        }

        groups.forEach { summary ->
            if (shouldNotifyGroup(summary, initialized, now)) {
                ChatMessageNotifications.showGroupMessage(
                    context = appContext,
                    groupId = summary.id,
                    fromUserId = "",
                    messageId = "",
                    content = summary.lastMessage.orEmpty(),
                    groupName = summary.name,
                    createdAt = summary.lastMessageAtRaw
                )
            }
            editor.putString(groupFingerprintKey(summary.id), summary.fingerprint)
            editor.putInt(groupUnreadKey(summary.id), summary.unreadCount)
        }

        val totalUnread = friends.sumOf { it.unreadCount } + groups.sumOf { it.unreadCount }
        editor.putBoolean(KEY_INITIALIZED, true).apply()
        setChatLauncherBadgeCount(appContext, totalUnread)
    }

    private fun shouldNotifyFriend(
        summary: FriendSummary,
        initialized: Boolean,
        now: Long
    ): Boolean {
        if (!summary.canNotify) return false
        val previousUnread = prefs.getInt(friendUnreadKey(summary.id), 0).coerceAtLeast(0)
        val previousFingerprint = prefs.getString(friendFingerprintKey(summary.id), null)
        return shouldNotifySummary(summary, initialized, previousUnread, previousFingerprint, now)
    }

    private fun shouldNotifyGroup(
        summary: GroupSummary,
        initialized: Boolean,
        now: Long
    ): Boolean {
        if (!summary.canNotify) return false
        val previousUnread = prefs.getInt(groupUnreadKey(summary.id), 0).coerceAtLeast(0)
        val previousFingerprint = prefs.getString(groupFingerprintKey(summary.id), null)
        return shouldNotifySummary(summary, initialized, previousUnread, previousFingerprint, now)
    }

    private fun shouldNotifySummary(
        summary: SummaryBase,
        initialized: Boolean,
        previousUnread: Int,
        previousFingerprint: String?,
        now: Long
    ): Boolean {
        if (!initialized) {
            val at = summary.lastMessageAt ?: return false
            return now - at in 0..FIRST_POLL_NOTIFY_WINDOW_MS
        }
        if (summary.unreadCount > previousUnread) return true
        return previousUnread > 0 && summary.fingerprint != previousFingerprint
    }

    private fun parseFriend(json: JSONObject): FriendSummary {
        val account = json.optString("account", "").trim()
        val nickname = json.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
        val phone = json.optString("phone", "").trim().takeIf { it.isNotEmpty() }
        val lastMessageAtRaw = json.optString("last_message_at", "").trim()
        val lastMessage = json.optString("last_message", "").trim().takeIf { it.isNotEmpty() }
        return FriendSummary(
            id = json.optString("id", "").trim(),
            name = nickname ?: account.ifBlank { phone ?: "\u597d\u53cb" },
            lastMessage = lastMessage,
            lastMessageAtRaw = lastMessageAtRaw,
            lastMessageAt = parseServerTime(lastMessageAtRaw),
            unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0)
        )
    }

    private fun parseGroup(json: JSONObject): GroupSummary {
        val lastMessageAtRaw = json.optString("last_message_at", "").trim()
        val lastMessage = json.optString("last_message", "").trim().takeIf { it.isNotEmpty() }
        return GroupSummary(
            id = json.optString("id", "").trim(),
            name = json.optString("name", "").trim().ifBlank { "\u7fa4\u804a" },
            lastMessage = lastMessage,
            lastMessageAtRaw = lastMessageAtRaw,
            lastMessageAt = parseServerTime(lastMessageAtRaw),
            unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0)
        )
    }

    private fun parseServerTime(value: String): Long? {
        if (value.isBlank()) return null
        return runCatching { Instant.parse(value).toEpochMilli() }.getOrNull()
    }

    private fun friendUnreadKey(id: String) = "friend_unread_$id"
    private fun friendFingerprintKey(id: String) = "friend_fp_$id"
    private fun groupUnreadKey(id: String) = "group_unread_$id"
    private fun groupFingerprintKey(id: String) = "group_fp_$id"

    private interface SummaryBase {
        val id: String
        val lastMessage: String?
        val lastMessageAtRaw: String
        val lastMessageAt: Long?
        val unreadCount: Int

        val canNotify: Boolean
            get() = id.isNotBlank() && unreadCount > 0 && !lastMessage.isNullOrBlank()

        val fingerprint: String
            get() = "$lastMessageAtRaw:${lastMessage.orEmpty().hashCode()}"
    }

    private data class FriendSummary(
        override val id: String,
        val name: String,
        override val lastMessage: String?,
        override val lastMessageAtRaw: String,
        override val lastMessageAt: Long?,
        override val unreadCount: Int
    ) : SummaryBase

    private data class GroupSummary(
        override val id: String,
        val name: String,
        override val lastMessage: String?,
        override val lastMessageAtRaw: String,
        override val lastMessageAt: Long?,
        override val unreadCount: Int
    ) : SummaryBase

    private companion object {
        private const val TAG = "SocialSummaryPoller"
        private const val PREFS_NAME = "social_summary_poller"
        private const val KEY_INITIALIZED = "initialized"
    }
}
