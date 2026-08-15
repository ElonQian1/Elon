package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationIndexState

internal class WebChatNavigationSession(
    val providerId: WebChatProviderId,
    val capabilities: Set<WebChatProviderCapability>,
    private val indexSource: () -> ChatGptWebConversationIndexState,
    private val refreshSource: () -> Boolean,
    private val newConversationSource: () -> Boolean,
    private val openConversationSource: (String) -> Boolean,
    private val openProjectSource: (String) -> Boolean,
) {
    fun index(): ChatGptWebConversationIndexState = indexSource()

    fun refresh(): Boolean = refreshSource()

    fun newConversation(): Boolean = newConversationSource()

    fun openConversation(path: String): Boolean = openConversationSource(path)

    fun openProject(path: String): Boolean = openProjectSource(path)
}

internal class WebChatNavigationSessionRegistry(
    sessions: List<WebChatNavigationSession>,
    private val identity: (WebChatProviderId) -> WebChatProviderIdentity = WebChatProviderRegistry::get,
) {
    private val sessionsByProvider = sessions.associateBy(WebChatNavigationSession::providerId)

    init {
        require(sessionsByProvider.size == sessions.size) { "Duplicate web chat navigation provider" }
    }

    fun session(providerId: WebChatProviderId): WebChatNavigationSession? {
        val provider = identity(providerId)
        if (!provider.selectable) return null
        return sessionsByProvider[providerId]?.takeIf { session ->
            WebChatProviderIdentity.REQUIRED_NATIVE_NAVIGATION.all(session.capabilities::contains) &&
                session.capabilities.all(provider.capabilities::contains)
        }
    }
}
