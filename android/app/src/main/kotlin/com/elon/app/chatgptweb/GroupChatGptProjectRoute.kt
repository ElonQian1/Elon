package com.elon.app.chatgptweb

import java.net.URI

internal object GroupChatGptProjectRoute {
    private const val UUID = "[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}"
    private val path = Regex("^/g/g-p-[a-f0-9]{32}/(?:project|c/$UUID)$")
    private val conversation = Regex("^(?:/g/g-p-[a-f0-9]{32})?/c/($UUID)$")
    fun conversationId(url: String): String? = uri(url)?.let { conversation.matchEntire(it.path)?.groupValues?.get(1) }
    fun ready(snapshot: ChatGptWebSnapshot, documentUrl: String): Boolean {
        val live = uri(documentUrl) ?: return false
        val seen = uri(snapshot.url) ?: return false
        return live.path == seen.path && path.matches(live.path) && snapshot.authenticated && !snapshot.loginRequired &&
            !snapshot.streaming && snapshot.draft.isBlank() && snapshot.attachments.isEmpty() &&
            (snapshot.composerReady || snapshot.privateSendReady)
    }
    private fun uri(url: String) = runCatching { URI(url) }.getOrNull()?.takeIf {
        it.scheme == "https" && it.host == "chatgpt.com" && it.port in listOf(-1, 443) &&
            it.userInfo == null && it.rawQuery == null && it.fragment == null
    }
}
