package com.elon.app

import android.text.InputType
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.view.ContextThemeWrapper
import androidx.core.content.ContextCompat
import okhttp3.OkHttpClient
import org.json.JSONObject
import kotlin.concurrent.thread

internal class GroupMessageRevisionController(
    private val activity: AppCompatActivity,
    http: OkHttpClient,
    serverUrl: String,
    private val isActive: (String) -> Boolean,
    private val onSaved: (String, JSONObject) -> Unit
) {
    private val api = GroupMessageRevisionApi(activity, http, serverUrl)
    private val context get() = ContextThemeWrapper(activity, R.style.Theme_Elon_GroupSummaryDialog)
    private var dialog: AlertDialog? = null
    private var generation = 0
    private var displayedMessageId: String? = null

    fun close() { generation++; dialog?.dismiss(); dialog = null; displayedMessageId = null }

    fun onMessagesChanged(messages: List<ChatMessage>) {
        if (messages.any { it.id == displayedMessageId && !it.recalledAt.isNullOrBlank() }) close()
    }

    fun actions(groupId: String, message: ChatMessage): List<TopAction> {
        if (message.id.isNullOrBlank() || !message.recalledAt.isNullOrBlank()) return emptyList()
        val result = mutableListOf<TopAction>()
        if (message.role == "user" && message.content.isNotBlank() && !message.content.startsWith("【一龙项目卡片】")) {
            result.add(TopAction("编辑", R.drawable.ic_msg_quote) { edit(groupId, message) })
        }
        if (message.revision > 1) result.add(TopAction("修改记录", R.drawable.ic_popup_history) { history(groupId, message) })
        return result
    }

    private fun edit(groupId: String, message: ChatMessage) {
        val id = message.id ?: return
        close()
        var expected = message.revision
        displayedMessageId = id
        val body = column()
        body.addView(text("保存后标记为已编辑，群成员可查看每一版文字。附件保持原样。"))
        val input = EditText(context).apply {
            setText(message.content)
            minLines = 4
            maxLines = 10
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
            setSelection(text.length)
        }
        body.addView(input)
        val status = text("").apply { accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        body.addView(status)
        val latest = text("").apply { visibility = View.GONE }
        body.addView(latest)
        val confirm = android.widget.Button(context).apply { text = "已核对，继续编辑我的草稿"; visibility = View.GONE }
        body.addView(confirm)
        val currentDialog = AlertDialog.Builder(context).setTitle("编辑消息")
            .setView(scroll(body)).setNegativeButton("取消", null).setPositiveButton("保存修改", null).create()
        dialog = currentDialog
        currentDialog.show()
        currentDialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
            val content = input.text.toString().trim()
            val length = content.codePointCount(0, content.length)
            if (length !in 1..4000) { status.text = "请输入 1 至 4000 字的文字"; return@setOnClickListener }
            status.text = "正在保存…"
            input.isEnabled = false
            setBusy(currentDialog, true)
            run(groupId, { api.save(groupId, id, content, expected) }) { result ->
                input.isEnabled = true
                setBusy(currentDialog, false)
                result.onSuccess { edited ->
                    onSaved(groupId, edited)
                    close()
                    Toast.makeText(activity, "修改已保存，群成员可查看历史", Toast.LENGTH_SHORT).show()
                }.onFailure { failure ->
                    status.text = "${failure.message ?: "保存失败"}\n草稿已保留。"
                    if ((failure as? GroupRevisionFailure)?.status == 409) {
                        setBusy(currentDialog, true)
                        run(groupId, { api.history(groupId, id, limit = 1) }) { historyResult ->
                            setBusy(currentDialog, false)
                            historyResult.onSuccess { data ->
                                val version = data.optJSONArray("revisions")?.optJSONObject(0) ?: return@onSuccess
                                latest.text = "其他设备已保存第 ${version.optLong("revision")} 版：\n${version.optString("content")}"
                                latest.visibility = View.VISIBLE
                                confirm.visibility = View.VISIBLE
                                currentDialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = false
                                confirm.setOnClickListener {
                                    expected = version.optLong("revision")
                                    confirm.visibility = View.GONE
                                    latest.visibility = View.GONE
                                    status.text = "已保留你的草稿，请核对后点击保存"
                                    currentDialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = true
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fun history(groupId: String, message: ChatMessage) {
        val id = message.id ?: return
        close()
        displayedMessageId = id
        val body = column()
        val content = column(0)
        val status = text("正在读取…")
        body.addView(text("按最新到最早排列，每一版均保留完整文字。"))
        body.addView(content)
        body.addView(status)
        val more = android.widget.Button(context).apply { text = "查看更早版本"; visibility = View.GONE }
        body.addView(more)
        var before: Long? = null
        val versions = mutableListOf<GroupTextRevision>()
        dialog = AlertDialog.Builder(context).setTitle("修改记录").setView(scroll(body)).setPositiveButton("关闭", null).show()
        fun load() {
            more.isEnabled = false
            status.text = "正在读取…"
            run(groupId, { api.history(groupId, id, before) }) { result ->
                more.isEnabled = true
                result.onSuccess { data ->
                    val rows = data.optJSONArray("revisions")
                    if (rows != null) for (index in 0 until rows.length()) {
                        val row = rows.getJSONObject(index)
                        versions.add(GroupTextRevision(row.optLong("revision"), row.optString("content"), row.optString("created_at")))
                    }
                    before = if (data.isNull("next_before_revision")) null else data.optLong("next_before_revision").takeIf { it > 0 }
                    renderGroupRevisionHistory(content, versions)
                    status.text = if (before == null) "已显示全部 ${versions.size} 个版本" else ""
                    more.text = "查看更早版本"
                    more.visibility = if (before == null) View.GONE else View.VISIBLE
                }.onFailure { failure ->
                    if ((failure as? GroupRevisionFailure)?.status in listOf(403, 404, 410)) { content.removeAllViews(); versions.clear() }
                    status.text = failure.message ?: "读取失败"
                    more.text = "重新读取"
                    more.visibility = View.VISIBLE
                }
            }
        }
        more.setOnClickListener { load() }
        load()
    }

    private fun run(group: String, work: () -> JSONObject, done: (Result<JSONObject>) -> Unit) {
        val token = generation
        thread(name = "group-message-revisions") {
            val result = runCatching(work)
            activity.runOnUiThread {
                if (token == generation && isActive(group) && !activity.isFinishing && !activity.isDestroyed && dialog?.isShowing == true) done(result)
            }
        }
    }

    private fun column(padding: Int = 20) = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        val px = (padding * resources.displayMetrics.density).toInt()
        setPadding(px, px / 2, px, px / 2)
    }
    private fun text(value: String) = TextView(context).apply {
        text = value
        textSize = 15f
        setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
        setTextIsSelectable(true)
        setPadding(0, 12, 0, 12)
    }
    private fun scroll(body: View) = ScrollView(context).apply { addView(body); isFillViewport = false }
    private fun setBusy(target: AlertDialog, busy: Boolean) {
        target.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = !busy
        target.getButton(AlertDialog.BUTTON_NEGATIVE)?.isEnabled = !busy
        target.setCancelable(!busy)
    }
}
