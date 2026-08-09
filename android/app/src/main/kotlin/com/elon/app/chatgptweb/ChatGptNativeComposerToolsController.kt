package com.elon.app.chatgptweb

import android.view.View
import android.widget.Button
import android.widget.ImageButton
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R

internal class ChatGptNativeComposerToolsController(
    private val activity: AppCompatActivity,
    private val modelButton: Button,
    private val attachmentButton: ImageButton,
    private val toolsButton: ImageButton,
    private val onChooseAttachments: () -> Unit,
    private val onRequestModelOptions: () -> Unit,
    private val onRequestTools: () -> Unit,
    private val onSelectModelOption: (String) -> Unit,
    private val onSelectTool: (String) -> Unit,
    private val onOpenOfficialModelSelector: () -> Unit,
    private val onOpenOfficialTools: () -> Unit,
) {
    private enum class Section { MODEL, TOOLS }

    private var bridgeReady = false
    private var snapshot: ChatGptWebSnapshot? = null
    private var pendingSection: Section? = null
    private var dialog: AlertDialog? = null

    init {
        modelButton.setOnClickListener {
            pendingSection = Section.MODEL
            onRequestModelOptions()
        }
        attachmentButton.setOnClickListener { onChooseAttachments() }
        toolsButton.setOnClickListener {
            pendingSection = Section.TOOLS
            onRequestTools()
        }
        updateControls()
    }

    fun render(value: ChatGptWebSnapshot) {
        snapshot = value
        modelButton.text = value.currentModel.ifBlank {
            activity.getString(R.string.chatgpt_native_model_default)
        }
        updateControls()
    }

    fun render(event: ChatGptWebEvent.ComposerControls) {
        val section = when (event.section) {
            "model" -> Section.MODEL
            "tools" -> Section.TOOLS
            else -> return
        }
        if (section != pendingSection) return
        if (event.currentModel.isNotBlank()) modelButton.text = event.currentModel
        showOptions(section, event.options)
    }

    fun onCommandResult(event: ChatGptWebEvent.CommandResult) {
        if (!event.ok && event.action in REQUEST_ACTIONS) pendingSection = null
    }

    fun setBridgeState(state: ChatGptWebPageAdapter.State) {
        bridgeReady = state == ChatGptWebPageAdapter.State.READY
        updateControls()
    }

    fun dispose() {
        dialog?.dismiss()
        dialog = null
    }

    private fun showOptions(section: Section, options: List<ChatGptWebComposerOption>) {
        dialog?.dismiss()
        if (options.isEmpty()) {
            openOfficial(section)
            return
        }
        val labels = options.map(ChatGptWebComposerOption::label).toTypedArray()
        val selected = options.indexOfFirst(ChatGptWebComposerOption::selected)
        val builder = AlertDialog.Builder(activity)
            .setTitle(
                if (section == Section.MODEL) {
                    R.string.chatgpt_native_model_title
                } else {
                    R.string.chatgpt_native_tools_title
                },
            )
            .setNegativeButton(R.string.chatgpt_web_cancel, null)
            .setNeutralButton(R.string.chatgpt_native_open_official) { _, _ -> openOfficial(section) }
        if (section == Section.MODEL) {
            builder.setSingleChoiceItems(labels, selected) { currentDialog, index ->
                currentDialog.dismiss()
                pendingSection = null
                onSelectModelOption(options[index].id)
            }
        } else {
            builder.setItems(labels) { _, index ->
                pendingSection = null
                onSelectTool(options[index].id)
            }
        }
        dialog = builder.create().also { it.show() }
    }

    private fun openOfficial(section: Section) {
        pendingSection = null
        if (section == Section.MODEL) onOpenOfficialModelSelector() else onOpenOfficialTools()
    }

    private fun updateControls() {
        val capabilities = snapshot?.capabilities ?: ChatGptWebCapabilities.EMPTY
        updateButton(modelButton, bridgeReady && capabilities.supports(ChatGptWebCapabilityId.MODEL_SELECTOR))
        updateButton(
            attachmentButton,
            bridgeReady && capabilities.supports(ChatGptWebCapabilityId.ATTACHMENTS),
        )
        updateButton(toolsButton, bridgeReady && capabilities.supports(ChatGptWebCapabilityId.COMPOSER_TOOLS))
    }

    private fun updateButton(view: View, enabled: Boolean) {
        view.isEnabled = enabled
        view.alpha = if (enabled) 1f else DISABLED_ALPHA
    }

    private companion object {
        const val DISABLED_ALPHA = 0.4f
        val REQUEST_ACTIONS = setOf("list_model_options", "list_composer_tools")
    }
}
