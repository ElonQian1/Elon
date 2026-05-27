package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.graphics.Rect
import android.view.View
import android.view.ViewTreeObserver
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
    private var baselineHiddenHeight = -1
    private var lastKeyboardHeight = -1
    private var usingEstimatedKeyboardHeight = false
    private val visibleFrame = Rect()

    fun install() {
        baseChatListPaddingBottom = binding.chatList.paddingBottom
        installVisibleFrameFallback()
        installFocusFallback()

        ViewCompat.setOnApplyWindowInsetsListener(binding.root) { _, insets ->
            if (!runningImeAnimation) {
                val keyboardHeight = keyboardHeightFromInsetsOrFrame(insets)
                if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight)) {
                    applyKeyboardHeight(keyboardHeight, animate = true)
                }
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
                        val keyboardHeight = keyboardHeightFromInsetsOrFrame(insets)
                        if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight)) {
                            applyKeyboardHeight(keyboardHeight, animate = false)
                        }
                    }
                    return insets
                }

                override fun onEnd(animation: WindowInsetsAnimationCompat) {
                    if (!animation.includesIme()) return
                    runningImeAnimation = false
                    binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_NONE, null)

                    val insets = ViewCompat.getRootWindowInsets(binding.root)
                    val keyboardHeight = insets?.let(::keyboardHeightFromInsetsOrFrame) ?: keyboardHeightFromVisibleFrame()
                    if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight)) {
                        applyKeyboardHeight(keyboardHeight, animate = false)
                    }
                }
            }
        )

        binding.root.post {
            ViewCompat.requestApplyInsets(binding.root)
        }
    }

    fun requestKeyboardLift() {
        binding.bottomBarContainer.bringToFront()
        applyKnownOrEstimatedKeyboardHeight()
        binding.root.post {
            applyKnownOrEstimatedKeyboardHeight()
        }
        binding.root.postDelayed({ applyKnownOrEstimatedKeyboardHeight() }, 90L)
        binding.root.postDelayed({ applyKnownOrEstimatedKeyboardHeight() }, 220L)
    }

    fun releaseKeyboardLift() {
        usingEstimatedKeyboardHeight = false
        applyKeyboardHeight(0, animate = true)
    }

    private fun installVisibleFrameFallback() {
        binding.root.viewTreeObserver.addOnGlobalLayoutListener(object : ViewTreeObserver.OnGlobalLayoutListener {
            override fun onGlobalLayout() {
                val keyboardHeight = keyboardHeightFromVisibleFrame()
                if (shouldIgnoreZeroKeyboardHeight(keyboardHeight)) return
                if (keyboardHeight == lastKeyboardHeight) return
                applyKeyboardHeight(keyboardHeight, animate = !runningImeAnimation)
            }
        })
    }

    private fun installFocusFallback() {
        binding.root.viewTreeObserver.addOnGlobalFocusChangeListener { _, newFocus ->
            if (newFocus == binding.inputEdit) {
                scheduleEstimatedKeyboardLift()
            } else if (!binding.inputEdit.hasFocus() && usingEstimatedKeyboardHeight) {
                usingEstimatedKeyboardHeight = false
                applyKeyboardHeight(0, animate = true)
            }
        }
    }

    private fun scheduleEstimatedKeyboardLift() {
        binding.root.postDelayed({ applyEstimatedKeyboardLiftIfNeeded() }, 80L)
        binding.root.postDelayed({ applyEstimatedKeyboardLiftIfNeeded() }, 180L)
    }

    private fun applyEstimatedKeyboardLiftIfNeeded() {
        if (!binding.inputEdit.hasFocus()) return
        if (keyboardHeightFromVisibleFrame() > 0) return
        val insetsHeight = ViewCompat.getRootWindowInsets(binding.root)?.let(::keyboardHeight).orEmpty()
        if (insetsHeight > 0) return

        usingEstimatedKeyboardHeight = true
        applyKeyboardHeight(estimatedKeyboardHeight(), animate = true)
    }

    private fun applyKnownOrEstimatedKeyboardHeight() {
        if (!binding.inputEdit.hasFocus()) return
        val knownHeight = ViewCompat.getRootWindowInsets(binding.root)
            ?.let(::keyboardHeightFromInsetsOrFrame)
            ?: keyboardHeightFromVisibleFrame()
        if (knownHeight > 0) {
            applyKeyboardHeight(knownHeight, animate = true)
            return
        }
        usingEstimatedKeyboardHeight = true
        applyKeyboardHeight(estimatedKeyboardHeight(), animate = true)
    }

    private fun applyKeyboardHeight(keyboardHeight: Int, animate: Boolean) {
        val target = keyboardHeight.coerceAtLeast(0)
        if (target > 0 && target != estimatedKeyboardHeight()) {
            usingEstimatedKeyboardHeight = false
        }
        if (target == 0) {
            usingEstimatedKeyboardHeight = false
        }
        lastKeyboardHeight = target
        if (animate) {
            animateBottomBarOffset(target)
        } else {
            applyBottomBarOffset(target)
        }
        applyChatBottomPadding(target)
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

    private fun keyboardHeightFromVisibleFrame(): Int {
        binding.root.getWindowVisibleDisplayFrame(visibleFrame)
        val rootHeight = binding.root.rootView.height
        if (rootHeight <= 0 || visibleFrame.bottom <= 0) return 0

        val hiddenHeight = (rootHeight - visibleFrame.bottom).coerceAtLeast(0)
        if (baselineHiddenHeight < 0 || hiddenHeight < baselineHiddenHeight) {
            baselineHiddenHeight = hiddenHeight
        }
        return (hiddenHeight - baselineHiddenHeight.coerceAtLeast(0)).coerceAtLeast(0)
    }

    private fun keyboardHeightFromInsetsOrFrame(insets: WindowInsetsCompat): Int {
        return maxOf(keyboardHeight(insets), keyboardHeightFromVisibleFrame())
    }

    private fun estimatedKeyboardHeight(): Int {
        val rootHeight = binding.root.rootView.height
        if (rootHeight <= 0) return 0
        return (rootHeight * 0.42f).toInt()
    }

    private fun shouldIgnoreZeroKeyboardHeight(keyboardHeight: Int): Boolean {
        return keyboardHeight == 0 && usingEstimatedKeyboardHeight && binding.inputEdit.hasFocus()
    }

    private fun keyboardHeight(insets: WindowInsetsCompat): Int {
        val imeBottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
        val systemBottom = insets.getInsets(WindowInsetsCompat.Type.systemBars()).bottom
        return (imeBottom - systemBottom).coerceAtLeast(0)
    }

    private fun Int?.orEmpty(): Int = this ?: 0

    private fun WindowInsetsAnimationCompat.includesIme(): Boolean {
        return typeMask and WindowInsetsCompat.Type.ime() != 0
    }
}
