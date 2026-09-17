package com.elon.app.sociallinks

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

/** Every descendant handles taps, including when chat adds recursive long-press listeners. */
internal class SocialLinkCardView(context: Context, open: () -> Unit) : LinearLayout(context) {
    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()
    val title = TextView(context).apply {
        tag = "social-link-title"; textSize = 16f; maxLines = 3; ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#F8F7F4")); includeFontPadding = false
    }
    val source = TextView(context).apply {
        tag = "social-link-source"; textSize = 12f; maxLines = 1; ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#B7BDC8")); includeFontPadding = false
    }
    val summary = TextView(context).apply {
        tag = "social-link-summary"; textSize = 13f; maxLines = 2; ellipsize = TextUtils.TruncateAt.END; visibility = View.GONE
        setTextColor(Color.parseColor("#C9CED8")); includeFontPadding = false
    }
    val cover = ImageView(context).apply {
        tag = "social-link-cover"; visibility = View.GONE; scaleType = ImageView.ScaleType.CENTER_CROP
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    val badge = TextView(context).apply {
        tag = "social-link-badge"; textSize = 16f; gravity = Gravity.CENTER
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    val time = TextView(context).apply {
        tag = "social-link-time"; textSize = 12f; includeFontPadding = false
        setTextColor(Color.parseColor("#B5CCEA"))
    }
    private val media = FrameLayout(context)
    init {
        tag = "social-link-card"; orientation = VERTICAL; isFocusable = true
        setPadding(dp(12), dp(12), dp(12), dp(12))
        background = GradientDrawable().apply { setColor(Color.parseColor("#252B33")); cornerRadius = dp(6).toFloat() }
        val headline = LinearLayout(context).apply { orientation = HORIZONTAL; gravity = Gravity.TOP }
        media.addView(badge, FrameLayout.LayoutParams(-1, -1))
        media.addView(cover, FrameLayout.LayoutParams(-1, -1)); media.clipToOutline = true
        headline.addView(title, LayoutParams(0, -2, 1f))
        headline.addView(media, LayoutParams(dp(56), dp(56)).apply { marginStart = dp(12) })
        addView(headline, LayoutParams(-1, -2))
        addView(summary, LayoutParams(-1, -2).apply { topMargin = dp(6) })
        addView(source, LayoutParams(-1, -2).apply { topMargin = dp(8) })
        addView(time, LayoutParams(-1, -2).apply { topMargin = dp(4) })
        // setOnLongClickListener makes even a TextView consume ACTION_UP. All hit targets
        // therefore own the same click; selection mode can still replace these normally.
        listOf(this, headline, title, media, badge, summary, source, time, cover).forEach { child -> child.setOnClickListener { open() } }
    }
    fun bind(item: SocialLink) {
        title.text = SocialLinkPresentation.title(item); source.text = SocialLinkPresentation.source(item)
        summary.text = item.summary; summary.visibility = if (item.summary.isBlank()) View.GONE else View.VISIBLE
        badge.text = SocialLinkPresentation.badge(item.site)
        val colors = SocialLinkPresentation.colors(item.site)
        badge.setTextColor(Color.parseColor(colors.second))
        media.background = GradientDrawable().apply { setColor(Color.parseColor(colors.first)); cornerRadius = dp(4).toFloat() }
        time.text = SocialLinkPresentation.time(item); time.visibility = if (time.text.isEmpty()) View.GONE else View.VISIBLE
        contentDescription = "打开${title.text}（${source.text}${if (time.text.isEmpty()) "" else "，${time.text}"}）"
    }
}
