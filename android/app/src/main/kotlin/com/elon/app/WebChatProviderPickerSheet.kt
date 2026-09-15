package com.elon.app

import androidx.appcompat.app.AppCompatActivity

internal object WebChatProviderPickerSheet {
    fun show(activity: AppCompatActivity, options: List<WebChatProviderPickerOption>,
        onProviderSelected: (WebChatProviderId) -> Boolean, onModelOptions: () -> Unit,
        onWebSkin: () -> Unit, onOfficialPage: () -> Unit) {
        val chatGpt = options.firstOrNull { it.selected }?.providerId == WebChatProviderId.CHATGPT_WEB
        ChatAiChoiceSheet.show(activity, activity.getString(R.string.web_chat_provider_picker_title),
            options.map { ChatAiChoice(it.providerId.wireValue, it.title, it.subtitle, it.avatarResId,
                it.selected, "web-chat-provider:${it.providerId.wireValue}:${if (it.selected) "selected" else "idle"}") },
            onSelected = { id -> options.firstOrNull { it.providerId.wireValue == id && !it.selected }
                ?.let { onProviderSelected(it.providerId) } },
            actions = buildList {
                if (chatGpt) {
                    add(ChatAiSheetAction(activity.getString(R.string.web_chat_provider_model_action),
                        "web-chat-provider-model-options", onModelOptions))
                    add(ChatAiSheetAction(activity.getString(R.string.web_chat_open_skin), "web-chat-provider-skin", onWebSkin))
                }
                add(ChatAiSheetAction(activity.getString(R.string.web_chat_open_official), "web-chat-provider-official", onOfficialPage))
            })
    }
}
