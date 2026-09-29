package com.elon.app.chatrecords

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.PopupMenu
import android.widget.Toast
import com.elon.app.articles.ArticleUi
import com.elon.app.sharing.ExternalShareActivity
import com.elon.app.sociallinks.SocialLink

internal object ChatRecordLinkActions {
    fun bind(host: LinearLayout, card: View, current: () -> SocialLink) {
        val ui = ArticleUi(host.context)
        fun copy() {
            (host.context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("链接", current().url))
            Toast.makeText(host.context, "链接已复制", Toast.LENGTH_SHORT).show()
        }
        fun forward() { ExternalShareActivity.shareText(host.context, current().url) }
        fun menu(anchor: View) {
            PopupMenu(host.context, anchor).apply {
                menu.add("转发").setOnMenuItemClickListener { forward(); true }
                menu.add("复制链接").setOnMenuItemClickListener { copy(); true }
            }.show()
        }
        // Card descendants already own click listeners. Long press must not replace those.
        fun bindLong(view: View) {
            view.setOnLongClickListener { menu(card); true }
            if (view is ViewGroup) for (i in 0 until view.childCount) bindLong(view.getChildAt(i))
        }
        bindLong(card)
        val actions = ui.row()
        fun action(label: String, callback: () -> Unit) = android.widget.Button(host.context, null, android.R.attr.borderlessButtonStyle).apply {
            text = label; textSize = 13f; isAllCaps = false; minHeight = ui.dp(48); minimumWidth = 0
            setTextColor(com.elon.app.MobileColors(host.context).primary); setPadding(ui.dp(2), 0, ui.dp(2), 0); setOnClickListener { callback() }
        }
        actions.addView(action("复制链接", ::copy), LinearLayout.LayoutParams(0, -2, 1f))
        host.addView(actions)
    }
}
