package com.elon.app

import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.TextView
import com.elon.app.databinding.ActivityMainBinding

internal class MainVoiceModeActions(
    private val binding: ActivityMainBinding,
    private val hideKeyboard: () -> Unit,
    private val inputModeButton: () -> ImageButton?,
    private val voiceHoldButton: () -> TextView?,
    private val inputCenterContainer: () -> FrameLayout?,
    private val expandedInputContainer: () -> FrameLayout?,
    private val collapsedInputPreview: () -> TextView?,
    private val modelButtonShell: () -> FrameLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val isVoiceMode: () -> Boolean,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    fun applyVoiceMode() {
        val modeButton = inputModeButton() ?: return
        val voiceButton = voiceHoldButton() ?: return
        val centerContainer = inputCenterContainer() ?: return
        val expandedContainer = expandedInputContainer() ?: return
        val collapsedPreview = collapsedInputPreview() ?: return
        if (isVoiceMode()) {
            hideKeyboard()
            modeButton.setImageResource(R.drawable.ic_input_keyboard_circle)
            inputComposerMotion()?.setExpanded(false, animate = true)
            voiceButton.detachFromParent()
            centerContainer.removeAllViews()
            centerContainer.addView(voiceButton)
            binding.inputEdit.visibility = View.GONE
            modelButtonShell()?.visibility = View.GONE
            voiceButton.visibility = View.VISIBLE
        } else {
            modeButton.setImageResource(R.drawable.ic_input_voice_circle)
            collapsedPreview.detachFromParent()
            voiceButton.detachFromParent()
            centerContainer.removeAllViews()
            centerContainer.addView(collapsedPreview)
            expandedContainer.addView(voiceButton)
            binding.inputEdit.visibility = View.VISIBLE
            modelButtonShell()?.visibility = if (inputComposerMotion()?.isExpanded == true) {
                View.VISIBLE
            } else {
                View.GONE
            }
            voiceButton.visibility = View.GONE
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun View.detachFromParent() {
        (parent as? ViewGroup)?.removeView(this)
    }
}
