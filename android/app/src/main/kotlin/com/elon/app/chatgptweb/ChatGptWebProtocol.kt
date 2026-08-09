package com.elon.app.chatgptweb

import org.json.JSONObject

internal data class ChatGptWebMessage(
    val id: String,
    val role: String,
    val content: String,
    val state: String,
    val parts: List<ChatGptWebMessagePart>,
)

internal data class ChatGptWebMessagePart(
    val type: String,
    val label: String,
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
    val currentModel: String,
    val attachments: List<ChatGptWebAttachment>,
    val dictationActive: Boolean,
    val capabilities: ChatGptWebCapabilities,
)

internal data class ChatGptWebComposerOption(
    val id: String,
    val label: String,
    val selected: Boolean,
    val kind: String,
)

internal data class ChatGptWebFeature(
    val id: String,
    val label: String,
    val kind: String,
    val selected: Boolean,
)

internal sealed interface ChatGptWebEvent {
    data class Snapshot(val value: ChatGptWebSnapshot) : ChatGptWebEvent

    data class ConversationList(
        val conversations: List<ChatGptWebConversation>,
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

    data class CommandResult(
        val action: String,
        val ok: Boolean,
        val detail: String,
    ) : ChatGptWebEvent
}

internal object ChatGptWebProtocol {
    fun parse(rawPayload: String, minimumAdapterVersion: Int = 0): ChatGptWebEvent? {
        val payload = runCatching { JSONObject(rawPayload) }.getOrNull() ?: return null
        if (payload.optInt("adapterVersion", 0) < minimumAdapterVersion) return null
        if (payload.optString("schema") == SCHEMA) {
            val event = payload.optJSONObject("event") ?: return null
            return when (event.optString("type")) {
                "message_snapshot" -> ChatGptWebEvent.Snapshot(parseSnapshot(event))
                "conversation_snapshot" -> ChatGptWebEvent.ConversationList(
                    parseConversations(event),
                )
                "composer_controls_snapshot" -> parseComposerControls(event)
                "navigation_snapshot" -> ChatGptWebEvent.FeatureNavigation(parseFeatures(event))
                "ui_manifest_snapshot" -> ChatGptWebEvent.UiManifest(parseUiManifest(event))
                "web_touch_request" -> parseWebTouchRequest(event)
                else -> null
            }
        }
        return when (payload.optString("type")) {
            "command_result" -> ChatGptWebEvent.CommandResult(
                action = payload.optString("action").take(40),
                ok = payload.optBoolean("ok"),
                detail = payload.optString("detail").take(160),
            )
            else -> null
        }
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
                val parts = parseMessageParts(item)
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
        return ChatGptWebSnapshot(
            title = event.optString("title").trim().take(120),
            url = event.optString("url").take(2_048),
            draft = event.optString("draft").take(MAX_DRAFT_LENGTH),
            messages = messages,
            authenticated = event.optBoolean("authenticated"),
            composerReady = event.optBoolean("composerReady"),
            streaming = event.optBoolean("streaming"),
            currentModel = event.optString("currentModel").trim().take(MAX_MODEL_LABEL_LENGTH),
            attachments = parseAttachments(event),
            dictationActive = event.optBoolean("dictationActive"),
            capabilities = ChatGptWebCapabilities(parseStringSet(event, "capabilities")),
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

    private fun parseMessageParts(message: JSONObject): List<ChatGptWebMessagePart> {
        val content = message.optJSONArray("content") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(content.length(), MAX_CONTENT_PARTS)) {
                val part = content.optJSONObject(index) ?: continue
                val type = part.optString("type")
                if (type !in STRUCTURED_CONTENT_TYPES) continue
                val label = part.optString("text").trim().take(MAX_MESSAGE_PART_LABEL_LENGTH)
                if (label.isBlank()) continue
                add(ChatGptWebMessagePart(type = type, label = label))
            }
        }.take(MAX_STRUCTURED_MESSAGE_PARTS)
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
                add(
                    ChatGptWebComposerOption(
                        id = id,
                        label = label,
                        selected = item.optBoolean("selected"),
                        kind = item.optString("kind").take(MAX_OPTION_KIND_LENGTH),
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
                val semantic = item.optString("semantic")
                    .takeIf { it in ChatGptWebUiSemantics.KNOWN }
                    ?: ChatGptWebUiSemantics.GENERIC_ACTION
                val label = item.optString("label").trim().take(MAX_UI_CONTROL_LABEL_LENGTH)
                val region = item.optString("region").takeIf { it in UI_REGIONS } ?: continue
                val role = item.optString("role").takeIf { it in UI_ROLES } ?: "button"
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
                        contextId = contextId,
                        inViewport = item.optBoolean("inViewport", true),
                        webXRatio = webXRatio,
                        webYRatio = webYRatio,
                    ),
                )
            }
        }
        return ChatGptWebUiManifest(
            version = event.optInt("version", 1).coerceIn(1, MAX_UI_MANIFEST_VERSION),
            pageKind = event.optString("pageKind").takeIf { it in UI_PAGE_KINDS } ?: "unknown",
            title = event.optString("title").trim().take(MAX_TITLE_LENGTH),
            compatibility = event.optString("compatibility")
                .takeIf { it in UI_COMPATIBILITY } ?: "partial",
            controls = controls,
        )
    }

    private fun parseConversations(event: JSONObject): List<ChatGptWebConversation> {
        val items = event.optJSONArray("conversations") ?: return emptyList()
        return buildList {
            for (index in 0 until minOf(items.length(), MAX_CONVERSATIONS)) {
                val item = items.optJSONObject(index) ?: continue
                val path = item.optString("path").take(MAX_PATH_LENGTH)
                if (!CONVERSATION_PATH.matches(path)) continue
                val id = item.optString("id").ifBlank { path.removePrefix("/c/") }
                val title = item.optString("title").trim().take(MAX_TITLE_LENGTH)
                if (title.isBlank()) continue
                add(
                    ChatGptWebConversation(
                        id = id.take(MAX_ID_LENGTH),
                        title = title,
                        path = path,
                        active = item.optBoolean("active"),
                    ),
                )
            }
        }
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
    private val STRUCTURED_CONTENT_TYPES = setOf(
        "image",
        "file",
        "citation",
        "artifact",
        "audio",
        "video",
    )
    private val SUPPORTED_COMPOSER_SECTIONS = setOf("model", "tools")
    private val SUPPORTED_TOUCH_PURPOSES = setOf(
        "list_model_options",
        "list_composer_tools",
        "select_model_option",
        "select_composer_tool",
        "open_model_selector",
        "open_composer_tools",
        "start_dictation",
        "remove_attachment",
        "list_navigation",
        "select_navigation",
        "dismiss_navigation",
        "invoke_ui_control",
    )
    private const val MAX_MESSAGES = 80
    private const val MAX_MESSAGE_LENGTH = 40_000
    private const val MAX_CONTENT_PARTS = 20
    private const val MAX_STRUCTURED_MESSAGE_PARTS = 16
    private const val MAX_MESSAGE_PART_LABEL_LENGTH = 180
    private const val MAX_DRAFT_LENGTH = 20_000
    private const val MAX_CONVERSATIONS = 100
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
    private const val MAX_UI_CONTROLS = 160
    private const val MAX_UI_CONTROL_ID_LENGTH = 72
    private const val MAX_UI_CONTROL_LABEL_LENGTH = 160
    private const val MAX_UI_MANIFEST_VERSION = 3
    private val CAPABILITY_ID = Regex("[a-z][a-z0-9_]{0,47}")
    private val OPTION_ID = Regex("[a-z][a-z0-9_]{1,63}")
    private val ATTACHMENT_ID = Regex("attachment_[a-z0-9]{1,48}")
    private val FEATURE_ID = Regex("feature_[a-z0-9]{1,48}")
    private val UI_CONTROL_ID = Regex("control_[a-z0-9_]{1,63}")
    private val UI_CONTEXT_ID = Regex("[A-Za-z0-9_.:-]{1,160}")
    private val ATTACHMENT_STATES = setOf("uploading", "ready", "error")
    private val FEATURE_KINDS = setOf(
        "library",
        "tasks",
        "projects",
        "gpts",
        "memory",
        "apps",
        "settings",
        "more",
        "navigation",
    )
    private val CONVERSATION_PATH = Regex("/c/[A-Za-z0-9_-]{1,160}")
    private val UI_REGIONS = setOf("header", "suggestions", "composer", "overlay", "message")
    private val UI_ROLES = setOf("button", "link", "menuitem", "switch", "tab")
    private val UI_PAGE_KINDS = setOf("home", "conversation", "feature", "auth", "unknown")
    private val UI_COMPATIBILITY = setOf("healthy", "partial", "fallback_required")
}
