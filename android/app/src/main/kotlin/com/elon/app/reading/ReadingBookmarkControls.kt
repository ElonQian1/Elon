package com.elon.app

import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.widget.PopupMenu
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import org.json.JSONObject

/** All positions are message-relative fractions; RecyclerView indexes are used only for the current render. */
internal class ReadingBookmarkControls(private val list: RecyclerView, private val reader: SocialChatReadChannel,
    private val rows: () -> List<ChatMessage>, private val locate: (Map<String, String>) -> Unit) {
    val strip = LinearLayout(list.context).apply { orientation = LinearLayout.VERTICAL }
    private val status = TextView(list.context)
    private val button = Button(list.context).apply { text = "阅读书签"; minHeight = (48 * resources.displayMetrics.density).toInt() }
    private var store: ReadingPositions? = null
    private var key = ""
    private var userScrolled = false
    private var historyBeforeJump = false
    private var pending: Triple<String, Boolean, JSONObject?>? = null
    private var dialog: androidx.appcompat.app.AlertDialog? = null
    private var deleted: JSONObject? = null
    var historical = false; private set
    init {
        strip.addView(button); strip.addView(status); strip.visibility = View.GONE
        button.setOnClickListener { show() }
        list.addOnScrollListener(object : RecyclerView.OnScrollListener() {
            override fun onScrollStateChanged(view: RecyclerView, state: Int) {
                if (state == RecyclerView.SCROLL_STATE_DRAGGING) userScrolled = true
                if (state == RecyclerView.SCROLL_STATE_IDLE) capture()
            }
        })
    }
    fun open(key: String) {
        close(); this.key = key
        store = reader.readingPositions(key, ::render).also { it.load() }; render()
    }
    private fun render() {
        val value = store ?: return
        strip.visibility = if (value.supported && !value.denied) View.VISIBLE else View.GONE
        val active = value.bookmarks.find { it.optString("id") == value.active }
        status.text = value.error.ifBlank { if (active != null) "正在续读：${active.optString("title")}" else if (historical) "正在阅读历史消息" else if (value.pending > 0) "阅读位置已保存到本机，待同步" else "" }
        status.visibility = if (status.text.isEmpty()) View.GONE else View.VISIBLE
    }
    fun capture() {
        if (!userScrolled || pending != null || !list.isShown) return
        val manager = list.layoutManager as? LinearLayoutManager ?: return
        val index = manager.findFirstVisibleItemPosition()
        val row = rows().getOrNull(index) ?: return
        val node = manager.findViewByPosition(index) ?: return
        val id = row.id ?: return
        val fraction = ((list.paddingTop - node.top).toDouble() / node.height.coerceAtLeast(1)).coerceIn(0.0, .99)
        store?.putPosition(JSONObject().put("message_id", id).put("fraction", fraction)); userScrolled = false
    }
    fun messageActions(anchor: View, message: ChatMessage, fallback: (View, ChatMessage) -> Unit) {
        if (store?.supported != true || message.id.isNullOrBlank()) { fallback(anchor, message); return }
        PopupMenu(list.context, anchor).apply {
            menu.add("添加阅读书签").setOnMenuItemClickListener { editor(store?.bookmarks?.find { it.optJSONObject("anchor")?.optString("message_id") == message.id }, message); true }
            menu.add("将此处设为当前续读位置").setOnMenuItemClickListener { store?.putPosition(JSONObject().put("message_id", message.id).put("fraction", 0.0)); true }
            menu.add("其他消息操作").setOnMenuItemClickListener { fallback(anchor, message); true }
            show()
        }
    }
    private fun builder(title: String) = MaterialAlertDialogBuilder(list.context).setTitle(title).setNegativeButton("关闭", null)
    private fun display(builder: MaterialAlertDialogBuilder) { dialog?.dismiss(); dialog = builder.show() }
    private fun editor(item: JSONObject?, message: ChatMessage? = null) {
        val box = LinearLayout(list.context).apply { orientation = LinearLayout.VERTICAL; setPadding(24, 8, 24, 8) }
        val title = EditText(list.context).apply { hint = "书签名称"; setText(item?.optString("title")); filters = arrayOf(android.text.InputFilter.LengthFilter(80)) }
        val note = EditText(list.context).apply { hint = "备注（可选）"; setText(item?.optString("note")); filters = arrayOf(android.text.InputFilter.LengthFilter(1000)) }
        box.addView(title); box.addView(note)
        display(builder(if (item == null) "添加书签" else "编辑书签").setView(box).setPositiveButton("保存") { _, _ ->
            val ok = if (item != null) store?.rename(item.getString("id"), title.text.toString(), note.text.toString()) == true
                else message != null && store?.add(message, title.text.toString(), note.text.toString()) != null
            Toast.makeText(list.context, if (ok) "书签已保存" else store?.error ?: "保存失败", Toast.LENGTH_SHORT).show(); store?.flush()
        })
    }
    private fun show() {
        val value = store ?: return
        val items = value.bookmarks
        val labels = mutableListOf("标记当前阅读位置", "继续上次会话阅读", "刷新与同步", "处理未同步的修改", "处理会话阅读位置冲突", "撤销刚才的删除")
        labels.addAll(items.map { it.optString("title") + if (value.candidates(it.getString("id")).isNotEmpty()) " · 位置冲突" else "" })
        display(builder("阅读书签").setItems(labels.toTypedArray()) { _, index ->
            when (index) {
                0 -> { val position = (list.layoutManager as? LinearLayoutManager)?.findFirstVisibleItemPosition() ?: -1; rows().getOrNull(position)?.let { editor(null, it) } }
                1 -> if (value.position("") != null) jump("", true) else Toast.makeText(list.context, "暂无已保存的阅读位置", Toast.LENGTH_SHORT).show()
                2 -> { value.load(); value.flush() }
                3 -> pendingChanges()
                4 -> conflicts("")
                5 -> { deleted?.let { if (value.restore(it) != null) deleted = null }; show() }
                else -> actions(items[index - 6])
            }
        })
    }
    private fun pendingChanges() {
        val value = store ?: return
        val blocked = value.blocked
        if (blocked.isEmpty()) { Toast.makeText(list.context, "没有需要处理的修改", Toast.LENGTH_SHORT).show(); return }
        display(builder("选择未同步的修改").setItems(blocked.map { it.optString("title", it.optString("action")) }.toTypedArray()) { _, index ->
            val op = blocked[index]
            display(builder("处理本机修改").setMessage(op.optString("title") + "\n" + op.optString("note"))
                .setPositiveButton("保留本机修改并重试") { _, _ -> value.retryOperation(op.getString("operation_id"), false) }
                .setNeutralButton("放弃本机修改") { _, _ -> value.retryOperation(op.getString("operation_id"), true) })
        })
    }
    private fun actions(item: JSONObject) {
        val id = item.getString("id")
        display(builder(item.optString("title")).setItems(arrayOf("继续阅读", "回到原始标记", "编辑名称与备注", "处理阅读位置冲突", "删除书签", "另存为新书签")) { _, index ->
            when (index) {
                0 -> jump(id, true)
                1 -> jump(id, false)
                2 -> editor(item)
                3 -> conflicts(id)
                4 -> display(builder("删除这个书签？").setMessage("只删除此书签及它的阅读进度，不会删除聊天消息。").setPositiveButton("删除") { _, _ -> if (store?.remove(id) == true) { deleted = JSONObject(item.toString()); Toast.makeText(list.context, "已删除，可在阅读书签中撤销", Toast.LENGTH_LONG).show() } })
                5 -> store?.add(ChatMessage("user", "", id = item.getJSONObject("anchor").getString("message_id")), item.optString("title") + "（副本）", item.optString("note"), true)
            }
        })
    }
    private fun conflicts(id: String) {
        val value = store ?: return
        val positions = listOfNotNull(value.progress(id)?.optJSONObject("position")) + value.candidates(id).mapNotNull { it.optJSONObject("position") }
        if (positions.size < 2) { Toast.makeText(list.context, "没有待处理的冲突", Toast.LENGTH_SHORT).show(); return }
        display(builder("选择要保留的阅读位置").setItems(positions.map { it.optString("created_at", it.optString("message_id")) }.toTypedArray()) { _, index -> value.resolve(id, positions[index]) })
    }
    private fun jump(id: String, resume: Boolean) {
        val value = store ?: return
        capture(); userScrolled = false; historyBeforeJump = historical; pending = Triple(id, resume, value.position(id, resume)); historical = true
        locate(value.query(id, resume)); render()
    }
    fun updated() {
        val request = pending ?: return
        val target = reader.timeline(key)?.target ?: return
        val id = target.optString("resolved_id")
        val index = rows().indexOfFirst { it.id == id }
        if (index < 0) return
        pending = null
        val manager = list.layoutManager as? LinearLayoutManager ?: return
        val fraction = if (target.optString("status") == "exact") {
            if (request.third?.optString("message_id") == id) request.third?.optDouble("fraction", 0.0) ?: 0.0 else target.optDouble("fraction", 0.0)
        } else 0.0
        manager.scrollToPositionWithOffset(index, 0)
        val expectedKey = key
        val expectedStore = store
        list.post { if (key == expectedKey && store === expectedStore) { manager.findViewByPosition(index)?.let { manager.scrollToPositionWithOffset(index, -(it.height * fraction).toInt()) }; store?.activate(request.first); render() } }
        if (target.optString("status") != "exact") Toast.makeText(list.context, "原消息已删除，已定位附近消息", Toast.LENGTH_LONG).show()
    }
    fun idle() {
        if (pending != null) { pending = null; historical = historyBeforeJump; render(); status.text = "书签定位失败，请重试"; status.visibility = View.VISIBLE }
    }
    fun latest() { capture(); pending = null; historical = false; store?.activate(""); render() }
    fun close() { capture(); store?.flush(); store?.close(); store = null; key = ""; historical = false; userScrolled = false; pending = null; deleted = null; dialog?.dismiss(); dialog = null; strip.visibility = View.GONE }
}
