package com.elon.app.chatgptweb

/** Keeps a ready native composer stable while the hidden official tool menu is open. */
internal object ChatGptWebTransientComposerReadiness {
    fun interactionActive(
        optionRequestActive: Boolean,
        commandRequests: List<ChatGptWebObservedState.CommandRequest>,
    ): Boolean = optionRequestActive || commandRequests.any { request ->
        request.status == ChatGptWebObservedState.CommandRequest.PENDING &&
            request.expectedAction in COMPOSER_INTERACTION_ACTIONS
    }

    fun reconcile(
        previous: ChatGptWebSnapshot?,
        incoming: ChatGptWebSnapshot,
        composerInteractionActive: Boolean,
    ): ChatGptWebSnapshot {
        if (incoming.composerReady) return incoming
        val preserveForNativeInteraction = composerInteractionActive || incoming.dictationActive
        if (!preserveForNativeInteraction) return incoming
        if (previous?.let(ChatGptWebAccessPolicy::canChat) != true) return incoming
        if (ChatGptWebAccessPolicy.requiresLogin(incoming)) return incoming
        if (!sameConversationSurface(previous, incoming)) return incoming
        return incoming.copy(
            composerReady = true,
            currentModel = incoming.currentModel.ifBlank { previous.currentModel },
            capabilities = ChatGptWebCapabilities(
                previous.capabilities.supported + incoming.capabilities.supported,
            ),
        )
    }

    private fun sameConversationSurface(
        previous: ChatGptWebSnapshot,
        incoming: ChatGptWebSnapshot,
    ): Boolean {
        val previousPath = ChatGptWebConversationPath.fromUrl(previous.url)
        val incomingPath = ChatGptWebConversationPath.fromUrl(incoming.url)
        if (previousPath != null || incomingPath != null) return previousPath == incomingPath
        return previous.pageKind in CHAT_SURFACES && incoming.pageKind in CHAT_SURFACES
    }

    private val CHAT_SURFACES = setOf("home", "conversation", "unknown")
    private val COMPOSER_INTERACTION_ACTIONS = setOf(
        "list_model_options",
        "list_composer_tools",
        "collect_model_options",
        "collect_composer_tools",
        "select_model_option",
        "select_composer_tool",
        "dismiss_composer_menu",
    )
}
