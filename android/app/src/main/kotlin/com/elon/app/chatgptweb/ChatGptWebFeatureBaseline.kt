package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebFeatureBaseline {
    const val VERSION = 4
    internal const val DEVICE_VERIFICATION_ADAPTER_VERSION = 54
    private val SHA256_PATTERN = Regex("^[0-9a-f]{64}$")
    private val DEVICE_VERIFICATION_CURRENT = isDeviceVerificationCurrent()

    internal fun isDeviceVerificationCurrent(
        adapterVersion: Int = ChatGptWebPageAdapter.ADAPTER_VERSION,
        currentInputSha256: String = BuildConfig.CHATGPT_WEB_INPUT_SHA256,
        verifiedInputSha256: String = BuildConfig.CHATGPT_WEB_VERIFIED_INPUT_SHA256,
    ): Boolean =
        adapterVersion == DEVICE_VERIFICATION_ADAPTER_VERSION &&
            SHA256_PATTERN.matches(currentInputSha256) &&
            currentInputSha256 == verifiedInputSha256

    enum class ImplementationStatus(val wireName: String) {
        COMPLETE("complete"),
        PARTIAL("partial"),
        FALLBACK_ONLY("fallback_only"),
    }

    enum class CodeStatus(val wireName: String) {
        IMPLEMENTED("implemented"),
        PARTIAL("partial"),
        OFFICIAL_FALLBACK("official_fallback"),
    }

    enum class VerificationStatus(val wireName: String) {
        OFFLINE_VERIFIED("offline_verified"),
        DEVICE_VERIFIED("device_verified"),
        USER_ACTION_REQUIRED("user_action_required"),
        DEFERRED("deferred"),
        FAILED("failed"),
    }

    enum class Delivery(val wireName: String) {
        DEDICATED_NATIVE("dedicated_native"),
        ADAPTIVE_NATIVE("adaptive_native"),
        MCP_ONLY("mcp_only"),
        OFFICIAL_WEB_WITH_NATIVE_ENTRY("official_web_with_native_entry"),
        FULLSCREEN_OFFICIAL("fullscreen_official"),
    }

    enum class Acceptance(val wireName: String) {
        READ_ONLY_DEVICE("read_only_device"),
        INTERACTIVE_DEVICE("interactive_device"),
        USER_DRIVEN_DEVICE("user_driven_device"),
    }

    data class Feature(
        val id: String,
        val group: String,
        val status: ImplementationStatus,
        val codeStatus: CodeStatus,
        val verificationStatus: VerificationStatus,
        val delivery: Delivery,
        val acceptance: Acceptance,
        val mcpActions: List<String>,
        val capabilityIds: Set<String> = emptySet(),
        val semantics: Set<String> = emptySet(),
        val codeGap: String? = null,
        val verificationGap: String? = null,
        val verificationCase: String? = null,
        val remainingGap: String? = null,
    )

    fun describe(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest?,
        mode: ChatGptWebModeController.Mode,
    ): JSONObject {
        val advertised = snapshot?.capabilities?.supported.orEmpty()
        val semantics = manifest?.controls.orEmpty()
            .groupingBy(ChatGptWebUiControl::semantic)
            .eachCount()
        val rows = FEATURES.map { feature ->
            val currentPageObserved = when (feature.id) {
                "official_authentication" -> snapshot?.authenticated == true
                "official_fullscreen_fallback" -> mode == ChatGptWebModeController.Mode.WEB
                "native_chat_composer" -> snapshot?.composerReady == true
                "disclosure_controls" -> manifest?.controls.orEmpty()
                    .any(ChatGptWebUiControl::supportsExpandedState)
                else -> feature.capabilityIds.any(advertised::contains) ||
                    feature.semantics.any { semantics[it].orZero() > 0 }
            }
            JSONObject()
                .put("id", feature.id)
                .put("group", feature.group)
                .put("implementation_status", feature.status.wireName)
                .put("code_status", feature.codeStatus.wireName)
                .put("verification_status", feature.verificationStatus.wireName)
                .put("delivery", feature.delivery.wireName)
                .put("acceptance", feature.acceptance.wireName)
                .put("current_page_observed", currentPageObserved)
                .put("mcp_actions", JSONArray(feature.mcpActions))
                .put("code_gap", feature.codeGap ?: JSONObject.NULL)
                .put("verification_case", feature.verificationCase ?: JSONObject.NULL)
                .put(
                    "verification_gap",
                    feature.verificationGap ?: JSONObject.NULL,
                )
                .put(
                    "remaining_gap",
                    feature.remainingGap ?: JSONObject.NULL,
                )
        }
        val incomplete = FEATURES.filter { it.status != ImplementationStatus.COMPLETE }
        val incompleteCode = FEATURES.filter { it.codeStatus == CodeStatus.PARTIAL }
        val pendingVerification = FEATURES.filter {
            it.verificationStatus != VerificationStatus.DEVICE_VERIFIED
        }
        return JSONObject()
            .put("schema", "elon.chatgpt_web.feature_baseline.v4")
            .put("version", VERSION)
            .put("device_verification_adapter_version", DEVICE_VERIFICATION_ADAPTER_VERSION)
            .put("device_verification_current", DEVICE_VERIFICATION_CURRENT)
            .put("device_verification_input_sha256", BuildConfig.CHATGPT_WEB_INPUT_SHA256)
            .put(
                "device_verification_verified_input_sha256",
                BuildConfig.CHATGPT_WEB_VERIFIED_INPUT_SHA256,
            )
            .put(
                "device_verification_provenance",
                JSONObject()
                    .put("schema", "elon.chatgpt_web.device_evidence.v1")
                    .put("verified_apk_version_name", BuildConfig.CHATGPT_WEB_VERIFIED_APK_VERSION_NAME)
                    .put("verified_apk_version_code", BuildConfig.CHATGPT_WEB_VERIFIED_APK_VERSION_CODE)
                    .put("verified_source_commit", BuildConfig.CHATGPT_WEB_VERIFIED_SOURCE_COMMIT)
                    .put(
                        "inherited_by_equivalent_inputs",
                        DEVICE_VERIFICATION_CURRENT &&
                            (
                                BuildConfig.VERSION_CODE != BuildConfig.CHATGPT_WEB_VERIFIED_APK_VERSION_CODE ||
                                    BuildConfig.VERSION_NAME != BuildConfig.CHATGPT_WEB_VERIFIED_APK_VERSION_NAME
                            ),
                    ),
            )
            .put("feature_count", FEATURES.size)
            .put(
                "summary",
                JSONObject()
                    .put(
                        "complete",
                        FEATURES.count { it.status == ImplementationStatus.COMPLETE },
                    )
                    .put(
                        "partial",
                        FEATURES.count { it.status == ImplementationStatus.PARTIAL },
                    )
                    .put(
                        "fallback_only",
                        FEATURES.count { it.status == ImplementationStatus.FALLBACK_ONLY },
                    )
                    .put("remaining", incomplete.size),
            )
            .put(
                "code_summary",
                JSONObject()
                    .put(
                        "implemented",
                        FEATURES.count { it.codeStatus == CodeStatus.IMPLEMENTED },
                    )
                    .put(
                        "partial",
                        FEATURES.count { it.codeStatus == CodeStatus.PARTIAL },
                    )
                    .put(
                        "official_fallback",
                        FEATURES.count { it.codeStatus == CodeStatus.OFFICIAL_FALLBACK },
                    )
                    .put("remaining", incompleteCode.size),
            )
            .put(
                "verification_summary",
                JSONObject()
                    .put(
                        "offline_verified",
                        FEATURES.count {
                            it.verificationStatus == VerificationStatus.OFFLINE_VERIFIED
                        },
                    )
                    .put(
                        "device_verified",
                        FEATURES.count {
                            it.verificationStatus == VerificationStatus.DEVICE_VERIFIED
                        },
                    )
                    .put(
                        "user_action_required",
                        FEATURES.count {
                            it.verificationStatus == VerificationStatus.USER_ACTION_REQUIRED
                        },
                    )
                    .put(
                        "deferred",
                        FEATURES.count { it.verificationStatus == VerificationStatus.DEFERRED },
                    )
                    .put(
                        "failed",
                        FEATURES.count { it.verificationStatus == VerificationStatus.FAILED },
                    )
                    // Compatibility aliases for v2 consumers. "verified" now means device proof.
                    .put(
                        "verified",
                        FEATURES.count {
                            it.verificationStatus == VerificationStatus.DEVICE_VERIFIED
                        },
                    )
                    .put(
                        "pending",
                        FEATURES.count {
                            it.verificationStatus in setOf(
                                VerificationStatus.OFFLINE_VERIFIED,
                                VerificationStatus.DEFERRED,
                                VerificationStatus.FAILED,
                            )
                        },
                    )
                    .put("remaining", pendingVerification.size),
            )
            .put("features", JSONArray(rows))
            .put("remaining_feature_ids", JSONArray(incomplete.map(Feature::id)))
            .put("remaining_code_feature_ids", JSONArray(incompleteCode.map(Feature::id)))
            .put(
                "pending_verification_feature_ids",
                JSONArray(pendingVerification.map(Feature::id)),
            )
    }

    fun ids(): Set<String> = FEATURES.mapTo(linkedSetOf(), Feature::id)

    private fun Int?.orZero(): Int = this ?: 0

    private val DEVICE_VERIFICATION_CASES = mapOf(
        "official_fullscreen_fallback" to "safe/read_only_surface",
        "native_chat_composer" to "reversible/send_probe",
        "streaming_and_stop" to "reversible/send_probe_with_stop",
        "conversation_context_paging" to "safe/read_only_surface",
        "conversation_history" to "safe/read_only_surface",
        "conversation_create_and_switch" to "reversible/send_probe",
        "model_selection" to "reversible/reversible_controls",
        "web_search" to "reversible/composer_controls",
        "rich_message_rendering" to "reversible/message_structure",
        "complex_output_rendering" to "reversible/message_structure",
        "message_action_context" to "safe/message_actions",
        "feature_navigation" to "safe/feature_pages",
        "disclosure_controls" to "reversible/reversible_controls",
        "official_change_detection" to "safe/read_only_surface",
        "stable_mcp_and_adb_controls" to "safe/read_only_surface",
        "session_continuity_and_recovery" to "safe/session_recovery",
    )

    private val FEATURES = listOf(
        feature(
            id = "official_authentication",
            group = "session",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_select_view", "state"),
            capabilityIds = setOf(ChatGptWebCapabilityId.GOOGLE_LOGIN_ENTRY),
            remainingGap = "identity_provider_completion_remains_user_driven",
        ),
        feature(
            id = "official_fullscreen_fallback",
            group = "session",
            delivery = Delivery.FULLSCREEN_OFFICIAL,
            mcpActions = listOf("chatgpt_select_view"),
        ),
        feature(
            id = "native_chat_composer",
            group = "chat",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("set_input_text", "send_input"),
            capabilityIds = setOf(ChatGptWebCapabilityId.DRAFT_SYNC),
            semantics = setOf("send"),
        ),
        feature(
            id = "streaming_and_stop",
            group = "chat",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("state", "chatgpt_stop_generation"),
            capabilityIds = setOf(ChatGptWebCapabilityId.STREAMING),
            semantics = setOf("stop"),
        ),
        feature(
            id = "conversation_context_paging",
            group = "chat",
            delivery = Delivery.MCP_ONLY,
            mcpActions = listOf("chatgpt_get_context"),
            capabilityIds = setOf(ChatGptWebCapabilityId.CURRENT_CONVERSATION),
        ),
        feature(
            id = "conversation_history",
            group = "history",
            mcpActions = listOf("chatgpt_get_conversations", "chatgpt_list_conversations"),
            capabilityIds = setOf(
                ChatGptWebCapabilityId.CONVERSATION_LIST,
                ChatGptWebCapabilityId.CONVERSATION_SEARCH,
            ),
            semantics = setOf("conversation", "search"),
        ),
        feature(
            id = "conversation_create_and_switch",
            group = "history",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_new_conversation", "chatgpt_open_conversation"),
            capabilityIds = setOf(ChatGptWebCapabilityId.NEW_CONVERSATION),
            semantics = setOf("new_conversation", "conversation"),
        ),
        feature(
            id = "model_selection",
            group = "composer",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf(
                "chatgpt_list_composer_options",
                "chatgpt_select_composer_option",
            ),
            capabilityIds = setOf(ChatGptWebCapabilityId.MODEL_SELECTOR),
            semantics = setOf("model"),
        ),
        feature(
            id = "attachment_lifecycle",
            group = "composer",
            status = ImplementationStatus.PARTIAL,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control", "chatgpt_remove_attachment"),
            capabilityIds = setOf(ChatGptWebCapabilityId.ATTACHMENTS),
            semantics = setOf("attachment"),
            remainingGap = "real_file_upload_and_reply_acceptance",
        ),
        feature(
            id = "composer_tools",
            group = "composer",
            status = ImplementationStatus.PARTIAL,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf(
                "chatgpt_list_composer_options",
                "chatgpt_select_composer_option",
            ),
            capabilityIds = setOf(ChatGptWebCapabilityId.COMPOSER_TOOLS),
            remainingGap = "tool_execution_end_to_end_acceptance",
        ),
        feature(
            id = "web_search",
            group = "composer",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf("search", "toggle"),
        ),
        feature(
            id = "dictation",
            group = "voice",
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_start_dictation", "chatgpt_invoke_control"),
            capabilityIds = setOf(ChatGptWebCapabilityId.DICTATION),
            semantics = setOf(ChatGptWebUiSemantics.DICTATION),
        ),
        feature(
            id = "realtime_voice",
            group = "voice",
            status = ImplementationStatus.FALLBACK_ONLY,
            codeGap = "native_realtime_voice_session",
            delivery = Delivery.FULLSCREEN_OFFICIAL,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control", "chatgpt_select_view"),
            semantics = setOf("voice_mode"),
            remainingGap = "native_realtime_voice_session",
        ),
        feature(
            id = "rich_message_rendering",
            group = "messages",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_get_context"),
            capabilityIds = setOf(ChatGptWebCapabilityId.RICH_TEXT),
            remainingGap = "all_rich_message_variants_device_acceptance",
        ),
        feature(
            id = "complex_output_rendering",
            group = "messages",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_get_context", "chatgpt_invoke_control"),
            capabilityIds = setOf(ChatGptWebCapabilityId.COMPLEX_OUTPUT),
            semantics = setOf("open_media", "sources", "reasoning_details"),
            remainingGap = "all_complex_output_variants_device_acceptance",
        ),
        feature(
            id = "message_copy",
            group = "messages",
            status = ImplementationStatus.PARTIAL,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            capabilityIds = setOf(ChatGptWebCapabilityId.MESSAGE_COPY),
            semantics = setOf("copy"),
            remainingGap = "clipboard_device_acceptance_without_content_logging",
        ),
        feature(
            id = "message_regenerate",
            group = "messages",
            status = ImplementationStatus.PARTIAL,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_regenerate_response"),
            capabilityIds = setOf(ChatGptWebCapabilityId.MESSAGE_REGENERATE),
            semantics = setOf("regenerate"),
            remainingGap = "regenerate_reply_device_acceptance",
        ),
        feature(
            id = "message_action_context",
            group = "messages",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_find_controls", "chatgpt_invoke_control"),
            semantics = setOf("more", "timestamp", "sources", "read_aloud", "branch"),
        ),
        feature(
            id = "message_actions",
            group = "messages",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf(
                "edit",
                "share",
                "feedback",
                "read_aloud",
                "branch",
                "delete",
            ),
            remainingGap = "destructive_and_external_message_actions_acceptance",
        ),
        feature(
            id = "feature_navigation",
            group = "navigation",
            mcpActions = listOf("chatgpt_get_navigation", "chatgpt_select_feature"),
            capabilityIds = setOf(ChatGptWebCapabilityId.FEATURE_NAVIGATION),
            semantics = setOf("navigation"),
        ),
        featurePage("projects", "project"),
        featurePage("tasks", "tasks"),
        featurePage("library", "library"),
        featurePage("gpts", "gpts"),
        featurePage("apps", "apps"),
        featurePage("settings", "settings"),
        feature(
            id = "account_menu",
            group = "account",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf("profile", "personalization", "help", "plan", "logout"),
            remainingGap = "logout_and_account_mutation_actions_remain_user_driven",
        ),
        feature(
            id = "conversation_management",
            group = "history",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf("conversation_files", "pin", "archive", "share", "delete"),
            remainingGap = "conversation_mutation_device_acceptance",
        ),
        feature(
            id = "adaptive_form_controls",
            group = "adaptation",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf(
                "chatgpt_invoke_control",
                "chatgpt_set_control_text",
                "chatgpt_set_control_selected",
                "chatgpt_select_control_choice",
                "chatgpt_set_control_slider",
                "chatgpt_set_control_expanded",
            ),
            semantics = setOf("text_input", "selection", "toggle", "slider", "confirm"),
            remainingGap = "official_feature_form_matrix_device_acceptance",
        ),
        feature(
            id = "disclosure_controls",
            group = "adaptation",
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_set_control_expanded", "chatgpt_find_controls"),
        ),
        feature(
            id = "official_change_detection",
            group = "adaptation",
            delivery = Delivery.MCP_ONLY,
            mcpActions = listOf("chatgpt_get_capability_matrix"),
        ),
        feature(
            id = "stable_mcp_and_adb_controls",
            group = "automation",
            delivery = Delivery.MCP_ONLY,
            mcpActions = listOf("state", "chatgpt_get_capability_matrix"),
        ),
        feature(
            id = "session_continuity_and_recovery",
            group = "recovery",
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_select_view", "state"),
        ),
        feature(
            id = "session_long_running_stability",
            group = "recovery",
            status = ImplementationStatus.PARTIAL,
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_select_view", "state"),
            remainingGap = "multi_hour_webview_and_network_stability_acceptance",
        ),
    )

    private fun featurePage(id: String, semantic: String): Feature = feature(
        id = id,
        group = "features",
        status = ImplementationStatus.PARTIAL,
        delivery = Delivery.ADAPTIVE_NATIVE,
        acceptance = Acceptance.INTERACTIVE_DEVICE,
        mcpActions = listOf("chatgpt_select_feature", "chatgpt_invoke_control"),
        semantics = setOf(semantic),
        remainingGap = "${id}_workflow_end_to_end_acceptance",
    )

    private fun feature(
        id: String,
        group: String,
        status: ImplementationStatus = ImplementationStatus.COMPLETE,
        codeStatus: CodeStatus? = null,
        verificationStatus: VerificationStatus? = null,
        delivery: Delivery = Delivery.DEDICATED_NATIVE,
        acceptance: Acceptance = Acceptance.READ_ONLY_DEVICE,
        mcpActions: List<String>,
        capabilityIds: Set<String> = emptySet(),
        semantics: Set<String> = emptySet(),
        codeGap: String? = null,
        remainingGap: String? = null,
    ): Feature {
        val resolvedCodeStatus = codeStatus ?: when (status) {
            ImplementationStatus.FALLBACK_ONLY -> CodeStatus.OFFICIAL_FALLBACK
            else -> CodeStatus.IMPLEMENTED
        }
        val resolvedVerificationStatus = verificationStatus ?: when {
            id in DEVICE_VERIFICATION_CASES &&
                DEVICE_VERIFICATION_CURRENT ->
                VerificationStatus.DEVICE_VERIFIED
            id in DEVICE_VERIFICATION_CASES -> VerificationStatus.DEFERRED
            acceptance == Acceptance.USER_DRIVEN_DEVICE ->
                VerificationStatus.USER_ACTION_REQUIRED
            else -> VerificationStatus.OFFLINE_VERIFIED
        }
        val resolvedVerificationGap = when (resolvedVerificationStatus) {
            VerificationStatus.DEVICE_VERIFIED -> null
            VerificationStatus.USER_ACTION_REQUIRED ->
                remainingGap ?: "supervised_device_acceptance_required"
            VerificationStatus.OFFLINE_VERIFIED ->
                remainingGap ?: "current_apk_device_acceptance_not_recorded"
            VerificationStatus.DEFERRED ->
                remainingGap ?: "adapter_or_chatgpt_web_inputs_changed_since_device_acceptance"
            VerificationStatus.FAILED ->
                remainingGap ?: "current_device_acceptance_failed"
        }
        return Feature(
            id = id,
            group = group,
            status = status,
            codeStatus = resolvedCodeStatus,
            verificationStatus = resolvedVerificationStatus,
            delivery = delivery,
            acceptance = acceptance,
            mcpActions = mcpActions,
            capabilityIds = capabilityIds,
            semantics = semantics,
            codeGap = codeGap,
            verificationGap = resolvedVerificationGap,
            verificationCase = DEVICE_VERIFICATION_CASES[id],
            remainingGap = remainingGap,
        )
    }

}
