package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import android.widget.Toast
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.net.URLEncoder
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/** Cache first; fetching a catalog never changes the work-page model preference. */
internal class GroupWorkAiSettings(private val activity: AppCompatActivity, private val server: String,
    private val group: String, private val store: GroupAiConfigurationStore,
    private val current: () -> GroupWorkAiConfiguration, private val valid: () -> Boolean,
    private val save: (GroupWorkAiConfiguration) -> Unit, private val switchAi: () -> Unit) {
    private var sheet: ChatAiChoiceSheetHandle? = null
    private var models = store.workModels()
    private var closed = false

    fun show() {
        sheet = ChatAiChoiceSheet.show(activity, "工作 AI", choices(), onSelected = { id ->
            if (valid()) {
                val model = models.firstOrNull { "agent:${it.id}" == id }
                if (id == DEFAULT || model != null) save(current().copy(agent = model?.id, label = model?.label ?: "服务器默认"))
            }
        }, actions = listOf(ChatAiSheetAction("切换 AI", "group-ai-switch", switchAi)),
            toggle = ChatAiSheetToggle("模型不可用时使用备用 AI", "group-ai-work-fallback", current().allowFallback) {
                if (valid()) save(current().copy(allowFallback = it))
            })
        sheet?.dialog?.setOnDismissListener { closed = true }
        if (!store.workModelsFresh()) refresh()
    }

    private fun choices(): List<ChatAiChoice> = buildList {
        val selected = current().agent
        add(ChatAiChoice(DEFAULT, "服务器默认", "工作 AI", R.drawable.ic_msg_ai_reply, selected == null, "group-ai-work-model:default"))
        models.forEach { add(ChatAiChoice("agent:${it.id}", it.label, it.model, R.drawable.ic_msg_ai_reply,
            selected == it.id, "group-ai-work-model:${it.id}")) }
        if (selected != null && models.none { it.id == selected }) {
            add(ChatAiChoice(selected, current().label, "待确认可用性", R.drawable.ic_msg_ai_reply,
                true, "group-ai-work-model:unconfirmed", enabled = false))
        }
    }

    private fun refresh() = thread {
        val result = runCatching {
            val path = "/api/me/groups/${URLEncoder.encode(group, "UTF-8")}/ai/work-models"
            val request = AuthManager.applyAuth(activity, Request.Builder().url(server + path)).get().build()
            http.newCall(request).execute().use {
                check(it.isSuccessful)
                val response = JSONObject(it.body?.string().orEmpty())
                check(response.optInt("schema") == 1)
                GroupWorkAiModel.parse(response.getJSONArray("models"))
            }
        }
        activity.runOnUiThread {
            if (!valid()) return@runOnUiThread
            result.onSuccess {
                models = it
                store.saveWorkModels(it)
                if (!closed) sheet?.update?.invoke(choices())
            }.onFailure {
                if (!closed) Toast.makeText(activity, "模型目录暂未更新，保留已有选择", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun close() { closed = true; sheet?.dialog?.dismiss(); sheet = null }
    companion object {
        private const val DEFAULT = "group-default"
        private val http = OkHttpClient.Builder().callTimeout(12, TimeUnit.SECONDS).build()
    }
}
