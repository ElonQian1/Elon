package com.elon.app.esk.handoff

import android.app.Activity
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.R

/** Native-only consent: no web content, balance cache or restorable account label. */
internal class EskSnapshotConsentView(private val activity: Activity, private val cancel: () -> Unit) {
    private fun dp(value: Int) = (activity.resources.displayMetrics.density * value).toInt()
    private fun color(id: Int) = activity.getColor(id)
    private var primary: Button? = null
    private var status: TextView? = null

    fun show(accountLabel: String?, error: String? = null, confirm: (() -> Unit)? = null) {
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(24), dp(28), dp(24), dp(32))
        }
        content.addView(text("ESK 只读资产授权", 24, true))
        content.addView(text("一龙量化 · 本机原生页面", 13))
        if (accountLabel != null) {
            content.addView(text("当前主项目账户", 13))
            content.addView(text(accountLabel.filterNot { it.isISOControl() }.take(64), 20, true))
            content.addView(text("仅向本机已验证的一龙量化应用提供一次 ESK 余额快照。不会传递登录凭据或账户身份，也不会买卖或转账。", 16))
            content.addView(text("当前是 Paper 平台记录，尚未上链；不代表真实已付款权益、现金兑付或实时余额。快照最多显示 60 秒，离开页面立即失效。", 14))
        }
        status = text(error ?: "请核对账户。切换账户或离开此页会取消授权。", 14)
        content.addView(status)
        primary = if (confirm != null) button("确认并读取快照", true, confirm).also { content.addView(it) } else null
        content.addView(button(if (error == null) "取消" else "返回量化应用重试", false, cancel))
        val root = ScrollView(activity).apply {
            setBackgroundColor(color(R.color.elon_bg_app))
            isFillViewport = true
            addView(content, ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT))
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
            setOnApplyWindowInsetsListener { _, insets ->
                setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                    insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
                insets
            }
        }
        disableState(root)
        activity.setContentView(root)
    }

    fun loading() {
        primary?.isEnabled = false
        status?.text = "正在安全读取一次快照，请保持此页打开…"
    }

    private fun text(value: String, size: Int, strong: Boolean = false) = TextView(activity).apply {
        text = value
        textSize = size.toFloat()
        setTextColor(color(if (strong) R.color.elon_text_primary else R.color.elon_text_secondary))
        if (strong) setTypeface(typeface, Typeface.BOLD)
        setLineSpacing(dp(3).toFloat(), 1f)
        setPadding(0, 0, 0, dp(18))
        setTextIsSelectable(false)
    }

    private fun button(label: String, main: Boolean, click: () -> Unit) = Button(activity).apply {
        text = label
        textSize = 16f
        isAllCaps = false
        minHeight = dp(52)
        filterTouchesWhenObscured = true
        setOnTouchListener { _, event ->
            event.flags and (android.view.MotionEvent.FLAG_WINDOW_IS_OBSCURED or
                android.view.MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0
        }
        setTextColor(color(if (main) R.color.elon_button_primary_text else R.color.elon_text_primary))
        background = if (main) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT, intArrayOf(
            color(R.color.elon_titanium), color(R.color.elon_titanium_mid), color(R.color.elon_titanium_end)))
            .apply { cornerRadius = dp(24).toFloat() }
        else GradientDrawable().apply { setColor(color(R.color.elon_surface_card)); cornerRadius = dp(24).toFloat() }
        layoutParams = LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(54)).apply { topMargin = dp(12) }
        setOnClickListener { click() }
    }

    private fun disableState(view: View) {
        view.isSaveEnabled = false
        view.isSaveFromParentEnabled = false
        if (view is ViewGroup) for (i in 0 until view.childCount) disableState(view.getChildAt(i))
    }
}
