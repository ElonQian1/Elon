package com.elon.app.esk.platform.handoff

import android.app.Activity
import android.graphics.drawable.GradientDrawable
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import com.elon.app.R

/** Uses the production preview layout; no retained account, snapshot or restorable state. */
internal class EskPlatformSnapshotConsentView(activity: Activity, cancel: () -> Unit) {
    private val root = activity.layoutInflater.inflate(R.layout.esk_platform_consent_preview, null)
    private val account = root.findViewById<TextView>(R.id.esk_platform_consent_account)
    private val status = root.findViewById<TextView>(R.id.esk_platform_consent_status)
    private val primary = root.findViewById<Button>(R.id.esk_platform_consent_primary_action)

    init {
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        disableState(root)
        root.setOnApplyWindowInsetsListener { view, insets ->
            view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                insets.systemWindowInsetRight, insets.systemWindowInsetBottom)
            insets
        }
        fun style(button: Button, main: Boolean) {
            button.isAllCaps = false
            button.filterTouchesWhenObscured = true
            button.setOnTouchListener { _, event ->
                event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or
                    MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0
            }
            button.setTextColor(activity.getColor(if (main) R.color.elon_button_primary_text
                else R.color.elon_text_primary))
            button.background = (if (main) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT,
                intArrayOf(activity.getColor(R.color.elon_titanium), activity.getColor(R.color.elon_titanium_mid),
                    activity.getColor(R.color.elon_titanium_end)))
                else GradientDrawable().apply { setColor(activity.getColor(R.color.elon_surface_card)) })
                .apply { cornerRadius = 24 * activity.resources.displayMetrics.density }
        }
        style(primary, true)
        root.findViewById<Button>(R.id.esk_platform_consent_cancel).also {
            style(it, false)
            it.setOnClickListener { cancel() }
        }
        clear()
        activity.setContentView(root)
    }

    fun show(label: String, confirm: () -> Unit) {
        account.text = label.filterNot { it.isISOControl() || Character.getType(it) == Character.FORMAT.toInt() }.take(64)
        status.text = "请核对当前主项目账户。切换账户或离开此页会取消授权。"
        primary.isEnabled = true
        primary.alpha = 1f
        primary.setOnClickListener { confirm() }
    }

    fun loading() {
        primary.isEnabled = false
        primary.alpha = 0.4f
        primary.setOnClickListener(null)
        status.text = "正在读取正式登记，请保持此页打开…"
    }

    fun unavailable(message: String) {
        clear()
        status.text = message
    }

    fun clear() {
        account.text = "尚未确认账户"
        status.text = "没有返回资产摘要。"
        primary.isEnabled = false
        primary.alpha = 0.4f
        primary.setOnClickListener(null)
    }

    private fun disableState(view: View) {
        view.isSaveEnabled = false
        view.isSaveFromParentEnabled = false
        if (view is TextView) view.setTextIsSelectable(false)
        if (view is ViewGroup) for (i in 0 until view.childCount) disableState(view.getChildAt(i))
    }
}
