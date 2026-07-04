package com.elon.app

import android.app.Activity
import android.graphics.Color
import android.graphics.Rect
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.ViewTreeObserver
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.TextView
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

internal class ChatImageAnnotationInputOverlay(
    private val activity: Activity,
    private val root: FrameLayout,
    private val canvasView: ChatImageEditCanvasView
) {
    private var panel: FrameLayout? = null
    private var input: EditText? = null
    private var currentIndex: Int? = null
    private var currentPanelWidth = 0
    private var keyboardTopInRoot = 0
    private var keyboardVisible = false
    private var layoutListenerAttached = false
    private val windowVisibleFrame = Rect()
    private val rootLocation = IntArray(2)
    private val globalLayoutListener = ViewTreeObserver.OnGlobalLayoutListener {
        updateKeyboardState()
        if (panel?.visibility == View.VISIBLE) {
            refreshPanelHeight()
        }
    }

    fun show(index: Int) {
        if (canvasView.annotationPanelBounds(index) == null) return
        if (root.width <= 0 || root.height <= 0) {
            root.post { show(index) }
            return
        }
        currentIndex = index
        val view = ensurePanel()
        updateKeyboardState()
        input?.setText(canvasView.annotationNote(index))
        input?.setSelection(input?.text?.length ?: 0)
        positionPanel(view)
        input?.post { refreshPanelHeight() }
        view.visibility = View.VISIBLE
        view.alpha = 0f
        view.scaleX = 0.96f
        view.scaleY = 0.96f
        view.animate().alpha(1f).scaleX(1f).scaleY(1f).setDuration(140L).start()
        input?.requestFocus()
        input?.post { showKeyboard(input) }
    }

    fun commitActive() {
        currentIndex?.let { index ->
            canvasView.updateAnnotationNote(index, input?.text?.toString().orEmpty())
        }
    }

    private fun ensurePanel(): FrameLayout {
        panel?.let { return it }
        attachLayoutListener()
        return FrameLayout(activity).apply {
            visibility = View.INVISIBLE
            background = roundedRect()
            elevation = dp(10).toFloat()
            addView(createInput())
            addView(createDoneButton())
            root.addView(this)
            panel = this
        }
    }

    private fun attachLayoutListener() {
        if (layoutListenerAttached) return
        root.viewTreeObserver.addOnGlobalLayoutListener(globalLayoutListener)
        layoutListenerAttached = true
    }

    private fun createInput(): EditText {
        return EditText(activity).apply {
            input = this
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.TOP or Gravity.START
            hint = "请输入标注内容"
            includeFontPadding = true
            minLines = 1
            maxLines = Int.MAX_VALUE
            isVerticalScrollBarEnabled = false
            setHintTextColor(Color.parseColor("#777777"))
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 15f
            setPadding(0, 0, 0, 0)
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
                override fun afterTextChanged(s: Editable?) {
                    post { refreshPanelHeight() }
                }
            })
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ).apply {
                leftMargin = dp(16)
                topMargin = dp(14)
                rightMargin = dp(16)
                bottomMargin = dp(52)
            }
        }
    }

    private fun createDoneButton(): TextView {
        return TextView(activity).apply {
            text = "完成"
            gravity = Gravity.CENTER
            includeFontPadding = false
            setTextColor(Color.parseColor("#D9D9D9"))
            textSize = 15f
            layoutParams = FrameLayout.LayoutParams(dp(64), dp(40), Gravity.END or Gravity.BOTTOM).apply {
                rightMargin = dp(16)
                bottomMargin = dp(8)
            }
            setOnClickListener { collapse() }
        }
    }

    private fun positionPanel(view: FrameLayout) {
        val width = min(root.width - dp(48), max(dp(280), (root.width * 0.72f).roundToInt()))
        currentPanelWidth = width
        val height = measuredPanelHeight()
        val left = ((root.width - width) / 2f).roundToInt()
            .coerceIn(dp(16), max(dp(16), root.width - width - dp(16)))
        val usableBottom = keyboardTopForPosition()
        val bottomGap = if (keyboardVisible) dp(52) else dp(188)
        val minTop = dp(72)
        val maxTop = max(minTop, usableBottom - height - dp(12))
        val top = (usableBottom - height - bottomGap).coerceIn(minTop, maxTop)
        val currentParams = view.layoutParams as? FrameLayout.LayoutParams
        if (
            currentParams?.width == width &&
            currentParams.height == height &&
            currentParams.leftMargin == left &&
            currentParams.topMargin == top
        ) {
            return
        }
        view.layoutParams = FrameLayout.LayoutParams(width, height).apply {
            leftMargin = left
            topMargin = top
        }
    }

    private fun refreshPanelHeight() {
        val view = panel ?: return
        currentIndex ?: return
        if (currentPanelWidth <= 0) return
        updateKeyboardState()
        positionPanel(view)
    }

    private fun measuredPanelHeight(): Int {
        val textView = input
        val lineCount = max(1, textView?.lineCount ?: 1)
        val lineHeight = textView?.lineHeight ?: dp(22)
        val textHeight = lineCount * lineHeight
        val chromeHeight = dp(14) + dp(56)
        val minHeight = dp(124)
        val desiredHeight = max(minHeight, textHeight + chromeHeight)
        val bottomGap = if (keyboardVisible) dp(52) else dp(188)
        val availableHeight = max(dp(124), keyboardTopForPosition() - dp(96) - bottomGap)
        return min(desiredHeight, availableHeight)
    }

    private fun updateKeyboardState() {
        if (root.height <= 0) {
            keyboardTopInRoot = 0
            keyboardVisible = false
            return
        }
        root.getWindowVisibleDisplayFrame(windowVisibleFrame)
        root.getLocationOnScreen(rootLocation)
        val visibleBottom = (windowVisibleFrame.bottom - rootLocation[1]).coerceIn(0, root.height)
        val hiddenHeight = (root.height - visibleBottom).coerceAtLeast(0)
        keyboardTopInRoot = if (visibleBottom > 0) visibleBottom else root.height
        keyboardVisible = hiddenHeight > max(dp(120), root.height / 5)
    }

    private fun keyboardTopForPosition(): Int {
        val keyboardTop = keyboardTopInRoot.takeIf { it > 0 } ?: root.height
        return keyboardTop.coerceIn(min(dp(160), root.height), root.height)
    }

    private fun collapse() {
        commitActive()
        currentIndex = null
        currentPanelWidth = 0
        hideKeyboard(input)
        panel?.animate()
            ?.alpha(0f)
            ?.scaleX(0.92f)
            ?.scaleY(0.92f)
            ?.setDuration(130L)
            ?.withEndAction {
                panel?.visibility = View.INVISIBLE
                panel?.scaleX = 1f
                panel?.scaleY = 1f
            }
            ?.start()
    }

    private fun showKeyboard(view: View?) {
        val inputMethod = activity.getSystemService(InputMethodManager::class.java)
        inputMethod?.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT)
    }

    private fun hideKeyboard(view: View?) {
        val inputMethod = activity.getSystemService(InputMethodManager::class.java)
        val token = view?.windowToken ?: return
        inputMethod?.hideSoftInputFromWindow(token, 0)
    }

    private fun roundedRect(): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = dp(8).toFloat()
            setColor(Color.parseColor("#171717"))
            setStroke(dp(1), Color.parseColor("#333333"))
        }
    }

    private fun dp(value: Int): Int {
        return (value * activity.resources.displayMetrics.density).toInt()
    }
}
