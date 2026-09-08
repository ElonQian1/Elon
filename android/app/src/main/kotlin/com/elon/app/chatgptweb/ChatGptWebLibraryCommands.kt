package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebLibraryCommands {
    val actions = setOf("chatgpt_list_library_files", "chatgpt_cancel_library_files", "chatgpt_download_library_file", "chatgpt_mutate_library_file")

    fun control(
        args: JSONObject,
        observed: ChatGptWebObservedState.Snapshot,
        commands: ChatGptWebMcpCommandPort,
        dispatch: (String, (String) -> Unit) -> Unit,
    ): String? {
        when (args.optString("action").trim().lowercase()) {
            "chatgpt_mutate_library_file" -> {
                if (args.opt("confirmed") != true) return "user_confirmation_required"
                val file = observed.libraryFiles?.items?.singleOrNull { it.handle == args.optString("file_handle") }
                    ?: return "library_selection_expired"
                val operation = args.optString("operation")
                if (file.kind != "file" || when (operation) {
                        "rename" -> !file.canRename
                        "trash" -> !file.canTrash
                        else -> true
                    }) return "library_mutation_unsupported"
                val name = args.optString("name")
                if (operation == "rename" && !validName(name)) return "invalid_library_name"
                dispatch("mutate_library_file") { commands.mutateLibraryFile(JSONObject()
                    .put("fileHandle", file.handle).put("operation", operation).put("name", name), it) }
            }
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

    fun validName(value: String) = value.isNotBlank() && value == value.trim() && value.length <= 180 &&
        value !in setOf(".", "..") && value.none { it.isISOControl() || it == '/' || it == '\\' }
}
