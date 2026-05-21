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

data class ChatMessage(val role: String, val content: String)

class ChatAdapter(
    private val messages: MutableList<ChatMessage>,
    private val onPauseWork: (() -> Unit)? = null
) :
    RecyclerView.Adapter<ChatAdapter.VH>() {

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val text: TextView = view.findViewById(R.id.messageText)
        val pauseButton: ImageButton? = view.findViewById(R.id.pauseWorkButton)
    }

    override fun getItemViewType(position: Int): Int = when (messages[position].role) {
        "user"        -> 0
        "ai"          -> 1
        "ai-working"  -> 2
        "ai-progress" -> 2
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
        val canPause = message.role in activeWorkflowRoles && onPauseWork != null
        holder.pauseButton?.visibility = if (canPause) View.VISIBLE else View.GONE
        holder.pauseButton?.setOnClickListener {
            if (message.role in activeWorkflowRoles) onPauseWork?.invoke()
        }
    }

    override fun getItemCount() = messages.size

    fun addMessage(msg: ChatMessage) {
        if (shouldReplaceLastMessage(msg)) {
            messages[messages.size - 1] = msg
            notifyItemChanged(messages.size - 1)
        } else {
            messages.add(msg)
            notifyItemInserted(messages.size - 1)
        }
    }

    private fun messageTextColor(role: String): Int = when (role) {
        "ai" -> Color.parseColor("#F4F4F4")
        "ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped" -> Color.WHITE
        "error" -> Color.parseColor("#C62828")
        else -> Color.parseColor("#111111")
    }

    private fun shouldReplaceLastMessage(msg: ChatMessage): Boolean {
        if (messages.isEmpty()) return false
        val lastRole = messages.last().role
        if (lastRole !in workflowRoles) return false
        return msg.role in workflowRoles || msg.role == "ai" || msg.role == "error"
    }

    private companion object {
        val workflowRoles = setOf("ai-working", "ai-progress", "ai-tool", "ai-complete", "ai-stopped")
        val activeWorkflowRoles = setOf("ai-working", "ai-progress", "ai-tool")
    }
}
