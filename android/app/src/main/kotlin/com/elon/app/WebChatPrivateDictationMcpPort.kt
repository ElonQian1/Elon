package com.elon.app

import org.json.JSONObject

/** Reports the fail-closed private dictation slot without exposing transcript contents. */
internal class WebChatPrivateDictationMcpPort(
    private val delegate: WebChatSocialMcpPort,
    private val dictation: WebChatPrivateDictationPort,
    private val readyCheck: () -> Boolean,
    private val enabled: Boolean = false,
) : WebChatSocialMcpPort {
    override fun uiState(): JSONObject = delegate.uiState().apply {
        put(STATE_KEY, stateJson())
    }

    override fun control(args: JSONObject): JSONObject = delegate.control(args)

    private fun stateJson(): JSONObject {
        val state = dictation.state()
        return JSONObject()
            .put("enabled", enabled)
            .put("ready", enabled && readyCheck())
            .put("active", state.active)
            .put("phase", state.phase.name.lowercase())
    }

    private companion object {
        const val STATE_KEY = "private_dictation_native"
    }
}
