package com.elon.chatvoice

import android.content.Context

internal enum class ChatVoiceEngineHealth {
    UNKNOWN,
    OK,
    FAILED,
}

internal object ChatVoiceEngineHealthStore {
    private const val PREFS = "elon_chat_voice_engine"
    private const val HEALTH_PREFIX = "health_"
    private const val ERROR_PREFIX = "error_"

    fun get(context: Context, key: String): ChatVoiceEngineHealth {
        val raw = prefs(context).getString(HEALTH_PREFIX + key, null) ?: return ChatVoiceEngineHealth.UNKNOWN
        return runCatching { ChatVoiceEngineHealth.valueOf(raw) }.getOrDefault(ChatVoiceEngineHealth.UNKNOWN)
    }

    fun markOk(context: Context, key: String) {
        prefs(context).edit()
            .putString(HEALTH_PREFIX + key, ChatVoiceEngineHealth.OK.name)
            .remove(ERROR_PREFIX + key)
            .apply()
    }

    fun markFailed(context: Context, key: String, code: Int, message: String) {
        prefs(context).edit()
            .putString(HEALTH_PREFIX + key, ChatVoiceEngineHealth.FAILED.name)
            .putString(ERROR_PREFIX + key, "$code|$message")
            .apply()
    }

    private fun prefs(context: Context) =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
