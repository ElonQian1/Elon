package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatProductionMessageActionJsonTest {
    @Test
    fun discoversOnlyEnabledNonCopyMessageContexts() {
        val state = JSONObject().put(
            "ui_manifest",
            JSONObject().put(
                "controls",
                JSONArray()
                    .put(control("copy", "复制", "message-1", enabled = true))
                    .put(control("read_aloud", "朗读", "message-1", enabled = true))
                    .put(control("action", "不可用", "message-2", enabled = false))
                    .put(control("action", "页面操作", "", enabled = true).put("region", "content")),
            ),
        )

        assertEquals(setOf("message-1"), WebChatProductionMessageActionJson.messageContextIds(state))
    }

    @Test
    fun parsesSafeNativeContextActionsWithoutDuplicatingCopyOrFallbackControls() {
        val response = JSONObject().put(
            "controls",
            JSONArray()
                .put(control("copy", "复制", "message-1", enabled = true))
                .put(
                    control("read_aloud", "朗读", "message-1", enabled = true)
                        .put("control_id", "control_read_aloud")
                        .put("requires_user_confirmation", true),
                )
                .put(
                    control("action", "官网专用", "message-1", enabled = true)
                        .put("control_id", "control_official")
                        .put("native_presentation", "official_fallback"),
                ),
        )

        assertEquals(
            listOf(WebChatContextAction("control_read_aloud", "朗读", true)),
            WebChatProductionMessageActionJson.contextActions(response),
        )
    }

    private fun control(
        semantic: String,
        label: String,
        contextId: String,
        enabled: Boolean,
    ): JSONObject = JSONObject()
        .put("control_id", "control_${semantic}_$contextId")
        .put("semantic", semantic)
        .put("label", label)
        .put("region", "message")
        .put("context_id", contextId)
        .put("enabled", enabled)
        .put("native_presentation", "menu")
}
