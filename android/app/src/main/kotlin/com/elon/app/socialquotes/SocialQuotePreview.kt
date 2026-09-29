package com.elon.app.socialquotes

import android.content.Context
import android.content.res.ColorStateList
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.AuthManager
import com.elon.app.R
import com.elon.app.ServerUrlManager
import com.elon.app.sociallinks.SocialLinkPolicy
import com.elon.app.sociallinks.SocialLinkPreviewApi
import com.elon.app.sociallinks.SocialLinkShareText

/** Shared compact preview: never renders the quoted message as another full bubble. */
internal class SocialQuotePreview(context: Context) : LinearLayout(context) {
    private val muted = ContextCompat.getColor(context, R.color.elon_text_secondary)
    private val body = LinearLayout(context).apply { gravity = Gravity.CENTER_VERTICAL; orientation = HORIZONTAL; minimumHeight = dp(48) }
    private val label = TextView(context).apply {
        textSize = 13f; setTextColor(muted); maxLines = 2; ellipsize = TextUtils.TruncateAt.END
        includeFontPadding = false; setPadding(dp(8), dp(4), dp(8), dp(4))
    }
    private val image = ImageView(context).apply { scaleType = ImageView.ScaleType.CENTER_CROP; importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO }
    private val rule = View(context).apply { setBackgroundColor(muted); alpha = .45f }
    private val close = ImageButton(context).apply {
        setImageResource(android.R.drawable.ic_menu_close_clear_cancel); imageTintList = ColorStateList.valueOf(muted)
        background = null; setPadding(dp(14), dp(14), dp(14), dp(14)); contentDescription = "取消引用"
    }
    private var generation = 0
    private var loadPreview: (() -> Unit)? = null

    init {
        orientation = HORIZONTAL; gravity = Gravity.CENTER_VERTICAL
        addView(body, LayoutParams(0, -2, 1f)); addView(close, LayoutParams(dp(48), dp(48)))
    }

    fun bind(quote: SocialQuote, own: Boolean = false, cancel: (() -> Unit)? = null, open: (() -> Unit)? = null) {
        val version = ++generation
        body.removeAllViews()
        if (!own) body.addView(rule, LayoutParams(dp(2), dp(32)))
        body.addView(label, LayoutParams(0, -2, 1f))
        body.addView(image, LayoutParams(dp(40), dp(40)).apply { marginEnd = dp(8) })
        if (own) body.addView(rule, LayoutParams(dp(2), dp(32)))
        val summary = SocialQuoteCodec.summary(quote)
        fun title(value: String) { label.text = if (quote.senderName.isBlank()) value else "${quote.senderName}: $value" }
        title(summary); image.visibility = GONE; image.setImageDrawable(null)
        body.contentDescription = "引用 ${label.text}"
        body.isFocusable = open != null; body.isClickable = open != null
        body.setOnClickListener { open?.invoke() }
        close.visibility = if (cancel == null) GONE else VISIBLE
        close.setOnClickListener { cancel?.invoke() }
        val link = if (quote.unavailable) null else SocialLinkPolicy.extract(summary).firstOrNull()
        val compact = link != null && SocialLinkShareText.compact(summary, listOf(link))
        if (compact) title(link!!.title.ifBlank { "[链接] ${link.site}" })
        val cover = if (quote.unavailable) null else quote.attachments.firstOrNull { it.isImage() }?.url
        val owner = AuthManager.userId(context); val server = ServerUrlManager.getActive(context)
        var loaded = false
        loadPreview = {
            if (!loaded && (link != null || cover != null)) {
                loaded = true
                SocialLinkPreviewApi.loader.execute {
                    val preview = link?.let { SocialLinkPreviewApi.load(context.applicationContext, it) }
                    val url = cover?.let { if (it.startsWith("/")) server.trimEnd('/') + it else it } ?: preview?.image
                    val bitmap = url?.let { SocialLinkPreviewApi.cover(context.applicationContext, it) }
                    post {
                        if (version == generation && owner == AuthManager.userId(context) && server == ServerUrlManager.getActive(context)) {
                            if (compact && preview != null && preview.title.isNotBlank()) title(preview.title)
                            image.setImageBitmap(bitmap); image.visibility = if (bitmap == null) GONE else VISIBLE
                            body.contentDescription = "引用 ${label.text}"
                        }
                    }
                }
            }
        }
        if (isAttachedToWindow) loadPreview?.invoke()
    }

    override fun onAttachedToWindow() { super.onAttachedToWindow(); loadPreview?.invoke() }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).toInt()
}
