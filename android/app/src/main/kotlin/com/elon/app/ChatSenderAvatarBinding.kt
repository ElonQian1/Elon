package com.elon.app

import android.widget.TextView
import androidx.core.graphics.drawable.RoundedBitmapDrawableFactory

internal fun bindSenderAvatar(avatar: TextView?, message: ChatMessage) {
    avatar ?: return
    val label = message.senderLabel?.trim().orEmpty()
    val avatarResId = message.senderAvatarResId
    if (avatarResId != null && avatarResId != 0) {
        avatar.setBackgroundResource(avatarResId)
        avatar.text = ""
        avatar.contentDescription = label.ifBlank { "AI 头像" }
        return
    }
    val bitmap = UserProfileStore.decodeAvatar(message.senderAvatarDataUrl)
    if (bitmap != null) {
        val radius = (6 * avatar.resources.displayMetrics.density + 0.5f).toInt()
        avatar.background = RoundedBitmapDrawableFactory.create(avatar.resources, bitmap).apply {
            cornerRadius = radius.toFloat()
            setAntiAlias(true)
        }
        avatar.text = ""
    } else {
        avatar.setBackgroundResource(R.drawable.bg_mock_avatar)
        avatar.text = UserProfileStore.avatarInitial(label.ifBlank { "好友" })
    }
    avatar.contentDescription = label.ifBlank { "好友头像" }
}
