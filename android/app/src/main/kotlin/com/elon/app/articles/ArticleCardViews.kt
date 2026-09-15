package com.elon.app.articles

import android.content.Intent
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import com.elon.app.ChatMessage
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.ConcurrentHashMap
import com.elon.app.AuthManager
import org.json.JSONObject

internal object ArticleCardViews {
    private val loader = ThreadPoolExecutor(2, 2, 30, TimeUnit.SECONDS, ArrayBlockingQueue(96), ThreadPoolExecutor.DiscardPolicy())
    private val previews = ConcurrentHashMap<String, Pair<Long, JSONObject>>()
    fun bind(container: LinearLayout?, text: TextView, message: ChatMessage): Boolean {
        val ref = ArticleApi.reference(message.content) ?: return false
        if (container == null) return false
        val id = ref.optString("article_id"); val revision = ref.optLong("revision")
        val context = container.context.applicationContext
        val user = AuthManager.userId(context).orEmpty()
        val key = "$user:$id:$revision"
        val ui = ArticleUi(container.context)
        val host = LinearLayout(container.context).apply { orientation = LinearLayout.VERTICAL }
        fun open() { container.context.startActivity(Intent(container.context, ArticleActivity::class.java).putExtra("article_id", id).putExtra("revision", revision)) }
        val cached = previews[key]?.takeIf { System.currentTimeMillis() - it.first < 60_000 }?.second
        host.addView(ui.card(cached ?: ref, ::open)); container.addView(host); container.visibility = View.VISIBLE; text.visibility = View.GONE
        if (cached != null) return true
        loader.execute {
            if (AuthManager.userId(context).orEmpty() != user) return@execute
            val card = runCatching { ArticleApi(context).read(id, revision, true) }.getOrNull()
            if (card != null && AuthManager.userId(context).orEmpty() == user) {
                if (previews.size >= 160) previews.clear()
                previews[key] = System.currentTimeMillis() to card
                host.post { if (host.parent === container && AuthManager.userId(context).orEmpty() == user) { host.removeAllViews(); host.addView(ui.card(card, ::open)) } }
            }
        }
        return true
    }
    fun openGroup(container: LinearLayout, groupId: String) {
        val old = container.findViewWithTag<View>("article-entry")
        if (old != null) container.removeView(old)
        container.addView(ArticleUi(container.context).button("文章") {
            container.context.startActivity(Intent(container.context, ArticleActivity::class.java).putExtra("group_id", groupId))
        }.apply { tag = "article-entry" })
    }
}
