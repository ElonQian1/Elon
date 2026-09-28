package com.elon.app.chatrecords

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import com.elon.app.ChatMessage
import com.elon.app.MobileColors
import com.elon.app.articles.ArticleUi
import com.elon.app.sociallinks.SocialLinkCards
import com.elon.app.sociallinks.SocialLinkPolicy

internal class ChatRecordRowView(private val ui: ArticleUi, private val model: ChatRecordReaderModel, private val open: (RecordRow) -> Unit, private val move: (String) -> Unit) {
    fun bind(host: LinearLayout, row: RecordRow) {
        host.removeAllViews(); host.tag = row.id
        val line = ui.row().apply { gravity = Gravity.TOP }
        val avatar = ui.text(ChatRecordPresentation.initial(row.sender), 18f).apply {
            gravity = Gravity.CENTER; setPadding(0, 0, 0, 0); setTextColor(Color.WHITE)
            background = GradientDrawable().apply { setColor(Color.parseColor(ChatRecordPresentation.color(row.sender))); cornerRadius = ui.dp(6).toFloat() }
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        line.addView(avatar, LinearLayout.LayoutParams(ui.dp(40), ui.dp(40)))
        val body = LinearLayout(ui.context).apply { orientation = LinearLayout.VERTICAL }
        line.addView(body, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = ui.dp(12) }); host.addView(line)
        body.addView(ui.text(row.sender.ifBlank { "未知发送者" }, 14f, true).apply { setPadding(0, 0, 0, 0) })
        if (row.time.isNotBlank()) body.addView(ui.text(row.time, 12f, true).apply { setPadding(0, ui.dp(2), 0, ui.dp(8)) })
        if (row.kind == "forward") {
            body.addView(ui.button("聊天记录 · ${model.document?.children(row.id)?.size ?: 0} 条") { move(row.id) }); return
        }
        val previews = SocialLinkPolicy.extract(row.text).associate { it.url to ChatRecordPresentation.Card(it.url, it.title, it.site) }.toMutableMap()
        val text = ui.text("", 16f).apply { setTextIsSelectable(true) }; body.addView(text)
        fun refreshText() {
            text.text = ChatRecordPresentation.text(row, previews.values.toList())
            text.visibility = if (text.text.isBlank()) View.GONE else View.VISIBLE
            android.text.util.Linkify.addLinks(text, android.text.util.Linkify.WEB_URLS)
        }
        refreshText()
        val links = LinearLayout(ui.context).apply { orientation = LinearLayout.VERTICAL }; body.addView(links)
        SocialLinkCards.bind(links, text, ChatMessage(role = "friend", content = row.text), true,
            readerActions = ChatRecordLinkActions::bind, onPreview = { if (host.tag == row.id) { previews[it.url] = ChatRecordPresentation.Card(it.url, it.title, it.site); refreshText() } })
        if (row.assetId == null) {
            if (row.filename.isNotBlank()) body.addView(ui.text("导出包未提供可用附件 · ${row.filename}", 13f, true))
        } else when (row.kind) {
            "image" -> {
                val image = ImageView(ui.context).apply { adjustViewBounds = true; contentDescription = row.filename; scaleType = ImageView.ScaleType.FIT_CENTER }
                body.addView(image, LinearLayout.LayoutParams(-1, ui.dp(220)))
                model.file(row) { result -> if (host.tag == row.id) result.onSuccess { f ->
                    com.elon.app.ChatImagePreviewLoader.load(ui.context, f.path) { bitmap -> image.post { if (host.tag == row.id) image.setImageBitmap(bitmap) } }
                }.onFailure { image.contentDescription = "图片加载失败，点击重试" } }
                image.setOnClickListener { open(row) }
            }
            "video" -> video(host, body, row)
            else -> body.addView(ui.button(row.filename.ifBlank { "打开附件" }) { open(row) })
        }
    }
    private fun video(host: LinearLayout, body: LinearLayout, row: RecordRow) {
        val palette = MobileColors(ui.context)
        val frame = FrameLayout(ui.context).apply {
            background = GradientDrawable().apply { setColor(palette.container); cornerRadius = ui.dp(8).toFloat() }; clipToOutline = true
            contentDescription = "播放视频：${row.filename}"; isFocusable = true; setOnClickListener { open(row) }
        }
        val image = ImageView(ui.context).apply { scaleType = ImageView.ScaleType.FIT_CENTER; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO }
        frame.addView(image, FrameLayout.LayoutParams(-1, -1))
        val play = ImageView(ui.context).apply {
            setImageResource(android.R.drawable.ic_media_play); setColorFilter(Color.WHITE); setPadding(ui.dp(10), ui.dp(10), ui.dp(10), ui.dp(10))
            background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(0x88000000.toInt()); setStroke(ui.dp(2), Color.WHITE) }
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        frame.addView(play, FrameLayout.LayoutParams(ui.dp(52), ui.dp(52), Gravity.CENTER))
        val duration = ui.text("", 12f).apply { setTextColor(Color.WHITE); setPadding(ui.dp(6), ui.dp(2), ui.dp(6), ui.dp(2)); setBackgroundColor(0x99000000.toInt()); visibility = View.GONE }
        frame.addView(duration, FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM or Gravity.END).apply { bottomMargin = ui.dp(8); marginEnd = ui.dp(8) })
        body.addView(frame, LinearLayout.LayoutParams(-1, ui.dp(240)))
        body.addView(ui.text(row.filename, 12f, true))
        model.poster(row) { result -> if (host.tag == row.id) result.onSuccess {
            image.setImageBitmap(it.bitmap); duration.text = ChatRecordPresentation.duration(it.duration); duration.visibility = if (duration.text.isBlank()) View.GONE else View.VISIBLE
        }.onFailure { frame.contentDescription = "缩略图暂不可用，点击播放：${row.filename}" } }
    }
}
