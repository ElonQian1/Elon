package com.elon.app

import org.json.JSONObject

internal interface WebChatSocialMcpPort {
    fun uiState(): JSONObject
    fun control(args: JSONObject): JSONObject
}
