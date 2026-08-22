package com.elon.app

import android.content.Context
import java.security.MessageDigest
import org.json.JSONArray
import org.json.JSONObject

internal enum class WebChatRealtimeVoiceLaunchPlan {
    DIRECT,
    REFRESH_CONTROLS,
    RECOVER_SESSION,
}

internal data class WebChatRealtimeVoiceLaunchSnapshot(
    val entries: List<WebChatRealtimeVoiceLaunchEntry>,
)

internal data class WebChatRealtimeVoiceLaunchEntry(
    val providerId: WebChatProviderId,
    val conversationHash: String,
    val updatedAtMs: Long,
)

internal interface WebChatRealtimeVoiceLaunchStorage {
    fun restore(): WebChatRealtimeVoiceLaunchSnapshot?
    fun save(snapshot: WebChatRealtimeVoiceLaunchSnapshot)
}

/**
 * Stores only a capability hint. A live WebRTC session and command receipt are
 * document-bound and must always be created again.
 */
internal class WebChatRealtimeVoiceLaunchCache(
    private val storage: WebChatRealtimeVoiceLaunchStorage? = null,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val entries = linkedMapOf<Key, Long>()

    init {
        restore(storage?.restore())
    }

    fun observe(providerId: WebChatProviderId, state: WebChatConsumerState?) {
        val key = state?.confirmedVoiceKey(providerId) ?: return
        val now = nowMs()
        val previous = entries.put(key, now)
        trim(now)
        if (previous == null || now - previous >= PERSIST_REFRESH_INTERVAL_MS) persist()
    }

    fun plan(
        providerId: WebChatProviderId,
        state: WebChatConsumerState?,
        sessionReady: Boolean,
    ): WebChatRealtimeVoiceLaunchPlan {
        if (!sessionReady || state == null || !state.adapterCurrent) {
            return WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION
        }
        val key = state.conversationKey(providerId)
            ?: return WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION
        if (state.hasEnabledVoiceControl()) {
            observe(providerId, state)
            return WebChatRealtimeVoiceLaunchPlan.DIRECT
        }
        val observedAt = entries[key]
        return if (observedAt != null && nowMs() - observedAt in 0..HINT_FRESH_MS) {
            WebChatRealtimeVoiceLaunchPlan.REFRESH_CONTROLS
        } else {
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION
        }
    }

    fun clear(providerId: WebChatProviderId) {
        if (entries.keys.removeAll { it.providerId == providerId }) persist()
    }

    private fun restore(snapshot: WebChatRealtimeVoiceLaunchSnapshot?) {
        val now = nowMs()
        snapshot?.entries.orEmpty()
            .asSequence()
            .filter { entry -> now - entry.updatedAtMs in 0..MAX_RETENTION_MS }
            .take(MAX_ENTRIES)
            .forEach { entry ->
                entries[Key(entry.providerId, entry.conversationHash)] = entry.updatedAtMs
            }
    }

    private fun trim(now: Long) {
        entries.entries.removeAll { now - it.value !in 0..MAX_RETENTION_MS }
        while (entries.size > MAX_ENTRIES) entries.remove(entries.keys.first())
    }

    private fun persist() {
        storage?.save(WebChatRealtimeVoiceLaunchSnapshot(
            entries = entries.map { (key, updatedAtMs) ->
                WebChatRealtimeVoiceLaunchEntry(
                    providerId = key.providerId,
                    conversationHash = key.conversationHash,
                    updatedAtMs = updatedAtMs,
                )
            },
        ))
    }

    private fun WebChatConsumerState.confirmedVoiceKey(providerId: WebChatProviderId): Key? =
        conversationKey(providerId)?.takeIf { adapterCurrent && hasEnabledVoiceControl() }

    private fun WebChatConsumerState.conversationKey(providerId: WebChatProviderId): Key? {
        val page = WebChatProductionPageIdentity.from(this)
        if (page.pageKind != "conversation" || page.conversationId == null) return null
        return Key(providerId, hash("${providerId.wireValue}:${page.cacheKey}"))
    }

    private fun WebChatConsumerState.hasEnabledVoiceControl(): Boolean = controls.any { descriptor ->
        descriptor.control.semantic == REALTIME_VOICE_SEMANTIC && descriptor.control.enabled
    }

    private data class Key(
        val providerId: WebChatProviderId,
        val conversationHash: String,
    )

    private companion object {
        const val REALTIME_VOICE_SEMANTIC = "voice_mode"
        const val HINT_FRESH_MS = 30L * 60L * 1_000L
        const val PERSIST_REFRESH_INTERVAL_MS = 5L * 60L * 1_000L
        const val MAX_RETENTION_MS = 7L * 24L * 60L * 60L * 1_000L
        const val MAX_ENTRIES = 64

        fun hash(value: String): String = MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString("") { byte ->
                (byte.toInt() and 0xff).toString(16).padStart(2, '0')
            }
    }
}

internal class WebChatRealtimeVoiceLaunchSnapshotStore(
    context: Context,
) : WebChatRealtimeVoiceLaunchStorage {
    private val preferences = AuthManager.userDataPrefs(context.applicationContext)

    override fun restore(): WebChatRealtimeVoiceLaunchSnapshot? {
        val raw = preferences.getString(KEY, null) ?: return null
        if (raw.toByteArray(Charsets.UTF_8).size > MAX_BYTES) return null
        return WebChatRealtimeVoiceLaunchSnapshotCodec.decode(raw)
    }

    override fun save(snapshot: WebChatRealtimeVoiceLaunchSnapshot) {
        val raw = WebChatRealtimeVoiceLaunchSnapshotCodec.encode(snapshot)
        if (raw.toByteArray(Charsets.UTF_8).size > MAX_BYTES) return
        preferences.edit().putString(KEY, raw).apply()
    }

    private companion object {
        const val KEY = "web_chat_realtime_voice_launch_hints_v1"
        const val MAX_BYTES = 16 * 1024
    }
}

internal object WebChatRealtimeVoiceLaunchSnapshotCodec {
    private const val SCHEMA = "elon.web_chat.realtime_voice_hints.v1"
    private const val MAX_ENTRIES = 64
    private val SHA256 = Regex("[a-f0-9]{64}")

    fun encode(snapshot: WebChatRealtimeVoiceLaunchSnapshot): String = JSONObject()
        .put("schema", SCHEMA)
        .put("entries", JSONArray().apply {
            snapshot.entries.take(MAX_ENTRIES).forEach { entry ->
                put(JSONObject()
                    .put("provider", entry.providerId.wireValue)
                    .put("conversation_hash", entry.conversationHash)
                    .put("updated_at_ms", entry.updatedAtMs))
            }
        })
        .toString()

    fun decode(raw: String): WebChatRealtimeVoiceLaunchSnapshot? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        val values = root.optJSONArray("entries") ?: JSONArray()
        return WebChatRealtimeVoiceLaunchSnapshot(buildList {
            for (index in 0 until minOf(values.length(), MAX_ENTRIES)) {
                val value = values.optJSONObject(index) ?: continue
                val provider = WebChatProviderId.fromWireValue(value.optString("provider"))
                if (provider.wireValue != value.optString("provider")) continue
                val conversationHash = value.optString("conversation_hash")
                val updatedAtMs = value.optLong("updated_at_ms", -1L)
                if (!SHA256.matches(conversationHash) || updatedAtMs < 0L) continue
                add(WebChatRealtimeVoiceLaunchEntry(provider, conversationHash, updatedAtMs))
            }
        })
    }
}
