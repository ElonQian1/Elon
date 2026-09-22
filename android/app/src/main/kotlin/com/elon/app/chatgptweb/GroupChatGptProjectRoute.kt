package com.elon.app.chatgptweb

import java.net.URI

internal object GroupChatGptProjectRoute {
    private const val UUID = "[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}"
    private const val PROJECT = "(g-p-[a-f0-9]{32})(?:-[A-Za-z0-9_-]{1,124})?"
    private val projectPath = Regex("^/g/$PROJECT/(?:project|c/($UUID))$")
    private val conversation = Regex("^/c/($UUID)$")
    private data class Target(val project: String?, val conversation: String?)
    private fun target(url: String): Target? {
        val path = uri(url)?.rawPath ?: return null
        projectPath.matchEntire(path)?.let { return Target(it.groupValues[1], it.groupValues[2].ifBlank { null }) }
        return conversation.matchEntire(path)?.let { Target(null, it.groupValues[1]) }
    }
    fun conversationId(url: String): String? = target(url)?.conversation
    fun matches(url: String, expectedUrl: String): Boolean {
        val actual = target(url) ?: return false
        val expected = target(expectedUrl) ?: return false
        if (expected.project == null) return false
        return actual.conversation == expected.conversation &&
            (actual.project == expected.project || actual.project == null && expected.conversation != null)
    }
    fun ready(snapshot: ChatGptWebSnapshot, documentUrl: String): Boolean {
        val live = uri(documentUrl) ?: return false
        val seen = uri(snapshot.url) ?: return false
        return live.rawPath == seen.rawPath && target(documentUrl) != null && snapshot.authenticated && !snapshot.loginRequired &&
            !snapshot.streaming && snapshot.draft.isBlank() && snapshot.attachments.isEmpty() &&
            (snapshot.composerReady || snapshot.privateSendReady)
    }
    private fun uri(url: String) = runCatching { URI(url) }.getOrNull()?.takeIf {
        it.scheme == "https" && it.host == "chatgpt.com" && it.port in listOf(-1, 443) &&
            it.userInfo == null && it.rawQuery == null && it.fragment == null
    }
}
