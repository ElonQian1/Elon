package com.elon.app

import android.content.Context
import android.util.AtomicFile
import java.io.File
import java.io.FileOutputStream
import org.json.JSONObject

internal class WebChatProviderDraftState(
    initialDrafts: Map<WebChatProviderId, String> = emptyMap(),
) {
    private val drafts = initialDrafts
        .mapValues { (_, value) -> normalize(value) }
        .filterValues(String::isNotBlank)
        .toMutableMap()

    fun remember(providerId: WebChatProviderId, value: CharSequence?): Boolean {
        val draft = normalize(value?.toString().orEmpty())
        val previous = drafts[providerId]
        if (draft.isBlank()) drafts.remove(providerId) else drafts[providerId] = draft
        return previous != drafts[providerId]
    }

    fun restore(providerId: WebChatProviderId): String = drafts[providerId].orEmpty()

    fun snapshot(): Map<WebChatProviderId, String> = drafts.toMap()

    internal companion object {
        const val MAX_DRAFT_LENGTH = 12_000

        private fun normalize(value: String): String = value.take(MAX_DRAFT_LENGTH)
    }
}

internal class WebChatProviderDraftStore(context: Context) {
    private val file = AtomicFile(File(context.noBackupFilesDir, FILE_NAME))

    fun restore(): WebChatProviderDraftState {
        val bytes = runCatching { file.readFully() }.getOrNull()
            ?.takeIf { it.size <= MAX_BYTES }
            ?: return WebChatProviderDraftState()
        val drafts = WebChatProviderDraftCodec.decode(bytes.toString(Charsets.UTF_8))
            ?: return WebChatProviderDraftState()
        return WebChatProviderDraftState(drafts)
    }

    fun save(state: WebChatProviderDraftState) {
        val payload = WebChatProviderDraftCodec.encode(state.snapshot()).toByteArray(Charsets.UTF_8)
        if (payload.size > MAX_BYTES) return
        val output: FileOutputStream = runCatching { file.startWrite() }.getOrNull() ?: return
        try {
            output.write(payload)
            file.finishWrite(output)
        } catch (_: Exception) {
            file.failWrite(output)
        }
    }

    private companion object {
        const val FILE_NAME = "web-chat-provider-drafts-v1.json"
        const val MAX_BYTES = 128 * 1024
    }
}

internal object WebChatProviderDraftCodec {
    private const val SCHEMA = "elon.web_chat.provider_drafts.v1"

    fun encode(drafts: Map<WebChatProviderId, String>): String = JSONObject()
        .put("schema", SCHEMA)
        .put("drafts", JSONObject().apply {
            WebChatProviderId.entries.forEach { providerId ->
                val draft = drafts[providerId]
                    ?.take(WebChatProviderDraftState.MAX_DRAFT_LENGTH)
                    ?.takeIf(String::isNotBlank)
                    ?: return@forEach
                put(providerId.wireValue, draft)
            }
        })
        .toString()

    fun decode(raw: String): Map<WebChatProviderId, String>? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        val values = root.optJSONObject("drafts") ?: return emptyMap()
        return buildMap {
            WebChatProviderId.entries.forEach { providerId ->
                val draft = values.optString(providerId.wireValue)
                    .take(WebChatProviderDraftState.MAX_DRAFT_LENGTH)
                if (draft.isNotBlank()) put(providerId, draft)
            }
        }
    }
}
