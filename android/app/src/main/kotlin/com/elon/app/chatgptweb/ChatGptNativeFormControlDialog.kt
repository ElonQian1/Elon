package com.elon.app.chatgptweb

import android.content.Context
import android.text.InputType
import android.widget.FrameLayout
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.widget.AppCompatEditText

internal object ChatGptNativeFormControlDialog {
    fun inputSelector(controlId: String): String =
        "chatgpt-control-input:${ChatGptNativeControlPresentation.stableContextId(controlId)}"

    fun commitSelector(controlId: String): String =
        "chatgpt-control-input-commit:${ChatGptNativeControlPresentation.stableContextId(controlId)}"

    fun show(
        context: Context,
        control: ChatGptWebUiControl,
        onSubmit: (String, String) -> Unit,
    ): AlertDialog {
        require(control.supportsTextEntry) { "Control is not a writable text field." }
        val input = AppCompatEditText(context).apply {
            hint = control.label
            inputType = inputTypeFor(control.inputKind)
            isSingleLine = control.inputKind !in MULTILINE_KINDS
            maxLines = if (isSingleLine) 1 else 8
            contentDescription = inputSelector(control.id)
            tag = control.id
        }
        val container = FrameLayout(context).apply {
            val horizontal = dp(context, 24)
            setPadding(horizontal, dp(context, 8), horizontal, 0)
            addView(
                input,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                ),
            )
        }
        return AlertDialog.Builder(context)
            .setTitle(control.label)
            .setView(container)
            .setNegativeButton(android.R.string.cancel, null)
            .setPositiveButton(android.R.string.ok, null)
            .create()
            .also { dialog ->
                dialog.setOnShowListener {
                    dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
                        contentDescription = commitSelector(control.id)
                        setOnClickListener {
                            onSubmit(control.id, input.text?.toString().orEmpty())
                            dialog.dismiss()
                        }
                    }
                    input.requestFocus()
                }
                dialog.show()
            }
    }

    private fun inputTypeFor(kind: String?): Int = when (kind) {
        "email" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        "url" -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
        "tel" -> InputType.TYPE_CLASS_PHONE
        "number" -> InputType.TYPE_CLASS_NUMBER or
            InputType.TYPE_NUMBER_FLAG_DECIMAL or InputType.TYPE_NUMBER_FLAG_SIGNED
        "date", "time", "datetime-local", "month", "week" -> InputType.TYPE_CLASS_DATETIME
        in MULTILINE_KINDS -> InputType.TYPE_CLASS_TEXT or
            InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        else -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
    }

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()

    private val MULTILINE_KINDS = setOf("textarea", "contenteditable")
}
