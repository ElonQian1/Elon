package com.elon.app.sociallinks

import android.content.Context
import com.elon.app.AuthManager
import com.elon.app.ServerUrlManager
import java.security.MessageDigest

/** Device-local, per account/server and platform. No URLs, cookies or sharing tokens are saved. */
internal class SocialMediaOpenPreferences(context: Context) {
    private val app = context.applicationContext
    val scope = scope(app)
    val preferences = app.getSharedPreferences("social-media-opening-v1", Context.MODE_PRIVATE)
    private fun key(platform: SocialMediaPlatform) = "$scope:${platform.key}"
    fun current() = scope == scope(app)
    fun get(platform: SocialMediaPlatform) = runCatching {
        SocialMediaOpenMode.valueOf(preferences.getString(key(platform), "APP") ?: "APP")
    }.getOrDefault(SocialMediaOpenMode.APP)
    fun set(platform: SocialMediaPlatform, mode: SocialMediaOpenMode) {
        if (current()) preferences.edit().putString(key(platform), mode.name).apply()
    }
    companion object {
        private fun scope(context: Context): String {
            val source = ServerUrlManager.getActive(context) + "\n" + AuthManager.userId(context).orEmpty()
            return MessageDigest.getInstance("SHA-256").digest(source.toByteArray(Charsets.UTF_8))
                .joinToString("") { "%02x".format(it) }
        }
    }
}
