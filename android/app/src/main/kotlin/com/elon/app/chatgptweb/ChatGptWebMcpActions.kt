package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebMcpActions(
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val uiManifest: () -> ChatGptWebUiManifest?,
    private val observedState: () -> ChatGptWebObservedState.Snapshot,
    private val bridgeState: () -> ChatGptWebPageAdapter.State,
    private val mode: () -> ChatGptWebModeController.Mode,
    private val inputText: () -> String,
    private val setInputText: (String) -> Unit,
    private val sendInput: () -> Unit,
    private val invokeControl: (String) -> Unit,
    private val newConversation: () -> Unit,
    private val stopGeneration: () -> Unit,
    private val refresh: () -> Unit,
    private val refreshControls: () -> Unit,
    private val selectMode: (ChatGptWebModeController.Mode) -> Unit,
    private val openConversation: (String) -> Unit,
    private val listConversations: () -> Unit,
) {
    fun uiState(): JSONObject {
        val current = snapshot()
        val observed = observedState()
        return JSONObject()
            .put("surface", "chatgpt_web")
            .put("active_page", "chatgpt_web")
            .put("bridge_state", bridgeState().name.lowercase())
            .put("view_mode", mode().name.lowercase())
            .put("authenticated", current?.authenticated ?: false)
            .put("composer_ready", current?.composerReady ?: false)
            .put("streaming", current?.streaming ?: false)
            .put("conversation", conversationJson(current))
            .put("input", JSONObject()
                .put("text", inputText().take(MAX_INPUT_CHARS))
                .put("text_length", inputText().length)
            )
            .put("ui_manifest", manifestJson(uiManifest()))
            .put("navigation", navigationSummary(observed))
            .put("last_command", commandJson(observed))
            .put("available_actions", JSONArray(AVAILABLE_ACTIONS))
    }

    fun control(args: JSONObject): JSONObject {
        val action = args.optString("action", "state").trim().lowercase()
        when (action) {
            "state" -> Unit
            "set_input_text" -> setInputText(args.optString("text").take(MAX_INPUT_CHARS))
            "send_input" -> sendInput()
            "chatgpt_invoke_control" -> {
                val controlId = args.optString("control_id")
                if (!CONTROL_ID.matches(controlId)) return error(action, "invalid_control_id")
                if (uiManifest()?.controls?.none { it.id == controlId } != false) {
                    return error(action, "stale_control_id")
                }
                invokeControl(controlId)
            }
            "chatgpt_new_conversation" -> newConversation()
            "chatgpt_stop_generation" -> stopGeneration()
            "chatgpt_refresh" -> refresh()
            "chatgpt_refresh_controls" -> refreshControls()
            "chatgpt_list_conversations" -> listConversations()
            "chatgpt_get_context" -> return contextPage(args)
            "chatgpt_find_controls" -> return controlsPage(args)
            "chatgpt_get_conversations" -> return conversationsPage(args)
            "chatgpt_get_navigation" -> return navigationPage(args)
            "chatgpt_get_capability_matrix" -> return ChatGptWebCapabilityMatrix.build(
                snapshot(), uiManifest(), bridgeState(), mode(),
            )
            "chatgpt_open_conversation" -> {
                val path = args.optString("conversation_path")
                if (!CONVERSATION_PATH.matches(path)) return error(action, "invalid_conversation_path")
                openConversation(path)
            }
            "chatgpt_select_view" -> {
                val next = when (args.optString("view_mode").lowercase()) {
                    "login", "quick" -> ChatGptWebModeController.Mode.QUICK
                    "official", "web" -> ChatGptWebModeController.Mode.WEB
                    "native", "yilong" -> ChatGptWebModeController.Mode.NATIVE
                    else -> return error(action, "invalid_view_mode")
                }
                selectMode(next)
            }
            else -> return error(action, "unsupported_action")
        }
        return uiState()
            .put("control_ok", true)
            .put("action", action)
            .apply {
                if (action in ASYNC_ACTIONS) {
                    put("command_status", "dispatched")
                    put("poll_hint", "读取 ui_state.last_command 确认官网命令结果")
                }
            }
    }

    private fun contextPage(args: JSONObject): JSONObject {
        val current = snapshot() ?: return error("chatgpt_get_context", "conversation_unavailable")
        val offset = args.optInt("message_offset", 0).coerceIn(0, current.messages.size)
        val limit = args.optInt("message_limit", DEFAULT_CONTEXT_PAGE_SIZE)
            .coerceIn(1, MAX_CONTEXT_PAGE_SIZE)
        val page = current.messages.drop(offset).take(limit)
        val nextOffset = offset + page.size
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_context")
            .put("conversation_title", current.title)
            .put("conversation_url", current.url)
            .put("message_count", current.messages.size)
            .put("message_offset", offset)
            .put("message_limit", limit)
            .put("next_message_offset", nextOffset)
            .put("has_more", nextOffset < current.messages.size)
            .put("messages", messagesJson(page, offset, MAX_CONTEXT_MESSAGE_CHARS))
    }

    private fun controlsPage(args: JSONObject): JSONObject {
        val manifest = uiManifest() ?: return error("chatgpt_find_controls", "manifest_unavailable")
        val query = args.optString("query").trim()
        val semantic = args.optString("semantic").trim().lowercase()
        val region = args.optString("region").trim().lowercase()
        val contextId = args.optString("context_id").trim()
        val matches = manifest.controls.filter { control ->
            (query.isBlank() || control.label.contains(query, ignoreCase = true)) &&
                (semantic.isBlank() || control.semantic == semantic) &&
                (region.isBlank() || control.region == region) &&
                (contextId.isBlank() || control.contextId == contextId)
        }
        val page = page(args, matches.size, DEFAULT_CONTROL_PAGE_SIZE, MAX_CONTROL_PAGE_SIZE)
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_find_controls")
            .put("query", query)
            .put("semantic", semantic)
            .put("region", region)
            .put("context_id", contextId)
            .put("match_count", matches.size)
            .put("offset", page.offset)
            .put("limit", page.limit)
            .put("next_offset", page.nextOffset)
            .put("has_more", page.hasMore)
            .put("controls", JSONArray().apply {
                matches.drop(page.offset).take(page.limit).forEach { put(controlJson(it)) }
            })
    }

    private fun conversationsPage(args: JSONObject): JSONObject {
        val observed = observedState()
        val query = args.optString("query").trim()
        val matches = observed.conversations.filter {
            query.isBlank() || it.title.contains(query, ignoreCase = true)
        }
        val page = page(args, matches.size, DEFAULT_LIST_PAGE_SIZE, MAX_LIST_PAGE_SIZE)
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_conversations")
            .put("query", query)
            .put("cached_at_ms", observed.updatedAtMs)
            .put("match_count", matches.size)
            .put("offset", page.offset)
            .put("limit", page.limit)
            .put("next_offset", page.nextOffset)
            .put("has_more", page.hasMore)
            .put("conversations", JSONArray().apply {
                matches.drop(page.offset).take(page.limit).forEach { conversation ->
                    put(JSONObject()
                        .put("id", conversation.id)
                        .put("title", conversation.title)
                        .put("path", conversation.path)
                        .put("active", conversation.active)
                    )
                }
            })
    }

    private fun navigationPage(args: JSONObject): JSONObject {
        val observed = observedState()
        val section = args.optString("section").trim().lowercase()
        val optionSections = observed.composerSections
            .filterKeys { section.isBlank() || it == section }
        return JSONObject()
            .put("control_ok", true)
            .put("action", "chatgpt_get_navigation")
            .put("cached_at_ms", observed.updatedAtMs)
            .put("features", JSONArray().apply {
                observed.features.forEach { feature ->
                    put(JSONObject()
                        .put("id", feature.id)
                        .put("label", feature.label)
                        .put("kind", feature.kind)
                        .put("selected", feature.selected)
                    )
                }
            })
            .put("composer_sections", JSONObject().apply {
                optionSections.forEach { (name, options) ->
                    put(name, JSONArray().apply {
                        options.forEach { option ->
                            put(JSONObject()
                                .put("id", option.id)
                                .put("label", option.label)
                                .put("kind", option.kind)
                                .put("selected", option.selected)
                            )
                        }
                    })
                }
            })
    }

    private fun conversationJson(value: ChatGptWebSnapshot?): Any {
        if (value == null) return JSONObject.NULL
        return JSONObject()
            .put("title", value.title)
            .put("url", value.url)
            .put("current_model", value.currentModel)
            .put("message_count", value.messages.size)
            .put("messages", messagesJson(value.messages.takeLast(MAX_MESSAGES),
                (value.messages.size - MAX_MESSAGES).coerceAtLeast(0), MAX_MESSAGE_CHARS))
            .put("attachments", JSONArray().apply {
                value.attachments.forEach { attachment ->
                    put(JSONObject()
                        .put("id", attachment.id)
                        .put("name", attachment.name)
                        .put("state", attachment.state)
                    )
                }
            })
    }

    private fun messagesJson(
        messages: List<ChatGptWebMessage>,
        startIndex: Int,
        maxChars: Int,
    ): JSONArray = JSONArray().apply {
        messages.forEachIndexed { offset, message ->
            put(JSONObject()
                .put("index", startIndex + offset)
                .put("id", message.id)
                .put("role", message.role)
                .put("state", message.state)
                .put("content", message.content.take(maxChars))
                .put("content_chars", message.content.length)
                .put("content_truncated", message.content.length > maxChars)
            )
        }
    }

    private fun navigationSummary(value: ChatGptWebObservedState.Snapshot): JSONObject = JSONObject()
        .put("conversation_count", value.conversations.size)
        .put("feature_count", value.features.size)
        .put("composer_sections", JSONArray(value.composerSections.keys.sorted()))
        .put("cached_at_ms", value.updatedAtMs)

    private fun commandJson(value: ChatGptWebObservedState.Snapshot): Any {
        val command = value.lastCommand ?: return JSONObject.NULL
        return JSONObject()
            .put("action", command.action)
            .put("ok", command.ok)
            .put("detail", command.detail)
            .put("observed_at_ms", value.updatedAtMs)
    }

    private fun manifestJson(value: ChatGptWebUiManifest?): Any {
        if (value == null) return JSONObject.NULL
        return JSONObject()
            .put("version", value.version)
            .put("page_kind", value.pageKind)
            .put("title", value.title)
            .put("compatibility", value.compatibility)
            .put("control_count", value.controls.size)
            .put("generic_control_count", value.controls.count { it.semantic == "action" })
            .put("message_control_count", value.controls.count { it.region == ChatGptWebUiRegion.MESSAGE })
            .put("web_position_count", value.controls.count { it.webXRatio != null && it.webYRatio != null })
            .put("controls", JSONArray().apply {
                value.controls.forEach { control ->
                    put(controlJson(control))
                }
            })
    }

    private fun controlJson(control: ChatGptWebUiControl): JSONObject = JSONObject()
        .put("control_id", control.id)
        .put("semantic", control.semantic)
        .put("label", control.label)
        .put("region", control.region)
        .put("role", control.role)
        .put("enabled", control.enabled)
        .put("selected", control.selected)
        .put("context_id", control.contextId ?: JSONObject.NULL)
        .put("in_viewport", control.inViewport)
        .put("web_x_ratio", control.webXRatio ?: JSONObject.NULL)
        .put("web_y_ratio", control.webYRatio ?: JSONObject.NULL)
        .put("adb_content_description", control.accessibilityLabel)

    private fun page(
        args: JSONObject,
        total: Int,
        defaultLimit: Int,
        maxLimit: Int,
    ): Page {
        val offset = args.optInt("offset", 0).coerceIn(0, total)
        val limit = args.optInt("limit", defaultLimit).coerceIn(1, maxLimit)
        val nextOffset = (offset + limit).coerceAtMost(total)
        return Page(offset, limit, nextOffset, nextOffset < total)
    }

    private fun error(action: String, code: String): JSONObject = uiState()
        .put("control_ok", false)
        .put("action", action)
        .put("error", code)

    private companion object {
        data class Page(
            val offset: Int,
            val limit: Int,
            val nextOffset: Int,
            val hasMore: Boolean,
        )

        const val MAX_MESSAGES = 50
        const val MAX_MESSAGE_CHARS = 30_000
        const val MAX_CONTEXT_MESSAGE_CHARS = 40_000
        const val MAX_INPUT_CHARS = 20_000
        const val DEFAULT_CONTEXT_PAGE_SIZE = 20
        const val MAX_CONTEXT_PAGE_SIZE = 40
        const val DEFAULT_CONTROL_PAGE_SIZE = 30
        const val MAX_CONTROL_PAGE_SIZE = 80
        const val DEFAULT_LIST_PAGE_SIZE = 30
        const val MAX_LIST_PAGE_SIZE = 50
        val CONTROL_ID = Regex("control_[a-z0-9_]{1,63}")
        val CONVERSATION_PATH = Regex("/c/[A-Za-z0-9_-]{1,160}")
        val AVAILABLE_ACTIONS = listOf(
            "state",
            "set_input_text",
            "send_input",
            "chatgpt_invoke_control",
            "chatgpt_new_conversation",
            "chatgpt_stop_generation",
            "chatgpt_refresh",
            "chatgpt_refresh_controls",
            "chatgpt_list_conversations",
            "chatgpt_get_context",
            "chatgpt_find_controls",
            "chatgpt_get_conversations",
            "chatgpt_get_navigation",
            "chatgpt_get_capability_matrix",
            "chatgpt_open_conversation",
            "chatgpt_select_view",
        )
        val ASYNC_ACTIONS = setOf(
            "send_input",
            "chatgpt_invoke_control",
            "chatgpt_new_conversation",
            "chatgpt_stop_generation",
            "chatgpt_refresh",
            "chatgpt_refresh_controls",
            "chatgpt_list_conversations",
            "chatgpt_open_conversation",
        )
    }
}
