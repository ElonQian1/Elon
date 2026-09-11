package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Typeface
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatButton
import com.elon.app.chatgptweb.ChatGptWebCanvasContent

internal object WebChatCanvasContentView {
    fun entry(activity: AppCompatActivity, open: () -> Unit): View = AppCompatButton(activity).apply {
        text = "查看画布"
        isAllCaps = false
        contentDescription = "web-chat-canvas-share-view"
        setCompoundDrawablesWithIntrinsicBounds(R.drawable.ic_side_menu_files, 0, 0, 0)
        setOnClickListener { open() }
    }

    fun dialog(activity: AppCompatActivity, value: ChatGptWebCanvasContent, back: () -> Unit): AlertDialog {
        val density = activity.resources.displayMetrics.density
        val padding = (20 * density).toInt()
        val layout = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding / 2, padding, padding)
        }
        layout.addView(TextView(activity).apply {
            text = value.title
            textSize = 18f
            setTextIsSelectable(true)
            contentDescription = "web-chat-canvas-content-title"
        })
        layout.addView(TextView(activity).apply {
            text = value.documentVersion?.let { "公开快照 · 版本 $it" } ?: "公开快照"
            textSize = 13f
            setPadding(0, padding / 2, 0, padding)
        })
        layout.addView(TextView(activity).apply {
            // TextView deliberately renders HTML, code and URLs as inert, selectable source text.
            text = value.content
            textSize = 16f
            if (value.isCode) typeface = Typeface.MONOSPACE
            setTextIsSelectable(true)
            contentDescription = "web-chat-canvas-content-body"
        })
        val scroll = ScrollView(activity).apply {
            addView(layout)
            contentDescription = "web-chat-canvas-content-scroll"
        }
        val frame = LinearLayout(activity).apply {
            val height = minOf((480 * density).toInt(), (activity.resources.displayMetrics.heightPixels * 0.55).toInt())
            addView(scroll, LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, height))
        }
        return AlertDialog.Builder(activity).setTitle("画布原文").setView(frame)
            .setPositiveButton("复制原文") { _, _ ->
                val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                clipboard?.setPrimaryClip(ClipData.newPlainText(value.title, value.content))
                if (clipboard != null) Toast.makeText(activity, "原文已复制", Toast.LENGTH_SHORT).show()
            }
            .setNegativeButton("返回链接") { _, _ -> back() }
            .create()
    }
}
