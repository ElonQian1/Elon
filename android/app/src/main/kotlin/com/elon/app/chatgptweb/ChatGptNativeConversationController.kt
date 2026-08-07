package com.elon.app.chatgptweb

import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.ImageButton
import android.widget.TextView
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.ChatAdapter
import com.elon.app.ChatMessage
import com.elon.app.R

internal class ChatGptNativeConversationController(
    private val messagesView: RecyclerView,
    private val emptyView: TextView,
    private val composer: EditText,
    private val sendButton: ImageButton,
    stopButton: ImageButton,
    newConversationButton: ImageButton,
    private val onSend: (String) -> Unit,
    onStop: () -> Unit,
    onNewConversation: () -> Unit,
) {
    private val messages = mutableListOf<ChatMessage>()
    private val adapter = ChatAdapter(messages)
    private var snapshot: ChatGptWebSnapshot? = null

    init {
        messagesView.layoutManager = LinearLayoutManager(messagesView.context)
        messagesView.adapter = adapter
        sendButton.setOnClickListener { submit() }
        stopButton.setOnClickListener { onStop() }
        newConversationButton.setOnClickListener { onNewConversation() }
        composer.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                submit()
                true
            } else {
                false
            }
        }
        setAvailable(false)
    }

    fun render(value: ChatGptWebSnapshot) {
        snapshot = value
        val nextMessages = value.messages.map { message ->
            ChatMessage(
                id = message.id,
                role = if (message.role == "user") "user" else "ai",
                content = message.content,
            )
        }
        val sameConversationShape = messages.size == nextMessages.size &&
            messages.indices.all { index -> messages[index].id == nextMessages[index].id }
        if (sameConversationShape) {
            nextMessages.forEachIndexed { index, next ->
                if (messages[index].content != next.content || messages[index].role != next.role) {
                    messages[index] = next
                    adapter.notifyItemChanged(index)
                }
            }
        } else {
            messages.clear()
            messages += nextMessages
            adapter.notifyDataSetChanged()
        }
        emptyView.visibility = if (messages.isEmpty()) View.VISIBLE else View.GONE
        setAvailable(value.composerReady && !value.streaming)
        if (messages.isNotEmpty()) messagesView.scrollToPosition(messages.lastIndex)
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        val available = state == ChatGptWebPageAdapter.State.READY && snapshot?.composerReady == true
        setAvailable(available && snapshot?.streaming != true)
        emptyView.setText(
            when (state) {
                ChatGptWebPageAdapter.State.READY -> R.string.chatgpt_native_empty
                ChatGptWebPageAdapter.State.CONNECTING -> R.string.chatgpt_native_connecting
                ChatGptWebPageAdapter.State.UNSUPPORTED -> R.string.chatgpt_native_unsupported
                ChatGptWebPageAdapter.State.WEB_ONLY -> R.string.chatgpt_native_web_login_required
            },
        )
    }

    fun restoreComposerState() {
        setAvailable(snapshot?.composerReady == true && snapshot?.streaming != true)
    }

    private fun submit() {
        val prompt = composer.text.toString().trim()
        if (prompt.isEmpty() || !sendButton.isEnabled) return
        onSend(prompt)
        composer.text?.clear()
        setAvailable(false)
    }

    private fun setAvailable(available: Boolean) {
        composer.isEnabled = available
        sendButton.isEnabled = available
        sendButton.alpha = if (available) 1f else 0.4f
    }
}
