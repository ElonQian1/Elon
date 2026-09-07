package com.elon.app.chatgptweb

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner

internal class ChatGptNewConversationConfirmationDialog(
    private val activity: AppCompatActivity,
    private val active: () -> Boolean,
    private val canConfirm: () -> Boolean,
    private val resolve: (String, Boolean) -> Boolean,
) : DefaultLifecycleObserver {
    private val handler = Handler(Looper.getMainLooper())
    private val state = ChatGptNewConversationConfirmationState(SystemClock::elapsedRealtime)
    private var pending: String? = null
    private var dialog: AlertDialog? = null
    private val expiry = Runnable {
        dismiss()
        if (active()) Toast.makeText(activity, "确认已过期，请重新点击新会话。", Toast.LENGTH_SHORT).show()
    }

    init { activity.lifecycle.addObserver(this) }

    fun onResult(event: ChatGptWebEvent.CommandResult) {
        val ticket = state.offer(event.action, event.ok, event.detail) ?: return
        pending = ticket
        // The session restores its retained transcript after delivering this receipt.
        // Show the dialog on the next main-loop turn, over that restored conversation.
        handler.post {
            if (pending != ticket) return@post
            if (!visible()) { dismiss(); return@post }
            val created = AlertDialog.Builder(activity)
                .setTitle("新建聊天？")
                .setMessage("当前为未登录聊天。新建后，官网将清除当前未保存的会话。")
                .setPositiveButton("清除并新建") { _, _ -> decide(ticket, true) }
                .setNegativeButton("保留当前聊天") { _, _ -> decide(ticket, false) }
                .setOnCancelListener { decide(ticket, false) }
                .create()
            dialog = created
            created.setOnDismissListener { if (pending == ticket) decide(ticket, false) }
            created.setOnShowListener {
                created.getButton(AlertDialog.BUTTON_POSITIVE).contentDescription = "web_chat_new_conversation_confirm"
                created.getButton(AlertDialog.BUTTON_NEGATIVE).contentDescription = "web_chat_new_conversation_cancel"
            }
            created.show()
            handler.postDelayed(expiry, ChatGptNewConversationConfirmationState.TIMEOUT_MS)
        }
    }

    private fun visible(): Boolean = active() && !activity.isFinishing && !activity.isDestroyed &&
        activity.lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)

    private fun decide(ticket: String, confirmed: Boolean) {
        if (pending != ticket) return
        pending = null
        handler.removeCallbacks(expiry)
        val old = dialog
        dialog = null
        old?.dismiss()
        if (!state.consume(ticket, requireFresh = confirmed)) return
        val allowed = !confirmed || visible() && canConfirm()
        val accepted = resolve(ticket, confirmed && allowed)
        if (confirmed && (!allowed || !accepted) && visible()) {
            Toast.makeText(activity, "当前状态已变化，聊天和草稿已保留，请重新操作。", Toast.LENGTH_LONG).show()
        }
    }

    fun dismiss() { pending?.let { decide(it, false) } }

    override fun onPause(owner: LifecycleOwner) = dismiss()

    override fun onDestroy(owner: LifecycleOwner) {
        dismiss()
        handler.removeCallbacksAndMessages(null)
        activity.lifecycle.removeObserver(this)
    }
}
