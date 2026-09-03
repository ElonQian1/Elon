package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationCollection
import com.elon.app.chatgptweb.ChatGptWebConversationIndexState
import com.elon.app.chatgptweb.ChatGptWebProject
import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatConversationProjectMoveRecoveryPolicyTest {
    @Test
    fun resolvesDestinationOnlyFromConsistentMembershipEvidence() {
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.MOVED_TO_DESTINATION,
            resolve(index(conversation(DESTINATION_PROJECT))),
        )
    }

    @Test
    fun resolvesNotAppliedOnlyFromAFreshCompleteDirectory() {
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.REMAINS_AT_SOURCE,
            resolve(index(conversation(SOURCE_PROJECT))),
        )
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.PENDING,
            resolve(index(
                conversation(SOURCE_PROJECT),
                collection = ChatGptWebConversationCollection.official(1).copy(stale = true),
            )),
        )
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.PENDING,
            resolve(index(
                conversation(SOURCE_PROJECT),
                collection = ChatGptWebConversationCollection.official(1).copy(
                    officialLoadState = ChatGptWebConversationCollection.LOAD_LOADING,
                ),
            )),
        )
    }

    @Test
    fun ambiguousOrInconsistentMembershipRemainsPending() {
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.PENDING,
            resolve(index(
                conversation(SOURCE_PROJECT),
                conversation(DESTINATION_PROJECT),
            )),
        )
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.PENDING,
            resolve(index(conversation(
                declaredProjectId = SOURCE_PROJECT,
                pathProjectId = DESTINATION_PROJECT,
            ))),
        )
    }

    @Test
    fun unassignedConversationCanBeConfirmedAtItsOriginalLocation() {
        val original = ChatGptWebConversation(
            id = CONVERSATION_ID,
            title = "Unassigned",
            path = "/c/$CONVERSATION_ID",
            active = false,
        )
        assertEquals(
            WebChatConversationProjectMoveRecoveryOutcome.REMAINS_AT_SOURCE,
            WebChatConversationProjectMoveRecoveryPolicy.resolve(
                index = index(original),
                conversation = original,
                sourceProjectId = null,
                destination = destination,
            ),
        )
    }

    private fun resolve(index: ChatGptWebConversationIndexState) =
        WebChatConversationProjectMoveRecoveryPolicy.resolve(
            index = index,
            conversation = conversation(SOURCE_PROJECT),
            sourceProjectId = SOURCE_PROJECT,
            destination = destination,
        )

    private fun index(
        vararg conversations: ChatGptWebConversation,
        collection: ChatGptWebConversationCollection =
            ChatGptWebConversationCollection.official(conversations.size),
    ) = ChatGptWebConversationIndexState(
        conversations = conversations.toList(),
        projects = listOf(destination),
        collection = collection,
    )

    private fun conversation(
        declaredProjectId: String,
        pathProjectId: String = declaredProjectId,
    ) = ChatGptWebConversation(
        id = CONVERSATION_ID,
        title = "Conversation",
        path = "/g/$pathProjectId/c/$CONVERSATION_ID",
        active = false,
        projectId = declaredProjectId,
    )

    private val destination = ChatGptWebProject(
        id = DESTINATION_PROJECT,
        title = "Destination",
        path = "/g/$DESTINATION_PROJECT/project",
    )

    private companion object {
        const val CONVERSATION_ID = "conversation-current"
        const val SOURCE_PROJECT = "g-p-source"
        const val DESTINATION_PROJECT = "g-p-destination"
    }
}
