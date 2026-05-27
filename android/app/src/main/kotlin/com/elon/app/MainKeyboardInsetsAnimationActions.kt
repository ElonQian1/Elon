package com.elon.app

import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsAnimationCompat
import androidx.core.view.WindowInsetsCompat
import com.elon.app.databinding.ActivityMainBinding

internal class MainKeyboardInsetsAnimationActions(
    private val binding: ActivityMainBinding
) {
    fun install() {
        val animatedView = binding.bottomBarContainer
        ViewCompat.setWindowInsetsAnimationCallback(
            binding.root,
            object : WindowInsetsAnimationCompat.Callback(DISPATCH_MODE_CONTINUE_ON_SUBTREE) {
                private var runningImeAnimation = false
                private var startBottom = 0
                private var endBottom = 0
                private var baseTranslationY = 0f

                override fun onPrepare(animation: WindowInsetsAnimationCompat) {
                    if (!animation.includesIme()) return
                    runningImeAnimation = true
                    startBottom = animatedView.bottom
                    baseTranslationY = animatedView.translationY
                }

                override fun onStart(
                    animation: WindowInsetsAnimationCompat,
                    bounds: WindowInsetsAnimationCompat.BoundsCompat
                ): WindowInsetsAnimationCompat.BoundsCompat {
                    if (runningImeAnimation && animation.includesIme()) {
                        endBottom = animatedView.bottom
                        applyProgress(animatedView, fraction = 0f)
                    }
                    return bounds
                }

                override fun onProgress(
                    insets: WindowInsetsCompat,
                    runningAnimations: MutableList<WindowInsetsAnimationCompat>
                ): WindowInsetsCompat {
                    val imeAnimation = runningAnimations.lastOrNull { it.includesIme() } ?: return insets
                    if (runningImeAnimation) {
                        applyProgress(
                            animatedView,
                            fraction = imeAnimation.interpolatedFraction.coerceIn(0f, 1f)
                        )
                    }
                    return insets
                }

                override fun onEnd(animation: WindowInsetsAnimationCompat) {
                    if (!animation.includesIme()) return
                    animatedView.translationY = baseTranslationY
                    runningImeAnimation = false
                    startBottom = 0
                    endBottom = 0
                }

                private fun applyProgress(view: View, fraction: Float) {
                    val resizeDelta = startBottom - endBottom
                    if (resizeDelta == 0) {
                        view.translationY = baseTranslationY
                        return
                    }
                    view.translationY = baseTranslationY + resizeDelta * (1f - fraction)
                }
            }
        )
    }

    private fun WindowInsetsAnimationCompat.includesIme(): Boolean {
        return typeMask and WindowInsetsCompat.Type.ime() != 0
    }
}
