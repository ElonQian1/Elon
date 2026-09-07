package com.elon.app.chatgptweb

internal object ChatGptWebRegenerationReceiptPolicy {
    private const val PREFIX = "official_runtime_v1:regenerate_"
    private val CODE = Regex("^[a-z_]{1,32}$")

    fun userDetail(action: String, ok: Boolean, detail: String): String? {
        if (action != "regenerate_response" || !detail.startsWith(PREFIX)) return null
        val value = detail.removePrefix(PREFIX)
        if (ok) return if (value == "observed") "已开始重新生成回复。" else null
        val kind = value.substringBefore(':')
        val code = value.substringAfter(':', "")
        if (!CODE.matches(code)) return null
        return when {
            kind == "rejected" -> "当前会话暂不能重新生成，本次未提交，请确认会话状态后重试。"
            kind != "unknown" -> null
            code == "busy" -> "上一条回复仍在处理中，请稍候。"
            else -> "重新生成结果正在核对，请稍候，勿重复提交。"
        }
    }
}
