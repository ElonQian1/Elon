package com.elon.app

import kotlinx.coroutines.CancellationException
import org.json.JSONObject

/** Only explicitly bound tasks are read. Provider cookies and raw result metadata never leave the page. */
internal class GroupAssistantSync(
    private val api: GroupAssistantApi,
    private val read: suspend (JSONObject) -> JSONObject,
) {
    suspend fun latest(id: String, scope: String): JSONObject = read(JSONObject().put("operation", "latest")
        .put("id", id).put("expectedScope", scope).put("force", true)).getJSONObject("data")

    suspend fun one(group: String, binding: JSONObject) {
        val task = binding.getString("task_id")
        val scope = binding.getString("account_scope")
        val payload = JSONObject().put("task_id", task).put("account_scope", scope)
        try {
            val data = latest(task, scope)
            val update = data.optJSONObject("update")
            val state = when (data.getString("state")) { "update" -> "ready"; "no_update" -> "no_update"; else -> "requires_action" }
            payload.put("state", state)
            if (state == "ready" && update != null) payload.put("update", JSONObject()
                .put("id", update.getString("id")).put("created_at", update.getString("createdAt"))
                .put("content", update.getString("contentText")))
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) {
            payload.put("state", when (error.message) {
                "tasks_not_found" -> "missing"
                "tasks_account_changed", "tasks_auth_required", "tasks_auth_unavailable" -> "auth_required"
                else -> "unavailable"
            })
        }
        api.call(group, binding.getString("id"), payload)
    }
}
