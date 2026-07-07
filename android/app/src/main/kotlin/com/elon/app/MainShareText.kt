package com.elon.app

internal fun promotionText(apkDownloadUrl: String): String {
    return buildString {
        append("我正在用「一龙」云端 APK 开发平台，手机里直接提需求，云端帮你改代码、打包并生成安装包。")
        append("\n\n")
        append("下载地址：")
        append(apkDownloadUrl)
    }
}

internal fun shareableMessageText(message: ChatMessage): String {
    if (message.isRecalled()) return ""
    return buildString {
        append(message.content.trim())
        val details = message.evidenceDetails?.trim().orEmpty()
        if (details.isNotBlank()) {
            append("\n\n")
            message.evidenceTitle?.trim()?.takeIf { it.isNotBlank() }?.let {
                append(it)
                append('\n')
            }
            append(details)
        }
    }.trim()
}
