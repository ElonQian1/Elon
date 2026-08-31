package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndex
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import java.util.Locale

internal object WebChatConversationProjectMovePolicy {
    fun destinations(
        index: ChatGptWebConversationIndexState,
        conversation: ChatGptWebConversation,
    ): List<ChatGptWebProject> = index.projects
        .asSequence()
        .filterNot { it.id == conversation.projectId }
        .distinctBy(ChatGptWebProject::id)
        .take(MAX_PROJECTS)
        .toList()

    fun conversationOptions(
        state: WebChatConsumerState,
        conversation: ChatGptWebConversation,
    ): WebChatConsumerControlDescriptor? {
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        return state.controls.asSequence()
            .filter { it.control.enabled }
            .filter { it.control.semantic == "conversation_options" }
            .filter { it.control.region == "header" }
            .filter { it.control.contextId == identity }
            .singleOrNull()
    }

    fun moveTrigger(
        state: WebChatConsumerState,
        conversation: ChatGptWebConversation,
    ): WebChatConsumerControlDescriptor? {
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        val candidates = state.controls.asSequence()
            .filter { it.control.enabled && it.control.region == "overlay" }
            .filter { it.control.contextId == identity }
            .filter { descriptor ->
                descriptor.control.semantic == "save_to_project" ||
                    isGenericMoveLabel(descriptor.control.label)
            }
            .toList()
        return candidates.singleOrNull { it.control.semantic == "save_to_project" }
            ?: candidates.singleOrNull()
    }

    fun projectChoice(
        state: WebChatConsumerState,
        destination: ChatGptWebProject,
    ): WebChatConsumerControlDescriptor? {
        val expected = normalizedLabel(destination.title)
        if (expected.isBlank() || isGenericMoveLabel(expected)) return null
        return state.controls.asSequence()
            .filter { it.control.enabled && it.control.region == "overlay" }
            .filter { it.control.role in PROJECT_CHOICE_ROLES }
            .filterNot { isGenericMoveLabel(it.control.label) }
            .filter { normalizedLabel(it.control.label) == expected }
            .singleOrNull()
    }

    fun commandStatus(
        state: WebChatConsumerState,
        requestId: String?,
    ): WebChatConsumerCommandStatus = requestId
        ?.let { id -> state.commandRequests.lastOrNull { it.id == id }?.status }
        ?: WebChatConsumerCommandStatus.UNKNOWN

    fun writeMayHaveBeenSubmitted(result: WebChatConsumerCommandResult): Boolean =
        result.accepted

    fun reconciled(
        index: ChatGptWebConversationIndexState,
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
    ): Boolean {
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        return index.conversations.any { current ->
            ChatGptWebConversationIndex.identityOf(current) == identity &&
                current.projectId == destination.id
        }
    }

    fun isGenericMoveLabel(value: String): Boolean = GENERIC_MOVE_LABEL.matches(
        normalizedLabel(value),
    )

    private fun normalizedLabel(value: String): String = value
        .trim()
        .replace(WHITESPACE, " ")
        .lowercase(Locale.ROOT)

    private const val MAX_PROJECTS = 40
    private val WHITESPACE = Regex("\\s+")
    private val GENERIC_MOVE_LABEL = Regex(
        "^(?:save|add|move)(?:\\s+(?:this|the))?\\s+(?:chat|conversation)?\\s*(?:to|into)?\\s*project$|" +
            "^(?:保存|添加|移动|存入)(?:到|至)?项目$",
        RegexOption.IGNORE_CASE,
    )
    private val PROJECT_CHOICE_ROLES = setOf(
        "button",
        "menuitem",
        "menuitemradio",
        "option",
        "radio",
    )
}
