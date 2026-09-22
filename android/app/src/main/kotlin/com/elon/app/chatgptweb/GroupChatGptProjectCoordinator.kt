package com.elon.app.chatgptweb

import org.json.JSONObject

/** One leased project operation. Provider identity stays in the owning WebView. */
internal class GroupChatGptProjectCoordinator(
    private val command: (String, String) -> Unit,
    private val server: (JSONObject, (Result<JSONObject>) -> Unit) -> Unit,
    private val navigate: (String) -> Unit,
    private val changed: () -> Unit,
    private val failure: (GroupWebAiFailureReason) -> Unit,
    private val confirmRebuild: (() -> Unit) -> Unit,
    private val schedule: (Long, () -> Unit) -> Unit,
    private val observe: (String) -> Unit = {},
) {
    private var closed = false
    private var started = false
    private var attempts = 0
    private var scope = ""
    private var binding: JSONObject? = null
    private var callback: ((JSONObject) -> Unit)? = null
    private var requestId = ""
    private var target = ""
    private var validating = false
    private var validated = false
    private var committed = false
    val ready: Boolean get() = !closed && validated
    fun matches(url: String): Boolean = ready && url == target
    fun verifyForSend(done: (Boolean) -> Unit) {
        if (!ready) { done(false); return }
        privateRequest("read") { done(it.optBoolean("ok")) }
    }

    fun snapshot(value: ChatGptWebSnapshot, documentUrl: String) {
        if (closed || !value.authenticated || value.loginRequired) return
        if (!started) { started = true; identify(); return }
        if (target.isNotEmpty() && documentUrl == target && !validating && !validated) {
            validating = true
            privateRequest("read") { result ->
                if (!result.optBoolean("ok")) fail(result) else { validated = true; changed() }
            }
        }
    }

    private fun identify() {
        request(JSONObject().put("operation", "identity")) { result ->
            if (!result.optBoolean("ok")) {
                if (++attempts < 10 && result.optString("code") == "project_identity_unavailable") {
                    schedule(1000) { if (!closed) identify() }
                } else fail(result)
                return@request
            }
            scope = result.getString("accountScope")
            serverAction("acquire") { prepare() }
        }
    }

    private fun prepare() {
        when (binding?.optString("state")) {
            "empty" -> serverAction("create_begin") {
                privateRequest("create") { result ->
                    if (result.optBoolean("ok")) bindProject(result) else fail(result)
                }
            }
            "creating" -> privateRequest("reconcile") { result ->
                if (result.optBoolean("ok")) bindProject(result) else fail(result)
            }
            "ready" -> privateRequest("read") { result ->
                if (result.optBoolean("ok")) open()
                else if (result.optString("code") == "project_not_found") {
                    // A 404 for the conversation alone is not evidence the project was deleted.
                    privateRequest("read", withoutConversation = true) { project ->
                        if (project.optString("code") != "project_not_found") fail(result)
                        else confirmRebuild {
                            if (!closed) serverAction("rebuild", JSONObject().put("confirmed_missing", true)) { prepare() }
                        }
                    }
                } else fail(result)
            }
            else -> failure(GroupWebAiFailureReason.PROJECT)
        }
    }

    private fun bindProject(result: JSONObject) = serverAction("bind",
        JSONObject().put("project_id", result.getString("projectId"))) { open() }

    private fun open() {
        val current = requireNotNull(binding)
        val project = current.getString("project_id")
        val conversation = current.optString("conversation_id").takeUnless { it.isBlank() || it == "null" }
        target = if (conversation == null) "https://chatgpt.com/g/$project/project"
            else "https://chatgpt.com/g/$project/c/$conversation"
        navigate(target)
    }

    fun complete(documentUrl: String, done: () -> Unit) {
        if (closed || committed) return
        committed = true
        val id = GroupChatGptProjectRoute.conversationId(documentUrl) ?: run {
            observe("project_thread_unconfirmed"); done(); return
        }
        server(leasePayload("remember").put("conversation_id", id)) { journal ->
            if (closed) return@server
            if (journal.isFailure) { observe("project_thread_journal_failed"); done(); return@server }
            commitConversation(id, done)
        }
    }

    private fun commitConversation(id: String, done: () -> Unit) {
        privateRequest("read", conversation = id) { result ->
            if (!result.optBoolean("ok")) { observe("project_thread_unconfirmed"); done() }
            else {
                val payload = leasePayload("conversation").put("conversation_id", id)
                server(payload) { receipt ->
                    if (!closed) {
                        receipt.onSuccess { binding = it }
                        observe(if (receipt.isSuccess) "project_thread_saved" else "project_thread_save_pending")
                        done()
                    }
                }
            }
        }
    }

    private fun privateRequest(operation: String, withoutConversation: Boolean = false,
        conversation: String? = null, complete: (JSONObject) -> Unit) {
        val current = requireNotNull(binding)
        request(JSONObject().put("operation", operation).put("accountScope", scope)
            .put("bindingId", current.getString("binding_id")).put("generation", current.getLong("generation"))
            .put("name", current.optString("group_name", "群聊").take(60))
            .apply {
                if (!current.isNull("project_id")) put("projectId", current.getString("project_id"))
                if (!withoutConversation) {
                    val id = conversation ?: current.optString("conversation_id").takeUnless { it.isBlank() || it == "null" }
                    if (id != null) put("conversationId", id)
                }
            }, complete)
    }

    private fun request(payload: JSONObject, complete: (JSONObject) -> Unit) {
        if (closed) return
        check(callback == null)
        requestId = GroupWebAiCommandIds.next()
        callback = complete
        command(payload.toString(), requestId)
    }

    fun event(event: ChatGptWebEvent) {
        if (closed || event !is ChatGptWebEvent.CommandResult || event.action != "group_project_request" || event.requestId != requestId) return
        val next = callback ?: return
        callback = null
        val result = runCatching { JSONObject(event.detail) }.getOrElse { JSONObject().put("code", "project_unavailable") }
        if (!event.ok) result.put("ok", false)
        val code = result.optString("code")
        if (Regex("project_[a-z_]{1,48}").matches(code)) observe(code)
        next(result)
    }

    private fun serverAction(action: String, extra: JSONObject = JSONObject(), next: () -> Unit) {
        if (closed) return
        val payload = leasePayload(action)
        observe("project_server_$action")
        extra.keys().forEach { key -> payload.put(key, extra.get(key)) }
        server(payload) { result ->
            if (closed) {
                if (action == "acquire") result.onSuccess { binding = it; server(leasePayload("release")) { } }
                return@server
            }
            result.onSuccess { binding = it; next() }.onFailure {
                observe("project_server_${action}_failed"); failure(GroupWebAiFailureReason.PROJECT)
            }
        }
    }

    private fun leasePayload(action: String) = JSONObject().put("action", action).put("account_scope", scope).apply {
        binding?.let {
            put("lease_id", it.getString("lease_id")).put("generation", it.getLong("generation"))
            if (!it.isNull("project_id")) put("project_id", it.getString("project_id"))
        }
    }

    private fun fail(result: JSONObject) = failure(when (result.optString("code")) {
        "project_create_unknown", "project_create_unresolved", "project_reconciliation_incomplete" -> GroupWebAiFailureReason.PROJECT_UNKNOWN
        "project_auth_required", "project_identity_changed", "project_identity_unavailable" -> GroupWebAiFailureReason.LOGIN
        else -> GroupWebAiFailureReason.PROJECT
    })

    fun close() {
        if (closed) return
        closed = true
        callback = null
        if (binding != null) server(leasePayload("release")) { }
    }
}
