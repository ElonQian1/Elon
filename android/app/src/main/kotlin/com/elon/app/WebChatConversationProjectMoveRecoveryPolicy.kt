package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.ChatGptWebConversationIndex
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebProject

internal enum class WebChatConversationProjectMoveRecoveryOutcome {
    MOVED_TO_DESTINATION,
    REMAINS_AT_SOURCE,
    PENDING,
}

internal object WebChatConversationProjectMoveRecoveryPolicy {
    fun resolve(
        index: ChatGptWebConversationIndexState,
        conversation: ChatGptWebConversation,
        sourceProjectId: String?,
        destination: ChatGptWebProject,
    ): WebChatConversationProjectMoveRecoveryOutcome {
        if (WebChatConversationProjectMovePolicy.reconciled(index, conversation, destination)) {
            return WebChatConversationProjectMoveRecoveryOutcome.MOVED_TO_DESTINATION
        }
        if (
            index.collection.officialLoadState != ChatGptWebConversationCollection.LOAD_READY ||
            index.collection.stale
        ) {
            return WebChatConversationProjectMoveRecoveryOutcome.PENDING
        }
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        val current = index.conversations.singleOrNull {
            ChatGptWebConversationIndex.identityOf(it) == identity
        } ?: return WebChatConversationProjectMoveRecoveryOutcome.PENDING
        val declaredProjectId = ChatGptWebConversationPath.canonicalProjectId(current.projectId)
        val pathProjectId = ChatGptWebConversationPath.projectId(current.path)
        if (declaredProjectId != pathProjectId) {
            return WebChatConversationProjectMoveRecoveryOutcome.PENDING
        }
        val expectedSource = ChatGptWebConversationPath.canonicalProjectId(sourceProjectId)
        return if (declaredProjectId == expectedSource) {
            WebChatConversationProjectMoveRecoveryOutcome.REMAINS_AT_SOURCE
        } else {
            WebChatConversationProjectMoveRecoveryOutcome.PENDING
        }
    }
}
