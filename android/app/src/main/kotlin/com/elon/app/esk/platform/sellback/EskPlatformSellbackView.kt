package com.elon.app.esk.platform.sellback

import android.app.Activity
import android.app.AlertDialog
import android.graphics.drawable.GradientDrawable
import android.view.LayoutInflater
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.R

internal class EskPlatformSellbackView(
    private val activity: Activity, onBack: () -> Unit, onRefresh: () -> Unit, onNext: () -> Unit,
    onSubmit: (String) -> Unit, onRetry: () -> Unit, onReviewed: () -> Unit,
    private val onCancel: (SellbackRecord) -> Unit,
) {
    private val root = LayoutInflater.from(activity).inflate(R.layout.esk_platform_sellback_preview, null) as ScrollView
    private val label = root.findViewById<TextView>(R.id.esk_sellback_account)
    private val total = root.findViewById<TextView>(R.id.esk_sellback_total)
    private val reserved = root.findViewById<TextView>(R.id.esk_sellback_reserved)
    private val available = root.findViewById<TextView>(R.id.esk_sellback_available)
    private val status = root.findViewById<TextView>(R.id.esk_sellback_status)
    private val terms = root.findViewById<TextView>(R.id.esk_sellback_terms)
    private val amount = root.findViewById<EditText>(R.id.esk_sellback_amount)
    private val entries = root.findViewById<LinearLayout>(R.id.esk_sellback_requests)
    private val refresh = root.findViewById<Button>(R.id.esk_platform_sellback_primary_action)
    private val submit = root.findViewById<Button>(R.id.esk_sellback_submit)
    private val retry = root.findViewById<Button>(R.id.esk_sellback_retry)
    private val reviewed = root.findViewById<Button>(R.id.esk_sellback_reviewed)
    private val next = root.findViewById<Button>(R.id.esk_sellback_next)
    private var dialog: AlertDialog? = null

    init {
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        root.setOnApplyWindowInsetsListener { view, insets ->
            view.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop,
                insets.systemWindowInsetRight, insets.systemWindowInsetBottom); insets
        }
        style(refresh, true, onRefresh); style(next, false, onNext)
        style(submit, true) { onSubmit(amount.text.toString()) }
        style(retry, false, onRetry); style(reviewed, false, onReviewed)
        style(root.findViewById(R.id.esk_sellback_back), false, onBack)
        disableState(root)
        activity.setContentView(root)
        root.requestApplyInsets()
        clear()
    }

    fun clear() {
        dialog?.dismiss(); dialog = null
        label.text = "当前账户"; total.text = "正式总量  — ESK"
        reserved.text = "申请占用  — ESK"; available.text = "可申请量  — ESK"
        status.text = "未显示本人申请。"; terms.text = ""
        amount.text.clear(); amount.isEnabled = false; entries.removeAllViews()
        for (button in listOf(refresh, submit, retry, reviewed, next)) button.isEnabled = false
        retry.visibility = View.GONE; reviewed.visibility = View.GONE; next.visibility = View.GONE
    }
    fun loading() { clear(); status.text = "正在核对本人申请，请保持此页打开…" }
    fun unavailable(message: String, canRetry: Boolean = false) {
        clear(); status.text = message; refresh.isEnabled = true
        retry.isEnabled = canRetry; retry.visibility = if (canRetry) View.VISIBLE else View.GONE
    }
    fun message(value: String) { status.text = value }

    fun page(page: SellbackPage, name: String, blocked: Boolean, canRetry: Boolean) = render(page.summary,
        page.requests, name, if (page.requests.isEmpty()) "暂无正式卖回申请。" else
            "第 ${page.start}–${page.end} 笔 / 共 ${page.summary.count} 笔", page.nextCursor != null, blocked, canRetry)
    fun receipt(result: SellbackResult, name: String, blocked: Boolean, canRetry: Boolean) = render(result.summary,
        listOf(result.request), name, "本次记录已查回；重新读取可查看全部本人申请。", false, blocked, canRetry)

    private fun render(summary: SellbackSummary, records: List<SellbackRecord>, name: String,
        position: String, more: Boolean, blocked: Boolean, canRetry: Boolean) {
        clear()
        label.text = "当前账户 · $name"
        total.text = "正式总量  ${sellbackAmount(summary.total)} ESK"
        reserved.text = "申请占用  ${sellbackAmount(summary.reserved)} ESK"
        available.text = "可申请量  ${sellbackAmount(summary.available)} ESK"
        status.text = position + if (blocked) "\n上次操作结果仍未确认。请核对记录，不要重复新建。" else ""
        val policy = summary.policy
        terms.text = if (summary.enabled && policy != null) "申请条款 · ${policy.revision}\n${policy.terms}\n" +
            "单笔 ${sellbackAmount(policy.minimum)}–${sellbackAmount(policy.maximum)} ESK\n" +
            "账户异常处理：${policy.recovery}" else "新申请暂未开放或当前账户不适用。仍可核对、取消已有本人申请。"
        amount.isEnabled = summary.enabled && summary.available > 0 && !blocked
        submit.isEnabled = amount.isEnabled
        refresh.isEnabled = true
        retry.isEnabled = canRetry; retry.visibility = if (canRetry) View.VISIBLE else View.GONE
        reviewed.isEnabled = blocked; reviewed.visibility = if (blocked) View.VISIBLE else View.GONE
        next.isEnabled = more; next.visibility = if (more) View.VISIBLE else View.GONE
        for (record in records) {
            entries.addView(text("${sellbackAmount(record.amount)} ESK · " +
                (if (record.status == "submitted") "已提交，占用中" else "已取消，占用已解除") +
                "\n${record.created}\n记录：${record.id}").apply { setPadding(0, dp(16), 0, dp(8)) })
            if (record.status == "submitted") entries.addView(Button(activity).apply {
                text = "核对并取消这笔申请"; minHeight = dp(52)
                style(this, false) { onCancel(record) }
                isEnabled = !blocked
                isSaveEnabled = false; isSaveFromParentEnabled = false
            })
        }
        root.scrollTo(0, 0)
    }

    fun confirm(action: SellbackAction, name: String, retrying: Boolean, confirmed: () -> Unit, dismissed: () -> Unit) {
        dialog?.dismiss()
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL; setPadding(dp(20), dp(8), dp(20), dp(16))
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
            addView(text("当前账户 · $name\n${if (action.isSubmit) "申请并占用" else "取消申请"} ${sellbackAmount(action.amount)} ESK\n\n${action.terms}"))
        }
        val consent = CheckBox(activity).apply {
            text = if (action.isSubmit) "我已阅读条款，同意申请占用；不代表成交或付款" else "我确认取消这笔本人申请，仅解除占用"
            setTextColor(activity.getColor(R.color.elon_text_primary)); minHeight = dp(52); textSize = 15f
            filterTouchesWhenObscured = true
        }
        val independent = CheckBox(activity).apply {
            text = if (retrying) "我确认使用原幂等键与原内容重试，不创建另一笔申请" else
                "我已核对本人申请，本次是新的独立操作，不是重试未确认的提交"
            setTextColor(activity.getColor(R.color.elon_text_primary)); minHeight = dp(52); textSize = 15f
            filterTouchesWhenObscured = true
        }
        for (check in listOf(consent, independent)) obscureGuard(check)
        content.addView(consent); content.addView(independent)
        content.addView(text("旧请求可能仍在处理；若不确定，请返回核对，不要重复新建。"))
        disableState(content)
        val modal = AlertDialog.Builder(activity).setTitle(if (retrying) "原内容确认重试" else "确认本人操作")
            .setView(ScrollView(activity).apply { addView(content) })
            .setPositiveButton(if (action.isSubmit) "确认提交申请" else "确认取消申请", null)
            .setNegativeButton("返回核对", null).create()
        modal.setOnDismissListener {
            consent.setOnCheckedChangeListener(null); independent.setOnCheckedChangeListener(null)
            eraseText(content); dismissed(); if (dialog === modal) dialog = null
        }
        modal.setOnShowListener {
            modal.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            modal.window?.setBackgroundDrawableResource(R.color.elon_surface_card)
            val button = modal.getButton(AlertDialog.BUTTON_POSITIVE)
            style(button, true) { confirmed(); modal.dismiss() }
            button.isEnabled = false
            val update = { button.isEnabled = consent.isChecked && independent.isChecked }
            consent.setOnCheckedChangeListener { _, _ -> update() }; independent.setOnCheckedChangeListener { _, _ -> update() }
            style(modal.getButton(AlertDialog.BUTTON_NEGATIVE), false) { modal.dismiss() }
        }
        dialog = modal; modal.show()
    }

    fun confirmReviewed(confirmed: () -> Unit) {
        dialog?.dismiss()
        val check = CheckBox(activity).apply {
            text = "我已核对本人记录，知道旧操作仍可能完成；后续申请是独立的新操作，不是原操作重试"
            setTextColor(activity.getColor(R.color.elon_text_primary)); textSize = 15f
            minHeight = dp(52); setPadding(dp(20), dp(16), dp(20), dp(16))
        }
        obscureGuard(check); disableState(check)
        val modal = AlertDialog.Builder(activity).setTitle("核对结果未知的操作")
            .setView(check).setPositiveButton("已核对，重新读取", null).setNegativeButton("继续核对", null).create()
        modal.setOnShowListener {
            modal.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
            modal.window?.setBackgroundDrawableResource(R.color.elon_surface_card)
            val button = modal.getButton(AlertDialog.BUTTON_POSITIVE)
            style(button, true) { confirmed(); modal.dismiss() }; button.isEnabled = false
            check.setOnCheckedChangeListener { _, value -> button.isEnabled = value }
            style(modal.getButton(AlertDialog.BUTTON_NEGATIVE), false) { modal.dismiss() }
        }
        modal.setOnDismissListener {
            check.setOnCheckedChangeListener(null); check.text = ""
            if (dialog === modal) dialog = null
        }
        dialog = modal; modal.show()
    }

    private fun text(value: String) = TextView(activity).apply {
        text = value; textSize = 15f; setTextColor(activity.getColor(R.color.elon_text_secondary))
        setLineSpacing(dp(4).toFloat(), 1f); setTextIsSelectable(false)
        isSaveEnabled = false; isSaveFromParentEnabled = false
    }
    private fun dp(value: Int) = (activity.resources.displayMetrics.density * value).toInt()
    private fun style(button: Button, primary: Boolean, action: () -> Unit) {
        button.isAllCaps = false; obscureGuard(button)
        button.setTextColor(activity.getColor(if (primary) R.color.elon_button_primary_text else R.color.elon_text_primary))
        button.backgroundTintList = null
        button.background = if (primary) GradientDrawable(GradientDrawable.Orientation.LEFT_RIGHT, intArrayOf(
            activity.getColor(R.color.elon_titanium), activity.getColor(R.color.elon_titanium_mid), activity.getColor(R.color.elon_titanium_end)))
            .apply { cornerRadius = dp(24).toFloat() }
        else GradientDrawable().apply { setColor(activity.getColor(R.color.elon_surface_card)); cornerRadius = dp(24).toFloat() }
        button.setOnClickListener { action() }
    }
    private fun obscureGuard(view: View) {
        view.filterTouchesWhenObscured = true
        view.setOnTouchListener { _, event -> event.flags and
            (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0 }
    }
    private fun eraseText(view: View) {
        if (view is TextView) view.text = ""
        if (view is ViewGroup) for (index in 0 until view.childCount) eraseText(view.getChildAt(index))
    }
    private fun disableState(view: View) {
        view.isSaveEnabled = false; view.isSaveFromParentEnabled = false
        if (view is ViewGroup) for (index in 0 until view.childCount) disableState(view.getChildAt(index))
    }
}
