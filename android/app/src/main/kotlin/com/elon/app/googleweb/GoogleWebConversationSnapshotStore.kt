package com.elon.app.googleweb

import android.content.Context
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import com.elon.app.chatgptweb.WebChatConversationSnapshotStore

internal class GoogleWebConversationSnapshotStore(
    context: Context,
    private val nowMs: () -> Long = System::currentTimeMillis,
) {
    private val store = WebChatConversationSnapshotStore(
        context = context,
        directoryName = DIRECTORY_NAME,
        fileNameForPath = ::fileName,
        acceptedFileName = FILE_NAME,
        nowMs = nowMs,
    )

    fun restore(path: String): ChatGptWebSnapshot? = store.restore(path)

    fun save(path: String, snapshot: ChatGptWebSnapshot) = store.save(path, snapshot)

    internal companion object {
        fun fileName(path: String): String? = PATH.matchEntire(path)
            ?.groupValues
            ?.get(1)
            ?.let { "google-web-conversation-$it-v1.json" }

        private const val DIRECTORY_NAME = "google-web-conversation-snapshots-v1"
        private val PATH = Regex("^/google-ai-mode/conversation/([a-f0-9]{64})$")
        private val FILE_NAME = Regex("^google-web-conversation-[a-f0-9]{64}-v1\\.json$")
    }
}
