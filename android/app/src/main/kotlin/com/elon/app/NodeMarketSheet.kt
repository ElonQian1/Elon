package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

// ─── 数据模型 ─────────────────────────────────────────────────────────────────

internal data class NodeModel(
    val modelId: String,
    val displayName: String,
    val nodeId: String,
    val nodeOwner: String,
    val contextLen: Int,
    val pricePerK: Double
)

internal data class NodeBalance(
    val balance: Double,
    val lifetime: Double
)

// ─── 主类 ─────────────────────────────────────────────────────────────────────

/**
 * 节点算力市场：
 * - 展示积分余额
 * - 展示在线节点提供的模型列表
 * - 点击模型可向该节点发送一条消息，返回结果显示在对话框
 */
internal class NodeMarketSheet(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String
) {

    private fun dp(n: Int): Int = (n * activity.resources.displayMetrics.density + 0.5f).toInt()

    // ─── 入口 ───────────────────────────────────────────────────────────────

    fun show() {
        val dialog = AlertDialog.Builder(activity, R.style.Theme_AppCompat_DayNight_Dialog)
            .setView(buildRootView())
            .create()
        dialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            (activity.resources.displayMetrics.heightPixels * 0.85).toInt()
        )
        dialog.window?.setBackgroundDrawable(
            GradientDrawable().apply {
                setColor(Color.parseColor("#151515"))
                cornerRadius = dp(14).toFloat()
            }
        )
        dialog.show()
    }

    // ─── 根布局 ─────────────────────────────────────────────────────────────

    private fun buildRootView(): View {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#151515"))
        }

        // 标题栏
        root.addView(buildHeader())

        // 积分余额条
        val balanceText = buildBalanceBar()
        root.addView(balanceText)

        // 分割线
        root.addView(buildDivider())

        // 模型列表区（含 loading）
        val listContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        val spinner = ProgressBar(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(64)
            )
            indeterminateTintList = ColorStateList.valueOf(Color.parseColor("#7070FF"))
        }
        listContainer.addView(spinner)

        val scroll = ScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0, 1f
            )
            addView(listContainer)
        }
        root.addView(scroll)

        // 异步加载余额和模型列表
        loadBalanceAndModels(balanceText, spinner, listContainer)

        return root
    }

    // ─── 头部 ───────────────────────────────────────────────────────────────

    private fun buildHeader(): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(22), dp(18), dp(16), dp(14))

            addView(TextView(activity).apply {
                text = "节点算力市场"
                textSize = 18f
                setTextColor(Color.parseColor("#E8E8E8"))
                typeface = Typeface.DEFAULT_BOLD
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })

            addView(TextView(activity).apply {
                text = "✕"
                textSize = 20f
                setTextColor(Color.parseColor("#888888"))
                setPadding(dp(10), dp(4), dp(10), dp(4))
            })
        }
    }

    // ─── 积分余额条 ─────────────────────────────────────────────────────────

    private fun buildBalanceBar(): TextView {
        return TextView(activity).apply {
            text = "积分余额：加载中…"
            textSize = 13f
            setTextColor(Color.parseColor("#AAAAAA"))
            setPadding(dp(22), dp(8), dp(22), dp(8))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun buildDivider(): View {
        return View(activity).apply {
            setBackgroundColor(Color.parseColor("#252525"))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(1)
            ).also { it.setMargins(0, dp(4), 0, dp(4)) }
        }
    }

    // ─── 网络加载 ───────────────────────────────────────────────────────────

    private fun loadBalanceAndModels(
        balanceText: TextView,
        spinner: ProgressBar,
        listContainer: LinearLayout
    ) {
        val ctx = activity.applicationContext
        thread(name = "node-market-load") {
            // 1. 余额
            val balance = runCatching { fetchBalance(ctx) }.getOrNull()
            // 2. 模型列表
            val models = runCatching { fetchModels() }.getOrElse { emptyList() }

            activity.runOnUiThread {
                if (activity.isFinishing || activity.isDestroyed) return@runOnUiThread

                // 更新余额
                if (balance != null) {
                    balanceText.text = "积分余额：${formatBalance(balance.balance)}  |  累计收益：${formatBalance(balance.lifetime)}"
                } else {
                    balanceText.text = if (AuthManager.isLoggedIn(ctx)) "积分余额：加载失败" else "积分余额：未登录"
                }

                // 移除 spinner
                listContainer.removeView(spinner)

                // 渲染列表
                if (models.isEmpty()) {
                    listContainer.addView(buildEmptyHint())
                } else {
                    // 标题
                    listContainer.addView(buildSectionTitle("在线模型  (${models.size})"))
                    models.forEach { model ->
                        listContainer.addView(buildModelCard(model))
                    }
                    listContainer.addView(buildBottomPad())
                }
            }
        }
    }

    // ─── 网络请求 ───────────────────────────────────────────────────────────

    private fun fetchBalance(ctx: Context): NodeBalance {
        val req = AuthManager.applyAuth(ctx,
            Request.Builder().url("$serverUrl/api/me/node-balance").get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val json = JSONObject(body)
        return NodeBalance(
            balance = json.optDouble("balance", 0.0),
            lifetime = json.optDouble("lifetime_earned", 0.0)
        )
    }

    private fun fetchModels(): List<NodeModel> {
        val req = Request.Builder().url("$serverUrl/api/nodes/models").get().build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "[]" }
        val arr = JSONArray(body)
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            NodeModel(
                modelId = o.optString("model_id"),
                displayName = o.optString("display_name", o.optString("model_id")),
                nodeId = o.optString("node_id"),
                nodeOwner = o.optString("owner_user_id", ""),
                contextLen = o.optInt("context_len", 2048),
                pricePerK = o.optDouble("price_per_1k_credits", 1.0)
            )
        }
    }

    private fun sendNodeChat(model: NodeModel, message: String): String {
        val ctx = activity.applicationContext
        val body = JSONObject().apply {
            put("model_id", model.modelId)
            put("messages", JSONArray().apply {
                put(JSONObject().apply {
                    put("role", "user")
                    put("content", message)
                })
            })
            put("max_tokens", 512)
        }.toString().toRequestBody("application/json".toMediaType())

        val req = AuthManager.applyAuth(ctx,
            Request.Builder()
                .url("$serverUrl/api/nodes/chat")
                .post(body)
        ).build()

        val resp = http.newCall(req).execute()
        val respBody = resp.use { it.body?.string() ?: "{}" }
        if (!resp.isSuccessful) {
            val errMsg = runCatching { JSONObject(respBody).optString("error", respBody) }.getOrDefault(respBody)
            throw RuntimeException(errMsg)
        }
        val json = JSONObject(respBody)
        val content = json.optString("content", "")
        val promptTokens = json.optInt("prompt_tokens", 0)
        val completionTokens = json.optInt("completion_tokens", 0)
        return "$content\n\n─────\n消耗：prompt $promptTokens + completion $completionTokens tokens"
    }

    // ─── UI 组件 ─────────────────────────────────────────────────────────────

    private fun buildSectionTitle(text: String): View {
        return TextView(activity).apply {
            this.text = text
            textSize = 12f
            setTextColor(Color.parseColor("#666666"))
            setPadding(dp(22), dp(14), dp(22), dp(6))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun buildModelCard(model: NodeModel): View {
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), dp(14), dp(22), dp(14))
            isClickable = true
            isFocusable = true
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#1A1A1A"))
                cornerRadius = dp(8).toFloat()
            }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(dp(12), dp(4), dp(12), dp(4)) }
        }

        // 模型名称行
        val nameRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        nameRow.addView(TextView(activity).apply {
            text = model.displayName.ifBlank { model.modelId }
            textSize = 15f
            setTextColor(Color.parseColor("#D8D8D8"))
            typeface = Typeface.DEFAULT_BOLD
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        })
        nameRow.addView(buildPill("${model.pricePerK} /千词", "#2A4A3A", "#50C878"))
        card.addView(nameRow)

        // 节点 & 上下文信息
        card.addView(TextView(activity).apply {
            text = "节点：${model.nodeId.take(8)}… · 上下文 ${model.contextLen} tokens"
            textSize = 12f
            setTextColor(Color.parseColor("#666666"))
            setPadding(0, dp(4), 0, 0)
        })

        card.setOnClickListener { openChatDialog(model) }
        return card
    }

    private fun buildPill(text: String, bgHex: String, textHex: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 11f
            setTextColor(Color.parseColor(textHex))
            setPadding(dp(8), dp(3), dp(8), dp(3))
            background = GradientDrawable().apply {
                setColor(Color.parseColor(bgHex))
                cornerRadius = dp(10).toFloat()
            }
        }
    }

    private fun buildEmptyHint(): View {
        return TextView(activity).apply {
            text = "暂无在线节点\n部署 PC 节点可在此列表中显示"
            textSize = 14f
            setTextColor(Color.parseColor("#555555"))
            gravity = Gravity.CENTER
            setPadding(dp(22), dp(40), dp(22), dp(40))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun buildBottomPad(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, dp(32)
            )
        }
    }

    // ─── 向节点发消息的对话框 ─────────────────────────────────────────────────

    private fun openChatDialog(model: NodeModel) {
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录", Toast.LENGTH_SHORT).show()
            return
        }

        val wrapper = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(8), dp(20), dp(4))
        }

        val modelLabel = TextView(activity).apply {
            text = "模型：${model.displayName.ifBlank { model.modelId }}"
            textSize = 13f
            setTextColor(Color.parseColor("#888888"))
            setPadding(0, 0, 0, dp(10))
        }
        wrapper.addView(modelLabel)

        val input = EditText(activity).apply {
            hint = "输入你的问题…"
            setHintTextColor(Color.parseColor("#555555"))
            setTextColor(Color.parseColor("#E0E0E0"))
            setBackgroundColor(Color.parseColor("#1A1A1A"))
            setPadding(dp(12), dp(10), dp(12), dp(10))
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            minLines = 2
            maxLines = 5
        }
        wrapper.addView(input)

        val progressRow = FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(40)
            )
            visibility = View.GONE
        }
        val progress = ProgressBar(activity).apply {
            layoutParams = FrameLayout.LayoutParams(dp(32), dp(32), Gravity.CENTER)
            indeterminateTintList = ColorStateList.valueOf(Color.parseColor("#7070FF"))
        }
        progressRow.addView(progress)
        wrapper.addView(progressRow)

        val resultText = TextView(activity).apply {
            textSize = 14f
            setTextColor(Color.parseColor("#D0D0D0"))
            setPadding(0, dp(12), 0, 0)
            visibility = View.GONE
        }
        wrapper.addView(resultText)

        val dialog = AlertDialog.Builder(activity)
            .setTitle("向节点提问")
            .setView(wrapper)
            .setPositiveButton("发送", null)
            .setNegativeButton("取消", null)
            .create()

        dialog.setOnShowListener {
            val sendBtn = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
            sendBtn.setOnClickListener {
                val msg = input.text.toString().trim()
                if (msg.isEmpty()) {
                    Toast.makeText(activity, "请输入问题", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                // 禁用输入，显示 loading
                input.isEnabled = false
                sendBtn.isEnabled = false
                progressRow.visibility = View.VISIBLE
                resultText.visibility = View.GONE

                thread(name = "node-chat") {
                    val result = runCatching { sendNodeChat(model, msg) }
                    activity.runOnUiThread {
                        if (activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                        progressRow.visibility = View.GONE
                        input.isEnabled = true
                        sendBtn.isEnabled = true
                        result.onSuccess { reply ->
                            resultText.text = reply
                            resultText.visibility = View.VISIBLE
                        }.onFailure { err ->
                            Toast.makeText(activity, "请求失败：${err.message}", Toast.LENGTH_LONG).show()
                        }
                    }
                }
            }
        }

        dialog.show()
    }

    // ─── 工具 ────────────────────────────────────────────────────────────────

    private fun formatBalance(v: Double): String {
        return if (v == v.toLong().toDouble()) "${v.toLong()}" else String.format("%.2f", v)
    }
}
