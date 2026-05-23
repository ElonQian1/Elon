package com.elon.app

import android.graphics.Color
import android.text.method.LinkMovementMethod
import android.text.util.Linkify
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView

data class ChatMessage(
    val role: String,
    val content: String,
    var evidenceTitle: String? = null,
    var evidenceDetails: String? = null,
    var evidenceExpanded: Boolean = false
)

class ChatAdapter(
    private val messages: MutableList<ChatMessage>,
    private val onPauseWork: (() -> Unit)? = null
) :
    RecyclerView.Adapter<ChatAdapter.VH>() {

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val text: TextView = view.findViewById(R.id.messageText)
        val evidenceSummary: TextView? = view.findViewById(R.id.evidenceSummary)
        val evidenceDetails: TextView? = view.findViewById(R.id.evidenceDetails)
        val pauseButton: ImageButton? = view.findViewById(R.id.pauseWorkButton)
    }

    override fun getItemViewType(position: Int): Int = when (messages[position].role) {
        "user"        -> 0
        "ai"          -> 1
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
        holder.text.paint.shader = null
        holder.text.alpha = 1f
        holder.text.text = message.content
        holder.text.setTextColor(messageTextColor(message.role))
        Linkify.addLinks(holder.text, Linkify.WEB_URLS)
        holder.text.movementMethod = LinkMovementMethod.getInstance()
        bindEvidence(holder, message, position)
        val canPause = position == messages.lastIndex && message.role in activeWorkflowRoles && onPauseWork != null
        holder.pauseButton?.visibility = if (canPause) View.VISIBLE else View.GONE
        holder.pauseButton?.setOnClickListener {
            if (message.role in activeWorkflowRoles) onPauseWork?.invoke()
        }
    }

    override fun getItemCount() = messages.size

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

    private fun messageTextColor(role: String): Int = when (role) {
        "ai" -> Color.parseColor("#F4F4F4")
        "ai-stopped" -> Color.parseColor("#D9B66B")
        "ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete" -> Color.parseColor("#9A9A9A")
        "error" -> Color.parseColor("#C62828")
        else -> Color.parseColor("#111111")
    }

    private fun bindEvidence(holder: VH, message: ChatMessage, position: Int) {
        val summary = holder.evidenceSummary ?: return
        val details = holder.evidenceDetails ?: return
        val hasEvidence = message.role == "ai" &&
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
        summary.setOnClickListener {
            message.evidenceExpanded = !message.evidenceExpanded
            notifyItemChanged(position)
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
        val transientWorkflowRoles = setOf("ai-working", "ai-progress", "ai-tool", "ai-cli-log")
        val workflowStatusRoles = setOf("ai-working", "ai-progress", "ai-tool", "ai-cli-log", "ai-complete", "ai-stopped")
        val terminalRoles = setOf("ai", "error")
    }
}
