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

internal object WebChatConversationProjectMoveTiming {
    const val POLL_INTERVAL_MS = 500L
    const val NAVIGATION_POLL_LIMIT = 60
    const val CONTROL_POLL_LIMIT = 30
    const val COMMAND_POLL_LIMIT = 30
    const val RECONCILIATION_POLL_LIMIT = 60
    const val CONTROL_REFRESH_POLL = 4
    const val DIRECTORY_REFRESH_POLL = 10

    const val NAVIGATION_TIMEOUT_MS = POLL_INTERVAL_MS * NAVIGATION_POLL_LIMIT
    const val CONTROL_TIMEOUT_MS = POLL_INTERVAL_MS * CONTROL_POLL_LIMIT
    const val COMMAND_TIMEOUT_MS = POLL_INTERVAL_MS * COMMAND_POLL_LIMIT
    const val RECONCILIATION_TIMEOUT_MS = POLL_INTERVAL_MS * RECONCILIATION_POLL_LIMIT

    fun shouldRefreshControls(attempt: Int): Boolean =
        attempt >= 0 &&
            attempt < CONTROL_POLL_LIMIT &&
            attempt % CONTROL_REFRESH_POLL == 0

    fun shouldRefreshDirectory(attempt: Int): Boolean =
        attempt > 0 &&
            attempt < RECONCILIATION_POLL_LIMIT &&
            attempt % DIRECTORY_REFRESH_POLL == 0
}

internal class WebChatConversationProjectMoveSheetLease {
    private var generation = 0L

    fun issue(): Long = ++generation

    fun invalidate() {
        generation += 1
    }

    fun owns(candidate: Long): Boolean = candidate == generation
}
