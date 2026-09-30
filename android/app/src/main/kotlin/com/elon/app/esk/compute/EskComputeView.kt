package com.elon.app.esk.compute

import android.app.Activity
import android.content.res.ColorStateList
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.R
import java.text.DateFormat
import java.util.Date

/** Uses the production View runtime and V2 semantic resources. */
internal class EskComputeView(
    private val activity: Activity, onBack: () -> Unit, private val refresh: () -> Unit,
    private val history: () -> Unit, private val assets: () -> Unit, private val changePage: (Int) -> Unit,
) {
    private val root = ScrollView(activity)
    private val column = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(16), dp(16), dp(16), dp(24)) }
    private val body = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
    private val account = text("当前账户", 14)
    private val status = text("未显示账户数据", 14)
    private val quote = text("USDT / 人民币参考估值暂不可用", 14)
    private var snapshot: EskComputeSnapshot? = null
    private var section = 0
    private val tabs = mutableListOf<Button>()
    init {
        root.setBackgroundColor(activity.getColor(R.color.elon_bg_app))
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        root.addView(column)
        root.setOnApplyWindowInsetsListener { view, insets ->
            view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop, insets.systemWindowInsetRight, insets.systemWindowInsetBottom); insets }
        column.addView(text("ESK 与算力", 24)); column.addView(account)
        val actions = LinearLayout(activity)
        actions.addView(button("返回", onBack)); actions.addView(button("刷新账户", refresh)); column.addView(actions)
        val nav = LinearLayout(activity)
        listOf("账户", "购入登记", "AI 用量", "账单").forEachIndexed { index, label ->
            nav.addView(button(label) { section = index; render() }.also { tabs.add(it) })
        }
        column.addView(HorizontalScrollView(activity).apply { isHorizontalScrollBarEnabled = false; addView(nav) })
        column.addView(status); column.addView(body)
        activity.setContentView(root); disableSave(root); root.requestApplyInsets()
    }
    fun clear() { snapshot = null; account.text = "当前账户"; quote.text = "参考估值暂不可用"; body.removeAllViews(); status.text = "未显示账户数据。" }
    fun loading() { clear(); status.text = "正在读取当前账户，请保持页面在前台…" }
    fun unavailable(message: String) { clear(); status.text = message }
    fun expireQuote() { quote.text = "USDT / 人民币报价已到期，请刷新获取最新参考估值。" }
    fun show(value: EskComputeSnapshot, name: String) { snapshot = value; account.text = name; render() }
    private fun render() {
        body.removeAllViews()
        tabs.forEachIndexed { i, b -> b.isSelected = i == section; b.contentDescription = b.text.toString() + if (i == section) "，已选中" else "" }
        val s = snapshot ?: return
        status.text = "正式平台登记 · 尚未上链 · ${time(s.observedAt)}读取 · 60 秒有效"
        when (section) {
            0 -> {
                body.addView(text("正式 ESK 总登记", 14)); body.addView(text("${amount(s.total)} ESK", 28))
                val q = s.quote
                quote.text = if (q != null && q.validUntil > System.currentTimeMillis() && q.usdt != null && q.cny != null)
                    "参考估值 ${amount(q.usdt)} USDT · ¥${amount(q.cny)} CNY\n来源：${q.source} · ${time(q.observedAt)}"
                    else "USDT / 人民币参考估值暂不可用"
                body.addView(quote)
                metric("卖回申请占用", "${amount(s.reserved)} ESK"); metric("剩余正式登记", "${amount(s.remaining)} ESK")
                metric("本月 AI 实际消费", "¥${amount(s.monthCost, 2)} CNY")
                metric("原人民币可用余额", s.balance?.let { "¥${amount(it, 2)} CNY" } ?: "未开通")
                body.addView(text("平台后续主要使用 ESK 支付 AI 服务，人民币和 USDT 用于参考折算。ESK 服务支付尚未接入；当前 AI 仍以人民币结算，剩余正式登记尚不能用于 AI 扣费。", 14))
                body.addView(button("购买 ESK · 收款渠道待配置", {}).apply { isEnabled = false })
                body.addView(button("查看正式资产与占用", assets))
                rows("当前 AI 预占 · CNY", s.holds)
                if (s.holdsHaveMore) body.addView(text("当前显示最近 20 项，另有预占未展示。", 14))
            }
            1 -> {
                body.addView(text("正式登记 ${s.entryCount} 笔 · 管理员审核到账", 16)); rows("购入登记", s.purchases)
                if (s.purchasesHaveMore) body.addView(text("当前显示最近 20 笔，可查看完整审核流水。", 14))
                body.addView(button("查看完整审核流水", history))
            }
            2 -> { body.addView(text("按 UTC 自然月统计。Token 是用量单位，具体价格由模型与服务决定。", 14)); rows("本月 AI 用量", s.usage) }
            3 -> {
                body.addView(text("历史账单保留真实币种和当时价格版本，不按当前报价改写为 ESK。", 14)); rows("AI 实际账单 · CNY", s.bills)
                body.addView(text("第 ${s.page} 页", 14))
                body.addView(button("上一页") { changePage(s.page - 1) }.apply { isEnabled = s.page > 1 })
                body.addView(button("下一页") { changePage(s.page + 1) }.apply { isEnabled = s.billsHasMore && s.page < 1000 })
            }
        }
        disableSave(body)
    }
    private fun rows(title: String, values: List<CenterRow>) {
        body.addView(text(title, 18))
        if (values.isEmpty()) body.addView(text("暂无记录。", 14))
        values.forEach { row ->
            val details = text(row.details.joinToString("\n") { "${it.first}：${it.second}" }, 14).apply { visibility = View.GONE }
            body.addView(button("${row.title}\n${row.amount} · 查看明细") { details.visibility = if (details.visibility == View.GONE) View.VISIBLE else View.GONE })
            body.addView(details)
        }
    }
    private fun metric(label: String, value: String) { body.addView(text("$label\n$value", 16)) }
    private fun text(value: String, sp: Int) = TextView(activity).apply {
        text = value; textSize = sp.toFloat(); setTextColor(activity.getColor(R.color.elon_text_primary))
        setPadding(0, dp(8), 0, dp(8)); setLineSpacing(dp(2).toFloat(), 1f); isSaveEnabled = false
    }
    private fun button(label: String, click: () -> Unit) = Button(activity).apply {
        text = label; isAllCaps = false; textSize = 14f; minimumHeight = dp(48); minHeight = dp(48)
        setPadding(dp(12), dp(8), dp(12), dp(8)); setTextColor(activity.getColor(R.color.elon_text_primary))
        backgroundTintList = ColorStateList.valueOf(activity.getColor(R.color.elon_surface_card))
        setOnClickListener { click() }; isSaveEnabled = false
    }
    private fun disableSave(view: View) { view.isSaveEnabled = false; view.isSaveFromParentEnabled = false
        if (view is ViewGroup) for (i in 0 until view.childCount) disableSave(view.getChildAt(i)) }
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density + .5f).toInt()
    private fun time(at: Long) = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(at))
}
