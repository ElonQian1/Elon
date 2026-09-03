package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationIndex
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebProject

internal object WebChatConversationProjectMoveDiagnostics {
    fun recordControls(
        stage: String,
        attempt: Int,
        state: WebChatConsumerState,
        conversation: ChatGptWebConversation,
    ) {
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        val overlay = state.controls.filter { it.control.region == "overlay" }
        val candidates = overlay.filter { descriptor ->
            descriptor.control.semantic == "save_to_project" ||
                WebChatConversationProjectMovePolicy.isGenericMoveLabel(descriptor.control.label)
        }
        DebugTraceStore.record(
            "web_chat_project_move_controls",
            mapOf(
                "stage" to stage,
                "attempt" to attempt,
                "adapter_current" to state.adapterCurrent,
                "control_count" to state.controls.size,
                "overlay_count" to overlay.size,
                "candidate_count" to candidates.size,
                "matching_context_count" to candidates.count { it.control.contextId == identity },
                "unscoped_count" to candidates.count { it.control.contextId == null },
            ),
        )
    }

    fun recordReconciliation(
        attempt: Int,
        index: ChatGptWebConversationIndexState,
        state: WebChatConsumerState,
        conversation: ChatGptWebConversation,
        destination: ChatGptWebProject,
    ) {
        val identity = ChatGptWebConversationIndex.identityOf(conversation)
        val matches = index.conversations.filter {
            ChatGptWebConversationIndex.identityOf(it) == identity
        }
        val pagePath = ChatGptWebConversationPath.fromUrl(state.pageUrl)
        val collection = index.projectCollections[destination.id]
        DebugTraceStore.record(
            "web_chat_project_move_reconciliation",
            mapOf(
                "attempt" to attempt,
                "identity_match_count" to matches.size,
                "destination_match_count" to matches.count { it.projectId == destination.id },
                "project_cache_state" to collection?.officialLoadState.orEmpty(),
                "project_cache_stale" to (collection?.stale == true),
                "page_identity_match" to
                    (ChatGptWebConversationPath.identity(pagePath) == identity),
                "page_project_match" to
                    (ChatGptWebConversationPath.projectId(pagePath) == destination.id),
                "confirmation_available" to
                    (WebChatConversationProjectMovePolicy.confirmation(state, conversation) != null),
            ),
        )
    }
}
