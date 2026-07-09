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
            val singleLineAnchorTop = dp(48)
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
            val anchorTopOffset = if (voiceMode) {
                0
            } else {
                val consumedAnchor = (desiredTextHeight - minTextHeight).coerceAtLeast(0)
                (singleLineAnchorTop - consumedAnchor).coerceIn(0, singleLineAnchorTop)
            }
            val desiredExpandedHeight = if (voiceMode) 0 else desiredTextHeight + anchorTopOffset

            val centerParams = centerContainer.layoutParams as LinearLayout.LayoutParams
            if (centerParams.height != collapsedHeight) {
                centerParams.height = collapsedHeight
                centerContainer.layoutParams = centerParams
            }
            applyInputTopMargin(inputEdit, anchorTopOffset)

            inputComposerMotion()?.let { motion ->
                motion.updateExpandedTextHeight(
                    desiredExpandedHeight,
                    animate = motion.isExpanded
                )
            }

            val multiline = !voiceMode && rawLineCount > 1
            inputEdit.gravity = (if (multiline) Gravity.TOP else Gravity.CENTER_VERTICAL) or Gravity.START
            inputEdit.isVerticalScrollBarEnabled = !voiceMode && rawLineCount > maxVisibleLines
        }
    }

    private fun applyInputTopMargin(inputEdit: EditText, topMargin: Int) {
        val params = inputEdit.layoutParams as? FrameLayout.LayoutParams ?: return
        if (params.topMargin == topMargin) return
        params.topMargin = topMargin
        inputEdit.layoutParams = params
    }
}
