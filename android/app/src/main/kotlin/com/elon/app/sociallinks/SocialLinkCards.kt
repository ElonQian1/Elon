package com.elon.app.sociallinks

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.Spannable
import android.text.SpannableString
import android.text.style.URLSpan
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.AuthManager
import com.elon.app.ChatMessage

internal object SocialLinkCards {
    fun bind(container: LinearLayout?, text: TextView, message: ChatMessage, enabled: Boolean) {
        if (!enabled || container == null || message.role !in listOf("user", "friend") || message.webChatMessage != null) return
        val items = SocialLinkPolicy.extract(message.content)
        if (items.isEmpty()) return
        // Original links and cards both enter the same isolated reader; message text is unchanged.
        val spans = SpannableString(text.text)
        spans.getSpans(0, spans.length, URLSpan::class.java).forEach { span ->
            val item = SocialLinkPolicy.link(span.url) ?: return@forEach
            val start = spans.getSpanStart(span); val end = spans.getSpanEnd(span)
            spans.removeSpan(span)
            spans.setSpan(object : URLSpan(span.url) { override fun onClick(widget: View) { SocialLinkBrowserActivity.open(widget.context, item) } }, start, end, Spannable.SPAN_EXCLUSIVE_EXCLUSIVE)
        }
        text.text = spans
        val context = container.context; val app = context.applicationContext
        val owner = AuthManager.userId(app)
        fun dp(n: Int) = (n * context.resources.displayMetrics.density).toInt()
        for (item in items) {
            var current = item; var busy = false
            val host = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
            val card = LinearLayout(context).apply {
                orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL; minimumHeight = dp(104)
                setPadding(dp(14), dp(14), dp(14), dp(14)); isFocusable = true
                background = GradientDrawable().apply { setColor(Color.parseColor("#272A30")); cornerRadius = dp(12).toFloat(); setStroke(dp(1), Color.parseColor("#3E444C")) }
            }
            val copy = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
            val title = TextView(context).apply { textSize = 15f; setTextColor(Color.parseColor("#F1F2F5")); maxLines = 3; ellipsize = android.text.TextUtils.TruncateAt.END; setTypeface(typeface, Typeface.NORMAL) }
            val source = TextView(context).apply { textSize = 12f; setTextColor(Color.parseColor("#B7BDC8")); setPadding(0, dp(12), 0, 0) }
            val cover = ImageView(context).apply { visibility = View.GONE; scaleType = ImageView.ScaleType.CENTER_CROP; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }
            val retry = TextView(context).apply { this.text = "更新预览"; textSize = 12f; setTextColor(Color.parseColor("#B5CCEA")); setPadding(dp(4), dp(8), dp(4), dp(8)); visibility = View.GONE; isFocusable = true }
            copy.addView(title); copy.addView(source); card.addView(copy, LinearLayout.LayoutParams(0, -2, 1f))
            card.addView(cover, LinearLayout.LayoutParams(dp(68), dp(68)).apply { marginStart = dp(12) })
            host.addView(card, LinearLayout.LayoutParams(-1, -2)); host.addView(retry)
            container.addView(host, LinearLayout.LayoutParams(dp(330).coerceAtMost(context.resources.displayMetrics.widthPixels - dp(112)), -2).apply { topMargin = dp(8) }); container.visibility = View.VISIBLE
            fun valid() = host.parent === container && AuthManager.userId(app) == owner
            fun draw(value: SocialLink) {
                current = value; title.text = value.title.ifBlank { "${value.site}分享" }
                source.text = value.site + if (value.author.isNotBlank()) " · ${value.author}" else ""
                card.contentDescription = "打开${title.text}（${value.site}）"
            }
            fun load(refresh: Boolean) {
                if (busy || !valid()) return
                busy = true; retry.isEnabled = false
                SocialLinkPreviewApi.loader.execute {
                    if (!valid()) return@execute
                    val result = SocialLinkPreviewApi.load(app, item, refresh)
                    host.post { busy = false; if (valid()) { draw(result); retry.isEnabled = true; retry.visibility = if (result.ready) View.GONE else View.VISIBLE } }
                    val bitmap = result.image?.let { SocialLinkPreviewApi.cover(app, it) }
                    if (bitmap != null) host.post { if (valid()) { cover.setImageBitmap(bitmap); cover.visibility = View.VISIBLE } }
                }
            }
            card.setOnClickListener { SocialLinkBrowserActivity.open(context, current) }
            card.setOnLongClickListener { text.performLongClick() }
            retry.setOnClickListener { load(true) }; draw(item)
            host.addOnAttachStateChangeListener(object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) { load(false) }
                override fun onViewDetachedFromWindow(v: View) = Unit
            })
            if (host.isAttachedToWindow) load(false)
        }
    }
}
