package com.elon.app

import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import com.elon.app.chatgptweb.GroupWebAiExecutor
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

    fun confirm(group: AppGroup, proceed: () -> Unit) {
        if (busy) { toast("群聊 AI 正在处理，请稍候"); return }
        if (preferences.contains(pendingKey(userId()))) { recover(); toast("先同步上一条 AI 回答"); return }
        busy = true
        AlertDialog.Builder(activity).setTitle("使用 ChatGPT 回复群聊")
            .setMessage("将把这个群最近最多 30 条文字发送到本机 ChatGPT 临时会话，并把回答发回群里。不会读取个人会话，也不会读取群附件。")
            .setNegativeButton("取消") { _, _ -> release() }
            .setOnCancelListener { release() }
            .setPositiveButton("继续") { _, _ -> busy = true; proceed() }.show()
    }

    fun release() { busy = false }

    fun prepare(group: AppGroup, messageId: String) = confirm(group) {
        val operation = UUID.randomUUID().toString()
        val owner = userId()
        thread {
            val result = runCatching {
                post("${base(group.id)}/messages/${part(messageId)}/web-ai", JSONObject().put("operation_id", operation))
                    .getJSONObject("request")
            }
            activity.runOnUiThread {
                result.onSuccess { execute(it, operation, owner) }
                    .onFailure { release(); toast("创建 AI 请求失败，请稍后重试") }
            }
        }
    }

    /** Called on the existing message-upload worker; reservation and message are one transaction. */
    fun send(group: AppGroup, payload: JSONObject): JSONObject {
        val operation = UUID.randomUUID().toString()
        val owner = userId()
        val response = post("${base(group.id)}/web-ai/messages", payload.put("operation_id", operation))
        val request = response.getJSONObject("request")
        activity.runOnUiThread { execute(request, operation, owner) }
        return response.getJSONObject("message")
    }

    private fun execute(request: JSONObject, operation: String, owner: String) {
        if (activity.isDestroyed || userId() != owner) { release(); return }
        busy = true
        progress = Snackbar.make(root, "ChatGPT 正在处理群聊回复", Snackbar.LENGTH_INDEFINITE)
            .setAction("取消") { executor?.cancel() }.also { it.show() }
        executor = GroupWebAiExecutor(activity, request.getString("prompt"),
            authorize = { ready ->
                thread {
                    val result = runCatching {
                        check(userId() == owner)
                        action(request, operation, "dispatch").optBoolean("dispatch_permit")
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
            onFailure = { uncertain ->
                executor = null
                progress?.dismiss()
                release()
                if (uncertain) {
                    thread { runCatching { if (userId() == owner) action(request, operation, "uncertain") } }
                    toast("请求可能已发送，未重复提交。请稍后查看群消息")
                } else if (!activity.isDestroyed && userId() == owner) {
                    AlertDialog.Builder(activity).setTitle("ChatGPT 尚未可用")
                        .setMessage("没有向 ChatGPT 发送。可以取消，或使用备用 AI（可能计费）。")
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
        ).also { runCatching { it.start() }.onFailure { _ -> it.cancel() } }
    }

    fun recover() {
        if (busy) return
        val owner = userId()
        val pending = preferences.getString(pendingKey(owner), null)?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return
        busy = true
        deliver(pending, owner)
    }

    private fun deliver(pending: JSONObject, owner: String) {
        thread {
            val result = runCatching {
                check(userId() == owner)
                action(pending.getJSONObject("request"), pending.getString("operation"), "complete", pending.getString("content"))
            }
            if (result.isSuccess) preferences.edit().remove(pendingKey(owner)).commit()
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

    private fun action(request: JSONObject, operation: String, action: String, text: String? = null): JSONObject =
        post("${base(request.getString("group_id"))}/web-ai/requests/${part(request.getString("id"))}",
            JSONObject().put("operation_id", operation).put("action", action).apply { if (text != null) put("content", text) })
            .getJSONObject("request")

    private fun post(path: String, payload: JSONObject): JSONObject {
        val request = AuthManager.applyAuth(activity, Request.Builder().url("$server$path")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))).build()
        return calls.newCall(request).execute().use {
            check(it.isSuccessful) { "群聊请求未完成" }
            JSONObject(it.body?.string().orEmpty())
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
