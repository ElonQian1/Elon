package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.net.Uri
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class MainShareActions(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int
) {
    fun searchMessageText(text: String) {
        Toast.makeText(activity, "搜一搜：${summarize(text, 12)}", Toast.LENGTH_SHORT).show()
    }

    fun toastMessageAction(text: String) {
        Toast.makeText(activity, text, Toast.LENGTH_SHORT).show()
    }

    fun copyMessageText(text: String) {
        copyText("一龙聊天内容", text)
        Toast.makeText(activity, "已复制", Toast.LENGTH_SHORT).show()
    }

    fun forwardMessageText(text: String) {
        shareText(text, "转发聊天内容")
    }

    fun showPromotionDialog(apkDownloadUrl: String, apkDownloadPageUrl: String) {
        val text = promotionText(apkDownloadUrl)
        val content = TextView(activity).apply {
            setText(text)
            setTextIsSelectable(true)
            setPadding(dp(22), dp(8), dp(22), dp(2))
            setTextColor(Color.parseColor("#333333"))
            textSize = 14f
            setLineSpacing(dp(3).toFloat(), 1.0f)
        }

        AlertDialog.Builder(activity)
            .setTitle("分享推广")
            .setView(content)
            .setPositiveButton("复制推广语") { _, _ -> copyPromotionText(text) }
            .setNeutralButton("系统分享") { _, _ -> shareText(text, "分享一龙 APK") }
            .setNegativeButton("打开下载页") { _, _ ->
                activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(apkDownloadPageUrl)))
            }
            .show()
    }

    private fun copyPromotionText(text: String) {
        copyText("一龙推广语", text)
        Toast.makeText(activity, "推广语已复制", Toast.LENGTH_SHORT).show()
    }

    private fun copyText(label: String, text: String) {
        val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboard.setPrimaryClip(ClipData.newPlainText(label, text))
    }

    private fun shareText(text: String, chooserTitle: String) {
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        activity.startActivity(Intent.createChooser(intent, chooserTitle))
    }
}
