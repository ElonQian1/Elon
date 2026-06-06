package com.elon.app

import android.content.Context

internal object VoiceTtsPreferences {
    private const val PREFS_NAME = "elon"
    private const val KEY_TTS_ENABLED = "tts_speak_enabled"
    private const val KEY_TTS_VOICE_ID = "tts_voice_id"

    const val DEFAULT_VOICE_ID = "female_warm"

    fun isSpeakEnabled(context: Context): Boolean =
        context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getBoolean(KEY_TTS_ENABLED, false)

    fun setSpeakEnabled(context: Context, enabled: Boolean) {
        context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putBoolean(KEY_TTS_ENABLED, enabled)
            .apply()
    }

    fun getSelectedVoiceId(context: Context): String {
        val saved = context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getString(KEY_TTS_VOICE_ID, null)
            ?.trim()
            .orEmpty()
        return saved.takeIf(VoiceTtsVoiceCatalog::isKnownVoiceId) ?: DEFAULT_VOICE_ID
    }

    fun setSelectedVoiceId(context: Context, voiceId: String) {
        val safeVoiceId = voiceId.trim()
        require(VoiceTtsVoiceCatalog.isKnownVoiceId(safeVoiceId)) {
            "unknown TTS voice id: $safeVoiceId"
        }
        context.applicationContext
            .getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_TTS_VOICE_ID, safeVoiceId)
            .apply()
    }
}
