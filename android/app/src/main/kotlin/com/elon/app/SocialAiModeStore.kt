package com.elon.app

import android.content.Context

internal class SocialAiModeStore(context: Context) {
    private val preferences = context.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)

    fun interactionMode(): SocialAiInteractionMode {
        if (!preferences.getBoolean(KEY_CHAT_DEFAULT_MIGRATED, false)) {
            preferences.edit()
                .putBoolean(KEY_CHAT_DEFAULT_MIGRATED, true)
                .putString(KEY_INTERACTION_MODE, SocialAiInteractionMode.CHAT.wireValue)
                .apply()
            return SocialAiInteractionMode.CHAT
        }
        return SocialAiInteractionMode.fromWireValue(preferences.getString(KEY_INTERACTION_MODE, null))
    }

    fun providerId(): WebChatProviderId =
        WebChatProviderId.fromWireValue(preferences.getString(KEY_PROVIDER_ID, null))

    fun save(mode: SocialAiInteractionMode, providerId: WebChatProviderId) {
        preferences.edit()
            .putBoolean(KEY_CHAT_DEFAULT_MIGRATED, true)
            .putString(KEY_INTERACTION_MODE, mode.wireValue)
            .putString(KEY_PROVIDER_ID, providerId.wireValue)
            .apply()
    }

    private companion object {
        const val PREFERENCES = "social_ai_mode"
        const val KEY_INTERACTION_MODE = "interaction_mode"
        const val KEY_PROVIDER_ID = "web_chat_provider_id"
        const val KEY_CHAT_DEFAULT_MIGRATED = "chat_default_migrated_v1"
    }
}
