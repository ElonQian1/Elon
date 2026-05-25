package com.elon.app

import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding

internal class MainSendEnabledActions(
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val setInputCanSend: (Boolean) -> Unit,
    private val inputModeButton: () -> ImageButton?,
    private val voiceHoldButton: () -> TextView?,
    private val modelButtonShell: () -> FrameLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val updateSendButtonVisual: () -> Unit,
    private val updateStageHintShimmer: () -> Unit
) {
    fun setSendEnabled(enabled: Boolean) {
        val conversationEnded = activeConversation().ended
        val canSend = enabled && !conversationEnded
        setInputCanSend(canSend)
        binding.inputEdit.isEnabled = !conversationEnded
        binding.inputEdit.hint = if (conversationEnded) {
            "会话已结束，请新建会话继续"
        } else {
            "文本内容在此输入。"
        }
        inputModeButton()?.let { button ->
            button.isEnabled = !conversationEnded
            button.alpha = if (conversationEnded) 0.55f else 1f
        }
        voiceHoldButton()?.let { button ->
            button.isEnabled = !conversationEnded
            button.alpha = if (conversationEnded) 0.55f else 1f
        }
        binding.modelButton.isEnabled = !conversationEnded
        modelButtonShell()?.let { shell ->
            shell.isEnabled = !conversationEnded
            shell.alpha = when {
                conversationEnded -> 0.55f
                inputComposerMotion()?.isExpanded == true -> 1f
                else -> 0f
            }
        }
        updateSendButtonVisual()
        updateStageHintShimmer()
    }
}
