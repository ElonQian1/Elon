package com.elon.app.chatgptweb

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.R

internal class ChatGptNativeConversationController(
    private val messagesView: RecyclerView,
    private val emptyView: TextView,
    private val composer: EditText,
    private val sendButton: ImageButton,
    private val stopButton: ImageButton,
    private val newConversationButton: ImageButton,
    private val onSend: (String, String) -> Unit,
    onStop: () -> Unit,
    onNewConversation: () -> Unit,
    onRegenerate: () -> Unit,
    onOpenOfficialOutput: () -> Unit,
) {
    private var messages: List<ChatGptWebMessage> = emptyList()
    private val adapter = ChatGptNativeMessageAdapter(
        parent = messagesView,
        onCopy = ::copyMessage,
        onRegenerate = onRegenerate,
        onOpenOfficial = onOpenOfficialOutput,
    )
    private var snapshot: ChatGptWebSnapshot? = null
    private var pendingPrompt: String? = null

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
        val pending = pendingPrompt
        if (pending != null && value.messages.any { it.role == "user" && it.content == pending }) {
            pendingPrompt = null
        }
        val nextMessages = value.messages.toMutableList()
        pendingPrompt?.let { prompt ->
            nextMessages += ChatGptWebMessage(
                id = PENDING_MESSAGE_ID,
                role = "user",
                content = prompt,
                state = "pending",
                parts = emptyList(),
            )
        }
        messages = nextMessages
        adapter.submit(nextMessages, value.capabilities)
        emptyView.visibility = if (messages.isEmpty()) View.VISIBLE else View.GONE
        if (!composer.hasFocus() && pendingPrompt == null && composer.text.toString() != value.draft) {
            composer.setText(value.draft)
            composer.setSelection(composer.text?.length ?: 0)
        }
        setControls(value)
        if (messages.isNotEmpty()) messagesView.scrollToPosition(messages.lastIndex)
    }

    fun onCommandResult(event: ChatGptWebEvent.CommandResult) {
        when (event.action) {
            "send_prompt" -> {
                if (event.ok) {
                    composer.text?.clear()
                } else {
                    pendingPrompt?.let(composer::setText)
                    composer.setSelection(composer.text?.length ?: 0)
                    pendingPrompt = null
                }
                snapshot?.let(::setControls)
            }
            "new_conversation" -> if (event.ok) {
                pendingPrompt = null
                messages = emptyList()
                adapter.submit(emptyList(), snapshot?.capabilities ?: ChatGptWebCapabilities.EMPTY)
                emptyView.visibility = View.VISIBLE
                composer.text?.clear()
            }
        }
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        val value = snapshot
        if (state == ChatGptWebPageAdapter.State.READY && value != null) {
            setControls(value)
        } else {
            setAvailable(false)
        }
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
        pendingPrompt?.let(composer::setText)
        pendingPrompt = null
        snapshot?.let(::setControls)
    }

    private fun submit() {
        val prompt = composer.text.toString().trim()
        val hasAttachments = snapshot?.attachments?.isNotEmpty() == true
        if ((prompt.isEmpty() && !hasAttachments) || !sendButton.isEnabled) return
        pendingPrompt = prompt.takeIf(String::isNotEmpty)
        onSend(prompt, snapshot?.draft.orEmpty())
        setAvailable(false)
    }

    private fun copyMessage(message: ChatGptWebMessage) {
        val clipboard = messagesView.context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText("ChatGPT message", message.content))
        Toast.makeText(messagesView.context, R.string.chatgpt_message_copied, Toast.LENGTH_SHORT).show()
    }

    private fun setControls(value: ChatGptWebSnapshot) {
        val authenticated = value.authenticated && value.composerReady
        setAvailable(authenticated && !value.streaming && pendingPrompt == null)
        stopButton.isEnabled = value.streaming
        stopButton.alpha = if (value.streaming) 1f else DISABLED_ALPHA
        newConversationButton.isEnabled = value.authenticated && !value.streaming
        newConversationButton.alpha = if (newConversationButton.isEnabled) 1f else DISABLED_ALPHA
    }

    private fun setAvailable(available: Boolean) {
        composer.isEnabled = available
        sendButton.isEnabled = available
        sendButton.alpha = if (available) 1f else DISABLED_ALPHA
    }

    private companion object {
        const val PENDING_MESSAGE_ID = "chatgpt-native-pending"
        const val DISABLED_ALPHA = 0.4f
    }
}
