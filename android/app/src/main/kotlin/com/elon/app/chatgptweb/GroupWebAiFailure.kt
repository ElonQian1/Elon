package com.elon.app.chatgptweb

internal enum class GroupWebAiFailureReason(val message: String) {
    PAGE("AI 页面连接失败，请检查网络或加速器后重试。"),
    PREPARATION("AI 页面尚未准备好，请稍后重试。"),
    TIMEOUT("等待 AI 超时，请检查网络后重试。"),
    LOGIN("请先在一龙 AI 中确认网页账号的登录状态，再重试。"),
    MODEL("未能确认群聊保存的模型或档位。请打开群聊档位菜单重新选择，或选择“跟随官网设置”，再重试。"),
    SEND("AI 未确认发送结果。"),
    CANCELLED("已取消本次分析。"),
}

/** Only a failure before authorization can safely reuse the same reserved selection. */
internal data class GroupWebAiFailure(val uncertain: Boolean, val reason: GroupWebAiFailureReason) {
    val canRetry: Boolean get() = !uncertain && reason != GroupWebAiFailureReason.CANCELLED
    val message: String get() = if (uncertain) {
        "请求可能已发送，但没有收到完整回答，因此尚未生成群回复。为避免重复发送，本次不会自动重试。"
    } else "${reason.message}\n所选消息尚未发送，群里还没有 AI 回答。"
}
