package com.elon.app.chatgptweb

internal object ChatGptWebFileByteOwnership {
    fun open(
        journal: ChatGptWebDownloadJournal,
        id: String,
        create: () -> ChatGptWebFileByteDestination,
    ): ChatGptWebFileByteDestination {
        val owner = journal.begin(id)
        val destination = try { create() } catch (failure: Exception) {
            owner.close()
            throw failure
        }
        return object : ChatGptWebFileByteDestination {
            override fun write(bytes: ByteArray) = destination.write(bytes)
            override fun publish() {
                destination.publish()
                // A journal cleanup failure cannot undo a successfully published file.
                runCatching { owner.complete() }.onFailure { runCatching { owner.close() } }
            }
            override fun discard() {
                try { destination.discard(); owner.complete() } finally { owner.close() }
            }
        }
    }
}
