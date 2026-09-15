package com.elon.app

internal object WebChatPendingWriteRecoveryPresentation {
    fun resolve(state: String): WebChatConsumerRecoveryState? {
        val message = when (state) {
            "checking" -> "正在核对上次发送结果"
            "unconfirmed" -> "上次发送结果待确认，请勿重复发送"
            "unavailable" -> "发送状态暂无法核对，请稍后再试"
            else -> return null
        }
        return WebChatConsumerRecoveryState(
            visible = true,
            message = message,
            retryVisible = false,
            officialVisible = state != "checking",
            officialLabel = "查看",
        )
    }
}
