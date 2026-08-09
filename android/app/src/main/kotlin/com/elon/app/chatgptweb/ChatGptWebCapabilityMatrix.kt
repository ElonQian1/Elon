package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebCapabilityMatrix {
    fun build(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest?,
        bridgeState: ChatGptWebPageAdapter.State,
        mode: ChatGptWebModeController.Mode,
    ): JSONObject {
        val advertised = snapshot?.capabilities?.supported.orEmpty()
        val semantics = manifest?.controls.orEmpty()
            .groupingBy(ChatGptWebUiControl::semantic)
            .eachCount()
            .toSortedMap()
        val knownCapabilityIds = DEFINITIONS.mapTo(mutableSetOf(), Definition::id)
        val unknownCapabilities = (advertised - knownCapabilityIds).sorted()
        val unknownSemantics = (semantics.keys - KNOWN_SEMANTICS).sorted()
        val rows = DEFINITIONS.map { definition ->
            val observed = when (definition.id) {
                SESSION_LOGIN -> snapshot?.authenticated == true
                PROMPT_SEND -> snapshot?.composerReady == true
                else -> definition.id in advertised
            }
            JSONObject()
                .put("id", definition.id)
                .put("observed", observed)
                .put("native_surface", definition.nativeSurface)
                .put("mcp_action", definition.mcpAction)
                .put("official_fallback", true)
                .put("coverage", if (observed) "native_and_mcp" else "not_observed")
        }
        val blockingGaps = buildList {
            if (bridgeState != ChatGptWebPageAdapter.State.READY) add("bridge_not_ready")
            if (snapshot?.authenticated != true) add("not_authenticated")
            if (snapshot?.authenticated == true && snapshot.composerReady.not()) add("composer_not_ready")
            if (manifest == null) add("manifest_unavailable")
            if (manifest != null && manifest.compatibility != "healthy") add("manifest_${manifest.compatibility}")
        }
        val reviewReasons = buildList {
            if (semantics[GENERIC_ACTION].orZero() > 0) add("generic_controls_present")
            if (unknownCapabilities.isNotEmpty()) add("unknown_capabilities")
            if (unknownSemantics.isNotEmpty()) add("unknown_semantics")
        }
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_capability_matrix")
            .put("schema", "elon.chatgpt_web.capability_matrix.v1")
            .put("bridge_state", bridgeState.name.lowercase())
            .put("view_mode", mode.name.lowercase())
            .put("authenticated", snapshot?.authenticated ?: false)
            .put("ready_for_chat", blockingGaps.isEmpty())
            .put("ready_for_mcp", bridgeState == ChatGptWebPageAdapter.State.READY && manifest != null)
            .put("official_fallback", true)
            .put("manifest", JSONObject()
                .put("version", manifest?.version ?: JSONObject.NULL)
                .put("page_kind", manifest?.pageKind ?: JSONObject.NULL)
                .put("compatibility", manifest?.compatibility ?: JSONObject.NULL)
                .put("control_count", manifest?.controls?.size ?: 0)
                .put("generic_control_count", semantics[GENERIC_ACTION].orZero())
            )
            .put("capabilities", JSONArray(rows))
            .put("observed_semantics", JSONObject().apply {
                semantics.forEach { (semantic, count) -> put(semantic, count) }
            })
            .put("unknown_capabilities", JSONArray(unknownCapabilities))
            .put("unknown_semantics", JSONArray(unknownSemantics))
            .put("blocking_gaps", JSONArray(blockingGaps))
            .put("adaptation_review", JSONObject()
                .put("required", reviewReasons.isNotEmpty())
                .put("reasons", JSONArray(reviewReasons))
            )
    }

    private fun Int?.orZero(): Int = this ?: 0

    private data class Definition(
        val id: String,
        val nativeSurface: Boolean,
        val mcpAction: String,
    )

    private const val SESSION_LOGIN = "session_login"
    private const val PROMPT_SEND = "prompt_send"
    private const val GENERIC_ACTION = "action"

    private val DEFINITIONS = listOf(
        Definition(SESSION_LOGIN, true, "ui_state"),
        Definition(PROMPT_SEND, true, "send_input"),
        Definition(ChatGptWebCapabilityId.STREAMING, true, "ui_state"),
        Definition(ChatGptWebCapabilityId.CURRENT_CONVERSATION, true, "chatgpt_get_context"),
        Definition(ChatGptWebCapabilityId.CONVERSATION_LIST, true, "chatgpt_get_conversations"),
        Definition(ChatGptWebCapabilityId.CONVERSATION_SEARCH, true, "chatgpt_get_conversations"),
        Definition(ChatGptWebCapabilityId.DRAFT_SYNC, true, "set_input_text"),
        Definition(ChatGptWebCapabilityId.NEW_CONVERSATION, true, "chatgpt_new_conversation"),
        Definition(ChatGptWebCapabilityId.ATTACHMENTS, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.MODEL_SELECTOR, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.COMPOSER_TOOLS, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.DICTATION, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.GOOGLE_LOGIN_ENTRY, true, "chatgpt_select_view"),
        Definition(ChatGptWebCapabilityId.RICH_TEXT, true, "chatgpt_get_context"),
        Definition(ChatGptWebCapabilityId.MESSAGE_COPY, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.MESSAGE_REGENERATE, true, "chatgpt_invoke_control"),
        Definition(ChatGptWebCapabilityId.FEATURE_NAVIGATION, true, "chatgpt_get_navigation"),
        Definition(ChatGptWebCapabilityId.COMPLEX_OUTPUT, true, "chatgpt_get_context"),
    )

    private val KNOWN_SEMANTICS = setOf(
        "navigation", "title", "profile", "new_conversation", "attachment", "model",
        "dictation", "send", "stop", "suggestion", "copy", "regenerate", "edit",
        "share", "feedback", "read_aloud", "branch", "delete", "close", "confirm",
        "conversation", "search", "library", "tasks", "project", "gpts", "settings",
        GENERIC_ACTION,
    )
}
