package com.elon.app.chatgptweb

internal object ChatGptWebUiSemantics {
    const val GENERIC_ACTION = "action"

    val KNOWN = setOf(
        "navigation",
        "title",
        "profile",
        "new_conversation",
        "attachment",
        "model",
        "dictation",
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
        "more",
        "timestamp",
        GENERIC_ACTION,
    )
}
