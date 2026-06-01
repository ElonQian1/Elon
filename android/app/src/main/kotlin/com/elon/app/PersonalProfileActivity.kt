package com.elon.app

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import kotlin.concurrent.thread

class PersonalProfileActivity : AppCompatActivity() {
    private lateinit var rows: LinearLayout
    private var profile: UserProfile? = null
    private val http = OkHttpClient()
    private val serverUrl = BuildConfig.SERVER_URL

    private val avatarPicker = registerForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
        if (uri == null) {
            Toast.makeText(this, "已取消选择头像", Toast.LENGTH_SHORT).show()
            return@registerForActivityResult
        }
        Toast.makeText(this, "正在处理头像...", Toast.LENGTH_SHORT).show()
        thread(name = "profile-avatar") {
            val result = runCatching { UserProfileStore.avatarDataUrlFromUri(this, uri) }
            runOnUiThread {
                result.onSuccess { dataUrl ->
                    saveProfile(avatarDataUrl = dataUrl)
                    Toast.makeText(this, "头像已更新", Toast.LENGTH_SHORT).show()
                }.onFailure {
                    Toast.makeText(this, "头像处理失败：${it.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(buildContent())
        renderRows()
    }

    private fun buildContent(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(android.graphics.Color.parseColor("#101010"))
            addView(topBar())
            addView(ScrollView(this@PersonalProfileActivity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f
                )
                rows = LinearLayout(this@PersonalProfileActivity).apply {
                    orientation = LinearLayout.VERTICAL
                    setPadding(0, dp(10), 0, dp(28))
                }
                addView(rows)
            })
        }
    }

    private fun topBar(): LinearLayout {
        return LinearLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(56)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(6), 0, dp(18), 0)
            setBackgroundColor(android.graphics.Color.parseColor("#101010"))
            addView(TextView(this@PersonalProfileActivity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(56), LinearLayout.LayoutParams.MATCH_PARENT)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "‹"
                setTextColor(android.graphics.Color.parseColor("#F2F5FA"))
                textSize = 34f
                setOnClickListener { finish() }
            })
            addView(TextView(this@PersonalProfileActivity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "个人资料"
                setTextColor(android.graphics.Color.parseColor("#F2F5FA"))
                textSize = 20f
            })
            addView(View(this@PersonalProfileActivity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(56), 1)
            })
        }
    }

    private fun renderRows() {
        profile = UserProfileStore.load(this)
        val current = profile ?: return
        rows.removeAllViews()
        rows.addView(
            UserProfileViews.row(
                context = this,
                title = "头像",
                trailing = UserProfileViews.createAvatarView(this, current, 46, 18f),
                onClick = ::openAvatarPicker
            )
        )
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "名字", current.displayName) { showEditDialog("名字", current.displayName) { saveProfile(displayName = it) } })
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "手机号", maskPhone(current.phone)))
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "账号", current.wechatId))
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "我的二维码", ""))
        rows.addView(UserProfileViews.spacer(this, 10))
        rows.addView(UserProfileViews.row(this, "地区", "未设置"))
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "签名", current.signature) { showEditDialog("签名", current.signature) { saveProfile(signature = it) } })
        rows.addView(UserProfileViews.spacer(this, 10))
        rows.addView(UserProfileViews.row(this, "拍一拍", ""))
        rows.addView(UserProfileViews.divider(this))
        rows.addView(UserProfileViews.row(this, "来电铃声", "默认"))
    }

    private fun openAvatarPicker() {
        runCatching {
            avatarPicker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
        }.onFailure {
            Toast.makeText(this, "无法打开相册", Toast.LENGTH_SHORT).show()
        }
    }

    private fun showEditDialog(title: String, initial: String, onSave: (String) -> Unit) {
        val input = EditText(this).apply {
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            setSingleLine(title != "签名")
            maxLines = if (title == "签名") 3 else 1
            setText(initial)
            setSelection(text.length)
        }
        AlertDialog.Builder(this)
            .setTitle("编辑$title")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存") { _, _ ->
                val value = input.text?.toString()?.trim().orEmpty()
                if (value.isBlank()) {
                    Toast.makeText(this, "$title 不能为空", Toast.LENGTH_SHORT).show()
                } else {
                    onSave(value)
                }
            }
            .show()
    }

    private fun saveProfile(
        displayName: String? = null,
        avatarDataUrl: String? = profile?.avatarDataUrl,
        signature: String? = null
    ) {
        val current = profile ?: UserProfileStore.load(this)
        UserProfileStore.save(
            context = this,
            displayName = displayName ?: current.displayName,
            avatarDataUrl = avatarDataUrl,
            signature = signature ?: current.signature
        )
        if (displayName != null && AuthManager.isLoggedIn(this)) {
            AuthManager.updateNickname(this, displayName)
            syncDisplayName(displayName)
        }
        if (avatarDataUrl != null && AuthManager.isLoggedIn(this)) {
            syncAvatar(avatarDataUrl)
        }
        setResult(Activity.RESULT_OK)
        renderRows()
    }

    private fun syncAvatar(avatarDataUrl: String) {
        thread(name = "avatar-sync") {
            val result = runCatching {
                syncAvatarToServer(http, serverUrl, this, avatarDataUrl)
            }
            runOnUiThread {
                result.onFailure {
                    Toast.makeText(this, "头像已本机保存，云端同步失败：${it.message}", Toast.LENGTH_LONG).show()
                }
            }
        }
    }

    private fun syncDisplayName(displayName: String) {
        thread(name = "profile-name-sync") {
            val result = runCatching {
                val body = JSONObject()
                    .put("nickname", displayName)
                    .toString()
                    .toRequestBody("application/json".toMediaType())
                val request = AuthManager.applyAuth(
                    this,
                    Request.Builder()
                        .url("$serverUrl/api/me/profile")
                        .patch(body)
                ).build()
                http.newCall(request).execute().use { response ->
                    val text = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(text, "云端资料同步失败"))
                    val nickname = JSONObject(text)
                        .optJSONObject("user")
                        ?.optString("nickname", "")
                        ?.trim()
                        .orEmpty()
                    if (nickname.isNotEmpty()) AuthManager.updateNickname(this, nickname)
                }
            }
            runOnUiThread {
                result.onFailure {
                    Toast.makeText(this, "名字已本机保存，云端同步失败：${it.message}", Toast.LENGTH_LONG).show()
                }
            }
        }
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density).toInt()

    private fun maskPhone(phone: String?): String =
        phone?.takeIf { it.length >= 7 }?.let { "${it.take(3)}****${it.takeLast(2)}" } ?: "未绑定"
}
