package com.elon.app

import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding

/** A recoverable empty state; never replace already displayed chat messages with a spinner. */
internal fun showSocialChatStatus(binding: ActivityMainBinding, text: String?, retry: (() -> Unit)? = null) {
    val frame = binding.chatListFrame
    val label = frame.findViewWithTag<TextView>("social-sync-status") ?: TextView(frame.context).also {
        it.tag = "social-sync-status"; it.gravity = Gravity.CENTER; it.textSize = 14f
        it.setPadding(24, 24, 24, 24)
        it.setTextColor(ContextCompat.getColor(frame.context, R.color.elon_text_primary))
        frame.addView(it, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.WRAP_CONTENT, Gravity.CENTER))
    }
    label.text = text.orEmpty()
    label.visibility = if (text == null) View.GONE else View.VISIBLE
    label.setOnClickListener(if (retry == null) null else View.OnClickListener { retry() })
}
