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
    private val onRequestModelOptions: () -> Unit,
    private val onRequestTools: () -> Unit,
    private val onSelectModelOption: (String) -> Unit,
    private val onSelectTool: (String) -> Unit,
    private val onDismissMenu: () -> Unit,
    private val onOpenOfficialModelSelector: () -> Unit,
    private val onOpenOfficialTools: () -> Unit,
) {
    private enum class Section(val wireName: String) {
        MODEL("model"),
        TOOLS("tools"),
        ATTACHMENTS("attachments"),
    }

    private var bridgeReady = false
    private var snapshot: ChatGptWebSnapshot? = null
    private var pendingSection: Section? = null
    private var dialog: AlertDialog? = null

    init {
        modelButton.setOnClickListener {
            pendingSection = Section.MODEL
            onRequestModelOptions()
        }
        attachmentButton.setOnClickListener {
            pendingSection = Section.ATTACHMENTS
            onRequestTools()
        }
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
        val eventSection = when (event.section) {
            "model" -> Section.MODEL
            "tools" -> Section.TOOLS
            else -> return
        }
        val section = when {
            eventSection == Section.TOOLS && pendingSection == Section.ATTACHMENTS -> Section.ATTACHMENTS
            eventSection == pendingSection -> eventSection
            else -> return
        }
        if (event.currentModel.isNotBlank()) modelButton.text = event.currentModel
        val options = if (section == Section.ATTACHMENTS) {
            event.options.filter(::isAttachmentOption)
        } else {
            event.options
        }
        showOptions(section, options)
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
        val title = activity.getString(
            if (section == Section.MODEL) {
                R.string.chatgpt_native_model_title
            } else if (section == Section.ATTACHMENTS) {
                R.string.chatgpt_native_attachment
            } else {
                R.string.chatgpt_native_tools_title
            },
        )
        dialog = ChatGptNativeComposerOptionDialog.show(
            context = activity,
            title = title,
            section = section.wireName,
            options = options,
            singleChoice = section == Section.MODEL,
            cancelLabel = R.string.chatgpt_web_cancel,
            officialLabel = R.string.chatgpt_native_open_official,
            onSelected = { option ->
                pendingSection = null
                if (section == Section.MODEL) {
                    onSelectModelOption(option.id)
                } else {
                    onSelectTool(option.id)
                }
            },
            onCancelled = {
                pendingSection = null
                onDismissMenu()
            },
            onOpenOfficial = { openOfficial(section) },
        )
    }

    private fun openOfficial(section: Section) {
        pendingSection = null
        if (section == Section.MODEL) onOpenOfficialModelSelector() else onOpenOfficialTools()
    }

    private fun isAttachmentOption(option: ChatGptWebComposerOption): Boolean {
        val label = option.label.trim().lowercase()
        return label in ATTACHMENT_LABELS || label.startsWith("upload ")
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
        val REQUEST_ACTIONS = setOf(
            "list_model_options",
            "list_composer_tools",
            "collect_model_options",
            "collect_composer_tools",
        )
        val ATTACHMENT_LABELS = setOf(
            "相机",
            "照片",
            "文件",
            "camera",
            "photo",
            "photos",
            "file",
            "files",
        )
    }
}
