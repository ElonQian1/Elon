package com.elon.app

import android.text.Editable
import android.text.TextWatcher
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ListView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal class AiConversationShareTargetPicker(
    private val activity: AppCompatActivity,
    http: OkHttpClient,
    server: String,
) {
    private val reader = SocialChatReadChannel(activity, http, server)
    private var dialog: AlertDialog? = null

    fun show(onSelected: (AiConversationShareTarget) -> Unit) {
        close()
        val session = socialSession(activity)
        var groups = emptyList<AiConversationShareTarget>()
        var shown = groups
        val search = EditText(activity).apply { hint = "搜索群聊"; setSingleLine(true) }
        val status = TextView(activity).apply { text = "正在加载群聊" }
        val list = ListView(activity)
        val adapter = ArrayAdapter<String>(activity, android.R.layout.simple_list_item_1)
        list.adapter = adapter
        fun render() {
            val query = search.text.toString().trim()
            shown = groups.filter { query.isBlank() || it.name.contains(query, ignoreCase = true) }
            adapter.clear(); adapter.addAll(shown.map { it.name }); adapter.notifyDataSetChanged()
            status.text = when { groups.isEmpty() -> "暂无可发送的群聊"; shown.isEmpty() -> "没有匹配的群聊"; else -> "" }
        }
        search.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = render()
            override fun afterTextChanged(s: Editable?) = Unit
        })
        val padding = (20 * activity.resources.displayMetrics.density).toInt()
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL; setPadding(padding, 0, padding, padding)
            addView(search); addView(status)
            addView(list, LinearLayout.LayoutParams(-1,
                (activity.resources.displayMetrics.heightPixels * 0.45f).toInt()))
        }
        val screen = AlertDialog.Builder(activity).setTitle("发送到群聊").setView(content)
            .setNegativeButton("取消", null).create()
        dialog = screen
        screen.setOnDismissListener { reader.cancel(); if (dialog === screen) dialog = null }
        list.setOnItemClickListener { _, _, position, _ ->
            val target = shown.getOrNull(position) ?: return@setOnItemClickListener
            if (session != socialSession(activity)) { close(); return@setOnItemClickListener }
            screen.dismiss(); onSelected(target)
        }
        screen.show()
        fun apply(rows: org.json.JSONArray) {
            if (dialog !== screen || session != socialSession(activity)) return
            groups = (0 until rows.length()).mapNotNull { index ->
                val row = rows.optJSONObject(index) ?: return@mapNotNull null
                val id = row.optString("id").takeIf { it.isNotBlank() } ?: return@mapNotNull null
                AiConversationShareTarget(id, row.optString("name").ifBlank { "群聊" })
            }
            render()
        }
        reader.read("groups", "/api/me/groups", "groups", true, ::apply, ::apply) { failure ->
            if (dialog === screen) {
                if (failure.socialAccessDenied()) { groups = emptyList(); render() }
                status.text = "群聊同步失败，请关闭后重试"
            }
        }
    }

    fun close() { dialog?.dismiss(); dialog = null; reader.cancel() }
}
