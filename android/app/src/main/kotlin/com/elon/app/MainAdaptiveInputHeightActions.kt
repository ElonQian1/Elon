package com.elon.app

import android.view.Gravity
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
            val collapsedHeight = dp(40)
            val minTextHeight = dp(46)
            val maxTextHeight = dp(112)
            val rawLineCount = inputEdit.lineCount.coerceAtLeast(1)
            val voiceMode = isVoiceMode()
            val desiredTextHeight = if (voiceMode) {
                0
            } else {
                val multilineTopGuard = if (rawLineCount > 1) dp(8) else 0
                (rawLineCount.coerceAtMost(4) * inputEdit.lineHeight +
                    inputEdit.paddingTop +
                    inputEdit.paddingBottom +
                    multilineTopGuard).coerceIn(minTextHeight, maxTextHeight)
            }

            val centerParams = centerContainer.layoutParams as LinearLayout.LayoutParams
            if (centerParams.height != collapsedHeight) {
                centerParams.height = collapsedHeight
                centerContainer.layoutParams = centerParams
            }

            inputComposerMotion()?.let { motion ->
                motion.updateExpandedTextHeight(
                    desiredTextHeight,
                    animate = motion.isExpanded
                )
            }

            val multiline = !voiceMode && rawLineCount > 1
            inputEdit.gravity = (if (multiline) Gravity.TOP else Gravity.CENTER_VERTICAL) or Gravity.START
            inputEdit.isVerticalScrollBarEnabled = !voiceMode && rawLineCount > 4
        }
    }
}
