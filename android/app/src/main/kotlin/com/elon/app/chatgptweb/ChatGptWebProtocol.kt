package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession

import org.json.JSONObject

internal data class ChatGptWebMessage(
    val id: String,
    val role: String,
    val content: String,
    val state: String,
    val parts: List<ChatGptWebMessagePart>,
)

internal data class ChatGptWebAttachment(
    val id: String,
    val name: String,
    val state: String,
    val removable: Boolean,
)

internal data class ChatGptWebSnapshot(
    val title: String,
    val url: String,
    val draft: String,
    val messages: List<ChatGptWebMessage>,
    val authenticated: Boolean,
    val composerReady: Boolean,
    val streaming: Boolean,
    val privateStreamObserved: Boolean = false,
    val privateStreamRevision: Long = 0L,
    val privateStreamState: String = "idle",
    val currentModel: String,
    val attachments: List<ChatGptWebAttachment>,
    val dictationActive: Boolean,
    val capabilities: ChatGptWebCapabilities,
    val pageKind: String = "unknown",
    val loginRequired: Boolean = false,
    val accessReason: String = "",
    val accessSource: String = "",
    val messageWindowStart: Int = 0,
    val observedMessageCount: Int = messages.size,
    val dictationCaptureActive: Boolean = false,
    val dictationCapturePending: Boolean = false,
)

internal data class ChatGptWebComposerOption(
    val id: String,
    val label: String,
    val selected: Boolean,
    val kind: String,
    val semantic: String = ChatGptWebComposerOptionSemantics.TOOL,
    val opensSubmenu: Boolean = false,
    val parentId: String? = null,
    val parentLabel: String? = null,
)

internal data class ChatGptWebFeature(
    val id: String,
    val label: String,
    val kind: String,
    val selected: Boolean,
)

internal data class ChatGptWebConversationCollection(
    val scrollerFound: Boolean = false,
    val scrolled: Boolean = false,
    val scrollRestored: Boolean = true,
    val reachedEnd: Boolean = false,
    val truncated: Boolean = false,
    val timedOut: Boolean = false,
    val observedCount: Int = 0,
    val steps: Int = 0,
    val source: String = SOURCE_NONE,
    val stale: Boolean = false,
    val officialLoadState: String = LOAD_IDLE,
    val cachedAtMs: Long = 0L,
) {
    val isComplete: Boolean
        get() = reachedEnd && !truncated && !timedOut

    companion object {
        const val SOURCE_NONE = "none"
        const val SOURCE_OFFICIAL = "official_dom"
        const val SOURCE_PRIVATE = "official_private"
        const val SOURCE_CACHE = "local_cache"
        const val LOAD_IDLE = "idle"
        const val LOAD_LOADING = "loading"
        const val LOAD_READY = "ready"
        const val LOAD_FAILED = "failed"

        fun official(count: Int): ChatGptWebConversationCollection =
            ChatGptWebConversationCollection(
                observedCount = count,
                source = SOURCE_OFFICIAL,
                officialLoadState = LOAD_READY,
            )

        fun acceptedOfficialSource(value: String): String =
            if (value == SOURCE_PRIVATE) SOURCE_PRIVATE else SOURCE_OFFICIAL

        fun cached(count: Int, savedAtMs: Long): ChatGptWebConversationCollection =
            ChatGptWebConversationCollection(
                observedCount = count,
                source = SOURCE_CACHE,
                stale = true,
                officialLoadState = LOAD_IDLE,
                cachedAtMs = savedAtMs,
            )
    }
}

internal sealed interface ChatGptWebEvent {
    data class AdapterReady(
        val capabilities: ChatGptWebCapabilities,
    ) : ChatGptWebEvent

    data class Snapshot(val value: ChatGptWebSnapshot) : ChatGptWebEvent

    data class ConversationList(
        val conversations: List<ChatGptWebConversation>,
        val collection: ChatGptWebConversationCollection =
            ChatGptWebConversationCollection.official(conversations.size),
        val projects: List<ChatGptWebProject> = emptyList(),
        val scopeProjectId: String? = null,
    ) : ChatGptWebEvent

    data class ComposerControls(
        val section: String,
        val currentModel: String,
        val options: List<ChatGptWebComposerOption>,
    ) : ChatGptWebEvent

    data class FeatureNavigation(
        val features: List<ChatGptWebFeature>,
    ) : ChatGptWebEvent

    data class UiManifest(
        val value: ChatGptWebUiManifest,
    ) : ChatGptWebEvent

    data class WebTouchRequest(
        val purpose: String,
        val xRatio: Double,
        val yRatio: Double,
        val controlId: String? = null,
    ) : ChatGptWebEvent

    data class AttachmentTransport(
        val evidence: ChatGptWebAttachmentTransportEvidence,
    ) : ChatGptWebEvent

    data class ImageAsset(
        val value: ChatGptWebImageAsset,
    ) : ChatGptWebEvent

    data class ImageGallerySnapshot(
        val value: ChatGptWebImageGallerySnapshot,
    ) : ChatGptWebEvent

    data class CommandResult(
        val action: String,
        val ok: Boolean,
        val detail: String,
        val requestId: String? = null,
    ) : ChatGptWebEvent
}

internal data class ChatGptWebProtocolMessage(
    val documentToken: String?,
    val event: ChatGptWebEvent,
)

internal object ChatGptWebProtocol {
    fun parse(rawPayload: String, minimumAdapterVersion: Int = 0): ChatGptWebEvent? =
        parseMessage(rawPayload, minimumAdapterVersion)?.event

    fun parseMessage(
        rawPayload: String,
        minimumAdapterVersion: Int = 0,
    ): ChatGptWebProtocolMessage? {
        val payload = runCatching { JSONObject(rawPayload) }.getOrNull() ?: return null
        if (payload.optInt("adapterVersion", 0) < minimumAdapterVersion) return null
        val parsedEvent = (if (payload.optString("schema") == SCHEMA) {
            val event = payload.optJSONObject("event") ?: return null
            when (event.optString("type")) {
                "adapter_ready" -> ChatGptWebEvent.AdapterReady(
                    ChatGptWebCapabilities(parseStringSet(event, "capabilities")),
                )
                "message_snapshot" -> ChatGptWebEvent.Snapshot(parseSnapshot(event))
                "conversation_snapshot" -> {
                    val conversations = parseConversations(event)
                    ChatGptWebEvent.ConversationList(
                        conversations = conversations,
                        projects = parseProjects(event),
                        collection = parseConversationCollection(event, conversations.size),
                        scopeProjectId = ChatGptWebConversationPath.canonicalProjectId(
                            event.optString("scopeProjectId"),
                        ),
                    )
                }
                "composer_controls_snapshot" -> parseComposerControls(event)
                "navigation_snapshot" -> ChatGptWebEvent.FeatureNavigation(parseFeatures(event))
                "ui_manifest_snapshot" -> ChatGptWebEvent.UiManifest(parseUiManifest(event))
                "web_touch_request" -> parseWebTouchRequest(event)
                "attachment_transport" -> parseAttachmentTransport(event)
                "image_asset" -> ChatGptWebImageAssetProtocol.parseAsset(event)
                    ?.let { ChatGptWebEvent.ImageAsset(it) }
                "image_gallery_snapshot" -> ChatGptWebImageAssetProtocol.parseGallery(event)
                    ?.let { ChatGptWebEvent.ImageGallerySnapshot(it) }
                else -> null
            }
        } else when (payload.optString("type")) {
            "command_result" -> ChatGptWebEvent.CommandResult(
                action = payload.optString("action").take(40),
                ok = payload.optBoolean("ok"),
                detail = payload.optString("detail").take(160),
                requestId = payload.optString("requestId")
                    .take(MAX_REQUEST_ID_LENGTH)
                    .takeIf(REQUEST_ID::matches),
            )
            else -> null
        }) ?: return null
        return ChatGptWebProtocolMessage(
            documentToken = payload.optString("documentToken")
                .takeIf(WebBridgeDocumentSession.DOCUMENT_TOKEN::matches),
            event = parsedEvent,
        )
    }

    private fun parseAttachmentTransport(event: JSONObject): ChatGptWebEvent.AttachmentTransport? {
        val evidence = ChatGptWebAttachmentTransportEvidence(
            version = event.optInt("transportVersion", 0),
            sequence = event.optLong("sequence", 0L),
            state = ChatGptWebAttachmentTransportState.fromWireValue(event.optString("state"))
                ?: return null,
            completedCount = event.optInt("completedCount", -1),
        )
        return evidence.takeIf(ChatGptWebAttachmentTransportEvidence::supported)
            ?.let { ChatGptWebEvent.AttachmentTransport(it) }
    }

    private fun parseSnapshot(event: JSONObject): ChatGptWebSnapshot {
        val rawMessages = event.optJSONArray("messages")
        val messages = buildList {
            if (rawMessages == null) return@buildList
            for (index in 0 until minOf(rawMessages.length(), MAX_MESSAGES)) {
                val item = rawMessages.optJSONObject(index) ?: continue
                val role = item.optString("role").lowercase()
                if (role !in SUPPORTED_ROLES) continue
                val content = textContent(item).trim().take(MAX_MESSAGE_LENGTH)
                val parts = ChatGptWebMessagePartParser.parse(item)
                if (content.isEmpty() && parts.isEmpty()) continue
                add(
                    ChatGptWebMessage(
                        id = item.optString("id").ifBlank { "$role-$index" }.take(160),
                        role = role,
                        content = content.ifBlank { parts.joinToString("\n", transform = ChatGptWebMessagePart::label) },
                        state = item.optString("state").takeIf { it in SUPPORTED_MESSAGE_STATES }
                            ?: "completed",
                        parts = parts,
                    ),
                )
            }
        }
        val messageWindowStart = event.optInt("messageWindowStart", 0)
            .coerceIn(0, MAX_OBSERVED_MESSAGES - messages.size)
        val observedMessageCount = event.optInt(
            "observedMessageCount",
            messageWindowStart + messages.size,
        ).coerceIn(messageWindowStart + messages.size, MAX_OBSERVED_MESSAGES)
        return ChatGptWebSnapshot(
            title = event.optString("title").trim().take(120),
            url = event.optString("url").take(2_048),
            draft = event.optString("draft").take(MAX_DRAFT_LENGTH),
            messages = messages,
            authenticated = event.optBoolean("authenticated"),
            composerReady = event.optBoolean("composerReady"),
            streaming = event.optBoolean("streaming"),
            privateStreamObserved = event.optBoolean("privateStreamObserved"),
            privateStreamRevision = event.optLong("privateStreamRevision")
                .coerceIn(0L, MAX_PRIVATE_STREAM_REVISION),
            privateStreamState = event.optString("privateStreamState")
                .takeIf { it in PRIVATE_STREAM_STATES }
                ?: "idle",
            currentModel = event.optString("currentModel").trim().take(MAX_MODEL_LABEL_LENGTH),
            attachments = parseAttachments(event),
            dictationActive = event.optBoolean("dictationActive"),
            capabilities = ChatGptWebCapabilities(parseStringSet(event, "capabilities")),
            pageKind = event.optString("pageKind").takeIf { it in PAGE_KINDS } ?: "unknown",
            loginRequired = event.optBoolean("loginRequired"),
            accessReason = event.optString("accessReason").takeIf { it in ACCESS_REASONS }.orEmpty(),
            accessSource = event.optString("accessSource").takeIf { it in ACCESS_SOURCES }.orEmpty(),
            messageWindowStart = messageWindowStart,
            observedMessageCount = observedMessageCount,
            dictationCaptureActive = event.optBoolean("dictationCaptureActive"),
            dictationCapturePending = event.optBoolean("dictationCapturePending"),
        )
    }

    private fun parseAttachments(event: JSONObject): List<ChatGptWebAttachment> {
        val values = event.optJSONArray("attachments") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(values.length(), MAX_ATTACHMENTS)) {
                val item = values.optJSONObject(index) ?: continue
                val id = item.optString("id").take(MAX_ATTACHMENT_ID_LENGTH)
                val name = item.optString("name").trim().take(MAX_ATTACHMENT_NAME_LENGTH)
                val state = item.optString("state").takeIf { it in ATTACHMENT_STATES } ?: "ready"
                if (!ATTACHMENT_ID.matches(id) || name.isBlank()) continue
                add(
                    ChatGptWebAttachment(
                        id = id,
                        name = name,
                        state = state,
                        removable = item.optBoolean("removable"),
                    ),
                )
            }
        }
    }

    private fun parseComposerControls(event: JSONObject): ChatGptWebEvent.ComposerControls? {
        val section = event.optString("section")
        if (section !in SUPPORTED_COMPOSER_SECTIONS) return null
        val rawOptions = event.optJSONArray("options")
        val options = buildList {
            if (rawOptions == null) return@buildList
            for (index in 0 until minOf(rawOptions.length(), MAX_COMPOSER_OPTIONS)) {
                val item = rawOptions.optJSONObject(index) ?: continue
                val id = item.optString("id").take(MAX_OPTION_ID_LENGTH)
                val label = item.optString("label").trim().take(MAX_OPTION_LABEL_LENGTH)
                if (!OPTION_ID.matches(id) || label.isBlank()) continue
                val parent = item.optJSONObject("parentOption")
                val parentId = parent?.optString("id")
                    ?.take(MAX_OPTION_ID_LENGTH)
                    ?.takeIf(OPTION_ID::matches)
                val parentLabel = parent?.optString("label")
                    ?.trim()
                    ?.take(MAX_OPTION_LABEL_LENGTH)
                    ?.takeIf { parentId != null && it.isNotBlank() }
                add(
                    ChatGptWebComposerOption(
                        id = id,
                        label = label,
                        selected = item.optBoolean("selected"),
                        kind = item.optString("kind").take(MAX_OPTION_KIND_LENGTH),
                        semantic = item.optString("semantic")
                            .takeIf { it in ChatGptWebComposerOptionSemantics.KNOWN }
                            ?: ChatGptWebComposerOptionSemantics.fallback(section),
                        opensSubmenu = item.optBoolean("opensSubmenu"),
                        parentId = parentId,
                        parentLabel = parentLabel,
                    ),
                )
            }
        }
        return ChatGptWebEvent.ComposerControls(
            section = section,
            currentModel = event.optString("currentModel").trim().take(MAX_MODEL_LABEL_LENGTH),
            options = options,
        )
    }

    private fun parseFeatures(event: JSONObject): List<ChatGptWebFeature> {
        val values = event.optJSONArray("features") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(values.length(), MAX_FEATURES)) {
                val item = values.optJSONObject(index) ?: continue
                val id = item.optString("id").take(MAX_FEATURE_ID_LENGTH)
                val label = item.optString("label").trim().take(MAX_FEATURE_LABEL_LENGTH)
                val kind = item.optString("kind").takeIf { it in FEATURE_KINDS } ?: "navigation"
                if (!FEATURE_ID.matches(id) || label.isBlank()) continue
                add(
                    ChatGptWebFeature(
                        id = id,
                        label = label,
                        kind = kind,
                        selected = item.optBoolean("selected"),
                    ),
                )
            }
        }
    }

    private fun parseWebTouchRequest(event: JSONObject): ChatGptWebEvent.WebTouchRequest? {
        val purpose = event.optString("purpose")
        if (purpose !in SUPPORTED_TOUCH_PURPOSES) return null
        val xRatio = event.optDouble("xRatio", Double.NaN)
        val yRatio = event.optDouble("yRatio", Double.NaN)
        if (!xRatio.isFinite() || !yRatio.isFinite()) return null
        if (xRatio !in 0.0..1.0 || yRatio !in 0.0..1.0) return null
        val controlId = event.optString("controlId")
            .takeIf { it.isNotBlank() && UI_CONTROL_ID.matches(it) }
        if (purpose == "invoke_ui_control" && controlId == null) return null
        return ChatGptWebEvent.WebTouchRequest(purpose, xRatio, yRatio, controlId)
    }

    private fun parseUiManifest(event: JSONObject): ChatGptWebUiManifest {
        val rawControls = event.optJSONArray("controls")
        val controls = buildList {
            if (rawControls == null) return@buildList
            for (index in 0 until minOf(rawControls.length(), MAX_UI_CONTROLS)) {
                val item = rawControls.optJSONObject(index) ?: continue
                val id = item.optString("id").take(MAX_UI_CONTROL_ID_LENGTH)
                val label = item.optString("label").trim().take(MAX_UI_CONTROL_LABEL_LENGTH)
                val region = item.optString("region").takeIf { it in UI_REGIONS } ?: continue
                val semantic = normalizeControlSemantic(
                    item.optString("semantic"),
                    label,
                    region,
                )
                val role = item.optString("role").takeIf { it in UI_ROLES } ?: "button"
                val inputKind = item.optString("inputKind")
                    .takeIf { it.isNotBlank() && it in UI_INPUT_KINDS }
                val writable = item.optBoolean("writable") &&
                    role == "textbox" && inputKind != null && inputKind in WRITABLE_UI_INPUT_KINDS
                val stateSettable = item.optBoolean("stateSettable") && (
                    role in STATE_SETTABLE_UI_ROLES || semantic == "temporary_chat"
                    )
                val choiceLabels = buildList {
                    val rawChoices = item.optJSONArray("choiceLabels") ?: return@buildList
                    for (choiceIndex in 0 until minOf(rawChoices.length(), MAX_UI_CHOICE_OPTIONS)) {
                        val choiceLabel = rawChoices.optString(choiceIndex).trim()
                            .take(MAX_UI_CHOICE_LABEL_LENGTH)
                        add(choiceLabel.ifBlank { "选项 ${choiceIndex + 1}" })
                    }
                }
                val selectedChoiceIndex = item.optInt("selectedChoiceIndex", -1)
                    .takeIf { it in choiceLabels.indices }
                val slider = parseSlider(item, role, inputKind)
                val expanded = item.opt("expanded") as? Boolean
                val expandable = item.optBoolean("expandable") && expanded != null
                val contextId = item.optString("contextId")
                    .takeIf { it.isNotBlank() && UI_CONTEXT_ID.matches(it) }
                val webXRatio = boundedRatio(item, "xRatio")
                val webYRatio = boundedRatio(item, "yRatio")
                if (!UI_CONTROL_ID.matches(id) || label.isBlank()) continue
                add(
                    ChatGptWebUiControl(
                        id = id,
                        semantic = semantic,
                        label = label,
                        region = region,
                        role = role,
                        enabled = item.optBoolean("enabled", true),
                        selected = item.optBoolean("selected"),
                        inputKind = inputKind,
                        writable = writable,
                        stateSettable = stateSettable,
                        choiceLabels = choiceLabels,
                        selectedChoiceIndex = selectedChoiceIndex,
                        slider = slider,
                        expanded = expanded,
                        expandable = expandable,
                        contextId = contextId,
                        inViewport = item.optBoolean("inViewport", true),
                        webXRatio = webXRatio,
                        webYRatio = webYRatio,
                    ),
                )
            }
        }
        val rawControlCount = rawControls?.length() ?: 0
        val discoveredControlCount = event
            .optInt("discoveredControlCount", rawControlCount)
            .coerceAtLeast(rawControlCount)
            .coerceIn(controls.size, MAX_DISCOVERED_UI_CONTROLS)
        return ChatGptWebUiManifest(
            version = event.optInt("version", 1).coerceIn(1, MAX_UI_MANIFEST_VERSION),
            pageKind = event.optString("pageKind").takeIf { it in UI_PAGE_KINDS } ?: "unknown",
            title = event.optString("title").trim().take(MAX_TITLE_LENGTH),
            compatibility = event.optString("compatibility")
                .takeIf { it in UI_COMPATIBILITY } ?: "partial",
            controls = controls,
            discoveredControlCount = discoveredControlCount,
            controlsTruncated = event.optBoolean("controlsTruncated") ||
                rawControlCount > MAX_UI_CONTROLS ||
                discoveredControlCount > rawControlCount,
        )
    }

    private fun normalizeControlSemantic(raw: String, label: String, region: String): String {
        val semantic = raw.takeIf { it in ChatGptWebUiSemantics.KNOWN }
            ?: ChatGptWebUiSemantics.GENERIC_ACTION
        if (
            semantic == ChatGptWebUiSemantics.GENERIC_ACTION &&
            region == ChatGptWebUiRegion.COMPOSER &&
            WEB_SEARCH_LABEL.matches(label.trim())
        ) {
            return ChatGptWebUiSemantics.WEB_SEARCH
        }
        return semantic
    }

    private fun parseConversations(event: JSONObject): List<ChatGptWebConversation> {
        val items = event.optJSONArray("conversations") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(items.length(), MAX_CONVERSATIONS)) {
                val item = items.optJSONObject(index) ?: continue
                val path = ChatGptWebConversationPath.normalize(
                    item.optString("path").take(MAX_PATH_LENGTH),
                ) ?: continue
                val id = item.optString("id").ifBlank { path.substringAfterLast('/') }
                val title = item.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank()) continue
                val projectPath = ChatGptWebConversationPath.normalizeProject(
                    item.optString("projectPath").take(MAX_PATH_LENGTH),
                )
                add(
                    ChatGptWebConversationIndex.sanitize(ChatGptWebConversation(
                        id = id.take(MAX_ID_LENGTH),
                        title = title,
                        path = path,
                        active = item.optBoolean("active"),
                        groupLabel = item.optionalString("groupLabel")
                            .orEmpty()
                            .take(MAX_GROUP_LABEL_LENGTH),
                        projectId = item.optionalString("projectId")
                            ?.take(MAX_PROJECT_ID_LENGTH)
                            ?.takeIf(PROJECT_ID::matches)
                            ?: ChatGptWebConversationPath.projectId(path),
                        projectTitle = item.optionalString("projectTitle")
                            ?.take(MAX_TITLE_LENGTH),
                        projectPath = projectPath,
                        activityDates = parseActivityDates(item),
                        providerUrl = item.optionalString("providerUrl")
                            ?.take(MAX_PROVIDER_URL_LENGTH),
                    )),
                )
            }
        }
    }

    private fun parseProjects(event: JSONObject): List<ChatGptWebProject> {
        val items = event.optJSONArray("projects") ?: return emptyList()
        return buildList {
            val seen = mutableSetOf<String>()
            for (index in 0 until minOf(items.length(), MAX_PROJECTS)) {
                val item = items.optJSONObject(index) ?: continue
                val path = ChatGptWebConversationPath.normalizeProject(
                    item.optString("path").take(MAX_PATH_LENGTH),
                ) ?: continue
                val id = item.optString("id").trim().take(MAX_PROJECT_ID_LENGTH)
                    .takeIf(PROJECT_ID::matches)
                    ?: ChatGptWebConversationPath.projectId(path)
                    ?: continue
                val title = item.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank() || !seen.add(path)) continue
                add(ChatGptWebProject(id, title, path, item.optBoolean("active")))
            }
        }
    }

    private fun parseActivityDates(item: JSONObject): Set<String> = buildSet {
        item.optString("activityDate").takeIf(ACTIVITY_DATE::matches)?.let(::add)
        val values = item.optJSONArray("activityDates") ?: return@buildSet
        for (index in 0 until minOf(values.length(), MAX_ACTIVITY_DATES)) {
            values.optString(index).takeIf(ACTIVITY_DATE::matches)?.let(::add)
        }
    }

    private fun JSONObject.optionalString(key: String): String? =
        opt(key)
            ?.takeUnless { it == JSONObject.NULL }
            ?.toString()
            ?.trim()
            ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }

    private fun parseConversationCollection(
        event: JSONObject,
        conversationCount: Int,
    ): ChatGptWebConversationCollection {
        val collection = event.optJSONObject("collection")
            ?: return ChatGptWebConversationCollection.official(conversationCount)
        return ChatGptWebConversationCollection(
            scrollerFound = collection.optBoolean("scrollerFound"),
            scrolled = collection.optBoolean("scrolled"),
            scrollRestored = !collection.has("scrollRestored") || collection.optBoolean("scrollRestored"),
            reachedEnd = collection.optBoolean("reachedEnd"),
            truncated = collection.optBoolean("truncated"),
            timedOut = collection.optBoolean("timedOut"),
            observedCount = conversationCount,
            steps = collection.optInt("steps", 0).coerceIn(0, MAX_CONVERSATION_COLLECTION_STEPS),
            source = ChatGptWebConversationCollection.acceptedOfficialSource(
                collection.optString("source"),
            ),
            officialLoadState = ChatGptWebConversationCollection.LOAD_READY,
        )
    }

    private fun parseSlider(item: JSONObject, role: String, inputKind: String?): ChatGptWebSlider? {
        if (!item.optBoolean("sliderSettable") || role != "slider" || inputKind != "range") return null
        val min = item.optDouble("sliderMin", Double.NaN)
        val max = item.optDouble("sliderMax", Double.NaN)
        val step = item.optDouble("sliderStep", Double.NaN)
        val value = item.optDouble("sliderValue", Double.NaN)
        if (!listOf(min, max, step, value).all(Double::isFinite)) return null
        if (max <= min || step <= 0 || value !in min..max) return null
        val rawSteps = (max - min) / step
        val roundedSteps = kotlin.math.round(rawSteps)
        if (kotlin.math.abs(rawSteps - roundedSteps) > 1e-7 || roundedSteps !in 1.0..10_000.0) return null
        return ChatGptWebSlider(min, max, step, value)
    }

    private fun boundedRatio(value: JSONObject, key: String): Double? {
        val ratio = value.optDouble(key, Double.NaN)
        return ratio.takeIf { it.isFinite() && it in 0.0..1.0 }
    }

    private fun parseStringSet(payload: JSONObject, key: String): Set<String> {
        val values = payload.optJSONArray(key) ?: return emptySet()
        return buildSet {
            for (index in 0 until minOf(values.length(), MAX_CAPABILITIES)) {
                val value = values.optString(index).trim().take(MAX_CAPABILITY_LENGTH)
                if (CAPABILITY_ID.matches(value)) add(value)
            }
        }
    }

    private fun textContent(message: JSONObject): String {
        val content = message.optJSONArray("content") ?: return ""
        return buildList {
            for (index in 0 until minOf(content.length(), MAX_CONTENT_PARTS)) {
                val part = content.optJSONObject(index) ?: continue
                if (part.optString("type") in SUPPORTED_CONTENT_TYPES) add(part.optString("text"))
            }
        }.joinToString("\n")
    }

    const val SCHEMA = "yilong.ai.ui.v1"
    private val SUPPORTED_ROLES = setOf("user", "assistant")
    private val SUPPORTED_MESSAGE_STATES = setOf("completed", "streaming")
    private val SUPPORTED_CONTENT_TYPES = setOf("text", "markdown")
    private val SUPPORTED_COMPOSER_SECTIONS = setOf("model", "tools")
    private val PAGE_KINDS = setOf("auth", "conversation", "home", "feature")
    private val ACCESS_REASONS = setOf("login_required", "rate_limited")
    private val ACCESS_SOURCES = setOf("visible_page", "private_response")
    private const val MAX_PRIVATE_STREAM_REVISION = 1_000_000_000L
    private val PRIVATE_STREAM_STATES = setOf("idle", "streaming", "completed")
    private val SUPPORTED_TOUCH_PURPOSES = setOf(
        "list_model_options",
        "list_composer_tools",
        "select_model_option",
        "select_composer_tool",
        "open_model_submenu",
        "open_composer_tools_submenu",
        "open_model_selector",
        "open_composer_tools",
        "start_dictation",
        "cancel_dictation",
        "submit_dictation",
        "remove_attachment",
        "list_navigation",
        "select_navigation",
        "dismiss_navigation",
        "invoke_ui_control",
        "regenerate_open_menu",
        "regenerate_retry",
    )
    private const val MAX_MESSAGES = 80
    private const val MAX_OBSERVED_MESSAGES = 1_000_000
    private const val MAX_REQUEST_ID_LENGTH = 36
    private const val MAX_MESSAGE_LENGTH = 40_000
    private const val MAX_CONTENT_PARTS = 20
    private const val MAX_DRAFT_LENGTH = 20_000
    private const val MAX_CONVERSATIONS = 100
    private const val MAX_PROJECTS = 40
    private const val MAX_PROJECT_ID_LENGTH = 166
    private const val MAX_GROUP_LABEL_LENGTH = 80
    private const val MAX_ACTIVITY_DATES = 32
    private const val MAX_CONVERSATION_COLLECTION_STEPS = 80
    private const val MAX_CAPABILITIES = 40
    private const val MAX_CAPABILITY_LENGTH = 48
    private const val MAX_MODEL_LABEL_LENGTH = 80
    private const val MAX_COMPOSER_OPTIONS = 30
    private const val MAX_OPTION_ID_LENGTH = 64
    private const val MAX_OPTION_LABEL_LENGTH = 120
    private const val MAX_OPTION_KIND_LENGTH = 32
    private const val MAX_ATTACHMENTS = 10
    private const val MAX_ATTACHMENT_ID_LENGTH = 64
    private const val MAX_ATTACHMENT_NAME_LENGTH = 180
    private const val MAX_FEATURES = 60
    private const val MAX_FEATURE_ID_LENGTH = 64
    private const val MAX_FEATURE_LABEL_LENGTH = 120
    private const val MAX_TITLE_LENGTH = 160
    private const val MAX_PATH_LENGTH = 256
    private const val MAX_ID_LENGTH = 160
    private const val MAX_PROVIDER_URL_LENGTH = 8_192
    private const val MAX_UI_CONTROLS = 512
    private const val MAX_DISCOVERED_UI_CONTROLS = 10_000
    private const val MAX_UI_CONTROL_ID_LENGTH = 72
    private const val MAX_UI_CONTROL_LABEL_LENGTH = 160
    private const val MAX_UI_CHOICE_OPTIONS = 50
    private const val MAX_UI_CHOICE_LABEL_LENGTH = 120
    private const val MAX_UI_MANIFEST_VERSION = 8
    private val CAPABILITY_ID = Regex("[a-z][a-z0-9_]{0,47}")
    private val OPTION_ID = Regex("[a-z][a-z0-9_]{1,63}")
    private val ATTACHMENT_ID = Regex("attachment_[a-z0-9]{1,48}")
    private val FEATURE_ID = Regex("feature_[a-z0-9]{1,48}")
    private val UI_CONTROL_ID = Regex("control_[a-z0-9_]{1,63}")
    private val UI_CONTEXT_ID = Regex("[A-Za-z0-9_.:-]{1,160}")
    private val ATTACHMENT_STATES = setOf("uploading", "ready", "error")
    private val FEATURE_KINDS = ChatGptWebProductCapabilityCatalog.FEATURE_KINDS
    private val PROJECT_ID = Regex("g-p-[A-Za-z0-9_-]{1,160}")
    private val ACTIVITY_DATE = Regex("\\d{4}-\\d{2}-\\d{2}")
    private val UI_REGIONS = setOf(
        ChatGptWebUiRegion.HEADER,
        ChatGptWebUiRegion.SUGGESTIONS,
        ChatGptWebUiRegion.COMPOSER,
        ChatGptWebUiRegion.OVERLAY,
        ChatGptWebUiRegion.MESSAGE,
        ChatGptWebUiRegion.CONTENT,
    )
    private val UI_ROLES = setOf(
        "button",
        "link",
        "menuitem",
        "menuitemcheckbox",
        "menuitemradio",
        "option",
        "switch",
        "tab",
        "textbox",
        "combobox",
        "checkbox",
        "radio",
        "slider",
        "treeitem",
    )
    private val UI_INPUT_KINDS = setOf(
        "text",
        "search",
        "email",
        "url",
        "tel",
        "number",
        "date",
        "time",
        "datetime-local",
        "month",
        "week",
        "textarea",
        "contenteditable",
        "password",
        "select",
        "checkbox",
        "radio",
        "switch",
        "tab",
        "range",
    )
    private val WRITABLE_UI_INPUT_KINDS = UI_INPUT_KINDS - setOf(
        "password",
        "select",
        "checkbox",
        "radio",
        "switch",
        "tab",
        "range",
    )
    private val STATE_SETTABLE_UI_ROLES = setOf(
        "checkbox", "radio", "menuitemcheckbox", "menuitemradio", "switch", "tab",
    )
    private val UI_PAGE_KINDS = setOf("home", "conversation", "feature", "auth", "unknown")
    private val UI_COMPATIBILITY = setOf("healthy", "partial", "fallback_required")
    private val REQUEST_ID = Regex("mcp_[a-z0-9]{1,32}")
    private val WEB_SEARCH_LABEL = Regex(
        "^(?:search|搜索|search the web|web search|browse|网页搜索|联网搜索)$",
        RegexOption.IGNORE_CASE,
    )
}
