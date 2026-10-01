package com.elon.app

import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import androidx.recyclerview.widget.LinearLayoutManager
import com.elon.app.databinding.ActivityMainBinding

/** Controls sit above the list, leaving messages and the composer unobscured. */
internal class MessageTimelineNavigation(private val binding: ActivityMainBinding, private val reader: SocialChatReadChannel) {
    private val strip = LinearLayout(binding.root.context)
    private val older = Button(binding.root.context).apply { text = "加载更早消息"; minHeight = (48 * resources.displayMetrics.density).toInt() }
    private val latest = Button(binding.root.context).apply { text = "回到最新消息"; minHeight = older.minHeight }
    private var saved: Pair<String?, Int>? = null
    init {
        strip.visibility = View.GONE
        strip.addView(older); strip.addView(latest)
        val parent = binding.chatListFrame.parent as LinearLayout
        parent.addView(strip, parent.indexOfChild(binding.chatListFrame), LinearLayout.LayoutParams(-1, -2))
    }
    fun update(key: String, messages: List<ChatMessage>, refresh: (Boolean) -> Unit) {
        val state = reader.timeline(key) ?: return
        val manager = binding.chatList.layoutManager as? LinearLayoutManager
        saved?.let { (id, offset) ->
            val index = messages.indexOfFirst { it.id == id }
            if (index >= 0) manager?.scrollToPositionWithOffset(index, offset)
            saved = null
        }
        strip.visibility = if (state.hasOlder || state.hasNewer) View.VISIBLE else View.GONE
        older.visibility = if (state.hasOlder) View.VISIBLE else View.GONE
        latest.visibility = if (state.hasNewer) View.VISIBLE else View.GONE
        older.setOnClickListener {
            val position = manager?.findFirstVisibleItemPosition() ?: -1
            if (position >= 0) saved = messages.getOrNull(position)?.id to (manager?.findViewByPosition(position)?.top ?: 0)
            state.older(); refresh(false)
        }
        latest.setOnClickListener { state.latest(); refresh(true) }
    }
    fun close() { strip.visibility = View.GONE; saved = null }
}
