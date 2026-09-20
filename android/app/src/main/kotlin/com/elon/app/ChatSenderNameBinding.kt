package com.elon.app

import android.view.Gravity
import android.view.View
import android.widget.TextView

internal const val ELON_CHAT_SENDER_NAME = "一龙ai EL"

internal fun chatSenderName(message: ChatMessage, ownName: String): String? {
    fun String?.cleanName() = this?.trim()?.takeUnless { it.isEmpty() || it.equals("null", true) }
    return when {
        message.senderUserId == "usr_elon_ai" -> ELON_CHAT_SENDER_NAME
        message.role == "user" -> message.senderLabel.cleanName() ?: ownName.cleanName() ?: "我"
        message.role == "friend" -> message.senderLabel.cleanName() ?: "群成员"
        message.role == "ai" || message.role == "ai-intent" -> message.senderLabel.cleanName() ?: ELON_CHAT_SENDER_NAME
        else -> null
    }
}

internal fun bindChatSenderName(view: View, message: ChatMessage) {
    val label = view.findViewById<TextView>(R.id.messageSenderName) ?: return
    val ownName = if (message.role == "user" && message.senderLabel.isNullOrBlank()) {
        UserProfileStore.load(view.context).displayName
    } else ""
    val name = chatSenderName(message, ownName)
    label.text = name.orEmpty()
    label.visibility = if (name == null) View.GONE else View.VISIBLE
    label.gravity = if (message.role == "user") Gravity.END else Gravity.START
}
