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
    fun show(activity: AppCompatActivity, messages: List<ChatMessage>, submit: (String, JSONObject) -> Unit) {
        val ids = messages.mapNotNull { it.id?.takeIf(String::isNotBlank) }.distinct()
        val revisions = JSONObject().apply { messages.forEach { message -> message.id?.let { put(it, message.revision) } } }
        if (ids.size != messages.size || ids.size !in 1..100 || messages.any { it.content.isBlank() || it.isRecalled() }) {
            Toast.makeText(activity, "请选择 1 至 100 条已同步的文字消息；纯附件暂不能分析", Toast.LENGTH_LONG).show()
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
                text = "仅发送选中的 ${ids.size} 条文字，回答将发布到当前群。不会附带其他群消息或附件。"
                setPadding(0, space / 2, 0, space)
            })
            addView(question)
        }
        AlertDialog.Builder(activity).setTitle("AI 分析所选消息").setView(content)
            .setNegativeButton("取消", null)
            .setPositiveButton("分析并回复群聊") { _, _ ->
                submit(ids.last(), JSONObject().put("message_ids", JSONArray(ids)).put("message_revisions", revisions)
                    .put("question", question.text.toString()))
            }.show().getButton(AlertDialog.BUTTON_POSITIVE).contentDescription = "group-ai-selection-submit"
    }
}
