package com.elon.app.grid.share

import android.content.Context
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.ChatMessage
import com.elon.app.R
import org.json.JSONObject
import java.text.DateFormat
import java.util.Date

/** View interop keeps this card inside the existing chat RecyclerView and its quote handling. */
internal class GridShareViews(private val context: Context) {
    fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
    fun column() = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(18), dp(12), dp(18), dp(12)) }
    fun text(value: String, size: Float = 14f, quiet: Boolean = false) = TextView(context).apply {
        text = value; textSize = size; setTextColor(context.getColor(if (quiet) R.color.mobile_on_surface_variant else R.color.mobile_on_surface))
        setPadding(0, dp(5), 0, dp(5)); setLineSpacing(dp(3).toFloat(), 1.05f)
    }
    fun button(label: String, action: () -> Unit) = Button(context).apply {
        text = label; isAllCaps = false; minHeight = dp(48); setOnClickListener { action() }
    }
    fun summary(grid: JSONObject): LinearLayout = column().apply {
        background = GradientDrawable().apply { cornerRadius = dp(16).toFloat(); setColor(context.getColor(R.color.mobile_surface_container)) }
        addView(text("币安 · 网格持仓分享", quiet = true))
        addView(text(GridShareModel.value(grid, "symbol"), 20f).apply { typeface = Typeface.DEFAULT_BOLD })
        addView(text("${GridShareModel.value(grid, "direction")} · ${GridShareModel.value(grid, "leverage")}× · ${GridShareModel.value(grid, "count")} 格 · ${GridShareModel.value(grid, "spacing")}"))
        val roi = grid.optJSONObject("fields")?.optString("roi").orEmpty()
        addView(text(if (roi.isBlank()) "—" else "$roi%", 30f))
        addView(text("策略总收益率${if (roi.isBlank()) " · 未读取" else ""}", quiet = true))
        addView(text("${GridShareModel.value(grid, "lower")} — ${GridShareModel.value(grid, "upper")}"))
        addView(text("价格区间 · 标记价 ${GridShareModel.value(grid, "markPrice")}", quiet = true))
        addView(text("${DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(grid.optLong("observed_at_ms")))} 的快照", quiet = true))
        addView(text(if (grid.optBoolean("show_amounts")) "已公开金额与数量" else "金额与数量未公开", quiet = true))
    }
    fun details(grid: JSONObject): View = column().apply {
        val rows = column()
        fun show(tab: String) {
            rows.removeAllViews()
            GridShareModel.sections.getValue(tab).forEach { key ->
                rows.addView(text("${GridShareModel.labels[key]}\n${GridShareModel.value(grid, key)}"))
            }
        }
        addView(LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            GridShareModel.sections.keys.forEach { key -> addView(button(key) { show(key) }, LinearLayout.LayoutParams(0, -2, 1f)) }
        })
        addView(rows); show("参数")
    }
    companion object {
        fun bind(container: LinearLayout?, text: TextView, message: ChatMessage): Boolean {
            val card = GridShareModel.card(message.content) ?: return false
            container ?: return false
            container.removeAllViews(); container.visibility = View.VISIBLE; text.text = ""; text.visibility = View.GONE
            val ui = GridShareViews(container.context)
            container.addView(ui.summary(card.getJSONObject("grid")).apply {
                layoutParams = LinearLayout.LayoutParams(minOf(ui.dp(310), resources.displayMetrics.widthPixels - ui.dp(88)), -2)
                addView(ui.text("查看网格详情 ›")); minimumHeight = ui.dp(48)
                isFocusable = true; contentDescription = "查看 ${card.optString("title")}"
                setOnClickListener { GridShareFeature.current?.openCard(card, message) }
            })
            return true
        }
    }
}
