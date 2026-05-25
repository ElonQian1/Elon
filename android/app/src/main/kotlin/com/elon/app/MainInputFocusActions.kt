package com.elon.app

import android.content.Context
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainInputFocusActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val isVoiceMode: () -> Boolean,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val setSuppressInputFocusAnimation: (Boolean) -> Unit,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    fun focusInputComposer() {
        if (activeConversation().ended) return
        if (isVoiceMode()) {
            setVoiceMode(false)
            applyVoiceMode()
        }
        inputComposerMotion()?.let { motion ->
            if (!motion.isExpanded) {
                motion.setExpanded(true, animate = true)
            }
        }
        binding.inputEdit.requestFocus()
        binding.inputEdit.post {
            showKeyboard()
        }
        binding.inputEdit.postDelayed({
            if (!binding.inputEdit.hasFocus()) return@postDelayed
            showKeyboard()
        }, 120L)
    }

    fun collapseInputComposer(animate: Boolean = true) {
        val motion = inputComposerMotion() ?: return
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.inputEdit.windowToken, 0)
        if (binding.inputEdit.hasFocus()) {
            setSuppressInputFocusAnimation(!animate)
            try {
                binding.inputEdit.clearFocus()
            } finally {
                setSuppressInputFocusAnimation(false)
            }
        }
        if (motion.isExpanded) {
            motion.setExpanded(false, animate = animate)
        }
        updateSendButtonVisual()
        updateAdaptiveInputHeight()
    }

    private fun showKeyboard() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.showSoftInput(binding.inputEdit, InputMethodManager.SHOW_IMPLICIT)
    }
}
