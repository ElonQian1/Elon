package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Request
import kotlin.concurrent.thread

/**
 * 我的页全站 PC 节点概览。
 * 这里展示的是 `/api/nodes` 全局发现列表，不是当前账号提供的节点。
 */
internal class ProfileNodeDirectoryCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val openMarket: () -> Unit
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0

    private lateinit var statusLabel: TextView
    private lateinit var onlineValue: TextView
    private lateinit var readyValue: TextView
    private lateinit var modelValue: TextView
    private lateinit var previewContainer: LinearLayout

    fun attachAndRefresh() {
        val host = binding.profileNodeDirectoryContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            renderSummary("--", "--", "--")
            statusLabel.text = "未登录"
            previewContainer.removeAllViews()
            previewContainer.addView(buildEmptyHint("登录后可浏览全站在线 PC 节点"))
            return
        }
        val serial = ++loadSerial
        renderSummary("…", "…", "…")
        statusLabel.text = "加载中…"
        previewContainer.removeAllViews()
        val ctx = activity.applicationContext
        thread(name = "profile-node-directory") {
            val result = runCatching { fetchNodes(ctx) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result
                    .onSuccess { renderNodes(it) }
                    .onFailure {
                        renderSummary("--", "--", "--")
                        statusLabel.text = "暂不可用"
                        previewContainer.removeAllViews()
                        previewContainer.addView(buildEmptyHint("节点列表加载失败"))
                    }
            }
        }
    }

    private fun fetchNodes(ctx: Context): List<NodeMarketNode> {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder().url("$serverUrl/api/nodes").get()
        ).build()
        val resp = http.newCall(req).execute()
        val body = resp.use { it.body?.string() ?: "{}" }
        if (!resp.isSuccessful) error(apiError(body, resp.code))
        return NodeMarketCatalog.parseNodes(body)
    }

    private fun renderNodes(nodes: List<NodeMarketNode>) {
        val summary = NodeMarketCatalog.summarize(nodes)
        renderSummary(
            summary.onlineNodes.toString(),
            summary.projectReadyNodes.toString(),
            summary.modelCount.toString()
        )
        statusLabel.text = if (summary.onlineNodes > 0) "在线 ${summary.onlineNodes}" else "暂无在线"
        statusLabel.setTextColor(Color.parseColor(if (summary.onlineNodes > 0) "#58BE6A" else "#6F7785"))
        (statusLabel.background as? GradientDrawable)?.setColor(
            Color.parseColor(if (summary.onlineNodes > 0) "#152C3E" else "#181B20")
        )

        previewContainer.removeAllViews()
        if (nodes.isEmpty()) {
            previewContainer.addView(buildEmptyHint("暂无在线 PC 节点"))
            return
        }
        nodes.sortedWith(
            compareByDescending<NodeMarketNode> { it.canAcceptProject }
                .thenByDescending { it.models.isNotEmpty() }
                .thenBy { it.displayName }
        ).take(2).forEach { node ->
            previewContainer.addView(buildNodePreview(node))
        }
        if (nodes.size > 2) {
            previewContainer.addView(TextView(activity).apply {
                includeFontPadding = false
                text = "还有 ${nodes.size - 2} 台节点，点击查看全部"
                textSize = 12f
                setTextColor(Color.parseColor("#6091CF"))
                setPadding(0, dp(8), 0, 0)
            })
        }
    }

    private fun renderSummary(online: String, ready: String, models: String) {
        onlineValue.text = online
        readyValue.text = ready
        modelValue.text = models
    }

    private fun buildCard(): LinearLayout {
        lateinit var card: LinearLayout
        card = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(0, dp(10), 0, 0) }
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), dp(18), dp(22), dp(16))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#181B20"))
                cornerRadius = dp(8).toFloat()
            }
            isClickable = true
            isFocusable = true
            setOnClickListener { openMarket() }

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                    includeFontPadding = false
                    text = "全站 PC 节点"
                    setTextColor(Color.parseColor("#F2F5FA"))
                    textSize = 16f
                    setTypeface(typeface, Typeface.BOLD)
                })
                statusLabel = TextView(activity).apply {
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setPadding(dp(10), dp(4), dp(10), dp(4))
                    text = "加载中…"
                    textSize = 11f
                    setTextColor(Color.parseColor("#81B3D9"))
                    background = GradientDrawable().apply {
                        setColor(Color.parseColor("#152C3E"))
                        cornerRadius = dp(8).toFloat()
                    }
                }
                addView(statusLabel)
            })

            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(14) }
                orientation = LinearLayout.HORIZONTAL
                addView(buildMetricBlock("在线节点") { onlineValue = it })
                addView(View(activity).apply { layoutParams = LinearLayout.LayoutParams(0, 1, 1f) })
                addView(buildMetricBlock("可接项目") { readyValue = it })
                addView(View(activity).apply { layoutParams = LinearLayout.LayoutParams(0, 1, 1f) })
                addView(buildMetricBlock("可用模型") { modelValue = it })
            })

            previewContainer = LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(12) }
                orientation = LinearLayout.VERTICAL
            }
            addView(previewContainer)

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(10) }
                includeFontPadding = false
                text = "进入 PC 节点大厅  →"
                setTextColor(Color.parseColor("#6091CF"))
                textSize = 13f
            })
        }
        return card
    }

    private fun buildMetricBlock(label: String, bindValue: (TextView) -> Unit): LinearLayout {
        val value = TextView(activity).apply {
            includeFontPadding = false
            text = "…"
            setTextColor(Color.parseColor("#F2F5FA"))
            textSize = 24f
            setTypeface(typeface, Typeface.BOLD)
        }
        bindValue(value)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = label
                setTextColor(Color.parseColor("#6F7785"))
                textSize = 11f
            })
            addView(value)
        }
    }

    private fun buildNodePreview(node: NodeMarketNode): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(12), dp(10), dp(12), dp(10))
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#0F1217"))
                cornerRadius = dp(6).toFloat()
            }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = dp(6) }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "${node.displayName} · ${node.capacityLabel.ifBlank { if (node.canAcceptProject) "可接项目" else "在线" }}"
                textSize = 13f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = nodePreviewDetail(node)
                textSize = 11f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(4), 0, 0)
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        }
    }

    private fun buildEmptyHint(textValue: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = dp(6) }
            includeFontPadding = false
            text = textValue
            textSize = 12f
            setTextColor(Color.parseColor("#6F7785"))
            gravity = Gravity.CENTER
            setPadding(0, dp(8), 0, dp(6))
        }
    }

    private fun nodePreviewDetail(node: NodeMarketNode): String {
        val modelText = if (node.models.isNotEmpty()) "模型 ${node.models.size}" else "暂无模型"
        val cliText = node.allowedClis.takeIf { it.isNotEmpty() }?.joinToString("/")?.let { "CLI $it" }
        val hardware = node.hardwareSummary.takeIf { it.isNotBlank() && it != "硬件未知" }
        return listOfNotNull(modelText, cliText, hardware, node.capacityWarnings.firstOrNull())
            .joinToString(" · ")
    }

    private fun apiError(body: String, code: Int): String {
        return runCatching {
            org.json.JSONObject(body).optString("error").ifBlank { "HTTP $code" }
        }.getOrDefault("HTTP $code")
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density + 0.5f).toInt()
}
