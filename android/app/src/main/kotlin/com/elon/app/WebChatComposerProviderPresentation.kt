package com.elon.app

import android.widget.TextView
import android.view.View
import androidx.core.content.ContextCompat

internal object WebChatComposerProviderPresentation {
    fun restoreWorkControls(
        views: MainInputComposerViews,
        modelButton: TextView,
        modelWidth: Int,
        clearQuickAction: () -> Unit,
        showWorkModelSelector: () -> Unit,
    ) {
        views.activeWebToolChip.render(null, clearQuickAction)
        views.modelButtonShell.tag = null
        views.modelButtonShell.layoutParams = views.modelButtonShell.layoutParams.apply {
            width = modelWidth
        }
        views.planModeButton.visibility = View.VISIBLE
        views.webToolsButton.visibility = View.GONE
        views.webToolsButton.setOnClickListener(null)
        views.attachmentButton.visibility = View.VISIBLE
        views.attachmentButton.contentDescription = WebChatProductionSelectors.WORK_ATTACHMENT
        views.modelButtonShell.setOnClickListener { showWorkModelSelector() }
        modelButton.setOnClickListener { showWorkModelSelector() }
    }

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

    fun applyChatGptModelLevel(
        button: TextView,
        provider: WebChatProviderIdentity,
        rawModelLabel: String,
    ) {
        val label = WebChatModelControlPolicy.compactLabel(rawModelLabel)
        button.text = label
        button.compoundDrawablePadding = 0
        button.setCompoundDrawablesRelative(null, null, null, null)
        button.contentDescription = description(provider, rawModelLabel.trim().ifBlank { label })
        (button.parent as? android.view.View)?.contentDescription = button.contentDescription
    }

    fun description(provider: WebChatProviderIdentity, modelLabel: String): String =
        "聊天模式；提供方：${provider.displayName}；模型：${modelLabel.trim()}"
}
