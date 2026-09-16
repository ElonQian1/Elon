package com.elon.app.sociallinks

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.text.Spannable
import android.text.SpannableString
import android.text.style.URLSpan
import android.view.View
import android.view.ViewTreeObserver
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.AuthManager
import com.elon.app.ChatMessage
import com.elon.app.ServerUrlManager

internal object SocialLinkCards {
    fun bind(container: LinearLayout?, text: TextView, message: ChatMessage, enabled: Boolean, bubble: LinearLayout? = null) {
        if (!enabled || container == null || message.role !in listOf("user", "friend") || message.webChatMessage != null) return
        val items = SocialLinkPolicy.extract(message.content)
        if (items.isEmpty()) return
        val compact = message.attachments.isNullOrEmpty() && SocialLinkShareText.compact(message.content, items)
        if (compact) {
            text.visibility = View.GONE
            bubble?.background = ColorDrawable(Color.TRANSPARENT); bubble?.setPadding(0, 0, 0, 0)
        }
        val spans = SpannableString(text.text)
        spans.getSpans(0, spans.length, URLSpan::class.java).forEach { span ->
            val item = SocialLinkPolicy.link(span.url) ?: return@forEach
            val start = spans.getSpanStart(span); val end = spans.getSpanEnd(span)
            spans.removeSpan(span)
            spans.setSpan(object : URLSpan(span.url) { override fun onClick(widget: View) { SocialLinkBrowserActivity.open(widget.context, item) } }, start, end, Spannable.SPAN_EXCLUSIVE_EXCLUSIVE)
        }
        text.text = spans
        val context = container.context; val app = context.applicationContext
        val owner = AuthManager.userId(app); val server = ServerUrlManager.getActive(app)
        fun dp(n: Int) = (n * context.resources.displayMetrics.density).toInt()
        for (item in items) {
            var current = item; var busy = false
            val host = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
            val card = SocialLinkCardView(context) { SocialLinkBrowserActivity.open(context, current) }
            val retry = TextView(context).apply {
                this.text = "更新预览"; textSize = 12f; minHeight = dp(48)
                setTextColor(Color.parseColor("#B4C5E3")); setPadding(dp(4), dp(8), dp(4), dp(8)); visibility = View.GONE
            }
            host.addView(card, LinearLayout.LayoutParams(-1, -2)); host.addView(retry)
            val width = minOf(dp(280), (context.resources.displayMetrics.widthPixels - dp(if (compact) 104 else 128)).coerceAtLeast(1))
            container.addView(host, LinearLayout.LayoutParams(width, -2).apply { topMargin = if (compact) 0 else dp(8) }); container.visibility = View.VISIBLE
            fun valid() = host.parent === container && AuthManager.userId(app) == owner && ServerUrlManager.getActive(app) == server
            fun draw(value: SocialLink) { current = value; card.bind(value) }
            fun load(refresh: Boolean) {
                if (busy || !valid()) return
                busy = true; retry.isEnabled = false
                SocialLinkPreviewApi.loader.execute {
                    if (!valid()) { host.post { busy = false }; return@execute }
                    val result = SocialLinkPreviewApi.load(app, item, refresh)
                    host.post { busy = false; if (valid()) { draw(result); retry.isEnabled = true; retry.visibility = if (result.ready || compact) View.GONE else View.VISIBLE } }
                    val bitmap = result.image?.let { SocialLinkPreviewApi.cover(app, it) }
                    host.post { if (valid() && current.image == result.image) { card.cover.setImageBitmap(bitmap); card.cover.visibility = if (bitmap == null) View.GONE else View.VISIBLE } }
                }
            }
            // Chat owns long-press and multi-select for every card descendant.
            retry.setOnClickListener { load(true) }; draw(item)
            val focus = ViewTreeObserver.OnWindowFocusChangeListener { hasFocus -> if (hasFocus) load(false) }
            host.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) { v.viewTreeObserver.addOnWindowFocusChangeListener(focus); load(false) }
                override fun onViewDetachedFromWindow(v: View) { v.viewTreeObserver.removeOnWindowFocusChangeListener(focus) }
            })
            if (host.isAttachedToWindow) { host.viewTreeObserver.addOnWindowFocusChangeListener(focus); load(false) }
        }
    }
}
