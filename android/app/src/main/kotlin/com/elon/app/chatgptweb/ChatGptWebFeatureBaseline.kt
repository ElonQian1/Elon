package com.elon.app.chatgptweb

import com.elon.app.BuildConfig
import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebFeatureBaseline {
    const val VERSION = 9
    internal const val DEVICE_VERIFICATION_ADAPTER_VERSION = 125
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
        val composerOptionSemantics: Set<String> = emptySet(),
        val codeGap: String? = null,
        val verificationGap: String? = null,
        val verificationCase: String? = null,
        val discoveryCase: String? = null,
        val remainingGap: String? = null,
    )

    fun describe(
        snapshot: ChatGptWebSnapshot?,
        manifest: ChatGptWebUiManifest?,
        mode: ChatGptWebPresentationMode,
        composerOptions: Collection<ChatGptWebComposerOption> = emptyList(),
        verificationEvidence: ChatGptWebVerificationEvidenceStore.Snapshot =
            ChatGptWebVerificationEvidenceStore.Snapshot.EMPTY,
    ): JSONObject {
        val advertised = snapshot?.capabilities?.supported.orEmpty()
        val semantics = manifest?.controls.orEmpty()
            .groupingBy(ChatGptWebUiControl::semantic)
            .eachCount()
        val composerSemantics = composerOptions
            .groupingBy(ChatGptWebComposerOption::semantic)
            .eachCount()
        val resolvedFeatures = FEATURES.map { feature ->
            feature.withVerificationEvidence(verificationEvidence)
        }
        val rows = resolvedFeatures.map { feature ->
            val discovery = ChatGptWebDiscoveryEvidence.resolve(
                feature.discoveryCase,
                verificationEvidence,
            )
            val currentPageObserved = when (feature.id) {
                "official_authentication" -> snapshot?.authenticated == true
                "anonymous_chat_access" -> snapshot?.let {
                    !it.authenticated && ChatGptWebAccessPolicy.canChat(it)
                } == true
                "official_fullscreen_fallback" -> mode == ChatGptWebPresentationMode.WEB
                "single_webview_skin" -> mode == ChatGptWebPresentationMode.SKIN
                "native_chat_composer" -> snapshot?.composerReady == true
                "disclosure_controls" -> manifest?.controls.orEmpty()
                    .any(ChatGptWebUiControl::supportsExpandedState)
                else -> feature.capabilityIds.any(advertised::contains) ||
                    feature.semantics.any { semantics[it].orZero() > 0 } ||
                    feature.composerOptionSemantics.any {
                        composerSemantics[it].orZero() > 0
                    }
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
                .put("composer_option_semantics", JSONArray(feature.composerOptionSemantics))
                .put("code_gap", feature.codeGap ?: JSONObject.NULL)
                .put("verification_case", feature.verificationCase ?: JSONObject.NULL)
                .put(
                    "verification_evidence_mode",
                    ChatGptWebAcceptanceCaseCatalog.evidenceMode(feature.verificationCase),
                )
                .put("discovery_case", feature.discoveryCase ?: JSONObject.NULL)
                .put("discovery_status", discovery.status)
                .put("discovery_gap", discovery.gap ?: JSONObject.NULL)
                .put(
                    "verification_gap",
                    feature.verificationGap ?: JSONObject.NULL,
                )
                .put(
                    "remaining_gap",
                    feature.remainingGap ?: JSONObject.NULL,
                )
        }
        val incomplete = resolvedFeatures.filter { it.status == ImplementationStatus.PARTIAL }
        val incompleteCode = resolvedFeatures.filter { it.codeStatus == CodeStatus.PARTIAL }
        val pendingVerification = resolvedFeatures.filter {
            it.verificationStatus != VerificationStatus.DEVICE_VERIFIED
        }
        return JSONObject()
            .put("schema", "elon.chatgpt_web.feature_baseline.v9")
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
            .put("verification_evidence", verificationEvidence.toJson())
            .put(
                "manual_only_verification_case_ids",
                JSONArray(ChatGptWebAcceptanceCaseCatalog.manualOnlyCaseIds()),
            )
            .put("feature_count", resolvedFeatures.size)
            .put(
                "summary",
                JSONObject()
                    .put(
                        "complete",
                        resolvedFeatures.count { it.status == ImplementationStatus.COMPLETE },
                    )
                    .put(
                        "partial",
                        resolvedFeatures.count { it.status == ImplementationStatus.PARTIAL },
                    )
                    .put(
                        "fallback_only",
                        resolvedFeatures.count { it.status == ImplementationStatus.FALLBACK_ONLY },
                    )
                    .put("remaining", incomplete.size),
            )
            .put(
                "code_summary",
                JSONObject()
                    .put(
                        "implemented",
                        resolvedFeatures.count { it.codeStatus == CodeStatus.IMPLEMENTED },
                    )
                    .put(
                        "partial",
                        resolvedFeatures.count { it.codeStatus == CodeStatus.PARTIAL },
                    )
                    .put(
                        "official_fallback",
                        resolvedFeatures.count { it.codeStatus == CodeStatus.OFFICIAL_FALLBACK },
                    )
                    .put("remaining", incompleteCode.size),
            )
            .put(
                "verification_summary",
                JSONObject()
                    .put(
                        "offline_verified",
                            resolvedFeatures.count {
                            it.verificationStatus == VerificationStatus.OFFLINE_VERIFIED
                        },
                    )
                    .put(
                        "device_verified",
                            resolvedFeatures.count {
                            it.verificationStatus == VerificationStatus.DEVICE_VERIFIED
                        },
                    )
                    .put(
                        "user_action_required",
                            resolvedFeatures.count {
                            it.verificationStatus == VerificationStatus.USER_ACTION_REQUIRED
                        },
                    )
                    .put(
                        "deferred",
                        resolvedFeatures.count { it.verificationStatus == VerificationStatus.DEFERRED },
                    )
                    .put(
                        "failed",
                        resolvedFeatures.count { it.verificationStatus == VerificationStatus.FAILED },
                    )
                    // Compatibility aliases for v2 consumers. "verified" now means device proof.
                    .put(
                        "verified",
                            resolvedFeatures.count {
                            it.verificationStatus == VerificationStatus.DEVICE_VERIFIED
                        },
                    )
                    .put(
                        "pending",
                            resolvedFeatures.count {
                            it.verificationStatus in setOf(
                                VerificationStatus.OFFLINE_VERIFIED,
                                VerificationStatus.DEFERRED,
                                VerificationStatus.FAILED,
                            )
                        },
                    )
                    .put("remaining", pendingVerification.size),
            )
            .put(
                "discovery_summary",
                JSONObject()
                    .put("required", resolvedFeatures.count { it.discoveryCase != null })
                    .put(
                        "device_observed",
                        resolvedFeatures.count {
                            ChatGptWebDiscoveryEvidence.resolve(
                                it.discoveryCase,
                                verificationEvidence,
                            ).status == "device_observed"
                        },
                    )
                    .put(
                        "remaining",
                        resolvedFeatures.count {
                            it.discoveryCase != null &&
                                ChatGptWebDiscoveryEvidence.resolve(
                                    it.discoveryCase,
                                    verificationEvidence,
                                ).status != "device_observed"
                        },
                    ),
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

    fun verificationCaseIds(): Set<String> =
        ChatGptWebAcceptanceCaseCatalog.verificationCaseIds()

    fun evidenceCaseIds(): Set<String> = ChatGptWebAcceptanceCaseCatalog.evidenceCaseIds()

    private fun Feature.withVerificationEvidence(
        evidence: ChatGptWebVerificationEvidenceStore.Snapshot,
    ): Feature {
        val caseId = verificationCase ?: return this
        val record = evidence.records[caseId]
        if (record?.current == true) {
            return copy(
                verificationStatus = VerificationStatus.DEVICE_VERIFIED,
                verificationGap = null,
            )
        }
        val gap = if (record == null) {
            "device_acceptance_not_recorded_for_current_case_input"
        } else {
            "verification_case_inputs_changed_since_device_acceptance"
        }
        return copy(
            verificationStatus = if (acceptance == Acceptance.USER_DRIVEN_DEVICE) {
                VerificationStatus.USER_ACTION_REQUIRED
            } else {
                VerificationStatus.DEFERRED
            },
            verificationGap = gap,
        )
    }

    private fun Int?.orZero(): Int = this ?: 0

    private val FEATURES = listOf(
        feature(
            id = "official_authentication",
            group = "session",
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_select_view", "state"),
            capabilityIds = setOf(ChatGptWebCapabilityId.GOOGLE_LOGIN_ENTRY),
            verificationGap = "identity_provider_completion_remains_user_driven",
        ),
        feature(
            id = "anonymous_chat_access",
            group = "session",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("state", "set_input_text", "send_input"),
            capabilityIds = setOf(ChatGptWebCapabilityId.DRAFT_SYNC),
            semantics = setOf("send"),
            verificationGap = "anonymous_isolated_send_and_reply_acceptance",
        ),
        feature(
            id = "official_fullscreen_fallback",
            group = "session",
            delivery = Delivery.FULLSCREEN_OFFICIAL,
            mcpActions = listOf("chatgpt_select_view"),
        ),
        feature(
            id = "single_webview_skin",
            group = "session",
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            mcpActions = listOf("chatgpt_select_view", "state"),
            verificationGap = "single_webview_skin_mode_device_acceptance",
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
            id = "temporary_chat",
            group = "history",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_find_controls", "chatgpt_invoke_control"),
            semantics = setOf("temporary_chat"),
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
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf(
                "stage_chatgpt_web_acceptance_attachment",
                "remove_chatgpt_web_acceptance_attachment",
                "send_input",
            ),
            capabilityIds = setOf(ChatGptWebCapabilityId.ATTACHMENTS),
            semantics = setOf("attachment"),
            verificationGap = "native_fixture_upload_and_reply_acceptance",
        ),
        feature(
            id = "composer_tools",
            group = "composer",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf(
                "chatgpt_list_composer_options",
                "chatgpt_select_composer_option",
            ),
            capabilityIds = setOf(ChatGptWebCapabilityId.COMPOSER_TOOLS),
            verificationGap = "tool_execution_end_to_end_acceptance",
        ),
        feature(
            id = "web_search",
            group = "composer",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf(
                "chatgpt_list_composer_options",
                "chatgpt_select_composer_option",
            ),
            semantics = setOf("search", "toggle"),
            composerOptionSemantics = setOf(ChatGptWebComposerOptionSemantics.WEB_SEARCH),
        ),
        composerTool("deep_research", ChatGptWebComposerOptionSemantics.DEEP_RESEARCH),
        composerTool("image_generation", ChatGptWebComposerOptionSemantics.IMAGE_GENERATION),
        composerTool("canvas", ChatGptWebComposerOptionSemantics.CANVAS),
        composerTool("study_mode", ChatGptWebComposerOptionSemantics.STUDY),
        composerTool(
            "agent_mode",
            ChatGptWebComposerOptionSemantics.AGENT,
            Acceptance.USER_DRIVEN_DEVICE,
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
            codeStatus = CodeStatus.OFFICIAL_FALLBACK,
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_start_realtime_voice", "chatgpt_select_view"),
            semantics = setOf("voice_mode"),
            verificationGap = "official_realtime_voice_round_trip_acceptance",
        ),
        feature(
            id = "rich_message_rendering",
            group = "messages",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_get_context"),
            capabilityIds = setOf(ChatGptWebCapabilityId.RICH_TEXT),
            verificationGap = "all_rich_message_variants_device_acceptance",
        ),
        feature(
            id = "complex_output_rendering",
            group = "messages",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_get_context", "chatgpt_invoke_control"),
            capabilityIds = setOf(ChatGptWebCapabilityId.COMPLEX_OUTPUT),
            semantics = setOf("open_media", "sources", "reasoning_details"),
            verificationGap = "all_complex_output_variants_device_acceptance",
        ),
        feature(
            id = "message_copy",
            group = "messages",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_copy_last_response"),
            capabilityIds = setOf(ChatGptWebCapabilityId.MESSAGE_COPY),
            semantics = setOf("copy"),
        ),
        feature(
            id = "message_regenerate",
            group = "messages",
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_regenerate_response"),
            capabilityIds = setOf(ChatGptWebCapabilityId.MESSAGE_REGENERATE),
            semantics = setOf("regenerate"),
        ),
        feature(
            id = "message_action_context",
            group = "messages",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_find_controls", "chatgpt_invoke_control"),
            semantics = setOf("more", "timestamp", "sources", "read_aloud", "branch", "save_to_project"),
        ),
        feature(
            id = "message_actions",
            group = "messages",
            delivery = Delivery.ADAPTIVE_NATIVE,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf(
                "edit",
                "share",
                "feedback",
                "read_aloud",
                "branch",
                "save_to_project",
                "delete",
            ),
            verificationGap = "destructive_and_external_message_actions_acceptance",
        ),
        feature(
            id = "feature_navigation",
            group = "navigation",
            mcpActions = listOf(
                "chatgpt_get_navigation",
                "chatgpt_dismiss_features",
                "chatgpt_select_feature",
            ),
            capabilityIds = setOf(ChatGptWebCapabilityId.FEATURE_NAVIGATION),
            semantics = setOf("navigation"),
        ),
        featurePage("projects", "project"),
        featurePage("tasks", "tasks"),
        featurePage("images", "images"),
        featurePage("library", "library"),
        featurePage("gpts", "gpts"),
        featurePage("apps", "apps"),
        featurePage("settings", "settings"),
        featurePage("health", "health", Acceptance.USER_DRIVEN_DEVICE),
        featurePage("finances", "finances", Acceptance.USER_DRIVEN_DEVICE),
        featurePage("work", "work"),
        feature(
            id = "account_menu",
            group = "account",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf("profile", "personalization", "help", "plan", "logout"),
            verificationGap = "account_menu_structure_device_acceptance",
        ),
        feature(
            id = "account_mutations",
            group = "account",
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control", "chatgpt_select_view"),
            semantics = setOf("personalization", "plan", "logout"),
            verificationGap = "logout_and_account_mutation_actions_remain_user_driven",
        ),
        feature(
            id = "conversation_management",
            group = "history",
            delivery = Delivery.ADAPTIVE_NATIVE,
            mcpActions = listOf("chatgpt_invoke_control"),
            semantics = setOf(
                "conversation_files",
                "rename",
                "pin",
                "archive",
                "share",
                "delete",
            ),
            verificationGap = "conversation_menu_structure_device_acceptance",
        ),
        feature(
            id = "conversation_mutations",
            group = "history",
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.USER_DRIVEN_DEVICE,
            mcpActions = listOf("chatgpt_invoke_control", "chatgpt_select_view"),
            semantics = setOf("rename", "pin", "archive", "share", "delete"),
            verificationGap = "conversation_mutation_device_acceptance",
        ),
        feature(
            id = "adaptive_form_controls",
            group = "adaptation",
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
            verificationGap = "official_feature_form_matrix_device_acceptance",
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
            delivery = Delivery.OFFICIAL_WEB_WITH_NATIVE_ENTRY,
            acceptance = Acceptance.INTERACTIVE_DEVICE,
            mcpActions = listOf("chatgpt_select_view", "state"),
            verificationGap = "multi_hour_webview_and_network_stability_acceptance",
        ),
    )

    private fun featurePage(
        id: String,
        semantic: String,
        acceptance: Acceptance = Acceptance.INTERACTIVE_DEVICE,
    ): Feature = feature(
        id = id,
        group = "features",
        delivery = Delivery.ADAPTIVE_NATIVE,
        acceptance = acceptance,
        mcpActions = listOf("chatgpt_select_feature", "chatgpt_invoke_control"),
        semantics = setOf(semantic),
        verificationGap = "${id}_workflow_end_to_end_acceptance",
    )

    private fun composerTool(
        id: String,
        semantic: String,
        acceptance: Acceptance = Acceptance.INTERACTIVE_DEVICE,
    ): Feature = feature(
        id = id,
        group = "composer",
        delivery = Delivery.DEDICATED_NATIVE,
        acceptance = acceptance,
        mcpActions = listOf(
            "chatgpt_list_composer_options",
            "chatgpt_select_composer_option",
        ),
        composerOptionSemantics = setOf(semantic),
        discoveryCase = ChatGptWebAcceptanceCaseCatalog.discoveryCase(id),
        verificationGap = "${id}_end_to_end_device_acceptance",
    )

    private fun feature(
        id: String,
        group: String,
        codeStatus: CodeStatus? = null,
        verificationStatus: VerificationStatus? = null,
        delivery: Delivery = Delivery.DEDICATED_NATIVE,
        acceptance: Acceptance = Acceptance.READ_ONLY_DEVICE,
        mcpActions: List<String>,
        capabilityIds: Set<String> = emptySet(),
        semantics: Set<String> = emptySet(),
        composerOptionSemantics: Set<String> = emptySet(),
        codeGap: String? = null,
        discoveryCase: String? = null,
        verificationGap: String? = null,
    ): Feature {
        val resolvedCodeStatus = codeStatus ?: CodeStatus.IMPLEMENTED
        val resolvedImplementationStatus = when (resolvedCodeStatus) {
            CodeStatus.IMPLEMENTED -> ImplementationStatus.COMPLETE
            CodeStatus.PARTIAL -> ImplementationStatus.PARTIAL
            CodeStatus.OFFICIAL_FALLBACK -> ImplementationStatus.FALLBACK_ONLY
        }
        val resolvedVerificationStatus = verificationStatus ?: when {
            acceptance == Acceptance.USER_DRIVEN_DEVICE ->
                VerificationStatus.USER_ACTION_REQUIRED
            ChatGptWebAcceptanceCaseCatalog.verificationCase(id) != null &&
                DEVICE_VERIFICATION_CURRENT ->
                VerificationStatus.DEVICE_VERIFIED
            ChatGptWebAcceptanceCaseCatalog.verificationCase(id) != null ->
                VerificationStatus.DEFERRED
            else -> VerificationStatus.OFFLINE_VERIFIED
        }
        val resolvedVerificationGap = when (resolvedVerificationStatus) {
            VerificationStatus.DEVICE_VERIFIED -> null
            VerificationStatus.USER_ACTION_REQUIRED ->
                verificationGap ?: "supervised_device_acceptance_required"
            VerificationStatus.OFFLINE_VERIFIED ->
                verificationGap ?: "current_apk_device_acceptance_not_recorded"
            VerificationStatus.DEFERRED ->
                verificationGap ?: "adapter_or_chatgpt_web_inputs_changed_since_device_acceptance"
            VerificationStatus.FAILED ->
                verificationGap ?: "current_device_acceptance_failed"
        }
        return Feature(
            id = id,
            group = group,
            status = resolvedImplementationStatus,
            codeStatus = resolvedCodeStatus,
            verificationStatus = resolvedVerificationStatus,
            delivery = delivery,
            acceptance = acceptance,
            mcpActions = mcpActions,
            capabilityIds = capabilityIds,
            semantics = semantics,
            composerOptionSemantics = composerOptionSemantics,
            codeGap = codeGap,
            verificationGap = resolvedVerificationGap,
            verificationCase = ChatGptWebAcceptanceCaseCatalog.verificationCase(id),
            discoveryCase = discoveryCase,
            remainingGap = if (resolvedCodeStatus == CodeStatus.PARTIAL) codeGap else null,
        )
    }

}
