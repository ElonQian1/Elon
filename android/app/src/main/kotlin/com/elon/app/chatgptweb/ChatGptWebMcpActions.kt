package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal class ChatGptWebMcpActions(
    private val snapshot: () -> ChatGptWebSnapshot?,
    private val uiManifest: () -> ChatGptWebUiManifest?,
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
        return uiState().put("control_ok", true).put("action", action)
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
                    put(JSONObject()
                        .put("control_id", control.id)
                        .put("semantic", control.semantic)
                        .put("label", control.label)
                        .put("region", control.region)
                        .put("role", control.role)
                        .put("enabled", control.enabled)
                        .put("selected", control.selected)
                        .put("context_id", control.contextId ?: JSONObject.NULL)
                        .put("web_x_ratio", control.webXRatio ?: JSONObject.NULL)
                        .put("web_y_ratio", control.webYRatio ?: JSONObject.NULL)
                        .put("adb_content_description", control.accessibilityLabel)
                    )
                }
            })
    }

    private fun error(action: String, code: String): JSONObject = uiState()
        .put("control_ok", false)
        .put("action", action)
        .put("error", code)

    private companion object {
        const val MAX_MESSAGES = 50
        const val MAX_MESSAGE_CHARS = 30_000
        const val MAX_CONTEXT_MESSAGE_CHARS = 40_000
        const val MAX_INPUT_CHARS = 20_000
        const val DEFAULT_CONTEXT_PAGE_SIZE = 20
        const val MAX_CONTEXT_PAGE_SIZE = 40
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
            "chatgpt_open_conversation",
            "chatgpt_select_view",
        )
    }
}
