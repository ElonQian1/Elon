package com.elon.app

import android.app.AlertDialog
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * 节点积分流水明细弹窗：
 * - 列出本用户作为节点提供者的最近 50 条收益记录
 * - 每条显示：模型名、消耗 token 数、收益积分、时间
 */
internal class NodeTransactionSheet(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String
) {

    private fun dp(n: Int): Int = (n * activity.resources.displayMetrics.density + 0.5f).toInt()

    // ─── 入口 ───────────────────────────────────────────────────────────────

    fun show() {
        val dialog = AlertDialog.Builder(activity)
            .setView(buildRootView())
            .create()
        dialog.window?.setLayout(
            ViewGroup.LayoutParams.MATCH_PARENT,
            (activity.resources.displayMetrics.heightPixels * 0.80).toInt()
        )
        dialog.window?.setBackgroundDrawable(
            GradientDrawable().apply {
                setColor(Color.parseColor("#0F1217"))
                cornerRadius = dp(14).toFloat()
            }
        )
        dialog.show()
    }

    // ─── 主视图 ─────────────────────────────────────────────────────────────

    private fun buildRootView(): LinearLayout {
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(20), dp(20), dp(20))
        }

        // 标题行
        root.addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.bottomMargin = dp(16) }

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                text = "积分流水明细"
                textSize = 18f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
                includeFontPadding = false
            })

            addView(TextView(activity).apply {
                text = "最近 50 条"
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                includeFontPadding = false
            })
        })

        // 分隔线
        root.addView(divider())

        // 内容区（Spinner 占位，加载后替换）
        val spinner = ProgressBar(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also {
                it.gravity = Gravity.CENTER_HORIZONTAL
                it.topMargin = dp(32)
                it.bottomMargin = dp(32)
            }
        }
        val scroll = ScrollView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0,
                1f
            )
        }
        val list = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        scroll.addView(list)

        root.addView(spinner)
        root.addView(scroll)
        scroll.visibility = android.view.View.GONE

        // 后台加载
        val ctx = activity.applicationContext
        thread(name = "node-tx-sheet") {
            val result = runCatching { fetchTransactions(ctx) }
            activity.runOnUiThread {
                if (activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                spinner.visibility = android.view.View.GONE
                scroll.visibility = android.view.View.VISIBLE
                result
                    .onSuccess { txs ->
                        if (txs.isEmpty()) {
                            list.addView(emptyLabel())
                        } else {
                            txs.forEachIndexed { idx, tx ->
                                if (idx > 0) list.addView(divider())
                                list.addView(buildTxRow(tx))
                            }
                        }
                    }
                    .onFailure {
                        list.addView(errorLabel("加载失败：${it.message ?: "未知错误"}"))
                    }
            }
        }

        return root
    }

    // ─── 每行记录 ────────────────────────────────────────────────────────────

    private fun buildTxRow(tx: NodeTx): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(0, dp(12), 0, dp(12))

        // 第一行：模型名  +  +X积分
        addView(LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                text = tx.modelId
                textSize = 14f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
                includeFontPadding = false
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })

            addView(TextView(activity).apply {
                text = "+${formatCredits(tx.settledCredits)} 积分"
                textSize = 14f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#58BE6A"))
                includeFontPadding = false
            })
        })

        // 第二行：节点 ID（短）  |  tokens  |  时间
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = dp(4) }
            val shortNode = if (tx.nodeId.length > 12) tx.nodeId.takeLast(8) else tx.nodeId
            text = "节点 $shortNode  ·  ${tx.promptTokens + tx.completionTokens} tokens  ·  ${formatTime(tx.createdAt)}"
            textSize = 11f
            setTextColor(Color.parseColor("#6F7785"))
            includeFontPadding = false
        })
    }

    // ─── 辅助视图 ────────────────────────────────────────────────────────────

    private fun divider() = android.view.View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, dp(1)
        )
        setBackgroundColor(Color.parseColor("#1E2126"))
    }

    private fun emptyLabel() = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(40); it.bottomMargin = dp(40) }
        text = "暂无流水记录"
        textSize = 14f
        gravity = Gravity.CENTER
        setTextColor(Color.parseColor("#6F7785"))
    }

    private fun errorLabel(msg: String) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(40); it.bottomMargin = dp(40) }
        text = msg
        textSize = 13f
        gravity = Gravity.CENTER
        setTextColor(Color.parseColor("#EF4444"))
    }

    // ─── 数据获取 ────────────────────────────────────────────────────────────

    private data class NodeTx(
        val nodeId: String,
        val modelId: String,
        val promptTokens: Int,
        val completionTokens: Int,
        val settledCredits: Double,
        val createdAt: String
    )

    private fun fetchTransactions(ctx: Context): List<NodeTx> {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder().url("$serverUrl/api/me/node-transactions").get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val arr = JSONObject(body).optJSONArray("transactions") ?: return emptyList()
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            NodeTx(
                nodeId = o.optString("node_id", ""),
                modelId = o.optString("model_id", ""),
                promptTokens = o.optInt("prompt_tokens", 0),
                completionTokens = o.optInt("completion_tokens", 0),
                settledCredits = o.optDouble("settled_credits", 0.0),
                createdAt = o.optString("created_at", "")
            )
        }
    }

    // ─── 格式化 ──────────────────────────────────────────────────────────────

    private fun formatCredits(v: Double): String = when {
        v >= 1_000_000 -> String.format("%.1fM", v / 1_000_000)
        v >= 1_000     -> String.format("%.1fK", v / 1_000)
        v >= 1.0       -> String.format("%.2f", v)
        else           -> String.format("%.4f", v)
    }

    /** 将 ISO8601 时间戳转换为简短显示文字 */
    private fun formatTime(ts: String): String {
        if (ts.length < 16) return ts
        // ts 格式: "2026-06-01T12:34:56.789" 或 "2026-06-01 12:34:56"
        val normalized = ts.replace('T', ' ')
        return normalized.substring(5, 16)   // "06-01 12:34"
    }
}
