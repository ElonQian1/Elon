package com.elon.app

import android.animation.ValueAnimator
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ScrollView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.abs
import kotlin.math.min

internal enum class HomeListFilterMode {
    All,
    Projects,
    Friends
}

internal fun HomeListFilterMode.nextPullMode(): HomeListFilterMode = when (this) {
    HomeListFilterMode.All -> HomeListFilterMode.Projects
    HomeListFilterMode.Projects -> HomeListFilterMode.Friends
    HomeListFilterMode.Friends -> HomeListFilterMode.All
}

internal class HomePullFilterController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val dp: (Int) -> Int,
    private val isEnabled: () -> Boolean,
    private val currentMode: () -> HomeListFilterMode,
    private val applyMode: (HomeListFilterMode) -> Unit
) {
    private val touchSlop = ViewConfiguration.get(activity).scaledTouchSlop
    private val activationDistance = maxOf(touchSlop * 1.6f, dp(24).toFloat())
    private val triggerDistance = dp(104).toFloat()
    private val decisiveDistance = triggerDistance + dp(18)
    private val maxPullDistance = dp(176).toFloat()
    private val maxContentStretch = dp(32).toFloat()
    private val indicator = HomePullFilterIndicatorView(activity)

    private var scroller: ScrollView? = null
    private var downX = 0f
    private var downY = 0f
    private var pulling = false
    private var longHoldTriggered = false
    private var longHoldArmed = false
    private var pullProgress = 0f
    private var thresholdReadySince = 0L
    private var maxGestureDistance = 0f
    private var startedAtTop = false
    private var gestureRejected = false
    private var lastModeAppliedAt = 0L

    private val longHoldRunnable = Runnable {
        if (!pulling || !isPastReleaseThreshold()) return@Runnable
        longHoldTriggered = true
        longHoldArmed = false
        indicator.flashWhite()
        applyMode(HomeListFilterMode.All)
        lastModeAppliedAt = SystemClock.uptimeMillis()
    }

    fun attach() {
        val scrollView = binding.conversationPage.parent as? ScrollView ?: return
        scroller = scrollView
        installIndicator()
        if (scrollView is HomeConversationScrollView) {
            scrollView.pullTouchHandler = ::handleTouch
        } else {
            scrollView.setOnTouchListener { _, event -> handleTouch(event) }
        }
    }

    private fun installIndicator() {
        if (indicator.parent == null) {
            binding.contentContainer.addView(
                indicator,
                FrameLayout.LayoutParams(dp(42), dp(42), android.view.Gravity.TOP or android.view.Gravity.CENTER_HORIZONTAL)
                    .apply { topMargin = dp(8) }
            )
        }
        indicator.visibility = View.GONE
    }

    private fun handleTouch(event: MotionEvent): Boolean {
        val scrollView = scroller ?: return false
        if (!isGestureEnabled()) {
            resetGesture()
            return false
        }

        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.x
                downY = event.y
                pulling = false
                longHoldTriggered = false
                pullProgress = 0f
                thresholdReadySince = 0L
                maxGestureDistance = 0f
                startedAtTop = !scrollView.canScrollVertically(-1)
                gestureRejected = false
                cancelLongHold()
                return false
            }

            MotionEvent.ACTION_MOVE -> {
                if (!startedAtTop || gestureRejected) {
                    return false
                }
                val dy = event.y - downY
                val dx = abs(event.x - downX)
                if (!pulling) {
                    if (dy < -touchSlop || (dx > activationDistance && dx > abs(dy) * HORIZONTAL_REJECT_RATIO)) {
                        gestureRejected = true
                        return false
                    }
                    if (
                        scrollView.canScrollVertically(-1) ||
                        dy < activationDistance ||
                        dy < dx * VERTICAL_ACTIVATION_RATIO ||
                        event.pointerCount != 1
                    ) {
                        return false
                    }
                    pulling = true
                    longHoldTriggered = false
                    scrollView.parent?.requestDisallowInterceptTouchEvent(true)
                    showIndicator()
                }
                updatePull((dy - activationDistance).coerceAtLeast(0f), event.eventTime)
                return true
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                if (!pulling) {
                    resetGesture()
                    return false
                }
                val shouldSwitch = event.actionMasked == MotionEvent.ACTION_UP &&
                    isStableRelease(event.eventTime)
                if (shouldSwitch) {
                    indicator.pulseComplete()
                    applyMode(currentMode().nextPullMode())
                    lastModeAppliedAt = event.eventTime
                }
                hideIndicator(if (longHoldTriggered) 220L else 160L)
                resetGesture()
                return true
            }
        }
        return false
    }

    private fun isGestureEnabled(): Boolean {
        return isEnabled() && (pulling || SystemClock.uptimeMillis() - lastModeAppliedAt >= SWITCH_COOLDOWN_MS)
    }

    private fun showIndicator() {
        indicator.animate().cancel()
        indicator.visibility = View.VISIBLE
        indicator.alpha = 1f
        indicator.scaleX = 0.86f
        indicator.scaleY = 0.86f
        indicator.translationY = 0f
        binding.conversationPage.animate().cancel()
        indicator.start()
        indicator.update(currentMode().nextPullMode(), 0f)
    }

    private fun updatePull(distance: Float, eventTime: Long) {
        val cappedDistance = min(distance, maxPullDistance)
        maxGestureDistance = maxOf(maxGestureDistance, cappedDistance)
        val nextProgress = (cappedDistance / triggerDistance).coerceIn(0f, 1f)
        pullProgress = nextProgress
        indicator.update(currentMode().nextPullMode(), nextProgress)
        indicator.translationY = min(cappedDistance * 0.34f, dp(34).toFloat())
        indicator.scaleX = 0.86f + 0.14f * nextProgress
        indicator.scaleY = indicator.scaleX
        binding.conversationPage.translationY = min(cappedDistance * 0.22f, maxContentStretch)

        if (nextProgress >= RELEASE_PROGRESS_THRESHOLD) {
            if (thresholdReadySince == 0L) thresholdReadySince = eventTime
            armLongHold()
        } else if (nextProgress < RELEASE_RESET_PROGRESS) {
            thresholdReadySince = 0L
            cancelLongHold()
        }
    }

    private fun isPastReleaseThreshold(): Boolean {
        return thresholdReadySince > 0L && pullProgress >= RELEASE_RESET_PROGRESS
    }

    private fun isStableRelease(eventTime: Long): Boolean {
        if (longHoldTriggered || !isPastReleaseThreshold()) return false
        return eventTime - thresholdReadySince >= RELEASE_STABLE_MS ||
            maxGestureDistance >= decisiveDistance
    }

    private fun armLongHold() {
        if (longHoldTriggered || longHoldArmed) return
        longHoldArmed = true
        indicator.postDelayed(longHoldRunnable, 1000L)
    }

    private fun cancelLongHold() {
        longHoldArmed = false
        indicator.removeCallbacks(longHoldRunnable)
    }

    private fun hideIndicator(durationMs: Long) {
        indicator.animate()
            .alpha(0f)
            .scaleX(0.82f)
            .scaleY(0.82f)
            .translationY(0f)
            .setDuration(durationMs)
            .withEndAction {
                indicator.visibility = View.GONE
                indicator.stop()
                indicator.update(currentMode().nextPullMode(), 0f)
            }
            .start()

        binding.conversationPage.animate()
            .translationY(0f)
            .setDuration(durationMs)
            .start()
    }

    private fun resetGesture() {
        pulling = false
        pullProgress = 0f
        longHoldTriggered = false
        thresholdReadySince = 0L
        maxGestureDistance = 0f
        startedAtTop = false
        gestureRejected = false
        cancelLongHold()
    }

    private companion object {
        const val RELEASE_PROGRESS_THRESHOLD = 1f
        const val RELEASE_RESET_PROGRESS = 0.68f
        const val RELEASE_STABLE_MS = 80L
        const val SWITCH_COOLDOWN_MS = 0L
        const val HORIZONTAL_REJECT_RATIO = 1.35f
        const val VERTICAL_ACTIVATION_RATIO = 0.7f
    }
}

private class HomePullFilterIndicatorView(context: android.content.Context) : View(context) {
    private val density = resources.displayMetrics.density
    private val ringRect = RectF()
    private val shellPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#212121")
        style = Paint.Style.FILL
    }
    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#4D4D4D")
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeWidth = 2.3f * density
    }
    private val progressPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeWidth = 2.8f * density
    }
    private val glowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeWidth = 4.2f * density
        color = Color.WHITE
    }

    private var targetMode = HomeListFilterMode.Projects
    private var progress = 0f
    private var spin = 0f
    private var whiteFlash = 0f
    private var flashAnimator: ValueAnimator? = null
    private val spinner = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 900L
        repeatCount = ValueAnimator.INFINITE
        repeatMode = ValueAnimator.RESTART
        interpolator = LinearInterpolator()
        addUpdateListener {
            spin = it.animatedFraction
            invalidate()
        }
    }

    fun start() {
        if (!spinner.isRunning) spinner.start()
    }

    fun stop() {
        spinner.cancel()
        flashAnimator?.cancel()
        whiteFlash = 0f
        spin = 0f
    }

    fun update(mode: HomeListFilterMode, value: Float) {
        targetMode = mode
        progress = value.coerceIn(0f, 1f)
        invalidate()
    }

    fun pulseComplete() {
        animate().cancel()
        animate().scaleX(1.08f).scaleY(1.08f).setDuration(90L).withEndAction {
            animate().scaleX(1f).scaleY(1f).setDuration(110L).start()
        }.start()
    }

    fun flashWhite() {
        flashAnimator?.cancel()
        flashAnimator = ValueAnimator.ofFloat(0f, 1f, 0f).apply {
            duration = 620L
            addUpdateListener {
                whiteFlash = it.animatedValue as Float
                invalidate()
            }
            start()
        }
    }

    override fun onDetachedFromWindow() {
        stop()
        super.onDetachedFromWindow()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val cx = width / 2f
        val cy = height / 2f
        val radius = (min(width, height) / 2f - 8.5f * density).coerceAtLeast(8f * density)
        ringRect.set(cx - radius, cy - radius, cx + radius, cy + radius)

        canvas.drawCircle(cx, cy, min(width, height) / 2f - density, shellPaint)
        canvas.drawArc(ringRect, -90f, 360f, false, basePaint)

        val targetColor = when (targetMode) {
            HomeListFilterMode.Friends -> Color.parseColor("#58BE6A")
            else -> Color.parseColor("#F2C94C")
        }
        progressPaint.color = blend(targetColor, Color.WHITE, whiteFlash)
        val start = -90f + spin * 360f
        canvas.drawArc(ringRect, start, 360f * progress, false, progressPaint)
        if (whiteFlash > 0f) {
            glowPaint.alpha = (whiteFlash * 210).toInt().coerceIn(0, 255)
            canvas.drawArc(ringRect, start + 18f, 84f, false, glowPaint)
        }
    }

    private fun blend(from: Int, to: Int, ratio: Float): Int {
        val t = ratio.coerceIn(0f, 1f)
        return Color.rgb(
            (Color.red(from) + (Color.red(to) - Color.red(from)) * t).toInt(),
            (Color.green(from) + (Color.green(to) - Color.green(from)) * t).toInt(),
            (Color.blue(from) + (Color.blue(to) - Color.blue(from)) * t).toInt()
        )
    }
}
