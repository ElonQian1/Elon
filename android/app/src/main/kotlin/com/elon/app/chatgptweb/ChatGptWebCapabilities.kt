package com.elon.app.chatgptweb

internal data class ChatGptWebCapabilities(
    val supported: Set<String>,
) {
    fun supports(capability: String): Boolean = capability in supported

    companion object {
        val EMPTY = ChatGptWebCapabilities(emptySet())
    }
}

internal object ChatGptWebCapabilityId {
    const val STREAMING = "streaming"
    const val CURRENT_CONVERSATION = "conversation_history"
    const val CONVERSATION_LIST = "conversation_list"
    const val CONVERSATION_SEARCH = "conversation_search"
    const val DRAFT_SYNC = "draft_sync"
    const val NEW_CONVERSATION = "new_conversation"
    const val ATTACHMENTS = "attachments"
    const val MODEL_SELECTOR = "model_selector"
    const val COMPOSER_TOOLS = "composer_tools"
    const val DICTATION = "dictation"
    const val GOOGLE_LOGIN_ENTRY = "google_login_entry"
    const val RICH_TEXT = "rich_text"
    const val MESSAGE_COPY = "message_copy"
    const val MESSAGE_REGENERATE = "message_regenerate"
    const val FEATURE_NAVIGATION = "feature_navigation"
    const val COMPLEX_OUTPUT = "complex_output"
}

internal data class ChatGptWebConversation(
    val id: String,
    val title: String,
    val path: String,
    val active: Boolean,
)
