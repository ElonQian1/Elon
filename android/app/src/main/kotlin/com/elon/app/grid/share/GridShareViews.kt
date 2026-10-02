package com.elon.app.grid.share

import android.content.Context
import android.graphics.Typeface
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import com.elon.app.ChatMessage
import com.elon.app.R
import org.json.JSONObject
import java.text.DateFormat
import java.util.Date

/** View interop preserves the existing chat RecyclerView and quote handling. */
internal class GridShareViews(private val context: Context) {
    private val s = GridShareStyle(context)
    fun dp(value: Int) = s.dp(value)
    fun column(padding: Int = 16) = s.column(padding)
    fun text(value: String, size: Float = 14f, quiet: Boolean = false) = s.text(value, size, quiet)
    fun button(label: String, action: () -> Unit) = s.button(label, action)
    fun dialogTitle(title: String) = s.dialogTitle(title)
    fun styleDialog(dialog: AlertDialog) = s.styleDialog(dialog)
    private fun raw(grid: JSONObject, key: String) = GridSharePresentation.raw(grid, key)
    private fun shown(grid: JSONObject, key: String) = raw(grid, key)?.let {
        if (GridSharePresentation.number(it) != null) GridSharePresentation.format(it) else GridShareModel.value(grid, key)
    } ?: GridShareModel.value(grid, key)
    private fun pair(label: String, value: String, tint: Int = s.ink) = s.row().apply {
        addView(text(label, 12f, quiet = true), LinearLayout.LayoutParams(0, -2, 1f))
        addView(text(value, 14f).apply { setTextColor(tint); gravity = Gravity.END; typeface = Typeface.DEFAULT_BOLD }, LinearLayout.LayoutParams(0, -2, 1.25f))
    }
    fun summary(grid: JSONObject): LinearLayout = s.column().apply {
        val tint = s.direction(raw(grid, "direction"))
        background = s.panel(border = s.color(R.color.mobile_outline_variant))
        addView(s.row().apply {
            addView(text("◆ 币安 · 网格快照", 12f).apply { setTextColor(s.color(R.color.mobile_warning)) }, LinearLayout.LayoutParams(0, -2, 1f))
            addView(s.pill("历史快照", s.muted))
        })
        addView(s.spaced(s.row().apply {
            val symbol = GridShareModel.value(grid, "symbol")
            addView(s.logo(symbol))
            addView(s.column(0).apply {
                setPadding(dp(10), 0, 0, 0)
                addView(text(symbol, 21f).apply { typeface = Typeface.DEFAULT_BOLD })
                addView(text("${shown(grid, "count")} 格 · ${shown(grid, "spacing")}", 12f, quiet = true))
            }, LinearLayout.LayoutParams(0, -2, 1f))
        }))
        addView(s.spaced(s.row().apply {
            addView(s.pill("${shown(grid, "direction")} · ${shown(grid, "leverage")}×", tint))
            if (raw(grid, "status") != null) addView(s.pill(shown(grid, "status"), s.primary), LinearLayout.LayoutParams(-2, -2).apply { marginStart = dp(8) })
        }))
        val metric = GridSharePresentation.metric(grid)
        addView(s.spaced(s.column(12).apply {
            val metricTint = if (metric != null) s.tone(raw(grid, metric)) else s.muted
            background = s.panel(androidx.core.graphics.ColorUtils.blendARGB(s.surface, metricTint, .08f), 12)
            addView(text(when (metric) { "roi" -> "策略总收益率"; "totalPnl" -> "策略总盈亏 · USDT"; "profit" -> "网格利润 · USDT"; else -> "收益数据" }, 12f, true))
            val amount = if (metric != null) GridSharePresentation.format(raw(grid, metric), true) + (if (metric == "roi") "%" else "")
                else if (!grid.optBoolean("show_amounts")) "历史分享未公开金额" else "暂未读取"
            addView(text(amount, if (metric != null) 26f else 16f).apply { setTextColor(metricTint); typeface = Typeface.DEFAULT_BOLD })
            if (metric == "profit") addView(text("网格利润不代表策略总盈亏", 11f, true))
            if (grid.optBoolean("show_amounts")) addView(pair("未实现盈亏 · USDT", GridSharePresentation.format(raw(grid, "unrealizedPnl"), true), s.tone(raw(grid, "unrealizedPnl"))))
        }))
        if (grid.optBoolean("show_amounts")) {
            addView(s.spaced(pair("投入保证金 · USDT", shown(grid, "investment"))))
            addView(pair("持仓数量 · ${GridSharePresentation.token(GridShareModel.value(grid, "symbol"))}", shown(grid, "positionQty")))
            addView(pair("持仓货值 · USDT", shown(grid, "positionNotional")))
        }
        addView(s.spaced(s.divider()))
        addView(pair("价格区间", GridSharePresentation.rangeLabel(grid), s.primary))
        addView(s.row().apply {
            addView(text(shown(grid, "lower"), 13f), LinearLayout.LayoutParams(0, -2, 1f))
            addView(text(shown(grid, "upper"), 13f).apply { gravity = Gravity.END }, LinearLayout.LayoutParams(0, -2, 1f))
        })
        addView(s.range(GridSharePresentation.range(grid)))
        addView(text("标记价 ${shown(grid, "markPrice")}", 12f, quiet = true))
        addView(s.spaced(s.divider()))
        addView(text(DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(grid.optLong("observed_at_ms"))) + " 采集", 11f, quiet = true))
        addView(text(if (grid.optBoolean("show_amounts")) "金额与数量全部公开" else "旧快照 · 金额与数量未公开", 11f).apply { setTextColor(s.primary) })
    }
    fun details(grid: JSONObject): View = s.column(0).apply {
        val rows = s.column(0)
        val tabs = s.row().apply { background = s.panel(s.color(R.color.mobile_surface_container_high), 12); setPadding(dp(4), dp(4), dp(4), dp(4)) }
        val buttons = linkedMapOf<String, TextView>()
        fun show(tab: String) {
            rows.removeAllViews()
            buttons.forEach { (key, button) ->
                button.isSelected = key == tab; button.setTextColor(if (key == tab) s.primary else s.muted)
                button.background = s.panel(if (key == tab) s.color(R.color.mobile_primary_container) else s.color(R.color.mobile_surface_container_high), 8)
                button.typeface = if (key == tab) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
            }
            GridShareModel.sections.getValue(tab).forEach { key ->
                val tint = if (key in setOf("profit", "totalPnl", "roi", "matchedPnl", "unrealizedPnl", "fundingFee")) s.tone(raw(grid, key)) else s.ink
                rows.addView(s.row().apply {
                    minimumHeight = dp(52); setPadding(dp(4), dp(8), dp(4), dp(8))
                    addView(text(GridShareModel.labels.getValue(key), 13f, true), LinearLayout.LayoutParams(0, -2, 1f))
                    addView(text(GridShareModel.value(grid, key), 14f).apply { setTextColor(tint); gravity = Gravity.END; setTextIsSelectable(true) }, LinearLayout.LayoutParams(0, -2, 1.1f))
                })
                rows.addView(s.divider())
            }
        }
        GridShareModel.sections.keys.forEach { key ->
            val tab = text(key, 14f).apply { gravity = Gravity.CENTER; minHeight = dp(48); isFocusable = true; contentDescription = "网格详情：$key"; setOnClickListener { show(key) } }
            buttons[key] = tab; tabs.addView(tab, LinearLayout.LayoutParams(0, -2, 1f))
        }
        addView(s.spaced(tabs, 16)); addView(rows); show("持仓")
    }
    companion object {
        fun bind(container: LinearLayout?, text: TextView, message: ChatMessage): Boolean {
            val card = GridShareModel.card(message.content) ?: return false
            container ?: return false
            container.removeAllViews(); container.visibility = View.VISIBLE; text.text = ""; text.visibility = View.GONE
            val ui = GridShareViews(container.context)
            container.addView(GridShareMessageCard(container.context).apply {
                layoutParams = LinearLayout.LayoutParams(-2, -2)
                addView(ui.summary(card.getJSONObject("grid")).apply {
                    addView(ui.text("查看网格详情 ›", 14f).apply { setTextColor(context.getColor(R.color.mobile_primary)); minHeight = ui.dp(48); gravity = Gravity.CENTER_VERTICAL; typeface = Typeface.DEFAULT_BOLD })
                }, LinearLayout.LayoutParams(-1, -2))
                minimumHeight = ui.dp(48); isFocusable = true
                setOnClickListener { GridShareFeature.current?.openCard(card, message) }
            })
            return true
        }
    }
}
