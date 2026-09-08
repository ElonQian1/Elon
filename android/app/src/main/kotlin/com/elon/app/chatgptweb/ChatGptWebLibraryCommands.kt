package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebLibraryCommands {
    val actions = setOf("chatgpt_list_library_files", "chatgpt_cancel_library_files", "chatgpt_download_library_file")

    fun control(
        args: JSONObject,
        observed: ChatGptWebObservedState.Snapshot,
        commands: ChatGptWebMcpCommandPort,
        dispatch: (String, (String) -> Unit) -> Unit,
    ): String? {
        when (args.optString("action").trim().lowercase()) {
            "chatgpt_list_library_files" -> {
                val directory = args.optString("directory_handle")
                val query = args.optString("query")
                val operation = args.optString("operation", "open")
                if (directory.isNotEmpty() && !ChatGptWebLibraryProtocol.HANDLE.matches(directory) ||
                    query.length > 200 || query.any(Char::isISOControl) || operation !in setOf("open", "refresh", "next")) {
                    return "invalid_library_request"
                }
                dispatch(ChatGptWebLibraryProtocol.ACTION) { id -> commands.listLibraryFiles(
                    JSONObject().put("directoryHandle", directory).put("query", query).put("operation", operation), id)
                }
            }
            "chatgpt_cancel_library_files" -> {
                val id = args.optString("library_request_id")
                if (!Regex("mcp_[a-z0-9]{1,32}").matches(id)) return "invalid_library_request"
                dispatch("cancel_library_files") { commands.cancelLibraryFiles(id, it) }
            }
            "chatgpt_download_library_file" -> {
                val file = observed.libraryFiles?.items?.singleOrNull { it.handle == args.optString("file_handle") }
                    ?: return "download_selection_expired"
                if (file.kind != "file" || !ChatGptWebFileDownloadPolicy.HANDLE.matches(file.downloadHandle)) return "download_not_supported"
                if (args.optString("download_handle") != file.downloadHandle) return "download_selection_expired"
                dispatch("download_library_file") { commands.downloadLibraryFile(file, it) }
            }
        }
        return null
    }
}
