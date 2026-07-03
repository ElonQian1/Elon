package com.elon.app

import com.google.gson.JsonObject
import org.json.JSONObject

internal fun structuredProcessEvidence(parsed: JSONObject): EvidenceEntry? {
    return structuredProcessEvidence(parsed.optString("type").takeIf { it.isNotBlank() }) { key ->
        parsed.optString(key).takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }
}

internal fun structuredProcessEvidence(type: String?, parsed: JsonObject): EvidenceEntry? {
    return structuredProcessEvidence(type) { key -> jsonStringOrNull(parsed, key) }
}

private fun structuredProcessEvidence(
    type: String?,
    value: (String) -> String?
): EvidenceEntry? {
    return when (type) {
        "pc_dispatch_started" -> {
            val cli = value("cli") ?: "codex"
            val node = value("agent_id") ?: value("node_id")
            val mode = value("mode")
            val pieces = buildList {
                add("已派发到 PC 节点")
                node?.let { add(it) }
                add("使用 $cli")
                mode?.let { add(it) }
            }
            EvidenceEntry("connection", pieces.joinToString(" · "))
        }
        "runtime_status" -> {
            val phase = value("phase") ?: value("status") ?: return null
            val node = value("node_id") ?: value("agent_id")
            EvidenceEntry("progress", listOfNotNull("运行状态：$phase", node).joinToString(" · "))
        }
        "runtime_summary" -> {
            val summary = value("summary") ?: value("message") ?: value("status") ?: return null
            EvidenceEntry("result", "运行摘要：${summarize(summary, 96)}")
        }
        else -> null
    }
}

internal fun usageEvidence(parsed: JSONObject): EvidenceEntry? {
    val total = parsed.optInt("total_tokens", 0).takeIf { it > 0 } ?: return null
    val model = parsed.optString("model").takeIf { it.isNotBlank() }
    return usageEvidence(model, total)
}

internal fun usageEvidence(parsed: JsonObject): EvidenceEntry? {
    val total = parsed.get("total_tokens")
        ?.takeIf { !it.isJsonNull }
        ?.let { runCatching { it.asInt }.getOrNull() }
        ?.takeIf { it > 0 }
        ?: return null
    val model = jsonStringOrNull(parsed, "model")
    return usageEvidence(model, total)
}

private fun usageEvidence(model: String?, total: Int): EvidenceEntry {
    val detail = if (model.isNullOrBlank()) {
        "模型用量：$total tokens"
    } else {
        "模型用量：$model · $total tokens"
    }
    return EvidenceEntry("result", detail)
}
