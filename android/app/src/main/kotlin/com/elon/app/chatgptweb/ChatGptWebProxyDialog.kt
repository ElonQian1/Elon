package com.elon.app.chatgptweb

import android.content.Context
import android.text.InputType
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import com.elon.app.R

internal object ChatGptWebProxyDialog {
    fun show(
        context: Context,
        controller: ChatGptWebProxyController,
        onApplied: (ChatGptWebProxyStatus) -> Unit,
    ) {
        val input = EditText(context).apply {
            hint = context.getString(R.string.chatgpt_web_proxy_hint)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            setSingleLine(true)
            controller.savedManualEndpoint()?.let {
                setText(ChatGptWebProxyController.displayEndpoint(it))
                setSelection(text.length)
            }
        }
        val content = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            val horizontal = context.dp(24)
            setPadding(horizontal, context.dp(4), horizontal, 0)
            addView(
                TextView(context).apply {
                    text = context.getString(
                        R.string.chatgpt_web_proxy_current,
                        controller.currentStatus().label,
                    )
                    setTextColor(context.getColor(R.color.elon_text_secondary))
                },
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            )
            addView(
                input,
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            )
        }
        val dialog = AlertDialog.Builder(context)
            .setTitle(R.string.chatgpt_web_proxy_title)
            .setView(content)
            .setNegativeButton(R.string.chatgpt_web_cancel, null)
            .setNeutralButton(R.string.chatgpt_web_proxy_use_system, null)
            .setPositiveButton(R.string.chatgpt_web_proxy_apply, null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                controller.useSystemNetwork { status ->
                    if (status.error == null) dialog.dismiss() else input.error = status.error
                    onApplied(status)
                }
            }
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                input.error = controller.setManualProxy(input.text.toString()) { status ->
                    if (status.error == null) dialog.dismiss() else input.error = status.error
                    onApplied(status)
                }
            }
        }
        dialog.show()
    }

    private fun Context.dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
