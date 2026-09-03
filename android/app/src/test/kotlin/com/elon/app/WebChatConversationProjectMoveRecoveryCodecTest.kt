package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WebChatConversationProjectMoveRecoveryCodecTest {
    @Test
    fun roundTripsTheMinimumLocalRecoveryState() {
        val record = WebChatConversationProjectMoveRecoveryRecord(
            conversationPath = "/g/$SOURCE/c/conversation-1",
            sourceProjectId = SOURCE,
            destinationProjectId = DESTINATION,
            stage = WebChatConversationProjectMoveStage.WRITE_ARMED,
            createdAtMs = 10L,
            updatedAtMs = 20L,
        )

        assertEquals(
            record,
            WebChatConversationProjectMoveRecoveryCodec.decode(
                WebChatConversationProjectMoveRecoveryCodec.encode(record),
            ),
        )
    }

    @Test
    fun acceptsAnUnfiledSourceWithoutInventingAProject() {
        val record = WebChatConversationProjectMoveRecoveryRecord(
            conversationPath = "/c/conversation-2",
            sourceProjectId = null,
            destinationProjectId = DESTINATION,
            stage = WebChatConversationProjectMoveStage.PREPARED,
            createdAtMs = 30L,
            updatedAtMs = 30L,
        )

        assertEquals(
            record,
            WebChatConversationProjectMoveRecoveryCodec.decode(
                WebChatConversationProjectMoveRecoveryCodec.encode(record),
            ),
        )
    }

    @Test
    fun rejectsInvalidOrSelfReferentialRecoveryRecords() {
        val raw = JSONObject()
            .put("schema", "elon.chatgpt_web.project_move_recovery.v1")
            .put("conversation_path", "/g/$SOURCE/c/conversation-3")
            .put("source_project_id", SOURCE)
            .put("destination_project_id", SOURCE)
            .put("stage", "write_armed")
            .put("created_at_ms", 40L)
            .put("updated_at_ms", 40L)
            .toString()

        assertNull(WebChatConversationProjectMoveRecoveryCodec.decode(raw))
        assertNull(WebChatConversationProjectMoveRecoveryCodec.decode("{}"))
    }

    private companion object {
        const val SOURCE = "g-p-11111111111111111111111111111111"
        const val DESTINATION = "g-p-22222222222222222222222222222222"
    }
}
