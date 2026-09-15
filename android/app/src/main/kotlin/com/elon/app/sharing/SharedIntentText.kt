package com.elon.app.sharing

import android.content.Intent
import android.text.Html
import android.text.style.URLSpan

internal object SharedIntentText {
    fun read(intent: Intent): String {
        val plain = intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString().orEmpty()
        if (plain.isNotBlank()) return plain
        val html = intent.getStringExtra(Intent.EXTRA_HTML_TEXT).orEmpty()
        require(html.length <= 100_000) { "分享内容过长，请复制文章链接" }
        if (html.isNotBlank()) {
            val parsed = Html.fromHtml(html, Html.FROM_HTML_MODE_LEGACY)
            val urls = parsed.getSpans(0, parsed.length, URLSpan::class.java).mapNotNull { SourceLink.webUrl(it.url) }.distinct()
            return if (urls.size == 1) urls[0] else parsed.toString()
        }
        return intent.clipData?.let { clip -> (0 until clip.itemCount.coerceAtMost(6)).mapNotNull { clip.getItemAt(it).text?.toString() }.distinct().joinToString("\n") }.orEmpty()
    }
}
