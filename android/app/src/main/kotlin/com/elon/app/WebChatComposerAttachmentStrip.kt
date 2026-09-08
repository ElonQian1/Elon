package com.elon.app

import android.os.SystemClock
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.core.content.ContextCompat

internal class WebChatComposerAttachmentStrip(private val host: LinearLayout) {
    private val context = host.context
    private val rows = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(12), dp(6), dp(12), dp(6))
    }
    private val view = HorizontalScrollView(context).apply {
        visibility = View.GONE
        contentDescription = "web-chat-composer-attachments"
        isHorizontalScrollBarEnabled = false
        addView(rows)
    }
    private var owner: WebChatConsumerPort? = null
    private var page = ""
    private var items = emptyList<WebChatComposerAttachment>()
    private var streaming = false
    private var removing: String? = null
    private var poll: Runnable? = null

    init {
        host.addView(view, LinearLayout.LayoutParams(-1, dp(76)))
        view.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
            override fun onViewAttachedToWindow(v: View) {
                render(owner, owner?.state())
                draw()
            }
            override fun onViewDetachedFromWindow(v: View) { stopWatching() }
        })
    }

    fun hide() = render(null, null)

    fun render(port: WebChatConsumerPort?, state: WebChatConsumerState?) {
        val next = WebChatComposerAttachments.visible(state)
        val nextPage = state?.pageUrl.orEmpty()
        if (owner === port && page == nextPage && items == next && streaming == (state?.streaming == true)) return
        if (owner !== port || page != nextPage) stopWatching()
        owner = port
        page = nextPage
        items = next
        streaming = state?.streaming == true
        draw()
    }

    private fun draw() {
        rows.removeAllViews()
        view.visibility = if (owner != null && items.isNotEmpty()) View.VISIBLE else View.GONE
        items.forEach { item ->
            rows.addView(LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                contentDescription = "web-chat-composer-attachment:${item.id}"
                background = ContextCompat.getDrawable(context, R.drawable.bg_chatgpt_attachment_chip)
                setPadding(dp(12), dp(6), 0, dp(6))
                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    addView(TextView(context).apply {
                        text = item.name.ifBlank { "附件" }
                        textSize = 14f
                        maxLines = 1
                        ellipsize = TextUtils.TruncateAt.END
                        setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
                    })
                    addView(TextView(context).apply {
                        text = if (removing == item.id) "正在移除" else WebChatComposerAttachments.status(item.state)
                        textSize = 12f
                        maxLines = 1
                        setTextColor(ContextCompat.getColor(context, R.color.elon_text_secondary))
                    })
                }, LinearLayout.LayoutParams(0, -2, 1f))
                addView(ImageButton(context).apply {
                    contentDescription = "web-chat-composer-attachment-remove:${item.id}"
                    tooltipText = "移除附件"
                    background = null
                    setImageResource(R.drawable.ic_chatgpt_attachment_remove)
                    setColorFilter(ContextCompat.getColor(context, R.color.elon_icon_primary))
                    setPadding(dp(14), dp(14), dp(14), dp(14))
                    isEnabled = item.removable && removing == null && !streaming
                    alpha = if (isEnabled) 1f else 0.4f
                    setOnClickListener { remove(item) }
                }, LinearLayout.LayoutParams(dp(48), dp(48)))
            }, LinearLayout.LayoutParams(dp(216), dp(64)).apply { marginEnd = dp(8) })
        }
    }

    private fun remove(item: WebChatComposerAttachment) {
        val port = owner ?: return
        if (removing != null || !WebChatComposerAttachments.canRemove(page, item, port.state())) return
        val result = port.removeComposerAttachment(item.id)
        if (!result.accepted || result.requestId == null) { failure(); return }
        removing = item.id
        draw()
        val expectedPage = page
        val started = SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (poll !== this) return
                val state = port.state()
                if (owner !== port || state.pageUrl != expectedPage || !state.adapterCurrent) {
                    stopWatching(); render(owner, owner?.state()); return
                }
                val receipt = state.commandRequests.firstOrNull { it.id == result.requestId }
                when {
                    state.attachments.none { it.id == item.id } -> {
                        stopWatching(); items = WebChatComposerAttachments.visible(state); draw()
                    }
                    receipt?.status in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT) ||
                        SystemClock.elapsedRealtime() - started >= 12_000 -> {
                        stopWatching(); draw(); failure()
                    }
                    else -> host.postDelayed(this, 300)
                }
            }
        }
        poll = task
        host.post(task)
    }

    private fun stopWatching() {
        poll?.let(host::removeCallbacks)
        poll = null
        removing = null
    }

    private fun failure() = Toast.makeText(context, "未能确认移除附件，请检查后重试", Toast.LENGTH_SHORT).show()
    private fun dp(value: Int) = (context.resources.displayMetrics.density * value).toInt()
}
