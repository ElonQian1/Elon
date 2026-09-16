package com.elon.app.sociallinks

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView

/** Every descendant handles taps, including when chat adds recursive long-press listeners. */
internal class SocialLinkCardView(context: Context, open: () -> Unit) : LinearLayout(context) {
    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()
    val title = TextView(context).apply {
        tag = "social-link-title"; textSize = 16f; maxLines = 2; ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#F8F7F4")); includeFontPadding = false
    }
    val source = TextView(context).apply {
        tag = "social-link-source"; textSize = 12f; maxLines = 1; ellipsize = TextUtils.TruncateAt.END
        setTextColor(Color.parseColor("#B3DDDBD5")); includeFontPadding = false
    }
    val cover = ImageView(context).apply {
        tag = "social-link-cover"; visibility = View.GONE; scaleType = ImageView.ScaleType.CENTER_CROP
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    init {
        tag = "social-link-card"; orientation = VERTICAL; isFocusable = true
        setPadding(dp(12), dp(12), dp(12), dp(12))
        background = GradientDrawable().apply { setColor(Color.parseColor("#252B33")); cornerRadius = dp(6).toFloat() }
        val footer = LinearLayout(context).apply { orientation = HORIZONTAL; gravity = Gravity.BOTTOM }
        val icon = ImageView(context).apply {
            setImageResource(com.elon.app.R.drawable.ic_social_article)
            imageTintList = android.content.res.ColorStateList.valueOf(Color.parseColor("#B3DDDBD5"))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        val origin = LinearLayout(context).apply { gravity = Gravity.CENTER_VERTICAL }
        origin.addView(icon, LayoutParams(dp(14), dp(14)).apply { marginEnd = dp(4) })
        origin.addView(source, LayoutParams(0, -2, 1f))
        footer.addView(origin, LayoutParams(0, -2, 1f))
        footer.addView(cover, LayoutParams(dp(72), dp(72)).apply { marginStart = dp(12) })
        addView(title, LayoutParams(-1, -2))
        addView(footer, LayoutParams(-1, -2).apply { topMargin = dp(12) })
        // setOnLongClickListener makes even a TextView consume ACTION_UP. All hit targets
        // therefore own the same click; selection mode can still replace these normally.
        listOf(this, title, footer, origin, icon, source, cover).forEach { child -> child.setOnClickListener { open() } }
    }
    fun bind(item: SocialLink) {
        title.text = item.title.ifBlank { if (item.site == "微信公众号") "微信公众号文章" else "${item.site}分享" }
        source.text = item.author.ifBlank { item.site }
        contentDescription = "打开${title.text}（${item.site}）"
    }
}
