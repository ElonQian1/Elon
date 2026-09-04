package com.elon.app.esk.platform

import android.app.Activity
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.R

/** Renders a single validated page. No storage, selection, sharing, or synthetic account values. */
internal class EskPlatformHistoryView(
    private val activity: Activity, onBack: () -> Unit, onRefresh: () -> Unit, onNext: () -> Unit,
) {
    private val root = LayoutInflater.from(activity).inflate(R.layout.esk_platform_history_preview, null) as ScrollView
    private val total = root.findViewById<TextView>(R.id.esk_history_total)
    private val label = root.findViewById<TextView>(R.id.esk_history_account)
    private val status = root.findViewById<TextView>(R.id.esk_history_status)
    private val entries = root.findViewById<LinearLayout>(R.id.esk_history_entries)
    private val refresh = root.findViewById<Button>(R.id.esk_platform_history_primary_action)
    private val next = root.findViewById<Button>(R.id.esk_history_next)

    init {
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        root.setOnApplyWindowInsetsListener { view, insets ->
            view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
            insets
        }
        style(refresh, true, onRefresh)
        style(next, true, onNext)
        style(root.findViewById(R.id.esk_history_back), false, onBack)
        disableState(root)
        activity.setContentView(root)
        root.requestApplyInsets()
    }

    fun clear() {
        total.text = "— ESK"
        label.text = "当前账户"
        status.text = "未显示账户流水。"
        entries.removeAllViews()
        refresh.isEnabled = false
        next.isEnabled = false
        next.visibility = View.GONE
    }

    fun loading() { clear(); status.text = "正在读取本人审核流水，请保持此页打开…" }
    fun unavailable(message: String) { clear(); status.text = message; refresh.isEnabled = true }

    fun show(page: EskPlatformHistoryPage, displayName: String) {
        clear()
        total.text = "${page.total} ESK"
        label.text = "当前账户 · $displayName"
        status.text = if (page.entries.isEmpty())
            "暂无正式登记。已付款但未显示时，请联系项目方核对；此处不代表付款不存在。"
        else "第 ${page.rangeStart}–${page.rangeEnd} 笔 / 共 ${page.entryCount} 笔\n" +
            "全账户最近更新：${page.updatedAt}" + if (page.hasMore) "" else "\n已到最后一页。"
        page.entries.forEach { entry ->
            entries.addView(TextView(activity).apply {
                text = "+${entry.amount} ESK\n${entry.createdAt} · 审核入账\n记录：${entry.entryId}"
                textSize = 15f
                setTextColor(activity.getColor(R.color.elon_text_primary))
                setLineSpacing(dp(4).toFloat(), 1f)
                setPadding(0, dp(16), 0, dp(16))
                isSaveEnabled = false; isSaveFromParentEnabled = false
                setTextIsSelectable(false)
            })
        }
        refresh.isEnabled = true
        next.isEnabled = page.hasMore
        next.visibility = if (page.hasMore) View.VISIBLE else View.GONE
        root.scrollTo(0, 0)
    }

    private fun dp(value: Int) = (activity.resources.displayMetrics.density * value).toInt()
    private fun style(button: Button, primary: Boolean, click: () -> Unit) = button.apply {
        isAllCaps = false
        filterTouchesWhenObscured = true
        setOnTouchListener { _, event -> event.flags and
            (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0 }
        setTextColor(activity.getColor(if (primary) R.color.elon_button_primary_text else R.color.elon_text_primary))
        backgroundTintList = null
        background = if (primary) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT, intArrayOf(
            activity.getColor(R.color.elon_titanium), activity.getColor(R.color.elon_titanium_mid),
            activity.getColor(R.color.elon_titanium_end))).apply { cornerRadius = dp(24).toFloat() }
        else GradientDrawable().apply {
            setColor(activity.getColor(R.color.elon_surface_card)); cornerRadius = dp(24).toFloat()
        }
        setOnClickListener { click() }
    }

    private fun disableState(view: View) {
        view.isSaveEnabled = false; view.isSaveFromParentEnabled = false
        if (view is ViewGroup) for (index in 0 until view.childCount) disableState(view.getChildAt(index))
    }
}
