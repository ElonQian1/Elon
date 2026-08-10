package com.elon.app.chatgptweb

import android.content.Context
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.TextView
import androidx.appcompat.app.AlertDialog

internal object ChatGptNativeControlDialog {
    fun show(
        context: Context,
        title: CharSequence,
        controls: List<ChatGptWebUiControl>,
        onSelected: (ChatGptWebUiControl) -> Unit,
    ): AlertDialog {
        val adapter = ControlAdapter(context, controls)
        return AlertDialog.Builder(context)
            .setTitle(title)
            .setAdapter(adapter) { opened, which ->
                opened.dismiss()
                onSelected(controls[which])
            }
            .setNegativeButton(android.R.string.cancel, null)
            .create()
            .also(AlertDialog::show)
    }

    private class ControlAdapter(
        context: Context,
        private val controls: List<ChatGptWebUiControl>,
    ) : ArrayAdapter<ChatGptWebUiControl>(context, android.R.layout.simple_list_item_1, controls) {
        override fun isEnabled(position: Int): Boolean = controls[position].enabled

        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
            val textView = (convertView ?: LayoutInflater.from(context)
                .inflate(android.R.layout.simple_list_item_1, parent, false)) as TextView
            val control = controls[position]
            textView.text = control.label
            textView.contentDescription = control.accessibilityLabel
            textView.tag = control.id
            textView.isEnabled = control.enabled
            textView.alpha = if (control.enabled) 1f else DISABLED_ALPHA
            return textView
        }
    }

    private const val DISABLED_ALPHA = 0.45f
}
