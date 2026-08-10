package com.elon.app.chatgptweb

import android.content.Context
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.TextView
import androidx.appcompat.app.AlertDialog

internal object ChatGptNativeComposerOptionDialog {
    fun show(
        context: Context,
        title: CharSequence,
        section: String,
        options: List<ChatGptWebComposerOption>,
        singleChoice: Boolean,
        cancelLabel: Int,
        officialLabel: Int,
        onSelected: (ChatGptWebComposerOption) -> Unit,
        onCancelled: () -> Unit,
        onOpenOfficial: () -> Unit,
    ): AlertDialog {
        val adapter = OptionAdapter(context, section, options, singleChoice)
        val selectedIndex = options.indexOfFirst(ChatGptWebComposerOption::selected)
        val builder = AlertDialog.Builder(context)
            .setTitle(title)
            .setNegativeButton(cancelLabel) { _, _ -> onCancelled() }
            .setNeutralButton(officialLabel) { _, _ -> onOpenOfficial() }
            .setOnCancelListener { onCancelled() }

        if (singleChoice) {
            builder.setSingleChoiceItems(adapter, selectedIndex) { opened, index ->
                opened.dismiss()
                onSelected(options[index])
            }
        } else {
            builder.setAdapter(adapter) { opened, index ->
                opened.dismiss()
                onSelected(options[index])
            }
        }

        return builder.create().also { dialog ->
            dialog.show()
            dialog.listView.contentDescription = ChatGptNativeNavigationSelector.composerDialog(section)
        }
    }

    private class OptionAdapter(
        context: Context,
        private val section: String,
        private val options: List<ChatGptWebComposerOption>,
        private val singleChoice: Boolean,
    ) : ArrayAdapter<ChatGptWebComposerOption>(
        context,
        if (singleChoice) android.R.layout.simple_list_item_single_choice
        else android.R.layout.simple_list_item_1,
        options,
    ) {
        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
            val layout = if (singleChoice) {
                android.R.layout.simple_list_item_single_choice
            } else {
                android.R.layout.simple_list_item_1
            }
            val textView = (convertView ?: LayoutInflater.from(context)
                .inflate(layout, parent, false)) as TextView
            val option = options[position]
            textView.text = option.label
            textView.contentDescription =
                ChatGptNativeNavigationSelector.composerOption(section, option)
            textView.tag = option.id
            return textView
        }
    }
}
