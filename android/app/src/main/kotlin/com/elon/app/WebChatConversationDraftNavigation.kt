package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal object WebChatConversationDraftNavigation {
    fun blocks(
        targetPath: String,
        currentPath: String?,
        draftPresent: Boolean,
    ): Boolean {
        if (!draftPresent) return false
        val targetIdentity = ChatGptWebConversationPath.identity(targetPath) ?: return false
        return targetIdentity != ChatGptWebConversationPath.identity(currentPath)
    }

    fun dialog(activity: AppCompatActivity): AlertDialog = AlertDialog.Builder(activity)
        .setTitle("先处理输入内容")
        .setMessage("当前输入框有未发送内容。为避免丢失，应用没有切换会话，也没有提交操作。请先发送或清空内容后再重试。")
        .setPositiveButton("知道了", null)
        .create()
}
