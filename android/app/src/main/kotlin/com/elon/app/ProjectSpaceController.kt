package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.text.InputFilter
import android.text.InputType
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
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
    private val openPersonalAiChat: (Int) -> Unit,
    private val showCreateAndOpenPersonalConversation: (suggestedTitle: String?, onCreated: (Int) -> Unit) -> Unit,
    private val selectedAgentForRequest: () -> String?,
    private val onProjectDescriptionUpdated: (projectId: String, description: String?) -> Unit,
    private val pickPostImage: (ProjectSpaceSummary, (Result<String>) -> Unit) -> Unit,
    private val localProjectIconDataUrl: (String) -> String?,
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
    private var activePostMessageId: String? = null
    private var activeMemberConversation: ActiveMemberConversation? = null
    private var activeRoute: ProjectSpaceRoute = ProjectSpaceRoute()
    private var activeMemberListUserId: String? = null
    private var pendingOpenSelfMemberList = false
    private var activeAdapter: ChatAdapter? = null
    private var polling = false
    private var pendingMemberBack: ProjectMember? = null
    private var projectSpaceAiExpanded = true
    private var projectSpaceAiAnimator: ValueAnimator? = null
    private var projectSpaceFeedActionsEnabled = false
    private val feedData = ProjectSpaceFeedData(
        activity = activity,
        http = http,
        serverUrl = serverUrl,
        route = { activeRoute },
        activeProjectId = { activeProjectId },
        isSpaceLandingActive = { activeChannel == null && activeMemberConversation == null },
        renderLanding = { renderProjectSpaceLanding() }
    )
    private val announcementEditor = ProjectSpaceAnnouncementEditor(
        activity = activity,
        dp = dp,
        onSubmit = { channel, content, onComplete ->
            feedData.submitAnnouncement(channel, content, onComplete)
        }
    )
    private val feedView = ProjectSpaceFeedView(
        activity = activity,
        dp = dp,
        selectableForeground = selectableForeground,
        openPost = { channel, post -> openChannel(channel, postMessage = post) },
        openPostComposer = { renderPostComposer() },
        openAnnouncementEditor = { channel, currentText ->
            activeSpace?.let { announcementEditor.show(it, channel, currentText) }
        },
        openProjectDocuments = { showProjectDocumentsDialog() },
        projectApkActionLabel = {
            val space = activeSpace
            projectApkActionLabel(
                activity,
                space?.project?.id.orEmpty(),
                space?.project?.name.orEmpty(),
                space?.latestApkUrl
            )
        },
        downloadProjectApk = {
            val space = activeSpace
            openProjectApkDownload(
                activity,
                space?.latestApkUrl,
                space?.project?.id,
                space?.project?.name
            )
        }
    )
    private val postComposer = ProjectSpacePostComposer(
        activity = activity,
        dp = dp,
        selectableForeground = selectableForeground,
        onBack = { renderProjectSpaceLanding() },
        onPickLocalImage = pickPostImage,
        onSubmit = { channel, title, body, onComplete ->
            feedData.submit(channel, title, body, onComplete)
        }
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
    init {
        binding.projectSpacePostFab.setOnClickListener { renderPostComposer() }
        setupProjectSpaceAiMenuMotion()
    }

    private data class ActiveMemberConversation(
        val projectId: String,
        val memberUserId: String,
        val memberAccount: String,
        val conversationId: String,
        val title: String
    ) {
        val key: String = "$projectId:$memberUserId:$conversationId"
    }

    private fun activeChannelMessageKey(channel: ProjectChannel, postId: String? = activePostMessageId): String {
        return postId?.takeIf { it.isNotBlank() }?.let { "${channel.id}:post:$it" } ?: channel.id
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

    fun openProjectSpace(projectId: String, title: String, animate: Boolean, iconDataUrl: String? = null) {
        openProjectSpace(projectId, title, animate, ProjectSpaceRoute(), localIconDataUrl = iconDataUrl)
    }

    fun openPersonalProjectSpace(project: AppProject, userId: String, animate: Boolean) {
        openProjectSpace(
            projectId = project.id,
            title = project.title,
            animate = animate,
            route = ProjectSpaceRoute(userId = userId, projectTitle = project.title),
            localIconDataUrl = project.iconDataUrl
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
        openSelfMemberList: Boolean = false,
        localIconDataUrl: String? = null
    ) {
        val switchingProject = activeProjectId != projectId
        activeProjectId = projectId
        activeProjectTitle = title.ifBlank { "项目空间" }
        activeChannel = null
        activePostMessageId = null
        activeMemberConversation = null
        activeRoute = route
        activeMemberListUserId = null
        pendingOpenSelfMemberList = openSelfMemberList
        activeAdapter = null
        stopPolling()
        resetProjectSpaceAiMenu()
        if (switchingProject) {
            feedData.reset()
        }
        val resolvedLocalIcon = localIconDataUrl.cleanProjectIconDataUrl()
            ?: localProjectIconDataUrl(projectId).cleanProjectIconDataUrl()
        val cached = spaceCache[projectId]?.withProjectIcon(resolvedLocalIcon)
        if (cached != null) {
            // 命中缓存：立即渲染，同时后台静默刷新
            spaceCache[projectId] = cached
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
                    val nextSpace = space.withProjectIcon(resolvedLocalIcon)
                    spaceCache[projectId] = nextSpace
                    activeSpace = nextSpace
                    activeProjectTitle = nextSpace.project.name
                    showProjectSpace(activeProjectTitle, false)
                    renderProjectSpaceLanding()
                }.onFailure { error ->
                    if (cached == null) renderError(error.message ?: "加载项目空间失败")
                    // 有缓存时静默失败，保留已显示的缓存内容
                }
            }
        }
    }

    fun updateProjectIcon(projectIds: Set<String>, iconDataUrl: String?) {
        val ids = projectIds.mapNotNull { it.trim().takeIf(String::isNotBlank) }.toSet()
        if (ids.isEmpty()) return
        val cleanIcon = iconDataUrl.cleanProjectIconDataUrl()
        var shouldRender = false

        spaceCache.keys.toList().forEach { key ->
            val space = spaceCache[key] ?: return@forEach
            if (key in ids || space.project.id in ids) {
                val next = space.withProjectIcon(cleanIcon, force = true)
                spaceCache[key] = next
                if (activeProjectId == key || activeSpace?.project?.id in ids) {
                    activeSpace = next
                    shouldRender = true
                }
            }
        }

        val current = activeSpace
        if (current != null && current.project.id in ids) {
            activeSpace = current.withProjectIcon(cleanIcon, force = true)
            shouldRender = true
        }

        if (shouldRender && activeChannel == null && activeMemberConversation == null) {
            renderProjectSpaceLanding()
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
        activeChannel = null
        activePostMessageId = null
        activeMemberConversation = null
        activeAdapter = null
        stopPolling()
        // 从个人会话返回时，若来源是成员会话列表，恢复到成员会话列表而不是顶层空间
        val backMember = pendingMemberBack
        if (backMember != null) {
            pendingMemberBack = null
            renderMemberConversationList(backMember)
            return
        }
        val space = activeSpace ?: return
        val container = prepareProjectContent(showAiMenu = true)
        container.removeAllViews()
        feedView.render(
            container = container,
            space = space,
            messagesByChannel = feedData.messagesByChannel,
            loading = feedData.isLoading(space)
        )
        showProjectSpaceFeedActions()
        feedData.ensure(space)
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
        val selfIds = currentProjectSelfIds()
        val memberLongPress: ((ProjectMember, () -> Unit) -> Boolean)? =
            if (canManageProjectMembers(space.project.role)) {
                { member, closeDialog ->
                    showMemberRoleDialogFromMemberList(space.project.id, member, selfIds, closeDialog)
                }
            } else {
                null
            }
        ProjectSpaceMemberDialog.show(
            activity,
            space.project.name,
            space.members,
            dp,
            onMemberLongPress = memberLongPress
        ) { member ->
            renderMemberConversationList(member)
        }
    }

    private fun currentProjectSelfIds(): Set<String> {
        return listOfNotNull(
            AuthManager.userId(activity),
            AuthManager.effectiveUserId(activity)
        ).toSet()
    }

    private fun showMemberRoleDialogFromMemberList(
        projectId: String,
        member: ProjectMember,
        selfIds: Set<String>,
        closeMemberDialog: () -> Unit
    ): Boolean {
        if (member.userId in selfIds || member.role.equals("owner", ignoreCase = true)) return true
        closeMemberDialog()
        ProjectSpaceMemberManagement.showRoleDialog(
            activity = activity,
            http = http,
            serverUrl = serverUrl,
            projectId = projectId,
            member = member,
            dp = dp,
            onChanged = { reloadProjectSpaceAfterMemberRoleChange(reopenMembers = true) }
        )
        return true
    }

    private fun reloadProjectSpaceAfterMemberRoleChange(reopenMembers: Boolean) {
        val projectId = activeProjectId ?: return
        val route = activeRoute
        thread(name = "project-reload-after-member-role") {
            val result = runCatching { fetchProjectSpace(http, serverUrl, activity, projectId, route) }
            activity.runOnUiThread {
                if (activeProjectId != projectId) return@runOnUiThread
                result.onSuccess { space ->
                    val icon = localProjectIconDataUrl(projectId).cleanProjectIconDataUrl()
                    val nextSpace = space.withProjectIcon(icon)
                    spaceCache[projectId] = nextSpace
                    activeSpace = nextSpace
                    activeProjectTitle = nextSpace.project.name
                    renderProjectSpaceLanding()
                    if (reopenMembers) showMembers()
                }.onFailure { error ->
                    Toast.makeText(activity, error.message ?: "成员列表刷新失败", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    fun isChannelActive(): Boolean = activeChannel != null || activeMemberConversation != null

    fun closeChannelChat() {
        activeChannel = null
        activePostMessageId = null
        activeMemberConversation = null
        activeAdapter = null
        stopPolling()
    }

    fun resumeIfActive() {
        if (activeChannel?.kind == DOCS_CHANNEL_KIND) return
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
        if (channel.kind == DOCS_CHANNEL_KIND) {
            Toast.makeText(activity, "文档频道为固定只读频道", Toast.LENGTH_SHORT).show()
            return true
        }
        if (text.isBlank()) return true

        val postMessageId = activePostMessageId
        val messageKey = activeChannelMessageKey(channel, postMessageId)
        val messages = messagesByChannel.getOrPut(messageKey) { mutableListOf() }
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
                    sendProjectChannelMessage(
                        http = http,
                        serverUrl = serverUrl,
                        context = activity,
                        projectId = channel.projectId,
                        channelId = channel.id,
                        content = text,
                        route = route,
                        replyToMessageId = postMessageId
                    )
                }
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id || activePostMessageId != postMessageId) return@runOnUiThread
                result.onSuccess { sent ->
                    if (channel.kind == AI_CHANNEL_KIND) {
                        clearCachedProjectSpaceDocuments(activity, serverUrl, channel.projectId, route)
                    }
                    val index = messages.indexOf(pending)
                    if (index >= 0) {
                        messages[index] = sent.toChatMessage(activeSpace?.project?.role, channel)
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
        if (channel.kind == DOCS_CHANNEL_KIND) return false
        val messageKey = activeChannelMessageKey(channel)
        val messages = messagesByChannel.getOrPut(messageKey) { mutableListOf() }
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
                if (activeChannel?.id != channel.id || activeChannelMessageKey(channel) != messageKey) return@runOnUiThread
                result.onSuccess { sent ->
                    val index = messages.indexOfFirst { it === pending }
                    if (index >= 0) {
                        messages[index] = sent.toChatMessage(activeSpace?.project?.role, channel)
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

    private fun openChannel(
        channel: ProjectChannel,
        animate: Boolean = true,
        postMessage: ProjectChannelMessage? = null
    ) {
        activeChannel = channel
        activePostMessageId = postMessage?.id
        activeMemberConversation = null
        if (channel.kind == DOCS_CHANNEL_KIND) stopPolling()
        val messageKey = activeChannelMessageKey(channel, activePostMessageId)
        val messages = messagesByChannel.getOrPut(messageKey) { mutableListOf() }
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
        if (channel.kind != DOCS_CHANNEL_KIND) startPolling()
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
        val postMessageId = activePostMessageId
        val messageKey = activeChannelMessageKey(channel, postMessageId)
        val currentMessages = messagesByChannel.getOrPut(messageKey) { mutableListOf() }
        if (!allowPendingRefresh && currentMessages.any { it.sendStatus == SENDING_STATUS }) return
        val route = activeRoute
        thread {
            val result = runCatching {
                val channelMessages = fetchProjectChannelMessages(
                    http,
                    serverUrl,
                    activity,
                    channel.projectId,
                    channel.id,
                    route = route
                )
                val replyCounts = projectSpaceReplyCountsByPost(channelMessages)
                val visibleMessages = postMessageId
                    ?.let { projectSpaceMessagesForPost(channelMessages, it) }
                    ?: channelMessages
                visibleMessages.map { message ->
                    message.toChatMessage(
                        projectRole = activeSpace?.project?.role,
                        channel = channel,
                        replyCount = replyCounts[message.id]
                    )
                }
            }
            activity.runOnUiThread {
                if (activeChannel?.id != channel.id || activePostMessageId != postMessageId) return@runOnUiThread
                result.onSuccess { remoteMessages ->
                    val changed = currentMessages.size != remoteMessages.size ||
                        currentMessages.zip(remoteMessages).any { (current, incoming) ->
                            current.role != incoming.role ||
                                current.content != incoming.content ||
                                current.senderLabel != incoming.senderLabel ||
                                current.senderAvatarDataUrl != incoming.senderAvatarDataUrl ||
                                current.projectPostCard != incoming.projectPostCard
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

    private fun renderPostComposer() {
        val space = activeSpace ?: return
        activeMemberListUserId = null
        activeChannel = null
        activePostMessageId = null
        activeMemberConversation = null
        activeAdapter = null
        stopPolling()
        postComposer.render(
            container = prepareProjectContent(),
            space = space,
            channels = space.channels
        )
    }

    private fun showProjectDocumentsDialog() {
        val space = activeSpace ?: return
        ProjectSpaceDocumentDialog.show(
            activity = activity,
            http = http,
            serverUrl = serverUrl,
            projectId = space.project.id,
            route = activeRoute,
            projectTitle = space.project.name,
            dp = dp
        )
    }

    private fun showProjectChannelsDialog(space: ProjectSpace) {
        val channels = space.channels.filter { it.kind == "announcements" || it.isProjectSpaceFeedChannel() }
        if (channels.isEmpty()) {
            Toast.makeText(activity, "暂无话题入口", Toast.LENGTH_SHORT).show()
            return
        }
        val labels = channels.map { projectSpaceTopicLabel(it) }.toTypedArray()
        AlertDialog.Builder(activity)
            .setTitle("话题入口")
            .setItems(labels) { dialog, which ->
                dialog.dismiss()
                openChannel(channels[which])
            }
            .show()
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
            setTextColor(Color.parseColor("#A8A8A8"))
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

    private fun projectIntroHeader(space: ProjectSpace): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(8), dp(20), dp(14))
            background = panelBackground("#222222")
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                val iconBitmap = UserProfileStore.decodeAvatar(space.project.iconDataUrl)
                if (iconBitmap != null) {
                    addView(projectSpaceIconView(iconBitmap), LinearLayout.LayoutParams(dp(42), dp(42)).apply {
                        marginEnd = dp(12)
                    })
                }
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.VERTICAL
                    gravity = Gravity.CENTER_VERTICAL
                    addView(TextView(activity).apply {
                        text = space.project.name
                        textSize = 20f
                        setTypeface(typeface, Typeface.BOLD)
                        setTextColor(Color.parseColor("#D6D6D6"))
                        maxLines = 2
                        ellipsize = TextUtils.TruncateAt.END
                    })
                    addView(TextView(activity).apply {
                        text = "${space.project.memberCount} 位成员 · ${projectRoleLabel(space.project.role)}"
                        textSize = 13f
                        setTextColor(Color.parseColor("#A8A8A8"))
                        setPadding(0, dp(8), 0, 0)
                    })
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 0.72f))
                addView(projectDescriptionCard(space), LinearLayout.LayoutParams(
                    0,
                    dp(84),
                    1.7f
                ).apply {
                    marginStart = dp(12)
                })
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun projectSpaceIconView(iconBitmap: android.graphics.Bitmap): View {
        return FrameLayout(activity).apply {
            background = GradientDrawable().apply {
                cornerRadius = dp(8).toFloat()
                setColor(Color.parseColor("#2A2A2A"))
            }
            clipToOutline = true
            addView(ImageView(activity).apply {
                setImageBitmap(iconBitmap)
                scaleType = ImageView.ScaleType.CENTER_CROP
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun ProjectSpace.withProjectIcon(iconDataUrl: String?, force: Boolean = false): ProjectSpace {
        val cleanIcon = iconDataUrl.cleanProjectIconDataUrl()
        if (cleanIcon == null && !force) return this
        if (!force && !project.iconDataUrl.isNullOrBlank()) return this
        if (project.iconDataUrl == cleanIcon) return this
        return copy(project = project.copy(iconDataUrl = cleanIcon))
    }

    private fun String?.cleanProjectIconDataUrl(): String? {
        return this?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
    }

    private fun projectDescriptionCard(space: ProjectSpace): TextView {
        val editable = canEditProjectDescription(space.project.role)
        val description = space.project.description?.trim().orEmpty()
        return TextView(activity).apply {
            text = description.ifBlank { if (editable) "添加项目简介" else "暂无项目简介" }
            textSize = 13f
            gravity = Gravity.CENTER
            includeFontPadding = false
            setTextColor(Color.parseColor(if (description.isBlank()) "#777777" else "#D6D6D6"))
            setLineSpacing(dp(3).toFloat(), 1.0f)
            maxLines = 8
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(16), dp(14), dp(16), dp(14))
            background = panelBackground("#202024").apply {
                cornerRadius = dp(14).toFloat()
            }
            if (editable) {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = "编辑项目简介"
                setOnClickListener { showProjectDescriptionDialog(space) }
            }
        }
    }

    private fun showProjectDescriptionDialog(space: ProjectSpace) {
        if (!canEditProjectDescription(space.project.role)) {
            Toast.makeText(activity, "当前成员角色不能编辑项目简介", Toast.LENGTH_SHORT).show()
            return
        }
        val input = EditText(activity).apply {
            setText(space.project.description.orEmpty())
            hint = "一款太逃杀类型的卡牌游戏"
            minLines = 4
            maxLines = 6
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            filters = arrayOf(InputFilter.LengthFilter(PROJECT_DESCRIPTION_MAX_CHARS))
            setTextColor(Color.parseColor("#D6D6D6"))
            setHintTextColor(Color.parseColor("#777777"))
            setPadding(dp(12), dp(10), dp(12), dp(10))
            background = panelBackground("#222222").apply {
                cornerRadius = dp(8).toFloat()
            }
            setSelection(text?.length ?: 0)
        }
        val wrapper = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(6), dp(20), 0)
            addView(input, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("项目简介")
            .setView(wrapper)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                saveProjectDescription(input.text?.toString().orEmpty(), dialog)
            }
            input.requestFocus()
        }
        dialog.show()
    }

    private fun saveProjectDescription(description: String, dialog: AlertDialog) {
        val projectId = activeProjectId ?: return
        val route = activeRoute
        val saveButton = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
        saveButton?.isEnabled = false
        thread(name = "project-description-save") {
            val result = runCatching {
                updateProjectSpaceDescription(http, serverUrl, activity, projectId, description, route)
            }
            activity.runOnUiThread {
                saveButton?.isEnabled = true
                result.onSuccess { updated ->
                    val current = activeSpace
                    if (current != null && current.project.id == updated.id) {
                        val next = current.copy(project = updated)
                        activeSpace = next
                        spaceCache[updated.id] = next
                        activeProjectTitle = updated.name
                        onProjectDescriptionUpdated(updated.id, updated.description)
                        renderProjectSpaceLanding()
                    }
                    dialog.dismiss()
                    Toast.makeText(activity, "项目简介已保存", Toast.LENGTH_SHORT).show()
                }.onFailure { error ->
                    Toast.makeText(activity, error.message ?: "保存项目简介失败", Toast.LENGTH_SHORT).show()
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
        activePostMessageId = null
        activeMemberConversation = null
        memberConversationViews.renderList(prepareProjectContent(), space, member, isSelf)
    }

    private fun prepareProjectContent(showAiMenu: Boolean = false): LinearLayout {
        binding.projectScrollView.stopNestedScroll()
        binding.projectScrollView.scrollTo(0, 0)
        if (showAiMenu) showProjectSpaceAiMenu() else hideProjectSpaceAiMenu()
        hideProjectSpaceFeedActions()
        binding.projectContentLayout.jumpDrawablesToCurrentState()
        return binding.projectContentLayout
    }

    private fun showProjectSpaceAiMenu() {
        binding.projectSpaceAiMenu.visibility = View.VISIBLE
        resetProjectSpaceAiMenu()
        binding.projectSpaceAiMenu.bringToFront()
    }

    private fun hideProjectSpaceAiMenu() {
        projectSpaceAiAnimator?.cancel()
        projectSpaceAiAnimator = null
        binding.projectSpaceAiMenu.visibility = View.GONE
        syncProjectSpacePostEntry()
    }

    private fun showProjectSpaceFeedActions() {
        projectSpaceFeedActionsEnabled = true
        syncProjectSpacePostEntry()
    }

    private fun hideProjectSpaceFeedActions() {
        projectSpaceFeedActionsEnabled = false
        binding.projectSpaceFeedActionsOverlay.visibility = View.GONE
    }

    private fun syncProjectSpacePostEntry() {
        val shouldShowPostEntry = projectSpaceFeedActionsEnabled &&
            binding.projectSpaceAiMenu.visibility == View.VISIBLE &&
            !projectSpaceAiExpanded
        binding.projectSpaceFeedActionsOverlay.visibility = if (shouldShowPostEntry) View.VISIBLE else View.GONE
        if (shouldShowPostEntry) binding.projectSpaceFeedActionsOverlay.bringToFront()
        if (binding.projectSpaceAiMenu.visibility == View.VISIBLE) {
            binding.projectSpaceAiMenu.bringToFront()
        }
    }

    private fun setupProjectSpaceAiMenuMotion() {
        binding.projectScrollView.setOnScrollChangeListener { _, _, scrollY, _, _ ->
            val shouldExpand = scrollY <= dp(PROJECT_SPACE_AI_EXPAND_AT_TOP_DP)
            updateProjectSpaceAiMenuExpanded(shouldExpand, animate = true)
        }
        resetProjectSpaceAiMenu()
    }

    private fun resetProjectSpaceAiMenu() {
        updateProjectSpaceAiMenuExpanded(expanded = true, animate = false)
    }

    private fun updateProjectSpaceAiMenuExpanded(expanded: Boolean, animate: Boolean) {
        val menu = binding.projectSpaceAiMenu
        val label = binding.projectSpaceAiLabel
        val collapsedWidth = dp(PROJECT_SPACE_AI_COLLAPSED_SIZE_DP)
        val expandedWidth = dp(PROJECT_SPACE_AI_EXPANDED_WIDTH_DP)
        val expandedLabelWidth = measureProjectSpaceAiLabelWidth()
        val targetWidth = if (expanded) expandedWidth else collapsedWidth
        val targetLabelWidth = if (expanded) expandedLabelWidth else 0
        val currentLayoutWidth = menu.layoutParams.width.takeIf { it > 0 } ?: targetWidth
        val currentLabelWidth = (label.layoutParams as LinearLayout.LayoutParams)
            .width
            .takeIf { it >= 0 } ?: expandedLabelWidth
        val sameTarget = projectSpaceAiExpanded == expanded
        if (sameTarget && projectSpaceAiAnimator?.isRunning == true) return

        val alreadyAtTarget = sameTarget &&
            currentLayoutWidth == targetWidth &&
            currentLabelWidth == targetLabelWidth &&
            projectSpaceAiAnimator == null
        if (alreadyAtTarget) {
            syncProjectSpacePostEntry()
            return
        }

        projectSpaceAiExpanded = expanded
        projectSpaceAiAnimator?.cancel()
        projectSpaceAiAnimator = null
        if (expanded) syncProjectSpacePostEntry()

        val targetIconMargin = if (expanded) dp(PROJECT_SPACE_AI_ICON_MARGIN_END_DP) else 0
        label.visibility = View.VISIBLE
        label.alpha = 1f

        if (!animate || menu.visibility != View.VISIBLE || menu.width <= 0) {
            applyProjectSpaceAiMenuFrame(targetWidth, targetIconMargin, targetLabelWidth)
            syncProjectSpacePostEntry()
            return
        }

        val startWidth = menu.width.takeIf { it > 0 } ?: currentLayoutWidth
        val startIconMargin = (binding.projectSpaceAiIcon.layoutParams as LinearLayout.LayoutParams).marginEnd
        val startLabelWidth = (label.layoutParams as LinearLayout.LayoutParams)
            .width
            .takeIf { it >= 0 } ?: expandedLabelWidth
        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = PROJECT_SPACE_AI_ANIMATION_MS
            interpolator = AccelerateDecelerateInterpolator()
            addUpdateListener { valueAnimator ->
                val progress = valueAnimator.animatedValue as Float
                val width = (startWidth + (targetWidth - startWidth) * progress).toInt()
                val iconMargin = (startIconMargin + (targetIconMargin - startIconMargin) * progress).toInt()
                val labelWidth = (startLabelWidth + (targetLabelWidth - startLabelWidth) * progress).toInt()
                applyProjectSpaceAiMenuFrame(width, iconMargin, labelWidth)
            }
            addListener(object : AnimatorListenerAdapter() {
                private var cancelled = false

                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (cancelled) return
                    applyProjectSpaceAiMenuFrame(targetWidth, targetIconMargin, targetLabelWidth)
                    projectSpaceAiAnimator = null
                    syncProjectSpacePostEntry()
                }
            })
        }
        projectSpaceAiAnimator = animator
        animator.start()
    }

    private fun applyProjectSpaceAiMenuFrame(
        width: Int,
        iconMarginEnd: Int,
        labelWidth: Int
    ) {
        val menu = binding.projectSpaceAiMenu
        val height = dp(PROJECT_SPACE_AI_COLLAPSED_SIZE_DP)
        val menuParams = menu.layoutParams
        if (menuParams.width != width || menuParams.height != height) {
            menuParams.width = width
            menuParams.height = height
            menu.layoutParams = menuParams
        }

        val iconParams = binding.projectSpaceAiIcon.layoutParams as LinearLayout.LayoutParams
        if (iconParams.marginEnd != iconMarginEnd) {
            iconParams.marginEnd = iconMarginEnd
            binding.projectSpaceAiIcon.layoutParams = iconParams
        }

        val label = binding.projectSpaceAiLabel
        val labelParams = label.layoutParams as LinearLayout.LayoutParams
        if (labelParams.width != labelWidth) {
            labelParams.width = labelWidth
            label.layoutParams = labelParams
        }
        label.alpha = 1f
        label.translationX = 0f
    }

    private fun measureProjectSpaceAiLabelWidth(): Int {
        val label = binding.projectSpaceAiLabel
        label.measure(
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED),
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED)
        )
        return label.measuredWidth.coerceAtLeast(dp(PROJECT_SPACE_AI_LABEL_MIN_WIDTH_DP))
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
        activePostMessageId = null
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
        val messageKey = activeChannelMessageKey(channel)
        activeAdapter?.notifyMessageUpdated(messagesByChannel[messageKey]?.indexOf(message) ?: -1)
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
                if (activeChannel?.id != channel.id || activeChannelMessageKey(channel) != messageKey) return@runOnUiThread
                val messages = messagesByChannel[messageKey] ?: return@runOnUiThread
                val index = messages.indexOfFirst { it.id == messageId }
                result.onSuccess { updated ->
                    if (index >= 0) {
                        messages[index] = updated.toChatMessage(activeSpace?.project?.role, channel)
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
        const val PROJECT_DESCRIPTION_MAX_CHARS = 240
        const val DOCS_CHANNEL_KIND = "docs"
        const val SUGGESTIONS_CHANNEL_KIND = "suggestions"
        const val PROJECT_SPACE_AI_ANIMATION_MS = 220L
        const val PROJECT_SPACE_AI_COLLAPSED_SIZE_DP = 64
        const val PROJECT_SPACE_AI_EXPANDED_WIDTH_DP = 176
        const val PROJECT_SPACE_AI_EXPAND_AT_TOP_DP = 4
        const val PROJECT_SPACE_AI_ICON_MARGIN_END_DP = 14
        const val PROJECT_SPACE_AI_LABEL_MIN_WIDTH_DP = 70

        fun canEditProjectDescription(role: String?): Boolean {
            return role?.trim()?.lowercase() in setOf("owner", "admin", "editor")
        }
    }
}
