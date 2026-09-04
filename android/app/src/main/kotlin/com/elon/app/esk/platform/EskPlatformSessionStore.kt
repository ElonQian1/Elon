package com.elon.app.esk.platform

import android.content.Context
import android.content.SharedPreferences
import com.elon.app.AuthManager
import java.io.Closeable

/** An in-memory capture, not a second account store. Never print credential-bearing fields. */
internal class EskPlatformSession private constructor(
    val userId: String,
    val token: String,
    val expiresAtMillis: Long,
    val revision: String?,
    val displayName: String,
    private val account: String?,
    private val nickname: String?,
) {
    fun validAt(nowEpochMillis: Long): Boolean = nowEpochMillis >= 0 &&
        (expiresAtMillis == 0L || expiresAtMillis > nowEpochMillis)

    fun sameAs(other: EskPlatformSession?): Boolean = other != null &&
        userId == other.userId && token == other.token && expiresAtMillis == other.expiresAtMillis &&
        revision == other.revision && account == other.account && nickname == other.nickname

    override fun toString(): String = "EskPlatformSession(redacted)"

    companion object {
        private val revisionPattern = Regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")

        fun fromPreferences(values: Map<String, *>, nowEpochMillis: Long): EskPlatformSession? = runCatching {
            val token = values["auth_token"] as? String ?: return null
            val userId = values["auth_user_id"] as? String ?: return null
            require(token.length in 1..8192 && token.all { it.code in 33..126 })
            require(userId.length in 1..160 && userId == userId.trim() && userId.none(Char::isISOControl))
            val expiry = if (values.containsKey("auth_expires_at")) values["auth_expires_at"] as? Long ?: return null else 0L
            require(expiry >= 0)
            // Legacy sessions have no revision. Every subsequent save/clear writes a fresh UUID atomically.
            val revision = if (values.containsKey("auth_session_revision"))
                (values["auth_session_revision"] as? String)?.takeIf(revisionPattern::matches) ?: return null else null
            fun label(key: String): String? {
                if (!values.containsKey(key)) return null
                return (values[key] as? String)?.takeIf { it.length <= 1024 } ?: error("Invalid label")
            }
            val account = label("auth_account")
            val nickname = label("auth_nickname")
            val display = (nickname?.takeIf(String::isNotBlank) ?: account?.takeIf(String::isNotBlank)
                ?: "当前登录账户").filterNot(Char::isISOControl).take(64)
            EskPlatformSession(userId, token, expiry, revision, display, account, nickname)
                .takeIf { it.validAt(nowEpochMillis) }
        }.getOrNull()
    }
}

internal class EskPlatformSessionStore(
    private val preferences: SharedPreferences,
    private val onInvalidated: () -> Unit,
) : Closeable {
    constructor(context: Context, onInvalidated: () -> Unit) : this(AuthManager.prefs(context), onInvalidated)

    @Volatile private var closed = false
    private val listener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
        if (!closed && (key == null || key.startsWith("auth_"))) onInvalidated()
    }

    init {
        preferences.registerOnSharedPreferenceChangeListener(listener)
    }

    fun capture(nowEpochMillis: Long = System.currentTimeMillis()): EskPlatformSession? = runCatching {
        if (closed) return null
        val values = preferences.all // One atomic preference snapshot; never mix separate token/user reads.
        EskPlatformSession.fromPreferences(values, nowEpochMillis).takeUnless { closed }
    }.getOrNull()

    override fun close() {
        closed = true
        preferences.unregisterOnSharedPreferenceChangeListener(listener)
    }
}
