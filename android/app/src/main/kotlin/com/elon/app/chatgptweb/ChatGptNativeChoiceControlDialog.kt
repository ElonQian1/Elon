package com.elon.app.chatgptweb

import android.content.Context
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.ListView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import com.elon.app.WebChatConsumerControl

internal object ChatGptNativeChoiceControlDialog {
    fun choiceSelector(controlId: String, choiceIndex: Int): String =
        "chatgpt-control-choice:${ChatGptNativeControlPresentation.stableContextId(controlId)}:$choiceIndex"

    fun show(
        context: Context,
        control: WebChatConsumerControl,
        onSelected: (String, Int) -> Unit,
    ): AlertDialog {
        require(control.supportsChoiceSelection) { "Control does not expose selectable choices." }
        val list = ListView(context).apply {
            choiceMode = ListView.CHOICE_MODE_SINGLE
            dividerHeight = 0
            adapter = ChoiceAdapter(context, control)
            control.selectedChoiceIndex?.let { setItemChecked(it, true) }
        }
        return AlertDialog.Builder(context)
            .setTitle(control.label)
            .setView(list)
            .setNegativeButton(android.R.string.cancel, null)
            .create()
            .also { dialog ->
                list.setOnItemClickListener { _, _, position, _ ->
                    onSelected(control.id, position)
                    dialog.dismiss()
                }
                dialog.show()
            }
    }

    private class ChoiceAdapter(
        context: Context,
        private val control: WebChatConsumerControl,
    ) : ArrayAdapter<String>(context, android.R.layout.simple_list_item_single_choice, control.choiceLabels) {
        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
            val view = convertView ?: LayoutInflater.from(context).inflate(
                android.R.layout.simple_list_item_single_choice,
                parent,
                false,
            )
            return view.apply {
                findViewById<TextView>(android.R.id.text1).apply {
                    text = control.choiceLabels[position]
                    contentDescription = choiceSelector(control.id, position)
                }
            }
        }
    }
}
