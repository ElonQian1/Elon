package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

/**
 * 全屏文章编辑器浮层。
 *
 * 当输入框内容较长时，用户可点击展开按钮进入全屏编辑模式，
 * 获得类似文章编辑器的宽敞输入体验。关闭时文本自动同步回原输入框。
 */
internal class FullScreenEditorOverlay(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val getInputText: () -> String,
    private val setInputText: (String) -> Unit,
    private val onSend: () -> Unit
) {
    private val overlayContainer: FrameLayout
    private lateinit var editor: EditText
    private lateinit var charCountText: TextView
    private var isShowing = false

    init {
        val topBarHeight = dp(54)

        overlayContainer = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor("#0B1118"))
            visibility = View.GONE
            elevation = 50f
        }

        // 顶部操作栏
        val topBar = LinearLayout(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                topBarHeight
            )
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setBackgroundColor(Color.parseColor("#0E1116"))
            setPadding(dp(4), 0, dp(12), 0)
        }

        // 返回按钮（保存文本回输入框）
        val closeButton = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            gravity = Gravity.CENTER
            text = "‹"
            textSize = 26f
            includeFontPadding = false
            setTextColor(Color.parseColor("#B3DDDBD5"))
            setOnClickListener { hide() }
        }

        // 标题
        val titleText = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            gravity = Gravity.CENTER_VERTICAL
            text = "长文本编辑"
            textSize = 15f
            setTextColor(Color.parseColor("#80BEBEBA"))
            setPadding(dp(2), 0, 0, 0)
        }

        // 字数统计
        charCountText = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { marginEnd = dp(12) }
            textSize = 12f
            setTextColor(Color.parseColor("#80BEBEBA"))
        }

        // 发送按钮
        val sendButton = TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(60), dp(34))
            gravity = Gravity.CENTER
            text = "发送"
            textSize = 14f
            setTextColor(Color.parseColor("#0B1118"))
            background = activity.getDrawable(R.drawable.bg_send_button)
            setOnClickListener {
                val text = editor.text.toString()
                setInputText(text)
                dismissOverlay()
                onSend()
            }
        }

        topBar.addView(closeButton)
        topBar.addView(titleText)
        topBar.addView(charCountText)
        topBar.addView(sendButton)

        // 分隔线
        val divider = View(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT, 1
            ).apply { topMargin = topBarHeight }
            setBackgroundColor(Color.parseColor("#20262E"))
        }

        // 编辑区域
        editor = EditText(activity).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ).apply { topMargin = topBarHeight + 1 }
            background = ColorDrawable(Color.TRANSPARENT)
            setPadding(dp(20), dp(16), dp(20), dp(120))
            setTextColor(Color.parseColor("#F8F7F4"))
            setHintTextColor(Color.parseColor("#80BEBEBA"))
            hint = "描述你想要的功能，越详细越好…"
            textSize = 16f
            setLineSpacing(0f, 1.5f)
            gravity = Gravity.TOP or Gravity.START
            setSingleLine(false)
            isVerticalScrollBarEnabled = true
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, st: Int, c: Int, a: Int) = Unit
                override fun afterTextChanged(s: Editable?) = Unit
                override fun onTextChanged(s: CharSequence?, st: Int, b: Int, c: Int) {
                    updateCharCount(s?.length ?: 0)
                }
            })
        }

        overlayContainer.addView(topBar)
        overlayContainer.addView(divider)
        overlayContainer.addView(editor)

        // 附加到 window 最顶层，确保覆盖整个界面
        (activity.window.decorView as? ViewGroup)?.addView(overlayContainer)
    }

    fun show() {
        if (isShowing) return
        isShowing = true
        val text = getInputText()
        editor.setText(text)
        editor.setSelection(text.length)
        updateCharCount(text.length)
        overlayContainer.visibility = View.VISIBLE
        overlayContainer.alpha = 0f
        overlayContainer.animate().alpha(1f).setDuration(180L).start()
        editor.requestFocus()
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.showSoftInput(editor, InputMethodManager.SHOW_IMPLICIT)
    }

    /**
     * 关闭浮层并将编辑内容同步回原输入框。
     * 按返回键或点击返回按钮时调用。
     */
    fun hide() {
        if (!isShowing) return
        isShowing = false
        setInputText(editor.text.toString())
        dismissOverlay()
    }

    fun isShowing() = isShowing

    private fun updateCharCount(length: Int) {
        charCountText.text = if (length > 0) "$length 字" else ""
    }

    private fun dismissOverlay() {
        val imm = activity.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        imm?.hideSoftInputFromWindow(editor.windowToken, 0)
        overlayContainer.animate().alpha(0f).setDuration(150L).withEndAction {
            overlayContainer.visibility = View.GONE
        }.start()
    }

    fun destroy() {
        (overlayContainer.parent as? ViewGroup)?.removeView(overlayContainer)
    }
}
