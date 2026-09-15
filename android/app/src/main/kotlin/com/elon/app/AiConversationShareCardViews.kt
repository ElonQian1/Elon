package com.elon.app

import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat

internal object AiConversationShareCardViews {
    fun bind(
        container: LinearLayout?,
        messageText: TextView,
        message: ChatMessage,
        onOpen: ((AiConversationShareCard) -> Unit)?,
        loadCover: ((AiConversationShareCard, (String?) -> Unit) -> Unit)? = null,
        onLongPress: ((View, ChatMessage, AiConversationShareCard) -> Unit)? = null,
    ): Boolean {
        val card = AiConversationShareCodec.parseCard(message.content) ?: return false
        container ?: return false
        messageText.text = ""
        messageText.visibility = View.GONE
        container.removeAllViews()
        container.visibility = View.VISIBLE
        val context = container.context
        fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
        fun label(value: String, size: Float, color: Int, lines: Int) = TextView(context).apply {
            text = value
            textSize = size
            maxLines = lines
            ellipsize = TextUtils.TruncateAt.END
            setTextColor(ContextCompat.getColor(context, color))
            layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(8) }
        }
        val body = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            val available = context.resources.displayMetrics.widthPixels - dp(100)
            layoutParams = LinearLayout.LayoutParams(minOf(dp(288), available.coerceAtLeast(dp(160))), -2)
            minimumHeight = dp(48)
            setPadding(dp(16), dp(12), dp(16), dp(12))
            background = GradientDrawable().apply {
                cornerRadius = dp(16).toFloat()
                setColor(ContextCompat.getColor(context, R.color.elon_surface_card))
                setStroke(dp(1), ContextCompat.getColor(context, R.color.elon_border_subtle))
            }
            isFocusable = onOpen != null
            contentDescription = context.getString(R.string.ai_conversation_share_card_description,
                card.senderName, card.provider, card.title, card.messageCount)
            setOnClickListener(onOpen?.let { callback -> View.OnClickListener { callback(card) } })
            setOnLongClickListener(onLongPress?.let { callback -> View.OnLongClickListener { callback(it, message, card); true } })
        }
        val source = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        source.addView(TextView(context).apply {
            gravity = Gravity.CENTER
            layoutParams = LinearLayout.LayoutParams(dp(28), dp(28)).apply { marginEnd = dp(8) }
            bindSenderAvatar(this, message.copy(senderLabel = card.senderName))
        })
        source.addView(label(context.getString(R.string.ai_conversation_share_source,
            card.senderName, card.provider), 12f, R.color.elon_text_secondary, 2).apply {
            layoutParams = LinearLayout.LayoutParams(0, -2, 1f)
        })
        body.addView(source)
        body.addView(label(card.title, 16f, R.color.elon_text_primary, 2).apply {
            setTypeface(typeface, Typeface.BOLD)
            setPadding(0, dp(10), 0, 0)
        })
        if (card.summary.isNotBlank()) body.addView(label(card.summary, 13f, R.color.elon_text_secondary, 3))
        if (card.coverAssetId != null && loadCover != null) {
            val cover = ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(-1, dp(136)).apply { bottomMargin = dp(8) }
                scaleType = ImageView.ScaleType.FIT_CENTER
                contentDescription = context.getString(R.string.ai_conversation_share_cover)
                visibility = View.GONE
                tag = "${card.groupId}:${card.id}:${card.coverAssetId}"
            }
            body.addView(cover)
            val expected = cover.tag
            val expectedAccount = AuthManager.effectiveUserId(context)
            val expectedRevision = AuthManager.prefs(context).getString("auth_session_revision", "")
            loadCover(card) { path ->
                val local = AiConversationShareReaderPresentation.localImageSource(path)
                if (local != null) ChatImagePreviewLoader.load(context, local) { bitmap ->
                    cover.post {
                        if (cover.tag == expected && cover.parent === body &&
                            AuthManager.effectiveUserId(context) == expectedAccount &&
                            AuthManager.prefs(context).getString("auth_session_revision", "") == expectedRevision) {
                            cover.setImageBitmap(bitmap)
                            cover.visibility = View.VISIBLE
                        }
                    }
                }
            }
        }
        body.addView(label(context.getString(R.string.ai_conversation_share_message_count, card.messageCount),
            12f, R.color.elon_text_tertiary, 1).apply { setCompoundDrawablesWithIntrinsicBounds(
                0, 0, R.drawable.ic_project_space_chevron_right, 0) })
        container.addView(body)
        return true
    }
}
