package com.elon.app.chatgptweb

/** Audience-preserving share links, validated before entering the command ledger. */
internal object ChatGptWebConversationShareReceipt {
    private const val PUBLIC = "share_link_ready:"
    private const val MEMBERS = "project_share_link_ready:"
    private const val UUID = "[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}"
    private val publicUrl = Regex("https://chatgpt\\.com/share/$UUID")
    private val memberUrl = Regex("https://chatgpt\\.com/g/(g-p-[a-fA-F0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?" +
        "/shared/c/($UUID)\\?owner_user_id=[A-Za-z0-9_-]{1,160}")
    private val error = Regex("(?:share_[a-z0-9_]{1,72}|user_confirmation_required)")

    data class Link(val url: String, val projectId: String? = null, val conversationId: String? = null)

    fun parse(raw: String?): Link? {
        if (raw == null || raw.length > 512) return null
        if (raw.startsWith(PUBLIC)) return raw.removePrefix(PUBLIC).takeIf(publicUrl::matches)?.let(::Link)
        if (!raw.startsWith(MEMBERS)) return null
        val url = raw.removePrefix(MEMBERS)
        val match = memberUrl.matchEntire(url) ?: return null
        return Link(url, match.groupValues[1], match.groupValues[2])
    }

    fun detail(raw: String): String = when {
        raw.startsWith("{") -> ChatGptWebSharedLinks.detail(raw)
        parse(raw) != null || error.matches(raw) -> raw
        else -> "share_result_unconfirmed"
    }
}
