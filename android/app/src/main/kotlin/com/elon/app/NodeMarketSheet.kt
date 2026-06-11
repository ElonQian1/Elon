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
import java.util.Locale
import kotlin.concurrent.thread

// ─── 主类 ─────────────────────────────────────────────────────────────────────

/**
 * PC 节点大厅：
 * - 展示积分余额
 * - 展示全局在线 PC 节点和节点提供的模型列表
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
        lateinit var dialog: AlertDialog
        val root = buildRootView { dialog.dismiss() }
        dialog = AlertDialog.Builder(activity)
            .setView(root)
            .create()
        dialog.show()
        dialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            (activity.resources.displayMetrics.heightPixels * 0.85).toInt()
        )
        dialog.window?.setBackgroundDrawable(
            GradientDrawable().apply {
                setColor(Color.parseColor("#0F1217"))
                cornerRadius = dp(14).toFloat()
            }
        )
    }

    // ─── 根布局 ─────────────────────────────────────────────────────────────

    private fun buildRootView(onClose: () -> Unit): View {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.parseColor("#0F1217"))
        }

        // 标题栏
        root.addView(buildHeader(onClose))

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

    private fun buildHeader(onClose: () -> Unit): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(22), dp(18), dp(16), dp(14))

            addView(TextView(activity).apply {
                text = "PC 节点大厅"
                textSize = 18f
                setTextColor(Color.parseColor("#F2F5FA"))
                typeface = Typeface.DEFAULT_BOLD
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            })

            addView(TextView(activity).apply {
                text = "✕"
                textSize = 20f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(dp(10), dp(4), dp(10), dp(4))
                isClickable = true
                isFocusable = true
                setOnClickListener { onClose() }
            })
        }
    }

    // ─── 积分余额条 ─────────────────────────────────────────────────────────

    private fun buildBalanceBar(): TextView {
        return TextView(activity).apply {
            text = "积分余额：加载中…"
            textSize = 13f
            setTextColor(Color.parseColor("#A6AFBD"))
            setPadding(dp(22), dp(8), dp(22), dp(8))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun buildDivider(): View {
        return View(activity).apply {
            setBackgroundColor(Color.parseColor("#283140"))
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
            // 2. 全局 PC 节点目录
            val nodeResult = if (AuthManager.isLoggedIn(ctx)) {
                runCatching { fetchNodes(ctx) }
            } else {
                Result.failure(IllegalStateException("请先登录后查看 PC 节点"))
            }

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

                nodeResult
                    .onSuccess { nodes -> renderNodes(listContainer, nodes) }
                    .onFailure { err -> listContainer.addView(buildErrorHint(err.message ?: "加载 PC 节点失败")) }
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

    private fun fetchNodes(ctx: Context): List<NodeMarketNode> {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder().url("$serverUrl/api/nodes").get()
        ).build()
        val resp = http.newCall(req).execute()
        val body = resp.use { it.body?.string() ?: "{}" }
        if (!resp.isSuccessful) {
            val err = runCatching { JSONObject(body).optString("error") }.getOrNull()
            throw RuntimeException(err?.takeIf { it.isNotBlank() } ?: "HTTP ${resp.code}")
        }
        return NodeMarketCatalog.parseNodes(body)
    }

    private fun sendNodeChat(model: NodeModel, message: String): String {
        val ctx = activity.applicationContext
        val body = JSONObject().apply {
            put("model_id", model.modelId)
            if (model.nodeId.isNotBlank()) put("node_id", model.nodeId)
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

    private fun renderNodes(listContainer: LinearLayout, nodes: List<NodeMarketNode>) {
        if (nodes.isEmpty()) {
            listContainer.addView(buildEmptyHint())
            return
        }
        listContainer.addView(buildSectionTitle("在线 PC 节点  (${nodes.size})"))
        nodes.sortedWith(
            compareByDescending<NodeMarketNode> { it.canAcceptProject }
                .thenByDescending { it.models.isNotEmpty() }
                .thenBy { it.displayName.lowercase(Locale.US) }
        ).forEach { node ->
            listContainer.addView(buildNodeCard(node))
        }

        val models = nodes.flatMap { it.models }
        if (models.isNotEmpty()) {
            listContainer.addView(buildSectionTitle("可调用模型  (${models.size})"))
            models.forEach { model ->
                listContainer.addView(buildModelCard(model))
            }
        }
        listContainer.addView(buildBottomPad())
    }

    private fun buildSectionTitle(text: String): View {
        return TextView(activity).apply {
            this.text = text
            textSize = 12f
            setTextColor(Color.parseColor("#6F7785"))
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
                setColor(Color.parseColor("#181B20"))
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
            setTextColor(Color.parseColor("#F2F5FA"))
            typeface = Typeface.DEFAULT_BOLD
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        })
        nameRow.addView(buildPill("${model.pricePerK} /千词", "#2A4A3A", "#50C878"))
        card.addView(nameRow)

        // 节点 & 上下文信息
        card.addView(TextView(activity).apply {
            text = "${model.nodeDisplayName} · ${model.nodeCapacityLabel} · 上下文 ${model.contextLen} tokens"
            textSize = 12f
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(0, dp(4), 0, 0)
        })
        if (model.nodeHardwareSummary.isNotBlank() && model.nodeHardwareSummary != "硬件未知") {
            card.addView(TextView(activity).apply {
                text = model.nodeHardwareSummary
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(3), 0, 0)
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        }

        card.setOnClickListener { openChatDialog(model) }
        return card
    }

    private fun buildNodeCard(node: NodeMarketNode): View {
        val card = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(16), dp(13), dp(16), dp(13))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#181B20"))
                cornerRadius = dp(8).toFloat()
                when (node.capacityTone.lowercase(Locale.US)) {
                    "bad" -> setStroke(dp(1), Color.parseColor("#784242"))
                    "warn" -> setStroke(dp(1), Color.parseColor("#6A5628"))
                }
            }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(dp(12), dp(4), dp(12), dp(4)) }
        }

        val titleRow = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        titleRow.addView(View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(8), dp(8)).also { it.marginEnd = dp(10) }
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor(if (node.online) "#58BE6A" else "#6F7785"))
            }
        })
        titleRow.addView(TextView(activity).apply {
            text = node.displayName
            textSize = 15f
            setTextColor(Color.parseColor("#F2F5FA"))
            typeface = Typeface.DEFAULT_BOLD
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        })
        titleRow.addView(buildPill(nodeStatusLabel(node), nodeStatusBg(node), nodeStatusColor(node)))
        card.addView(titleRow)

        card.addView(TextView(activity).apply {
            text = nodeSubtitle(node)
            textSize = 12f
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(0, dp(7), 0, 0)
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        })

        val metrics = listOfNotNull(
            nodeProjectSlotsText(node),
            formatBytes(node.diskFreeBytes).takeIf { it.isNotBlank() }?.let { "磁盘 $it" },
            node.hardwareSummary.takeIf { it.isNotBlank() && it != "硬件未知" },
            node.allowedClis.takeIf { it.isNotEmpty() }?.joinToString("/")?.let { "CLI $it" },
            node.capacityWarnings.firstOrNull()
        )
        card.addView(TextView(activity).apply {
            text = metrics.joinToString(" · ").ifBlank { "硬件与容量待上报" }
            textSize = 12f
            setTextColor(nodeMetricColor(node))
            setPadding(0, dp(5), 0, 0)
            maxLines = 2
            ellipsize = android.text.TextUtils.TruncateAt.END
        })

        val modelText = if (node.models.isNotEmpty()) {
            node.models.take(4).joinToString("  ·  ") { it.displayName.ifBlank { it.modelId } } +
                if (node.models.size > 4) "  +${node.models.size - 4}" else ""
        } else {
            "暂无在线模型"
        }
        card.addView(TextView(activity).apply {
            text = modelText
            textSize = 12f
            setTextColor(Color.parseColor(if (node.models.isNotEmpty()) "#81B3D9" else "#6F7785"))
            setPadding(0, dp(5), 0, 0)
            maxLines = 1
            ellipsize = android.text.TextUtils.TruncateAt.END
        })
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
            text = "暂无在线 PC 节点\n节点上线后会在这里显示"
            textSize = 14f
            setTextColor(Color.parseColor("#6F7785"))
            gravity = Gravity.CENTER
            setPadding(dp(22), dp(40), dp(22), dp(40))
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
    }

    private fun buildErrorHint(message: String): View {
        return TextView(activity).apply {
            text = message
            textSize = 14f
            setTextColor(Color.parseColor("#D97A7A"))
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
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(0, 0, 0, dp(10))
        }
        wrapper.addView(modelLabel)

        val input = EditText(activity).apply {
            hint = "输入你的问题…"
            setHintTextColor(Color.parseColor("#6F7785"))
            setTextColor(Color.parseColor("#F2F5FA"))
            setBackgroundColor(Color.parseColor("#181B20"))
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
            setTextColor(Color.parseColor("#F2F5FA"))
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

    private fun nodeSubtitle(node: NodeMarketNode): String {
        return if (node.deviceName.isNotBlank() && !node.deviceName.equals(node.displayName, ignoreCase = true)) {
            "设备: ${node.deviceName} · ID: ${node.shortId}"
        } else {
            "ID: ${node.shortId}"
        }
    }

    private fun nodeStatusLabel(node: NodeMarketNode): String {
        return node.capacityLabel.ifBlank {
            when {
                !node.online -> "离线"
                node.canAcceptProject -> "可接项目"
                else -> "在线"
            }
        }
    }

    private fun nodeStatusBg(node: NodeMarketNode): String {
        return when (node.capacityTone.lowercase(Locale.US)) {
            "ok" -> "#152C3E"
            "bad" -> "#2A1F1F"
            "warn" -> "#283140"
            else -> "#152C3E"
        }
    }

    private fun nodeStatusColor(node: NodeMarketNode): String {
        return when (node.capacityTone.lowercase(Locale.US)) {
            "ok" -> "#58BE6A"
            "bad" -> "#E99191"
            "warn" -> "#F7D28A"
            else -> "#81B3D9"
        }
    }

    private fun nodeMetricColor(node: NodeMarketNode): Int {
        return Color.parseColor(
            when (node.capacityTone.lowercase(Locale.US)) {
                "bad" -> "#E99191"
                "warn" -> "#F7D28A"
                else -> "#6F7785"
            }
        )
    }

    private fun nodeProjectSlotsText(node: NodeMarketNode): String {
        return if (node.projectLimit > 0) {
            "项目 ${node.projectCount}/${node.projectLimit}，剩余 ${node.projectSlotsRemaining.coerceAtLeast(0)}"
        } else {
            "项目 ${node.projectCount}"
        }
    }

    private fun formatBytes(value: Long?): String {
        val bytes = value ?: return ""
        if (bytes <= 0L) return ""
        val units = listOf("B", "KB", "MB", "GB", "TB")
        var amount = bytes.toDouble()
        var index = 0
        while (amount >= 1024.0 && index < units.lastIndex) {
            amount /= 1024.0
            index += 1
        }
        return if (index >= 3) {
            String.format(Locale.US, "%.1f %s", amount, units[index])
        } else {
            "${amount.toInt()} ${units[index]}"
        }
    }
}
