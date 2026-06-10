package com.elon.app

import android.graphics.Color
import android.text.InputFilter
import android.text.InputType
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class ProjectSpaceAnnouncementEditor(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val onSubmit: (ProjectChannel, String, (Result<Unit>) -> Unit) -> Unit
) {
    fun show(space: ProjectSpace, channel: ProjectChannel, currentText: String) {
        if (!canEditProjectAnnouncement(space.project.role)) {
            Toast.makeText(activity, "只有项目创建者可以编辑公告", Toast.LENGTH_SHORT).show()
            return
        }
        val input = EditText(activity).apply {
            setText(currentText.takeUnless { it == DEFAULT_ANNOUNCEMENT_TEXT }.orEmpty())
            hint = "请输入项目公告、规则或重要更新"
            minLines = 4
            maxLines = 8
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            filters = arrayOf(InputFilter.LengthFilter(PROJECT_ANNOUNCEMENT_MAX_CHARS))
            setTextColor(Color.parseColor("#F2F5FA"))
            setHintTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(12), dp(10), dp(12), dp(10))
            background = roundedInputBackground()
            setSelection(text?.length ?: 0)
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("编辑公告")
            .setView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(20), dp(6), dp(20), 0)
                addView(input, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ))
            })
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val content = input.text?.toString()?.trim().orEmpty()
                if (content.isBlank()) {
                    Toast.makeText(activity, "公告内容不能为空", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                save(channel, content, dialog)
            }
            input.requestFocus()
        }
        dialog.show()
    }

    private fun save(channel: ProjectChannel, content: String, dialog: AlertDialog) {
        val saveButton = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
        saveButton?.isEnabled = false
        onSubmit(channel, content) { result ->
            saveButton?.isEnabled = true
            result.onSuccess {
                dialog.dismiss()
            }.onFailure { error ->
                Toast.makeText(activity, error.message ?: "保存公告失败", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun roundedInputBackground(): android.graphics.drawable.GradientDrawable {
        return android.graphics.drawable.GradientDrawable().apply {
            setColor(Color.parseColor("#181B20"))
            cornerRadius = dp(8).toFloat()
        }
    }

    private companion object {
        const val DEFAULT_ANNOUNCEMENT_TEXT = "不得发布与主题内容不相关的帖子。"
        const val PROJECT_ANNOUNCEMENT_MAX_CHARS = 300
    }
}
