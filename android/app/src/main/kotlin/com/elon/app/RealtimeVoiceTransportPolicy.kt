package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal enum class RealtimeVoiceTransportId {
    WEB_ACCOUNT,
    NATIVE_API,
}

internal enum class RealtimeVoiceConversationScope {
    CURRENT_PROVIDER_CONVERSATION,
    NEW_LOCAL_CONVERSATION,
}

internal data class RealtimeVoiceTransportDescriptor(
    val id: RealtimeVoiceTransportId,
    val capabilityId: String,
    val label: String,
    val scope: RealtimeVoiceConversationScope,
)

internal object RealtimeVoiceTransportCatalog {
    val webAccount = RealtimeVoiceTransportDescriptor(
        id = RealtimeVoiceTransportId.WEB_ACCOUNT,
        capabilityId = "android_chatgpt_web_realtime_voice_v1",
        label = "ChatGPT 网页语音",
        scope = RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION,
    )

    val nativeApi = RealtimeVoiceTransportDescriptor(
        id = RealtimeVoiceTransportId.NATIVE_API,
        capabilityId = "android_openai_native_realtime_voice_v1",
        label = "原生实时 AI",
        scope = RealtimeVoiceConversationScope.NEW_LOCAL_CONVERSATION,
    )

    fun describe(): JSONArray = JSONArray().apply {
        put(entry(
            descriptor = webAccount,
            implementationStatus = "completed",
            verificationStatus = "device_verified",
            fallback = "official_webview_voice",
        ))
        put(entry(
            descriptor = nativeApi,
            implementationStatus = "implemented",
            verificationStatus = "targeted_tests_passed_device_pending",
            fallback = "official_webview_voice",
        ))
    }

    private fun entry(
        descriptor: RealtimeVoiceTransportDescriptor,
        implementationStatus: String,
        verificationStatus: String,
        fallback: String,
    ): JSONObject = JSONObject()
        .put("capability_id", descriptor.capabilityId)
        .put("transport_id", descriptor.id.name.lowercase())
        .put("label", descriptor.label)
        .put("conversation_scope", descriptor.scope.name.lowercase())
        .put("implementation_status", implementationStatus)
        .put("verification_status", verificationStatus)
        .put("runtime_enabled", true)
        .put("fallback", fallback)
}

internal object RealtimeVoiceTransportPolicy {
    fun canUseCurrentProviderConversation(transport: RealtimeVoiceTransportDescriptor): Boolean =
        transport.scope == RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION

    fun contextFor(transport: RealtimeVoiceTransportDescriptor): WebChatRealtimeVoiceContext =
        when (transport.scope) {
            RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION ->
                WebChatRealtimeVoiceContext(
                    conversationPath = null,
                    label = "当前 ChatGPT 会话",
                    savedToHistory = true,
                )
            RealtimeVoiceConversationScope.NEW_LOCAL_CONVERSATION ->
                WebChatRealtimeVoiceContext(
                    conversationPath = null,
                    label = "一龙 AI 新会话",
                    savedToHistory = true,
                    openable = true,
                )
        }
}
