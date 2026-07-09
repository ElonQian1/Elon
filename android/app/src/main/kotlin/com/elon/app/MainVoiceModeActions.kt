package com.elon.app

import android.content.Context
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainVoiceModeActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val inputModeButton: () -> ImageButton?,
    private val emojiButton: () -> ImageButton?,
    private val voiceHoldButton: () -> TextView?,
    private val inputCenterContainer: () -> FrameLayout?,
    private val expandedInputContainer: () -> FrameLayout?,
    private val collapsedInputPreview: () -> TextView?,
    private val modelButtonShell: () -> FrameLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val isVoiceMode: () -> Boolean,
    private val setVoiceMode: (Boolean) -> Unit,
    private val ttsSpeakerButton: () -> ImageButton?,
    private val isDirectSocialAiChatActive: () -> Boolean = { false },
    private val collapseAttachmentPanel: () -> Unit,
    private val collapseEmojiPanel: () -> Unit,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    fun toggleVoiceMode() {
        setVoiceMode(!isVoiceMode())
        collapseAttachmentPanel()
        collapseEmojiPanel()
        applyVoiceMode()
    }

    fun applyVoiceMode() {
        val modeButton = inputModeButton() ?: return
        val voiceButton = voiceHoldButton() ?: return
        val centerContainer = inputCenterContainer() ?: return
        val expandedContainer = expandedInputContainer() ?: return
        val collapsedPreview = collapsedInputPreview() ?: return
        if (isVoiceMode()) {
            hideKeyboard()
            modeButton.setImageResource(R.drawable.ic_input_keyboard_circle)
            emojiButton()?.visibility = View.GONE
            inputComposerMotion()?.setExpanded(false, animate = true)
            voiceButton.detachFromParent()
            centerContainer.removeAllViews()
            centerContainer.addView(voiceButton)
            binding.inputEdit.visibility = View.GONE
            voiceButton.visibility = View.VISIBLE
            // 普通聊天的语音按钮保持按住说话；一龙AI 实时通话由顶部电话入口处理。
            val speakerBtn = ttsSpeakerButton()
            if (VoiceInputModeSettings.get(activity) == VoiceInputMode.CLOUD_REALTIME &&
                !isDirectSocialAiChatActive()
            ) {
                speakerBtn?.setImageResource(
                    if (VoiceSpeaker.isTtsEnabled(activity)) R.drawable.ic_input_tts_on_circle
                    else R.drawable.ic_input_tts_off_circle
                )
                speakerBtn?.visibility = View.VISIBLE
            } else {
                speakerBtn?.visibility = View.GONE
            }
        } else {
            modeButton.setImageResource(R.drawable.ic_input_voice_wave_new)
            emojiButton()?.visibility = View.VISIBLE
            collapsedPreview.detachFromParent()
            voiceButton.detachFromParent()
            centerContainer.removeAllViews()
            centerContainer.addView(collapsedPreview)
            attachExpandedTextInput(expandedContainer)
            expandedContainer.addView(voiceButton, minOf(1, expandedContainer.childCount))
            binding.inputEdit.visibility = View.VISIBLE
            voiceButton.visibility = View.GONE
            ttsSpeakerButton()?.visibility = View.GONE
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun attachExpandedTextInput(expandedContainer: FrameLayout) {
        if (binding.inputEdit.parent === expandedContainer) return
        binding.inputEdit.detachFromParent()
        expandedContainer.addView(binding.inputEdit, 0)
    }

    private fun hideKeyboard() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.root.windowToken, 0)
        binding.inputEdit.clearFocus()
    }

    private fun View.detachFromParent() {
        (parent as? ViewGroup)?.removeView(this)
    }
}
