package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

internal data class NodeModel(
    val modelId: String,
    val displayName: String,
    val nodeId: String,
    val nodeDisplayName: String,
    val nodeCapacityLabel: String,
    val nodeHardwareSummary: String,
    val nodeOwner: String,
    val contextLen: Int,
    val pricePerK: Double
)

internal data class NodeBalance(
    val balance: Double,
    val lifetime: Double
)

internal data class NodeMarketNode(
    val nodeId: String,
    val displayName: String,
    val shortId: String,
    val deviceName: String,
    val hardwareSummary: String,
    val models: List<NodeModel>,
    val allowedClis: List<String>,
    val online: Boolean,
    val canAcceptProject: Boolean,
    val projectCount: Int,
    val projectLimit: Int,
    val projectSlotsRemaining: Int,
    val diskFreeBytes: Long?,
    val capacityLabel: String,
    val capacityTone: String,
    val capacityWarnings: List<String>
)

internal data class NodeMarketSummary(
    val onlineNodes: Int,
    val projectReadyNodes: Int,
    val modelCount: Int,
    val warningNodes: Int
)

internal object NodeMarketCatalog {
    fun parseNodes(body: String): List<NodeMarketNode> {
        val arr = JSONObject(body).optJSONArray("nodes") ?: return emptyList()
        return (0 until arr.length()).map { i ->
            parseNode(arr.getJSONObject(i))
        }
    }

    fun summarize(nodes: List<NodeMarketNode>): NodeMarketSummary {
        return NodeMarketSummary(
            onlineNodes = nodes.count { it.online },
            projectReadyNodes = nodes.count { it.canAcceptProject },
            modelCount = nodes.sumOf { it.models.size },
            warningNodes = nodes.count {
                it.capacityTone.equals("warn", ignoreCase = true) ||
                    it.capacityTone.equals("bad", ignoreCase = true) ||
                    it.capacityWarnings.isNotEmpty()
            }
        )
    }

    private fun parseNode(o: JSONObject): NodeMarketNode {
        val nodeId = o.optString("node_id", o.optString("agent_id", "")).trim()
        val shortId = o.optString("short_id").ifBlank { formatNodeId(nodeId) }
        val deviceName = o.optString("device_name").trim()
        val displayName = o.optString("display_name")
            .ifBlank { o.optString("label") }
            .ifBlank { deviceName }
            .ifBlank { shortId }
        val capacityLabel = o.optString("capacity_label").trim()
        val hardwareSummary = o.optString("hardware_summary").ifBlank { "硬件未知" }
        val warnings = o.optJSONArray("capacity_warnings").toStringList()
        val allowedClis = o.optJSONArray("allowed_clis").toStringList()
        val projectCount = o.optInt("project_count", 0).coerceAtLeast(0)
        val projectLimit = o.optInt("project_limit", 0).coerceAtLeast(0)
        val node = NodeMarketNode(
            nodeId = nodeId,
            displayName = displayName,
            shortId = shortId,
            deviceName = deviceName,
            hardwareSummary = hardwareSummary,
            models = emptyList(),
            allowedClis = allowedClis,
            online = o.optBoolean("online", false),
            canAcceptProject = o.optBoolean("can_accept_project", false),
            projectCount = projectCount,
            projectLimit = projectLimit,
            projectSlotsRemaining = o.optInt(
                "project_slots_remaining",
                (projectLimit - projectCount).coerceAtLeast(0)
            ).coerceAtLeast(0),
            diskFreeBytes = if (o.has("disk_free_bytes") && !o.isNull("disk_free_bytes")) {
                o.optLong("disk_free_bytes").takeIf { it > 0L }
            } else {
                null
            },
            capacityLabel = capacityLabel,
            capacityTone = o.optString("capacity_tone").trim(),
            capacityWarnings = warnings
        )
        val modelsArr = o.optJSONArray("models") ?: JSONArray()
        val models = (0 until modelsArr.length()).mapNotNull { index ->
            val model = modelsArr.optJSONObject(index) ?: return@mapNotNull null
            val modelId = model.optString("model_id").trim()
            if (modelId.isBlank()) return@mapNotNull null
            NodeModel(
                modelId = modelId,
                displayName = model.optString("display_name").ifBlank { modelId },
                nodeId = nodeId,
                nodeDisplayName = displayName,
                nodeCapacityLabel = capacityLabel.ifBlank { if (node.online) "在线" else "离线" },
                nodeHardwareSummary = hardwareSummary,
                nodeOwner = o.optString("owner_user_id", ""),
                contextLen = model.optInt("context_len", 2048),
                pricePerK = model.optDouble("price_per_1k_credits", 1.0)
            )
        }
        return node.copy(models = models)
    }

    private fun JSONArray?.toStringList(): List<String> {
        val arr = this ?: return emptyList()
        return (0 until arr.length()).mapNotNull { idx ->
            arr.optString(idx).trim().takeIf { it.isNotBlank() }
        }
    }

    private fun formatNodeId(id: String): String {
        return if (id.length > 16) "...${id.takeLast(14)}" else id
    }
}
