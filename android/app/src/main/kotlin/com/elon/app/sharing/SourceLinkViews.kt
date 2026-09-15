package com.elon.app.sharing

import android.content.Context
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import com.elon.app.ChatAttachment
import com.elon.app.ChatImagePreviewLoader
import com.elon.app.ChatMessage
import com.elon.app.chatAttachmentImageSource
import com.elon.app.articles.ArticleUi
import java.util.concurrent.Executors

internal object SourceLinkViews {
    private val scanner = Executors.newSingleThreadExecutor()
    fun wrapImage(context: Context, attachment: ChatAttachment, image: View): View {
        val ui = ArticleUi(context)
        val host = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            layoutParams = LinearLayout.LayoutParams(-2, -2)
        }
        host.addView(image)
        attachment.sourceLink?.takeIf { SourceLink.webUrl(it.url) != null }?.let { link ->
            host.addView(ui.button(link.label) { ArticleLinkActivity.open(context, link.url) })
        }
        image.setOnLongClickListener {
            AlertDialog.Builder(context).setItems(arrayOf("识别二维码")) { _, _ ->
                val source = chatAttachmentImageSource(attachment)
                if (source == null) Toast.makeText(context, "图片暂不可用，请重新加载", Toast.LENGTH_SHORT).show()
                else ChatImagePreviewLoader.load(context, source) { bitmap ->
                    scanner.execute {
                        val links = bitmap?.let(ImageQrLinks::decode).orEmpty()
                        image.post { choose(context, links) { ArticleLinkActivity.open(context, it.url) } }
                    }
                }
            }.show(); true
        }
        return host
    }
    fun choose(context: Context, links: List<SourceLink>, chosen: (SourceLink) -> Unit) {
        if (links.isEmpty()) { Toast.makeText(context, "未识别到网页二维码，可尝试原图或复制链接", Toast.LENGTH_LONG).show(); return }
        AlertDialog.Builder(context).setTitle("选择二维码链接")
            .setItems(links.map { it.url }.toTypedArray()) { _, index -> chosen(links[index]) }
            .setNegativeButton("取消", null).show()
    }
    fun bindCard(container: LinearLayout?, text: TextView, message: ChatMessage): Boolean {
        if (com.elon.app.sociallinks.SocialLinkPolicy.extract(message.content).isNotEmpty()) return false
        if (container == null || !message.attachments.isNullOrEmpty()) return false
        val link = SourceLink.fromText(message.content) ?: return false
        val ui = ArticleUi(container.context)
        container.addView(ui.button("${link.label}\n${java.net.URI(link.url).host}") { ArticleLinkActivity.open(container.context, link.url) })
        container.visibility = View.VISIBLE
        text.visibility = View.GONE
        return true
    }
}
