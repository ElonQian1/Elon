package com.elon.app.mcp

import android.content.Context
import com.elon.app.AppConversation
import com.elon.app.AppProject
import com.elon.app.AuthManager
import com.elon.app.ChatMessage
import com.elon.app.CodexInteractionPresentation
import com.elon.app.DebugTraceStore
import com.elon.app.ELON_SELF_PROJECT_ID
import com.elon.app.ProjectRequestExecutionMode
import com.elon.app.defaultAppConversation
import com.elon.app.initialWorkflowMessage
import com.elon.app.loadStoredProjects
import com.elon.app.projectSpaceId
import com.elon.app.saveStoredProjects
import com.elon.app.summarize
import com.elon.app.welcomeChatMessage
import com.google.gson.Gson
import org.json.JSONObject

internal data class McpConversationSeed(
    val traceId: String,
    val projectId: String,
    val projectTitle: String,
    val conversationId: String,
    val conversationTitle: String?,
    val message: String,
    val isDevelopment: Boolean,
    val executionMode: ProjectRequestExecutionMode
)

internal data class McpConversationSeedApplyResult(
    val appended: Boolean,
    val projectId: String,
    val projectTitle: String,
    val projectIndex: Int,
    val conversationId: String,
    val conversationTitle: String,
    val conversationIndex: Int,
    val messageCount: Int
) {
    fun toJson(): JSONObject {
        return JSONObject()
            .put("seeded", true)
            .put("appended", appended)
            .put("project_id", projectId)
            .put("project_title", projectTitle)
            .put("project_index", projectIndex)
            .put("conversation_id", conversationId)
            .put("conversation_title", conversationTitle)
            .put("conversation_index", conversationIndex)
            .put("message_count", messageCount)
    }
}

internal fun seedMcpConversation(context: Context, seed: McpConversationSeed): JSONObject {
    val prefs = AuthManager.userDataPrefs(context)
    val gson = Gson()
    val loaded = loadStoredProjects(
        prefs = prefs,
        gson = gson,
        normalizeProject = ::normalizeMcpStoredProject
    )
    val projects = loaded.projects
    val now = System.currentTimeMillis()
    val result = applyMcpConversationSeed(projects, seed, now)
    saveStoredProjects(prefs, gson, projects, result.projectIndex, result.projectId, synchronous = true)

    DebugTraceStore.record(
        "mcp_conversation_seeded",
        mapOf(
            "trace_id" to seed.traceId,
            "project_id" to result.projectId,
            "conversation_id" to result.conversationId,
            "appended" to result.appended,
            "message_count" to result.messageCount
        )
    )

    return result.toJson()
}

internal fun applyMcpConversationSeed(
    projects: MutableList<AppProject>,
    seed: McpConversationSeed,
    now: Long
): McpConversationSeedApplyResult {
    val projectIndex = ensureMcpProject(projects, seed.projectId, seed.projectTitle, now)
    val project = projects[projectIndex]
    val conversationIndex = ensureMcpConversation(project, seed, now)
    val conversation = project.conversations[conversationIndex]
    val alreadySeeded = conversation.messages.any { it.id == mcpSeedMessageId(seed.traceId, "user") }

    if (!alreadySeeded) {
        if (conversation.messages.size == 1 && conversation.messages.first().content == welcomeChatMessage().content) {
            conversation.messages.clear()
        }
        conversation.messages.add(
            ChatMessage(
                role = "user",
                content = seed.message,
                id = mcpSeedMessageId(seed.traceId, "user")
            )
        )
        if (seed.isDevelopment) {
            val intent = CodexInteractionPresentation.intentMessage(
                visibleText = seed.message,
                outgoingText = seed.message,
                isDevelopment = true,
                executionMode = seed.executionMode,
                hasAttachments = false
            )
            intent.id = mcpSeedMessageId(seed.traceId, "intent")
            intent.evidenceWorking = true
            conversation.messages.add(intent)
        }
        conversation.messages.add(
            ChatMessage(
                role = "ai-working",
                content = initialWorkflowMessage(seed.isDevelopment),
                id = mcpSeedMessageId(seed.traceId, "working")
            )
        )
    }

    conversation.title = conversation.title.takeIf { it.isNotBlank() }
        ?: mcpConversationTitle(seed)
    conversation.subtitle = summarize(seed.message, 30)
    conversation.updatedAt = now
    project.activeConversationIndex = conversationIndex
    project.subtitle = summarize(seed.message, 34)
    project.updatedAt = now

    return McpConversationSeedApplyResult(
        appended = !alreadySeeded,
        projectId = project.id,
        projectTitle = project.title,
        projectIndex = projectIndex,
        conversationId = conversation.id,
        conversationTitle = conversation.title,
        conversationIndex = conversationIndex,
        messageCount = conversation.messages.size
    )
}

internal fun openSeededMcpConversationInUi(
    context: Context,
    seed: McpConversationSeed,
    showInUi: Boolean
): JSONObject {
    if (!showInUi) {
        return JSONObject()
            .put("attempted", false)
            .put("reason", "disabled")
    }
    val result = McpNativeControlBridge.control(
        context,
        JSONObject()
            .put("action", "seed_project_chat")
            .put("trace_id", seed.traceId)
            .put("project_id", seed.projectId)
            .put("project_title", seed.projectTitle)
            .put("conversation_id", seed.conversationId)
            .put("conversation_title", seed.conversationTitle ?: JSONObject.NULL)
            .put("message", seed.message)
            .put("is_development", seed.isDevelopment)
            .put("execution_mode", seed.executionMode.wireValue)
    )
    return JSONObject()
        .put("attempted", true)
        .put("opened", !result.has("error") && result.optBoolean("control_ok", true))
        .put("state", result)
}

internal fun generatedMcpConversationId(): String {
    return "mcp_${System.currentTimeMillis()}_${java.util.UUID.randomUUID().toString().take(8)}"
}

internal fun mcpConversationTitle(seed: McpConversationSeed): String {
    return mcpConversationTitle(seed.message, seed.conversationTitle)
}

internal fun mcpConversationTitle(message: String, requestedTitle: String? = null): String {
    return requestedTitle
        ?.trim()
        ?.takeIf { it.isNotBlank() }
        ?: summarize(message, 24).ifBlank { "MCP 调试会话" }
}

internal fun mcpExecutionMode(args: JSONObject): ProjectRequestExecutionMode {
    val execution = args.optString("execution_mode")
        .trim()
        .lowercase()
    return if (args.optBoolean("plan_mode", false) || execution == ProjectRequestExecutionMode.Plan.wireValue) {
        ProjectRequestExecutionMode.Plan
    } else {
        ProjectRequestExecutionMode.Execute
    }
}

private fun ensureMcpProject(
    projects: MutableList<AppProject>,
    projectId: String,
    projectTitle: String,
    now: Long
): Int {
    val existing = projects.indexOfFirst { it.id == projectId || it.projectSpaceId() == projectId }
    if (existing >= 0) {
        val project = projects[existing]
        if (project.title.isBlank()) project.title = summarize(projectTitle, 24)
        return existing
    }
    val project = AppProject(
        id = projectId,
        title = summarize(projectTitle, 24).ifBlank { "MCP 调试项目" },
        subtitle = "MCP 调试会话",
        updatedAt = now,
        isJointProject = projectId == ELON_SELF_PROJECT_ID,
        conversations = mutableListOf()
    )
    projects.add(project)
    return projects.lastIndex
}

private fun ensureMcpConversation(
    project: AppProject,
    seed: McpConversationSeed,
    now: Long
): Int {
    val existing = project.conversations.indexOfFirst { it.id == seed.conversationId }
    if (existing >= 0) {
        val conversation = project.conversations[existing]
        if (conversation.title.isBlank() || conversation.title == "MCP 调试会话") {
            conversation.title = mcpConversationTitle(seed.message, seed.conversationTitle)
        }
        return existing
    }
    project.conversations.add(
        AppConversation(
            id = seed.conversationId,
            title = mcpConversationTitle(seed.message, seed.conversationTitle),
            subtitle = "MCP 调试会话",
            updatedAt = now,
            messages = mutableListOf()
        )
    )
    return project.conversations.lastIndex
}

private fun normalizeMcpStoredProject(project: AppProject) {
    if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
    project.conversations.forEach { conversation ->
        if (conversation.messages.isEmpty()) conversation.messages.add(welcomeChatMessage())
        conversation.messages.forEach { message ->
            message.evidenceWorking = false
            message.sendStatus = null
        }
    }
    project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
    if (project.stage.isBlank()) project.stage = "待提交需求"
    if (project.subtitle.isBlank()) project.subtitle = "点击进入会话"
}

private fun mcpSeedMessageId(traceId: String, suffix: String): String = "mcp:$traceId:$suffix"
