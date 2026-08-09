package com.elon.app.chatgptweb

internal class ChatGptWebObservedState {
    private var conversations: List<ChatGptWebConversation> = emptyList()
    private var features: List<ChatGptWebFeature> = emptyList()
    private var composerSections: Map<String, List<ChatGptWebComposerOption>> = emptyMap()
    private var lastCommand: ChatGptWebEvent.CommandResult? = null
    private var updatedAtMs: Long = 0L

    fun accept(event: ChatGptWebEvent) {
        when (event) {
            is ChatGptWebEvent.ConversationList -> conversations = event.conversations
            is ChatGptWebEvent.FeatureNavigation -> features = event.features
            is ChatGptWebEvent.ComposerControls -> {
                composerSections = composerSections + (event.section to event.options)
            }
            is ChatGptWebEvent.CommandResult -> lastCommand = event
            else -> return
        }
        updatedAtMs = System.currentTimeMillis()
    }

    fun snapshot(): Snapshot = Snapshot(
        conversations = conversations,
        features = features,
        composerSections = composerSections,
        lastCommand = lastCommand,
        updatedAtMs = updatedAtMs,
    )

    internal data class Snapshot(
        val conversations: List<ChatGptWebConversation>,
        val features: List<ChatGptWebFeature>,
        val composerSections: Map<String, List<ChatGptWebComposerOption>>,
        val lastCommand: ChatGptWebEvent.CommandResult?,
        val updatedAtMs: Long,
    ) {
        companion object {
            val EMPTY = Snapshot(emptyList(), emptyList(), emptyMap(), null, 0L)
        }
    }
}
