package com.elon.app.scan

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.elon.app.AddFriendActivity
import com.elon.app.AuthManager
import com.elon.app.LoginActivity

internal object ScanResultDialog {
    fun show(activity: AppCompatActivity, values: List<String>, onDismiss: () -> Unit) {
        if (values.size > 1) {
            var selected = false
            MaterialAlertDialogBuilder(activity).setTitle("识别到 ${values.size} 个码")
                .setItems(values.map { runCatching { ScanPayloadParser.parse(it).title }.getOrDefault("文本") + " · " + it.take(60) }.toTypedArray()) { _, index ->
                    selected = true; show(activity, listOf(values[index]), onDismiss)
                }.setNegativeButton("关闭", null).setOnDismissListener { if (!selected) onDismiss() }.show()
            return
        }
        val result = runCatching { ScanPayloadParser.parse(values.single()) }.getOrElse {
            Toast.makeText(activity, it.message, Toast.LENGTH_LONG).show(); onDismiss(); return
        }
        val body = TextView(activity).apply {
            text = result.raw; setTextIsSelectable(true); textSize = 16f
            val padding = (24 * resources.displayMetrics.density).toInt(); setPadding(padding, padding / 2, padding, padding / 2)
        }
        val builder = MaterialAlertDialogBuilder(activity).setTitle(result.title)
            .setView(ScrollView(activity).apply { addView(body) })
            .setNegativeButton("关闭", null)
            .setNeutralButton("复制内容") { _, _ ->
                (activity.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("扫码结果", result.raw))
                Toast.makeText(activity, "已复制", Toast.LENGTH_SHORT).show()
            }.setOnDismissListener { onDismiss() }
        val action = when (result.kind) {
            "friend" -> if (AuthManager.isLoggedIn(activity)) "查找用户" else "登录后查找"
            "url" -> "打开网页"
            "phone" -> "打开拨号器"
            "sms" -> "编写短信"
            "email" -> "编写邮件"
            "geo" -> "打开地图"
            else -> null
        }
        if (action != null) builder.setPositiveButton(action) { _, _ ->
            val intent = when (result.kind) {
                "friend" -> if (AuthManager.isLoggedIn(activity)) Intent(activity, AddFriendActivity::class.java).putExtra("scan_account_id", result.target) else Intent(activity, LoginActivity::class.java)
                "phone" -> Intent(Intent.ACTION_DIAL, Uri.parse(result.target))
                "sms", "email" -> Intent(Intent.ACTION_SENDTO, Uri.parse(result.target))
                else -> Intent(Intent.ACTION_VIEW, Uri.parse(result.target)).addCategory(Intent.CATEGORY_BROWSABLE)
            }
            runCatching { activity.startActivity(intent) }.onFailure { Toast.makeText(activity, "没有可打开此内容的应用，可复制内容", Toast.LENGTH_LONG).show() }
        }
        builder.show()
    }
}
