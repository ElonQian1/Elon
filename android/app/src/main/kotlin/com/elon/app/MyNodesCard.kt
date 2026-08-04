package com.elon.app

import android.content.Context
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
import java.util.Locale
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
    private lateinit var refreshButton: TextView
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

    fun refresh() {
        val serial = ++loadSerial
        setRefreshLoading(true)
        if (!AuthManager.isLoggedIn(activity)) {
            statusPill.text = "未登录"
            statusPill.setTextColor(activity.elonColor(R.color.elon_text_tertiary))
            (statusPill.background as? GradientDrawable)?.setColor(
                activity.elonColor(R.color.elon_badge_neutral_bg)
            )
            statusPill.visibility = View.VISIBLE
            nodeListContainer.removeAllViews()
            setRefreshLoading(false)
            return
        }
        statusPill.text = "加载中…"
        statusPill.setTextColor(activity.elonColor(R.color.elon_status_success))
        (statusPill.background as? GradientDrawable)?.setColor(
            activity.elonColor(R.color.elon_badge_success_bg)
        )
        statusPill.visibility = View.VISIBLE

        val ctx = activity.applicationContext
        thread(name = "my-nodes-card") {
            val result = runCatching { fetchMyNodes(ctx) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result
                    .onSuccess { nodes ->
                        setRefreshLoading(false)
                        nodeListContainer.removeAllViews()
                        if (nodes.isEmpty()) {
                            statusPill.text = "无节点"
                            statusPill.setTextColor(activity.elonColor(R.color.elon_text_tertiary))
                            (statusPill.background as? GradientDrawable)?.setColor(
                                activity.elonColor(R.color.elon_badge_neutral_bg)
                            )
                            statusPill.visibility = View.VISIBLE
                            nodeListContainer.addView(buildEmptyHint())
                        } else {
                            val online = nodes.count { it.online }
                            statusPill.text = if (online > 0) "在线 $online/${nodes.size}" else "全部离线"
                            statusPill.setTextColor(
                                activity.elonColor(
                                    if (online > 0) R.color.elon_status_success else R.color.elon_text_tertiary
                                )
                            )
                            val pillBg = statusPill.background as? GradientDrawable
                            pillBg?.setColor(
                                activity.elonColor(
                                    if (online > 0) R.color.elon_badge_success_bg else R.color.elon_badge_neutral_bg
                                )
                            )
                            statusPill.visibility = View.VISIBLE
                            nodes.forEach { node ->
                                nodeListContainer.addView(buildNodeRow(node))
                            }
                        }
                    }
                    .onFailure {
                        setRefreshLoading(false)
                        statusPill.text = "暂不可用"
                        statusPill.setTextColor(activity.elonColor(R.color.elon_text_tertiary))
                        (statusPill.background as? GradientDrawable)?.setColor(
                            activity.elonColor(R.color.elon_badge_neutral_bg)
                        )
                        statusPill.visibility = View.VISIBLE
                    }
            }
        }
    }

    // ─── 网络 ────────────────────────────────────────────────────────────────

    private data class NodeItem(
        val nodeId: String,
        val displayName: String,
        val shortId: String,
        val deviceName: String,
        val models: List<String>,
        val connectedAt: Long,
        val online: Boolean,
        val projectCount: Int,
        val projectLimit: Int,
        val projectSlotsRemaining: Int,
        val capacityLabel: String,
        val capacityTone: String,
        val capacityWarnings: List<String>,
        val diskFreeBytes: Long?
    )

    private fun fetchMyNodes(ctx: Context): List<NodeItem> {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder()
                .url("$serverUrl/api/me/nodes")
                .header("Cache-Control", "no-cache")
                .get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val arr: JSONArray = JSONObject(body).optJSONArray("nodes") ?: return emptyList()
        return (0 until arr.length()).map { i ->
            val o = arr.getJSONObject(i)
            val nodeId = o.optString("node_id", o.optString("agent_id", ""))
            val shortId = o.optString("short_id").ifBlank { formatNodeId(nodeId) }
            val deviceName = o.optString("device_name").trim()
            val displayName = o.optString("display_name")
                .ifBlank { o.optString("label") }
                .ifBlank { deviceName }
                .ifBlank { shortId }
            val modelsArr = o.optJSONArray("models") ?: JSONArray()
            val modelIds = (0 until modelsArr.length()).map { j ->
                modelsArr.getJSONObject(j).optString("model_id", "")
            }.filter { it.isNotEmpty() }
            val warningsArr = o.optJSONArray("capacity_warnings") ?: JSONArray()
            val warnings = (0 until warningsArr.length()).mapNotNull { j ->
                warningsArr.optString(j).trim().takeIf { it.isNotBlank() }
            }
            val projectCount = o.optInt("project_count", 0).coerceAtLeast(0)
            val projectLimit = o.optInt("project_limit", 0).coerceAtLeast(0)
            NodeItem(
                nodeId = nodeId,
                displayName = displayName,
                shortId = shortId,
                deviceName = deviceName,
                models = modelIds,
                connectedAt = o.optLong("connected_at", 0),
                online = o.optBoolean("online", false),
                projectCount = projectCount,
                projectLimit = projectLimit,
                projectSlotsRemaining = o.optInt(
                    "project_slots_remaining",
                    (projectLimit - projectCount).coerceAtLeast(0)
                ).coerceAtLeast(0),
                capacityLabel = o.optString("capacity_label").trim(),
                capacityTone = o.optString("capacity_tone").trim(),
                capacityWarnings = warnings,
                diskFreeBytes = if (o.has("disk_free_bytes") && !o.isNull("disk_free_bytes")) {
                    o.optLong("disk_free_bytes").takeIf { it > 0L }
                } else {
                    null
                }
            )
        }
    }

    // ─── UI 构建 ─────────────────────────────────────────────────────────────

    private fun buildCard(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(0, dp(18), 0, 0) }
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, 0)

            addView(buildSectionDivider())

            // 标题行
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                minimumHeight = dp(48)

                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                    includeFontPadding = false
                    text = "我的节点"
                    setTextColor(activity.elonColor(R.color.elon_text_primary))
                    textSize = 15f
                    setTypeface(typeface, Typeface.BOLD)
                })

                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL

                    statusPill = TextView(this@MyNodesCard.activity).apply {
                        includeFontPadding = false
                        gravity = Gravity.CENTER
                        setPadding(dp(10), dp(4), dp(10), dp(4))
                        text = "加载中…"
                        textSize = 11f
                        setTextColor(activity.elonColor(R.color.elon_status_success))
                        background = GradientDrawable().apply {
                            setColor(activity.elonColor(R.color.elon_badge_success_bg))
                            cornerRadius = dp(8).toFloat()
                        }
                    }
                    addView(statusPill)

                    refreshButton = TextView(this@MyNodesCard.activity).apply {
                        layoutParams = LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.WRAP_CONTENT,
                            LinearLayout.LayoutParams.WRAP_CONTENT
                        ).also { it.marginStart = dp(8) }
                        includeFontPadding = false
                        gravity = Gravity.CENTER
                        minWidth = dp(56)
                        minHeight = dp(48)
                        setPadding(dp(12), 0, dp(12), 0)
                        text = "刷新"
                        textSize = 11f
                        setTextColor(activity.elonColor(R.color.elon_status_success))
                        isClickable = true
                        isFocusable = true
                        contentDescription = "刷新我的节点状态"
                        setOnClickListener { refresh() }
                    }
                    addView(refreshButton)
                })
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
            setColor(activity.elonColor(R.color.elon_surface_header))
            cornerRadius = dp(6).toFloat()
            when (node.capacityTone.lowercase(Locale.US)) {
                "bad" -> setStroke(dp(1), activity.elonColor(R.color.elon_status_danger))
                "warn" -> setStroke(dp(1), activity.elonColor(R.color.elon_status_project))
            }
        }

        // 状态指示圆点
        addView(View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(8), dp(8)).also {
                it.marginEnd = dp(10)
            }
            background = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setColor(
                    activity.elonColor(
                        if (node.online) R.color.elon_status_success else R.color.elon_text_tertiary
                    )
                )
            }
        })

        // 节点展示名 + 设备/短 ID + 模型列表
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            orientation = LinearLayout.VERTICAL

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = node.displayName
                textSize = 13f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(
                    activity.elonColor(
                        if (node.online) R.color.elon_text_primary else R.color.elon_text_tertiary
                    )
                )
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(2) }
                includeFontPadding = false
                text = nodeSubtitle(node)
                textSize = 11f
                setTextColor(activity.elonColor(R.color.elon_text_tertiary))
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
                    setTextColor(activity.elonColor(R.color.elon_text_tertiary))
                    maxLines = 1
                    ellipsize = android.text.TextUtils.TruncateAt.END
                })
            } else {
                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).also { it.topMargin = dp(2) }
                    includeFontPadding = false
                    text = "暂无可用模型"
                    textSize = 11f
                    setTextColor(activity.elonColor(R.color.elon_text_tertiary))
                    maxLines = 1
                })
            }

            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(4) }
                includeFontPadding = false
                text = nodeCapacityLine(node)
                textSize = 11f
                setTextColor(nodeCapacityTextColor(node))
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        })

        // 容量 badge
        addView(TextView(activity).apply {
            includeFontPadding = false
            text = nodeCapacityBadge(node)
            textSize = 11f
            setTextColor(nodeCapacityTextColor(node))
            gravity = Gravity.CENTER
        })
    }

    private fun buildEmptyHint(): TextView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).also { it.topMargin = dp(8); it.bottomMargin = dp(4) }
        includeFontPadding = false
        text = "暂无你提供的节点\n所有在线节点请看 PC 节点大厅"
        textSize = 12f
        setTextColor(activity.elonColor(R.color.elon_text_tertiary))
        gravity = Gravity.CENTER
        setPadding(0, dp(8), 0, dp(8))
    }

    // ─── 工具 ────────────────────────────────────────────────────────────────

    private fun formatNodeId(id: String): String {
        // 节点 ID 格式通常是 "node-XXXX" 或 UUID，截取后半段方便阅读
        return if (id.length > 16) "…${id.takeLast(14)}" else id
    }

    private fun nodeSubtitle(node: NodeItem): String {
        val idText = node.shortId.ifBlank { formatNodeId(node.nodeId) }
        return if (node.deviceName.isNotBlank() && !node.deviceName.equals(node.displayName, ignoreCase = true)) {
            "设备: ${node.deviceName} · ID: $idText"
        } else {
            "ID: $idText"
        }
    }

    private fun nodeCapacityBadge(node: NodeItem): String {
        return node.capacityLabel.ifBlank {
            if (node.online) "容量未知" else "离线"
        }
    }

    private fun nodeCapacityLine(node: NodeItem): String {
        val slotText = if (node.projectLimit > 0) {
            "项目 ${node.projectCount}/${node.projectLimit}，剩余 ${node.projectSlotsRemaining.coerceAtLeast(0)}"
        } else {
            "项目 ${node.projectCount}"
        }
        val diskText = formatBytes(node.diskFreeBytes).takeIf { it.isNotBlank() }?.let { "磁盘 $it" }
        val warning = node.capacityWarnings.firstOrNull()
        return listOfNotNull(slotText, diskText, warning).joinToString(" · ")
    }

    private fun nodeCapacityTextColor(node: NodeItem): Int {
        if (!node.online) return activity.elonColor(R.color.elon_text_tertiary)
        return when (node.capacityTone.lowercase(Locale.US)) {
            "ok" -> activity.elonColor(R.color.elon_status_success)
            "bad" -> activity.elonColor(R.color.elon_status_danger)
            "warn" -> activity.elonColor(R.color.elon_status_project)
            else -> activity.elonColor(R.color.elon_status_success)
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

    private fun buildSectionDivider(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(1)
            ).also { it.bottomMargin = dp(16) }
            setBackgroundColor(activity.elonColor(R.color.elon_border_subtle))
            alpha = 0.45f
        }
    }

    private fun setRefreshLoading(loading: Boolean) {
        if (!::refreshButton.isInitialized) return
        refreshButton.isEnabled = !loading
        refreshButton.alpha = if (loading) 0.55f else 1f
        refreshButton.text = if (loading) "刷新中" else "刷新"
    }

    private fun dp(n: Int): Int =
        (n * activity.resources.displayMetrics.density + 0.5f).toInt()
}
