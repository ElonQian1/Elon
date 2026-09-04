package com.elon.app.esk.platform.progress

import android.app.Activity
import android.graphics.drawable.GradientDrawable
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import com.elon.app.R

/** The preview is the production layout; account labels and click authority are never saved. */
internal class EskPlatformProgressConsentView(activity: Activity, cancel: () -> Unit) {
    private val root = activity.layoutInflater.inflate(R.layout.esk_platform_progress_consent_preview, null)
    private val account = root.findViewById<TextView>(R.id.esk_platform_progress_consent_account)
    private val scope = root.findViewById<TextView>(R.id.esk_platform_progress_consent_scope)
    private val status = root.findViewById<TextView>(R.id.esk_platform_progress_consent_status)
    private val primary = root.findViewById<Button>(R.id.esk_platform_progress_consent_primary_action)

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
                val obscured = event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or
                    MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0
                if (obscured) cancel()
                obscured
            }
            button.setTextColor(activity.getColor(if (main) R.color.elon_button_primary_text else R.color.elon_text_primary))
            button.background = (if (main) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT,
                intArrayOf(activity.getColor(R.color.elon_titanium), activity.getColor(R.color.elon_titanium_mid),
                    activity.getColor(R.color.elon_titanium_end)))
                else GradientDrawable().apply { setColor(activity.getColor(R.color.elon_surface_card)) })
                .apply { cornerRadius = 24 * activity.resources.displayMetrics.density }
        }
        style(primary, true)
        root.findViewById<Button>(R.id.esk_platform_progress_consent_cancel).also {
            style(it, false)
            it.setOnClickListener { cancel() }
        }
        clear()
        activity.setContentView(root)
    }

    fun show(label: String, continuation: Boolean, confirm: () -> Unit) {
        account.text = label.filterNot { it.isISOControl() || Character.getType(it) == Character.FORMAT.toInt() }.take(64)
        scope.text = if (continuation) "本次查看已选择的下一页，最多 20 条；不会合并旧页。"
            else "本次查看首页，最多 20 条；下一页仍需重新确认。"
        status.text = "请核对当前主项目账户。切换账户或离开此页会取消授权。"
        primary.isEnabled = true
        primary.alpha = 1f
        primary.setOnClickListener { confirm() }
    }

    fun loading() {
        primary.isEnabled = false
        primary.alpha = 0.4f
        primary.setOnClickListener(null)
        status.text = "正在读取本页正式额度与进度，请保持此页打开…"
    }

    fun unavailable(message: String) { clear(); status.text = message }

    fun clear() {
        account.text = "尚未确认账户"
        scope.text = "没有授权连续读取或任何财务操作。"
        status.text = "没有返回额度与进度。"
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
