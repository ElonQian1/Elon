package com.elon.app

internal data class WebChatSessionRecoveryFailure(
    val kind: Kind,
    val pageError: String = "",
) {
    enum class Kind { NAVIGATION_TIMEOUT, BRIDGE_TIMEOUT, PAGE_ERROR, RETRY_NOT_STARTED, UNKNOWN }

    fun userMessage(): String = when (kind) {
        Kind.NAVIGATION_TIMEOUT -> "网页加载超时，请重试或打开官网查看"
        Kind.BRIDGE_TIMEOUT -> "网页已加载，但聊天输入器尚未就绪，请重试或打开官网查看"
        Kind.PAGE_ERROR -> pageError.ifBlank { "网页加载失败，请重试或打开官网查看" }
        Kind.RETRY_NOT_STARTED -> "暂时无法重新加载网页，请重试"
        Kind.UNKNOWN -> "网页会话尚未恢复，请重试或打开官网查看"
    }
}
