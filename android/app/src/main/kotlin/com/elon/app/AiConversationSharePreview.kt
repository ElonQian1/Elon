package com.elon.app

import android.text.InputFilter
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

internal data class AiConversationShareTarget(val id: String, val name: String)

internal class AiConversationSharePreview(
    private val activity: AppCompatActivity,
    private val draft: AiConversationShareDraft,
    private val onPreview: () -> Unit,
    private val onSelectTarget: ((AiConversationShareTarget) -> Unit) -> Unit,
    private val onSubmit: (AiConversationShareTarget, String, String, Boolean) -> Unit,
    onDismiss: () -> Unit,
) {
    private val title = field("卡片标题", draft.title, 120, 2)
    private val summary = field("摘要", draft.summary, 300, 4)
    private var target: AiConversationShareTarget? = null
    private val targetButton = row("选择群聊", "ai-conversation-share-target")
    private val status = row("", "ai-conversation-share-status").apply {
        textSize = 13f
        visibility = View.GONE
        minHeight = 0
    }
    private val cover = android.widget.CheckBox(activity).apply {
        text = "使用所选内容中的第一张图片作封面"
        textSize = 13f
        minHeight = dp(48)
        isChecked = true
        setTextColor(color(R.color.elon_text_secondary))
        visibility = if (draft.messages.any { message ->
            message.attachments.orEmpty().any(ChatAttachment::isImage) ||
                message.webChatMessage?.contentParts.orEmpty().any { it.type == "image" }
        }) View.VISIBLE else View.GONE
        contentDescription = "ai-conversation-share-cover"
    }
    private val preview = row("查看 ${draft.messages.size} 条消息", "ai-conversation-share-preview")
    private val content = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(20), dp(4), dp(20), dp(12))
        addView(row("${if (draft.provider == "chatgpt") "ChatGPT" else "Google AI"} · 精选聊天记录", "ai-conversation-share-source").apply {
            textSize = 13f; minHeight = dp(32); setTextColor(color(R.color.elon_text_secondary))
        })
        addView(title)
        addView(summary)
        addView(cover)
        addView(preview)
        addView(targetButton)
        addView(status)
    }
    private val dialog = AlertDialog.Builder(activity)
        .setTitle("发送聊天记录")
        .setView(ScrollView(activity).apply { addView(content); isFillViewport = false })
        .setNegativeButton("取消", null)
        .setPositiveButton("发送", null)
        .create()

    init {
        preview.setOnClickListener { onPreview() }
        targetButton.setCompoundDrawablesWithIntrinsicBounds(R.drawable.ic_home_action_group, 0,
            R.drawable.profile_icon_chevron, 0)
        targetButton.compoundDrawablePadding = dp(8)
        targetButton.setOnClickListener {
            onSelectTarget { selected ->
                if (dialog.isShowing) {
                    target = selected
                    targetButton.text = selected.name
                    dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = true
                }
            }
        }
        dialog.setOnDismissListener { onDismiss() }
        dialog.setOnShowListener {
            dialog.window?.setSoftInputMode(android.view.WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN)
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
                isEnabled = false
                contentDescription = "ai-conversation-share-send"
                setOnClickListener {
                    val selected = target ?: return@setOnClickListener
                    val name = title.text.toString().trim()
                    if (name.isBlank()) { title.error = "请填写标题"; return@setOnClickListener }
                    onSubmit(selected, name, summary.text.toString().trim(), cover.isChecked)
                }
            }
        }
    }

    fun show() { dialog.show() }
    fun dismiss() { dialog.dismiss() }
    fun isShowing() = dialog.isShowing

    fun progress(text: String, cancelPreparation: (() -> Unit)? = null) {
        status.text = text
        status.visibility = View.VISIBLE
        setBusy(true)
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE).apply {
            isEnabled = cancelPreparation != null
            setOnClickListener { cancelPreparation?.invoke() }
        }
    }

    fun failed(text: String) {
        status.text = text
        status.visibility = View.VISIBLE
        setBusy(false)
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener { dialog.dismiss() }
    }

    private fun setBusy(busy: Boolean) {
        listOf(title, summary, cover, targetButton, preview).forEach { it.isEnabled = !busy }
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = !busy && target != null
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled = !busy
        dialog.setCancelable(!busy)
        dialog.setCanceledOnTouchOutside(!busy)
    }

    private fun field(label: String, value: String, max: Int, lines: Int) = EditText(activity).apply {
        hint = label; setText(value); textSize = 16f; maxLines = lines
        filters = arrayOf(InputFilter.LengthFilter(max))
        setTextColor(color(R.color.elon_text_primary))
        setHintTextColor(color(R.color.elon_text_secondary))
        minHeight = dp(48)
        contentDescription = label
        layoutParams = LinearLayout.LayoutParams(-1, -2)
    }

    private fun row(label: String, semanticId: String) = TextView(activity).apply {
        text = label; textSize = 16f; minHeight = dp(48); gravity = Gravity.CENTER_VERTICAL
        maxLines = 2; ellipsize = TextUtils.TruncateAt.END
        contentDescription = semanticId
        setTextColor(color(R.color.elon_text_primary))
        layoutParams = LinearLayout.LayoutParams(-1, -2)
    }
    private fun color(id: Int) = ContextCompat.getColor(activity, id)
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
}
