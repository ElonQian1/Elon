package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAcceptanceAttachmentNativeActionsTest {
    @Test
    fun stagesOnlyThePinnedFixtureInReadyChatMode() {
        var staged = false
        val actions = actions(
            stage = {
                staged = true
                ChatGptWebAcceptanceFixtureStageResult.STAGED
            },
            fixtureStaged = { staged },
        )

        val result = actions.control(
            ChatGptWebAcceptanceAttachmentNativeActions.STAGE_ACTION,
            JSONObject().put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID),
        )!!

        assertTrue(result.getBoolean("control_ok"))
        assertEquals("staged", result.getString("stage_result"))
        assertTrue(result.getBoolean("fixture_staged"))
        assertTrue(result.getBoolean("local_only"))
        assertFalse(result.getBoolean("upload_started"))
    }

    @Test
    fun rejectsUnknownFixturesAndNonReadyChatModes() {
        val actions = actions()
        assertEquals(
            "invalid_fixture_id",
            actions.control(
                ChatGptWebAcceptanceAttachmentNativeActions.STAGE_ACTION,
                JSONObject().put("fixture_id", "user_file"),
            )!!.getString("error"),
        )
        assertEquals(
            "chatgpt_web_ai_not_active",
            actions(isChatActive = false).control(
                ChatGptWebAcceptanceAttachmentNativeActions.STAGE_ACTION,
                JSONObject().put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID),
            )!!.getString("error"),
        )
        assertEquals(
            "chatgpt_web_ai_not_ready",
            actions(webState = "loading").control(
                ChatGptWebAcceptanceAttachmentNativeActions.STAGE_ACTION,
                JSONObject().put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID),
            )!!.getString("error"),
        )
    }

    @Test
    fun removalIsIdempotentAndNeverStartsAnUpload() {
        var removed = false
        val result = actions(remove = {
            removed = true
            true
        }).control(
            ChatGptWebAcceptanceAttachmentNativeActions.REMOVE_ACTION,
            JSONObject().put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID),
        )!!

        assertTrue(removed)
        assertTrue(result.getBoolean("control_ok"))
        assertTrue(result.getBoolean("removed"))
        assertFalse(result.getBoolean("upload_started"))
    }

    @Test
    fun refusesRemovalWhileTheAttachmentSendOwnsTheFixture() {
        val result = actions(attachmentSendPhase = "uploading").control(
            ChatGptWebAcceptanceAttachmentNativeActions.REMOVE_ACTION,
            JSONObject().put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID),
        )!!

        assertFalse(result.getBoolean("control_ok"))
        assertEquals("attachment_send_in_progress", result.getString("error"))
    }

    private fun actions(
        isChatActive: Boolean = true,
        webState: String = "ready",
        stage: () -> ChatGptWebAcceptanceFixtureStageResult = {
            ChatGptWebAcceptanceFixtureStageResult.STAGED
        },
        remove: () -> Boolean = { false },
        fixtureStaged: () -> Boolean = { false },
        attachmentSendPhase: String = "idle",
    ) = ChatGptWebAcceptanceAttachmentNativeActions(
        isChatModeActive = { isChatActive },
        webChatState = { webState },
        stageFixture = stage,
        removeFixture = remove,
        pendingCount = { if (fixtureStaged()) 1 else 0 },
        fixtureStaged = fixtureStaged,
        attachmentSendPhase = { attachmentSendPhase },
    )
}
