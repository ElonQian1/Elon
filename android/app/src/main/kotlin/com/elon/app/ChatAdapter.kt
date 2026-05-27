package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.LinearGradient
import android.graphics.Matrix
import android.graphics.Shader
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
    var id: String? = null
)

class ChatAdapter(
    private val messages: MutableList<ChatMessage>,
    private val onPauseWork: (() -> Unit)? = null,
    private val onMessageLongPress: ((View, ChatMessage) -> Unit)? = null,
    private val onRetryFailedSend: ((ChatMessage) -> Unit)? = null,
    private val onProjectShareAction: ((ChatProjectShare) -> Unit)? = null,
    private val onProjectShareLongPress: ((View, ChatMessage, ChatProjectShare) -> Unit)? = null
) :
    RecyclerView.Adapter<ChatAdapter.VH>() {
    private var cachedUserProfile: UserProfile? = null

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val text: TextView = view.findViewById(R.id.messageText)
        val status: TextView? = view.findViewById(R.id.messageStatus)
        val attachmentList: LinearLayout? = view.findViewById(R.id.messageAttachmentList)
        val bubble: LinearLayout? = view.findViewById(R.id.messageBubble)
        val evidenceSummary: TextView? = view.findViewById(R.id.evidenceSummary)
        val evidenceDetails: TextView? = view.findViewById(R.id.evidenceDetails)
        val evidenceLastEntry: TextView? = view.findViewById(R.id.evidenceLastEntry)
        val pauseButton: ImageButton? = view.findViewById(R.id.pauseWorkButton)
        val userAvatar: TextView? = view.findViewById(R.id.userAvatar)
        val friendAvatar: TextView? = view.findViewById(R.id.friendAvatar)
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
            else -> R.layout.item_message_ai
        }
        val view = LayoutInflater.from(parent.context).inflate(layout, parent, false)
        return VH(view)
    }

    override fun onBindViewHolder(holder: VH, position: Int) {
        val message = messages[position]
        holder.stopShimmer()
        bindChatAttachmentViews(holder.attachmentList, message.attachments)
        val projectCardBound = bindChatProjectShareView(
            holder.attachmentList,
            holder.text,
            message,
            onProjectShareAction,
            onProjectShareLongPress
        )
        applyChatProjectBubbleStyle(holder.bubble, message.role, projectCardBound)
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
        holder.friendAvatar?.text = message.senderLabel?.trim()?.take(1)?.ifBlank { "友" } ?: "友"
        bindMessageActions(holder, message, projectCardBound)
        bindEvidence(holder, message, position)
        if (message.role in shimmerWorkflowRoles) startShimmer(holder, message.role)
        val canPause = position == messages.lastIndex && message.role in activeWorkflowRoles && onPauseWork != null
        holder.pauseButton?.visibility = if (canPause) View.VISIBLE else View.GONE
        holder.pauseButton?.setOnClickListener {
            if (message.role in activeWorkflowRoles) onPauseWork?.invoke()
        }
    }

    private fun bindSendStatus(holder: VH, message: ChatMessage) {
        val status = holder.status ?: return
        val text = message.sendStatus?.takeIf { it.isNotBlank() }
        val canRetry = message.canRetryFailedAttachmentSend()
        status.visibility = if (text == null) View.GONE else View.VISIBLE
        status.text = if (canRetry) "发送失败，点此重试" else text.orEmpty()
        status.setTextColor(Color.parseColor(if (canRetry) "#C62828" else "#66111111"))
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
    }

    fun refreshUserProfile() {
        cachedUserProfile = null
        notifyDataSetChanged()
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

    private fun bindMessageActions(holder: VH, message: ChatMessage, projectCardBound: Boolean) {
        val canAct = !projectCardBound && onMessageLongPress != null && message.content.isNotBlank()
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
        holder.text.setOnLongClickListener(listener)
    }

    private fun bindUserAvatar(avatar: TextView?) {
        avatar ?: return
        val profile = cachedUserProfile ?: UserProfileStore.load(avatar.context).also {
            cachedUserProfile = it
        }
        val bitmap = UserProfileStore.decodeAvatar(profile.avatarDataUrl)
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

    private fun messageTextColor(role: String): Int = when (role) {
        "ai", "ai-intent", "friend" -> Color.parseColor("#F4F4F4")
        "ai-stopped" -> Color.parseColor("#D9B66B")
        "ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete" -> Color.parseColor("#9A9A9A")
        "error" -> Color.parseColor("#C62828")
        else -> Color.parseColor("#111111")
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
                    Color.parseColor("#9A9A9A"),
                    Color.parseColor("#CFCFCF"),
                    Color.parseColor("#F6F6F6"),
                    Color.parseColor("#D8D8D8"),
                    Color.parseColor("#9A9A9A")
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
            Color.parseColor("#8D8D8D"),
            Color.parseColor("#CFCFCF"),
            Color.parseColor("#F6F6F6"),
            Color.parseColor("#D8D8D8"),
            Color.parseColor("#8D8D8D")
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
    }
}

private fun ChatMessage.canRetryFailedAttachmentSend(): Boolean {
    return role == "user" &&
        !attachments.isNullOrEmpty() &&
        sendStatus.orEmpty().contains("失败")
}
