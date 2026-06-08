package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

internal class MainConversationIdentityActions(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val userId: String,
    private val activeProject: () -> AppProject,
    private val saveProjects: () -> Unit,
    private val copyText: (String, String) -> Unit,
    private val switchToLockedAgent: ((String) -> Unit)? = null
) {
    fun copyConversationIdentity(conversationIndex: Int) {
        val project = activeProject()
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        Thread {
            val remoteInfo = fetchConversationIdentity(project, conversation)
            val codexUri = remoteInfo?.codexThreadUri
                ?: conversation.codexThreadUri?.takeIf { it.isNotBlank() }
            if (!codexUri.isNullOrBlank() && codexUri != conversation.codexThreadUri) {
                conversation.codexThreadUri = codexUri
                saveProjects()
            }
            val text = conversationIdentityText(project, conversation, codexUri)
            activity.runOnUiThread {
                copyText("会话 ID", text)
                if (remoteInfo == null) {
                    Toast.makeText(activity, "已复制本机会话信息，Codex 链接暂未同步", Toast.LENGTH_SHORT).show()
                }
            }
        }.start()
    }

    /// 进入会话时调用：拉取会话身份信息，自动切换到首次使用的 CLI（软锁定）
    fun syncConversationAgentLock(conversationIndex: Int) {
        val project = activeProject()
        val conversation = project.conversations.getOrNull(conversationIndex) ?: return
        Thread {
            val remoteInfo = fetchConversationIdentity(project, conversation) ?: return@Thread
            // 更新 codexThreadUri
            if (!remoteInfo.codexThreadUri.isNullOrBlank() && remoteInfo.codexThreadUri != conversation.codexThreadUri) {
                conversation.codexThreadUri = remoteInfo.codexThreadUri
                saveProjects()
            }
            // 软锁定：有 lockedAgentName 则更新本地缓存并自动切换
            val locked = remoteInfo.lockedAgentName?.takeIf { it.isNotBlank() } ?: return@Thread
            if (locked != conversation.lockedAgentName) {
                conversation.lockedAgentName = locked
                saveProjects()
            }
            activity.runOnUiThread {
                switchToLockedAgent?.invoke(locked)
            }
        }.start()
    }

    private fun fetchConversationIdentity(
        project: AppProject,
        conversation: AppConversation
    ): ConversationIdentityInfo? {
        val url = "$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(project.id)}" +
            "/conversations/${urlPart(conversation.id)}/identity" +
            "?title=${urlPart(project.title)}&conversation_title=${urlPart(conversation.title)}"
        return runCatching {
            http.newCall(Request.Builder().url(url).get().build()).execute().use { response ->
                if (!response.isSuccessful) return@runCatching null
                val body = response.body?.string().orEmpty()
                val json = JSONObject(body)
                ConversationIdentityInfo(
                    codexThreadId = json.optString("codex_thread_id").takeIf { it.isNotBlank() },
                    codexThreadUri = json.optString("codex_thread_uri").takeIf { it.isNotBlank() },
                    lockedAgentName = json.optString("locked_agent_name").takeIf { it.isNotBlank() }
                )
            }
        }.getOrNull()
    }

    private fun conversationIdentityText(
        project: AppProject,
        conversation: AppConversation,
        codexThreadUri: String?
    ): String {
        return buildString {
            appendLine("项目：${project.title}")
            appendLine("project_id=${project.id}")
            appendLine("会话：${conversation.title}")
            appendLine("conversation_id=${conversation.id}")
            appendLine("codex_thread_uri=${codexThreadUri ?: "未同步"}")
        }.trim()
    }
}

private data class ConversationIdentityInfo(
    val codexThreadId: String?,
    val codexThreadUri: String?,
    val lockedAgentName: String? = null
)
