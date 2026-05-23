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
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import kotlin.math.sin

data class ChatMessage(
    val role: String,
    val content: String,
    var evidenceTitle: String? = null,
    var evidenceDetails: String? = null,
    var evidenceExpanded: Boolean = false,
    var evidenceWorking: Boolean = false
)

class ChatAdapter(
    private val messages: MutableList<ChatMessage>,
    private val onPauseWork: (() -> Unit)? = null,
    private val onMessageLongPress: ((View, ChatMessage) -> Unit)? = null
) :
    RecyclerView.Adapter<ChatAdapter.VH>() {

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val text: TextView = view.findViewById(R.id.messageText)
        val evidenceSummary: TextView? = view.findViewById(R.id.evidenceSummary)
        val evidenceDetails: TextView? = view.findViewById(R.id.evidenceDetails)
        val pauseButton: ImageButton? = view.findViewById(R.id.pauseWorkButton)
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
        }
    }

    override fun getItemViewType(position: Int): Int = when (messages[position].role) {
        "user"        -> 0
        "ai"          -> 1
        "ai-intent"   -> 1
        "ai-working"  -> 2
        "ai-progress" -> 2
        "ai-cli-log"  -> 2
        "ai-tool"     -> 2
        "ai-complete" -> 2
        "ai-stopped"  -> 2
        "error"       -> 3
        else          -> 1
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): VH {
        val layout = when (viewType) {
            0    -> R.layout.item_message_user
            2    -> R.layout.item_message_progress
            3    -> R.layout.item_message_error
            else -> R.layout.item_message_ai
        }
        val view = LayoutInflater.from(parent.context).inflate(layout, parent, false)
        return VH(view)
    }

    override fun onBindViewHolder(holder: VH, position: Int) {
        val message = messages[position]
        holder.stopShimmer()
        holder.text.text = message.content
        holder.text.setTextColor(messageTextColor(message.role))
        Linkify.addLinks(holder.text, Linkify.WEB_URLS)
        holder.text.movementMethod = LinkMovementMethod.getInstance()
        bindMessageActions(holder, message)
        bindEvidence(holder, message, position)
        if (message.role in shimmerWorkflowRoles) startShimmer(holder, message.role)
        val canPause = position == messages.lastIndex && message.role in activeWorkflowRoles && onPauseWork != null
        holder.pauseButton?.visibility = if (canPause) View.VISIBLE else View.GONE
        holder.pauseButton?.setOnClickListener {
            if (message.role in activeWorkflowRoles) onPauseWork?.invoke()
        }
    }

    override fun getItemCount() = messages.size

    override fun onViewRecycled(holder: VH) {
        holder.stopShimmer()
        super.onViewRecycled(holder)
    }

    fun notifyMessageUpdated(index: Int) {
        if (index in messages.indices) notifyItemChanged(index)
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

    private fun bindMessageActions(holder: VH, message: ChatMessage) {
        val canAct = onMessageLongPress != null && message.content.isNotBlank()
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

    private fun messageTextColor(role: String): Int = when (role) {
        "ai", "ai-intent" -> Color.parseColor("#F4F4F4")
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
        val hasEvidence = message.role in evidenceBubbleRoles &&
            !message.evidenceTitle.isNullOrBlank() &&
            !message.evidenceDetails.isNullOrBlank()

        if (!hasEvidence) {
            summary.visibility = View.GONE
            details.visibility = View.GONE
            return
        }

        val marker = if (message.evidenceExpanded) "⌄" else "›"
        summary.text = "$marker ${message.evidenceTitle}"
        summary.visibility = View.VISIBLE
        details.text = message.evidenceDetails
        details.visibility = if (message.evidenceExpanded) View.VISIBLE else View.GONE
        if (message.evidenceWorking) {
            startEvidenceShimmer(holder, message)
        }
        summary.setOnClickListener {
            message.evidenceExpanded = !message.evidenceExpanded
            notifyItemChanged(position)
        }
    }

    private fun startEvidenceShimmer(holder: VH, expectedMessage: ChatMessage) {
        val summary = holder.evidenceSummary ?: return
        val details = holder.evidenceDetails
        summary.post {
            val position = holder.adapterPosition
            if (position == RecyclerView.NO_POSITION) return@post
            if (messages.getOrNull(position) !== expectedMessage || !expectedMessage.evidenceWorking) return@post

            val width = summary.width.coerceAtLeast(summary.measuredWidth)
            if (width <= 0) return@post

            val shader = LinearGradient(
                0f,
                0f,
                width.toFloat(),
                0f,
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
                    val alpha = 0.76f + 0.24f * sin(Math.PI * fraction).toFloat()
                    summary.paint.shader = shader
                    summary.alpha = alpha
                    summary.invalidate()
                    details?.takeIf { it.visibility == View.VISIBLE }?.let {
                        it.paint.shader = shader
                        it.alpha = alpha
                        it.invalidate()
                    }
                }
                start()
            }
        }
    }

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
