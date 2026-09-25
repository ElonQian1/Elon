package com.elon.app

import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONArray
import org.json.JSONObject

internal object GroupAiSelectionPreview {
    fun show(activity: AppCompatActivity, messages: List<ChatMessage>, allowProjectMemory: Boolean = false,
        submit: (String, JSONObject, Boolean) -> Unit) {
        val ids = messages.mapNotNull { it.id?.takeIf(String::isNotBlank) }.distinct()
        val revisions = JSONObject().apply { messages.forEach { message -> message.id?.let { put(it, message.revision) } } }
        if (ids.size != messages.size || ids.size !in 1..100 || messages.any { (it.content.isBlank() && it.attachments.isNullOrEmpty()) || it.isRecalled() }) {
            Toast.makeText(activity, "请选择 1 至 100 条已同步且未撤回的消息", Toast.LENGTH_LONG).show()
            return
        }
        val space = (20 * activity.resources.displayMetrics.density).toInt()
        val question = EditText(activity).apply {
            hint = "想问什么？留空则总结所选消息"
            contentDescription = "group-ai-selection-question"
            minLines = 2
            maxLines = 5
            filters = arrayOf(android.text.InputFilter.LengthFilter(2000))
        }
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(space, 0, space, 0)
            addView(TextView(activity).apply {
                text = "发送选中的 ${ids.size} 条消息及其图片、文件，回答将发布到当前群。不会附带未选择的消息。"
                setPadding(0, space / 2, 0, space)
            })
            addView(question)
        }
        val share = android.widget.CheckBox(activity).apply {
            text = "允许群成员用自己的 ChatGPT 继续讨论所选记录、问题和回答"
            contentDescription = "group-ai-selection-allow-continue"
        }
        content.addView(share)
        val memory = android.widget.CheckBox(activity).apply {
            text = "加入本群 ChatGPT 项目，结合本群已有记忆分析"
            contentDescription = "group-ai-selection-project-memory"
            isChecked = allowProjectMemory
            visibility = if (allowProjectMemory) android.view.View.VISIBLE else android.view.View.GONE
        }
        content.addView(memory)
        if (allowProjectMemory) content.addView(TextView(activity).apply {
            text = "取消勾选后，仅分析所选消息，不使用本群既有记忆。"
        })
        AlertDialog.Builder(activity).setTitle("AI 分析所选消息").setView(content)
            .setNegativeButton("取消", null)
            .setPositiveButton("分析并回复群聊") { _, _ ->
                submit(ids.last(), JSONObject().put("message_ids", JSONArray(ids)).put("message_revisions", revisions)
                    .put("question", question.text.toString()).put("allow_continue", share.isChecked).put("attachment_transport_version", 1),
                    allowProjectMemory && memory.isChecked)
            }.show().getButton(AlertDialog.BUTTON_POSITIVE).contentDescription = "group-ai-selection-submit"
    }
}
