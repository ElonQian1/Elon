package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * "AI 记住的你"卡片 —— 在"我的"页展示 AI 长期记忆，支持单条删除。
 * 调用 GET /api/memories 读取，DELETE /api/memories/:id 删除。
 */
internal class UserMemoriesCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0

    private lateinit var statusPill: TextView
    private lateinit var memoryListContainer: LinearLayout

    // ─── 公开 API ────────────────────────────────────────────────────────────

    fun attachAndRefresh() {
        val host = binding.profileMemoriesContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    // ─── 刷新 ────────────────────────────────────────────────────────────────

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            statusPill.text = "未登录"
            statusPill.visibility = android.view.View.VISIBLE
            memoryListContainer.removeAllViews()
            return
        }
        val serial = ++loadSerial
        statusPill.text = "加载中…"
        statusPill.visibility = android.view.View.VISIBLE
        memoryListContainer.removeAllViews()

        thread(name = "user-memories-card") {
            val result = runCatching { fetchMemories() }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result
                    .onSuccess { items ->
                        statusPill.visibility = android.view.View.GONE
                        if (items.isEmpty()) {
                            memoryListContainer.addView(buildEmptyHint())
                        } else {
                            statusPill.text = "${items.size} 条"
                            statusPill.visibility = android.view.View.VISIBLE
                            items.forEach { item ->
                                memoryListContainer.addView(buildMemoryRow(item))
                            }
                        }
                    }
                    .onFailure {
                        statusPill.text = "暂不可用"
                        statusPill.visibility = android.view.View.VISIBLE
                    }
            }
        }
    }

    // ─── 网络 ────────────────────────────────────────────────────────────────

    private data class MemoryItem(
        val id: String,
        val content: String,
        val category: String,
        val importance: Int
    )

    private fun fetchMemories(): List<MemoryItem> {
        val req = AuthManager.applyAuth(
            activity,
            Request.Builder().url("$serverUrl/api/memories?limit=30").get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val arr = JSONObject(body).optJSONArray("memories") ?: return emptyList()
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            MemoryItem(
                id = o.optString("id", ""),
                content = o.optString("content", ""),
                category = o.optString("category", "fact"),
                importance = o.optInt("importance", 5)
            )
        }
    }

    private fun deleteMemory(id: String): Boolean {
        val req = AuthManager.applyAuth(
            activity,
            Request.Builder().url("$serverUrl/api/memories/$id").delete()
        ).build()
        return http.newCall(req).execute().use { it.isSuccessful }
    }

    private fun createMemory(content: String, category: String): Boolean {
        val json = org.json.JSONObject().apply {
            put("content", content.trim())
            put("category", category)
            put("importance", 6)
        }.toString()
        val body = json.toByteArray(Charsets.UTF_8).let {
            okhttp3.RequestBody.create(
                okhttp3.MediaType.parse("application/json; charset=utf-8"), it
            )
        }
        val req = AuthManager.applyAuth(
            activity,
            Request.Builder().url("$serverUrl/api/memories").post(body)
        ).build()
        return http.newCall(req).execute().use { it.isSuccessful }
    }

    // ─── UI 构建 ─────────────────────────────────────────────────────────────

    private fun buildCard(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(0, dp(10), 0, 0) }
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), dp(16), dp(22), dp(16))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#0E1318"))
                cornerRadius = dp(8).toFloat()
            }

            // 标题行
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                    includeFontPadding = false
                    text = "AI 记住的你"
                    setTextColor(Color.parseColor("#E6EEF8"))
                    textSize = 15f
                    setTypeface(typeface, Typeface.BOLD)
                })

                statusPill = TextView(activity).apply {
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setPadding(dp(10), dp(4), dp(10), dp(4))
                    text = "加载中…"
                    textSize = 11f
                    setTextColor(Color.parseColor("#7ECFFF"))
                    background = GradientDrawable().apply {
                        setColor(Color.parseColor("#0C2D40"))
                        cornerRadius = dp(8).toFloat()
                    }
                }
                addView(statusPill)

                // + 按钮
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.marginStart = dp(8) }
                    includeFontPadding = false
                    text = "+"
                    textSize = 20f
                    setTextColor(Color.parseColor("#7ECFFF"))
                    setPadding(dp(6), 0, dp(2), 0)
                    setOnClickListener { showAddDialog() }
                })
            })

            // 副标题说明
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(4) }
                includeFontPadding = false
                text = "AI 在对话中自动记录，让每次回答更贴合你"
                textSize = 11f
                setTextColor(Color.parseColor("#4B5563"))
            })

            // 记忆列表
            memoryListContainer = LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(10) }
                orientation = LinearLayout.VERTICAL
            }
            addView(memoryListContainer)
        }
    }

    private fun buildMemoryRow(item: MemoryItem): LinearLayout = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(6) }
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(12), dp(9), dp(8), dp(9))
        background = GradientDrawable().apply {
            setColor(Color.parseColor("#141B22"))
            cornerRadius = dp(6).toFloat()
        }

        // 分类色条
        addView(android.view.View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(3), LinearLayout.LayoutParams.MATCH_PARENT).also {
                it.marginEnd = dp(10)
            }
            background = GradientDrawable().apply {
                setColor(Color.parseColor(categoryColor(item.category)))
                cornerRadius = dp(2).toFloat()
            }
        })

        // 正文
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            includeFontPadding = false
            text = item.content
            textSize = 13f
            setTextColor(Color.parseColor("#C8D8EE"))
            maxLines = 3
            ellipsize = android.text.TextUtils.TruncateAt.END
        })

        // 删除按钮
        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(32), dp(32)).also {
                it.marginStart = dp(6)
            }
            setImageResource(android.R.drawable.ic_menu_close_clear_cancel)
            setColorFilter(Color.parseColor("#4B5563"))
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(6), dp(6), dp(6), dp(6))
            background = null
            setOnClickListener {
                confirmDelete(item)
            }
        })
    }

    private fun buildEmptyHint(): TextView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(6); it.bottomMargin = dp(4) }
        includeFontPadding = false
        text = "还没有记忆\n多和 AI 对话，它会逐渐记住你的偏好"
        textSize = 12f
        setTextColor(Color.parseColor("#4B5563"))
        gravity = Gravity.CENTER
        setPadding(0, dp(8), 0, dp(8))
    }

    // ─── 删除确认 ─────────────────────────────────────────────────────────────

    private fun confirmDelete(item: MemoryItem) {
        AlertDialog.Builder(activity)
            .setTitle("删除这条记忆？")
            .setMessage("「${item.content}」")
            .setPositiveButton("删除") { _, _ -> doDelete(item) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doDelete(item: MemoryItem) {
        thread(name = "delete-memory") {
            val ok = runCatching { deleteMemory(item.id) }.getOrDefault(false)
            activity.runOnUiThread {
                if (ok) {
                    refresh()
                } else {
                    Toast.makeText(activity, "删除失败，请重试", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    // ─── 手动添加 ─────────────────────────────────────────────────────────────

    private fun showAddDialog() {
        val input = android.widget.EditText(activity).apply {
            hint = "例如：我是 Rust 开发者，偏好完整代码"
            maxLines = 3
            inputType = android.text.InputType.TYPE_CLASS_TEXT or
                    android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE
            setPadding(dp(16), dp(12), dp(16), dp(12))
            setTextColor(Color.parseColor("#E6EEF8"))
            setHintTextColor(Color.parseColor("#4B5563"))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#141B22"))
                cornerRadius = dp(6).toFloat()
            }
        }
        // 分类选择 Spinner
        val categories = arrayOf("fact - 事实", "preference - 偏好", "profile - 个人信息", "goal - 目标")
        val categoryKeys = arrayOf("fact", "preference", "profile", "goal")
        val spinner = android.widget.Spinner(activity).apply {
            adapter = android.widget.ArrayAdapter(activity,
                android.R.layout.simple_spinner_dropdown_item, categories)
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = dp(10) }
        }
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(8), dp(20), dp(4))
            addView(input)
            addView(spinner)
        }
        AlertDialog.Builder(activity)
            .setTitle("主动告诉 AI")
            .setView(container)
            .setPositiveButton("保存") { _, _ ->
                val text = input.text.toString().trim()
                val category = categoryKeys[spinner.selectedItemPosition]
                if (text.isNotEmpty()) doCreate(text, category)
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doCreate(content: String, category: String) {
        thread(name = "create-memory") {
            val ok = runCatching { createMemory(content, category) }.getOrDefault(false)
            activity.runOnUiThread {
                if (ok) {
                    refresh()
                } else {
                    Toast.makeText(activity, "保存失败，请重试", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    // ─── 工具 ────────────────────────────────────────────────────────────────

    private fun categoryColor(category: String): String = when (category) {
        "preference" -> "#7ECFFF"
        "profile"    -> "#4ADE80"
        "goal"       -> "#FCD34D"
        else         -> "#6B7280"  // fact
    }

    private fun dp(n: Int): Int =
        (n * activity.resources.displayMetrics.density + 0.5f).toInt()
}
