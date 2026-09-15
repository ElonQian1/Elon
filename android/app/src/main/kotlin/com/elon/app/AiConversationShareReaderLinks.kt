package com.elon.app

import android.content.Intent
import android.net.Uri
import android.text.SpannableString
import android.text.Spanned
import android.text.method.LinkMovementMethod
import android.text.style.ClickableSpan
import android.text.style.URLSpan
import android.view.View
import android.widget.TextView
import java.net.URI
import java.util.Locale

internal object AiConversationShareReaderLinks {
    fun publicUrl(value: String): String? {
        if (value.length > 4096 || value.any { it.isWhitespace() || it.code < 32 } || '\\' in value) return null
        val uri = runCatching { URI(value) }.getOrNull() ?: return null
        val scheme = uri.scheme?.lowercase(Locale.ROOT) ?: return null
        if (scheme !in setOf("http", "https") || uri.rawUserInfo != null ||
            uri.rawQuery != null || uri.rawFragment != null || uri.port !in setOf(-1, 80, 443)) return null
        val host = uri.host?.lowercase(Locale.ROOT)?.trimEnd('.') ?: return null
        if ('.' !in host || host.contains(':') || host.last().isDigit() ||
            !host.matches(Regex("[a-z0-9.-]+"))) return null
        if (PRIVATE_SUFFIXES.any { host == it || host.endsWith(".$it") }) return null
        val path = uri.path.orEmpty().lowercase(Locale.ROOT)
        if (path.any { it.code < 32 } || PRIVATE_PATHS.any(path::contains)) return null
        return value
    }

    fun bind(view: TextView) {
        val source = view.text as? Spanned ?: run { view.movementMethod = null; return }
        val text = SpannableString(source)
        var hasLinks = false
        text.getSpans(0, text.length, ClickableSpan::class.java).forEach { span ->
            val start = text.getSpanStart(span)
            val end = text.getSpanEnd(span)
            val safe = (span as? URLSpan)?.url?.let(::publicUrl)
            text.removeSpan(span)
            if (safe != null && start >= 0 && end > start) {
                text.setSpan(object : ClickableSpan() {
                    override fun onClick(widget: View) {
                        val url = publicUrl(safe) ?: return
                        runCatching { widget.context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))
                            .addCategory(Intent.CATEGORY_BROWSABLE)) }
                    }
                }, start, end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
                hasLinks = true
            }
        }
        view.text = text
        view.movementMethod = if (hasLinks) LinkMovementMethod.getInstance() else null
    }

    private val PRIVATE_SUFFIXES = setOf(
        "localhost", "local", "internal", "test", "invalid", "home", "lan",
        "chatgpt.com", "chat.openai.com", "oaistatic.com", "oaiusercontent.com", "openaiusercontent.com",
        "googleusercontent.com", "gemini.google.com", "aistudio.google.com", "claude.ai",
        "blob.core.windows.net",
    )
    private val PRIVATE_PATHS = setOf(
        "/backend-api", "/api/auth", "/api/assets", "/api/files", "/api/media", "/api/download",
        "access_token=", "api_key=", "signature=", "sig=", "token=", "x-amz-", "x-goog-",
    )
}
