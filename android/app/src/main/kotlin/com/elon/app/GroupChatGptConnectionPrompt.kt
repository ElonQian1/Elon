package com.elon.app

import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebAccountConnection
import com.elon.app.chatgptweb.ChatGptWebLoginReturn
import com.elon.app.chatgptweb.ChatGptWebOfficialFallbackIntent
import com.google.android.material.bottomsheet.BottomSheetDialog

/** Group-only account entry. Observes local evidence without starting a WebView or sending context. */
internal class GroupChatGptConnectionPrompt(
    private val activity: AppCompatActivity,
    private val valid: () -> Boolean,
) {
    private val connection = ChatGptWebAccountConnection(activity)
    private var unsubscribe: (() -> Unit)? = null
    private var sheet: BottomSheetDialog? = null
    private var selected = false
    private val title = label(14f, R.color.elon_text_primary)
    private val subtitle = label(11f, R.color.elon_text_secondary)
    private val memoryStatus = label(11f, R.color.elon_text_secondary).apply { text = "群聊记忆待接通" }
    val root = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        minimumHeight = dp(48)
        setPadding(0, dp(4), dp(4), dp(4))
        isClickable = true
        isFocusable = true
        val value = android.util.TypedValue()
        activity.theme.resolveAttribute(android.R.attr.selectableItemBackground, value, true)
        if (value.resourceId != 0) setBackgroundResource(value.resourceId)
        addView(title, LinearLayout.LayoutParams(-1, -2))
        addView(subtitle, LinearLayout.LayoutParams(-1, -2))
        addView(memoryStatus, LinearLayout.LayoutParams(-1, -2))
        setOnClickListener { if (selected && valid()) showDetails() }
    }

    fun attach(row: LinearLayout, useChatGpt: Boolean) {
        selected = useChatGpt
        if (root.parent !== row) {
            (root.parent as? ViewGroup)?.removeView(root)
            row.addView(root, LinearLayout.LayoutParams(0, -2, 1f))
        }
        if (useChatGpt) resume() else stop()
        render()
    }

    fun resume() {
        if (selected && valid() && unsubscribe == null) {
            unsubscribe = connection.subscribe { render() }
        }
        render()
    }

    private fun render() {
        root.visibility = if (selected && valid()) View.VISIBLE else View.GONE
        val connected = connection.state() == ChatGptWebAccountConnection.State.CONNECTED
        title.text = if (connected) "我的 ChatGPT ›" else "接入我的 ChatGPT ›"
        subtitle.text = if (connected) "本机已接入 · 群聊记忆待接通" else "聊天不耗算力，又可训练群聊记忆"
        memoryStatus.visibility = if (connected) View.GONE else View.VISIBLE
        root.contentDescription = "${title.text}；${subtitle.text}" + if (connected) "" else "；${memoryStatus.text}"
        root.tag = "group-chatgpt-account"
    }

    private fun showDetails() {
        sheet?.dismiss()
        val connected = connection.state() == ChatGptWebAccountConnection.State.CONNECTED
        val dialog = BottomSheetDialog(activity)
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(20), dp(20), dp(24))
            setBackgroundColor(activity.getColor(R.color.elon_bg_app))
            addView(label(20f, R.color.elon_text_primary).apply { text = "接入我的 ChatGPT" })
            addView(paragraph("聊天不耗算力，又可训练群聊记忆", true))
            addView(paragraph("网页 ChatGPT 聊天不扣一龙工作 AI 算力；实际用量、模型和工具仍受你的 ChatGPT 账号规则约束。切换工作 AI 时会另行确认。"))
            addView(paragraph("长期群聊记忆 · 待接通", true))
            addView(paragraph("目标是你的账号内，每个群对应独立 ChatGPT 项目及长期会话，群改名仍保持关联。当前群分析仍使用临时会话，登录不会立即开启长期记忆，也不会自动上传群消息。"))
            addView(paragraph("这里的“训练群聊记忆”指积累项目和会话上下文，不是训练模型，也不代表自动记住全部群消息。"))
            addView(paragraph("在官方页面登录你自己的账号，登录状态仅保存在此设备，与一龙账号独立。换人使用时请先核对或切换 ChatGPT 账号。确认登录后返回当前群，草稿和所选消息保持不变。"))
            addView(label(16f, R.color.elon_text_primary).apply {
                text = if (connected) "查看我的 ChatGPT 账号 ›" else "登录并接入 ›"
                gravity = Gravity.CENTER_VERTICAL
                minimumHeight = dp(52)
                isClickable = true
                isFocusable = true
                tag = "group-chatgpt-account-open"
                setOnClickListener {
                    dialog.dismiss()
                    if (selected && valid()) activity.startActivity(if (connected) {
                        ChatGptWebOfficialFallbackIntent.create(activity, "https://chatgpt.com/")
                    } else ChatGptWebLoginReturn.intent(activity))
                }
            }, LinearLayout.LayoutParams(-1, -2))
        }
        dialog.setContentView(ScrollView(activity).apply { addView(body) })
        sheet = dialog
        dialog.show()
    }

    fun stop() {
        unsubscribe?.invoke()
        unsubscribe = null
        sheet?.dismiss()
        sheet = null
    }

    fun close() {
        selected = false
        stop()
        (root.parent as? ViewGroup)?.removeView(root)
    }

    private fun paragraph(copy: String, emphasis: Boolean = false) =
        label(if (emphasis) 16f else 14f, if (emphasis) R.color.elon_text_primary else R.color.elon_text_secondary).apply {
            text = copy
            setPadding(0, dp(16), 0, 0)
        }

    private fun label(size: Float, color: Int) = TextView(activity).apply {
        textSize = size
        setTextColor(activity.getColor(color))
    }
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
}
