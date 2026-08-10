package com.elon.app.chatgptweb

internal object ChatGptWebUiSemantics {
    const val GENERIC_ACTION = "action"
    const val DICTATION = "dictation"

    val KNOWN = setOf(
        "navigation",
        "title",
        "profile",
        "new_conversation",
        "attachment",
        "model",
        DICTATION,
        "send",
        "stop",
        "suggestion",
        "copy",
        "regenerate",
        "edit",
        "share",
        "feedback",
        "read_aloud",
        "branch",
        "delete",
        "close",
        "confirm",
        "conversation",
        "search",
        "library",
        "tasks",
        "project",
        "gpts",
        "settings",
        "create_asset",
        "sources",
        "conversation_files",
        "pin",
        "archive",
        "more",
        "timestamp",
        GENERIC_ACTION,
    )
}
