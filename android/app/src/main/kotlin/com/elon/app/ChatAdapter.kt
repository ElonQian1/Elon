package com.elon.app

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView

data class ChatMessage(val role: String, val content: String)

class ChatAdapter(private val messages: MutableList<ChatMessage>) :
    RecyclerView.Adapter<ChatAdapter.VH>() {

    inner class VH(view: View) : RecyclerView.ViewHolder(view) {
        val text: TextView = view.findViewById(R.id.messageText)
    }

    override fun getItemViewType(position: Int): Int = when (messages[position].role) {
        "user"        -> 0
        "ai"          -> 1
        "ai-progress" -> 2
        "ai-tool"     -> 2
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
        holder.text.text = messages[position].content
    }

    override fun getItemCount() = messages.size

    fun addMessage(msg: ChatMessage) {
        // 进度消息：如果上一条也是进度，覆盖它
        if (msg.role in listOf("ai-progress", "ai-tool") &&
            messages.isNotEmpty() &&
            messages.last().role in listOf("ai-progress", "ai-tool")
        ) {
            messages[messages.size - 1] = msg
            notifyItemChanged(messages.size - 1)
        } else {
            messages.add(msg)
            notifyItemInserted(messages.size - 1)
        }
    }
}
