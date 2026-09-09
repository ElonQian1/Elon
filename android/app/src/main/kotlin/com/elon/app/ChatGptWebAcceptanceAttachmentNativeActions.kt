package com.elon.app

import org.json.JSONObject
import org.json.JSONArray

internal class ChatGptWebAcceptanceAttachmentNativeActions(
    private val isChatModeActive: () -> Boolean,
    private val webChatState: () -> String,
    private val stageFixture: (String) -> ChatGptWebAcceptanceFixtureStageResult,
    private val removeFixture: () -> Boolean,
    private val pendingCount: () -> Int,
    private val fixtureStaged: () -> Boolean,
    private val attachmentSendPhase: () -> String = { "idle" },
) {
    fun stateJson(): JSONObject {
        val phase = attachmentSendPhase()
        return JSONObject()
            .put("schema", "elon.chatgpt_web.native_attachment_fixture.v1")
            .put("fixture_id", ChatGptWebAcceptanceAttachmentFixture.ID)
            .put("supported_fixture_ids", JSONArray(ChatGptWebAcceptanceAttachmentFixture.supportedIds.toList()))
            .put("fixture_staged", fixtureStaged())
            .put("composer_pending_count", pendingCount())
            .put("local_only", phase !in setOf("uploading", "sending"))
            .put("upload_started", phase in setOf("uploading", "sending"))
            .put("attachment_send_phase", phase)
    }

    fun control(action: String, args: JSONObject): JSONObject? = when (action) {
        STAGE_ACTION -> stage(args)
        REMOVE_ACTION -> remove(args)
        else -> null
    }

    private fun stage(args: JSONObject): JSONObject {
        val id = args.optString("fixture_id")
        if (!ChatGptWebAcceptanceAttachmentFixture.supports(id)) {
            return failure(STAGE_ACTION, "invalid_fixture_id")
        }
        if (!isChatModeActive()) return failure(STAGE_ACTION, "chatgpt_web_ai_not_active")
        if (webChatState() != "ready") return failure(STAGE_ACTION, "chatgpt_web_ai_not_ready")
        if (attachmentSendPhase() in setOf("uploading", "sending")) {
            return failure(STAGE_ACTION, "attachment_send_in_progress")
        }
        return when (val result = stageFixture(id)) {
            ChatGptWebAcceptanceFixtureStageResult.STAGED,
            ChatGptWebAcceptanceFixtureStageResult.ALREADY_STAGED -> stateJson()
                .put("control_ok", true)
                .put("action", STAGE_ACTION)
                .put("stage_result", result.wireValue)
                .put("fixture_id", id)
            ChatGptWebAcceptanceFixtureStageResult.PENDING_ATTACHMENTS_PRESENT ->
                failure(STAGE_ACTION, result.wireValue)
            ChatGptWebAcceptanceFixtureStageResult.FAILED -> failure(STAGE_ACTION, result.wireValue)
        }
    }

    private fun remove(args: JSONObject): JSONObject {
        if (!ChatGptWebAcceptanceAttachmentFixture.supports(args.optString("fixture_id"))) {
            return failure(REMOVE_ACTION, "invalid_fixture_id")
        }
        if (attachmentSendPhase() in setOf("uploading", "sending")) {
            return failure(REMOVE_ACTION, "attachment_send_in_progress")
        }
        val removed = removeFixture()
        return stateJson()
            .put("control_ok", true)
            .put("action", REMOVE_ACTION)
            .put("removed", removed)
    }

    private fun failure(action: String, error: String): JSONObject = stateJson()
        .put("control_ok", false)
        .put("action", action)
        .put("error", error)

    companion object {
        const val STAGE_ACTION = "stage_chatgpt_web_acceptance_attachment"
        const val REMOVE_ACTION = "remove_chatgpt_web_acceptance_attachment"
    }
}
