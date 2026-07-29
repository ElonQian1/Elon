package com.elon.app

import android.content.Intent
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.time.Instant
import kotlin.concurrent.thread

internal data class AppFriend(
    val id: String,
    val name: String,
    val account: String,
    val phone: String?,
    val avatarDataUrl: String?,
    val friendSince: String?,
    val lastMessage: String?,
    val lastMessageAt: Long?,
    val unreadCount: Int,
    val isOnline: Boolean = false,
    val lastReceivedMessage: String? = null,
    val lastReceivedAt: Long? = null,
)

internal class MainFriendActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val setFriends: (List<AppFriend>) -> Unit,
    private val onFriendsChanged: () -> Unit,
    private val onSessionExpired: () -> Unit,
    private val openAddFriendPage: () -> Unit
) {
    private var sessionExpiredPromptShown = false

    fun loadFriends() {
        if (AuthManager.isSessionExpired(activity)) {
            handleSessionExpired()
            return
        }
        if (!AuthManager.isLoggedIn(activity)) {
            setFriends(emptyList())
            onFriendsChanged()
            return
        }
        thread {
            val result = runCatching {
                val builder = Request.Builder()
                    .url("$serverUrl/api/me/friends")
                    .get()
                val request = AuthManager.applyAuth(activity, builder).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (response.code == 401) {
                        throw SessionExpiredException(
                            readErrorMessage(body, "登录已过期，请重新登录")
                        )
                    }
                    if (!response.isSuccessful) error(readErrorMessage(body, "加载好友失败"))
                    val array = JSONObject(body).optJSONArray("friends") ?: org.json.JSONArray()
                    List(array.length()) { index ->
                        parseFriend(array.optJSONObject(index) ?: JSONObject())
                    }
                }
            }
            activity.runOnUiThread {
                result.onSuccess {
                    sessionExpiredPromptShown = false
                    setFriends(it)
                    onFriendsChanged()
                }
                result.exceptionOrNull()
                    ?.takeIf { it is SessionExpiredException }
                    ?.let { handleSessionExpired() }
            }
        }
    }

    fun showAddFriendDialog() {
        if (!ensureLoggedIn()) return
        openAddFriendPage()
    }

    private fun ensureLoggedIn(): Boolean {
        if (AuthManager.isLoggedIn(activity)) return true
        AlertDialog.Builder(activity)
            .setTitle("需要登录")
            .setMessage("添加好友需要先登录账号。")
            .setNegativeButton("取消", null)
            .setPositiveButton("去登录") { _, _ ->
                activity.startActivity(Intent(activity, LoginActivity::class.java))
            }
            .show()
        return false
    }

    private fun handleSessionExpired() {
        AuthManager.clear(activity)
        setFriends(emptyList())
        onFriendsChanged()
        onSessionExpired()
        if (sessionExpiredPromptShown || activity.isFinishing || activity.isDestroyed) return
        sessionExpiredPromptShown = true
        AlertDialog.Builder(activity)
            .setTitle("登录已过期")
            .setMessage("好友数据仍保存在原账号中。请重新登录原账号恢复好友列表。")
            .setNegativeButton("稍后", null)
            .setPositiveButton("重新登录") { _, _ ->
                activity.startActivity(Intent(activity, LoginActivity::class.java))
            }
            .show()
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }

    private fun parseFriend(json: JSONObject): AppFriend {
        val account = json.optString("account", "").trim()
        val phone = json.optString("phone", "").trim().takeIf { it.isNotEmpty() }
        val nickname = json.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
        return AppFriend(
            id = json.optString("id", "").trim(),
            name = nickname ?: account.ifBlank { phone ?: "好友" },
            account = account,
            phone = phone,
            avatarDataUrl = json.optString("avatar_data_url", "").trim().takeIf { it.isNotEmpty() },
            friendSince = json.optString("friend_since", "").trim().takeIf { it.isNotEmpty() },
            lastMessage = json.optString("last_message", "").trim().takeIf { it.isNotEmpty() },
            lastMessageAt = parseServerTime(json.optString("last_message_at", "").trim()),
            unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0),
            isOnline = json.optBoolean("is_online", false),
            lastReceivedMessage = json.optString("last_received_message", "").trim()
                .takeIf { it.isNotEmpty() },
            lastReceivedAt = parseServerTime(json.optString("last_received_at", "").trim()),
        )
    }

    private fun parseServerTime(value: String): Long? {
        if (value.isBlank()) return null
        return runCatching { Instant.parse(value).toEpochMilli() }.getOrNull()
    }

    private class SessionExpiredException(message: String) : IllegalStateException(message)
}
