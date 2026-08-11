package com.elon.app.chatgptweb

internal object ChatGptNativeMessageRevealTarget {
    const val MESSAGE = "message"
    const val CONTENT = "content"
    const val COPY = "copy"
    const val REGENERATE = "regenerate"
    const val ACTIONS = "actions"

    val ALL = listOf(MESSAGE, CONTENT, COPY, REGENERATE, ACTIONS)
}
