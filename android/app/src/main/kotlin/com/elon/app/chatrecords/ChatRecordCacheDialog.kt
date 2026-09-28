package com.elon.app.chatrecords

import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.app.AlertDialog

internal object ChatRecordCacheDialog {
    fun show(activity: AppCompatActivity, model: ChatRecordReaderModel) {
        val dialog = AlertDialog.Builder(activity).setTitle("聊天记录缓存").setMessage("正在统计…").setNegativeButton("关闭", null).create()
        dialog.show()
        model.cacheUsage { result ->
            if (!dialog.isShowing || activity.isFinishing || activity.isDestroyed) return@cacheUsage
            result.onFailure { dialog.setMessage("暂时无法统计缓存，请重试") }.onSuccess { (current, total) ->
                dialog.dismiss()
                fun size(bytes: Long) = android.text.format.Formatter.formatShortFileSize(activity, bytes)
                fun confirm(all: Boolean) {
                    AlertDialog.Builder(activity).setTitle(if (all) "清理全部聊天记录缓存？" else "清理当前记录缓存？")
                        .setMessage("只清理本机已下载的记录、图片、视频及缩略图，不删除群消息、导入草稿或登录信息。下次查看会重新下载。")
                        .setNegativeButton("取消", null).setPositiveButton("清理") { _, _ -> model.clearCache(all) }.show()
                }
                AlertDialog.Builder(activity).setTitle("聊天记录缓存")
                    .setMessage("当前记录：${size(current)}\n全部记录：${size(total)}\n\n上限 128 MB；7 天未使用自动清理；超过上限时优先清理最久未使用的记录附件。")
                    .setPositiveButton("清理当前") { _, _ -> confirm(false) }
                    .setNeutralButton("清理全部") { _, _ -> confirm(true) }.setNegativeButton("关闭", null).show()
            }
        }
    }
}
