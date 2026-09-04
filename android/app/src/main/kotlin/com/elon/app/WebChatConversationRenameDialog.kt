package com.elon.app

import android.text.InputType
import android.widget.FrameLayout
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatEditText
import com.elon.app.chatgptweb.ChatGptWebConversation

internal object WebChatConversationRenameDialog {
    const val INPUT_SELECTOR = "web-chat-conversation-rename-input"
    const val COMMIT_SELECTOR = "web-chat-conversation-rename-commit"

    fun show(
        activity: AppCompatActivity,
        conversation: ChatGptWebConversation,
        onSubmit: (String) -> Unit,
    ): AlertDialog {
        val input = AppCompatEditText(activity).apply {
            setText(conversation.title)
            selectAll()
            hint = "会话名称"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            isSingleLine = true
            maxLines = 1
            contentDescription = INPUT_SELECTOR
        }
        val container = FrameLayout(activity).apply {
            val horizontal = dp(activity, 24)
            setPadding(horizontal, dp(activity, 8), horizontal, 0)
            addView(
                input,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                ),
            )
        }
        return AlertDialog.Builder(activity)
            .setTitle("重命名会话")
            .setView(container)
            .setNegativeButton(android.R.string.cancel, null)
            .setPositiveButton(android.R.string.ok, null)
            .create()
            .also { dialog ->
                dialog.setOnShowListener {
                    dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
                        contentDescription = COMMIT_SELECTOR
                        setOnClickListener {
                            val title = WebChatConversationMutationPolicy.normalizedTitle(
                                input.text?.toString().orEmpty(),
                            )
                            if (title == null) {
                                input.error = "请输入 1-${WebChatConversationMutationPolicy.MAX_TITLE_LENGTH} 个字符"
                                return@setOnClickListener
                            }
                            onSubmit(title)
                            dialog.dismiss()
                        }
                    }
                    input.requestFocus()
                }
                dialog.show()
            }
    }

    private fun dp(activity: AppCompatActivity, value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
