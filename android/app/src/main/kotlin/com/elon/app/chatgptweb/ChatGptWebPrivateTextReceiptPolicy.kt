package com.elon.app.chatgptweb

import com.elon.app.WebChatSendAuthority

internal data class ChatGptWebSendReceiptSemantics(
    val authority: WebChatSendAuthority,
    val indeterminate: Boolean,
)

internal object ChatGptWebPrivateTextReceiptPolicy {
    private const val ACCEPTED = "private_text_v1:accepted"
    private const val UNKNOWN_PREFIX = "private_text_v1:unknown:"
    private val SAFE_CODE = Regex("^[a-z_]{1,32}$")

    fun resolve(event: ChatGptWebEvent.CommandResult): ChatGptWebSendReceiptSemantics {
        if (event.action != "send_prompt") return official()
        if (event.ok && event.detail == ACCEPTED) {
            return ChatGptWebSendReceiptSemantics(
                authority = WebChatSendAuthority.SAME_ORIGIN_PRIVATE,
                indeterminate = false,
            )
        }
        if (!event.ok && event.detail.startsWith(UNKNOWN_PREFIX)) {
            val code = event.detail.removePrefix(UNKNOWN_PREFIX)
            if (SAFE_CODE.matches(code)) {
                return ChatGptWebSendReceiptSemantics(
                    authority = WebChatSendAuthority.SAME_ORIGIN_PRIVATE,
                    indeterminate = true,
                )
            }
        }
        return official()
    }

    fun userDetail(event: ChatGptWebEvent.CommandResult): String =
        if (resolve(event).indeterminate) {
            "发送结果正在核对，为避免重复发送，请稍候。"
        } else {
            event.detail
        }

    private fun official() = ChatGptWebSendReceiptSemantics(
        authority = WebChatSendAuthority.OFFICIAL_PAGE,
        indeterminate = false,
    )
}
