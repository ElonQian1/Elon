package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class WebChatProviderPicker(
    private val activity: AppCompatActivity,
    private val currentProvider: () -> WebChatProviderId,
    private val currentModel: () -> String,
    private val selectProvider: (WebChatProviderId) -> Boolean,
    private val requestModelOptions: () -> Unit,
    private val openOfficialFallback: () -> Unit,
) {
    fun show() {
        val selectedProvider = currentProvider()
        val options = webChatProviderPickerOptions(
            providers = WebChatProviderRegistry.available(),
            selectedProvider = selectedProvider,
            currentModel = currentModel(),
        )
        val selectedIndex = options.indexOfFirst(WebChatProviderPickerOption::selected).coerceAtLeast(0)
        val secondaryAction = if (selectedProvider == WebChatProviderId.CHATGPT_WEB) {
            activity.getString(R.string.web_chat_provider_model_action)
        } else {
            activity.getString(R.string.web_chat_open_official)
        }
        AlertDialog.Builder(activity)
            .setTitle(R.string.web_chat_provider_picker_title)
            .setSingleChoiceItems(options.map { it.label }.toTypedArray(), selectedIndex) { dialog, which ->
                options.getOrNull(which)?.let { option ->
                    if (!option.selected) selectProvider(option.providerId)
                }
                dialog.dismiss()
            }
            .setNeutralButton(secondaryAction, null)
            .setNegativeButton(android.R.string.cancel, null)
            .create()
            .also { dialog ->
                dialog.setOnShowListener {
                    dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                        dialog.dismiss()
                        if (selectedProvider == WebChatProviderId.CHATGPT_WEB) {
                            requestModelOptions()
                        } else {
                            openOfficialFallback()
                        }
                    }
                }
                dialog.show()
            }
    }
}

internal data class WebChatProviderPickerOption(
    val providerId: WebChatProviderId,
    val label: String,
    val selected: Boolean,
)

internal fun webChatProviderPickerOptions(
    providers: List<WebChatProviderIdentity>,
    selectedProvider: WebChatProviderId,
    currentModel: String,
): List<WebChatProviderPickerOption> = providers.map { provider ->
    val selected = provider.id == selectedProvider
    val model = currentModel.trim().takeIf { selected && it.isNotBlank() }
    WebChatProviderPickerOption(
        providerId = provider.id,
        label = listOfNotNull(provider.displayName, model).joinToString(" · "),
        selected = selected,
    )
}
