package com.elon.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.InputFilter
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState

internal data class WebChatLocalProjectActions(
    val createProject: (String) -> Boolean,
    val assignConversation: (String, String?) -> Boolean,
)

internal object WebChatLocalProjectDialogs {
    fun showCreate(
        activity: AppCompatActivity,
        actions: WebChatLocalProjectActions,
        onChanged: () -> Unit,
    ) {
        val input = EditText(activity).apply {
            hint = "项目名称"
            setSingleLine(true)
            filters = arrayOf(InputFilter.LengthFilter(MAX_PROJECT_TITLE_LENGTH))
            setTextColor(Color.WHITE)
            setHintTextColor(Color.parseColor("#808080"))
            setPadding(dp(activity, 16), 0, dp(activity, 16), 0)
            background = GradientDrawable().apply {
                shape = GradientDrawable.RECTANGLE
                cornerRadius = dp(activity, 8).toFloat()
                setColor(Color.parseColor("#27282C"))
            }
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("新建本机项目")
            .setView(input)
            .setPositiveButton("新建", null)
            .setNegativeButton("取消", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text?.toString().orEmpty().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                if (!actions.createProject(title)) {
                    input.error = "项目名称已存在"
                    return@setOnClickListener
                }
                dialog.dismiss()
                onChanged()
            }
        }
        dialog.setOnDismissListener { input.clearFocus() }
        dialog.show()
    }

    fun showAssignment(
        activity: AppCompatActivity,
        state: ChatGptWebConversationIndexState,
        conversation: ChatGptWebConversation,
        actions: WebChatLocalProjectActions,
        onChanged: () -> Unit,
    ) {
        if (state.projects.isEmpty()) {
            AlertDialog.Builder(activity)
                .setTitle("整理会话")
                .setMessage("还没有本机项目。")
                .setPositiveButton("新建项目") { _, _ -> showCreate(activity, actions, onChanged) }
                .setNegativeButton("取消", null)
                .show()
            return
        }
        val projectIds = listOf<String?>(null) + state.projects.map { it.id }
        val labels = listOf("不归入项目") + state.projects.map { it.title }
        val selected = projectIds.indexOf(conversation.projectId).coerceAtLeast(0)
        AlertDialog.Builder(activity)
            .setTitle("整理会话")
            .setSingleChoiceItems(labels.toTypedArray(), selected) { dialog, index ->
                val ok = actions.assignConversation(conversation.path, projectIds[index])
                if (ok) {
                    dialog.dismiss()
                    onChanged()
                } else {
                    Toast.makeText(activity, "无法更新项目，请重试", Toast.LENGTH_SHORT).show()
                }
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun dp(activity: AppCompatActivity, value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private const val MAX_PROJECT_TITLE_LENGTH = 80
}
