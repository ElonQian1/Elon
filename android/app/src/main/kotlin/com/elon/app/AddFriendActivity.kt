package com.elon.app

import android.content.Context
import android.content.Intent
import android.content.res.ColorStateList
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import kotlin.concurrent.thread

class AddFriendActivity : AppCompatActivity() {
    private val http = OkHttpClient()
    private val serverUrl get() = ServerUrlManager.getActive(this)
    private val recommendations = mutableListOf<AddFriendRecommendation>()
    private var qrBitmap: Bitmap? = null
    private lateinit var searchInput: EditText
    private lateinit var recommendationScroll: ScrollView
    private lateinit var recommendationList: LinearLayout
    private lateinit var resultText: TextView
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK
        if (!AuthManager.isLoggedIn(this)) {
            Toast.makeText(this, "添加好友需要先登录账号", Toast.LENGTH_SHORT).show()
            startActivity(Intent(this, LoginActivity::class.java))
            finish()
            return
        }
        setContentView(buildContent())
        loadRecommendations()
    }

    override fun onDestroy() {
        qrBitmap?.recycle()
        qrBitmap = null
        super.onDestroy()
    }
    private fun buildContent(): View {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.BLACK)
            addView(topBar(), LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(50)))
            addView(searchBar(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(40)
            ).apply {
                leftMargin = dp(16)
                rightMargin = dp(16)
                topMargin = dp(6)
            })
            addView(ScrollView(this@AddFriendActivity).apply {
                overScrollMode = View.OVER_SCROLL_NEVER
                addView(pageBody())
            }, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f))
        }
    }
    private fun topBar(): FrameLayout {
        return FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(TextView(this@AddFriendActivity).apply {
                text = "‹"
                gravity = Gravity.CENTER
                includeFontPadding = false
                textSize = 27f
                setTextColor(Color.parseColor("#D9D9D9"))
                isClickable = true
                isFocusable = true
                foreground = selectableForeground()
                contentDescription = "返回"
                setOnClickListener { finish() }
            }, FrameLayout.LayoutParams(dp(50), FrameLayout.LayoutParams.MATCH_PARENT).apply {
                gravity = Gravity.START or Gravity.CENTER_VERTICAL
            })
            addView(TextView(this@AddFriendActivity).apply {
                text = "添加朋友"
                gravity = Gravity.CENTER
                includeFontPadding = false
                textSize = 16f
                setTextColor(Color.parseColor("#D9D9D9"))
            }, FrameLayout.LayoutParams(FrameLayout.LayoutParams.WRAP_CONTENT, FrameLayout.LayoutParams.MATCH_PARENT).apply {
                gravity = Gravity.CENTER
            })
        }
    }
    private fun searchBar(): LinearLayout {
        searchInput = EditText(this).apply {
            hint = "搜索账号/手机"
            setSingleLine(true)
            textSize = 16f
            imeOptions = EditorInfo.IME_ACTION_SEARCH
            setTextColor(Color.parseColor("#D9D9D9"))
            setHintTextColor(Color.parseColor("#777777"))
            backgroundTintList = ColorStateList.valueOf(Color.TRANSPARENT)
            background = null
            includeFontPadding = false
            setPadding(dp(8), 0, 0, 0)
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                    renderRecommendations()
                }
                override fun afterTextChanged(s: Editable?) = Unit
            })
            setOnEditorActionListener { view, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_SEARCH) {
                    hideKeyboard(view)
                    true
                } else {
                    false
                }
            }
        }
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            background = roundedRect("#272727", 20)
            setPadding(dp(14), 0, dp(14), 0)
            addView(ImageView(this@AddFriendActivity).apply {
                setImageResource(R.drawable.ic_search_simple)
                imageTintList = ColorStateList.valueOf(Color.parseColor("#777777"))
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(22), dp(22)))
            addView(searchInput, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        }
    }

    private fun pageBody(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(36), dp(16), dp(44))
            addView(TextView(this@AddFriendActivity).apply {
                text = "推荐"
                includeFontPadding = false
                textSize = 16f
                setTextColor(Color.parseColor("#D9D9D9"))
            })
            recommendationList = LinearLayout(this@AddFriendActivity).apply {
                orientation = LinearLayout.VERTICAL
            }
            recommendationScroll = ScrollView(this@AddFriendActivity).apply {
                overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
                setBackgroundColor(Color.BLACK)
                addView(recommendationList)
            }
            addView(recommendationScroll, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0
            ).apply {
                topMargin = dp(22)
            })
            resultText = TextView(this@AddFriendActivity).apply {
                text = "正在加载推荐好友..."
                minHeight = dp(24)
                textSize = 13f
                setTextColor(Color.parseColor("#777777"))
            }
            addView(resultText, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(6)
            })
            addView(scanButton(), LinearLayout.LayoutParams(dp(112), dp(44)).apply {
                gravity = Gravity.CENTER_HORIZONTAL
                topMargin = dp(48)
            })
            addView(qrCard(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.CENTER_HORIZONTAL
                topMargin = dp(36)
            })
        }
    }

    private fun recommendationRow(item: AddFriendRecommendation): LinearLayout {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            minimumHeight = dp(76)
        }
        row.addView(avatarView(item), LinearLayout.LayoutParams(dp(44), dp(44)))
        row.addView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            addView(TextView(this@AddFriendActivity).apply {
                text = item.name
                includeFontPadding = false
                maxLines = 1
                textSize = 16f
                setTextColor(Color.parseColor("#D9D9D9"))
            })
            addView(TextView(this@AddFriendActivity).apply {
                text = item.account
                includeFontPadding = false
                maxLines = 1
                textSize = 13f
                setTextColor(Color.parseColor("#777777"))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(4)
            })
        }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
            leftMargin = dp(12)
            rightMargin = dp(10)
        })
        if (item.mutualFriendCount > 0) {
            row.addView(mutualFriendView(item.mutualFriendCount), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                rightMargin = dp(10)
            })
        }
        row.addView(addButton(item), LinearLayout.LayoutParams(dp(64), dp(36)))
        return row
    }

    private fun avatarView(item: AddFriendRecommendation): View {
        val bitmap = UserProfileStore.decodeAvatar(item.avatarDataUrl)
        return ImageView(this).apply {
            background = roundedRect("#D9D9D9", 6)
            scaleType = ImageView.ScaleType.CENTER_CROP
            contentDescription = "${item.name}头像"
            bitmap?.let { setImageBitmap(it) }
        }
    }

    private fun mutualFriendView(count: Int): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(ImageView(this@AddFriendActivity).apply {
                setImageResource(R.drawable.ic_add_friend_mutual)
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(24), dp(18)))
            addView(TextView(this@AddFriendActivity).apply {
                text = "${count}名共同好友"
                includeFontPadding = false
                textSize = 12f
                setTextColor(Color.parseColor("#777777"))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(4)
            })
        }
    }

    private fun addButton(item: AddFriendRecommendation): TextView {
        return TextView(this).apply {
            text = if (item.alreadyFriend) "已添加" else "添加"
            gravity = Gravity.CENTER
            includeFontPadding = false
            textSize = 16f
            setTypeface(typeface, Typeface.BOLD)
            isEnabled = !item.alreadyFriend
            alpha = if (item.alreadyFriend) 0.48f else 1f
            setTextColor(Color.BLACK)
            background = roundedRect("#D9D9D9", 18)
            isClickable = !item.alreadyFriend
            isFocusable = !item.alreadyFriend
            foreground = selectableForeground()
            setOnClickListener { addRecommendedFriend(item, this) }
        }
    }

    private fun scanButton(): LinearLayout {
        return LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            background = roundedRect("#D9D9D9", 22)
            addView(ImageView(this@AddFriendActivity).apply {
                setImageResource(R.drawable.ic_add_friend_scan)
                imageTintList = ColorStateList.valueOf(Color.BLACK)
                contentDescription = null
            }, LinearLayout.LayoutParams(dp(24), dp(24)))
            addView(TextView(this@AddFriendActivity).apply {
                text = "扫一扫"
                includeFontPadding = false
                textSize = 16f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.BLACK)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(8)
            })
        }
    }

    private fun qrCard(): LinearLayout {
        val qrSize = (resources.displayMetrics.widthPixels - dp(112)).coerceIn(dp(220), dp(320))
        qrBitmap = QrCodeBitmap.create(UserProfileStore.personalQrPayload(this), qrSize)
        return LinearLayout(this).apply {
            background = roundedRect("#FFFFFF", 12)
            setPadding(dp(10), dp(10), dp(10), dp(10))
            addView(ImageView(this@AddFriendActivity).apply {
                setImageBitmap(qrBitmap)
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = "我的一龙账号二维码"
            }, LinearLayout.LayoutParams(qrSize, qrSize))
        }
    }

    private fun loadRecommendations() {
        resultText.text = "正在加载推荐好友..."
        thread(name = "add-friend-recommendations") {
            val result = runCatching {
                val request = AuthManager.applyAuth(
                    this,
                    Request.Builder()
                        .url("$serverUrl/api/me/friends/recommendations")
                        .get()
                ).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "推荐好友加载失败"))
                    val array = JSONObject(body).optJSONArray("recommendations") ?: org.json.JSONArray()
                    List(array.length()) { index ->
                        parseRecommendation(array.optJSONObject(index) ?: JSONObject())
                    }
                }
            }
            runOnUiThread {
                result.fold(
                    onSuccess = {
                        recommendations.clear()
                        recommendations.addAll(it)
                        renderRecommendations()
                    },
                    onFailure = {
                        resultText.text = it.message ?: "推荐好友加载失败"
                        resultText.setTextColor(Color.parseColor("#D97A7A"))
                    }
                )
            }
        }
    }

    private fun renderRecommendations() {
        if (!::recommendationList.isInitialized) return
        recommendationList.removeAllViews()
        val query = searchInput.text?.toString()?.trim()?.lowercase().orEmpty()
        val items = recommendations.filter { item ->
            query.isBlank() ||
                item.name.lowercase().contains(query) ||
                item.account.lowercase().contains(query) ||
                item.phone.orEmpty().lowercase().contains(query) ||
                item.id.lowercase().contains(query)
        }
        items.forEach { item ->
            recommendationList.addView(recommendationRow(item), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(76)
            ))
        }
        (recommendationScroll.layoutParams as? LinearLayout.LayoutParams)?.let {
            it.height = dp(items.size.coerceAtMost(12) * 76)
            recommendationScroll.layoutParams = it
        }
        resultText.setTextColor(Color.parseColor("#777777"))
        resultText.text = when {
            recommendations.isEmpty() -> "暂无可推荐的注册用户"
            items.isEmpty() -> "没有匹配的注册用户"
            else -> ""
        }
    }

    private fun addRecommendedFriend(item: AddFriendRecommendation, button: TextView) {
        button.isEnabled = false
        button.alpha = 0.55f
        thread(name = "add-recommended-friend") {
            val result = runCatching {
                val payload = JSONObject()
                    .put("search_type", "account_id")
                    .put("query", item.id)
                    .toString()
                    .toRequestBody("application/json".toMediaType())
                val request = AuthManager.applyAuth(
                    this,
                    Request.Builder()
                        .url("$serverUrl/api/me/friends")
                        .post(payload)
                ).build()
                http.newCall(request).execute().use { response ->
                    val body = response.body?.string().orEmpty()
                    if (!response.isSuccessful) error(readErrorMessage(body, "添加失败"))
                    JSONObject(body).optBoolean("already_friend", false)
                }
            }
            runOnUiThread {
                result.fold(
                    onSuccess = { alreadyFriend ->
                        val index = recommendations.indexOfFirst { it.id == item.id }
                        if (index >= 0) {
                            recommendations[index] = recommendations[index].copy(alreadyFriend = true)
                        }
                        setResult(RESULT_OK)
                        Toast.makeText(
                            this,
                            if (alreadyFriend) "已经是好友：${item.name}" else "已添加好友：${item.name}",
                            Toast.LENGTH_SHORT
                        ).show()
                        renderRecommendations()
                    },
                    onFailure = {
                        button.isEnabled = true
                        button.alpha = 1f
                        Toast.makeText(this, it.message ?: "添加失败", Toast.LENGTH_SHORT).show()
                    }
                )
            }
        }
    }

    private fun parseRecommendation(json: JSONObject): AddFriendRecommendation {
        val id = json.optString("id", "").trim()
        val account = json.optString("account", "").trim().ifBlank { id }
        val phone = json.optString("phone", "").trim().takeIf { it.isNotEmpty() }
        val nickname = json.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
        return AddFriendRecommendation(
            id = id,
            name = nickname ?: account.ifBlank { "朋友名称" },
            account = account,
            phone = phone,
            avatarDataUrl = json.optString("avatar_data_url", "").trim().takeIf { it.isNotEmpty() },
            mutualFriendCount = json.optInt("mutual_friend_count", 0).coerceAtLeast(0),
            alreadyFriend = json.optBoolean("already_friend", false)
        )
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }

    private fun hideKeyboard(view: View) {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(view.windowToken, 0)
    }

    private fun selectableForeground(): Drawable? {
        val out = TypedValue()
        theme.resolveAttribute(android.R.attr.selectableItemBackground, out, true)
        return ContextCompat.getDrawable(this, out.resourceId)
    }

    private fun roundedRect(fillColor: String, radiusDp: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(Color.parseColor(fillColor))
            cornerRadius = dp(radiusDp).toFloat()
        }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private data class AddFriendRecommendation(
        val id: String,
        val name: String,
        val account: String,
        val phone: String?,
        val avatarDataUrl: String?,
        val mutualFriendCount: Int,
        val alreadyFriend: Boolean
    )
}
