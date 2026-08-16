package com.elon.app.chatgptweb

import android.content.Context
import android.content.Intent

internal object ChatGptWebOfficialFallbackIntent {
    private const val EXTRA_PRODUCT_ENTRY = "chatgpt_product_entry"
    private const val EXTRA_START_URL = "chatgpt_product_start_url"

    fun create(context: Context, currentUrl: String?): Intent =
        Intent(context, ChatGptWebTestActivity::class.java)
            .putExtra(EXTRA_PRODUCT_ENTRY, true)
            .apply {
                sanitizeStartUrl(currentUrl)?.let { putExtra(EXTRA_START_URL, it) }
            }

    fun isProductEntry(intent: Intent): Boolean =
        intent.getBooleanExtra(EXTRA_PRODUCT_ENTRY, false)

    fun startUrl(intent: Intent): String? =
        sanitizeStartUrl(intent.getStringExtra(EXTRA_START_URL))

    fun sanitizeStartUrl(rawUrl: String?): String? = rawUrl
        ?.trim()
        ?.takeIf(String::isNotEmpty)
        ?.takeIf(ChatGptWebNavigationPolicy::allows)
}
