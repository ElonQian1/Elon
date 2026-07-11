package com.elon.app

import android.view.Gravity
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import com.elon.app.databinding.ActivityMainBinding

internal class MainAdaptiveInputHeightActions(
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val inputCenterContainer: () -> FrameLayout?,
    private val inputBarContainer: () -> LinearLayout?,
    private val inputComposerMotion: () -> InputComposerMotion?,
    private val isVoiceMode: () -> Boolean
) {
    fun updateAdaptiveInputHeight() {
        if (inputCenterContainer() == null || inputBarContainer() == null) return
        val inputEdit = binding.inputEdit
        inputEdit.post {
            val centerContainer = inputCenterContainer() ?: return@post
            if (inputBarContainer() == null) return@post
            val collapsedHeight = dp(38)
            val minTextHeight = dp(42)
            val topSafeInset = dp(12)
            val bottomActionReserve = dp(48)
            val maxVisibleLines = 6
            val maxTextHeight = dp(140)
            val rawLineCount = inputEdit.lineCount.coerceAtLeast(1)
            val voiceMode = isVoiceMode()
            val desiredTextHeight = if (voiceMode) {
                0
            } else {
                (rawLineCount.coerceAtMost(maxVisibleLines) * inputEdit.lineHeight +
                    inputEdit.paddingTop +
                    inputEdit.paddingBottom).coerceIn(minTextHeight, maxTextHeight)
            }
            val anchorTopOffset = if (voiceMode) 0 else topSafeInset
            val desiredExpandedHeight = if (voiceMode) {
                0
            } else {
                desiredTextHeight + topSafeInset + bottomActionReserve
            }

            val centerParams = centerContainer.layoutParams as LinearLayout.LayoutParams
            if (centerParams.height != collapsedHeight) {
                centerParams.height = collapsedHeight
                centerContainer.layoutParams = centerParams
            }
            applyInputBounds(inputEdit, anchorTopOffset, desiredTextHeight)

            inputComposerMotion()?.let { motion ->
                motion.updateExpandedTextHeight(
                    desiredExpandedHeight,
                    animate = motion.isExpanded
                )
            }

            inputEdit.gravity = Gravity.TOP or Gravity.START
            inputEdit.isVerticalScrollBarEnabled = !voiceMode && rawLineCount > maxVisibleLines
        }
    }

    private fun applyInputBounds(inputEdit: EditText, topMargin: Int, height: Int) {
        val params = inputEdit.layoutParams as? FrameLayout.LayoutParams ?: return
        var changed = false
        if (params.topMargin != topMargin) {
            params.topMargin = topMargin
            changed = true
        }
        if (params.height != height) {
            params.height = height
            changed = true
        }
        if (changed) inputEdit.layoutParams = params
    }
}
