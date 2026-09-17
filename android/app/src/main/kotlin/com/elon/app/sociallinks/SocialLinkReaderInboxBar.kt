package com.elon.app.sociallinks

import android.content.Context
import android.graphics.Color
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

/**
 * "N 条新消息" banner shown inside the reader, with an inline quick-reply row. The banner
 * only reflects [SocialLinkReaderInbox]; sending goes through the same chat API as the composer.
 */
internal class SocialLinkReaderInboxBar(context: Context, private val onReturnToChat: () -> Unit) : LinearLayout(context), SocialLinkReaderInbox.Ui {
    private fun dp(n: Int) = (resources.displayMetrics.density * n).toInt()
    private val summary = TextView(context).apply { textSize = 13f; setTextColor(Color.parseColor("#F1F2F5")); maxLines = 2; ellipsize = android.text.TextUtils.TruncateAt.END }
    private val replyRow = LinearLayout(context).apply { visibility = View.GONE }
    private val input = EditText(context).apply {
        hint = "快捷回复…"; textSize = 14f; setTextColor(Color.parseColor("#F1F2F5")); setHintTextColor(Color.parseColor("#7F8896"))
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        maxLines = 3; imeOptions = EditorInfo.IME_ACTION_SEND; setHorizontallyScrolling(false)
        setOnEditorActionListener { _, action, _ -> if (action == EditorInfo.IME_ACTION_SEND) { send(); true } else false }
    }
    private val sendButton = action("发送") { send() }
    private var sending = false

    init {
        orientation = VERTICAL
        setBackgroundColor(Color.parseColor("#233043"))
        setPadding(dp(16), dp(8), dp(16), dp(8))
        visibility = View.GONE
        val top = LinearLayout(context).apply { gravity = Gravity.CENTER_VERTICAL }
        top.addView(summary, LayoutParams(0, LayoutParams.WRAP_CONTENT, 1f))
        top.addView(action("回复") { toggleReply() })
        top.addView(action("查看") { onReturnToChat() })
        top.addView(action("忽略") { SocialLinkReaderInbox.clear() })
        addView(top)
        replyRow.addView(input, LayoutParams(0, LayoutParams.WRAP_CONTENT, 1f))
        replyRow.addView(sendButton)
        addView(replyRow)
    }

    private fun action(label: String, onClick: () -> Unit) = TextView(context).apply {
        text = label; textSize = 13f; setTextColor(Color.parseColor("#C5D6EC")); gravity = Gravity.CENTER
        minHeight = dp(40); minWidth = dp(48); setPadding(dp(8), 0, dp(8), 0); isFocusable = true; setOnClickListener { onClick() }
    }

    override fun onInbox(count: Int, latest: SocialLinkReaderInbox.Arrival?) {
        if (count == 0 || latest == null) {
            visibility = View.GONE; replyRow.visibility = View.GONE; return
        }
        val where = latest.conversation.name
        summary.text = "$count 条新消息 · $where\n${latest.sender}：${latest.preview}"
        visibility = View.VISIBLE
        input.hint = "回复 $where…"
    }

    private fun toggleReply() {
        val target = SocialLinkReaderInbox.target() ?: run { onReturnToChat(); return }
        input.hint = "回复 ${target.name}…"
        replyRow.visibility = if (replyRow.visibility == View.VISIBLE) View.GONE else View.VISIBLE
        if (replyRow.visibility == View.VISIBLE) input.requestFocus()
    }

    private fun send() {
        if (sending) return
        val target = SocialLinkReaderInbox.target() ?: return
        val text = input.text.toString().trim()
        if (text.isEmpty()) return
        sending = true; sendButton.text = "发送中…"; input.isEnabled = false
        SocialLinkReaderInbox.reply(context, target, text) { result ->
            sending = false; sendButton.text = "发送"; input.isEnabled = true
            result.fold({
                input.setText(""); replyRow.visibility = View.GONE
                summary.text = "已回复 ${target.name}"
            }, { summary.text = it.message ?: "发送失败，请重试" })
        }
    }

    fun attach() { SocialLinkReaderInbox.bind(context, this) }
    fun detach() { SocialLinkReaderInbox.unbind(this) }
}
