package com.elon.app.chatgptweb

import android.content.Context
import java.security.MessageDigest

internal class ChatGptConversationSnapshotStore(
    context: Context,
    nowMs: () -> Long = System::currentTimeMillis,
) : WebChatConversationSnapshotRepository {
    private val store = WebChatConversationSnapshotStore(
        context = context,
        directoryName = DIRECTORY_NAME,
        fileNameForPath = ::fileName,
        acceptedFileName = FILE_NAME,
        nowMs = nowMs,
    )

    override fun restore(path: String): ChatGptWebSnapshot? = store.restore(path)

    override fun save(path: String, snapshot: ChatGptWebSnapshot) = store.save(path, snapshot)

    internal companion object {
        fun fileName(path: String): String? = ChatGptWebConversationPath.identity(path)
            ?.let(::sha256)
            ?.let { "chatgpt-web-conversation-$it-v1.json" }

        private fun sha256(value: String): String = MessageDigest.getInstance("SHA-256")
            .digest(value.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it) }

        private const val DIRECTORY_NAME = "chatgpt-web-conversation-snapshots-v1"
        private val FILE_NAME = Regex("^chatgpt-web-conversation-[a-f0-9]{64}-v1\\.json$")
    }
}
