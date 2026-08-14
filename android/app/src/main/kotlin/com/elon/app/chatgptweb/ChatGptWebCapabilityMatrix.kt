package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebCapabilityMatrix {
    fun build(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest?,
        bridgeState: ChatGptWebPageAdapter.State,
        mode: ChatGptWebModeController.Mode,
        document: ChatGptWebObservedState.Snapshot? = null,
        verificationEvidence: ChatGptWebVerificationEvidenceStore.Snapshot =
            ChatGptWebVerificationEvidenceStore.Snapshot.EMPTY,
    ): JSONObject {
        val adapterCurrent = document?.adapterCurrent ?: true
        val advertised = snapshot?.capabilities?.supported.orEmpty()
        val semantics = manifest?.controls.orEmpty()
            .groupingBy(ChatGptWebUiControl::semantic)
            .eachCount()
            .toSortedMap()
        val knownCapabilityIds = DEFINITIONS.mapTo(mutableSetOf(), Definition::id)
        val unknownCapabilities = (advertised - knownCapabilityIds).sorted()
        val unknownSemantics = (semantics.keys - ChatGptWebUiSemantics.KNOWN).sorted()
        val controlCoverage = ChatGptNativeControlPresentation.describe(manifest?.controls.orEmpty())
        val fallbackControls = manifest?.controls.orEmpty().filter { control ->
            controlCoverage.getValue(control.id).kind == ChatGptNativeControlPresentation.Kind.OFFICIAL_FALLBACK
        }
        val expectedFallbackControls = fallbackControls.filter(
            ChatGptNativeControlPresentation::isExpectedOfficialFallback,
        )
        val unexpectedFallbackControls = fallbackControls - expectedFallbackControls.toSet()
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
        val loginRequired = snapshot?.let(ChatGptWebAccessPolicy::requiresLogin) == true
        val canChat = snapshot?.let(ChatGptWebAccessPolicy::canChat) == true
        val blockingGaps = buildList {
            if (bridgeState != ChatGptWebPageAdapter.State.READY) add("bridge_not_ready")
            if (!adapterCurrent) add("adapter_generation_not_ready")
            if (snapshot == null) add("snapshot_unavailable")
            if (loginRequired) add("login_required")
            if (
                snapshot != null &&
                !loginRequired &&
                !canChat &&
                !snapshot.dictationActive
            ) add("composer_not_ready")
            if (manifest == null) add("manifest_unavailable")
            if (
                manifest != null &&
                manifest.compatibility != "healthy" &&
                snapshot?.dictationActive != true
            ) add("manifest_${manifest.compatibility}")
            if (manifest?.controlsTruncated == true) add("manifest_controls_truncated")
        }
        val reviewReasons = buildList {
            if (semantics[ChatGptWebUiSemantics.GENERIC_ACTION].orZero() > 0) {
                add("generic_controls_present")
            }
            if (unknownCapabilities.isNotEmpty()) add("unknown_capabilities")
            if (unknownSemantics.isNotEmpty()) add("unknown_semantics")
            if (unexpectedFallbackControls.isNotEmpty()) {
                add("unexpected_official_fallback_controls_present")
            }
            if (manifest?.controlsTruncated == true) add("manifest_controls_truncated")
        }
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_capability_matrix")
            .put("schema", "elon.chatgpt_web.capability_matrix.v3")
            .put("adapter_version", ChatGptWebPageAdapter.ADAPTER_VERSION)
            .put("page_generation", document?.pageGeneration ?: 0L)
            .put("adapter_generation", document?.adapterGeneration ?: 0L)
            .put("adapter_current", adapterCurrent)
            .put(
                "app",
                JSONObject()
                    .put("version_name", BuildConfig.VERSION_NAME)
                    .put("version_code", BuildConfig.VERSION_CODE),
            )
            .put("bridge_state", bridgeState.name.lowercase())
            .put("view_mode", mode.name.lowercase())
            .put("authenticated", snapshot?.authenticated ?: false)
            .put("login_required", loginRequired)
            .put("chat_access_available", canChat)
            .put("dictation_active", snapshot?.dictationActive ?: false)
            .put("ready_for_chat", blockingGaps.isEmpty())
            .put(
                "ready_for_mcp",
                bridgeState == ChatGptWebPageAdapter.State.READY &&
                    adapterCurrent &&
                    manifest != null &&
                    manifest.controlsTruncated.not(),
            )
            .put("official_fallback", true)
            .put("manifest", JSONObject()
                .put("version", manifest?.version ?: JSONObject.NULL)
                .put("page_kind", manifest?.pageKind ?: JSONObject.NULL)
                .put("compatibility", manifest?.compatibility ?: JSONObject.NULL)
                .put("control_count", manifest?.controls?.size ?: 0)
                .put("discovered_control_count", manifest?.discoveredControlCount ?: 0)
                .put("controls_truncated", manifest?.controlsTruncated ?: false)
                .put(
                    "generic_control_count",
                    semantics[ChatGptWebUiSemantics.GENERIC_ACTION].orZero(),
                )
                .put(
                    "native_control_count",
                    controlCoverage.values.count { it.kind.isNativeAction() },
                )
                .put(
                    "native_menu_control_count",
                    controlCoverage.values.count { it.kind == ChatGptNativeControlPresentation.Kind.MENU },
                )
                .put("official_fallback_control_count", fallbackControls.size)
                .put("expected_official_fallback_control_count", expectedFallbackControls.size)
                .put("unexpected_official_fallback_control_count", unexpectedFallbackControls.size)
            )
            .put("capabilities", JSONArray(rows))
            .put(
                "product_capabilities",
                ChatGptWebProductCapabilityCatalog.describe(
                    features = document?.features.orEmpty(),
                    composerOptions = document?.composerSections
                        ?.values
                        ?.flatten()
                        .orEmpty(),
                    controls = manifest?.controls.orEmpty(),
                ),
            )
            .put("control_coverage", JSONArray().apply {
                manifest?.controls.orEmpty().forEach { control ->
                    val coverage = controlCoverage.getValue(control.id)
                    val invocationRisk = ChatGptWebControlInvocationPolicy.risk(control)
                    put(JSONObject()
                        .put("control_id", control.id)
                        .put("semantic", control.semantic)
                        .put("region", control.region)
                        .put(
                            "invocation_risk",
                            invocationRisk.wireName,
                        )
                        .put(
                            "requires_user_confirmation",
                            invocationRisk ==
                                ChatGptWebControlInvocationPolicy.Risk.USER_CONFIRMATION,
                        )
                        .put(
                            "confirmation_argument",
                            if (
                                invocationRisk ==
                                ChatGptWebControlInvocationPolicy.Risk.USER_CONFIRMATION
                            ) {
                                "user_confirmed"
                            } else {
                                JSONObject.NULL
                            },
                        )
                        .put("presentation", coverage.kind.wireName)
                        .put(
                            "official_fallback_policy",
                            when {
                                coverage.kind != ChatGptNativeControlPresentation.Kind.OFFICIAL_FALLBACK ->
                                    JSONObject.NULL
                                ChatGptNativeControlPresentation.isExpectedOfficialFallback(control) ->
                                    "expected"
                                else -> "review_required"
                            },
                        )
                        .put("native_adb_content_description", coverage.nativeSelector ?: JSONObject.NULL)
                        .put(
                            "native_trigger_content_description",
                            coverage.nativeTriggerSelector ?: JSONObject.NULL,
                        )
                        .put(
                            "mcp_action",
                            if (coverage.kind == ChatGptNativeControlPresentation.Kind.METADATA) {
                                JSONObject.NULL
                            } else {
                                "chatgpt_invoke_control"
                            },
                        )
                        .put(
                            "mcp_arguments",
                            if (coverage.kind == ChatGptNativeControlPresentation.Kind.METADATA) {
                                JSONObject.NULL
                            } else {
                                JSONObject().put("control_id", control.id)
                            },
                        )
                        .put("official_fallback", true)
                    )
                }
            })
            .put("observed_semantics", JSONObject().apply {
                semantics.forEach { (semantic, count) -> put(semantic, count) }
            })
            .put("unknown_capabilities", JSONArray(unknownCapabilities))
            .put("unknown_semantics", JSONArray(unknownSemantics))
            .put("blocking_gaps", JSONArray(blockingGaps))
            .put(
                "feature_baseline",
                ChatGptWebFeatureBaseline.describe(
                    snapshot,
                    manifest,
                    mode,
                    document?.composerSections?.values?.flatten().orEmpty(),
                    verificationEvidence,
                ),
            )
            .put("adaptation_review", JSONObject()
                .put("required", reviewReasons.isNotEmpty())
                .put("reasons", JSONArray(reviewReasons))
            )
    }

    private fun Int?.orZero(): Int = this ?: 0

    private fun ChatGptNativeControlPresentation.Kind.isNativeAction(): Boolean =
        this != ChatGptNativeControlPresentation.Kind.OFFICIAL_FALLBACK &&
            this != ChatGptNativeControlPresentation.Kind.METADATA

    private data class Definition(
        val id: String,
        val nativeSurface: Boolean,
        val mcpAction: String,
    )

    private const val SESSION_LOGIN = "session_login"
    private const val PROMPT_SEND = "prompt_send"
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
        Definition(ChatGptWebCapabilityId.MESSAGE_COPY, true, "chatgpt_copy_last_response"),
        Definition(ChatGptWebCapabilityId.MESSAGE_REGENERATE, true, "chatgpt_regenerate_response"),
        Definition(ChatGptWebCapabilityId.FEATURE_NAVIGATION, true, "chatgpt_get_navigation"),
        Definition(ChatGptWebCapabilityId.COMPLEX_OUTPUT, true, "chatgpt_get_context"),
    )

}
