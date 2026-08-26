package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal enum class RealtimeVoiceTransportId {
    OFFICIAL_WEB_RTC,
    SERVER_API_EXPERIMENT,
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
    val identityLayer: String,
    val mediaTransport: String,
    val presentationLayer: String,
    val consumerDefault: Boolean,
    val runtimeEnabled: Boolean,
    val userVisible: Boolean,
)

internal object RealtimeVoiceTransportCatalog {
    val officialWebRtc = RealtimeVoiceTransportDescriptor(
        id = RealtimeVoiceTransportId.OFFICIAL_WEB_RTC,
        capabilityId = "android_chatgpt_web_realtime_voice_v1",
        label = "官网实时语音",
        scope = RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION,
        identityLayer = "persistent_background_webview",
        mediaTransport = "official_webrtc",
        presentationLayer = "native_ui_overlay",
        consumerDefault = true,
        runtimeEnabled = true,
        userVisible = true,
    )

    val serverApiExperiment = RealtimeVoiceTransportDescriptor(
        id = RealtimeVoiceTransportId.SERVER_API_EXPERIMENT,
        capabilityId = "android_openai_native_realtime_voice_v1",
        label = "API 实时语音（实验）",
        scope = RealtimeVoiceConversationScope.NEW_LOCAL_CONVERSATION,
        identityLayer = "yilong_server_session",
        mediaTransport = "server_realtime_api_websocket",
        presentationLayer = "native_ui_overlay",
        consumerDefault = false,
        runtimeEnabled = false,
        userVisible = false,
    )

    fun describe(): JSONArray = JSONArray().apply {
        put(entry(
            descriptor = officialWebRtc,
            implementationStatus = "completed",
            verificationStatus = "device_verified",
            fallback = "official_webview_voice",
        ))
        put(entry(
            descriptor = serverApiExperiment,
            implementationStatus = "implemented",
            verificationStatus = "experimental_disabled",
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
        .put("identity_layer", descriptor.identityLayer)
        .put("media_transport", descriptor.mediaTransport)
        .put("presentation_layer", descriptor.presentationLayer)
        .put("consumer_default", descriptor.consumerDefault)
        .put("user_visible", descriptor.userVisible)
        .put("implementation_status", implementationStatus)
        .put("verification_status", verificationStatus)
        .put("runtime_enabled", descriptor.runtimeEnabled)
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
