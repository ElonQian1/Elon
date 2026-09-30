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
        column.addView(text("我的 AI 账户", 24)); column.addView(account)
        column.addView(text("ESK 是平台 AI 服务的支付单位。你可以在这里查看余额、到账记录和每笔 AI 使用费用。", 14))
        val actions = LinearLayout(activity)
        actions.addView(button("返回", onBack)); actions.addView(button("刷新账户", refresh)); column.addView(actions)
        val nav = LinearLayout(activity)
        listOf("余额", "到账记录", "AI 用量", "消费账单").forEachIndexed { index, label ->
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
        status.text = "更新于 ${time(s.observedAt)}"
        when (section) {
            0 -> {
                body.addView(text("我的 ESK 余额", 14)); body.addView(text("${amount(s.total)} ESK", 28))
                val q = s.quote
                quote.text = if (q != null && q.validUntil > System.currentTimeMillis() && q.usdt != null && q.cny != null)
                    "参考估值 ${amount(q.usdt)} USDT · ¥${amount(q.cny)} CNY\n来源：${q.source} · ${time(q.observedAt)}"
                    else "USDT / 人民币参考估值暂不可用"
                body.addView(quote)
                metric("申请处理中 · 暂时冻结", "${amount(s.reserved)} ESK"); metric("未冻结余额", "${amount(s.remaining)} ESK")
                metric("本月 AI 消费", "¥${amount(s.monthCost, 2)} CNY")
                metric("人民币账户余额", s.balance?.let { "¥${amount(it, 2)} CNY" } ?: "未开通")
                body.addView(text("ESK 是平台 AI 服务的支付单位。充值和 ESK 支付暂未开放，当前 AI 费用仍从人民币账户结算。这里的 ESK 是平台账户记录，尚未上链。", 14))
                body.addView(button("充值 ESK · 暂未开放", {}).apply { isEnabled = false })
                body.addView(button("查看 ESK 余额明细", assets))
                rows("AI 任务处理中 · 暂时冻结的人民币", s.holds)
                if (s.holdsHaveMore) body.addView(text("这里显示最近 20 项，更多处理中任务未展示。", 14))
            }
            1 -> {
                body.addView(text("已审核记录 ${s.entryCount} 笔", 16)); rows("ESK 到账与余额变动", s.purchases)
                if (s.purchasesHaveMore) body.addView(text("这里显示最近 20 笔，更多记录请查看全部记录。", 14))
                body.addView(button("查看全部到账与审核记录", history))
            }
            2 -> { body.addView(text("Token 是 AI 处理文字的计量单位。发送内容和 AI 回复分别计量，不同模型的价格不同。以下按世界标准时间（UTC）统计本月用量。", 14)); rows("本月 AI 用量", s.usage) }
            3 -> {
                body.addView(text("这里展示每笔 AI 使用的实际费用。历史人民币账单仍按人民币显示。", 14)); rows("AI 消费记录 · 人民币", s.bills)
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
    private fun metric(label: String, value: String) { body.addView(text("$label\n$value", 16).apply { setPadding(dp(16), dp(12), dp(16), dp(12)); setBackgroundResource(R.color.elon_surface_card); layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(8) } }) }
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
