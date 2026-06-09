package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
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
    private var activeSpace: ProjectSpace? = null
    private var activeProjectId: String? = null
    private var activeProjectTitle: String = "项目空间"
    private var activeChannel: ProjectChannel? = null
    private var activeRoute: ProjectSpaceRoute = ProjectSpaceRoute()
    private var activeAdapter: ChatAdapter? = null
    private var polling = false

    private val pollRunnable = object : Runnable {
        override fun run() {
            val channel = activeChannel ?: return
            loadMessages(channel, silent = true, scrollToBottom = false)
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

    private fun openProjectSpace(projectId: String, title: String, animate: Boolean, route: ProjectSpaceRoute) {
        activeProjectId = projectId
        activeProjectTitle = title.ifBlank { "项目空间" }
        activeSpace = null
        activeChannel = null
        activeRoute = route
        activeAdapter = null
        stopPolling()
        showProjectSpace(activeProjectTitle, animate)
        renderLoading()
        thread {
            val result = runCatching { fetchProjectSpace(http, serverUrl, activity, projectId, route) }
            activity.runOnUiThread {
                result.onSuccess {
                    activeSpace = it
                    activeProjectTitle = it.project.name
                    val defaultChannel = it.defaultGroupChannel()
                    if (defaultChannel != null) {
                        openChannel(defaultChannel, animate = false)
                    } else {
                        showProjectSpace(activeProjectTitle, false)
                        renderActiveSpace()
                    }
                }.onFailure { error ->
                    renderError(error.message ?: "加载项目空间失败")
                }
            }
        }
    }

    fun renderActiveSpace() {
        val space = activeSpace ?: run {
            renderLoading()
            return
        }
        val container = binding.projectContentLayout
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

    fun isChannelActive(): Boolean = activeChannel != null

    fun closeChannelChat() {
        activeChannel = null
        activeAdapter = null
        stopPolling()
    }

    fun resumeIfActive() {
        if (activeChannel != null) startPolling()
    }

    fun stopPolling() {
        polling = false
        pollHandler.removeCallbacks(pollRunnable)
    }

    fun trySendMessage(rawText: String, hasAttachments: Boolean): Boolean {
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
                        messages[index] = sent.toChatMessage()
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
                        messages[index] = sent.toChatMessage()
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

    private fun ProjectSpace.defaultGroupChannel(): ProjectChannel? {
        return if (project.role == "observer") {
            channels.firstOrNull { it.kind == AI_CHANNEL_KIND }
                ?: channels.firstOrNull { it.kind == DEFAULT_GROUP_CHANNEL_KIND }
                ?: channels.firstOrNull()
        } else {
            channels.firstOrNull { it.kind == DEFAULT_GROUP_CHANNEL_KIND }
                ?: channels.firstOrNull()
        }
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
                    .map { it.toChatMessage() }
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

    private fun renderLoading() {
        val container = binding.projectContentLayout
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
        val container = binding.projectContentLayout
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
                    append(roleLabel(space.project.role))
                    space.project.description?.let { append("\n").append(it) }
                }
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                setPadding(0, dp(8), 0, 0)
            })
            space.latestApkUrl?.takeIf { it.isNotBlank() }?.let { apkUrl ->
                addView(downloadApkButton(apkUrl))
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
                text = channelHint(channel)
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
        if (members.isEmpty()) {
            container.addView(emptyPersonalConversationRow().apply { text = "暂无成员" })
            if (space.project.role != "observer") container.addView(createPersonalConversationRow())
            return
        }
        var selfInList = false
        members.forEach { member ->
            val isSelf = member.userId == selfId
            if (isSelf) selfInList = true
            container.addView(memberCard(member, isSelf, space))
        }
        if (space.project.role != "observer") {
            if (!selfInList) container.addView(createPersonalConversationRow())
        }
    }

    private fun memberCard(member: ProjectMember, isSelf: Boolean, space: ProjectSpace): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground(if (isSelf) "#1E2A38" else "#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { renderMemberConversationList(member) }
            addView(TextView(activity).apply {
                text = buildString {
                    append(member.account)
                    if (isSelf) append(" (我)")
                }
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = buildString {
                    append(roleLabel(member.role))
                    val convCount = personalConversations().takeIf { isSelf }?.size
                    if (convCount != null && convCount > 0) append(" · $convCount 个会话")
                }
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(5), 0, 0)
            })
        }
    }

    private fun openPersonalConversationById(conversationId: String) {
        val index = personalConversations().indexOfFirst { it.id == conversationId }
        if (index >= 0) {
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

        val container = binding.projectContentLayout
        container.removeAllViews()
        container.addView(backRow("← 项目空间") { renderActiveSpace() })
        container.addView(memberPageHeader(member))
        container.addView(sectionTitle("项目 AI 会话"))

        val loadingView = inlineStatusRow("正在加载会话...", "#A6AFBD")
        container.addView(loadingView)

        thread {
            val result = runCatching {
                fetchProjectMemberConversations(http, serverUrl, activity, space.project.id, member.userId)
            }
            activity.runOnUiThread {
                if (container.indexOfChild(loadingView) < 0) return@runOnUiThread
                container.removeView(loadingView)
                result.onSuccess { conversations ->
                    if (conversations.isEmpty()) {
                        container.addView(inlineStatusRow("还没有项目 AI 会话", "#6F7785"))
                    } else {
                        conversations.forEach { conversation ->
                            container.addView(
                                memberConversationInlineCard(conversation, member, isSelf, space)
                            )
                        }
                    }
                    if (isSelf && space.project.role != "observer") {
                        container.addView(TextView(activity).apply {
                            text = "+ 新建个人 AI 会话"
                            textSize = 15f
                            setTextColor(Color.parseColor("#F2F5FA"))
                            setPadding(dp(20), dp(14), dp(20), dp(14))
                            background = panelBackground("#181B20")
                            isClickable = true
                            foreground = selectableForeground()
                            setOnClickListener {
                                showCreateAndOpenPersonalConversation(null) { index -> openPersonalAiChat(index) }
                            }
                        })
                    }
                }.onFailure { error ->
                    container.addView(inlineStatusRow(error.message ?: "加载失败", "#FF7A7A"))
                }
            }
        }
    }

    private fun renderMemberConversationMessages(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        space: ProjectSpace,
        onBack: () -> Unit
    ) {
        val container = binding.projectContentLayout
        container.removeAllViews()
        container.addView(backRow("← ${member.account} 的会话", onBack))
        container.addView(sectionTitle(conversation.title?.takeIf { it.isNotBlank() } ?: "会话消息"))

        val loadingView = inlineStatusRow("正在加载消息...", "#A6AFBD")
        container.addView(loadingView)

        thread {
            val result = runCatching {
                fetchProjectMemberConversationMessages(
                    http = http, serverUrl = serverUrl, context = activity,
                    projectId = space.project.id, memberUserId = member.userId,
                    conversationId = conversation.id
                )
            }
            activity.runOnUiThread {
                if (container.indexOfChild(loadingView) < 0) return@runOnUiThread
                container.removeView(loadingView)
                result.onSuccess { messages ->
                    if (messages.isEmpty()) {
                        container.addView(inlineStatusRow("暂无消息", "#6F7785"))
                    } else {
                        messages.forEach { msg -> container.addView(messageInlineCard(msg)) }
                    }
                }.onFailure { error ->
                    container.addView(inlineStatusRow(error.message ?: "加载失败", "#FF7A7A"))
                }
            }
        }
    }

    private fun backRow(label: String, onClick: () -> Unit): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                text = label
                textSize = 15f
                setTextColor(Color.parseColor("#60A5FA"))
            })
        }
    }

    private fun memberPageHeader(member: ProjectMember): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(8))
            addView(TextView(activity).apply {
                text = buildString {
                    append(member.account)
                    val selfId = AuthManager.userId(activity) ?: AuthManager.effectiveUserId(activity)
                    if (member.userId == selfId) append(" (我)")
                }
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
            })
            addView(TextView(activity).apply {
                text = roleLabel(member.role)
                textSize = 13f
                setTextColor(Color.parseColor("#A6AFBD"))
                setPadding(0, dp(6), 0, 0)
            })
        }
    }

    private fun memberConversationInlineCard(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        isSelf: Boolean,
        space: ProjectSpace
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                if (isSelf) {
                    openPersonalConversationById(conversation.id)
                } else {
                    renderMemberConversationMessages(
                        conversation, member, space,
                        onBack = { renderMemberConversationList(member) }
                    )
                }
            }
            addView(TextView(activity).apply {
                text = conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}"
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = buildString {
                    append("${conversation.messageCount} 条消息")
                    if (conversation.taskCount > 0) append(" · ${conversation.taskCount} 个任务")
                    conversation.lastTaskStatus?.takeIf { it.isNotBlank() }?.let { st ->
                        append(" \u00b7 ").append(when (st) {
                            "running" -> "运行中"
                            "done" -> "已完成"
                            "failed" -> "失败"
                            else -> st
                        })
                    }
                    conversation.updatedAt.takeIf { it.isNotBlank() }?.let { append(" · ").append(it) }
                }
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(5), 0, 0)
            })
            conversation.lastMessage?.takeIf { it.isNotBlank() }?.let { preview ->
                addView(TextView(activity).apply {
                    text = preview
                    textSize = 13f
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setPadding(0, dp(8), 0, 0)
                    maxLines = 2
                    ellipsize = android.text.TextUtils.TruncateAt.END
                })
            }
            if (!isSelf) {
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.END
                    setPadding(0, dp(10), 0, 0)
                    addView(TextView(activity).apply {
                        text = "在此基础上分叉 →"
                        textSize = 13f
                        setTextColor(Color.parseColor("#58BE6A"))
                        setTypeface(typeface, Typeface.BOLD)
                        isClickable = true
                        setOnClickListener {
                            val forkTitle = "分叉 ${member.account}：${conversation.title ?: "会话"}"
                            showCreateAndOpenPersonalConversation(forkTitle) { index -> openPersonalAiChat(index) }
                        }
                    })
                })
            }
        }
    }

    private fun messageInlineCard(message: ProjectMemberConversationMessage): LinearLayout {
        val roleColor = when (message.role) {
            "user" -> "#93C5FD"; "assistant" -> "#A7F3D0"; "system" -> "#FCA5A5"; else -> "#A6AFBD"
        }
        val roleText = when (message.role) {
            "user" -> "成员"; "assistant" -> "AI"; "system" -> "系统"; else -> message.role
        }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground("#181B20")
            addView(TextView(activity).apply {
                text = "$roleText · ${message.createdAt}"
                textSize = 12f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor(roleColor))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = message.content.ifBlank { "(空消息)" }
                textSize = 14f
                setTextColor(Color.parseColor("#F2F5FA"))
                setPadding(0, dp(6), 0, 0)
            })
        }
    }

    private fun inlineStatusRow(text: String, colorHex: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(colorHex))
            setPadding(dp(20), dp(36), dp(20), dp(36))
        }
    }

    private fun renderPersonalConversations(container: LinearLayout) {
        val conversations = personalConversations()
        if (conversations.isEmpty()) {
            container.addView(emptyPersonalConversationRow())
        } else {
            val activeIndex = activePersonalConversationIndex()
            conversations.forEachIndexed { index, conversation ->
                container.addView(
                    personalConversationRow(
                        index = index,
                        conversation = conversation,
                        active = index == activeIndex,
                        working = isPersonalConversationWorking(index)
                    )
                )
            }
        }
        container.addView(createPersonalConversationRow())
    }

    private fun personalConversationRow(
        index: Int,
        conversation: AppConversation,
        active: Boolean,
        working: Boolean
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            val rowBackground = panelBackground(if (active) "#283140" else "#181B20")
            background = rowBackground
            if (working) startProjectConversationShimmer(this, rowBackground)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPersonalAiChat(index) }
            setOnLongClickListener {
                showPersonalConversationActions(index)
                true
            }
            addView(TextView(activity).apply {
                text = buildString {
                    append(conversation.title.ifBlank { "个人会话 ${index + 1}" })
                    if (active) append("  ·  当前")
                    if (conversation.ended) append("  ·  已结束")
                }
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = personalConversationHint(conversation)
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(5), 0, 0)
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        }
    }

    private fun createPersonalConversationRow(): TextView {
        return TextView(activity).apply {
            text = "+ 新建个人 AI 会话"
            textSize = 15f
            setTextColor(Color.parseColor("#F2F5FA"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreatePersonalConversation() }
        }
    }

    private fun emptyPersonalConversationRow(): TextView {
        return TextView(activity).apply {
            text = "暂无个人会话"
            textSize = 13f
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
        }
    }

    private fun personalConversationHint(conversation: AppConversation): String {
        val subtitle = conversation.subtitle.takeIf { it.isNotBlank() } ?: "还没有消息"
        return if (conversation.messages.isEmpty()) {
            subtitle
        } else {
            "${conversation.messages.size} 条消息 · $subtitle"
        }
    }

    private fun ProjectChannelMessage.toChatMessage(): ChatMessage {
        val role = when (kind) {
            "ai_progress" -> "ai-progress"
            "ai_result" -> "ai-complete"
            "system" -> "ai"
            else -> if (outgoing) "user" else "friend"
        }
        return ChatMessage(
            role = role,
            content = content,
            senderLabel = if (role == "friend") senderName else null,
            id = id,
            suggestionStatus = suggestionStatus,
            suggestionResolvedByName = suggestionResolvedByName,
            suggestionResolvedAt = suggestionResolvedAt,
            canResolveSuggestion = canResolveSuggestion(this),
            createdAtMs = parseChatMessageCreatedAt(createdAt) ?: 0L
        )
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
                        messages[index] = updated.toChatMessage()
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

    private fun canResolveSuggestion(message: ProjectChannelMessage): Boolean {
        return message.kind == "suggestion" &&
            message.suggestionStatus != "updated" &&
            canResolveProjectSuggestion(activeSpace?.project?.role)
    }

    private fun channelHint(channel: ProjectChannel): String {
        channel.lastMessage?.takeIf { it.isNotBlank() }?.let { return it }
        return when (channel.kind) {
            "announcements" -> "项目公告、规则和重要更新。"
            "discussion" -> "成员日常讨论和协作交流。"
            "requirements" -> "集中提出功能想法，后续可转为 AI 开发任务。"
            SUGGESTIONS_CHANNEL_KIND -> "游客和成员在这里发布建议，开发者完成后可标记已更新。"
            "issues" -> "反馈 bug、安装问题和体验问题。"
            AI_CHANNEL_KIND -> if (activeSpace?.project?.role == "observer") {
                "只读模式下可以询问 AI；涉及修改代码、编译或发布的请求会被拒绝。"
            } else {
                "在这里发消息会发起集体 AI 开发任务，过程和结果对成员可见。"
            }
            "builds" -> "构建、发布、APK 下载和部署结果记录。"
            else -> "项目成员共享频道。"
        }
    }

    private fun roleLabel(role: String): String = when (role) {
        "owner" -> "所有者"
        "editor" -> "协作者"
        "member" -> "成员"
        "observer" -> "只读成员"
        else -> role
    }

    private fun downloadApkButton(apkUrl: String): TextView {
        return TextView(activity).apply {
            text = "下载最新 APK"
            textSize = 15f
            gravity = Gravity.CENTER
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#07120A"))
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor("#58BE6A"))
            }
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openApkDownload(apkUrl) }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            ).apply { topMargin = dp(14) }
        }
    }

    private fun openApkDownload(apkUrl: String) {
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (token.isBlank()) {
            Toast.makeText(activity, "请先登录后下载 APK", Toast.LENGTH_SHORT).show()
            return
        }
        openProjectApkInstall(activity, apkUrl, token)
    }

    private fun panelBackground(color: String): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(color))
            cornerRadius = dp(0).toFloat()
        }
    }

    private companion object {
        const val POLL_INTERVAL_MS = 3000L
        const val SENDING_STATUS = "发送中..."
        const val DEFAULT_GROUP_CHANNEL_KIND = "discussion"
        const val AI_CHANNEL_KIND = "ai_development"
        const val SUGGESTIONS_CHANNEL_KIND = "suggestions"
    }
}

private fun canResolveProjectSuggestion(role: String?): Boolean {
    return role == "owner" || role == "editor" || role == "member"
}
