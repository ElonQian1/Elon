package com.elon.app

import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.mcp.McpConversationSeed
import com.elon.app.mcp.applyMcpConversationSeed
import com.elon.app.mcp.mcpExecutionMode
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

internal class MainMcpNativeControlActions(
    private val binding: ActivityMainBinding,
    private val projects: MutableList<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val activeConversationIndex: () -> Int,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val saveProjects: () -> Unit,
    private val reloadProjects: () -> Unit,
    private val renderConversationList: () -> Unit,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val pauseCurrentWork: () -> Unit,
    private val showMessageActions: (android.view.View, ChatMessage) -> Unit,
    private val retryFailedAttachmentMessage: (ChatMessage) -> Unit,
    private val navigationController: () -> MainNavigationController,
    private val sendMessage: () -> Unit,
    private val waitingForReply: () -> Boolean,
    private val backendConnected: () -> Boolean,
    private val activeRequestIsDevelopment: () -> Boolean,
    private val runningTaskCount: () -> Int,
    private val currentStage: () -> String,
    private val rememberMcpConversationSeed: (McpConversationSeed) -> Unit = {}
) {
    fun uiState(): JSONObject {
        val project = activeProject()
        val conversation = activeConversation()
        val lastMessage = conversation.messages.lastOrNull()
        return JSONObject()
            .put("active_page", activePage())
            .put("toolbar_title", binding.topTitleText.text?.toString().orEmpty())
            .put("bottom_tab", activeBottomTab())
            .put("project_top_tab", activeProjectTopTab())
            .put("visibility", visibilityJson())
            .put("active_project", projectJson(project, activeProjectIndex()))
            .put("project_count", projects.size)
            .put("projects", projectsJson())
            .put("active_conversation", conversationJson(conversation, activeConversationIndex(), lastMessage))
            .put("input", inputJson())
            .put("runtime", runtimeJson())
    }

    fun control(args: JSONObject): JSONObject {
        val action = args.optString("action", "state").trim().lowercase(Locale.ROOT)
        DebugTraceStore.record(
            "mcp_native_ui_control",
            mapOf(
                "action" to action,
                "project_id" to args.optString("project_id").takeIf { it.isNotBlank() },
                "conversation_id" to args.optString("conversation_id").takeIf { it.isNotBlank() }
            )
        )
        val result = when (action) {
            "state" -> uiState()
            "reload_projects" -> {
                reloadProjects()
                renderConversationList()
                uiState()
            }
            "show_conversation_home" -> {
                navigationController().showConversationHome(animate = false)
                uiState()
            }
            "show_project_home" -> {
                navigationController().showProjectHome(animate = false)
                uiState()
            }
            "show_project_plaza" -> {
                navigationController().showProjectPlaza()
                uiState()
            }
            "open_project_chat" -> {
                val beforeProject = activeProject()
                val beforeConversationId = activeConversation().id
                val wasTargetChatOpen = activePage() == "chat"
                reloadTargetConversationIfNeeded(args)
                selectProject(args)?.let { return it }
                selectConversation(args, createIfMissing = false)?.let { return it }
                if (!isSameOpenProjectChat(
                        wasTargetChatOpen,
                        beforeProject,
                        beforeConversationId
                    )
                ) {
                    openActiveProjectConversation()
                }
                uiState()
            }
            "seed_project_chat" -> seedProjectChat(args)
            "new_project_conversation" -> {
                selectProject(args)?.let { return it }
                createProjectConversation(args)
                openActiveProjectConversation()
                uiState()
            }
            "set_input_text" -> {
                binding.inputEdit.setText(args.optString("text"))
                binding.inputEdit.setSelection(binding.inputEdit.text.length)
                uiState()
            }
            "send_input" -> {
                sendMessage()
                uiState()
            }
            "send_project_message" -> {
                selectProject(args)?.let { return it }
                val createConversation = args.optBoolean("new_conversation", false)
                selectConversation(args, createIfMissing = createConversation)?.let { return it }
                if (createConversation && args.optString("conversation_id").isBlank()) {
                    createProjectConversation(args)
                }
                openActiveProjectConversation()
                binding.inputEdit.setText(args.optString("message"))
                binding.inputEdit.setSelection(binding.inputEdit.text.length)
                sendMessage()
                uiState()
            }
            else -> return errorJson(action, "unsupported_action")
        }
        return result.put("control_ok", true)
    }

    private fun seedProjectChat(args: JSONObject): JSONObject {
        val seed = McpConversationSeed(
            traceId = args.optString("trace_id").trim(),
            projectId = args.optString("project_id").trim().ifBlank { ELON_SELF_PROJECT_ID },
            projectTitle = args.optString("project_title").trim().ifBlank { "Elon debug project" },
            conversationId = args.optString("conversation_id").trim().ifBlank { "default" },
            conversationTitle = args.optString("conversation_title").trim().takeIf { it.isNotBlank() },
            message = args.optString("message"),
            isDevelopment = if (args.has("is_development")) args.optBoolean("is_development") else true,
            executionMode = mcpExecutionMode(args)
        )
        rememberMcpConversationSeed(seed)
        val result = applyMcpConversationSeed(projects, seed, System.currentTimeMillis())
        setActiveProjectIndex(result.projectIndex)
        activeProject().activeConversationIndex = result.conversationIndex
        saveProjects()
        renderConversationList()
        openActiveProjectConversation()
        return uiState().put("conversation_seed", result.toJson())
    }

    private fun reloadTargetConversationIfNeeded(args: JSONObject) {
        if (!args.optBoolean("reload_if_missing", true)) return
        val requestedProject = localProjectForArgs(args)
        if (requestedProject == null) {
            reloadProjects()
            return
        }
        val hasConversationTarget = args.optString("conversation_id").isNotBlank() || args.has("conversation_index")
        if (!hasConversationTarget) return
        if (!localConversationExists(requestedProject, args)) {
            reloadProjects()
        }
    }

    private fun localProjectForArgs(args: JSONObject): AppProject? {
        val requestedId = args.optString("project_id").trim().takeIf { it.isNotBlank() }
        val requestedIndex = if (args.has("project_index")) args.optInt("project_index") else null
        return when {
            requestedId != null -> projects.firstOrNull { it.id == requestedId || it.projectSpaceId() == requestedId }
            requestedIndex != null -> projects.getOrNull(requestedIndex)
            else -> projects.getOrNull(activeProjectIndex())
        }
    }

    private fun localConversationExists(project: AppProject, args: JSONObject): Boolean {
        val requestedId = args.optString("conversation_id").trim().takeIf { it.isNotBlank() }
        val requestedIndex = if (args.has("conversation_index")) args.optInt("conversation_index") else null
        return when {
            requestedId != null -> project.conversations.any { it.id == requestedId }
            requestedIndex != null -> requestedIndex in project.conversations.indices
            else -> true
        }
    }

    private fun selectProject(args: JSONObject): JSONObject? {
        val requestedId = args.optString("project_id").trim().takeIf { it.isNotBlank() }
        val requestedIndex = if (args.has("project_index")) args.optInt("project_index") else null
        val index = when {
            requestedId != null -> projects.indexOfFirst { it.id == requestedId || it.projectSpaceId() == requestedId }
            requestedIndex != null -> requestedIndex
            else -> activeProjectIndex()
        }
        val resolvedIndex = if (index !in projects.indices && args.optBoolean("reload_if_missing", true)) {
            reloadProjects()
            when {
                requestedId != null -> projects.indexOfFirst { it.id == requestedId || it.projectSpaceId() == requestedId }
                requestedIndex != null -> requestedIndex
                else -> activeProjectIndex()
            }
        } else {
            index
        }
        if (resolvedIndex !in projects.indices) {
            return errorJson(args.optString("action"), "project_not_found")
                .put("requested_project_id", requestedId ?: JSONObject.NULL)
                .put("requested_project_index", requestedIndex ?: JSONObject.NULL)
        }
        setActiveProjectIndex(resolvedIndex)
        saveProjects()
        return null
    }

    private fun selectConversation(args: JSONObject, createIfMissing: Boolean): JSONObject? {
        val project = activeProject()
        if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
        val requestedId = args.optString("conversation_id").trim().takeIf { it.isNotBlank() }
        val requestedIndex = if (args.has("conversation_index")) args.optInt("conversation_index") else null
        val index = when {
            requestedId != null -> project.conversations.indexOfFirst { it.id == requestedId }
            requestedIndex != null -> requestedIndex
            else -> activeConversationIndex()
        }
        val resolvedIndex = if (index !in project.conversations.indices && args.optBoolean("reload_if_missing", true)) {
            reloadProjects()
            val reloadedProject = activeProject()
            when {
                requestedId != null -> reloadedProject.conversations.indexOfFirst { it.id == requestedId }
                requestedIndex != null -> requestedIndex
                else -> activeConversationIndex()
            }
        } else {
            index
        }
        val resolvedProject = activeProject()
        if (resolvedIndex in resolvedProject.conversations.indices) {
            setActiveConversationIndex(resolvedIndex)
            saveProjects()
            return null
        }
        if (requestedId != null && createIfMissing) {
            val targetProject = activeProject()
            targetProject.conversations.add(newMcpConversation(args))
            setActiveConversationIndex(targetProject.conversations.lastIndex)
            saveProjects()
            return null
        }
        if (requestedId != null || requestedIndex != null) {
            return errorJson(args.optString("action"), "conversation_not_found")
                .put("requested_conversation_id", requestedId ?: JSONObject.NULL)
                .put("requested_conversation_index", requestedIndex ?: JSONObject.NULL)
        }
        return null
    }

    private fun createProjectConversation(args: JSONObject) {
        val project = activeProject()
        project.conversations.add(newMcpConversation(args))
        project.activeConversationIndex = project.conversations.lastIndex
        project.updatedAt = System.currentTimeMillis()
        project.subtitle = "${project.conversations.size} 个会话"
        saveProjects()
        renderConversationList()
    }

    private fun newMcpConversation(args: JSONObject): AppConversation {
        val requestedId = args.optString("conversation_id").trim().takeIf { it.isNotBlank() }
        if (requestedId == null) return newAppConversation(conversationTitle(args), "MCP 调试会话")
        return AppConversation(
            id = requestedId,
            title = summarize(conversationTitle(args), 24),
            subtitle = "MCP 调试会话",
            updatedAt = System.currentTimeMillis(),
            messages = mutableListOf(welcomeChatMessage())
        )
    }

    private fun conversationTitle(args: JSONObject): String {
        return args.optString("conversation_title")
            .trim()
            .takeIf { it.isNotBlank() }
            ?: args.optString("title").trim().takeIf { it.isNotBlank() }
            ?: "MCP 调试会话"
    }

    private fun openActiveProjectConversation() {
        val adapter = ChatAdapter(activeConversation().messages, pauseCurrentWork, showMessageActions, retryFailedAttachmentMessage)
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        navigationController().showProjectChat(animate = false)
        if (adapter.itemCount > 0) binding.chatList.scrollToPosition(adapter.itemCount - 1)
    }

    private fun isSameOpenProjectChat(
        wasTargetChatOpen: Boolean,
        beforeProject: AppProject,
        beforeConversationId: String
    ): Boolean {
        if (!wasTargetChatOpen) return false
        val project = activeProject()
        return beforeConversationId == activeConversation().id &&
            (beforeProject.id == project.id || beforeProject.projectSpaceId() == project.projectSpaceId())
    }

    private fun activePage(): String {
        return when {
            binding.chatPage.visibility == android.view.View.VISIBLE -> "chat"
            binding.isProjectHomeSurfaceVisible() -> "project_home"
            binding.projectPage.visibility == android.view.View.VISIBLE -> "project_space"
            binding.marketplacePage.visibility == android.view.View.VISIBLE -> "project_plaza"
            binding.conversationPage.visibility == android.view.View.VISIBLE -> "conversation_home"
            binding.profilePage.visibility == android.view.View.VISIBLE -> "profile"
            binding.agentPage.root.visibility == android.view.View.VISIBLE -> "agent"
            else -> "unknown"
        }
    }

    private fun activeBottomTab(): String {
        return when {
            binding.tabChat.isSelected -> "chat"
            binding.tabProject.isSelected -> "project"
            binding.tabProfile.isSelected -> "profile"
            else -> "unknown"
        }
    }

    private fun activeProjectTopTab(): String {
        return when {
            binding.projectTopTabs.visibility != android.view.View.VISIBLE -> "hidden"
            binding.projectHomeTabIndicator.visibility == android.view.View.VISIBLE -> "home"
            binding.projectPlazaTabIndicator.visibility == android.view.View.VISIBLE -> "plaza"
            else -> "unknown"
        }
    }

    private fun visibilityJson(): JSONObject {
        return JSONObject()
            .put("conversation_page", visible(binding.conversationPage))
            .put("chat_page", visible(binding.chatPage))
            .put("project_page", visible(binding.projectPage))
            .put("marketplace_page", visible(binding.marketplacePage))
            .put("profile_page", visible(binding.profilePage))
            .put("agent_page", visible(binding.agentPage.root))
            .put("bottom_tabs", visible(binding.pageTabs))
            .put("input", visible(binding.inputLayout))
    }

    private fun projectsJson(): JSONArray {
        return JSONArray().apply {
            projects.take(30).forEachIndexed { index, project ->
                put(projectJson(project, index))
            }
        }
    }

    private fun projectJson(project: AppProject, index: Int): JSONObject {
        return JSONObject()
            .put("index", index)
            .put("id", project.id)
            .put("title", project.title)
            .put("space_id", project.projectSpaceId())
            .put("stage", project.stage)
            .put("conversation_count", project.conversations.size)
            .put("is_joint_development_project", project.isJointDevelopmentProject())
            .put("workspace_health_label", project.workspaceHealthLabel ?: JSONObject.NULL)
    }

    private fun conversationJson(
        conversation: AppConversation,
        index: Int,
        lastMessage: ChatMessage?
    ): JSONObject {
        return JSONObject()
            .put("index", index)
            .put("id", conversation.id)
            .put("title", conversation.title)
            .put("subtitle", conversation.subtitle)
            .put("ended", conversation.ended)
            .put("message_count", conversation.messages.size)
            .put("last_message_role", lastMessage?.role ?: JSONObject.NULL)
            .put("last_message_preview", lastMessage?.content?.replace('\n', ' ')?.take(240) ?: JSONObject.NULL)
            .put("messages", messagesJson(conversation.messages))
            .put("locked_agent_name", conversation.lockedAgentName ?: JSONObject.NULL)
            .put("codex_thread_uri", conversation.codexThreadUri ?: JSONObject.NULL)
    }

    private fun messagesJson(messages: List<ChatMessage>): JSONArray {
        val start = (messages.size - 16).coerceAtLeast(0)
        return JSONArray().apply {
            messages.drop(start).forEachIndexed { offset, message ->
                put(messageJson(start + offset, message))
            }
        }
    }

    private fun messageJson(index: Int, message: ChatMessage): JSONObject {
        return JSONObject()
            .put("index", index)
            .put("id", message.id ?: JSONObject.NULL)
            .put("role", message.role)
            .put("content_preview", message.content.replace('\n', ' ').take(360))
            .put("content_chars", message.content.length)
            .put("evidence_title", message.evidenceTitle ?: JSONObject.NULL)
            .put("evidence_details_preview", message.evidenceDetails?.replace('\n', ' ')?.take(500) ?: JSONObject.NULL)
            .put("evidence_working", message.evidenceWorking)
            .put("evidence_expanded", message.evidenceExpanded)
            .put("process_layer", message.processLayer)
            .put("final_reply", message.finalReply)
            .put("codex_thread_uri", message.codexThreadUri ?: JSONObject.NULL)
            .put("stream_id", message.streamId ?: JSONObject.NULL)
            .put("model_used", message.modelUsed ?: JSONObject.NULL)
            .put("node_id", message.nodeId ?: JSONObject.NULL)
    }

    private fun inputJson(): JSONObject {
        val text = binding.inputEdit.text?.toString().orEmpty()
        return JSONObject()
            .put("has_text", text.isNotBlank())
            .put("text_length", text.length)
            .put("send_enabled", binding.sendButton.isEnabled)
    }

    private fun runtimeJson(): JSONObject {
        return JSONObject()
            .put("waiting_for_reply", waitingForReply())
            .put("backend_connected", backendConnected())
            .put("active_request_is_development", activeRequestIsDevelopment())
            .put("running_task_count", runningTaskCount())
            .put("current_stage", currentStage())
    }

    private fun visible(view: android.view.View): Boolean = view.visibility == android.view.View.VISIBLE

    private fun errorJson(action: String, code: String): JSONObject {
        return uiState()
            .put("control_ok", false)
            .put("action", action)
            .put("error", code)
    }
}
