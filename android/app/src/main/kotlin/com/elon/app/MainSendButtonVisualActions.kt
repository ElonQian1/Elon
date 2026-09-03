package com.elon.app

import android.view.Gravity
import android.view.View
import android.graphics.drawable.InsetDrawable
import android.widget.FrameLayout
import android.widget.ImageButton
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainSendButtonVisualActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val attachmentButton: () -> ImageButton?,
    private val inputModeButton: () -> ImageButton?,
    private val webDictationButton: () -> ImageButton?,
    private val inputRightControls: () -> FrameLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val isVoiceMode: () -> Boolean,
    private val hasPendingAttachments: () -> Boolean,
    private val inputCanSend: () -> Boolean,
    private val composerCanSubmit: () -> Boolean,
    private val isWebChatStreaming: () -> Boolean,
    private val isWebChatDictationActive: () -> Boolean,
    private val activeConversation: () -> AppConversation,
    private val isFriendChatActive: () -> Boolean
) {
    fun updateSendButtonVisual() {
        val hasText = binding.inputEdit.text.toString().trim().isNotEmpty()
        val hasAttachments = hasPendingAttachments()
        val composerExpanded = inputComposerMotion()?.isExpanded == true
        val streaming = isWebChatStreaming()
        val visualMode = WebChatProductionComposerVisualModeResolver.resolve(
            streaming = streaming,
            hasText = hasText,
            hasAttachments = hasAttachments,
            voiceMode = isVoiceMode(),
            composerExpanded = composerExpanded,
            dictationActive = isWebChatDictationActive(),
        )
        val params = binding.sendButton.layoutParams as? FrameLayout.LayoutParams
        if (visualMode != WebChatProductionComposerVisualMode.INPUT_MODE) {
            params?.width = dp(38)
            val icon = if (visualMode == WebChatProductionComposerVisualMode.STOP) {
                R.drawable.ic_input_stop_new
            } else {
                R.drawable.ic_input_send_new
            }
            activity.getDrawable(icon)?.let {
                binding.sendButton.background = InsetDrawable(it, dp(3))
            }
            binding.sendButton.text = ""
            val webChatComposer = binding.modelButton.tag == WEB_CHAT_MODEL_BUTTON_OWNER
            binding.sendButton.contentDescription = if (webChatComposer) {
                WebChatProductionSelectors.composerAction(
                    streaming = visualMode == WebChatProductionComposerVisualMode.STOP,
                )
            } else if (visualMode == WebChatProductionComposerVisualMode.STOP) {
                WebChatProductionSelectors.STOP_GENERATION
            } else {
                "发送消息"
            }
            binding.sendButton.visibility = View.VISIBLE
            inputModeButton()?.visibility = View.GONE
        } else {
            binding.sendButton.visibility = View.GONE
            inputModeButton()?.let { button ->
                button.visibility = if (button.tag == WEB_CHAT_REALTIME_VOICE_HIDDEN_TAG) {
                    View.GONE
                } else {
                    View.VISIBLE
                }
            }
        }
        webDictationButton()?.let { button ->
            button.visibility = if (
                visualMode == WebChatProductionComposerVisualMode.INPUT_MODE && button.isActivated
            ) {
                View.VISIBLE
            } else {
                View.GONE
            }
        }
        params?.height = dp(38)
        params?.gravity = Gravity.END or Gravity.CENTER_VERTICAL
        params?.let { binding.sendButton.layoutParams = it }

        inputRightControls()?.let { controls ->
            val controlsParams = controls.layoutParams
            // The initial voice action is a 48dp control. Keeping its host at the
            // 38dp send-button width clips the leading edge of the circular icon.
            val targetWidth = dp(
                if (visualMode == WebChatProductionComposerVisualMode.INPUT_MODE) 48 else 38
            )
            if (controlsParams.width != targetWidth) {
                controlsParams.width = targetWidth
                controls.layoutParams = controlsParams
            }
        }

        val conversationEnded = !isFriendChatActive() && activeConversation().ended
        val submissionEnabled = composerCanSubmit()
        val sendEnabled = !conversationEnded && (
            visualMode == WebChatProductionComposerVisualMode.STOP ||
                (inputCanSend() && submissionEnabled)
        )
        binding.sendButton.isEnabled = sendEnabled
        binding.sendButton.alpha = if (sendEnabled) 1f else 0.55f
        attachmentButton()?.let { button ->
            button.isEnabled = !conversationEnded && !streaming && submissionEnabled
            button.alpha = if (button.isEnabled) 1f else 0.55f
        }
        inputModeButton()?.let { button ->
            button.isEnabled = !conversationEnded
            button.alpha = if (conversationEnded) 0.55f else 1f
        }
        webDictationButton()?.let { button ->
            button.isEnabled = !conversationEnded && !streaming
            button.alpha = if (button.isEnabled) 1f else 0.55f
        }
    }
}
