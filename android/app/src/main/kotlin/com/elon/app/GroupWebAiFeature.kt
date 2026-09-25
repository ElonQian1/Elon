package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.elon.app.chatgptweb.GroupWebAiExecutor
import com.elon.app.chatgptweb.GroupWebAiAttachments
import com.elon.app.chatgptweb.GroupWebAiFailure
import com.elon.app.chatgptweb.GroupWebAiFailureReason
import com.google.android.material.snackbar.Snackbar
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder
import java.util.UUID
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

internal class GroupWebAiFeature(
    private val activity: AppCompatActivity,
    private val root: View,
    private val http: OkHttpClient,
    private val server: String,
    private val userId: () -> String,
    private val refresh: (String) -> Unit,
) : DefaultLifecycleObserver {
    private var busy = false
    private var executor: GroupWebAiExecutor? = null
    private var progress: Snackbar? = null
    private val preferences = activity.getSharedPreferences("group_web_ai_results_v1", 0)
    private val calls = http.newBuilder().callTimeout(25, TimeUnit.SECONDS).build()

    init { activity.lifecycle.addObserver(this) }

    fun confirm(group: AppGroup, configuration: GroupAiConfiguration = GroupAiConfiguration(), proceed: () -> Unit) {
        if (busy) { toast("群聊 AI 正在处理，请稍候"); return }
        if (preferences.contains(pendingKey(userId()))) { recover(); toast("先同步上一条 AI 回答"); return }
        busy = true
        AlertDialog.Builder(activity).setTitle("使用 ${configuration.engine.label} 回复群聊")
            .setMessage(GroupWebAiRequestPolicy.consent(configuration))
            .setNegativeButton("取消") { _, _ -> release() }
            .setOnCancelListener { release() }
            .setPositiveButton("继续") { _, _ -> busy = true; proceed() }.show()
    }

    fun release() { busy = false }

    fun beginWork(): Boolean {
        if (busy) { toast("群聊 AI 正在处理，请稍候"); return false }
        busy = true
        return true
    }

    fun prepare(group: AppGroup, messageId: String, configuration: GroupAiConfiguration = GroupAiConfiguration()) = confirm(group, configuration) {
        prepareRequest(group, messageId, configuration)
    }

    fun prepareSelected(group: AppGroup, source: String, selection: JSONObject, configuration: GroupAiConfiguration,
        projectMemory: Boolean = false): Boolean {
        if (preferences.contains(pendingKey(userId()))) { recover(); toast("先同步上一条 AI 回答"); return false }
        if (!configuration.usesWebAi || !beginWork()) return false
        prepareRequest(group, source, configuration, selection, projectMemory)
        return true
    }

    private fun prepareRequest(group: AppGroup, messageId: String, configuration: GroupAiConfiguration, selection: JSONObject? = null,
        projectMemory: Boolean = selection == null) {
        val operation = UUID.randomUUID().toString()
        val owner = userId()
        thread {
            val result = runCatching {
                post("${base(group.id)}/messages/${part(messageId)}/web-ai", JSONObject().put("operation_id", operation)
                    .apply { if (selection != null) put("selected_context", selection) })
                    .getJSONObject("request")
                    .also {
                        check(selection == null || it.optString("context_scope") == "selected") { "服务器未确认选区范围，未发送给 AI" }
                        check(selection == null || it.optJSONArray("attachments") != null) { "服务器尚未支持附件清单，请等待服务更新；未发送给 AI" }
                    }
            }
            activity.runOnUiThread {
                result.onSuccess { execute(it, operation, owner, configuration, projectMemory) {
                    prepareRequest(group, messageId, configuration, selection, projectMemory)
                } }
                    .onFailure { release(); toast(it.message?.take(200) ?: "创建 AI 请求失败，请稍后重试") }
            }
        }
    }

    /** Called on the existing message-upload worker; reservation and message are one transaction. */
    fun send(group: AppGroup, payload: JSONObject, configuration: GroupAiConfiguration = GroupAiConfiguration()): JSONObject {
        val operation = UUID.randomUUID().toString()
        val owner = userId()
        val response = post("${base(group.id)}/web-ai/messages", payload.put("operation_id", operation))
        val request = response.getJSONObject("request")
        activity.runOnUiThread { execute(request, operation, owner, configuration) {
            prepareRequest(group, request.getString("trigger_message_id"), configuration)
        } }
        return response.getJSONObject("message")
    }

    fun prepareWork(group: AppGroup, messageId: String, configuration: GroupAiConfiguration) {
        if (beginWork()) prepareRequest(group, messageId, configuration)
    }

    private fun execute(request: JSONObject, operation: String, owner: String, configuration: GroupAiConfiguration,
        projectMemory: Boolean = request.optString("context_scope") != "selected",
        restart: () -> Unit) {
        if (activity.isDestroyed || userId() != owner) { release(); return }
        if (!configuration.usesWebAi) { executeWork(request, operation, owner, configuration.work); return }
        busy = true
        progress = Snackbar.make(root, "${configuration.engine.label} 正在处理群聊回复", Snackbar.LENGTH_INDEFINITE)
            .setAction("取消") { executor?.cancel() }.also { it.show() }
        val prompt = request.getString("prompt") + if (projectMemory && configuration.engine == GroupAiEngine.CHATGPT)
            "\n\n本次群分析请求标记：${request.getString("id")}。请勿在回答中复述标记。" else ""
        executor = GroupWebAiExecutor(activity, prompt,
            attachments = GroupWebAiAttachments(activity, http, server, request.optJSONArray("attachments") ?: org.json.JSONArray(),
                { !activity.isDestroyed && userId() == owner }),
            configuration = configuration,
            projectServer = if (projectMemory && configuration.engine == GroupAiEngine.CHATGPT)
                GroupChatGptProjectClient(activity, http, server, request.getString("group_id"), operation, owner,
                    { userId() == owner })::action else null,
            authorize = { ready ->
                thread {
                    val result = runCatching {
                        check(userId() == owner)
                        val receipt = action(request, operation, "dispatch", provider = configuration.engine.providerId?.wireValue)
                        GroupWebAiRequestPolicy.permitted(configuration, receipt)
                    }
                    activity.runOnUiThread { ready(userId() == owner && result.getOrDefault(false)) }
                }
            },
            onResult = { text ->
                executor = null
                val pending = JSONObject().put("request", request).put("operation", operation).put("content", text)
                preferences.edit().putString(pendingKey(owner), pending.toString()).commit()
                deliver(pending, owner)
            },
            onFailure = { failure ->
                executor = null
                progress?.dismiss()
                release()
                if (failure.rejectedBeforeSend) {
                    // A lost retirement receipt cannot authorize another send. Keep the
                    // old reservation closed until the server confirms cancellation.
                    busy = true
                    thread {
                        val retired = runCatching {
                            check(userId() == owner)
                            action(request, operation, "not_sent").getString("state") == "cancelled"
                        }.getOrDefault(false)
                        activity.runOnUiThread {
                            release()
                            if (retired) showSelectedFailure(request, operation, owner, configuration, failure, projectMemory, restart)
                            else if (userId() == owner && !activity.isDestroyed) AlertDialog.Builder(activity)
                                .setTitle("本次没有发送")
                                .setMessage("输入框未接受内容，但未能确认取消发送授权。本次不会重新发送，请检查网络后重新选择消息。")
                                .setPositiveButton("知道了", null).show()
                        }
                    }
                } else if (failure.uncertain) {
                    thread { runCatching { if (userId() == owner) action(request, operation, "uncertain") } }
                    showSelectedFailure(request, operation, owner, configuration, failure, projectMemory, restart)
                } else if (failure.reason == GroupWebAiFailureReason.CANCELLED) {
                    thread { runCatching { if (userId() == owner) action(request, operation, "cancel") } }
                } else if (request.optString("context_scope") == "selected") {
                    showSelectedFailure(request, operation, owner, configuration, failure, projectMemory, restart)
                } else if (!activity.isDestroyed && userId() == owner) {
                    AlertDialog.Builder(activity).setTitle("${configuration.engine.label} 尚未就绪")
                        .setMessage("没有向 ${configuration.engine.label} 发送。可以取消，或使用备用 AI（可能计费）。")
                        .setNegativeButton("取消") { _, _ ->
                            thread { runCatching { action(request, operation, "cancel") } }
                        }
                        .setPositiveButton("使用备用 AI") { _, _ ->
                            thread {
                                val result = runCatching { check(userId() == owner); action(request, operation, "fallback") }
                                activity.runOnUiThread {
                                    toast(if (result.isSuccess) "备用 AI 正在回复" else "切换失败，未重复发送")
                                    refresh(request.getString("group_id"))
                                }
                            }
                        }.show()
                }
            },
        ).also { it.start() }
    }

    private fun showSelectedFailure(request: JSONObject, operation: String, owner: String,
        configuration: GroupAiConfiguration, failure: GroupWebAiFailure, projectMemory: Boolean, restart: () -> Unit) {
        if (activity.isDestroyed || userId() != owner) return
        busy = true
        val dismiss = {
            release()
            if (!failure.uncertain && !failure.rejectedBeforeSend) thread {
                runCatching { if (userId() == owner) action(request, operation, "cancel") }
            }
            Unit
        }
        val dialog = AlertDialog.Builder(activity).setTitle("AI 回答未发到群聊")
            .setMessage(failure.message)
            .setNegativeButton(if (failure.canRetry) "取消" else "知道了") { _, _ -> dismiss() }
            .setOnCancelListener { dismiss() }
        if (failure.canRetry) dialog.setPositiveButton("重试本次分析") { _, _ ->
            if (userId() == owner && !activity.isDestroyed) {
                if (failure.rejectedBeforeSend) restart()
                else execute(request, operation, owner, configuration, projectMemory, restart)
            }
            else release()
        }
        dialog.show().getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "group-ai-analysis-retry"
    }

    fun recover() {
        if (busy) return
        val owner = userId()
        val pending = preferences.getString(pendingKey(owner), null)?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return
        busy = true
        deliver(pending, owner)
    }

    private fun executeWork(request: JSONObject, operation: String, owner: String, options: GroupWorkAiConfiguration) {
        thread {
            val result = runCatching {
                check(userId() == owner)
                post("${base(request.getString("group_id"))}/web-ai/requests/${part(request.getString("id"))}",
                    JSONObject().put("operation_id", operation).put("action", "work").put("work_options", options.request()))
            }
            activity.runOnUiThread {
                release()
                if (userId() != owner || activity.isDestroyed) return@runOnUiThread
                toast(if (result.isSuccess) "工作 AI 正在回复" else "工作 AI 请求未确认，请先查看群消息或检查模型设置，勿重复发送")
                refresh(request.getString("group_id"))
            }
        }
    }

    private fun deliver(pending: JSONObject, owner: String) {
        DebugTraceStore.record("group_web_ai", mapOf("stage" to "deliver_started"))
        thread {
            val result = runCatching {
                check(userId() == owner)
                action(pending.getJSONObject("request"), pending.getString("operation"), "complete", pending.getString("content"))
            }
            if (result.isSuccess) preferences.edit().remove(pendingKey(owner)).commit()
            DebugTraceStore.record("group_web_ai", mapOf("stage" to if (result.isSuccess) "deliver_completed" else "deliver_failed"))
            activity.runOnUiThread {
                progress?.dismiss()
                release()
                if (userId() != owner || activity.isDestroyed) return@runOnUiThread
                if (result.isSuccess) refresh(pending.getJSONObject("request").getString("group_id"))
                else AlertDialog.Builder(activity).setTitle("回答尚未同步")
                    .setMessage("回答已保存在本机。可以重试，或放弃把这条回答发送到群里。")
                    .setPositiveButton("重试") { _, _ -> recover() }
                    .setNegativeButton("放弃同步") { _, _ -> preferences.edit().remove(pendingKey(owner)).apply() }
                    .show()
            }
        }
    }

    private fun action(request: JSONObject, operation: String, action: String, text: String? = null, provider: String? = null): JSONObject =
        post("${base(request.getString("group_id"))}/web-ai/requests/${part(request.getString("id"))}",
            JSONObject().put("operation_id", operation).put("action", action).apply {
                if (text != null) put("content", text)
                if (provider != null) put("web_provider", provider)
            })
            .getJSONObject("request")

    private fun post(path: String, payload: JSONObject): JSONObject {
        val request = AuthManager.applyAuth(activity, Request.Builder().url("$server$path")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))).build()
        return calls.newCall(request).execute().use {
            val body = it.body?.string().orEmpty()
            check(it.isSuccessful) {
                runCatching { JSONObject(body).optString("error").takeIf(String::isNotBlank)?.take(200) }.getOrNull() ?: "群聊请求未完成"
            }
            JSONObject(body)
        }
    }

    override fun onDestroy(owner: LifecycleOwner) { executor?.cancel(); progress?.dismiss() }
    private fun toast(text: String) { if (!activity.isDestroyed) Toast.makeText(activity, text, Toast.LENGTH_LONG).show() }
    private fun base(group: String) = "/api/me/groups/${part(group)}"
    private fun part(value: String) = URLEncoder.encode(value, "UTF-8")
    private fun pendingKey(owner: String) = "$server|$owner"

    companion object {
        fun mentionsAi(text: String): Boolean = text.replace('＠', '@').lowercase().let {
            it.contains("@el") || it.contains("@ai")
        }
    }
}
