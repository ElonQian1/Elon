package com.elon.app.chatrecords

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.Toast
import com.elon.app.ChatMessage
import com.elon.app.R
import com.elon.app.TopAction
import com.elon.app.isRecalled

/** Record-specific additions; quoting, selection and recall remain ordinary message actions. */
internal object ChatRecordMenuActions {
    fun actions(context: Context, message: ChatMessage): List<TopAction> {
        if (message.isRecalled()) return emptyList()
        val card = ChatRecordDocument.card(message.content) ?: return emptyList()
        return listOf(
            TopAction("打开记录", R.drawable.ic_popup_chat) {
                ChatRecordReaderActivity.open(context, card.getString("group_id"), card.getString("record_id"))
            },
            TopAction("复制标题", R.drawable.ic_msg_copy) {
                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText("聊天记录标题", card.optString("title")))
                Toast.makeText(context, "已复制标题", Toast.LENGTH_SHORT).show()
            }
        )
    }
}
