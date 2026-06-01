package com.elon.app

import android.content.Context
import android.graphics.Rect
import android.os.SystemClock
import android.view.inputmethod.InputMethodManager
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
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
    private val isEmojiPanelOpen: () -> Boolean,
    private val isEmojiKeyboardOverlayActive: () -> Boolean,
    private val showKeyboardOverEmojiPanel: () -> Unit,
    private val requestKeyboardLift: () -> Unit,
    private val releaseKeyboardLift: () -> Unit,
    private val setSuppressInputFocusAnimation: (Boolean) -> Unit,
    private val collapseEmojiPanel: () -> Unit,
    private val updateSendButtonVisual: () -> Unit,
    private val updateAdaptiveInputHeight: () -> Unit
) {
    private var keyboardWatcherInstalled = false
    private var lastKeyboardVisible = false
    private var keyboardVisibleSinceFocus = false
    private var keyboardDismissCheckPending = false
    private var keyboardVisibleAt = 0L

    fun focusInputComposer() {
        if (!isFriendChatActive() && activeConversation().ended) return
        if (isEmojiPanelOpen()) {
            installKeyboardCollapseWatcher()
            if (isVoiceMode()) {
                setVoiceMode(false)
                applyVoiceMode()
            }
            inputComposerMotion()?.let { motion ->
                if (!motion.isExpanded) {
                    motion.setExpanded(true, animate = false)
                }
            }
            if (!binding.inputEdit.hasFocus()) {
                binding.inputEdit.requestFocusFromTouch()
                if (!binding.inputEdit.hasFocus()) {
                    binding.inputEdit.requestFocus()
                }
            }
            binding.inputEdit.isCursorVisible = true
            showKeyboardOverEmojiPanel()
            return
        }
        if (isEmojiKeyboardOverlayActive()) {
            installKeyboardCollapseWatcher()
            if (!binding.inputEdit.hasFocus()) {
                binding.inputEdit.requestFocus()
            }
            binding.inputEdit.isCursorVisible = true
            return
        }
        collapseEmojiPanel()
        keyboardVisibleSinceFocus = false
        keyboardVisibleAt = 0L
        installKeyboardCollapseWatcher()
        if (isVoiceMode()) {
            setVoiceMode(false)
            applyVoiceMode()
        }
        requestKeyboardLift()
        inputComposerMotion()?.let { motion ->
            if (!motion.isExpanded) {
                motion.prepareKeyboardSynchronizedExpansion()
            }
        }
        focusAndShowKeyboard()
        binding.inputEdit.post {
            focusAndShowKeyboard()
            requestKeyboardLift()
        }
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
        val alreadyFocused = binding.inputEdit.hasFocus()
        if (!alreadyFocused) {
            binding.inputEdit.requestFocusFromTouch()
            if (!binding.inputEdit.hasFocus()) {
                binding.inputEdit.requestFocus()
            }
        }
        binding.inputEdit.isCursorVisible = true
        if (!alreadyFocused) {
            binding.inputEdit.setSelection(binding.inputEdit.text?.length ?: 0)
        }
        showKeyboard()
    }

    private fun focusAndShowKeyboardIfComposerOpen() {
        if (isVoiceMode()) return
        if (isEmojiKeyboardOverlayActive()) return
        if (inputComposerMotion()?.isExpanded != true) return
        if (binding.inputEdit.hasFocus() && isKeyboardVisible()) {
            requestKeyboardLift()
            return
        }
        focusAndShowKeyboard()
        requestKeyboardLift()
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
                if (keyboardVisibleAt == 0L) {
                    keyboardVisibleAt = SystemClock.uptimeMillis()
                }
            }
            if (
                lastKeyboardVisible &&
                !keyboardVisible &&
                binding.inputEdit.hasFocus() &&
                inputComposerMotion()?.isExpanded == true &&
                keyboardVisibleSinceFocus
            ) {
                scheduleKeyboardDismissCheck()
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

    private fun scheduleKeyboardDismissCheck() {
        if (keyboardDismissCheckPending) return
        keyboardDismissCheckPending = true
        val visibleForMs = (SystemClock.uptimeMillis() - keyboardVisibleAt).coerceAtLeast(0L)
        val delayMs = maxOf(KEYBOARD_DISMISS_CONFIRM_MS, KEYBOARD_STABLE_VISIBLE_MS - visibleForMs)
        binding.root.postDelayed({
            keyboardDismissCheckPending = false
            if (!keyboardVisibleSinceFocus) return@postDelayed
            if (isKeyboardVisible() || isImeVisible()) return@postDelayed
            if (isEmojiPanelOpen()) return@postDelayed
            if (!binding.inputEdit.hasFocus()) return@postDelayed
            if (inputComposerMotion()?.isExpanded != true) return@postDelayed
            keyboardVisibleSinceFocus = false
            keyboardVisibleAt = 0L
            collapseInputComposer(animate = true)
        }, delayMs)
    }

    private fun isImeVisible(): Boolean {
        return ViewCompat.getRootWindowInsets(binding.root)
            ?.isVisible(WindowInsetsCompat.Type.ime()) == true
    }

    private companion object {
        private const val KEYBOARD_DISMISS_CONFIRM_MS = 80L
        private const val KEYBOARD_STABLE_VISIBLE_MS = 160L
    }
}
