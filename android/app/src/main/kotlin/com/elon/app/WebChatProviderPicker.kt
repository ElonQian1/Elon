package com.elon.app

import androidx.appcompat.app.AppCompatActivity

internal class WebChatProviderPicker(
    private val activity: AppCompatActivity,
    private val currentProvider: () -> WebChatProviderId,
    private val currentModel: () -> String,
    private val currentState: () -> String,
    private val authenticated: () -> Boolean,
    private val composerReady: () -> Boolean,
    private val selectProvider: (WebChatProviderId) -> Boolean,
    private val requestModelOptions: () -> Unit,
    private val openOfficialFallback: () -> Unit,
) {
    fun show() {
        val selectedProvider = currentProvider()
        val options = webChatProviderPickerOptions(
            providers = WebChatProviderRegistry.available(),
            selectedProvider = selectedProvider,
            currentModel = currentModel(),
            currentState = currentState(),
            authenticated = authenticated(),
            composerReady = composerReady(),
        )
        WebChatProviderPickerSheet.show(
            activity = activity,
            options = options,
            onProviderSelected = selectProvider,
            onModelOptions = requestModelOptions,
            onOfficialPage = openOfficialFallback,
        )
    }
}

internal data class WebChatProviderPickerOption(
    val providerId: WebChatProviderId,
    val title: String,
    val subtitle: String,
    val avatarResId: Int,
    val selected: Boolean,
) {
    val label: String
        get() = listOf(title, subtitle.substringBefore(" · "))
            .filter(String::isNotBlank)
            .joinToString(" · ")
}

internal fun webChatProviderPickerOptions(
    providers: List<WebChatProviderIdentity>,
    selectedProvider: WebChatProviderId,
    currentModel: String,
    currentState: String = "ready",
    authenticated: Boolean = false,
    composerReady: Boolean = true,
): List<WebChatProviderPickerOption> = providers.map { provider ->
    val selected = provider.id == selectedProvider
    val model = currentModel.trim().takeIf { selected && it.isNotBlank() }
    val session = if (selected) {
        webChatProviderSessionLabel(currentState, authenticated, composerReady)
    } else {
        "点击切换"
    }
    WebChatProviderPickerOption(
        providerId = provider.id,
        title = provider.displayName,
        subtitle = listOfNotNull(model, session).joinToString(" · "),
        avatarResId = provider.avatarResId,
        selected = selected,
    )
}

internal fun webChatProviderSessionLabel(
    state: String,
    authenticated: Boolean,
    composerReady: Boolean,
): String = when {
    state == "error" -> "连接异常"
    state == "login_required" -> "需要登录"
    !composerReady -> "正在连接"
    authenticated -> "账号会话"
    else -> "访客会话"
}
