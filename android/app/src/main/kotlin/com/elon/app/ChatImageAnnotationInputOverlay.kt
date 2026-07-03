package com.elon.app

import android.app.Activity
import android.graphics.Color
import android.graphics.RectF
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
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
    private var currentBounds: RectF? = null
    private var currentPanelWidth = 0

    fun show(index: Int) {
        val bounds = canvasView.annotationPanelBounds(index) ?: return
        if (root.width <= 0 || root.height <= 0) {
            root.post { show(index) }
            return
        }
        currentIndex = index
        currentBounds = bounds
        val view = ensurePanel()
        input?.setText(canvasView.annotationNote(index))
        input?.setSelection(input?.text?.length ?: 0)
        positionPanel(view, bounds)
        input?.post { refreshPanelHeight() }
        view.visibility = View.VISIBLE
        view.alpha = 0f
        view.scaleX = 0.96f
        view.scaleY = 0.96f
        view.animate().alpha(1f).scaleX(1f).scaleY(1f).setDuration(140L).start()
        input?.requestFocus()
        input?.post { showKeyboard(input) }
    }

    private fun ensurePanel(): FrameLayout {
        panel?.let { return it }
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

    private fun createInput(): EditText {
        return EditText(activity).apply {
            input = this
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.TOP or Gravity.START
            hint = "请输入标注内容"
            includeFontPadding = false
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
                topMargin = dp(16)
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

    private fun positionPanel(view: FrameLayout, bounds: RectF) {
        val width = min(root.width - dp(48), max(dp(260), (bounds.width() * 0.74f).roundToInt()))
        currentPanelWidth = width
        val height = measuredPanelHeight()
        val left = (bounds.left + (bounds.width() - width) / 2f)
            .roundToInt()
            .coerceIn(dp(16), max(dp(16), root.width - width - dp(16)))
        val topLimit = max(dp(72), root.height - height - dp(180))
        val top = (bounds.top + (bounds.height() - height) / 2f)
            .roundToInt()
            .coerceIn(dp(72), topLimit)
        view.layoutParams = FrameLayout.LayoutParams(width, height).apply {
            leftMargin = left
            topMargin = top
        }
    }

    private fun refreshPanelHeight() {
        val view = panel ?: return
        val bounds = currentBounds ?: return
        if (currentPanelWidth <= 0) return
        positionPanel(view, bounds)
        view.requestLayout()
    }

    private fun measuredPanelHeight(): Int {
        val textView = input
        val lineCount = max(1, textView?.lineCount ?: 1)
        val lineHeight = textView?.lineHeight ?: dp(22)
        val textHeight = lineCount * lineHeight
        val chromeHeight = dp(16) + dp(52)
        val minHeight = max(dp(124), (currentBounds?.height()?.times(0.72f) ?: 0f).roundToInt())
        val desiredHeight = max(minHeight, textHeight + chromeHeight)
        val availableHeight = max(dp(124), root.height - dp(96) - dp(180))
        return min(desiredHeight, availableHeight)
    }

    private fun collapse() {
        currentIndex?.let { index ->
            canvasView.updateAnnotationNote(index, input?.text?.toString().orEmpty())
        }
        currentIndex = null
        currentBounds = null
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
