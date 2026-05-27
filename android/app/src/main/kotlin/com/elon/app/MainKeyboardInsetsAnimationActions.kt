package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.view.View
import android.view.animation.PathInterpolator
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsAnimationCompat
import androidx.core.view.WindowInsetsCompat
import com.elon.app.databinding.ActivityMainBinding

internal class MainKeyboardInsetsAnimationActions(
    private val binding: ActivityMainBinding
) {
    private val fallbackInterpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private var bottomOffsetAnimator: ValueAnimator? = null
    private var runningImeAnimation = false
    private var baseChatListPaddingBottom = 0

    fun install() {
        baseChatListPaddingBottom = binding.chatList.paddingBottom

        ViewCompat.setOnApplyWindowInsetsListener(binding.root) { _, insets ->
            if (!runningImeAnimation) {
                val keyboardHeight = keyboardHeight(insets)
                animateBottomBarOffset(keyboardHeight)
                applyChatBottomPadding(keyboardHeight)
            }
            insets
        }

        ViewCompat.setWindowInsetsAnimationCallback(
            binding.root,
            object : WindowInsetsAnimationCompat.Callback(DISPATCH_MODE_CONTINUE_ON_SUBTREE) {
                override fun onPrepare(animation: WindowInsetsAnimationCompat) {
                    if (!animation.includesIme()) return
                    runningImeAnimation = true
                    bottomOffsetAnimator?.cancel()
                    binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_HARDWARE, null)
                }

                override fun onProgress(
                    insets: WindowInsetsCompat,
                    runningAnimations: MutableList<WindowInsetsAnimationCompat>
                ): WindowInsetsCompat {
                    if (runningAnimations.any { it.includesIme() }) {
                        applyBottomBarOffset(keyboardHeight(insets))
                    }
                    return insets
                }

                override fun onEnd(animation: WindowInsetsAnimationCompat) {
                    if (!animation.includesIme()) return
                    runningImeAnimation = false
                    binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_NONE, null)

                    val insets = ViewCompat.getRootWindowInsets(binding.root)
                    val keyboardHeight = insets?.let(::keyboardHeight) ?: 0
                    applyBottomBarOffset(keyboardHeight)
                    applyChatBottomPadding(keyboardHeight)
                }
            }
        )

        binding.root.post {
            ViewCompat.requestApplyInsets(binding.root)
        }
    }

    private fun applyBottomBarOffset(keyboardHeight: Int) {
        bottomOffsetAnimator?.cancel()
        binding.bottomBarContainer.translationY = -keyboardHeight.toFloat()
    }

    private fun animateBottomBarOffset(keyboardHeight: Int) {
        val target = -keyboardHeight.toFloat()
        val start = binding.bottomBarContainer.translationY
        if (kotlin.math.abs(start - target) < 0.5f) return
        bottomOffsetAnimator?.cancel()
        binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_HARDWARE, null)
        bottomOffsetAnimator = ValueAnimator.ofFloat(start, target).apply {
            duration = 220L
            interpolator = fallbackInterpolator
            addUpdateListener { animator ->
                binding.bottomBarContainer.translationY = animator.animatedValue as Float
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_NONE, null)
                }
            })
            start()
        }
    }

    private fun applyChatBottomPadding(keyboardHeight: Int) {
        val targetBottom = baseChatListPaddingBottom + keyboardHeight
        if (binding.chatList.paddingBottom == targetBottom) return
        binding.chatList.setPadding(
            binding.chatList.paddingLeft,
            binding.chatList.paddingTop,
            binding.chatList.paddingRight,
            targetBottom
        )
    }

    private fun keyboardHeight(insets: WindowInsetsCompat): Int {
        val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        val systemBottom = insets.getInsets(WindowInsetsCompat.Type.systemBars()).bottom
        return (imeBottom - systemBottom).coerceAtLeast(0)
    }

    private fun WindowInsetsAnimationCompat.includesIme(): Boolean {
        return typeMask and WindowInsetsCompat.Type.ime() != 0
    }
}
