package com.elon.app

import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.widget.EditText
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import okhttp3.Call
import okhttp3.OkHttpClient

internal data class MentionEdit(val text: String, val cursor: Int)

internal fun insertGroupMentions(text: String, start: Int, end: Int, targets: List<GroupMentionTarget>): MentionEdit {
    val from = start.coerceIn(0, text.length)
    val to = end.coerceIn(from, text.length)
    val prefix = if (from > 0 && !text[from - 1].isWhitespace()) " " else ""
    val value = prefix + targets.distinctBy { it.id }.joinToString("") { "@${it.name} " }
    return MentionEdit(text.replaceRange(from, to, value), from + value.length)
}

internal fun isGroupMentionTrigger(text: CharSequence, start: Int, before: Int, count: Int): Boolean =
    before == 0 && count == 1 && start in text.indices && text[start] in "@＠" &&
        (start == 0 || text[start - 1].isWhitespace() || text[start - 1] in "，。！？：；、,!?;:([{（【")

internal class GroupMentionController(
    private val activity: AppCompatActivity,
    private val input: EditText,
    http: OkHttpClient,
    serverUrl: String,
    private val selfId: () -> String,
    private val focusComposer: () -> Unit,
) : DefaultLifecycleObserver {
    private val directory = GroupMentionDirectory(activity, http, serverUrl)
    private var group: AppGroup? = null
    private var editing = false
    private var picker: GroupMentionPicker? = null
    private var call: Call? = null
    private val watcher = object : TextWatcher {
        override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
        override fun afterTextChanged(s: Editable?) = Unit
        override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
            if (!editing && group != null && s != null && isGroupMentionTrigger(s, start, before, count)) {
                val snapshot = s.toString()
                input.post { if (input.text.toString() == snapshot) showPicker(start, snapshot) }
            }
        }
    }

    init { input.addTextChangedListener(watcher); activity.lifecycle.addObserver(this) }

    fun setGroup(value: AppGroup?) {
        group = value
        picker?.dismiss()
        picker = null
        call?.cancel()
    }

    fun mentionSender(message: ChatMessage) {
        val current = group ?: return
        val id = message.senderUserId?.takeIf { it.isNotBlank() && it != selfId() } ?: return
        val member = current.members.firstOrNull { it.id == id }
        val isAi = id == "usr_elon_ai"
        val name = if (isAi) "EL" else member?.displayName ?: message.senderLabel ?: return
        val from = minOf(input.selectionStart, input.selectionEnd).coerceAtLeast(0)
        val to = maxOf(input.selectionStart, input.selectionEnd).coerceAtLeast(from)
        applyEdit(insertGroupMentions(input.text.toString(), from, to, listOf(GroupMentionTarget(id, name, isAi = isAi))))
    }

    private fun showPicker(at: Int, snapshot: String) {
        val current = group ?: return
        if (picker != null || activity.isFinishing || activity.isDestroyed) return
        fun restoreInput() { if (group?.id == current.id && !activity.isDestroyed) focusInput() }
        val sheet = GroupMentionPicker(activity, onSelected = { selected ->
            // Reject a stale picker after changing conversation or editing the draft elsewhere.
            if (group?.id == current.id && input.text.toString() == snapshot) {
                applyEdit(insertGroupMentions(snapshot, at, at + 1, selected))
            }
        }, onDismissed = { call?.cancel(); picker = null; restoreInput() })
        picker = sheet
        fun load() {
            sheet.showLoading()
            call?.cancel()
            call = directory.load(current.id, selfId()) { result ->
                activity.runOnUiThread {
                    if (picker !== sheet || group?.id != current.id || activity.isDestroyed) return@runOnUiThread
                    result.fold(sheet::showMembers) { sheet.showError { load() } }
                }
            }
        }
        sheet.show()
        load()
    }

    private fun applyEdit(edit: MentionEdit) {
        editing = true
        try { input.setText(edit.text); input.setSelection(edit.cursor) } finally { editing = false }
        focusInput()
    }

    private fun focusInput() {
        val draft = input.text.toString()
        val start = input.selectionStart.coerceAtLeast(0)
        val end = input.selectionEnd.coerceAtLeast(0)
        input.post {
            if (!activity.isDestroyed && group != null) {
                focusComposer()
                if (input.text.toString() == draft) input.setSelection(start, end)
            }
        }
    }

    override fun onDestroy(owner: LifecycleOwner) { setGroup(null); input.removeTextChangedListener(watcher) }
}

internal fun bindGroupMentionAvatar(view: View?, message: ChatMessage, callback: ((ChatMessage) -> Unit)?) {
    view ?: return
    val enabled = callback != null && !message.senderUserId.isNullOrBlank() && message.role != "user" && message.recalledAt == null
    view.setOnLongClickListener(if (enabled) View.OnLongClickListener { callback?.invoke(message); true } else null)
    view.isLongClickable = enabled
    view.contentDescription = if (enabled) "${message.senderLabel ?: "EL"}，长按提及"
        else message.senderLabel ?: view.context.getString(R.string.app_name)
}
