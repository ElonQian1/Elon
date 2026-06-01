package com.elon.app

import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageButton
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainSendButtonVisualActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val attachmentButton: () -> ImageButton?,
    private val inputRightControls: () -> FrameLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val isVoiceMode: () -> Boolean,
    private val hasPendingAttachments: () -> Boolean,
    private val inputCanSend: () -> Boolean,
    private val activeConversation: () -> AppConversation,
    private val isFriendChatActive: () -> Boolean
) {
    fun updateSendButtonVisual() {
        val hasText = binding.inputEdit.text.toString().trim().isNotEmpty()
        val hasAttachments = hasPendingAttachments()
        val composerExpanded = inputComposerMotion()?.isExpanded == true
        val sendMode = (hasText || hasAttachments) && !isVoiceMode() && (composerExpanded || hasAttachments)
        val params = binding.sendButton.layoutParams as? FrameLayout.LayoutParams
        if (sendMode) {
            params?.width = dp(42)
            binding.sendButton.background = activity.getDrawable(R.drawable.ic_input_send_arrow_circle)
            binding.sendButton.text = ""
            binding.sendButton.visibility = View.VISIBLE
            attachmentButton()?.visibility = View.VISIBLE
        } else {
            binding.sendButton.visibility = View.GONE
            attachmentButton()?.visibility = View.VISIBLE
        }
        params?.height = dp(42)
        params?.gravity = Gravity.END or Gravity.CENTER_VERTICAL
        params?.let { binding.sendButton.layoutParams = it }

        attachmentButton()?.let { button ->
            val attachmentParams = button.layoutParams as? FrameLayout.LayoutParams
            attachmentParams?.gravity = if (sendMode) {
                Gravity.START or Gravity.CENTER_VERTICAL
            } else {
                Gravity.END or Gravity.CENTER_VERTICAL
            }
            attachmentParams?.let { button.layoutParams = it }
        }

        inputRightControls()?.let { controls ->
            val controlsParams = controls.layoutParams
            val targetWidth = dp(if (sendMode) 84 else 42)
            if (controlsParams.width != targetWidth) {
                controlsParams.width = targetWidth
                controls.layoutParams = controlsParams
            }
        }

        val conversationEnded = !isFriendChatActive() && activeConversation().ended
        val sendEnabled = !conversationEnded && (!sendMode || inputCanSend())
        binding.sendButton.isEnabled = sendEnabled
        binding.sendButton.alpha = if (sendEnabled) 1f else 0.55f
        attachmentButton()?.let { button ->
            button.isEnabled = !conversationEnded
            button.alpha = if (conversationEnded) 0.55f else 1f
        }
    }
}
