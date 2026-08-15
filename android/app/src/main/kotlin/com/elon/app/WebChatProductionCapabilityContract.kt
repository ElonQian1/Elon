package com.elon.app

import com.elon.app.chatgptweb.ChatGptNativeNavigationSelector
import org.json.JSONArray
import org.json.JSONObject

/**
 * Auditable delivery contract for capabilities exposed by the real friend-chat surface.
 * Diagnostic-Activity controls are intentionally excluded.
 */
internal object WebChatProductionCapabilityContract {
    enum class Access(val wireValue: String) {
        ADB("adb"),
        ADB_AND_MCP("adb_and_mcp"),
        MCP_READ("mcp_read"),
    }

    enum class McpChannel(val wireValue: String) {
        MAIN("main_native_ui"),
        CHATGPT("chatgpt_web"),
    }

    enum class SelectorMode(val wireValue: String) {
        EXACT("exact"),
        PREFIX("prefix"),
        TEMPLATE("template"),
        NONE("none"),
    }

    data class Delivery(
        val capability: WebChatProviderCapability,
        val providers: Set<WebChatProviderId>,
        val nativeSurface: String,
        val access: Access,
        val selector: String?,
        val selectorMode: SelectorMode,
        val mcpChannel: McpChannel,
        val readAction: String,
        val controlAction: String?,
        val officialFallbackAction: String = OFFICIAL_FALLBACK_ACTION,
    ) {
        fun supports(providerId: WebChatProviderId): Boolean = providerId in providers

        fun isComplete(): Boolean =
            nativeSurface.isNotBlank() &&
                readAction.isNotBlank() &&
                officialFallbackAction.isNotBlank() &&
                when (access) {
                    Access.ADB -> !selector.isNullOrBlank() && selectorMode != SelectorMode.NONE
                    Access.ADB_AND_MCP ->
                        !selector.isNullOrBlank() &&
                            selectorMode != SelectorMode.NONE &&
                            !controlAction.isNullOrBlank()
                    Access.MCP_READ ->
                        selector.isNullOrBlank() &&
                            selectorMode == SelectorMode.NONE &&
                            controlAction.isNullOrBlank()
                }

        fun selectorFor(provider: WebChatProviderIdentity): String? = selector
            ?.replace("{provider}", provider.id.wireValue)
            ?.replace("{provider_name}", provider.displayName)
    }

    fun deliveries(providerId: WebChatProviderId): Map<WebChatProviderCapability, Delivery> =
        DELIVERIES.filter { it.supports(providerId) }.associateBy(Delivery::capability)

    fun knownCapabilities(): Set<WebChatProviderCapability> =
        DELIVERIES.mapTo(linkedSetOf(), Delivery::capability)

    fun missing(
        providerId: WebChatProviderId,
        capabilities: Set<WebChatProviderCapability>,
    ): Set<WebChatProviderCapability> {
        val available = deliveries(providerId)
        return capabilities.filterTo(linkedSetOf()) { capability ->
            available[capability]?.isComplete() != true
        }
    }

    fun isComplete(
        providerId: WebChatProviderId,
        capabilities: Set<WebChatProviderCapability>,
    ): Boolean = missing(providerId, capabilities).isEmpty()

    fun describe(provider: WebChatProviderIdentity): JSONObject {
        val available = deliveries(provider.id)
        val gaps = missing(provider.id, provider.capabilities)
        val rows = provider.capabilities
            .sortedBy(WebChatProviderCapability::ordinal)
            .map { capability ->
                val delivery = available[capability]
                JSONObject()
                    .put("capability", capability.name.lowercase())
                    .put("covered", delivery?.isComplete() == true)
                    .put("native_surface", delivery?.nativeSurface ?: JSONObject.NULL)
                    .put("access", delivery?.access?.wireValue ?: JSONObject.NULL)
                    .put("adb_selector", delivery?.selectorFor(provider) ?: JSONObject.NULL)
                    .put("selector_mode", delivery?.selectorMode?.wireValue ?: JSONObject.NULL)
                    .put("mcp_channel", delivery?.mcpChannel?.wireValue ?: JSONObject.NULL)
                    .put("read_action", delivery?.readAction ?: JSONObject.NULL)
                    .put("control_action", delivery?.controlAction ?: JSONObject.NULL)
                    .put(
                        "official_fallback_action",
                        delivery?.officialFallbackAction ?: JSONObject.NULL,
                    )
            }
        return JSONObject()
            .put("schema", "elon.web_chat.production_capabilities.v1")
            .put("provider_id", provider.id.wireValue)
            .put("declared_capability_count", provider.capabilities.size)
            .put("covered_capability_count", provider.capabilities.size - gaps.size)
            .put("ready", gaps.isEmpty())
            .put("missing", JSONArray(gaps.map { it.name.lowercase() }))
            .put("capabilities", JSONArray(rows))
    }

    private fun shared(
        capability: WebChatProviderCapability,
        nativeSurface: String,
        access: Access,
        selector: String?,
        selectorMode: SelectorMode,
        mcpChannel: McpChannel = McpChannel.MAIN,
        readAction: String = "state",
        controlAction: String? = null,
    ) = Delivery(
        capability = capability,
        providers = ALL_PROVIDERS,
        nativeSurface = nativeSurface,
        access = access,
        selector = selector,
        selectorMode = selectorMode,
        mcpChannel = mcpChannel,
        readAction = readAction,
        controlAction = controlAction,
    )

    private fun chatGpt(
        capability: WebChatProviderCapability,
        nativeSurface: String,
        access: Access,
        selector: String?,
        selectorMode: SelectorMode,
        readAction: String,
        controlAction: String? = null,
    ) = Delivery(
        capability = capability,
        providers = setOf(WebChatProviderId.CHATGPT_WEB),
        nativeSurface = nativeSurface,
        access = access,
        selector = selector,
        selectorMode = selectorMode,
        mcpChannel = McpChannel.CHATGPT,
        readAction = readAction,
        controlAction = controlAction,
    )

    private val ALL_PROVIDERS = WebChatProviderId.entries.toSet()

    private val DELIVERIES = listOf(
        shared(
            WebChatProviderCapability.CONVERSATION_LIST,
            "friend_chat_sidebar",
            Access.ADB_AND_MCP,
            ChatGptNativeNavigationSelector.CONVERSATION_LIST_TRIGGER,
            SelectorMode.EXACT,
            readAction = "get_web_chat_navigation",
            controlAction = "open_chat_side_menu",
        ),
        shared(
            WebChatProviderCapability.PROJECT_LIST,
            "friend_chat_sidebar",
            Access.ADB_AND_MCP,
            ChatGptNativeNavigationSelector.PROJECTS_TAB,
            SelectorMode.PREFIX,
            readAction = "get_web_chat_navigation",
            controlAction = "set_web_chat_sidebar",
        ),
        shared(
            WebChatProviderCapability.NEW_CONVERSATION,
            "friend_chat_sidebar",
            Access.ADB_AND_MCP,
            ChatGptNativeNavigationSelector.NEW_CONVERSATION,
            SelectorMode.EXACT,
            controlAction = "start_new_web_chat_conversation",
        ),
        shared(
            WebChatProviderCapability.MESSAGE_COPY,
            "friend_chat_message_actions",
            Access.ADB,
            "web-chat-message-action:{provider}:{context_id}:copy",
            SelectorMode.TEMPLATE,
        ),
        chatGpt(
            WebChatProviderCapability.MESSAGE_REGENERATE,
            "friend_chat_message_actions",
            Access.ADB_AND_MCP,
            "web-chat-message-action:{provider}:{context_id}:regenerate",
            SelectorMode.TEMPLATE,
            readAction = "chatgpt_get_context",
            controlAction = "chatgpt_regenerate_response",
        ),
        chatGpt(
            WebChatProviderCapability.MESSAGE_CONTEXT_ACTIONS,
            "friend_chat_message_actions",
            Access.ADB_AND_MCP,
            "web-chat-message-action:{provider}:{context_id}:more",
            SelectorMode.TEMPLATE,
            readAction = "chatgpt_find_controls",
            controlAction = "chatgpt_invoke_control",
        ),
        chatGpt(
            WebChatProviderCapability.MODEL_SELECTOR,
            "friend_chat_provider_picker",
            Access.ADB_AND_MCP,
            "聊天模式；提供方：{provider_name}；模型：",
            SelectorMode.PREFIX,
            readAction = "ui_state",
            controlAction = "chatgpt_select_composer_option",
        ),
        chatGpt(
            WebChatProviderCapability.ATTACHMENT_UPLOAD,
            "friend_chat_composer",
            Access.ADB,
            "web-chat-attachment:{provider}",
            SelectorMode.EXACT,
            readAction = "ui_state",
        ),
        chatGpt(
            WebChatProviderCapability.COMPOSER_TOOLS,
            "friend_chat_composer_tools",
            Access.ADB_AND_MCP,
            "web-chat-composer-tools:{provider}",
            SelectorMode.EXACT,
            readAction = "chatgpt_get_navigation",
            controlAction = "chatgpt_select_composer_option",
        ),
        chatGpt(
            WebChatProviderCapability.FEATURE_NAVIGATION,
            "friend_chat_sidebar",
            Access.ADB_AND_MCP,
            ChatGptNativeNavigationSelector.FEATURE_LIST_TRIGGER,
            SelectorMode.EXACT,
            readAction = "chatgpt_get_navigation",
            controlAction = "chatgpt_select_feature",
        ),
        chatGpt(
            WebChatProviderCapability.PAGE_ACTIONS,
            "friend_chat_toolbar",
            Access.ADB_AND_MCP,
            "web-chat-page-actions:{provider}",
            SelectorMode.EXACT,
            readAction = "chatgpt_find_controls",
            controlAction = "chatgpt_invoke_control",
        ),
        shared(
            WebChatProviderCapability.STOP_GENERATION,
            "friend_chat_composer",
            Access.ADB,
            WebChatProductionSelectors.STOP_GENERATION,
            SelectorMode.EXACT,
        ),
        chatGpt(
            WebChatProviderCapability.DICTATION,
            "friend_chat_composer_tools",
            Access.ADB_AND_MCP,
            "web-chat-composer-command:{provider}:{start-or-submit}-dictation",
            SelectorMode.TEMPLATE,
            readAction = "ui_state",
            controlAction = "chatgpt_start_dictation",
        ),
        chatGpt(
            WebChatProviderCapability.REALTIME_VOICE,
            "friend_chat_composer_tools",
            Access.ADB_AND_MCP,
            "web-chat-composer-command:{provider}:start-realtime-voice",
            SelectorMode.EXACT,
            readAction = "ui_state",
            controlAction = "chatgpt_start_realtime_voice",
        ),
        shared(
            WebChatProviderCapability.RICH_TEXT,
            "friend_chat_messages",
            Access.MCP_READ,
            null,
            SelectorMode.NONE,
            readAction = "state",
        ),
        chatGpt(
            WebChatProviderCapability.RICH_PARTS,
            "friend_chat_messages",
            Access.MCP_READ,
            null,
            SelectorMode.NONE,
            readAction = "chatgpt_get_context",
        ),
    )

    private const val OFFICIAL_FALLBACK_ACTION = "open_web_chat_official_fallback"
}
