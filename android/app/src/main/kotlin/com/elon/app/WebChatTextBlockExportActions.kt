package com.elon.app

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.widget.Toast

internal object WebChatTextBlockExportActions {
    fun intent(result: WebChatTextBlockExport.Result, share: Boolean): Intent? {
        val uri = result.uri?.takeIf { it.scheme == "content" } ?: return null
        return (if (share) Intent(Intent.ACTION_SEND).setType(result.mediaType)
            .putExtra(Intent.EXTRA_STREAM, uri).putExtra(Intent.EXTRA_TITLE, result.name)
        else Intent(Intent.ACTION_VIEW).setDataAndType(uri, result.mediaType))
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            .apply { clipData = ClipData.newRawUri(result.name, uri) }
    }

    fun open(context: Context, result: WebChatTextBlockExport.Result, share: Boolean) {
        val action = intent(result, share)
        if (action == null) {
            Toast.makeText(context, "文件已导出，请在下载目录中查找", Toast.LENGTH_LONG).show()
            return
        }
        runCatching { context.startActivity(Intent.createChooser(action, if (share) "分享副本" else "打开副本")) }
            .onFailure { Toast.makeText(context, "没有可处理此文件的应用，文件仍在下载目录", Toast.LENGTH_LONG).show() }
    }
}
