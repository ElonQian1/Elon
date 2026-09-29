package com.elon.app.socialquotes

import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.Toast
import com.elon.app.AuthManager
import com.elon.app.ChatMessage
import com.elon.app.databinding.ActivityMainBinding

internal class SocialQuoteComposer(private val binding: ActivityMainBinding) {
    private val pending = linkedMapOf<String, SocialQuote>()
    private var scope: String? = null
    private var owner = AuthManager.userId(binding.root.context)
    private var view: SocialQuotePreview? = null

    fun open(key: String) {
        val next = AuthManager.userId(binding.root.context)
        if (next != owner) { pending.clear(); owner = next }
        close(); scope = key; render()
    }
    fun close() { view?.let { (it.parent as? ViewGroup)?.removeView(it) }; view = null; scope = null }
    fun select(message: ChatMessage): Boolean {
        val key = scope ?: return false
        if (message.id.isNullOrBlank() || !message.recalledAt.isNullOrBlank()) {
            Toast.makeText(binding.root.context, "原消息尚未同步或已撤回", Toast.LENGTH_SHORT).show(); return true
        }
        pending[key] = SocialQuoteCodec.from(message, "我")
        while (pending.size > 30) pending.remove(pending.keys.first())
        render(); binding.inputEdit.performClick(); return true
    }
    fun current(): SocialQuote? = pending[scope]
    fun sent(key: String, captured: SocialQuote?) {
        if (pending[key] === captured) { pending.remove(key); if (scope == key) render() }
    }
    private fun render() {
        view?.let { (it.parent as? ViewGroup)?.removeView(it) }; view = null
        val quote = current() ?: return
        val expanded = binding.inputEdit.parent as? ViewGroup ?: return
        val panel = expanded.parent as? LinearLayout ?: return
        val preview = SocialQuotePreview(binding.root.context)
        preview.bind(quote, cancel = { scope?.let(pending::remove); render() })
        panel.addView(preview, (panel.indexOfChild(expanded) + 2).coerceAtMost(panel.childCount), LinearLayout.LayoutParams(-1, -2))
        view = preview
    }
}
