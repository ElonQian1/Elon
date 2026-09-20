package com.elon.app.chatgptweb

/** Reuse the canonical sequence accepted by both the bridge and private-send ledger. */
internal object GroupWebAiCommandIds {
    private val sequence = ChatGptWebObservedState()

    @Synchronized
    fun next(): String = sequence.nextRequestId()
}
