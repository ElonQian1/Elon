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
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * 我的节点状态卡片 —— 展示本用户当前注册的节点机器（在线/离线）及其支持的模型数量。
 * 点击某个节点行可直接打开算力市场并选中该节点。
 */
internal class MyNodesCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0

    private lateinit var statusPill: TextView
    private lateinit var nodeListContainer: LinearLayout

    // ─── 公开 API ────────────────────────────────────────────────────────────

    fun attachAndRefresh() {
        val host = binding.profileMyNodesContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    // ─── 刷新逻辑 ────────────────────────────────────────────────────────────

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            statusPill.text = "未登录"
            statusPill.visibility = View.VISIBLE
            nodeListContainer.removeAllViews()
            return
        }
        val serial = ++loadSerial
        statusPill.text = "加载中…"
        statusPill.visibility = View.VISIBLE
        nodeListContainer.removeAllViews()

        val ctx = activity.applicationContext
        thread(name = "my-nodes-card") {
            val result = runCatching { fetchMyNodes(ctx) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result
                    .onSuccess { nodes ->
                        statusPill.visibility = View.GONE
                        if (nodes.isEmpty()) {
                            nodeListContainer.addView(buildEmptyHint())
                        } else {
                            val online = nodes.count { it.online }
                            statusPill.text = if (online > 0) "在线 $online/${nodes.size}" else "全部离线"
                            statusPill.setTextColor(Color.parseColor(if (online > 0) "#58BE6A" else "#6F7785"))
                            val pillBg = statusPill.background as? GradientDrawable
                            pillBg?.setColor(Color.parseColor(if (online > 0) "#152C3E" else "#181B20"))
                            statusPill.visibility = View.VISIBLE
                            nodes.forEach { node ->
                                nodeListContainer.addView(buildNodeRow(node))
                            }
                        }
                    }
                    .onFailure {
                        statusPill.text = "暂不可用"
                        statusPill.visibility = View.VISIBLE
                    }
            }
        }
    }

    // ─── 网络 ────────────────────────────────────────────────────────────────

    private data class NodeItem(
        val nodeId: String,
        val models: List<String>,
        val connectedAt: Long,
        val online: Boolean
    )

    private fun fetchMyNodes(ctx: Context): List<NodeItem> {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder().url("$serverUrl/api/me/nodes").get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val arr: JSONArray = JSONObject(body).optJSONArray("nodes") ?: return emptyList()
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            val modelsArr = o.optJSONArray("models") ?: JSONArray()
            val modelIds = (0 until modelsArr.length()).map { j ->
                modelsArr.getJSONObject(j).optString("model_id", "")
            }.filter { it.isNotEmpty() }
            NodeItem(
                nodeId = o.optString("node_id", ""),
                models = modelIds,
                connectedAt = o.optLong("connected_at", 0),
                online = o.optBoolean("online", false)
            )
        }
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
                setColor(Color.parseColor("#0F1217"))
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
                    text = "我的节点"
                    setTextColor(Color.parseColor("#F2F5FA"))
                    textSize = 15f
                    setTypeface(typeface, Typeface.BOLD)
                })

                statusPill = TextView(activity).apply {
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
                addView(statusPill)
            })

            // 节点列表
            nodeListContainer = LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(10) }
                orientation = LinearLayout.VERTICAL
            }
            addView(nodeListContainer)
        }
    }

    private fun buildNodeRow(node: NodeItem): LinearLayout = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(8) }
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(12), dp(10), dp(12), dp(10))
        background = GradientDrawable().apply {
            setColor(Color.parseColor("#181B20"))
            cornerRadius = dp(6).toFloat()
        }

        // 状态指示圆点
        addView(View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(8), dp(8)).also {
                it.marginEnd = dp(10)
            }
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(Color.parseColor(if (node.online) "#58BE6A" else "#6F7785"))
            }
        })

        // 节点 ID（缩短显示）+ 模型列表
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            orientation = LinearLayout.VERTICAL

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = formatNodeId(node.nodeId)
                textSize = 13f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor(if (node.online) "#F2F5FA" else "#6F7785"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })

            if (node.models.isNotEmpty()) {
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.topMargin = dp(2) }
                    includeFontPadding = false
                    text = node.models.take(3).joinToString("  ·  ") +
                            if (node.models.size > 3) "  +${node.models.size - 3}" else ""
                    textSize = 11f
                    setTextColor(Color.parseColor("#6F7785"))
                    maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                })
            }
        })

        // 模型数量 badge
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = "${node.models.size} 模型"
            textSize = 11f
            setTextColor(Color.parseColor(if (node.online) "#81B3D9" else "#6F7785"))
            gravity = Gravity.CENTER
        })
    }

    private fun buildEmptyHint(): TextView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(8); it.bottomMargin = dp(4) }
        includeFontPadding = false
        text = "暂无注册节点\n运行 elon-node-agent 即可在此显示"
        textSize = 12f
        setTextColor(Color.parseColor("#6F7785"))
        gravity = Gravity.CENTER
        setPadding(0, dp(8), 0, dp(8))
    }

    // ─── 工具 ────────────────────────────────────────────────────────────────

    private fun formatNodeId(id: String): String {
        // 节点 ID 格式通常是 "node-XXXX" 或 UUID，截取后半段方便阅读
        return if (id.length > 16) "…${id.takeLast(14)}" else id
    }

    private fun dp(n: Int): Int =
        (n * activity.resources.displayMetrics.density + 0.5f).toInt()
}
