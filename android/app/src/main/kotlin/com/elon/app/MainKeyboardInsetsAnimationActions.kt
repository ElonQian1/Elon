package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.graphics.Rect
import android.os.SystemClock
import android.view.View
import android.view.ViewTreeObserver
import android.view.animation.PathInterpolator
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsAnimationCompat
import androidx.core.view.WindowInsetsCompat
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.elon.app.databinding.ActivityMainBinding

internal class MainKeyboardInsetsAnimationActions(
    private val binding: ActivityMainBinding
) {
    private val fallbackInterpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private var bottomOffsetAnimator: ValueAnimator? = null
    private var chatLiftAnimator: ValueAnimator? = null
    private var runningImeAnimation = false
    private var baseChatListPaddingBottom = 0
    private var baselineHiddenHeight = -1
    private var lastKeyboardHeight = -1
    private var lastKnownKeyboardHeight = -1
    private var usingEstimatedKeyboardHeight = false
    private var keyboardOverlayMode = false
    private var liftRequestedAt = 0L
    private var keyboardWasVisibleSinceLift = false
    private var followChatBottomDuringLift = false
    private val visibleFrame = Rect()

    fun install() {
        baseChatListPaddingBottom = binding.chatList.paddingBottom
        installVisibleFrameFallback()
        installFocusFallback()
        installBottomBarLayoutSync()

        ViewCompat.setOnApplyWindowInsetsListener(binding.root) { _, insets ->
            if (!runningImeAnimation) {
                val keyboardHeight = keyboardHeightFromInsetsOrFrame(insets)
                val imeVisible = insets.isVisible(WindowInsetsCompat.Type.ime())
                markKeyboardVisibleIfNeeded(keyboardHeight)
                if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight, imeVisible)) {
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
                    chatLiftAnimator?.cancel()
                    binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_HARDWARE, null)
                }

                override fun onProgress(
                    insets: WindowInsetsCompat,
                    runningAnimations: MutableList<WindowInsetsAnimationCompat>
                ): WindowInsetsCompat {
                    if (runningAnimations.any { it.includesIme() }) {
                        val keyboardHeight = keyboardHeightFromAnimationProgress(insets)
                        val imeVisible = insets.isVisible(WindowInsetsCompat.Type.ime())
                        markKeyboardVisibleIfNeeded(keyboardHeight)
                        if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight, imeVisible)) {
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
                    val imeVisible = insets?.isVisible(WindowInsetsCompat.Type.ime()) == true
                    markKeyboardVisibleIfNeeded(keyboardHeight)
                    if (!shouldIgnoreZeroKeyboardHeight(keyboardHeight, imeVisible)) {
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
        liftRequestedAt = SystemClock.uptimeMillis()
        keyboardWasVisibleSinceLift = false
        followChatBottomDuringLift = shouldFollowLatestMessage()
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
        keyboardWasVisibleSinceLift = false
        applyKeyboardHeight(0, animate = true)
    }

    fun setKeyboardOverlayMode(enabled: Boolean) {
        if (keyboardOverlayMode == enabled) return
        keyboardOverlayMode = enabled
        if (enabled) {
            bottomOffsetAnimator?.cancel()
            binding.bottomBarContainer.setLayerType(View.LAYER_TYPE_NONE, null)
            applyBottomBarOffset(0)
            applyChatBottomPadding(0)
            return
        }
        val target = lastKeyboardHeight.coerceAtLeast(0)
        applyKeyboardHeight(target, animate = true)
    }

    fun replacementPanelHeight(fallbackHeight: Int): Int {
        val rememberedHeight = when {
            lastKeyboardHeight > 0 -> lastKeyboardHeight
            lastKnownKeyboardHeight > 0 -> lastKnownKeyboardHeight
            else -> 0
        }
        if (rememberedHeight > 0) return rememberedHeight

        val rootHeight = binding.root.rootView.height
        val maxHeight = if (rootHeight > 0) {
            (rootHeight * MAX_REPLACEMENT_PANEL_RATIO).toInt()
        } else {
            Int.MAX_VALUE
        }
        val minHeight = if (rootHeight > 0) {
            (rootHeight * MIN_REPLACEMENT_PANEL_RATIO).toInt()
        } else {
            0
        }
        val estimatedHeight = estimatedKeyboardHeight()
        val candidate = if (estimatedHeight > 0) estimatedHeight else fallbackHeight
        return candidate
            .coerceAtLeast(minHeight.coerceAtMost(maxHeight))
            .coerceAtMost(maxHeight)
            .takeIf { it > 0 }
            ?: fallbackHeight
    }

    private fun installVisibleFrameFallback() {
        binding.root.viewTreeObserver.addOnGlobalLayoutListener(object : ViewTreeObserver.OnGlobalLayoutListener {
            override fun onGlobalLayout() {
                val keyboardHeight = keyboardHeightFromVisibleFrame()
                val imeVisible = isImeVisible()
                markKeyboardVisibleIfNeeded(keyboardHeight)
                if (shouldIgnoreZeroKeyboardHeight(keyboardHeight, imeVisible)) return
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

    private fun installBottomBarLayoutSync() {
        binding.bottomBarContainer.addOnLayoutChangeListener { _, _, top, _, bottom, _, oldTop, _, oldBottom ->
            if (binding.chatPage.visibility != View.VISIBLE) return@addOnLayoutChangeListener
            val oldHeight = oldBottom - oldTop
            val newHeight = bottom - top
            val heightDelta = newHeight - oldHeight
            if (oldHeight <= 0 || heightDelta == 0) return@addOnLayoutChangeListener
            if (!followChatBottomDuringLift && !shouldFollowLatestMessage()) return@addOnLayoutChangeListener
            followChatBottomDuringLift = binding.inputEdit.hasFocus() || followChatBottomDuringLift
            binding.chatList.post {
                if (heightDelta != 0) {
                    binding.chatList.scrollBy(0, heightDelta)
                }
                keepLatestMessageAboveInput(binding.chatList.layoutManager as? LinearLayoutManager)
            }
        }
    }

    private fun scheduleEstimatedKeyboardLift() {
        binding.root.postDelayed({ applyEstimatedKeyboardLiftIfNeeded() }, 80L)
        binding.root.postDelayed({ applyEstimatedKeyboardLiftIfNeeded() }, 180L)
    }

    private fun applyEstimatedKeyboardLiftIfNeeded() {
        if (!binding.inputEdit.hasFocus()) return
        val frameHeight = keyboardHeightFromVisibleFrame()
        if (frameHeight > 0) {
            markKeyboardVisibleIfNeeded(frameHeight)
            return
        }
        val insets = ViewCompat.getRootWindowInsets(binding.root)
        val insetsHeight = insets?.let(::keyboardHeight).orEmpty()
        if (insetsHeight > 0) {
            markKeyboardVisibleIfNeeded(insetsHeight)
            return
        }
        usingEstimatedKeyboardHeight = true
        applyKeyboardHeight(estimatedKeyboardHeight(), animate = true)
    }

    private fun applyKnownOrEstimatedKeyboardHeight() {
        if (!binding.inputEdit.hasFocus()) return
        val insets = ViewCompat.getRootWindowInsets(binding.root)
        val knownHeight = insets?.let(::keyboardHeightFromInsetsOrFrame) ?: keyboardHeightFromVisibleFrame()
        markKeyboardVisibleIfNeeded(knownHeight)
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
        if (target > 0) {
            lastKnownKeyboardHeight = target
        }
        if (target == 0) {
            usingEstimatedKeyboardHeight = false
            keyboardWasVisibleSinceLift = false
        }
        lastKeyboardHeight = target
        if (keyboardOverlayMode) {
            applyBottomBarOffset(0)
            applyChatBottomPadding(0)
            return
        }
        if (animate) {
            animateBottomBarOffset(target)
            animateChatBottomPadding(target)
        } else {
            applyBottomBarOffset(target)
            chatLiftAnimator?.cancel()
            applyChatBottomPadding(target)
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

    private fun animateChatBottomPadding(keyboardHeight: Int) {
        val target = keyboardHeight.coerceAtLeast(0)
        val start = (binding.chatList.paddingBottom - baseChatListPaddingBottom).coerceAtLeast(0)
        if (start == target) {
            applyChatBottomPadding(target)
            return
        }
        var cancelled = false
        chatLiftAnimator?.cancel()
        binding.chatList.setLayerType(View.LAYER_TYPE_HARDWARE, null)
        chatLiftAnimator = ValueAnimator.ofInt(start, target).apply {
            duration = 220L
            interpolator = fallbackInterpolator
            addUpdateListener { animator ->
                applyChatBottomPadding(animator.animatedValue as Int)
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                }

                override fun onAnimationEnd(animation: Animator) {
                    binding.chatList.setLayerType(View.LAYER_TYPE_NONE, null)
                    if (!cancelled) {
                        applyChatBottomPadding(target)
                    }
                    chatLiftAnimator = null
                }
            })
            start()
        }
    }

    private fun applyChatBottomPadding(keyboardHeight: Int) {
        val targetBottom = baseChatListPaddingBottom + keyboardHeight
        val currentBottom = binding.chatList.paddingBottom
        if (currentBottom == targetBottom) {
            if (keyboardHeight == 0) {
                followChatBottomDuringLift = false
            }
            return
        }
        val layoutManager = binding.chatList.layoutManager as? LinearLayoutManager
        val shouldFollowLatest = followChatBottomDuringLift || shouldFollowLatestMessage(layoutManager)
        val anchor = if (shouldFollowLatest) null else chatListAnchor(layoutManager)
        binding.chatList.setPadding(
            binding.chatList.paddingLeft,
            binding.chatList.paddingTop,
            binding.chatList.paddingRight,
            targetBottom
        )
        if (shouldFollowLatest) {
            followChatBottomDuringLift = keyboardHeight > 0
            val delta = targetBottom - currentBottom
            if (delta != 0) {
                binding.chatList.scrollBy(0, delta)
            }
            keepLatestMessageAboveInput(layoutManager)
        } else if (anchor != null) {
            layoutManager?.scrollToPositionWithOffset(anchor.position, anchor.offset)
        }
        if (keyboardHeight == 0) {
            followChatBottomDuringLift = false
        }
    }

    private fun shouldFollowLatestMessage(
        layoutManager: LinearLayoutManager? = binding.chatList.layoutManager as? LinearLayoutManager
    ): Boolean {
        if (binding.chatPage.visibility != View.VISIBLE) return false
        val itemCount = binding.chatList.adapter?.itemCount ?: return false
        if (itemCount <= 0) return false
        val lastVisible = layoutManager?.findLastVisibleItemPosition() ?: RecyclerView.NO_POSITION
        return lastVisible >= itemCount - 1
    }

    private fun keepLatestMessageAboveInput(layoutManager: LinearLayoutManager?) {
        val itemCount = binding.chatList.adapter?.itemCount ?: return
        if (itemCount <= 0) return
        val latestPosition = itemCount - 1
        val viewportBottom = binding.chatList.height - binding.chatList.paddingBottom
        val latestView = layoutManager?.findViewByPosition(latestPosition)
        if (latestView == null) {
            binding.chatList.scrollToPosition(latestPosition)
            return
        }
        val coveredDistance = latestView.bottom - viewportBottom
        if (coveredDistance > 0) {
            binding.chatList.scrollBy(0, coveredDistance)
        }
    }

    private fun chatListAnchor(layoutManager: LinearLayoutManager?): ChatListAnchor? {
        val anchorPosition = layoutManager?.findFirstVisibleItemPosition() ?: RecyclerView.NO_POSITION
        if (anchorPosition == RecyclerView.NO_POSITION) return null
        val anchorView = layoutManager?.findViewByPosition(anchorPosition) ?: return null
        return ChatListAnchor(
            position = anchorPosition,
            offset = anchorView.top - binding.chatList.paddingTop
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

    private fun keyboardHeightFromAnimationProgress(insets: WindowInsetsCompat): Int {
        val insetHeight = keyboardHeight(insets)
        if (insetHeight > 0 || insets.isVisible(WindowInsetsCompat.Type.ime())) {
            return insetHeight
        }
        return keyboardHeightFromVisibleFrame()
    }

    private fun estimatedKeyboardHeight(): Int {
        val rootHeight = binding.root.rootView.height
        if (rootHeight <= 0) return 0
        return (rootHeight * 0.42f).toInt()
    }

    private fun shouldIgnoreZeroKeyboardHeight(keyboardHeight: Int, imeVisible: Boolean): Boolean {
        if (keyboardHeight != 0 || !binding.inputEdit.hasFocus()) {
            return false
        }
        if (imeVisible) {
            return true
        }
        if (!keyboardWasVisibleSinceLift && recentlyRequestedKeyboardLift()) {
            return true
        }
        return false
    }

    private fun markKeyboardVisibleIfNeeded(keyboardHeight: Int) {
        if (keyboardHeight > 0) {
            keyboardWasVisibleSinceLift = true
            usingEstimatedKeyboardHeight = false
        }
    }

    private fun isImeVisible(): Boolean {
        return ViewCompat.getRootWindowInsets(binding.root)
            ?.isVisible(WindowInsetsCompat.Type.ime()) == true
    }

    private fun recentlyRequestedKeyboardLift(): Boolean {
        val elapsed = SystemClock.uptimeMillis() - liftRequestedAt
        return elapsed >= 0L && elapsed < ZERO_HEIGHT_GRACE_MS
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

    private data class ChatListAnchor(
        val position: Int,
        val offset: Int
    )

    private companion object {
        private const val ZERO_HEIGHT_GRACE_MS = 900L
        private const val MIN_REPLACEMENT_PANEL_RATIO = 0.24f
        private const val MAX_REPLACEMENT_PANEL_RATIO = 0.48f
    }
}
