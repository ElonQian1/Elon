package com.elon.app.chatgptweb

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView
import android.text.method.LinkMovementMethod
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.R
import io.noties.markwon.Markwon
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin

internal class ChatGptNativeMessageAdapter(
    parent: View,
    private val onCopy: (ChatGptWebMessage) -> Unit,
    private val onRegenerate: () -> Unit,
    private val onInvokeControl: (String) -> Unit,
    onOpenOfficial: () -> Unit,
) : RecyclerView.Adapter<ChatGptNativeMessageAdapter.MessageViewHolder>() {
    private val markwon = Markwon.builder(parent.context)
        .usePlugin(StrikethroughPlugin.create())
        .usePlugin(TablePlugin.create(parent.context))
        .build()
    private val partRenderer = ChatGptNativeMessagePartRenderer(onOpenOfficial)
    private var messages: List<ChatGptWebMessage> = emptyList()
    private var capabilities = ChatGptWebCapabilities.EMPTY
    private var messageControls: Map<String, List<ChatGptWebUiControl>> = emptyMap()

    fun submit(
        nextMessages: List<ChatGptWebMessage>,
        nextCapabilities: ChatGptWebCapabilities,
    ) {
        val previous = messages
        val diff = DiffUtil.calculateDiff(MessageDiff(previous, nextMessages))
        messages = nextMessages
        capabilities = nextCapabilities
        diff.dispatchUpdatesTo(this)
    }

    fun submitUiControls(controls: List<ChatGptWebUiControl>) {
        messageControls = ChatGptNativeControlPresentation.messageActions(controls)
        if (messages.isNotEmpty()) notifyItemRangeChanged(0, messages.size, PAYLOAD_ACTIONS)
    }

    override fun getItemViewType(position: Int): Int =
        if (messages[position].role == "user") VIEW_TYPE_USER else VIEW_TYPE_ASSISTANT

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): MessageViewHolder {
        val layout = if (viewType == VIEW_TYPE_USER) {
            R.layout.item_chatgpt_message_user
        } else {
            R.layout.item_chatgpt_message_assistant
        }
        return MessageViewHolder(LayoutInflater.from(parent.context).inflate(layout, parent, false))
    }

    override fun onBindViewHolder(holder: MessageViewHolder, position: Int) {
        val message = messages[position]
        val stableId = message.id.replace(Regex("[^A-Za-z0-9_-]"), "_").take(80)
        holder.itemView.contentDescription = "chatgpt-message:$stableId:${message.role}"
        holder.text.contentDescription = "chatgpt-message-content:$stableId"
        if (message.role == "assistant") {
            markwon.setMarkdown(holder.text, message.content)
            holder.text.movementMethod = LinkMovementMethod.getInstance()
        } else {
            holder.text.text = message.content
            holder.text.movementMethod = null
        }
        holder.copy.setOnClickListener { onCopy(message) }
        holder.copy.contentDescription = ChatGptNativeControlPresentation.messageCopySelector(message.id)
        partRenderer.render(holder.parts, message.parts)
        holder.regenerate.visibility = if (canRegenerate(message, position)) View.VISIBLE else View.GONE
        holder.regenerate.setOnClickListener { onRegenerate() }
        holder.regenerate.contentDescription =
            ChatGptNativeControlPresentation.messageRegenerateSelector(message.id)
        val actions = messageControls[messageContextId(message.id)].orEmpty()
        holder.more.visibility = if (actions.isEmpty()) View.GONE else View.VISIBLE
        holder.more.contentDescription =
            ChatGptNativeControlPresentation.messageActionsSelector(message.id, actions.size)
        holder.more.setOnClickListener { showMessageActions(holder, actions) }
        holder.state.visibility = if (message.state in ACTIVE_STATES) View.VISIBLE else View.GONE
        holder.state.setText(
            if (message.state == "pending") {
                R.string.chatgpt_message_sending
            } else {
                R.string.chatgpt_message_streaming
            },
        )
    }

    override fun getItemCount(): Int = messages.size

    private fun canRegenerate(message: ChatGptWebMessage, position: Int): Boolean =
        message.role == "assistant" &&
            message.state == "completed" &&
            position == messages.indexOfLast { it.role == "assistant" } &&
            capabilities.supports(ChatGptWebCapabilityId.MESSAGE_REGENERATE)

    private fun showMessageActions(holder: MessageViewHolder, actions: List<ChatGptWebUiControl>) {
        val enabledActions = actions.filter(ChatGptWebUiControl::enabled).distinctBy(ChatGptWebUiControl::id)
        if (enabledActions.isEmpty()) return
        ChatGptNativeControlDialog.show(
            context = holder.itemView.context,
            title = holder.itemView.context.getString(R.string.chatgpt_message_more_actions),
            controls = enabledActions,
            onSelected = { onInvokeControl(it.id) },
        )
    }

    private fun messageContextId(value: String): String =
        ChatGptNativeControlPresentation.stableContextId(value)

    internal class MessageViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        val text: TextView = itemView.findViewById(R.id.chatGptMessageText)
        val parts: LinearLayout = itemView.findViewById(R.id.chatGptMessageParts)
        val state: TextView = itemView.findViewById(R.id.chatGptMessageState)
        val copy: ImageButton = itemView.findViewById(R.id.chatGptMessageCopy)
        val regenerate: ImageButton = itemView.findViewById(R.id.chatGptMessageRegenerate)
        val more: ImageButton = itemView.findViewById(R.id.chatGptMessageMore)
    }

    private class MessageDiff(
        private val previous: List<ChatGptWebMessage>,
        private val next: List<ChatGptWebMessage>,
    ) : DiffUtil.Callback() {
        override fun getOldListSize(): Int = previous.size

        override fun getNewListSize(): Int = next.size

        override fun areItemsTheSame(oldItemPosition: Int, newItemPosition: Int): Boolean =
            previous[oldItemPosition].id == next[newItemPosition].id

        override fun areContentsTheSame(oldItemPosition: Int, newItemPosition: Int): Boolean =
            previous[oldItemPosition] == next[newItemPosition]
    }

    private companion object {
        const val VIEW_TYPE_USER = 1
        const val VIEW_TYPE_ASSISTANT = 2
        const val PAYLOAD_ACTIONS = "chatgpt_message_actions"
        val ACTIVE_STATES = setOf("pending", "streaming")
    }
}
