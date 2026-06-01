package com.elon.app

import android.content.Intent
import android.text.InputType
import android.view.inputmethod.InputMethodManager
import android.content.Context
import android.view.View
import android.widget.AdapterView
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
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
)

internal class MainFriendActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val setFriends: (List<AppFriend>) -> Unit,
    private val onFriendsChanged: () -> Unit
) {
    fun loadFriends() {
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
                    if (!response.isSuccessful) error(readErrorMessage(body, "加载好友失败"))
                    val array = JSONObject(body).optJSONArray("friends") ?: org.json.JSONArray()
                    List(array.length()) { index ->
                        parseFriend(array.optJSONObject(index) ?: JSONObject())
                    }
                }
            }
            activity.runOnUiThread {
                result.onSuccess {
                    setFriends(it)
                    onFriendsChanged()
                }
            }
        }
    }

    fun showAddFriendDialog() {
        if (!ensureLoggedIn()) return

        val searchOptions = listOf(
            FriendSearchOption("auto", "自动识别", "手机号、邮箱、账号 ID 或昵称", InputType.TYPE_CLASS_TEXT),
            FriendSearchOption("phone", "手机号", "好友手机号", InputType.TYPE_CLASS_PHONE),
            FriendSearchOption(
                "email",
                "邮箱",
                "好友邮箱",
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
            ),
            FriendSearchOption("account_id", "账号 ID", "usr_ 开头的账号 ID", InputType.TYPE_CLASS_TEXT),
            FriendSearchOption("nickname", "昵称", "好友昵称", InputType.TYPE_CLASS_TEXT)
        )
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), dp(8), dp(22), dp(2))
        }
        val hint = TextView(activity).apply {
            text = "选择搜索方式后输入关键词，只能搜索到已注册一龙账号的用户。"
            textSize = 14f
            alpha = 0.72f
        }
        val searchTypeRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, dp(10), 0, 0)
        }
        val searchTypeLabel = TextView(activity).apply {
            text = "搜索类型"
            textSize = 15f
            alpha = 0.82f
            setPadding(0, 0, dp(12), 0)
        }
        val searchTypeSpinner = Spinner(activity).apply {
            adapter = ArrayAdapter(
                activity,
                android.R.layout.simple_spinner_dropdown_item,
                searchOptions
            )
        }
        val input = EditText(activity).apply {
            setHint(searchOptions.first().hint)
            inputType = searchOptions.first().inputType
            setSingleLine(true)
            textSize = 18f
            setSelectAllOnFocus(false)
        }
        val result = TextView(activity).apply {
            textSize = 14f
            alpha = 0.82f
            setPadding(0, dp(8), 0, 0)
        }
        container.addView(hint)
        searchTypeRow.addView(searchTypeLabel)
        searchTypeRow.addView(searchTypeSpinner)
        container.addView(searchTypeRow)
        container.addView(input)
        container.addView(result)

        val dialog = AlertDialog.Builder(activity)
            .setTitle("添加好友")
            .setView(container)
            .setNegativeButton("取消", null)
            .setPositiveButton("搜索", null)
            .create()

        dialog.setOnShowListener {
            searchTypeSpinner.onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) {
                    val option = searchOptions.getOrElse(position) { searchOptions.first() }
                    input.hint = option.hint
                    input.inputType = option.inputType
                    input.setSingleLine(true)
                }

                override fun onNothingSelected(parent: AdapterView<*>?) = Unit
            }
            val searchButton = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
            searchButton.setOnClickListener {
                val selected = searchOptions.getOrElse(searchTypeSpinner.selectedItemPosition) { searchOptions.first() }
                val keyword = input.text?.toString()?.trim().orEmpty()
                if (keyword.isBlank()) {
                    result.text = "请输入搜索内容"
                    return@setOnClickListener
                }
                result.text = "正在搜索..."
                searchButton.isEnabled = false
                searchFriend(selected.key, keyword) { candidate, error ->
                    searchButton.isEnabled = true
                    when {
                        error != null -> result.text = error
                        candidate == null -> result.text = "没有找到对应的一龙账号"
                        candidate.isSelf -> result.text = "这是你自己的账号，不能添加自己"
                        candidate.alreadyFriend -> result.text = "已经是好友：${candidate.name}"
                        else -> {
                            result.text = "找到：${candidate.name}"
                            showConfirmAddDialog(dialog, selected.key, keyword, candidate)
                        }
                    }
                }
            }
            input.requestFocus()
            input.postDelayed({
                val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
                imm?.showSoftInput(input, InputMethodManager.SHOW_IMPLICIT)
            }, 160)
        }

        dialog.show()
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

    private fun showConfirmAddDialog(
        parentDialog: AlertDialog,
        searchType: String,
        keyword: String,
        candidate: FriendCandidate
    ) {
        val message = buildString {
            append(candidate.name)
            candidate.account.takeIf { it.isNotBlank() }?.let { append("\n账号：").append(it) }
            candidate.phone?.takeIf { it.isNotBlank() }?.let { append("\n手机号：").append(it) }
        }
        AlertDialog.Builder(activity)
            .setTitle("添加好友")
            .setMessage(message)
            .setNegativeButton("取消", null)
            .setPositiveButton("添加") { _, _ ->
                addFriend(searchType, keyword) { addedName, alreadyFriend, error ->
                    if (error != null) {
                        Toast.makeText(activity, error, Toast.LENGTH_SHORT).show()
                        return@addFriend
                    }
                    parentDialog.dismiss()
                    loadFriends()
                    val text = if (alreadyFriend) {
                        "已经是好友：$addedName"
                    } else {
                        "已添加好友：$addedName"
                    }
                    Toast.makeText(activity, text, Toast.LENGTH_SHORT).show()
                }
            }
            .show()
    }

    private fun searchFriend(
        searchType: String,
        keyword: String,
        onDone: (FriendCandidate?, String?) -> Unit
    ) {
        thread {
            val result = runCatching {
                val builder = Request.Builder()
                    .url("$serverUrl/api/me/friends/search?search_type=${urlPart(searchType)}&query=${urlPart(keyword)}")
                    .get()
                val request = AuthManager.applyAuth(activity, builder).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "搜索失败：HTTP ${response.code}"))
                    val json = JSONObject(body)
                    if (!json.optBoolean("found", false)) return@runCatching null
                    val user = json.optJSONObject("user") ?: JSONObject()
                    val account = user.optString("account", "").trim()
                    val phoneText = user.optString("phone", "").trim().takeIf { it.isNotEmpty() }
                    val nickname = user.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
                    FriendCandidate(
                        name = nickname ?: account.ifBlank { "已注册用户" },
                        account = account,
                        phone = phoneText,
                        alreadyFriend = json.optBoolean("already_friend", false),
                        isSelf = json.optBoolean("is_self", false)
                    )
                }
            }
            activity.runOnUiThread {
                result.fold(
                    onSuccess = { onDone(it, null) },
                    onFailure = { onDone(null, it.message ?: "搜索失败") }
                )
            }
        }
    }

    private fun addFriend(
        searchType: String,
        keyword: String,
        onDone: (name: String, alreadyFriend: Boolean, error: String?) -> Unit
    ) {
        thread {
            val result = runCatching {
                val payload = JSONObject()
                    .put("search_type", searchType)
                    .put("query", keyword)
                    .toString()
                    .toRequestBody("application/json".toMediaType())
                val builder = Request.Builder()
                    .url("$serverUrl/api/me/friends")
                    .post(payload)
                val request = AuthManager.applyAuth(activity, builder).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "添加失败：HTTP ${response.code}"))
                    val json = JSONObject(body)
                    val friend = json.optJSONObject("friend") ?: JSONObject()
                    val nickname = friend.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
                    val account = friend.optString("account", "").trim()
                    Triple(
                        nickname ?: account.ifBlank { "好友" },
                        json.optBoolean("already_friend", false),
                        null as String?
                    )
                }
            }
            activity.runOnUiThread {
                result.fold(
                    onSuccess = { onDone(it.first, it.second, null) },
                    onFailure = { onDone("", false, it.message ?: "添加失败") }
                )
            }
        }
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
        )
    }

    private fun parseServerTime(value: String): Long? {
        if (value.isBlank()) return null
        return runCatching { Instant.parse(value).toEpochMilli() }.getOrNull()
    }

    private data class FriendCandidate(
        val name: String,
        val account: String,
        val phone: String?,
        val alreadyFriend: Boolean,
        val isSelf: Boolean
    )

    private data class FriendSearchOption(
        val key: String,
        val label: String,
        val hint: String,
        val inputType: Int
    ) {
        override fun toString(): String = label
    }
}
