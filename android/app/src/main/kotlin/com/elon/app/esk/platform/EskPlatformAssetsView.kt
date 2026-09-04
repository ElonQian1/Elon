package com.elon.app.esk.platform

import android.app.Activity
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.R

/** One native source only; this page never receives or adds a Paper balance. */
internal class EskPlatformAssetsView(activity: Activity, onBack: () -> Unit, onRefresh: () -> Unit) {
    private val context = activity
    private val root = LayoutInflater.from(activity).inflate(R.layout.esk_platform_assets_preview, null)
    private val total = root.findViewById<TextView>(R.id.esk_platform_total)
    private val accountLabel = root.findViewById<TextView>(R.id.esk_platform_account_label)
    private val status = root.findViewById<TextView>(R.id.esk_platform_status)
    private val entries = root.findViewById<LinearLayout>(R.id.esk_platform_entries)
    private val refresh = root.findViewById<Button>(R.id.esk_platform_assets_primary_action)

    init {
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        root.setOnApplyWindowInsetsListener { view, insets ->
            view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
            insets
        }
        styleButton(refresh, true, onRefresh)
        styleButton(root.findViewById(R.id.esk_platform_back), false, onBack)
        disableState(root)
        activity.setContentView(root)
        root.requestApplyInsets()
    }

    fun clear() {
        total.text = "— ESK"
        accountLabel.text = "当前账户"
        status.text = "未显示账户数据。"
        entries.removeAllViews()
        refresh.isEnabled = false
    }

    fun loading() {
        clear()
        status.text = "正在读取正式平台登记，请保持此页打开…"
    }

    fun unavailable(message: String) {
        clear()
        status.text = message
        refresh.isEnabled = true
    }

    fun show(account: EskPlatformAccount, displayName: String) {
        clear()
        accountLabel.text = "当前账户 · $displayName"
        total.text = "${account.total} ESK"
        status.text = if (account.entries.isEmpty()) "暂无正式登记。已付款但未显示时，请联系项目方核对入账；此处不代表付款不存在。"
        else "共 ${account.entryCount} 笔正式登记 · 最近更新 ${account.updatedAt}\n" +
            if (account.historyHasMore) "仅显示最近 ${account.entries.size} 笔，数量来自全部已审核账本。" else "已显示全部审核入账。"
        account.entries.forEach { entry ->
            entries.addView(TextView(context).apply {
                text = "+${entry.amount} ESK\n${entry.createdAt} · 审核入账\n记录：${entry.entryId}"
                textSize = 15f
                setTextColor(context.getColor(R.color.elon_text_primary))
                setLineSpacing(dp(4).toFloat(), 1f)
                setPadding(0, dp(16), 0, dp(16))
                isSaveEnabled = false
                isSaveFromParentEnabled = false
                setTextIsSelectable(false)
            })
        }
        refresh.isEnabled = true
    }

    private fun dp(value: Int) = (context.resources.displayMetrics.density * value).toInt()

    private fun styleButton(button: Button, primary: Boolean, click: () -> Unit) = button.apply {
        isAllCaps = false
        filterTouchesWhenObscured = true
        setOnTouchListener { _, event -> event.flags and
            (android.view.MotionEvent.FLAG_WINDOW_IS_OBSCURED or
                android.view.MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0 }
        setTextColor(context.getColor(if (primary) R.color.elon_button_primary_text else R.color.elon_text_primary))
        backgroundTintList = null
        background = if (primary) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT, intArrayOf(
            context.getColor(R.color.elon_titanium), context.getColor(R.color.elon_titanium_mid),
            context.getColor(R.color.elon_titanium_end))).apply { cornerRadius = dp(24).toFloat() }
        else GradientDrawable().apply {
            setColor(context.getColor(R.color.elon_surface_card)); cornerRadius = dp(24).toFloat()
        }
        setOnClickListener { click() }
    }

    private fun disableState(view: View) {
        view.isSaveEnabled = false
        view.isSaveFromParentEnabled = false
        if (view is ViewGroup) for (index in 0 until view.childCount) disableState(view.getChildAt(index))
    }
}
