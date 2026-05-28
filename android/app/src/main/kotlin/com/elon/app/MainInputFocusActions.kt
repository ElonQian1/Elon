package com.elon.app

import android.content.Context
import android.graphics.Rect
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainInputFocusActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val isFriendChatActive: () -> Boolean,
    private val isVoiceMode: () -> Boolean,
    private val setVoiceMode: (Boolean) -> Unit,
    private val applyVoiceMode: () -> Unit,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val requestKeyboardLift: () -> Unit,
    private val releaseKeyboardLift: () -> Unit,
    private val setSuppressInputFocusAnimation: (Boolean) -> Unit,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    private var keyboardWatcherInstalled = false
    private var lastKeyboardVisible = false
    private var keyboardVisibleSinceFocus = false

    fun focusInputComposer() {
        if (!isFriendChatActive() && activeConversation().ended) return
        keyboardVisibleSinceFocus = false
        installKeyboardCollapseWatcher()
        if (isVoiceMode()) {
            setVoiceMode(false)
            applyVoiceMode()
        }
        inputComposerMotion()?.let { motion ->
            if (!motion.isExpanded) {
                motion.setExpanded(true, animate = true)
            }
        }
        requestKeyboardLift()
        focusAndShowKeyboard()
        binding.inputEdit.post { focusAndShowKeyboard() }
        binding.inputEdit.postDelayed({ focusAndShowKeyboardIfComposerOpen() }, 90L)
        binding.inputEdit.postDelayed({ focusAndShowKeyboardIfComposerOpen() }, 220L)
    }

    fun collapseInputComposerForBack(): Boolean {
        val motion = inputComposerMotion()
        val shouldCollapse = binding.inputEdit.hasFocus() || motion?.isExpanded == true
        if (!shouldCollapse) return false
        collapseInputComposer(animate = true)
        return true
    }

    fun collapseInputComposer(animate: Boolean = true) {
        val motion = inputComposerMotion() ?: return
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(binding.inputEdit.windowToken, 0)
        releaseKeyboardLift()
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

    private fun focusAndShowKeyboard() {
        if (!binding.inputEdit.hasFocus()) {
            binding.inputEdit.requestFocusFromTouch()
            if (!binding.inputEdit.hasFocus()) {
                binding.inputEdit.requestFocus()
            }
        }
        binding.inputEdit.isCursorVisible = true
        binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
        showKeyboard()
    }

    private fun focusAndShowKeyboardIfComposerOpen() {
        if (isVoiceMode()) return
        if (inputComposerMotion()?.isExpanded != true) return
        if (binding.inputEdit.hasFocus() && isKeyboardVisible()) return
        focusAndShowKeyboard()
    }

    private fun showKeyboard() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.showSoftInput(binding.inputEdit, InputMethodManager.SHOW_IMPLICIT)
    }

    private fun installKeyboardCollapseWatcher() {
        if (keyboardWatcherInstalled) return
        keyboardWatcherInstalled = true
        lastKeyboardVisible = isKeyboardVisible()
        binding.root.viewTreeObserver.addOnGlobalLayoutListener {
            val keyboardVisible = isKeyboardVisible()
            if (keyboardVisible) {
                keyboardVisibleSinceFocus = true
            }
            if (
                lastKeyboardVisible &&
                !keyboardVisible &&
                binding.inputEdit.hasFocus() &&
                inputComposerMotion()?.isExpanded == true &&
                keyboardVisibleSinceFocus
            ) {
                keyboardVisibleSinceFocus = false
                collapseInputComposer(animate = true)
            }
            lastKeyboardVisible = keyboardVisible
        }
    }

    private fun isKeyboardVisible(): Boolean {
        val visibleFrame = Rect()
        binding.root.getWindowVisibleDisplayFrame(visibleFrame)
        val rootHeight = binding.root.rootView.height
        val hiddenHeight = rootHeight - visibleFrame.bottom
        val threshold = (binding.root.resources.displayMetrics.density * 120f).toInt()
        return hiddenHeight > threshold
    }
}
