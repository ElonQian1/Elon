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
    private val collapseInputComposer: () -> Unit,
    private val openPersonalAiChat: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?
) {
    private val pollHandler = Handler(Looper.getMainLooper())
    private val messagesByChannel = linkedMapOf<String, MutableList<ChatMessage>>()
    private var activeSpace: ProjectSpace? = null
    private var activeProjectId: String? = null
    private var activeProjectTitle: String = "项目空间"
    private var activeChannel: ProjectChannel? = null
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
        activeProjectId = projectId
        activeProjectTitle = title.ifBlank { "项目空间" }
        activeSpace = null
        activeChannel = null
        activeAdapter = null
        stopPolling()
        showProjectSpace(activeProjectTitle, animate)
        renderLoading()
        thread {
            val result = runCatching { fetchProjectSpace(http, serverUrl, activity, projectId) }
            activity.runOnUiThread {
                result.onSuccess {
                    activeSpace = it
                    activeProjectTitle = it.project.name
                    showProjectSpace(activeProjectTitle, false)
                    renderActiveSpace()
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
        container.addView(sectionTitle("成员"))
        container.addView(memberSummary(space.members))
        container.addView(personalAiEntry())
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
        val pending = ChatMessage("user", text, sendStatus = SENDING_STATUS)
        messages.add(pending)
        activeAdapter?.notifyItemInserted(messages.lastIndex)
        binding.chatList.scrollToPosition(messages.lastIndex)
        binding.inputEdit.text.clear()
        collapseInputComposer()

        thread {
            val result = runCatching {
                if (channel.kind == AI_CHANNEL_KIND) {
                    startProjectChannelAiTask(http, serverUrl, activity, channel.projectId, channel.id, text)
                } else {
                    sendProjectChannelMessage(http, serverUrl, activity, channel.projectId, channel.id, text)
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

    private fun openChannel(channel: ProjectChannel) {
        activeChannel = channel
        val messages = messagesByChannel.getOrPut(channel.id) { mutableListOf() }
        val adapter = ChatAdapter(messages, onMessageLongPress = showMessageActions)
        activeAdapter = adapter
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        showProjectChannelChat("#${channel.name}", true)
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
        thread {
            val result = runCatching {
                fetchProjectChannelMessages(http, serverUrl, activity, channel.projectId, channel.id)
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
            setTextColor(Color.parseColor("#B8B8B8"))
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
            background = panelBackground("#202020")
            addView(TextView(activity).apply {
                text = space.project.name
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F0F0F0"))
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
                setTextColor(Color.parseColor("#A8A8A8"))
                setPadding(0, dp(8), 0, 0)
            })
        }
    }

    private fun sectionTitle(textValue: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 13f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor("#888888"))
            setPadding(dp(20), dp(18), dp(20), dp(8))
        }
    }

    private fun channelRow(channel: ProjectChannel): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground("#1A1A1A")
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
                setTextColor(Color.parseColor("#E2E2E2"))
            })
            addView(TextView(activity).apply {
                text = channelHint(channel)
                textSize = 12f
                setTextColor(Color.parseColor("#8E8E8E"))
                setPadding(0, dp(5), 0, 0)
                maxLines = 2
                ellipsize = android.text.TextUtils.TruncateAt.END
            })
        }
    }

    private fun memberSummary(members: List<ProjectMember>): TextView {
        return TextView(activity).apply {
            text = if (members.isEmpty()) {
                "暂无成员"
            } else {
                members.take(12).joinToString("  ·  ") { "${it.account} (${roleLabel(it.role)})" }
            }
            textSize = 13f
            setTextColor(Color.parseColor("#B8B8B8"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#1A1A1A")
        }
    }

    private fun personalAiEntry(): TextView {
        return TextView(activity).apply {
            text = "个人 AI 会话"
            textSize = 16f
            setTextColor(Color.parseColor("#E2E2E2"))
            setPadding(dp(20), dp(16), dp(20), dp(16))
            background = panelBackground("#1A1A1A")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPersonalAiChat() }
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
            senderLabel = if (role == "friend") senderName else null
        )
    }

    private fun channelHint(channel: ProjectChannel): String {
        channel.lastMessage?.takeIf { it.isNotBlank() }?.let { return it }
        return when (channel.kind) {
            "announcements" -> "项目公告、规则和重要更新。"
            "discussion" -> "成员日常讨论和协作交流。"
            "requirements" -> "集中提出功能想法，后续可转为 AI 开发任务。"
            "issues" -> "反馈 bug、安装问题和体验问题。"
            AI_CHANNEL_KIND -> "在这里发消息会发起集体 AI 开发任务，过程和结果对成员可见。"
            "builds" -> "构建、发布、APK 下载和部署结果记录。"
            else -> "项目成员共享频道。"
        }
    }

    private fun roleLabel(role: String): String = when (role) {
        "owner" -> "所有者"
        "editor" -> "协作者"
        "member" -> "成员"
        else -> role
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
        const val AI_CHANNEL_KIND = "ai_development"
    }
}
