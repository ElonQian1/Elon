package com.elon.app

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal interface WebChatProductionInteractionSnapshotStorage {
    fun restore(): WebChatProductionInteractionSnapshot?
    fun save(snapshot: WebChatProductionInteractionSnapshot)
}

/** User-scoped, non-secret metadata cache. SharedPreferences.apply keeps writes off the UI path. */
internal class WebChatProductionInteractionSnapshotStore(
    context: Context,
) : WebChatProductionInteractionSnapshotStorage {
    private val preferences = AuthManager.userDataPrefs(context.applicationContext)

    override fun restore(): WebChatProductionInteractionSnapshot? {
        val raw = preferences.getString(KEY, null) ?: return null
        if (raw.toByteArray(Charsets.UTF_8).size > MAX_BYTES) return null
        return WebChatProductionInteractionSnapshotCodec.decode(raw)
    }

    override fun save(snapshot: WebChatProductionInteractionSnapshot) {
        val raw = WebChatProductionInteractionSnapshotCodec.encode(snapshot)
        if (raw.toByteArray(Charsets.UTF_8).size > MAX_BYTES) return
        preferences.edit().putString(KEY, raw).apply()
    }

    private companion object {
        const val KEY = "web_chat_production_interaction_snapshot_v1"
        const val MAX_BYTES = 128 * 1024
    }
}

internal object WebChatProductionInteractionSnapshotCodec {
    private const val SCHEMA = "elon.web_chat.production_interactions.v1"
    private const val MAX_COMPOSER_GROUPS = 8
    private const val MAX_OPTIONS = 48
    private const val MAX_FEATURE_GROUPS = 2
    private const val MAX_FEATURES = 60
    private const val MAX_ID = 180
    private const val MAX_LABEL = 120
    private const val MAX_SEMANTIC = 64
    private const val MAX_SELECTOR = 220
    private val SECTION = Regex("[a-z0-9_-]{1,32}")

    fun encode(snapshot: WebChatProductionInteractionSnapshot): String = JSONObject()
        .put("schema", SCHEMA)
        .put("composer", JSONArray().apply {
            snapshot.composer.take(MAX_COMPOSER_GROUPS).forEach { group ->
                put(JSONObject()
                    .put("provider", group.providerId.wireValue)
                    .put("section", group.section.normalizedSection())
                    .put("updated_at_ms", group.updatedAtMs)
                    .put("options", JSONArray().apply {
                        group.options.take(MAX_OPTIONS).forEach { put(optionJson(it)) }
                    }))
            }
        })
        .put("features", JSONArray().apply {
            snapshot.features.take(MAX_FEATURE_GROUPS).forEach { group ->
                put(JSONObject()
                    .put("provider", group.providerId.wireValue)
                    .put("updated_at_ms", group.updatedAtMs)
                    .put("items", JSONArray().apply {
                        group.features.take(MAX_FEATURES).forEach { put(featureJson(it)) }
                    }))
            }
        })
        .toString()

    fun decode(raw: String): WebChatProductionInteractionSnapshot? {
        val root = runCatching { JSONObject(raw) }.getOrNull() ?: return null
        if (root.optString("schema") != SCHEMA) return null
        return WebChatProductionInteractionSnapshot(
            composer = decodeComposer(root.optJSONArray("composer")),
            features = decodeFeatures(root.optJSONArray("features")),
        )
    }

    private fun decodeComposer(groups: JSONArray?): List<WebChatProductionComposerSnapshot> =
        buildList {
            if (groups == null) return@buildList
            for (index in 0 until minOf(groups.length(), MAX_COMPOSER_GROUPS)) {
                val group = groups.optJSONObject(index) ?: continue
                val providerId = provider(group.optString("provider")) ?: continue
                val section = group.optString("section").normalizedSection()
                    .takeIf(SECTION::matches) ?: continue
                val updatedAtMs = group.optLong("updated_at_ms", -1L).takeIf { it >= 0L } ?: continue
                val options = decodeOptions(group.optJSONArray("options"))
                if (options.isNotEmpty()) add(WebChatProductionComposerSnapshot(
                    providerId = providerId,
                    section = section,
                    updatedAtMs = updatedAtMs,
                    options = options,
                ))
            }
        }

    private fun decodeFeatures(groups: JSONArray?): List<WebChatProductionFeatureSnapshot> =
        buildList {
            if (groups == null) return@buildList
            for (index in 0 until minOf(groups.length(), MAX_FEATURE_GROUPS)) {
                val group = groups.optJSONObject(index) ?: continue
                val providerId = provider(group.optString("provider")) ?: continue
                val updatedAtMs = group.optLong("updated_at_ms", -1L).takeIf { it >= 0L } ?: continue
                val features = decodeFeatureItems(group.optJSONArray("items"))
                if (features.isNotEmpty()) add(WebChatProductionFeatureSnapshot(
                    providerId = providerId,
                    updatedAtMs = updatedAtMs,
                    features = features,
                ))
            }
        }

    private fun optionJson(option: WebChatConsumerOption) = JSONObject()
        .put("id", option.id.take(MAX_ID))
        .put("label", option.label.take(MAX_LABEL))
        .put("semantic", option.semantic.take(MAX_SEMANTIC))
        .put("opens_submenu", option.opensSubmenu)
        .put("native_selector", option.nativeSelector.take(MAX_SELECTOR))
        .put("parent_id", option.parentId?.take(MAX_ID) ?: JSONObject.NULL)
        .put("parent_label", option.parentLabel?.take(MAX_LABEL) ?: JSONObject.NULL)

    private fun decodeOptions(values: JSONArray?): List<WebChatConsumerOption> = buildList {
        if (values == null) return@buildList
        for (index in 0 until minOf(values.length(), MAX_OPTIONS)) {
            val value = values.optJSONObject(index) ?: continue
            val id = value.optString("id").trim().take(MAX_ID)
            val label = value.optString("label").trim().take(MAX_LABEL)
            if (id.isBlank() || label.isBlank()) continue
            add(WebChatConsumerOption(
                id = id,
                label = label,
                selected = false,
                semantic = value.optString("semantic").trim().take(MAX_SEMANTIC),
                opensSubmenu = value.optBoolean("opens_submenu", false),
                nativeSelector = value.optString("native_selector").trim().take(MAX_SELECTOR),
                parentId = value.nullableString("parent_id", MAX_ID),
                parentLabel = value.nullableString("parent_label", MAX_LABEL),
            ))
        }
    }.distinctBy(WebChatConsumerOption::id)

    private fun featureJson(feature: WebChatConsumerFeature) = JSONObject()
        .put("id", feature.id.take(MAX_ID))
        .put("label", feature.label.take(MAX_LABEL))
        .put("kind", feature.kind.take(MAX_SEMANTIC))
        .put("requires_confirmation", feature.requiresUserConfirmation)
        .put("native_selector", feature.nativeSelector.take(MAX_SELECTOR))

    private fun decodeFeatureItems(values: JSONArray?): List<WebChatConsumerFeature> = buildList {
        if (values == null) return@buildList
        for (index in 0 until minOf(values.length(), MAX_FEATURES)) {
            val value = values.optJSONObject(index) ?: continue
            val id = value.optString("id").trim().take(MAX_ID)
            val label = value.optString("label").trim().take(MAX_LABEL)
            if (id.isBlank() || label.isBlank()) continue
            add(WebChatConsumerFeature(
                id = id,
                label = label,
                kind = value.optString("kind").trim().take(MAX_SEMANTIC),
                selected = false,
                requiresUserConfirmation = value.optBoolean("requires_confirmation", false),
                nativeSelector = value.optString("native_selector").trim().take(MAX_SELECTOR),
            ))
        }
    }.distinctBy(WebChatConsumerFeature::id)

    private fun provider(value: String): WebChatProviderId? {
        val provider = WebChatProviderId.fromWireValue(value)
        return provider.takeIf { it.wireValue == value }
    }

    private fun JSONObject.nullableString(key: String, maxLength: Int): String? =
        takeIf { has(key) && !isNull(key) }
            ?.optString(key)
            ?.trim()
            ?.take(maxLength)
            ?.takeIf(String::isNotBlank)

    private fun String.normalizedSection(): String = trim().lowercase()
}
