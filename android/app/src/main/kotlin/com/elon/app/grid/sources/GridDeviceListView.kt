package com.elon.app.grid.sources

import android.app.Activity
import android.view.View
import android.view.ViewGroup
import android.widget.*
import com.elon.app.grid.ui.BinanceGridAppearance
import java.text.DateFormat
import java.util.Date

/** Recycled rows, no WebView, chart, polling, credentials or trading callbacks. */
internal class GridDeviceListView(activity: Activity, changed: () -> Unit) {
    private val ui = BinanceGridAppearance(activity)
    private val preferences = activity.getSharedPreferences("grid_source_selection_v1", android.content.Context.MODE_PRIVATE)
    val root = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background)
        setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8))
    }
    val controls = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
    val status = ui.label("", 14f).apply { accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
    val apk = check(activity, "APK 端", preferences.getBoolean("android", true))
    val win = check(activity, "Win 端", preferences.getBoolean("windows", true))
    private var entries = emptyList<String>()
    private val adapter = object : BaseAdapter() {
        override fun getCount() = entries.size
        override fun getItem(position: Int) = entries[position]
        override fun getItemId(position: Int) = position.toLong()
        override fun isEnabled(position: Int) = false
        override fun getView(position: Int, recycled: View?, parent: ViewGroup): View =
            ((recycled as? TextView) ?: ui.label("", 15f)).apply {
                text = entries[position]; setPadding(ui.dp(12), ui.dp(12), ui.dp(12), ui.dp(12))
                setBackgroundColor(ui.surface); minimumHeight = ui.dp(64); isSaveEnabled = false
            }
    }
    init {
        val header = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        header.addView(ui.label("多端网格", 22f))
        header.addView(ui.label("可同时选择两个来源；仅查看，不在远端执行交易。", 14f))
        val selectors = LinearLayout(activity)
        selectors.addView(apk, LinearLayout.LayoutParams(0, -2, 1f))
        selectors.addView(win, LinearLayout.LayoutParams(0, -2, 1f))
        header.addView(selectors); header.addView(controls); header.addView(status)
        root.addView(ListView(activity).apply {
            addHeaderView(header, null, false)
            adapter = this@GridDeviceListView.adapter; isSaveEnabled = false
            dividerHeight = ui.dp(8)
        }, LinearLayout.LayoutParams(-1, 0, 1f))
        apk.setOnCheckedChangeListener { _, checked -> preferences.edit().putBoolean("android", checked).apply(); changed() }
        win.setOnCheckedChangeListener { _, checked -> preferences.edit().putBoolean("windows", checked).apply(); changed() }
    }
    fun button(label: String, action: () -> Unit) = ui.button(label, label, action = action).also { controls.addView(it) }
    private fun check(activity: Activity, label: String, checked: Boolean) = CheckBox(activity).apply {
        text = label; isChecked = checked; setTextColor(ui.text); minHeight = ui.dp(48)
        isSaveEnabled = false; filterTouchesWhenObscured = true
    }
    fun show(sources: List<GridDeviceSource>, now: Long) {
        val selected = sources.filter { if (it.platform == "android") apk.isChecked else win.isChecked }
        entries = buildList {
            if (!apk.isChecked && !win.isChecked) add("请选择 APK 端、Win 端，或同时选择两个。")
            for (platform in listOf("android", "windows")) {
                if (!(if (platform == "android") apk.isChecked else win.isChecked)) continue
                val matches = selected.filter { it.platform == platform }
                val label = if (platform == "android") "APK 端" else "Win 端"
                if (matches.isEmpty()) add(label + " · 尚无同步数据\n请在来源端登录同一一龙账号，并开启本人设备同步。")
                for (source in matches) {
                    val fresh = source.fresh(now)
                    val time = DateFormat.getTimeInstance(DateFormat.SHORT).format(Date(source.observed))
                    add(source.label + " · 设备 " + source.device.take(8) + " · 账号 " + (source.account?.take(8) ?: "未确认") +
                        "\n" + (if (fresh) "已更新 " else "未连接或已过期 ") + time + " · " + source.accountKind)
                    if (!fresh) continue
                    if (source.rows.isEmpty()) add("此来源当前没有运行中的网格。")
                    source.rows.forEach { row ->
                        add(listOf(row["symbol"], row["direction"] ?: "方向未读取", row["status"]).joinToString(" · ") +
                            "\n区间 " + (row["lower"] ?: "—") + "–" + (row["upper"] ?: "—") +
                            " · " + (row["count"] ?: "—") + " 格 · " + (row["leverage"] ?: "—") + "×" +
                            "\n收益 " + (row["profit"] ?: "未读取") + " USDT · #" + row["id"])
                    }
                }
            }
            if (selected.size > 1) add("按设备分别展示。同一账号的同一策略可能出现在两端；本页不重复合计收益。")
        }
        adapter.notifyDataSetChanged()
    }
}
