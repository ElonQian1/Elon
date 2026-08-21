package com.elon.app.chatgptweb

import android.content.Context
import android.content.Intent

internal object ChatGptWebOfficialFallbackIntent {
    internal const val LOGIN_URL = "https://chatgpt.com/auth/login"
    private const val EXTRA_START_URL = "chatgpt_product_start_url"
    private const val EXTRA_STARTUP_ACTION = "chatgpt_product_startup_action"

    fun create(
        context: Context,
        currentUrl: String?,
        startupAction: ChatGptWebOfficialStartupAction? = null,
    ): Intent =
        Intent(context, ChatGptWebOfficialActivity::class.java)
            .apply {
                sanitizeStartUrl(currentUrl)?.let { putExtra(EXTRA_START_URL, it) }
                startupAction?.let { putExtra(EXTRA_STARTUP_ACTION, it.wireValue) }
            }

    fun createLogin(context: Context): Intent = create(context, LOGIN_URL)

    fun startUrl(intent: Intent): String? =
        sanitizeStartUrl(intent.getStringExtra(EXTRA_START_URL))

    fun startupAction(intent: Intent): ChatGptWebOfficialStartupAction? =
        ChatGptWebOfficialStartupAction.fromWireValue(
            intent.getStringExtra(EXTRA_STARTUP_ACTION),
        )

    fun sanitizeStartUrl(rawUrl: String?): String? = rawUrl
        ?.trim()
        ?.takeIf(String::isNotEmpty)
        ?.takeIf(ChatGptWebNavigationPolicy::allows)
}
