package com.elon.app

import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.content.Context
import android.content.res.ColorStateList
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import android.widget.Toast
import android.view.inputmethod.InputMethodManager
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
        var selectedOption = searchOptions.first()
        var searchTypePopup: PopupWindow? = null

        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(24), dp(24), dp(22))
            background = roundedRect("#242424", 12)
        }
        val title = TextView(activity).apply {
            text = "添加好友"
            textSize = 24f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.parseColor("#D6D6D6"))
        }
        val hint = TextView(activity).apply {
            text = "选择搜索方式后输入关键词，只能搜索到已注册一龙账号的用户。"
            textSize = 14f
            setTextColor(Color.parseColor("#A8A8A8"))
            setLineSpacing(0f, 1.08f)
        }
        val searchTypeLabel = TextView(activity).apply {
            text = "搜索类型"
            textSize = 13f
            setTextColor(Color.parseColor("#A8A8A8"))
        }
        val searchTypeValue = TextView(activity).apply {
            text = selectedOption.label
            textSize = 17f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.parseColor("#D6D6D6"))
        }
        val searchTypeHint = TextView(activity).apply {
            text = selectedOption.hint
            textSize = 12f
            setTextColor(Color.parseColor("#777777"))
        }
        val input = EditText(activity).apply {
            setHint(selectedOption.hint)
            inputType = selectedOption.inputType
            setSingleLine(true)
            textSize = 17f
            setSelectAllOnFocus(false)
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#777777"))
            backgroundTintList = ColorStateList.valueOf(Color.TRANSPARENT)
            background = roundedRect("#222222", 8, "#2E2E2E")
            minimumHeight = dp(54)
            setPadding(dp(16), 0, dp(16), 0)
        }
        val searchTypeRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            isClickable = true
            isFocusable = true
            background = roundedRect("#222222", 8, "#2E2E2E")
            minimumHeight = dp(58)
            setPadding(dp(16), dp(9), dp(14), dp(9))
            setOnClickListener {
                searchTypePopup = showSearchTypePopup(
                    anchor = this,
                    previousPopup = searchTypePopup,
                    options = searchOptions,
                    selectedKey = selectedOption.key
                ) { option ->
                    selectedOption = option
                    searchTypeValue.text = option.label
                    searchTypeHint.text = option.hint
                    input.hint = option.hint
                    input.inputType = option.inputType
                    input.setSingleLine(true)
                }
            }
        }
        val searchTypeTextBlock = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(searchTypeValue)
            addView(searchTypeHint, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(2)
            })
        }
        val arrow = TextView(activity).apply {
            text = "▾"
            textSize = 20f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A8A8A8"))
        }
        val result = TextView(activity).apply {
            textSize = 14f
            minHeight = dp(24)
            setTextColor(Color.parseColor("#A8A8A8"))
        }
        val cancelButton = dialogButton("取消", "#2A2A2A", "#D6D6D6")
        val searchButton = dialogButton("搜索", "#C8C8C8", "#101010")
        val actions = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            addView(cancelButton, LinearLayout.LayoutParams(0, dp(44), 1f))
            addView(searchButton, LinearLayout.LayoutParams(0, dp(44), 1f).apply {
                leftMargin = dp(12)
            })
        }

        searchTypeRow.addView(searchTypeTextBlock, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        searchTypeRow.addView(arrow, LinearLayout.LayoutParams(dp(28), dp(28)))
        container.addView(title)
        container.addView(hint, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(10)
        })
        container.addView(searchTypeLabel, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(18)
        })
        container.addView(searchTypeRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(7)
        })
        container.addView(input, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(12)
        })
        container.addView(result, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(10)
        })
        container.addView(actions, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(18)
        })

        lateinit var dialog: AlertDialog
        dialog = AlertDialog.Builder(activity)
            .setView(container)
            .create()

        fun setResult(text: String, color: String = "#A8A8A8") {
            result.text = text
            result.setTextColor(Color.parseColor(color))
        }

        cancelButton.setOnClickListener { dialog.dismiss() }
        searchButton.setOnClickListener {
            val keyword = input.text?.toString()?.trim().orEmpty()
            if (keyword.isBlank()) {
                setResult("请输入搜索内容", "#D97A7A")
                return@setOnClickListener
            }
            setResult("正在搜索...")
            searchButton.isEnabled = false
            searchButton.alpha = 0.55f
            searchFriend(selectedOption.key, keyword) { candidate, error ->
                searchButton.isEnabled = true
                searchButton.alpha = 1f
                when {
                    error != null -> setResult(error, "#D97A7A")
                    candidate == null -> setResult("没有找到对应的一龙账号", "#D97A7A")
                    candidate.isSelf -> setResult("这是你自己的账号，不能添加自己", "#D97A7A")
                    candidate.alreadyFriend -> setResult("已经是好友：${candidate.name}")
                    else -> {
                        setResult("找到：${candidate.name}", "#58BE6A")
                        showConfirmAddDialog(dialog, selectedOption.key, keyword, candidate)
                    }
                }
            }
        }

        dialog.setOnShowListener {
            input.requestFocus()
            input.postDelayed({
                val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
                imm?.showSoftInput(input, InputMethodManager.SHOW_IMPLICIT)
            }, 160)
        }
        dialog.setOnDismissListener { searchTypePopup?.dismiss() }

        showStyledDialog(dialog)
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
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(24), dp(24), dp(22))
            background = roundedRect("#242424", 12)
        }
        val title = TextView(activity).apply {
            text = "确认添加"
            textSize = 22f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.parseColor("#D6D6D6"))
        }
        val summary = TextView(activity).apply {
            text = "将这位用户添加到你的好友列表"
            textSize = 14f
            setTextColor(Color.parseColor("#A8A8A8"))
        }
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = roundedRect("#222222", 8, "#2E2E2E")
            setPadding(dp(16), dp(14), dp(16), dp(14))
        }
        val name = TextView(activity).apply {
            text = candidate.name
            textSize = 18f
            typeface = Typeface.DEFAULT_BOLD
            setTextColor(Color.parseColor("#D6D6D6"))
        }
        card.addView(name)
        candidate.account.takeIf { it.isNotBlank() }?.let {
            card.addView(detailText("账号：$it"))
        }
        candidate.phone?.takeIf { it.isNotBlank() }?.let {
            card.addView(detailText("手机号：$it"))
        }
        val cancelButton = dialogButton("取消", "#2A2A2A", "#D6D6D6")
        val addButton = dialogButton("添加", "#C8C8C8", "#101010")
        val actions = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            addView(cancelButton, LinearLayout.LayoutParams(0, dp(44), 1f))
            addView(addButton, LinearLayout.LayoutParams(0, dp(44), 1f).apply {
                leftMargin = dp(12)
            })
        }

        container.addView(title)
        container.addView(summary, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(8)
        })
        container.addView(card, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(16)
        })
        container.addView(actions, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(18)
        })

        lateinit var dialog: AlertDialog
        dialog = AlertDialog.Builder(activity)
            .setView(container)
            .create()
        cancelButton.setOnClickListener { dialog.dismiss() }
        addButton.setOnClickListener {
            addButton.isEnabled = false
            addButton.alpha = 0.55f
            addFriend(searchType, keyword) { addedName, alreadyFriend, error ->
                addButton.isEnabled = true
                addButton.alpha = 1f
                if (error != null) {
                    Toast.makeText(activity, error, Toast.LENGTH_SHORT).show()
                    return@addFriend
                }
                parentDialog.dismiss()
                dialog.dismiss()
                loadFriends()
                val text = if (alreadyFriend) {
                    "已经是好友：$addedName"
                } else {
                    "已添加好友：$addedName"
                }
                Toast.makeText(activity, text, Toast.LENGTH_SHORT).show()
            }
        }
        showStyledDialog(dialog)
    }

    private fun showStyledDialog(dialog: AlertDialog) {
        dialog.show()
        dialog.window?.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            decorView.setPadding(0, 0, 0, 0)
            setDimAmount(0.72f)
            val width = (activity.resources.displayMetrics.widthPixels - dp(48)).coerceAtMost(dp(386))
            setLayout(width, ViewGroup.LayoutParams.WRAP_CONTENT)
        }
    }

    private fun showSearchTypePopup(
        anchor: View,
        previousPopup: PopupWindow?,
        options: List<FriendSearchOption>,
        selectedKey: String,
        onSelected: (FriendSearchOption) -> Unit
    ): PopupWindow {
        previousPopup?.dismiss()
        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(6), dp(6), dp(6), dp(6))
            background = roundedRect("#242424", 10)
            alpha = 0f
            scaleY = 0.96f
        }
        lateinit var popup: PopupWindow
        options.forEach { option ->
            val selected = option.key == selectedKey
            panel.addView(searchTypePopupRow(option, selected) {
                onSelected(option)
                popup.dismiss()
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(52)
            ))
        }
        val popupWidth = anchor.width.coerceAtLeast(dp(236)).coerceAtMost(
            activity.resources.displayMetrics.widthPixels - dp(48)
        )
        popup = PopupWindow(panel, popupWidth, ViewGroup.LayoutParams.WRAP_CONTENT, true).apply {
            isOutsideTouchable = true
            elevation = dp(10).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAsDropDown(anchor, 0, dp(6))
        }
        panel.animate().alpha(1f).scaleY(1f).setDuration(120L).start()
        return popup
    }

    private fun searchTypePopupRow(
        option: FriendSearchOption,
        selected: Boolean,
        onClick: () -> Unit
    ): LinearLayout {
        val row = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            isClickable = true
            isFocusable = true
            setPadding(dp(12), 0, dp(10), 0)
            if (selected) background = roundedRect("#2A2A2A", 8)
            setOnClickListener { onClick() }
        }
        val labelBlock = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        labelBlock.addView(TextView(activity).apply {
            text = option.label
            textSize = 15f
            setTextColor(Color.parseColor("#D6D6D6"))
        })
        labelBlock.addView(TextView(activity).apply {
            text = option.hint
            textSize = 11.5f
            setTextColor(Color.parseColor("#777777"))
        })
        row.addView(labelBlock, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        row.addView(TextView(activity).apply {
            text = if (selected) "✓" else ""
            textSize = 16f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#58BE6A"))
        }, LinearLayout.LayoutParams(dp(26), dp(26)))
        return row
    }

    private fun detailText(textValue: String): TextView =
        TextView(activity).apply {
            text = textValue
            textSize = 14f
            setTextColor(Color.parseColor("#A8A8A8"))
            setPadding(0, dp(6), 0, 0)
        }

    private fun dialogButton(label: String, bgColor: String, textColor: String): TextView =
        TextView(activity).apply {
            text = label
            textSize = 15f
            typeface = Typeface.DEFAULT_BOLD
            gravity = Gravity.CENTER
            isClickable = true
            isFocusable = true
            setTextColor(Color.parseColor(textColor))
            background = roundedRect(bgColor, 8)
        }

    private fun roundedRect(fillColor: String, radiusDp: Int, strokeColor: String? = null): GradientDrawable =
        GradientDrawable().apply {
            setColor(Color.parseColor(fillColor))
            cornerRadius = dp(radiusDp).toFloat()
            strokeColor?.let { setStroke(dp(1), Color.parseColor(it)) }
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
