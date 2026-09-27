package com.elon.app.chatrecords

import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.ChatMessage
import com.elon.app.articles.ArticleUi
import com.elon.app.R
import com.elon.app.elonColor

internal object ChatRecordCardViews {
    fun bind(container: LinearLayout?, text: TextView, message: ChatMessage): Boolean {
        val card = ChatRecordDocument.card(message.content) ?: return false
        container ?: return false
        text.text = ""; text.visibility = View.GONE; container.removeAllViews(); container.visibility = View.VISIBLE
        val ui = ArticleUi(container.context)
        val body = ui.column().apply {
            layoutParams = LinearLayout.LayoutParams(minOf(ui.dp(288), resources.displayMetrics.widthPixels - ui.dp(100)), -2)
            background = android.graphics.drawable.GradientDrawable().apply { cornerRadius = ui.dp(8).toFloat(); setColor(context.elonColor(R.color.elon_surface_card)) }
            addView(ui.text(card.optString("title"), 17f).apply { maxLines = 2 })
            addView(ui.text(card.optString("summary"), 14f, true).apply { maxLines = 3; ellipsize = android.text.TextUtils.TruncateAt.END })
            addView(ui.text("聊天记录 · ${card.optInt("message_count")} 条", 12f, true))
            isFocusable = true; contentDescription = "查看聊天记录 ${card.optString("title")}"; setOnClickListener { ChatRecordReaderActivity.open(context, card.getString("group_id"), card.getString("record_id")) }
        }
        container.addView(body); return true
    }
}
