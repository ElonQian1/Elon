package com.elon.app.articles

import android.content.Context
import android.graphics.BitmapFactory
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.Base64
import android.view.View
import android.widget.Button
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.R
import com.elon.app.elonColor
import org.json.JSONObject

internal class ArticleUi(val context: Context) {
    fun dp(n: Int) = (n * context.resources.displayMetrics.density).toInt()
    fun column() = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(18), dp(12), dp(18), dp(20)) }
    fun row() = LinearLayout(context).apply { orientation = LinearLayout.HORIZONTAL; gravity = android.view.Gravity.CENTER_VERTICAL }
    fun text(value: String, size: Float = 17f, quiet: Boolean = false) = TextView(context).apply {
        text = value; textSize = size
        setTextColor(context.elonColor(if (quiet) R.color.elon_text_secondary else R.color.elon_text_primary))
        setPadding(0, dp(10), 0, dp(10)); setLineSpacing(dp(4).toFloat(), 1.12f)
    }
    fun button(value: String, action: () -> Unit) = Button(context).apply { text = value; isAllCaps = false; minHeight = dp(44); setOnClickListener { action() } }
    fun image(data: String, cover: Boolean = false): View {
        val bytes = runCatching { Base64.decode(data.substringAfter(','), Base64.DEFAULT) }.getOrNull()
        val bitmap = bytes?.let {
            val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeByteArray(it, 0, it.size, options)
            var sample = 1
            while (maxOf(options.outWidth, options.outHeight) / sample > 1200) sample *= 2
            BitmapFactory.decodeByteArray(it, 0, it.size, BitmapFactory.Options().apply { inSampleSize = sample })
        }
        return ImageView(context).apply {
            setImageBitmap(bitmap); contentDescription = if (cover) "文章封面" else "正文图片"
            adjustViewBounds = true; scaleType = if (cover) ImageView.ScaleType.CENTER_CROP else ImageView.ScaleType.FIT_CENTER
            layoutParams = LinearLayout.LayoutParams(-1, if (cover) dp(190) else -2)
        }
    }
    fun card(value: JSONObject, click: () -> Unit): View = column().apply {
        background = GradientDrawable().apply { setColor(context.elonColor(R.color.elon_surface_card)); cornerRadius = dp(12).toFloat() }
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) }
        value.optString("cover_data_url").takeIf { it.startsWith("data:image/") }?.let { addView(image(it, true)) }
        addView(text("文章 · ${value.optString("author_name", "")}", 12f, true))
        addView(text(value.optString("title").ifBlank { "未命名文章" }, 20f).apply { typeface = Typeface.DEFAULT_BOLD; maxLines = 3 })
        if (value.optString("summary").isNotBlank()) addView(text(value.optString("summary"), 14f, true).apply { maxLines = 3 })
        addView(text(if (value.optString("status") == "withdrawn") "已撤下" else "阅读全文 ›", 13f, true))
        isClickable = true; isFocusable = true; contentDescription = "阅读文章：${value.optString("title")}"; setOnClickListener { click() }
    }
    fun body(a: JSONObject): View = column().apply {
        val document = a.optJSONObject("document") ?: JSONObject(); val media = a.optJSONObject("media") ?: JSONObject()
        media.optString(document.optString("cover")).takeIf { it.isNotBlank() }?.let { addView(image(it, true)) }
        addView(text(document.optString("title").ifBlank { "未命名文章" }, 28f).apply { typeface = Typeface.DEFAULT_BOLD })
        addView(text("${a.optString("author_name")} · ${a.optString("updated_at").take(10)}", 13f, true))
        if (document.optString("summary").isNotBlank()) addView(text(document.optString("summary"), 16f, true))
        val blocks = document.optJSONArray("blocks")
        for (i in 0 until (blocks?.length() ?: 0)) {
            val b = blocks!!.getJSONObject(i)
            when (b.optString("type")) {
                "image" -> { addView(image(media.optString(b.optString("media_id")))); if (b.optString("caption").isNotBlank()) addView(text(b.optString("caption"), 13f, true)) }
                "heading" -> addView(text(b.optString("text"), 22f).apply { typeface = Typeface.DEFAULT_BOLD })
                "quote" -> addView(text(b.optString("text"), 18f, true).apply { setPadding(dp(16), dp(12), dp(12), dp(12)); setBackgroundColor(context.elonColor(R.color.elon_surface_card)) })
                else -> addView(text(b.optString("text"), 19f).apply { setTextIsSelectable(true) })
            }
        }
    }
}
