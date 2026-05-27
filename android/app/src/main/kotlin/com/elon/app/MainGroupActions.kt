package com.elon.app

import android.content.Intent
import android.view.Gravity
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant
import kotlin.concurrent.thread

internal data class AppGroup(
    val id: String,
    val name: String,
    val memberCount: Int,
    val members: List<AppGroupMember>,
    val createdAt: Long?,
    val lastMessage: String?,
    val lastMessageAt: Long?,
    val unreadCount: Int
)

internal data class AppGroupMember(
    val id: String,
    val displayName: String,
    val avatarDataUrl: String?
)

internal class MainGroupActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val friends: () -> List<AppFriend>,
    private val dp: (Int) -> Int,
    private val setGroups: (List<AppGroup>) -> Unit,
    private val onGroupsChanged: () -> Unit,
    private val openGroup: (AppGroup) -> Unit
) {
    fun loadGroups() {
        if (!AuthManager.isLoggedIn(activity)) {
            setGroups(emptyList())
            onGroupsChanged()
            return
        }
        thread {
            val result = runCatching {
                val request = AuthManager.applyAuth(
                    activity,
                    Request.Builder().url("$serverUrl/api/me/groups").get()
                ).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "加载群聊失败"))
                    val array = JSONObject(body).optJSONArray("groups") ?: JSONArray()
                    List(array.length()) { index ->
                        parseGroup(array.optJSONObject(index) ?: JSONObject())
                    }
                }
            }
            activity.runOnUiThread {
                result.onSuccess {
                    setGroups(it)
                    onGroupsChanged()
                }
            }
        }
    }

    fun showCreateGroupDialog() {
        if (!ensureLoggedIn()) return
        val friendList = friends()
        if (friendList.isEmpty()) {
            Toast.makeText(activity, "先添加好友后再发起群聊", Toast.LENGTH_SHORT).show()
            return
        }

        val selectedIds = linkedSetOf<String>()
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), dp(8), dp(22), dp(2))
        }
        val hint = TextView(activity).apply {
            text = "选择好友后创建群聊，群成员都可以收发消息。"
            textSize = 14f
            alpha = 0.72f
        }
        val nameInput = EditText(activity).apply {
            setHint("群聊名称（可选）")
            setSingleLine(true)
            textSize = 16f
        }
        val listContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        friendList.forEach { friend ->
            listContainer.addView(CheckBox(activity).apply {
                text = "${friend.name}  ${friend.phone ?: friend.account}"
                textSize = 16f
                gravity = Gravity.CENTER_VERTICAL
                setPadding(0, dp(8), 0, dp(8))
                setOnCheckedChangeListener { _, checked ->
                    if (checked) selectedIds.add(friend.id) else selectedIds.remove(friend.id)
                }
            })
        }
        val scroll = ScrollView(activity).apply {
            addView(listContainer)
        }
        container.addView(hint)
        container.addView(nameInput)
        container.addView(scroll, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(260)
        ))

        val dialog = AlertDialog.Builder(activity)
            .setTitle("发起群聊")
            .setView(container)
            .setNegativeButton("取消", null)
            .setPositiveButton("完成", null)
            .create()

        dialog.setOnShowListener {
            val doneButton = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
            doneButton.setOnClickListener {
                if (selectedIds.isEmpty()) {
                    Toast.makeText(activity, "请选择至少 1 位好友", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                doneButton.isEnabled = false
                createGroup(nameInput.text?.toString()?.trim().orEmpty(), selectedIds.toList()) { group, error ->
                    doneButton.isEnabled = true
                    if (error != null) {
                        Toast.makeText(activity, error, Toast.LENGTH_SHORT).show()
                        return@createGroup
                    }
                    dialog.dismiss()
                    group?.let {
                        loadGroups()
                        openGroup(it)
                    }
                }
            }
        }

        dialog.show()
    }

    private fun ensureLoggedIn(): Boolean {
        if (AuthManager.isLoggedIn(activity)) return true
        AlertDialog.Builder(activity)
            .setTitle("需要登录")
            .setMessage("发起群聊需要先登录账号。")
            .setNegativeButton("取消", null)
            .setPositiveButton("去登录") { _, _ ->
                activity.startActivity(Intent(activity, LoginActivity::class.java))
            }
            .show()
        return false
    }

    private fun createGroup(
        name: String,
        memberIds: List<String>,
        onDone: (AppGroup?, String?) -> Unit
    ) {
        thread {
            val result = runCatching {
                val payload = JSONObject()
                    .put("name", name.takeIf { it.isNotBlank() })
                    .put("member_ids", JSONArray(memberIds))
                    .toString()
                    .toRequestBody("application/json".toMediaType())
                val request = AuthManager.applyAuth(
                    activity,
                    Request.Builder().url("$serverUrl/api/me/groups").post(payload)
                ).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "创建群聊失败"))
                    parseGroup(JSONObject(body).optJSONObject("group") ?: JSONObject())
                }
            }
            activity.runOnUiThread {
                result.fold(
                    onSuccess = { onDone(it, null) },
                    onFailure = { onDone(null, it.message ?: "创建群聊失败") }
                )
            }
        }
    }

    private fun parseGroup(json: JSONObject): AppGroup {
        return AppGroup(
            id = json.optString("id", "").trim(),
            name = json.optString("name", "").trim().ifBlank { "群聊" },
            memberCount = json.optInt("member_count", 0).coerceAtLeast(0),
            members = parseGroupMembers(json.optJSONArray("members")),
            createdAt = parseServerTime(json.optString("created_at", "").trim()),
            lastMessage = json.optString("last_message", "").trim().takeIf { it.isNotEmpty() },
            lastMessageAt = parseServerTime(json.optString("last_message_at", "").trim()),
            unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0)
        )
    }

    private fun parseGroupMembers(array: JSONArray?): List<AppGroupMember> {
        if (array == null) return emptyList()
        return List(array.length()) { index ->
            val item = array.optJSONObject(index) ?: JSONObject()
            AppGroupMember(
                id = item.optString("id", "").trim(),
                displayName = item.optString("display_name", "").trim().ifBlank { "成员" },
                avatarDataUrl = item.optString("avatar_data_url", "").trim().takeIf { it.isNotEmpty() }
            )
        }
    }

    private fun parseServerTime(value: String): Long? {
        if (value.isBlank()) return null
        return runCatching { Instant.parse(value).toEpochMilli() }.getOrNull()
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }
}
