package com.elon.app.chatgptweb

import android.view.MotionEvent
import android.view.View
import android.view.Window
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.elon.app.R
import com.google.android.material.button.MaterialButton
import com.google.android.material.button.MaterialButtonToggleGroup

internal class ChatGptWebModeController(
    private val window: Window,
    private val root: View,
    private val toggle: MaterialButtonToggleGroup,
    private val quickButton: MaterialButton,
    private val webButton: MaterialButton,
    private val nativeButton: MaterialButton,
    private val webView: View,
    private val quickRoot: View,
    private val nativeRoot: View,
    private val onModeChanged: (Mode) -> Unit,
) {
    enum class Mode {
        QUICK,
        WEB,
        NATIVE,
    }

    fun attach() {
        listOf(quickButton, webButton, nativeButton).forEach { button ->
            button.setOnTouchListener { _, event ->
                if (event.actionMasked == MotionEvent.ACTION_DOWN) hideKeyboard()
                false
            }
        }
        toggle.addOnButtonCheckedListener { _, checkedId, isChecked ->
            if (isChecked) render(modeFor(checkedId))
        }
        select(Mode.QUICK)
    }

    fun select(mode: Mode) {
        val buttonId = when (mode) {
            Mode.QUICK -> R.id.chatGptModeQuick
            Mode.WEB -> R.id.chatGptModeWeb
            Mode.NATIVE -> R.id.chatGptModeNative
        }
        if (toggle.checkedButtonId == buttonId) {
            render(mode)
        } else {
            toggle.check(buttonId)
        }
    }

    fun isNativeSelected(): Boolean = toggle.checkedButtonId == R.id.chatGptModeNative

    fun isQuickSelected(): Boolean = toggle.checkedButtonId == R.id.chatGptModeQuick

    private fun render(mode: Mode) {
        hideKeyboard()
        webView.visibility = View.VISIBLE
        quickRoot.visibility = if (mode == Mode.QUICK) View.VISIBLE else View.GONE
        nativeRoot.visibility = if (mode == Mode.NATIVE) View.VISIBLE else View.GONE
        if (mode != Mode.WEB) webView.clearFocus()
        onModeChanged(mode)
    }

    private fun hideKeyboard() {
        WindowInsetsControllerCompat(window, root)
            .hide(WindowInsetsCompat.Type.ime())
        root.findFocus()?.clearFocus()
    }

    private fun modeFor(checkedId: Int): Mode = when (checkedId) {
        R.id.chatGptModeNative -> Mode.NATIVE
        R.id.chatGptModeWeb -> Mode.WEB
        else -> Mode.QUICK
    }

}
