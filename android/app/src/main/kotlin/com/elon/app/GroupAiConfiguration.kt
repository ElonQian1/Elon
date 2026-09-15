package com.elon.app

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject
import java.security.MessageDigest

internal enum class GroupAiEngine(val label: String) {
    CHATGPT("ChatGPT"), WORK("工作 AI");
}

/** Persist intent, never a page-local control id or provider credential. */
internal data class GroupAiModelChoice(
    val label: String,
    val submenu: Boolean = false,
    val rangeIndex: Int? = null,
    val rangeCount: Int? = null,
)

internal data class GroupAiConfiguration(
    val engine: GroupAiEngine = GroupAiEngine.CHATGPT,
    val modelPath: List<GroupAiModelChoice> = emptyList(),
) {
    val label: String get() = if (engine == GroupAiEngine.WORK) engine.label
        else modelPath.lastOrNull()?.label?.let(WebChatModelControlPolicy::compactLabel) ?: "默认"
    val usesWebAi: Boolean get() = engine == GroupAiEngine.CHATGPT
}

/** Group-only preference namespace; personal/work AI preferences remain untouched. */
internal class GroupAiConfigurationStore(context: Context, server: String, owner: String) {
    private val prefs = context.getSharedPreferences("group_ai_composer_v1", 0)
    private val scope = key(server, owner)

    fun read(group: String): GroupAiConfiguration = runCatching {
        decode(JSONObject(prefs.getString("$scope:${key(group)}", "{}").orEmpty()))
    }.getOrDefault(GroupAiConfiguration())

    fun save(group: String, config: GroupAiConfiguration) {
        prefs.edit().putString("$scope:${key(group)}", encode(config).toString()).apply()
    }

    fun catalog(): WebChatProductionInteractionCache = WebChatProductionInteractionCache(
        object : WebChatProductionInteractionSnapshotStorage {
            override fun restore() = prefs.getString("$scope:catalog", null)?.let {
                if (it.length <= 128 * 1024) WebChatProductionInteractionSnapshotCodec.decode(it) else null
            }
            override fun save(snapshot: WebChatProductionInteractionSnapshot) {
                val raw = WebChatProductionInteractionSnapshotCodec.encode(snapshot)
                if (raw.length <= 128 * 1024) prefs.edit().putString("$scope:catalog", raw).apply()
            }
        },
    )

    companion object {
        fun key(vararg values: String): String = MessageDigest.getInstance("SHA-256")
            .digest(JSONArray(values.toList()).toString().toByteArray()).joinToString("") { "%02x".format(it) }

        fun encode(config: GroupAiConfiguration) = JSONObject().put("engine", config.engine.name)
            .put("model_path", JSONArray().apply {
                config.modelPath.take(6).forEach { choice ->
                    put(JSONObject().put("label", choice.label).put("submenu", choice.submenu)
                        .put("range_index", choice.rangeIndex).put("range_count", choice.rangeCount))
                }
            })

        fun decode(json: JSONObject): GroupAiConfiguration {
            val path = json.optJSONArray("model_path") ?: JSONArray()
            val choices = (0 until path.length().coerceAtMost(6)).mapNotNull { index ->
                val value = path.optJSONObject(index) ?: return@mapNotNull null
                val label = value.optString("label").trim().take(120)
                if (label.isBlank()) return@mapNotNull null
                val count = value.optInt("range_count").takeIf { it in 2..6 }
                val step = value.optInt("range_index", -1).takeIf { count != null && it in 0 until count }
                GroupAiModelChoice(label, value.optBoolean("submenu"), step, count.takeIf { step != null })
            }
            return GroupAiConfiguration(
                GroupAiEngine.entries.firstOrNull { it.name == json.optString("engine") } ?: GroupAiEngine.CHATGPT,
                choices,
            )
        }
    }
}
