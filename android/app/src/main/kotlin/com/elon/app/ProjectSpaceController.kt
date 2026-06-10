package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectSpaceController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val showProjectSpace: (String, Boolean) -> Unit,
    private val showProjectChannelChat: (String, Boolean) -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val onProjectShareAction: (ChatProjectShare) -> Unit,
    private val collapseInputComposer: () -> Unit,
    private val personalConversations: () -> List<AppConversation>,
    private val activePersonalConversationIndex: () -> Int,
    private val isPersonalConversationWorking: (Int) -> Boolean,
    private val openPersonalAiChat: (Int) -> Unit,
    private val showPersonalConversationActions: (Int) -> Unit,
    private val showCreatePersonalConversation: () -> Unit,
    private val showCreateAndOpenPersonalConversation: (suggestedTitle: String?, onCreated: (Int) -> Unit) -> Unit,
    private val selectedAgentForRequest: () -> String?,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?
) {
    private val pollHandler = Handler(Looper.getMainLooper())
    private val messagesByChannel = linkedMapOf<String, MutableList<ChatMessage>>()
    private val messagesByMemberConversation = linkedMapOf<String, MutableList<ChatMessage>>()
    private val spaceCache = linkedMapOf<String, ProjectSpace>()
    private var activeSpace: ProjectSpace? = null
    private var activeProjectId: String? = null
    private var activeProjectTitle: String = "项目空间"
    private var activeChannel: ProjectChannel? = null
    private var activeMemberConversation: ActiveMemberConversation? = null
    private var activeRoute: ProjectSpaceRoute = ProjectSpaceRoute()
    private var activeMemberListUserId: String? = null
    private var pendingOpenSelfMemberList = false
    private var activeAdapter: ChatAdapter? = null
    private var polling = false
    private var pendingMemberBack: ProjectMember? = null
    private val personalConversationPanel = ProjectSpacePersonalConversationPanel(
        activity = activity,
        personalConversations = personalConversations,
        activePersonalConversationIndex = activePersonalConversationIndex,
        isPersonalConversationWorking = isPersonalConversationWorking,
        openPersonalAiChat = openPersonalAiChat,
        showPersonalConversationActions = showPersonalConversationActions,
        showCreatePersonalConversation = showCreatePersonalConversation,
        dp = dp,
        selectableForeground = selectableForeground
    )
    private val memberConversationViews = ProjectSpaceMemberConversationViews(
        activity = activity,
        http = http,
        serverUrl = serverUrl,
        dp = dp,
        selectableForeground = selectableForeground,
        renderActiveSpace = {
            pendingMemberBack = null
            renderActiveSpace()
        },
        renderMessages = { conversation, member, space ->
            renderMemberConversationMessages(conversation, member, space)
        },
        openPersonalConversationById = { conversationId ->
            openPersonalConversationById(conversationId)
        },
        showCreateAndOpenPersonalConversation = showCreateAndOpenPersonalConversation,
        openPersonalAiChat = openPersonalAiChat
    )
    private val memberListView = ProjectSpaceMemberListView(
        activity = activity,
        http = http,
        serverUrl = serverUrl,
        dp = dp,
        selectableForeground = selectableForeground,
        personalConversations = personalConversations,
        showCreatePersonalConversation = showCreatePersonalConversation,
        openMemberConversations = { member -> renderMemberConversationList(member) },
        reloadActiveSpace = { reloadActiveSpace() }
    )

    private data class ActiveMemberConversation(
        val projectId: String,
        val memberUserId: String,
        val memberAccount: String,
        val conversationId: String,
        val title: String
    ) {
        val key: String = "$projectId:$memberUserId:$conversationId"
    }

    private val pollRunnable = object : Runnable {
        override fun run() {
            val channel = activeChannel
            if (channel != null) {
                loadMessages(channel, silent = true, scrollToBottom = false)
            } else {
                val memberConversation = activeMemberConversation ?: return
                loadMemberConversationMessages(memberConversation, silent = true, scrollToBottom = false)
            }
            if (polling) pollHandler.postDelayed(this, POLL_INTERVAL_MS)
        }
    }

    fun openProjectSpace(projectId: String, title: String, animate: Boolean) {
        openProjectSpace(projectId, title, animate, ProjectSpaceRoute())
    }

    fun openPersonalProjectSpace(project: AppProject, userId: String, animate: Boolean) {
        openProjectSpace(
            projectId = project.id,
            title = project.title,
            animate = animate,
            route = ProjectSpaceRoute(userId = userId, projectTitle = project.title)
        )
    }

    fun openProjectMemberConversations(projectId: String, title: String, animate: Boolean) {
        openProjectSpace(projectId, title, animate, ProjectSpaceRoute(), openSelfMemberList = true)
    }

    fun openPersonalProjectMemberConversations(project: AppProject, userId: String, animate: Boolean) {
        openProjectSpace(
            projectId = project.id,
            title = project.title,
            animate = animate,
            route = ProjectSpaceRoute(userId = userId, projectTitle = project.title),
            openSelfMemberList = true
        )
    }

    private fun openProjectSpace(
        projectId: String,
        title: String,
        animate: Boolean,
        route: ProjectSpaceRoute,
        openSelfMemberList: Boolean = false
    ) {
        activeProjectId = projectId
        activeProjectTitle = title.ifBlank { "项目空间" }
        activeChannel = null
        activeMemberConversation = null
        activeRoute = route
        activeMemberListUserId = null
        pendingOpenSelfMemberList = openSelfMemberList
        activeAdapter = null
        stopPolling()
        val cached = spaceCache[projectId]
        if (cached != null) {
            // 命中缓存：立即渲染，同时后台静默刷新
            activeSpace = cached
            activeProjectTitle = cached.project.name
            showProjectSpace(activeProjectTitle, animate)
            renderProjectSpaceLanding()
        } else {
            activeSpace = null
            showProjectSpace(activeProjectTitle, animate)
            renderLoading()
        }
        thread {
            val result = runCatching { fetchProjectSpace(http, serverUrl, activity, projectId, route) }
            activity.runOnUiThread {
                if (activeProjectId != projectId) return@runOnUiThread  // 用户已切走
                result.onSuccess { space ->
                    spaceCache[projectId] = space
                    activeSpace = space
                    activeProjectTitle = space.project.name
                    showProjectSpace(activeProjectTitle, false)
                    renderProjectSpaceLanding()
                }.onFailure { error ->
                    if (cached == null) renderError(error.message ?: "加载项目空间失败")
                    // 有缓存时静默失败，保留已显示的缓存内容
                }
            }
        }
    }

    fun renderProjectSpaceLanding() {
        if (activeSpace == null) {
            renderLoading()
            return
        }
        if (pendingOpenSelfMemberList) {
            pendingOpenSelfMemberList = false
            if (renderSelfMemberConversationList()) return
        }
        activeMemberListUserId?.let { userId ->
            if (renderMemberConversationListByUserId(userId)) return
        }
        renderActiveSpace()
    }

    private fun renderSelfMemberConversationList(): Boolean {
        val selfIds = listOfNotNull(
            AuthManager.userId(activity),
            AuthManager.effectiveUserId(activity)
        ).toSet()
        val space = activeSpace ?: return false
        val member = space.members.firstOrNull { it.userId in selfIds }
            ?: space.members.firstOrNull { activeRoute.isUserProject && it.role == "owner" }
            ?: space.members.firstOrNull { activeRoute.isUserProject }
        return member?.let {
            renderMemberConversationList(it)
            true
        } ?: false
    }

    private fun renderMemberConversationListByUserId(userId: String): Boolean {
        val member = activeSpace?.members?.firstOrNull { it.userId == userId } ?: return false
        renderMemberConversationList(member)
        return true
    }

    fun renderActiveSpace() {
        activeMemberListUserId = null
        // 从个人会话返回时，若来源是成员会话列表，恢复到成员会话列表而不是顶层空间
        val backMember = pendingMemberBack
        if (backMember != null) {
            pendingMemberBack = null
            renderMemberConversationList(backMember)
            return
        }
        val space = activeSpace ?: return
        val container = prepareProjectContent()
        container.removeAllViews()
        container.addView(spaceHeader(space))
        container.addView(sectionTitle("频道"))
        space.channels.forEach { channel ->
            container.addView(channelRow(channel))
        }
        container.addView(sectionTitle("团队成员"))
        renderMemberList(container, space.members)
    }

    fun showMembers() {
        val space = activeSpace
        if (space == null) {
            Toast.makeText(activity, "成员列表加载中", Toast.LENGTH_SHORT).show()
            return
        }
        if (activeRoute.isUserProject) {
            Toast.makeText(activity, "个人项目成员列表暂不展开", Toast.LENGTH_SHORT).show()
            return
        }
        ProjectSpaceMemberDialog.show(activity, space.project.name, space.members, dp) { member ->
            renderMemberConversationList(member)
        }
    }

    fun isChannelActive(): Boolean = activeChannel != null || activeMemberConversation != null

    fun closeChannelChat() {
        activeChannel = null
        activeMemberConversation = null
        activeAdapter = null
        stopPolling()
    }

    fun resumeIfActive() {
        if (activeChannel != null || activeMemberConversation != null) startPolling()
    }

    fun stopPolling() {
        polling = false
        pollHandler.removeCallbacks(pollRunnable)
    }

    fun trySendMessage(rawText: String, hasAttachments: Boolean): Boolean {
        activeMemberConversation?.let { memberConversation ->
            return trySendMemberConversationMessage(memberConversation, rawText, hasAttachments)
        }
        val channel = activeChannel ?: return false
        val text = rawText.trim()
        if (hasAttachments) {
            Toast.makeText(activity, "项目频道暂不支持发送附件", Toast.LENGTH_SHORT).show()
            return true
        }
        if (text.isBlank()) return true

        val messages = messagesByChannel.getOrPut(channel.id) { mutableListOf() }
        val route = activeRoute
        val pending = ChatMessage("user", text, sendStatus = SENDING_STATUS)
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        collapseInputComposer()

        thread {
            val result = runCatching {
                if (channel.kind == AI_CHANNEL_KIND) {
                    startProjectChannelAiTask(
                        http = http,
                        serverUrl = serverUrl,
                        context = activity,
                        projectId = channel.projectId,
                        channelId = channel.id,
                        content = text,
                        agent = selectedAgentForRequest(),
                        route = route
                    )
                } else {
                    sendProjectChannelMessage(http, serverUrl, activity, channel.projectId, channel.id, text, route)
                }
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id) return@runOnUiThread
                result.onSuccess { sent ->
                    val index = messages.indexOf(pending)
                    if (index >= 0) {
                        messages[index] = sent.toChatMessage(activeSpace?.project?.role)
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    loadMessages(channel, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    private fun trySendMemberConversationMessage(
        memberConversation: ActiveMemberConversation,
        rawText: String,
        hasAttachments: Boolean
    ): Boolean {
        val text = rawText.trim()
        if (hasAttachments) {
            Toast.makeText(activity, "成员会话讨论暂不支持发送附件", Toast.LENGTH_SHORT).show()
            return true
        }
        if (text.isBlank()) return true

        val messages = messagesByMemberConversation.getOrPut(memberConversation.key) { mutableListOf() }
        val pending = ChatMessage("user", text, sendStatus = SENDING_STATUS)
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        collapseInputComposer()

        thread {
            val result = runCatching {
                sendProjectMemberConversationMessage(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = memberConversation.projectId,
                    memberUserId = memberConversation.memberUserId,
                    conversationId = memberConversation.conversationId,
                    content = text
                )
            }
            activity.runOnUiThread {
                if (activeMemberConversation?.key != memberConversation.key) return@runOnUiThread
                result.onSuccess { sent ->
                    val index = messages.indexOf(pending)
                    if (index >= 0) {
                        messages[index] = sent.toChatMessage()
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    loadMemberConversationMessages(
                        memberConversation,
                        silent = true,
                        scrollToBottom = true,
                        allowPendingRefresh = true
                    )
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "发送失败"
                    val index = messages.indexOf(pending)
                    if (index >= 0) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    fun summarizeSelectedDiscussion(summary: SelectedDiscussionSummary): Boolean {
        val channel = activeChannel ?: return false
        val messages = messagesByChannel.getOrPut(channel.id) { mutableListOf() }
        val route = activeRoute
        val pending = ChatMessage("user", summary.channelPost, sendStatus = SENDING_STATUS)
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        collapseInputComposer()

        thread {
            val result = runCatching {
                summarizeProjectChannelMessages(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = channel.projectId,
                    channelId = channel.id,
                    postContent = summary.channelPost,
                    summaryPrompt = summary.channelPrompt,
                    agent = selectedAgentForRequest(),
                    route = route
                )
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id) return@runOnUiThread
                result.onSuccess { sent ->
                    val index = messages.indexOfFirst { it === pending }
                    if (index >= 0) {
                        messages[index] = sent.toChatMessage(activeSpace?.project?.role)
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    loadMessages(channel, silent = true, scrollToBottom = true, allowPendingRefresh = true)
                }.onFailure { error ->
                    pending.sendStatus = error.message ?: "总结失败"
                    val index = messages.indexOfFirst { it === pending }
                    if (index >= 0) activeAdapter?.notifyMessageUpdated(index)
                }
            }
        }
        return true
    }

    private fun openChannel(channel: ProjectChannel, animate: Boolean = true) {
        activeChannel = channel
        activeMemberConversation = null
        val messages = messagesByChannel.getOrPut(channel.id) { mutableListOf() }
        val adapter = ChatAdapter(
            messages = messages,
            onMessageLongPress = showMessageActions,
            onProjectShareAction = onProjectShareAction
        )
        adapter.onSuggestionResolve = { message -> markSuggestionUpdated(message) }
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        showProjectChannelChat("#${channel.name}", animate)
        loadMessages(channel, silent = false, scrollToBottom = true)
        startPolling()
    }

    private fun startPolling() {
        if (polling) return
        polling = true
        pollHandler.removeCallbacks(pollRunnable)
        pollHandler.postDelayed(pollRunnable, POLL_INTERVAL_MS)
    }

    private fun loadMessages(
        channel: ProjectChannel,
        silent: Boolean,
        scrollToBottom: Boolean,
        allowPendingRefresh: Boolean = false
    ) {
        val currentMessages = messagesByChannel.getOrPut(channel.id) { mutableListOf() }
        if (!allowPendingRefresh && currentMessages.any { it.sendStatus == SENDING_STATUS }) return
        val route = activeRoute
        thread {
            val result = runCatching {
                fetchProjectChannelMessages(http, serverUrl, activity, channel.projectId, channel.id, route = route)
                    .map { it.toChatMessage(activeSpace?.project?.role) }
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id) return@runOnUiThread
                result.onSuccess { remoteMessages ->
                    val changed = currentMessages.size != remoteMessages.size ||
                        currentMessages.zip(remoteMessages).any { (current, incoming) ->
                            current.role != incoming.role ||
                                current.content != incoming.content ||
                                current.senderLabel != incoming.senderLabel
                        }
                    currentMessages.clear()
                    currentMessages.addAll(remoteMessages)
                    activeAdapter?.notifyDataSetChanged()
                    if (scrollToBottom && currentMessages.isNotEmpty()) {
                        binding.chatList.scrollToPosition(currentMessages.lastIndex)
                    }
                    if (changed && !silent) {
                        Toast.makeText(activity, "频道消息已更新", Toast.LENGTH_SHORT).show()
                    }
                }.onFailure { error ->
                    if (!silent) Toast.makeText(
                        activity,
                        error.message ?: "加载频道消息失败",
                        Toast.LENGTH_SHORT
                    ).show()
                }
            }
        }
    }

    private fun loadMemberConversationMessages(
        memberConversation: ActiveMemberConversation,
        silent: Boolean,
        scrollToBottom: Boolean,
        allowPendingRefresh: Boolean = false
    ) {
        val currentMessages = messagesByMemberConversation.getOrPut(memberConversation.key) { mutableListOf() }
        if (!allowPendingRefresh && currentMessages.any { it.sendStatus == SENDING_STATUS }) return
        thread {
            val result = runCatching {
                fetchProjectMemberConversationMessages(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = memberConversation.projectId,
                    memberUserId = memberConversation.memberUserId,
                    conversationId = memberConversation.conversationId
                ).map { it.toChatMessage() }
            }
            activity.runOnUiThread {
                if (activeMemberConversation?.key != memberConversation.key) return@runOnUiThread
                result.onSuccess { remoteMessages ->
                    val changed = currentMessages.size != remoteMessages.size ||
                        currentMessages.zip(remoteMessages).any { (current, incoming) ->
                            current.role != incoming.role ||
                                current.content != incoming.content ||
                                current.senderLabel != incoming.senderLabel ||
                                current.id != incoming.id
                        }
                    currentMessages.clear()
                    currentMessages.addAll(remoteMessages)
                    activeAdapter?.notifyDataSetChanged()
                    if (scrollToBottom && currentMessages.isNotEmpty()) {
                        binding.chatList.scrollToPosition(currentMessages.lastIndex)
                    }
                    if (changed && !silent) {
                        Toast.makeText(activity, "会话消息已更新", Toast.LENGTH_SHORT).show()
                    }
                }.onFailure { error ->
                    if (!silent) Toast.makeText(
                        activity,
                        error.message ?: "加载会话消息失败",
                        Toast.LENGTH_SHORT
                    ).show()
                }
            }
        }
    }

    private fun renderLoading() {
        val container = prepareProjectContent()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = "正在进入项目空间..."
            textSize = 15f
            setTextColor(Color.parseColor("#A6AFBD"))
            gravity = Gravity.CENTER
            setPadding(dp(24), dp(80), dp(24), dp(80))
        })
    }

    private fun renderError(message: String) {
        val container = prepareProjectContent()
        container.removeAllViews()
        container.addView(TextView(activity).apply {
            text = message
            textSize = 15f
            setTextColor(Color.parseColor("#FF7A7A"))
            gravity = Gravity.CENTER
            setPadding(dp(24), dp(80), dp(24), dp(80))
        })
    }

    private fun spaceHeader(space: ProjectSpace): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(18), dp(20), dp(18))
            background = panelBackground("#181B20")
            addView(TextView(activity).apply {
                text = space.project.name
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = buildString {
                    append("${space.project.memberCount} 位成员")
                    append(" · ")
                    append(projectRoleLabel(space.project.role))
                    space.project.description?.let { append("\n").append(it) }
                }
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                setPadding(0, dp(8), 0, 0)
            })
            space.latestApkUrl?.takeIf { it.isNotBlank() }?.let { apkUrl ->
                addView(projectSpaceDownloadButton(activity, apkUrl, dp, selectableForeground))
            }
        }
    }

    private fun sectionTitle(textValue: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 13f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(20), dp(18), dp(20), dp(8))
        }
    }

    private fun channelRow(channel: ProjectChannel): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openChannel(channel) }
            addView(TextView(activity).apply {
                text = buildString {
                    append("# ")
                    append(channel.name)
                    if (channel.unreadCount > 0) append("  ·  ").append(channel.unreadCount).append(" 条未读")
                }
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
            })
            addView(TextView(activity).apply {
                text = projectChannelHint(channel, activeSpace?.project?.role)
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(5), 0, 0)
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        }
    }

    private fun renderMemberList(container: LinearLayout, members: List<ProjectMember>) {
        val space = activeSpace ?: return
        val selfId = AuthManager.userId(activity) ?: AuthManager.effectiveUserId(activity)
        memberListView.render(container, space, members, selfId, activeRoute.isUserProject)
    }

    private fun reloadActiveSpace() {
        val projectId = activeProjectId ?: return
        val route = activeRoute
        thread(name = "project-space-reload") {
            val result = runCatching { fetchProjectSpace(http, serverUrl, activity, projectId, route) }
            activity.runOnUiThread {
                result.onSuccess { space ->
                    activeSpace = space
                    activeProjectTitle = space.project.name
                    renderProjectSpaceLanding()
                }.onFailure { error ->
                    Toast.makeText(activity, error.message ?: "刷新项目空间失败", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    private fun openPersonalConversationById(conversationId: String, fromMember: ProjectMember? = null) {
        val index = personalConversations().indexOfFirst { it.id == conversationId }
        if (index >= 0) {
            if (fromMember != null) pendingMemberBack = fromMember
            openPersonalAiChat(index)
        } else {
            Toast.makeText(activity, "找不到该会话，可能已删除", Toast.LENGTH_SHORT).show()
        }
    }

    // ── 成员会话列表（内联页面）───────────────────────────────────────

    private fun renderMemberConversationList(member: ProjectMember) {
        val space = activeSpace ?: return
        val selfId = AuthManager.userId(activity) ?: AuthManager.effectiveUserId(activity)
        val isSelf = member.userId == selfId
        activeMemberListUserId = member.userId
        activeChannel = null
        activeMemberConversation = null
        memberConversationViews.renderList(prepareProjectContent(), space, member, isSelf)
    }

    private fun prepareProjectContent(): LinearLayout {
        binding.projectPage.stopNestedScroll()
        binding.projectPage.scrollTo(0, 0)
        binding.projectContentLayout.jumpDrawablesToCurrentState()
        return binding.projectContentLayout
    }

    private fun renderMemberConversationMessages(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        space: ProjectSpace
    ) {
        val title = conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}"
        val memberConversation = ActiveMemberConversation(
            projectId = space.project.id,
            memberUserId = member.userId,
            memberAccount = member.account,
            conversationId = conversation.id,
            title = title
        )
        activeChannel = null
        activeMemberConversation = memberConversation
        pendingMemberBack = member
        val messages = messagesByMemberConversation.getOrPut(memberConversation.key) { mutableListOf() }
        val adapter = ChatAdapter(
            messages = messages,
            onMessageLongPress = showMessageActions,
            onProjectShareAction = onProjectShareAction
        )
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        showProjectChannelChat("${member.account.ifBlank { "成员" }} · $title", true)
        loadMemberConversationMessages(memberConversation, silent = false, scrollToBottom = true)
        startPolling()
    }

    private fun renderPersonalConversations(container: LinearLayout) {
        personalConversationPanel.render(container)
    }

    private fun markSuggestionUpdated(message: ChatMessage) {
        val channel = activeChannel ?: return
        val messageId = message.id?.takeIf { it.isNotBlank() } ?: run {
            Toast.makeText(activity, "建议消息还没有同步完成", Toast.LENGTH_SHORT).show()
            return
        }
        if (!canResolveProjectSuggestion(activeSpace?.project?.role)) {
            Toast.makeText(activity, "当前角色不能标记建议已更新", Toast.LENGTH_SHORT).show()
            return
        }
        message.canResolveSuggestion = false
        activeAdapter?.notifyMessageUpdated(messagesByChannel[channel.id]?.indexOf(message) ?: -1)
        thread {
            val route = activeRoute
            val result = runCatching {
                markProjectSuggestionUpdated(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = channel.projectId,
                    channelId = channel.id,
                    messageId = messageId,
                    route = route
                )
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id) return@runOnUiThread
                val messages = messagesByChannel[channel.id] ?: return@runOnUiThread
                val index = messages.indexOfFirst { it.id == messageId }
                result.onSuccess { updated ->
                    if (index >= 0) {
                        messages[index] = updated.toChatMessage(activeSpace?.project?.role)
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    Toast.makeText(activity, "已标记为更新完成", Toast.LENGTH_SHORT).show()
                }.onFailure { error ->
                    if (index >= 0) {
                        messages[index].canResolveSuggestion = canResolveProjectSuggestion(activeSpace?.project?.role)
                        activeAdapter?.notifyMessageUpdated(index)
                    }
                    Toast.makeText(activity, error.message ?: "标记失败", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    private companion object {
        const val POLL_INTERVAL_MS = 3000L
        const val SENDING_STATUS = "发送中..."
        const val AI_CHANNEL_KIND = "ai_development"
        const val SUGGESTIONS_CHANNEL_KIND = "suggestions"
    }
}
