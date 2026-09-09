package com.elon.app.chatgptweb

import android.graphics.Color
import android.os.SystemClock
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.BaseAdapter
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.ListView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.WebChatConsumerCommandStatus
import com.elon.app.WebChatConsumerPort

/** One visible page; official continuation handles never replace the recent-history cache. */
internal class ChatGptWebDirectoryBrowserView(
    activity: AppCompatActivity,
    private val port: () -> WebChatConsumerPort?,
    private val onReturn: () -> Unit,
    private val onConversation: (String) -> Unit,
    private val dp: (Int) -> Int,
) : LinearLayout(activity) {
    private data class Selection(val scope: String, val title: String, val handle: String = "",
        val number: Int = 1, val previous: List<String> = emptyList(), val parent: Selection? = null)
    private var selection = Selection("conversations", "未归入项目")
    private var pending = selection
    private var page: ChatGptWebDirectoryPage? = null
    private var owner: WebChatConsumerPort? = null
    private var requestId: String? = null
    private var lastRequestId: String? = null
    private var poll: Runnable? = null
    private var busy = false
    private var failed = false
    private var expired = false
    private var visibleRows: List<Any> = emptyList()
    private val title = label(16f)
    private val status = label(13f)
    private val entries = Entries()
    private val list = ListView(activity).apply { adapter = entries; dividerHeight = 0 }
    private val previous = icon(android.R.drawable.ic_media_previous, "上一页") {
        selection.previous.lastOrNull()?.let {
            load(selection.copy(handle = it, number = selection.number - 1, previous = selection.previous.dropLast(1)))
        }
    }
    private val next = icon(android.R.drawable.ic_media_next, "下一页") {
        page?.nextHandle?.let {
            load(selection.copy(handle = it, number = selection.number + 1,
                previous = (selection.previous + selection.handle).takeLast(24)))
        }
    }
    private val retry = label(14f).apply {
        text = "重试"; gravity = Gravity.CENTER; contentDescription = "web-chat-directory-retry"
        setOnClickListener { load(if (expired) selection.copy(handle = "", number = 1, previous = emptyList()) else pending) }
    }

    init {
        orientation = VERTICAL
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(icon(android.R.drawable.ic_media_previous, "返回会话目录") {
                selection.parent?.let(::load) ?: onReturn()
            })
            addView(title, LayoutParams(0, dp(52), 1f))
            addView(icon(android.R.drawable.ic_popup_sync, "刷新全部会话目录") {
                refresh()
            })
        })
        status.setPadding(dp(8), dp(8), dp(8), dp(8))
        status.contentDescription = "web-chat-directory-status"
        status.accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        addView(status)
        addView(list, LayoutParams(-1, 0, 1f))
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(previous)
            addView(retry, LayoutParams(0, dp(48), 1f))
            addView(next)
        })
    }

    fun open(scope: String, name: String) {
        owner = port()
        selection = Selection(scope, name)
        load(selection)
    }

    fun current(): Boolean = owner == null || port() === owner &&
        (page == null || busy || owner?.directoryPage()?.requestId == page?.requestId)

    fun refresh() { load(selection.copy(handle = "", number = 1, previous = emptyList())) }

    fun stop() {
        poll?.let(::removeCallbacks); poll = null
        (requestId ?: lastRequestId)?.let { owner?.cancelDirectoryPage(it) }
        requestId = null; lastRequestId = null; busy = false
    }

    override fun onDetachedFromWindow() { stop(); super.onDetachedFromWindow() }

    private fun load(target: Selection) {
        val current = owner ?: port()?.also { owner = it } ?: run {
            pending = target; failed = true; busy = false; render(); return
        }
        if (port() !== current) { onReturn(); return }
        // Keep successful waypoints across page reads. Cancel only on failure/dismissal.
        poll?.let(::removeCallbacks)
        requestId?.let { current.cancelDirectoryPage(it) }
        requestId = null
        pending = target; busy = true; failed = false; expired = false
        render()
        val started = SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (poll !== this) return
                if (port() !== current || !isAttachedToWindow) { stop(); return }
                if (SystemClock.elapsedRealtime() - started >= 18_000) { finish(false); return }
                if (requestId == null) {
                    val result = current.browseDirectoryPage(target.scope, target.handle)
                    if (!result.accepted || result.requestId == null) {
                        if (result.error in setOf("adapter_generation_not_ready", "bridge_not_ready")) {
                            postDelayed(this, 400); return
                        }
                        finish(false); return
                    }
                    requestId = result.requestId; lastRequestId = result.requestId
                }
                val value = current.directoryPage()?.takeIf { it.requestId == requestId &&
                    it.scope == target.scope && it.requestedHandle == target.handle }
                val command = current.state().commandRequests.firstOrNull { it.id == requestId }
                when (command?.status) {
                    WebChatConsumerCommandStatus.SUCCEEDED -> {
                        if (value != null) {
                            page = value; selection = target.copy(handle = value.handle)
                            list.setSelection(0)
                        }
                        finish(value != null)
                    }
                    WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT -> {
                        expired = command.detail == "directory_page_expired" || command.detail == "directory_context_changed"
                        finish(false)
                    }
                    else -> postDelayed(this, 250)
                }
            }
        }
        poll = task
        post(task)
    }

    private fun finish(success: Boolean) {
        if (!success) requestId?.let { owner?.cancelDirectoryPage(it) }
        poll?.let(::removeCallbacks); poll = null
        requestId = null; busy = false; failed = !success
        render()
    }

    private fun render() {
        visibleRows = if (selection.scope == "projects") page?.projects.orEmpty() else
            page?.conversations.orEmpty().filter {
                selection.scope != "conversations" || it.projectId.isNullOrBlank()
            }.distinctBy { it.id }
        title.text = "${selection.title} · 第 ${selection.number} 页"
        status.text = when {
            busy -> "正在读取${pending.title}…"
            expired -> "目录已更新，请重新读取"
            failed -> "读取失败，已保留上一页"
            page?.complete == true && entries.count == 0 -> "本页没有${selection.title}，已到最后一页"
            page?.complete == true -> "已到最后一页"
            page?.nextHandle != null -> "还有更多会话"
            else -> "目录尚未确认完整，可刷新重试"
        }
        previous.isEnabled = !busy && selection.previous.isNotEmpty()
        next.isEnabled = !busy && page?.nextHandle != null
        retry.visibility = if (failed) VISIBLE else INVISIBLE
        retry.isEnabled = !busy
        retry.text = if (expired) "重新读取" else "重试"
        list.isEnabled = !busy
        entries.notifyDataSetChanged()
    }

    private fun label(size: Float) = TextView(context).apply {
        textSize = size; setTextColor(Color.parseColor("#F4F4F4"))
        gravity = Gravity.CENTER_VERTICAL; maxLines = 2; ellipsize = TextUtils.TruncateAt.END
    }

    private fun icon(resource: Int, name: String, action: () -> Unit) = ImageButton(context).apply {
        layoutParams = LayoutParams(dp(48), dp(48))
        setImageResource(resource); setColorFilter(Color.WHITE); setBackgroundColor(Color.TRANSPARENT)
        contentDescription = name; tooltipText = name; setOnClickListener { action() }
    }

    private inner class Entries : BaseAdapter() {
        override fun getCount() = visibleRows.size
        override fun getItem(position: Int): Any = visibleRows[position]
        override fun getItemId(position: Int) = position.toLong()
        override fun getView(position: Int, recycled: View?, parent: ViewGroup): View {
            val row = getItem(position)
            return ((recycled as? TextView) ?: label(15f).apply {
                layoutParams = android.widget.AbsListView.LayoutParams(-1, dp(64))
                setPadding(dp(8), dp(8), dp(8), dp(8))
            }).apply {
                text = if (row is ChatGptWebProject) row.title else (row as ChatGptWebConversation).title
                contentDescription = if (row is ChatGptWebProject) "web-chat-directory-project:${row.id}"
                    else "web-chat-directory-conversation:${(row as ChatGptWebConversation).id}"
                isEnabled = !busy
                setOnClickListener {
                    if (busy || port() !== owner) return@setOnClickListener
                    if (row is ChatGptWebProject) load(Selection(row.id, row.title, parent = selection))
                    else { stop(); onConversation((row as ChatGptWebConversation).path) }
                }
            }
        }
    }
}
