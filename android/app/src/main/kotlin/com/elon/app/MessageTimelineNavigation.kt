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
    private var saved: Pair<String?, Int>? = null
    private var key: String? = null
    private var messages = emptyList<ChatMessage>()
    private var refresh: ((Boolean) -> Unit)? = null
    private var waiting = false
    private var loadingHistory = false
    private val edge = MessageHistoryScroll(list, ::loadOlder)
    init {
        strip.visibility = View.GONE
        strip.addView(older); strip.addView(latest)
        val parent = content.parent as LinearLayout
        parent.addView(strip, parent.indexOfChild(content), LinearLayout.LayoutParams(-1, -2))
        older.setOnClickListener { loadOlder() }
        latest.setOnClickListener { waiting = false; saved = null; key?.let { reader.timeline(it)?.latest() }; refresh?.invoke(true) }
        reader.onIdle = {
            loadingHistory = false
            buttons()
            if (waiting) loadOlder()
        }
    }
    fun update(key: String, messages: List<ChatMessage>, refresh: (Boolean) -> Unit) {
        if (this.key != key) { waiting = false; saved = null; loadingHistory = false }
        this.key = key; this.messages = messages; this.refresh = refresh
        val manager = list.layoutManager as? LinearLayoutManager
        saved?.let { (id, offset) ->
            val index = messages.indexOfFirst { it.id == id }
            if (index >= 0) manager?.scrollToPositionWithOffset(index, offset)
            saved = null
        }
        buttons()
    }
    private fun buttons() {
        val state = key?.let(reader::timeline)
        edge.enabled = state?.hasOlder == true
        strip.visibility = if (state != null && (state.hasOlder || state.hasNewer)) View.VISIBLE else View.GONE
        if (state == null) { waiting = false; return }
        older.visibility = if (state.hasOlder) View.VISIBLE else View.GONE
        latest.visibility = if (state.hasNewer) View.VISIBLE else View.GONE
        older.isEnabled = !loadingHistory && !waiting
        older.text = if (loadingHistory || waiting) "正在加载…" else "加载更早消息"
    }
    private fun loadOlder() {
        val current = key ?: return
        val state = reader.timeline(current) ?: return
        if (!state.hasOlder || loadingHistory) { waiting = false; return }
        if (reader.isReading(current)) { waiting = true; buttons(); return }
        waiting = false; loadingHistory = true
        val manager = list.layoutManager as? LinearLayoutManager
        val position = manager?.findFirstVisibleItemPosition() ?: -1
        if (position >= 0) saved = messages.getOrNull(position)?.id to (manager?.findViewByPosition(position)?.top ?: 0)
        state.older(); buttons(); refresh?.invoke(false)
    }
    fun close() { strip.visibility = View.GONE; saved = null; key = null; refresh = null; messages = emptyList(); waiting = false; loadingHistory = false; edge.enabled = false }
}
