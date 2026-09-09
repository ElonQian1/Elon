package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebDirectoryPageCommands {
    val actions = setOf("chatgpt_browse_directory_page", "chatgpt_cancel_directory_page")

    fun control(args: JSONObject, commands: ChatGptWebMcpCommandPort, dispatch: (String, (String) -> Unit) -> Unit): String? {
        if (args.optString("action").trim().lowercase() == "chatgpt_cancel_directory_page") {
            val request = args.optString("directory_request_id")
            if (!Regex("mcp_[a-z0-9]{1,32}").matches(request)) return "invalid_directory_request"
            dispatch("cancel_directory_page") { commands.cancelDirectoryPage(request, it) }
        } else {
            val scope = args.optString("scope")
            val handle = args.optString("page_handle")
            if (!ChatGptWebDirectoryPage.validScope(scope) || handle.isNotEmpty() && !ChatGptWebDirectoryPage.HANDLE.matches(handle)) {
                return "invalid_directory_request"
            }
            dispatch(ChatGptWebDirectoryPage.ACTION) { commands.browseDirectoryPage(
                JSONObject().put("scope", scope).put("handle", handle), it) }
        }
        return null
    }
}
