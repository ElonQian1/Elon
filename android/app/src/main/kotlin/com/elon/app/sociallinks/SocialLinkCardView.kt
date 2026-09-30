package com.elon.app.sociallinks

import android.content.Context
import android.graphics.Color
import android.graphics.Bitmap
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.MobileColors

/** Every descendant handles taps, including when chat adds recursive long-press listeners. */
internal class SocialLinkCardView(context: Context, private val poster: Boolean = false, open: () -> Unit) : LinearLayout(context) {
    private val palette = MobileColors(context)
    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()
    val title = TextView(context).apply {
        tag = "social-link-title"; textSize = 16f; maxLines = 3; ellipsize = TextUtils.TruncateAt.END
        setTextColor(palette.text); includeFontPadding = false
    }
    val source = TextView(context).apply {
        tag = "social-link-source"; textSize = 12f; maxLines = 1; ellipsize = TextUtils.TruncateAt.END
        setTextColor(palette.muted); includeFontPadding = false
    }
    val summary = TextView(context).apply {
        tag = "social-link-summary"; textSize = 13f; maxLines = 2; ellipsize = TextUtils.TruncateAt.END; visibility = View.GONE
        setTextColor(palette.muted); includeFontPadding = false
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
        setTextColor(palette.primary)
    }
    private val media = FrameLayout(context)
    val creatorAvatar = ImageView(context).apply {
        tag = "social-link-avatar"; scaleType = ImageView.ScaleType.CENTER_CROP; visibility = View.GONE
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    private val creatorInitial = TextView(context).apply {
        gravity = Gravity.CENTER; textSize = 13f; setTextColor(palette.onPrimaryContainer)
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    val play = ImageView(context).apply {
        tag = "social-link-play"; setImageResource(android.R.drawable.ic_media_play); setColorFilter(Color.WHITE)
        setPadding(dp(13), dp(13), dp(11), dp(13))
        background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(0x66000000); setStroke(dp(2), Color.WHITE) }
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    private var avatarSource: String? = null
    private var coverSource: String? = null
    private var posterRatio: Float? = null
    private val action = TextView(context).apply { textSize = 12f; setTextColor(palette.muted); includeFontPadding = false }
    init {
        tag = "social-link-card"; orientation = VERTICAL; isFocusable = true
        setPadding(dp(12), dp(12), dp(12), dp(12))
        background = GradientDrawable().apply { setColor(palette.container); cornerRadius = dp(12).toFloat() }
        val headline = LinearLayout(context).apply { orientation = HORIZONTAL; gravity = Gravity.TOP }
        media.addView(badge, FrameLayout.LayoutParams(-1, -1))
        media.addView(cover, FrameLayout.LayoutParams(-1, -1)); media.clipToOutline = true
        if (poster) {
            setPadding(0, 0, 0, 0)
            cover.scaleType = ImageView.ScaleType.CENTER_INSIDE
            clipToOutline = true
            media.addView(play, FrameLayout.LayoutParams(dp(52), dp(52), Gravity.CENTER))
            media.removeView(badge)
            badge.textSize = 11f; badge.setPadding(dp(7), dp(3), dp(7), dp(3))
            badge.background = GradientDrawable().apply { setColor(0x99000000.toInt()); cornerRadius = dp(4).toFloat() }
            media.addView(badge, FrameLayout.LayoutParams(-2, -2, Gravity.TOP or Gravity.START).apply { topMargin = dp(8); marginStart = dp(8) })
            addView(media, LayoutParams(-1, dp(294)))
            title.maxLines = 2; title.textSize = 14f
            title.setPadding(dp(12), dp(10), dp(12), 0)
            addView(title, LayoutParams(-1, -2))
            time.setPadding(dp(12), dp(6), dp(12), 0)
            addView(time, LayoutParams(-1, -2))
            val footer = LinearLayout(context).apply {
                orientation = HORIZONTAL; gravity = Gravity.CENTER_VERTICAL; minimumHeight = dp(58)
                setPadding(dp(10), dp(8), dp(10), dp(8)); setBackgroundColor(palette.elevated)
            }
            val identity = FrameLayout(context).apply {
                background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(palette.primaryContainer) }; clipToOutline = true
                addView(creatorInitial, FrameLayout.LayoutParams(-1, -1)); addView(creatorAvatar, FrameLayout.LayoutParams(-1, -1))
            }
            source.setTextColor(palette.text); source.textSize = 13f
            val author = LinearLayout(context).apply {
                orientation = VERTICAL; addView(source, LayoutParams(-1, -2))
                addView(action, LayoutParams(-1, -2).apply { topMargin = dp(2) })
            }
            footer.addView(identity, LayoutParams(dp(40), dp(40)))
            footer.addView(author, LayoutParams(0, -2, 1f).apply { marginStart = dp(8) })
            addView(footer, LayoutParams(-1, -2))
            listOf(footer, identity, creatorInitial, creatorAvatar, author, action, play).forEach { child -> child.setOnClickListener { open() } }
        } else {
            headline.addView(title, LayoutParams(0, -2, 1f))
            headline.addView(media, LayoutParams(dp(56), dp(56)).apply { marginStart = dp(12) })
            addView(headline, LayoutParams(-1, -2))
            addView(summary, LayoutParams(-1, -2).apply { topMargin = dp(6) })
            addView(source, LayoutParams(-1, -2).apply { topMargin = dp(8) })
            addView(time, LayoutParams(-1, -2).apply { topMargin = dp(4) })
        }
        // setOnLongClickListener makes even a TextView consume ACTION_UP. All hit targets
        // therefore own the same click; selection mode can still replace these normally.
        listOf(this, headline, title, media, badge, summary, source, time, cover).forEach { child -> child.setOnClickListener { open() } }
    }
    fun bind(item: SocialLink) {
        val channels = WechatChannelsPolicy.isChannels(item.url)
        title.text = SocialLinkPresentation.title(item); source.text = SocialLinkPresentation.source(item)
        if (poster) {
            title.visibility = View.VISIBLE
            action.text = if (item.site == "小红书") "笔记" else "视频"
            play.setImageResource(if (item.site == "小红书") android.R.drawable.ic_menu_view else android.R.drawable.ic_media_play)
            creatorInitial.text = item.author.ifBlank { item.site }.let { String(Character.toChars(it.codePointAt(0))) }
            if (avatarSource != item.authorAvatar) { avatarSource = item.authorAvatar; bindAvatar(null) }
            if (coverSource != item.image) { coverSource = item.image; bindCover(null) }
        }
        summary.text = item.summary; summary.visibility = if (item.summary.isBlank()) View.GONE else View.VISIBLE
        badge.text = if (channels) "视频号" else SocialLinkPresentation.badge(item.site)
        val colors = SocialLinkPresentation.colors(item.site)
        badge.setTextColor(if (poster) Color.WHITE else Color.parseColor(colors.second))
        media.background = GradientDrawable().apply { setColor(Color.parseColor(colors.first)); cornerRadius = dp(4).toFloat() }
        time.text = SocialLinkPresentation.time(item); time.visibility = if (time.text.isEmpty()) View.GONE else View.VISIBLE
        contentDescription = "打开${title.text}（${source.text}${if (time.text.isEmpty()) "" else "，${time.text}"}）"
    }
    fun bindCover(bitmap: Bitmap?) {
        cover.setImageBitmap(bitmap); cover.visibility = if (bitmap == null) View.GONE else View.VISIBLE
        posterRatio = bitmap?.let { (it.width.toFloat() / it.height).coerceIn(.5f, 2f) }
        requestLayout()
    }
    fun bindAvatar(bitmap: Bitmap?) {
        creatorAvatar.setImageBitmap(bitmap); creatorAvatar.visibility = if (bitmap == null) View.GONE else View.VISIBLE
        creatorInitial.visibility = if (bitmap == null) View.VISIBLE else View.GONE
    }
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        if (poster) media.layoutParams.height = posterRatio?.let { (MeasureSpec.getSize(widthMeasureSpec) / it).toInt().coerceAtLeast(1) } ?: dp(100)
        super.onMeasure(widthMeasureSpec, heightMeasureSpec)
    }
}
