package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionFeatureCompletionTest {
    @Test
    fun waitsForBothSuccessfulReceiptAndSettledFeaturePage() {
        val feature = feature("tasks", "tasks")
        val pendingPage = state("mcp_1", "succeeded", pageKind = "conversation")
        val settledPage = state("mcp_1", "succeeded", pageKind = "tasks")

        assertEquals(
            WebChatProductionFeatureCompletionDecision.WAITING,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature, "mcp_1", pendingPage),
        )
        assertEquals(
            WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature, "mcp_1", settledPage),
        )
    }

    @Test
    fun acceptsKnownOfficialUrlAfterNavigation() {
        val state = state("mcp_2", "succeeded", pageKind = "unknown")
            .put("conversation", JSONObject().put("url", "https://chatgpt.com/tasks?ref=native"))

        assertTrue(WebChatProductionFeatureCompletionPolicy.pageSettled(feature("tasks", "tasks"), state))
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(feature("library", "library"), state))
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(
            feature("tasks", "tasks"),
            state.put("conversation", JSONObject().put("url", "https://chatgpt.com/tasks-unrelated")),
        ))
    }

    @Test
    fun rejectsUntrustedUrlAndFailedCommand() {
        val state = state("mcp_3", "failed", pageKind = "tasks")
            .put("conversation", JSONObject().put("url", "https://example.com/tasks"))

        assertEquals(
            WebChatProductionFeatureCompletionDecision.FAILED,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature("tasks", "tasks"), "mcp_3", state),
        )
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(
            feature("tasks", "tasks"),
            state.put("page_kind", "unknown"),
        ))
    }

    @Test
    fun futureFeatureUsesPageKindAndOfficialCompletionByDefault() {
        val future = feature("future", "future_workspace")
        val state = state("mcp_4", "succeeded", pageKind = "future_workspace")

        assertTrue(WebChatProductionFeatureCompletionPolicy.requiresOfficialCompletion(future.kind))
        assertEquals(
            WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL,
            WebChatProductionFeatureCompletionPolicy.evaluate(future, "mcp_4", state),
        )
    }

    @Test
    fun extractsOnlyStructuredCommandReceiptIds() {
        assertEquals(
            "mcp_5",
            WebChatProductionFeatureCompletionPolicy.requestId(JSONObject()
                .put("command_receipt", JSONObject().put("request_id", "mcp_5"))),
        )
        assertNull(WebChatProductionFeatureCompletionPolicy.requestId(JSONObject()))
        assertNull(WebChatProductionFeatureCompletionPolicy.requestId(JSONObject()
            .put("command_receipt", JSONObject().put("request_id", JSONObject.NULL))))
    }

    private fun feature(id: String, kind: String) = WebChatProductionFeature(
        id = id,
        label = id,
        kind = kind,
        selected = false,
        requiresUserConfirmation = false,
        officialCompletion = true,
        nativeSelector = "web-chat-feature:$id",
    )

    private fun state(requestId: String, status: String, pageKind: String): JSONObject = JSONObject()
        .put("page_kind", pageKind)
        .put("conversation", JSONObject().put("url", "https://chatgpt.com/"))
        .put("command_requests", JSONArray().put(JSONObject()
            .put("request_id", requestId)
            .put("status", status)))
}
