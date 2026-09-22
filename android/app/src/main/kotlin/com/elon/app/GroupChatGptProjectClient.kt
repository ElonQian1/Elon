package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/** Sends only provider-account fingerprints and object IDs, never provider credentials. */
internal class GroupChatGptProjectClient(
    private val activity: AppCompatActivity,
    http: OkHttpClient,
    private val server: String,
    private val group: String,
    private val operation: String,
    private val owner: String,
    private val isOwner: () -> Boolean,
) {
    private val calls = http.newBuilder().callTimeout(15, TimeUnit.SECONDS).build()
    private val journal = activity.getSharedPreferences("group_chatgpt_thread_receipts_v1", 0)
    fun action(payload: JSONObject, complete: (Result<JSONObject>) -> Unit) {
        thread {
            val result = runCatching {
                check(isOwner())
                val key = GroupAiConfigurationStore.key(server, owner, group, payload.getString("account_scope"))
                val action = payload.getString("action")
                if (action in setOf("remember", "conversation")) check(journal.edit().putString(key, payload.toString()).commit())
                if (action == "remember") return@runCatching JSONObject()
                val path = "/api/me/groups/${URLEncoder.encode(group, "UTF-8")}/web-ai/project"
                val body = JSONObject(payload.toString()).put("operation_id", operation).toString()
                val request = AuthManager.applyAuth(activity, Request.Builder().url(server + path)
                    .post(body.toRequestBody("application/json".toMediaType()))).build()
                calls.newCall(request).execute().use { response ->
                    check(response.isSuccessful)
                    JSONObject(response.body?.string().orEmpty())
                }.also { receipt ->
                    check(isOwner())
                    if (action == "conversation" || action == "rebuild") journal.edit().remove(key).commit()
                    if (action == "acquire" && receipt.optString("state") == "ready" && receipt.isNull("conversation_id")) {
                        val pending = journal.getString(key, null)?.let { runCatching { JSONObject(it) }.getOrNull() }
                        if (pending != null && pending.optString("project_id") == receipt.optString("project_id") &&
                            pending.optLong("generation") == receipt.optLong("generation")) {
                            // The page revalidates project membership before this candidate is used.
                            receipt.put("conversation_id", pending.getString("conversation_id"))
                        }
                    }
                }
            }
            activity.runOnUiThread { complete(result) }
        }
    }
}
