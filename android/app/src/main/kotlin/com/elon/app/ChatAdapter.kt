package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Matrix
import android.graphics.Shader
import android.graphics.drawable.ColorDrawable
import android.text.method.LinkMovementMethod
import android.text.util.Linkify
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory
import androidx.recyclerview.widget.RecyclerView
import kotlin.math.sin

data class ChatMessage(
    val role: String,
    var content: String,
    var attachments: List<ChatAttachment>? = null,
    var sendStatus: String? = null,
    var evidenceTitle: String? = null,
    var evidenceDetails: String? = null,
    var evidenceExpanded: Boolean = false,
    var evidenceWorking: Boolean = false,
    var senderLabel: String? = null,
    var id: String? = null,
    var apkUrl: String? = null,
    var senderAvatarDataUrl: String? = null,
    var suggestionStatus: String? = null,
    var suggestionResolvedByName: String? = null,
    var suggestionResolvedAt: String? = null,
    var canResolveSuggestion: Boolean = false,
    var createdAtMs: Long = System.currentTimeMillis(),
    /** 仅对自己发出的消息（role == "user"）有效：对方已读时为 true */
    var isRead: Boolean = false,
    /** 回答本条消息的模型 ID，如 "gpt-4o-mini"、"qwen2:7b" */
    var modelUsed: String? = null,
    /** 若回答来自用户贡献的 PC 节点，填写节点 ID */
    var nodeId: String? = null,
    /** Codex 桌面会话深链，用于把公开过程跳转到原生 Codex 线程。 */
    var codexThreadUri: String? = null,
    /** 本条消息只是过程承载层，终态回复出现后会把 evidence 合并到最终回复。 */
    var processLayer: Boolean = false,
    /** 只有任务结束时的真正终态回复为 true，用于显示“最终回复”标签。 */
    var finalReply: Boolean = false,
    /** 流式气泡 ID，用于 AssistantChunk 追加内容（打字机效果） */
    var streamId: String? = null,
    var projectPostCard: ChatProjectPostCard? = null
)

class ChatAdapter(
    private val messages: MutableList<ChatMessage>,
    private val onPauseWork: (() -> Unit)? = null,
    private val onMessageLongPress: ((View, ChatMessage) -> Unit)? = null,
    private val onRetryFailedSend: ((ChatMessage) -> Unit)? = null,
    private val onProjectShareAction: ((ChatProjectShare) -> Unit)? = null,
    private val onProjectShareLongPress: ((View, ChatMessage, ChatProjectShare) -> Unit)? = null
) : RecyclerView.Adapter<ChatAdapter.VH>() {
    /** 处理消息气泡上的 APK 操作按钮（安装 / 复制链接 / 分享），由 Activity 注入。 */
    var onApkAction: ((action: String, url: String) -> Unit)? = null
    var onSuggestionResolve: ((ChatMessage) -> Unit)? = null
    /** 语音附件长按回调（转文字 / 其他操作），由 Activity 通过 setAdapterAndWireApkActions 注入。 */
    var onVoiceAttachmentLongPress: ((message: ChatMessage, attachment: ChatAttachment) -> Unit)? = null
    private var cachedUserProfile: UserProfile? = null
    private var cachedUserBitmap: Bitmap? = null
    private var selectionMode = false
    private var selectionChangedListener: ((Int) -> Unit)? = null
    private val selectedPositions = linkedSetOf<Int>()
    private var lastTogglePosition = RecyclerView.NO_POSITION
    private var lastToggleAtMs = 0L

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val selectionCheck: TextView? = view.findViewById(R.id.messageSelectionCheck)
        val timelineLabel: TextView? = view.findViewById(R.id.messageTimelineLabel)
        val text: TextView = view.findViewById(R.id.messageText)
        val status: TextView? = view.findViewById(R.id.messageStatus)
        val attachmentList: LinearLayout? = view.findViewById(R.id.messageAttachmentList)
        val bubble: LinearLayout? = view.findViewById(R.id.messageBubble)
        val evidenceSummary: TextView? = view.findViewById(R.id.evidenceSummary)
        val evidenceDetails: TextView? = view.findViewById(R.id.evidenceDetails)
        val evidenceLastEntry: TextView? = view.findViewById(R.id.evidenceLastEntry)
        val finalReplyLabel: TextView? = view.findViewById(R.id.finalReplyLabel)
        val pauseButton: ImageButton? = view.findViewById(R.id.pauseWorkButton)
        val userAvatar: TextView? = view.findViewById(R.id.userAvatar)
        val friendAvatar: TextView? = view.findViewById(R.id.friendAvatar)
        val apkActionBar: LinearLayout? = view.findViewById(R.id.apkActionBar)
        val modelAttribution: TextView? = view.findViewById(R.id.modelAttributionText)
        var shimmerAnimator: ValueAnimator? = null
        var evidenceShimmerAnimator: ValueAnimator? = null

        fun stopShimmer() {
            shimmerAnimator?.cancel()
            shimmerAnimator = null
            text.paint.shader = null
            text.alpha = 1f
            text.invalidate()

            evidenceShimmerAnimator?.cancel()
            evidenceShimmerAnimator = null
            evidenceSummary?.paint?.shader = null
            evidenceSummary?.alpha = 1f
            evidenceSummary?.invalidate()
            evidenceDetails?.paint?.shader = null
            evidenceDetails?.alpha = 1f
            evidenceDetails?.invalidate()
            evidenceLastEntry?.paint?.shader = null
            evidenceLastEntry?.alpha = 1f
            evidenceLastEntry?.invalidate()
        }
    }

    override fun getItemViewType(position: Int): Int {
        if (messages[position].projectPostCard != null) return 6
        if (parseChatProjectShareMessage(messages[position].content) != null) return 5
        return when (messages[position].role) {
            "user"        -> 0
            "ai"          -> 1
            "ai-intent"   -> 1
            "friend"      -> 4
            "ai-working"  -> 2
            "ai-progress" -> 2
            "ai-cli-log"  -> 2
            "ai-tool"     -> 2
            "ai-complete" -> 2
            "ai-stopped"  -> 2
            "error"       -> 3
            else          -> 1
        }
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): VH {
        val layout = when (viewType) {
            0    -> R.layout.item_message_user
            2    -> R.layout.item_message_progress
            3    -> R.layout.item_message_error
            4    -> R.layout.item_message_friend
            5    -> R.layout.item_message_project_share
            6    -> R.layout.item_message_project_post
            else -> R.layout.item_message_ai
        }
        val view = LayoutInflater.from(parent.context).inflate(layout, parent, false)
        return VH(view)
    }

    override fun onBindViewHolder(holder: VH, position: Int) {
        val message = messages[position]
        holder.stopShimmer()
        bindTimelineLabel(holder.timelineLabel, position)
        bindChatAttachmentViews(
            holder.attachmentList,
            message.attachments,
            isSent = message.role == "user",
            onVoiceLongPress = onVoiceAttachmentLongPress?.let { cb ->
                { attachment -> cb.invoke(message, attachment) }
            }
        )
        bindChatSuggestionStatus(holder.attachmentList, message, onSuggestionResolve)
        val postCardBound = bindChatProjectPostCardView(holder.attachmentList, holder.text, message)
        val projectShareBound = if (postCardBound) {
            false
        } else {
            bindChatProjectShareView(
                holder.attachmentList,
                holder.text,
                message,
                onProjectShareAction,
                onProjectShareLongPress
            )
        }
        val projectCardBound = postCardBound || projectShareBound
        applyChatProjectBubbleStyle(holder.bubble, message.role, projectCardBound)
        applyImageOnlyBubbleStyle(holder.bubble, message, projectCardBound)
        applyVoiceOnlyBubbleStyle(holder.bubble, message, projectCardBound)
        if (!projectCardBound) {
            holder.text.text = message.content
            holder.text.visibility = if (message.content.isBlank() && !message.attachments.isNullOrEmpty()) {
                View.GONE
            } else {
                View.VISIBLE
            }
            holder.text.setTextColor(messageTextColor(message.role))
            Linkify.addLinks(holder.text, Linkify.WEB_URLS)
            holder.text.movementMethod = LinkMovementMethod.getInstance()
        }
        bindSendStatus(holder, message)
        bindUserAvatar(holder.userAvatar)
        bindFriendAvatar(holder.friendAvatar, message)
        bindSelectionVisual(holder, message, projectCardBound, position)
        bindMessageActions(holder, message, projectCardBound)
        bindFinalReplyLabel(holder, message)
        bindEvidence(holder, message, position)
        bindModelAttribution(holder, message)
        if (message.role in shimmerWorkflowRoles) startShimmer(holder, message.role)
        val canPause = position == messages.lastIndex && message.role in activeWorkflowRoles && onPauseWork != null
        holder.pauseButton?.visibility = if (canPause) View.VISIBLE else View.GONE
        holder.pauseButton?.setOnClickListener {
            if (message.role in activeWorkflowRoles) onPauseWork?.invoke()
        }
        bindApkActionBar(holder, message)
    }

    private fun bindApkActionBar(holder: VH, message: ChatMessage) {
        val bar = holder.apkActionBar ?: return
        val url = message.apkUrl
        if (url.isNullOrBlank()) {
            bar.visibility = View.GONE
            return
        }
        bar.visibility = View.VISIBLE
        bar.findViewById<android.widget.Button>(R.id.apkInstallBtn)?.setOnClickListener {
            onApkAction?.invoke("install", url)
        }
        bar.findViewById<android.widget.Button>(R.id.apkCopyBtn)?.setOnClickListener {
            onApkAction?.invoke("copy", url)
        }
        bar.findViewById<android.widget.Button>(R.id.apkShareBtn)?.setOnClickListener {
            onApkAction?.invoke("share", url)
        }
    }

    private fun applyImageOnlyBubbleStyle(
        bubble: LinearLayout?,
        message: ChatMessage,
        projectCardBound: Boolean
    ) {
        if (bubble == null || projectCardBound || !message.isImageOnlyMessage()) return
        bubble.background = ColorDrawable(Color.TRANSPARENT)
        bubble.setPadding(0, 0, 0, 0)
    }

    private fun applyVoiceOnlyBubbleStyle(
        bubble: LinearLayout?,
        message: ChatMessage,
        projectCardBound: Boolean
    ) {
        if (bubble == null || projectCardBound || !message.isVoiceOnlyMessage()) return
        bubble.background = ColorDrawable(Color.TRANSPARENT)
        bubble.setPadding(0, 0, 0, 0)
    }

    private fun bindSendStatus(holder: VH, message: ChatMessage) {
        val status = holder.status ?: return
        val canRetry = message.canRetryFailedAttachmentSend()
        val text = when {
            canRetry -> "发送失败，点此重试"
            message.sendStatus?.isNotBlank() == true -> message.sendStatus!!
            message.isRead && message.role == "user" -> "已读"
            else -> null
        }
        status.visibility = if (text == null) View.GONE else View.VISIBLE
        status.text = text.orEmpty()
        status.setTextColor(Color.parseColor(
            if (canRetry) "#C62828" else if (message.isRead && message.role == "user") "#58BE6A" else "#66111111"
        ))
        status.isClickable = canRetry
        status.isFocusable = canRetry
        status.setOnClickListener(
            if (canRetry) {
                View.OnClickListener {
                    val position = holder.adapterPosition
                    val current = messages.getOrNull(position) ?: message
                    if (current.canRetryFailedAttachmentSend()) onRetryFailedSend?.invoke(current)
                }
            } else {
                null
            }
        )
    }

    override fun getItemCount() = messages.size

    override fun onViewRecycled(holder: VH) {
        holder.stopShimmer()
        super.onViewRecycled(holder)
    }

    fun notifyMessageUpdated(index: Int) {
        if (index in messages.indices) notifyItemChanged(index)
        if (index + 1 in messages.indices) notifyItemChanged(index + 1)
    }

    fun notifyMessageRemoved(index: Int) {
        notifyItemRemoved(index)
        if (index in messages.indices) notifyItemChanged(index)
    }

    fun refreshUserProfile() {
        cachedUserProfile = null
        cachedUserBitmap = null
        notifyDataSetChanged()
    }

    fun setSelectionChangedListener(listener: ((Int) -> Unit)?) {
        selectionChangedListener = listener
        if (selectionMode) selectionChangedListener?.invoke(selectedPositions.size)
    }

    fun startSelection(message: ChatMessage) {
        if (!isSelectableMessage(message)) return
        val position = messages.indexOfFirst { it === message }.takeIf { it >= 0 }
            ?: messages.indexOf(message)
        if (position !in messages.indices) return
        selectionMode = true
        selectedPositions.clear()
        selectedPositions.add(position)
        notifyDataSetChanged()
        notifySelectionChanged()
    }

    fun exitSelection() {
        if (!selectionMode && selectedPositions.isEmpty()) return
        selectionMode = false
        selectedPositions.clear()
        lastTogglePosition = RecyclerView.NO_POSITION
        lastToggleAtMs = 0L
        notifyDataSetChanged()
        notifySelectionChanged()
    }

    fun isSelectionModeActive(): Boolean = selectionMode

    fun selectedMessagesInOrder(): List<ChatMessage> {
        return selectedPositions
            .sorted()
            .mapNotNull { position -> messages.getOrNull(position) }
    }

    fun selectedPositionsDescending(): List<Int> {
        return selectedPositions.sortedDescending()
    }

    fun ownsMessages(candidate: List<ChatMessage>): Boolean {
        return messages === candidate
    }

    fun addMessage(msg: ChatMessage) {
        if (messages.isNotEmpty() && shouldDropLastTransientBefore(msg)) {
            val lastIndex = messages.lastIndex
            messages.removeAt(lastIndex)
            notifyItemRemoved(lastIndex)
        }

        if (shouldReplaceLastMessage(msg)) {
            val lastIndex = messages.lastIndex
            messages[lastIndex] = msg
            notifyItemChanged(lastIndex)
            return
        }

        messages.add(msg)
        notifyItemInserted(messages.size - 1)
    }

    /**
     * 流式追加：找到具有相同 streamId 的最后一条消息，把 [chunk] 追加到它的 content，
     * 然后 notifyItemChanged 触发气泡原地刷新（打字机效果）。
     * 若找不到对应气泡则忽略（容错：可能先到 chunk 后到 AssistantMessage）。
     */
    fun streamAppendChunk(streamId: String, chunk: String) {
        val idx = messages.indexOfLast { it.streamId == streamId }
        if (idx < 0) return
        messages[idx].content += chunk
        notifyItemChanged(idx)
    }

    private fun bindTimelineLabel(label: TextView?, position: Int) {
        label ?: return
        val message = messages.getOrNull(position) ?: run {
            label.visibility = View.GONE
            return
        }
        if (!shouldShowChatTimelineLabel(messages, position)) {
            label.visibility = View.GONE
            return
        }
        val text = formatChatTimelineLabel(message.createdAtMs)
        label.visibility = if (text.isBlank()) View.GONE else View.VISIBLE
        label.text = text
    }

    private fun bindMessageActions(holder: VH, message: ChatMessage, projectCardBound: Boolean) {
        val canSelect = !projectCardBound && isSelectableMessage(message)
        if (selectionMode) {
            val clickListener = if (canSelect) {
                View.OnClickListener {
                    val position = holder.adapterPosition
                    val current = messages.getOrNull(position) ?: return@OnClickListener
                    toggleSelection(current, position)
                }
            } else {
                null
            }
            holder.itemView.setOnClickListener(clickListener)
            holder.bubble?.setOnClickListener(clickListener)
            holder.text.setOnClickListener(null)
            holder.text.isClickable = false
            holder.itemView.setOnLongClickListener(null)
            holder.bubble?.setOnLongClickListener(null)
            holder.text.setOnLongClickListener(null)
            holder.itemView.isLongClickable = false
            holder.bubble?.isLongClickable = false
            holder.text.isLongClickable = false
            return
        }

        holder.itemView.setOnClickListener(null)
        holder.bubble?.setOnClickListener(null)
        holder.text.setOnClickListener(null)
        holder.text.isClickable = true
        val canAct = !projectCardBound && isActionableMessage(message) && onMessageLongPress != null
        holder.itemView.isLongClickable = canAct
        holder.text.isLongClickable = canAct

        val listener = if (canAct) {
            View.OnLongClickListener {
                val position = holder.adapterPosition
                val current = messages.getOrNull(position) ?: message
                onMessageLongPress?.invoke(holder.text, current)
                true
            }
        } else {
            null
        }
        holder.itemView.setOnLongClickListener(listener)
        holder.bubble?.setOnLongClickListener(listener)
        holder.text.setOnLongClickListener(listener)
    }

    private fun bindSelectionVisual(holder: VH, message: ChatMessage, projectCardBound: Boolean, position: Int) {
        val canSelect = !projectCardBound && isSelectableMessage(message)
        val selected = selectionMode && selectedPositions.contains(position)
        holder.itemView.setBackgroundColor(Color.TRANSPARENT)
        holder.itemView.alpha = if (selectionMode && !canSelect) 0.62f else 1f
        holder.bubble?.alpha = 1f
        holder.selectionCheck?.visibility = if (selectionMode && canSelect) View.VISIBLE else View.GONE
        holder.selectionCheck?.text = if (selected) "✓" else ""
        holder.selectionCheck?.setBackgroundResource(
            if (selected) R.drawable.bg_message_selection_on else R.drawable.bg_message_selection_off
        )
        holder.itemView.contentDescription = if (selectionMode && canSelect) {
            if (selected) "已选中消息，点击取消选择" else "未选中消息，点击选择"
        } else {
            null
        }
    }

    private fun toggleSelection(message: ChatMessage, position: Int) {
        if (!isSelectableMessage(message)) return
        if (position !in messages.indices) return
        val now = android.os.SystemClock.elapsedRealtime()
        if (position == lastTogglePosition && now - lastToggleAtMs < 180L) return
        lastTogglePosition = position
        lastToggleAtMs = now
        if (selectedPositions.contains(position)) {
            selectedPositions.remove(position)
        } else {
            selectedPositions.add(position)
        }
        if (selectedPositions.isEmpty()) {
            exitSelection()
            return
        }
        if (position != RecyclerView.NO_POSITION) notifyItemChanged(position)
        notifySelectionChanged()
    }

    private fun isSelectableMessage(message: ChatMessage): Boolean {
        if (message.role !in selectableMessageRoles) return false
        return isActionableMessage(message)
    }

    private fun isActionableMessage(message: ChatMessage): Boolean {
        return message.content.isNotBlank() || !message.attachments.isNullOrEmpty()
    }

    private fun notifySelectionChanged() {
        selectionChangedListener?.invoke(selectedPositions.size)
    }

    private fun bindUserAvatar(avatar: TextView?) {
        avatar ?: return
        val profile = cachedUserProfile ?: UserProfileStore.load(avatar.context).also {
            cachedUserProfile = it
            cachedUserBitmap = null  // profile 更新时清除 bitmap 缓存
        }
        val bitmap = cachedUserBitmap ?: UserProfileStore.decodeAvatar(profile.avatarDataUrl).also {
            cachedUserBitmap = it
        }
        if (bitmap != null) {
            val radius = (6 * avatar.resources.displayMetrics.density + 0.5f).toInt()
            avatar.background = RoundedBitmapDrawableFactory.create(avatar.resources, bitmap).apply {
                cornerRadius = radius.toFloat()
                setAntiAlias(true)
            }
            avatar.text = ""
        } else {
            avatar.setBackgroundResource(R.drawable.bg_avatar_user)
            avatar.text = UserProfileStore.avatarInitial(profile.displayName)
        }
        avatar.contentDescription = "我的头像"
    }

    private fun bindFriendAvatar(avatar: TextView?, message: ChatMessage) {
        avatar ?: return
        val label = message.senderLabel?.trim().orEmpty()
        val bitmap = UserProfileStore.decodeAvatar(message.senderAvatarDataUrl)
        if (bitmap != null) {
            val radius = (6 * avatar.resources.displayMetrics.density + 0.5f).toInt()
            avatar.background = RoundedBitmapDrawableFactory.create(avatar.resources, bitmap).apply {
                cornerRadius = radius.toFloat()
                setAntiAlias(true)
            }
            avatar.text = ""
        } else {
            avatar.setBackgroundResource(R.drawable.bg_mock_avatar)
            avatar.text = UserProfileStore.avatarInitial(label.ifBlank { "好友" })
        }
        avatar.contentDescription = label.ifBlank { "好友头像" }
    }

    private fun messageTextColor(role: String): Int = when (role) {
        "ai", "ai-intent", "friend" -> Color.parseColor("#D6D6D6")
        "ai-stopped" -> Color.parseColor("#D9B66B")
        "ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete" -> Color.parseColor("#A8A8A8")
        "error" -> Color.parseColor("#C62828")
        else -> Color.parseColor("#101010")
    }

    private fun startShimmer(holder: VH, expectedRole: String) {
        val text = holder.text
        text.post {
            val width = text.width.coerceAtLeast(text.measuredWidth)
            val position = holder.adapterPosition
            if (width <= 0 || position == RecyclerView.NO_POSITION) return@post
            if (messages.getOrNull(position)?.role != expectedRole) return@post

            val shader = LinearGradient(
                0f,
                0f,
                width.toFloat(),
                0f,
                intArrayOf(
                    Color.parseColor("#A8A8A8"),
                    Color.parseColor("#CFCFCF"),
                    Color.parseColor("#F6F6F6"),
                    Color.parseColor("#D6D6D6"),
                    Color.parseColor("#A8A8A8")
                ),
                floatArrayOf(0f, 0.28f, 0.5f, 0.72f, 1f),
                Shader.TileMode.CLAMP
            )
            val matrix = Matrix()
            text.paint.shader = shader

            holder.shimmerAnimator?.cancel()
            holder.shimmerAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 1350L
                repeatCount = ValueAnimator.INFINITE
                repeatMode = ValueAnimator.RESTART
                interpolator = LinearInterpolator()
                addUpdateListener { animator ->
                    val fraction = animator.animatedFraction
                    matrix.setTranslate(width * (fraction * 2f - 1f), 0f)
                    shader.setLocalMatrix(matrix)
                    text.alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    text.invalidate()
                }
                start()
            }
        }
    }

    private fun bindModelAttribution(holder: VH, message: ChatMessage) {
        val tv = holder.modelAttribution ?: return
        val model = message.modelUsed
        if (model.isNullOrBlank() || message.role !in setOf("ai", "assistant", "ai-intent")) {
            tv.visibility = View.GONE
            return
        }
        tv.text = if (!message.nodeId.isNullOrBlank()) "🖥️ $model" else "✦ $model"
        tv.visibility = View.VISIBLE
    }

    private fun bindFinalReplyLabel(holder: VH, message: ChatMessage) {
        val label = holder.finalReplyLabel ?: return
        val show = message.role == "ai" &&
            !message.processLayer &&
            message.finalReply &&
            !message.evidenceTitle.isNullOrBlank()
        label.visibility = if (show) View.VISIBLE else View.GONE
        label.text = "最终回复"
    }

    private fun bindEvidence(holder: VH, message: ChatMessage, position: Int) {
        val summary = holder.evidenceSummary ?: return
        val details = holder.evidenceDetails ?: return
        val lastEntry = holder.evidenceLastEntry
        val hasEvidence = message.role in evidenceBubbleRoles &&
            !message.evidenceTitle.isNullOrBlank() &&
            !message.evidenceDetails.isNullOrBlank()

        if (!hasEvidence) {
            summary.visibility = View.GONE
            details.visibility = View.GONE
            lastEntry?.visibility = View.GONE
            return
        }

        val marker = if (message.evidenceExpanded) "⌄" else "›"
        summary.text = "$marker ${message.evidenceTitle}"
        summary.visibility = View.VISIBLE

        if (message.evidenceExpanded) {
            // 展开：summary 静止，最后一条 entry 闪烁
            summary.paint.shader = null
            summary.alpha = 1f

            val lines = message.evidenceDetails!!.split("\n")
            val allButLast = lines.dropLast(1).joinToString("\n")
            val last = lines.last()

            if (allButLast.isBlank()) {
                details.visibility = View.GONE
            } else {
                details.text = allButLast
                details.visibility = View.VISIBLE
            }

            if (lastEntry != null) {
                lastEntry.text = last
                lastEntry.visibility = View.VISIBLE
                val dp4 = (4 * lastEntry.resources.displayMetrics.density + 0.5f).toInt()
                (lastEntry.layoutParams as? android.view.ViewGroup.MarginLayoutParams)?.topMargin =
                    if (details.visibility == View.VISIBLE) 0 else dp4
            } else {
                details.text = message.evidenceDetails
                details.visibility = View.VISIBLE
            }

            if (message.evidenceWorking) {
                startEvidenceShimmerOnLastEntry(holder, message)
            }
        } else {
            // 折叠：details 隐藏，summary 标题闪烁
            details.visibility = View.GONE
            lastEntry?.visibility = View.GONE
            if (message.evidenceWorking) {
                startEvidenceShimmerOnSummary(holder, message)
            }
        }

        summary.setOnClickListener {
            message.evidenceExpanded = !message.evidenceExpanded
            notifyItemChanged(position)
        }
    }

    private fun startEvidenceShimmerOnSummary(holder: VH, expectedMessage: ChatMessage) {
        val summary = holder.evidenceSummary ?: return
        summary.post {
            val position = holder.adapterPosition
            if (position == RecyclerView.NO_POSITION) return@post
            if (messages.getOrNull(position) !== expectedMessage || !expectedMessage.evidenceWorking) return@post
            if (expectedMessage.evidenceExpanded) return@post

            val width = summary.width.coerceAtLeast(summary.measuredWidth)
            if (width <= 0) return@post

            val shader = buildEvidenceShader(width)
            val matrix = Matrix()
            holder.evidenceShimmerAnimator?.cancel()
            holder.evidenceShimmerAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 1350L
                repeatCount = ValueAnimator.INFINITE
                repeatMode = ValueAnimator.RESTART
                interpolator = LinearInterpolator()
                addUpdateListener { animator ->
                    val fraction = animator.animatedFraction
                    matrix.setTranslate(width * (fraction * 2f - 1f), 0f)
                    shader.setLocalMatrix(matrix)
                    summary.paint.shader = shader
                    summary.alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    summary.invalidate()
                }
                start()
            }
        }
    }

    private fun startEvidenceShimmerOnLastEntry(holder: VH, expectedMessage: ChatMessage) {
        val lastEntry = holder.evidenceLastEntry ?: return
        lastEntry.post {
            val position = holder.adapterPosition
            if (position == RecyclerView.NO_POSITION) return@post
            if (messages.getOrNull(position) !== expectedMessage || !expectedMessage.evidenceWorking) return@post
            if (!expectedMessage.evidenceExpanded) return@post

            val width = lastEntry.width.coerceAtLeast(lastEntry.measuredWidth)
            if (width <= 0) return@post

            val shader = buildEvidenceShader(width)
            val matrix = Matrix()
            holder.evidenceShimmerAnimator?.cancel()
            holder.evidenceShimmerAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
                duration = 1350L
                repeatCount = ValueAnimator.INFINITE
                repeatMode = ValueAnimator.RESTART
                interpolator = LinearInterpolator()
                addUpdateListener { animator ->
                    val fraction = animator.animatedFraction
                    matrix.setTranslate(width * (fraction * 2f - 1f), 0f)
                    shader.setLocalMatrix(matrix)
                    lastEntry.paint.shader = shader
                    lastEntry.alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    lastEntry.invalidate()
                }
                start()
            }
        }
    }

    private fun buildEvidenceShader(width: Int): LinearGradient = LinearGradient(
        0f, 0f, width.toFloat(), 0f,
        intArrayOf(
            Color.parseColor("#777777"),
            Color.parseColor("#CFCFCF"),
            Color.parseColor("#F6F6F6"),
            Color.parseColor("#D6D6D6"),
            Color.parseColor("#777777")
        ),
        floatArrayOf(0f, 0.28f, 0.5f, 0.72f, 1f),
        Shader.TileMode.CLAMP
    )

    private fun shouldReplaceLastMessage(msg: ChatMessage): Boolean {
        if (messages.isEmpty()) return false
        val lastRole = messages.last().role
        return lastRole in transientWorkflowRoles && msg.role in workflowStatusRoles
    }

    private fun shouldDropLastTransientBefore(msg: ChatMessage): Boolean {
        val lastRole = messages.lastOrNull()?.role ?: return false
        return lastRole in transientWorkflowRoles && msg.role in terminalRoles
    }

    private companion object {
        val activeWorkflowRoles = setOf("ai-working", "ai-progress", "ai-tool")
        val shimmerWorkflowRoles = setOf("ai-working")
        val transientWorkflowRoles = setOf("ai-working", "ai-progress", "ai-tool", "ai-cli-log")
        val workflowStatusRoles = setOf("ai-working", "ai-progress", "ai-tool", "ai-cli-log", "ai-complete", "ai-stopped")
        val terminalRoles = setOf("ai", "ai-intent", "error")
        val evidenceBubbleRoles = setOf("ai", "ai-intent")
        val selectableMessageRoles = setOf("user", "ai", "ai-intent", "ai-complete", "friend")
    }
}

private fun ChatMessage.canRetryFailedAttachmentSend(): Boolean {
    return role == "user" &&
        !attachments.isNullOrEmpty() &&
        sendStatus.orEmpty().contains("失败")
}

private fun ChatMessage.isImageOnlyMessage(): Boolean {
    val items = attachments.orEmpty()
    return content.isBlank() && items.isNotEmpty() && items.all { it.isImage() }
}

private fun ChatMessage.isVoiceOnlyMessage(): Boolean {
    val items = attachments.orEmpty()
    return content.isBlank() && items.isNotEmpty() && items.all { it.isVoice() }
}
