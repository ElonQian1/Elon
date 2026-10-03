package com.elon.app

import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.databinding.ActivityMainBinding

/** Controls sit above the list, leaving messages and the composer unobscured. */
internal class MessageTimelineNavigation(private val list: RecyclerView, content: View, private val reader: SocialChatReadChannel) {
    constructor(binding: ActivityMainBinding, reader: SocialChatReadChannel) : this(binding.chatList, binding.chatListFrame, reader)
    private val strip = LinearLayout(list.context)
    private val older = Button(list.context).apply { text = "加载更早消息"; minHeight = (48 * resources.displayMetrics.density).toInt() }
    private val latest = Button(list.context).apply { text = "回到最新消息"; minHeight = older.minHeight }
    private val newer = Button(list.context).apply { text = "继续向后阅读"; minHeight = older.minHeight }
    private var saved: Pair<String?, Int>? = null
    private var key: String? = null
    private var messages = emptyList<ChatMessage>()
    private var refresh: ((Boolean) -> Unit)? = null
    private var waiting: String? = null
    private var loading: String? = null
    private val reading = ReadingBookmarkControls(list, reader, { messages }) { query ->
        key?.let { reader.locate(it, query); saved = null; loading = "around"; refresh?.invoke(false) }
    }
    private val edge = MessageHistoryScroll(list, { load("older") }, { load(if (reading.historical) "newer" else "latest") })
    init {
        strip.visibility = View.GONE
        strip.addView(older); strip.addView(latest); strip.addView(newer)
        val parent = content.parent as LinearLayout
        parent.addView(strip, parent.indexOfChild(content), LinearLayout.LayoutParams(-1, -2))
        parent.addView(reading.strip, parent.indexOfChild(content), LinearLayout.LayoutParams(-1, -2))
        older.setOnClickListener { load("older") }
        latest.setOnClickListener { load("latest") }
        newer.setOnClickListener { load("newer") }
        reader.onIdle = {
            loading = null
            val requested = waiting; waiting = null
            buttons()
            reading.idle()
            if (requested != null) load(requested)
        }
    }
    /** Rebind before reading: an empty sync intentionally does not invoke the rows callback. */
    fun open(key: String, messages: List<ChatMessage>, restorePosition: Boolean = false, refresh: (Boolean) -> Unit) {
        close()
        if (!restorePosition) reader.timeline(key)?.latest()
        update(key, messages, refresh)
        reading.open(key)
    }
    fun update(key: String, messages: List<ChatMessage>, refresh: (Boolean) -> Unit) {
        if (this.key != key) { waiting = null; saved = null; loading = null }
        this.key = key; this.messages = messages; this.refresh = refresh
        if (loading == "latest") reading.latest()
        val manager = list.layoutManager as? LinearLayoutManager
        saved?.let { (id, offset) ->
            val index = messages.indexOfFirst { it.id == id }
            if (index >= 0) manager?.scrollToPositionWithOffset(index, offset)
            saved = null
        }
        buttons()
        reading.updated()
    }
    private fun buttons() {
        val state = key?.let(reader::timeline)
        edge.enabled = state != null
        strip.visibility = if (state != null) View.VISIBLE else View.GONE
        if (state == null) { waiting = null; return }
        older.visibility = if (state.hasOlder) View.VISIBLE else View.GONE
        latest.visibility = View.VISIBLE
        older.isEnabled = loading == null && waiting == null
        latest.isEnabled = older.isEnabled
        newer.visibility = if (reading.historical && state.hasNewer) View.VISIBLE else View.GONE
        newer.isEnabled = older.isEnabled
        older.text = if (loading == "older" || waiting == "older") "正在加载…" else "加载更早消息"
        latest.text = if (loading == "latest" || waiting == "latest") "正在刷新…" else if (state.hasNewer) "回到最新消息" else "刷新最新消息"
    }
    private fun load(direction: String) {
        val current = key ?: return
        val state = reader.timeline(current) ?: return
        if (loading != null || (direction == "older" && !state.hasOlder) || (direction == "newer" && !state.hasNewer)) return
        if (reader.isReading(current)) { waiting = direction; buttons(); return }
        waiting = null; loading = direction
        if (direction == "older" || direction == "newer") {
            val manager = list.layoutManager as? LinearLayoutManager
            val position = manager?.findFirstVisibleItemPosition() ?: -1
            if (position >= 0) saved = messages.getOrNull(position)?.id to (manager?.findViewByPosition(position)?.top ?: 0)
            if (direction == "older") state.older() else state.newer()
        } else { reading.capture(); saved = null; state.latest() }
        buttons(); refresh?.invoke(direction == "latest")
    }
    fun messageActions(message: ChatMessage): List<TopAction> = reading.messageActions(message)
    fun prepareLatest() { reading.capture(); loading = "latest"; key?.let { reader.timeline(it)?.latest() } }
    fun close() { reading.close(); strip.visibility = View.GONE; saved = null; key = null; refresh = null; messages = emptyList(); waiting = null; loading = null; edge.enabled = false }
}
