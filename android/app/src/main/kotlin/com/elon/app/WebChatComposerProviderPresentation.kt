package com.elon.app

import android.widget.TextView
import androidx.core.content.ContextCompat

internal object WebChatComposerProviderPresentation {
    fun apply(
        button: TextView,
        provider: WebChatProviderIdentity,
        rawModelLabel: String,
    ) {
        val modelLabel = rawModelLabel.trim().ifBlank { provider.displayName }
        val iconSize = (18 * button.resources.displayMetrics.density).toInt()
        val icon = ContextCompat.getDrawable(button.context, provider.avatarResId)
            ?.mutate()
            ?.apply { setBounds(0, 0, iconSize, iconSize) }
        button.text = modelLabel
        button.compoundDrawablePadding = (7 * button.resources.displayMetrics.density).toInt()
        button.setCompoundDrawablesRelative(icon, null, null, null)
        button.contentDescription = description(provider, modelLabel)
        (button.parent as? android.view.View)?.contentDescription = button.contentDescription
    }

    fun clear(button: TextView) {
        button.compoundDrawablePadding = 0
        button.setCompoundDrawablesRelative(null, null, null, null)
    }

    fun description(provider: WebChatProviderIdentity, modelLabel: String): String =
        "聊天模式；提供方：${provider.displayName}；模型：${modelLabel.trim()}"
}
